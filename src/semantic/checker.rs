use crate::ast::*;
use crate::diagnostic::{Diagnostic, DiagnosticBag, DiagnosticCode, Span};
use crate::semantic::types::RjType;
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TypedSourceFile {
    pub source: SourceFile,
    pub package_name: String,
    pub class_name: Option<String>,
    pub symbol_types: FxHashMap<String, RjType>,
}

pub struct SemanticChecker {
    diagnostics: DiagnosticBag,
    source_root: PathBuf,
}

impl SemanticChecker {
    pub fn new(source_root: impl Into<PathBuf>) -> Self {
        Self {
            diagnostics: DiagnosticBag::new(),
            source_root: source_root.into(),
        }
    }

    pub fn check_project(
        &mut self,
        files: &[SourceFile],
    ) -> Result<Vec<TypedSourceFile>, DiagnosticBag> {
        self.check_circular_imports(files)?;
        let mut results = Vec::new();

        for file in files {
            match self.check_file(file) {
                Ok(Some(typed)) => results.push(typed),
                Ok(None) => {}
                Err(_) => {}
            }
        }

        if self.diagnostics.has_errors() {
            Err(self.diagnostics.clone())
        } else {
            Ok(results)
        }
    }

    fn check_circular_imports(&mut self, files: &[SourceFile]) -> Result<(), DiagnosticBag> {
        let mut graph: FxHashMap<String, Vec<String>> = FxHashMap::default();
        for file in files {
            let pkg = file
                .package
                .as_ref()
                .map(|(p, _)| p.join("."))
                .unwrap_or_default();
            let key = if pkg.is_empty() {
                path_stem(&file.path)
            } else {
                format!("{pkg}.{}", path_stem(&file.path))
            };
            let deps: Vec<String> = file.imports.iter().map(|i| i.path.join(".")).collect();
            graph.insert(key, deps);
        }

        for start in graph.keys().cloned().collect::<Vec<_>>() {
            if let Some(path) = find_cycle(&graph, &start) {
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::E120, Span::synthetic(&start))
                        .with_arg("path", path),
                );
                return Err(self.diagnostics.clone());
            }
        }
        Ok(())
    }

    fn check_file(&mut self, file: &SourceFile) -> Result<Option<TypedSourceFile>, DiagnosticBag> {
        let package_name = if let Some((parts, span)) = &file.package {
            let expected = expected_package_from_path(Path::new(&file.path), &self.source_root);
            let found = parts.join(".");
            if let Some(expected_pkg) = expected {
                if expected_pkg != found {
                    self.diagnostics.push(
                        Diagnostic::new(DiagnosticCode::E102, span.clone())
                            .with_arg("expected", expected_pkg)
                            .with_arg("found", found.clone()),
                    );
                }
            }
            found
        } else {
            self.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E010,
                Span::synthetic(&file.path),
            ));
            return Ok(None);
        };

        let mut class_name = None;
        for decl in &file.decls {
            if let TopLevelDecl::Class(c) = decl {
                let file_stem = path_stem(&file.path);
                if file_stem != c.name {
                    self.diagnostics.push(
                        Diagnostic::new(DiagnosticCode::E101, c.span.clone())
                            .with_arg("file", file_stem)
                            .with_arg("class", c.name.clone()),
                    );
                }
                self.check_class_field_order(c);
                class_name = Some(c.name.clone());
            }
        }

        let mut symbols = FxHashMap::default();
        self.collect_symbols(file, &mut symbols);
        self.check_unused(file, &symbols);

        if self.diagnostics.has_errors() {
            return Err(self.diagnostics.clone());
        }

        Ok(Some(TypedSourceFile {
            source: file.clone(),
            package_name,
            class_name,
            symbol_types: symbols,
        }))
    }

    fn check_class_field_order(&mut self, _class: &ClassDecl) {
        // E110 is reported by the parser when a field follows a method.
    }

    fn collect_symbols(&self, file: &SourceFile, symbols: &mut FxHashMap<String, RjType>) {
        for decl in &file.decls {
            match decl {
                TopLevelDecl::Let { name, ty, value, .. }
                | TopLevelDecl::Var { name, ty, value, .. } => {
                    let t = ty
                        .as_ref()
                        .map(RjType::from_ast)
                        .unwrap_or_else(|| infer_expr_type(value));
                    symbols.insert(name.clone(), t);
                }
                TopLevelDecl::Fn(f) => {
                    symbols.insert(f.name.clone(), RjType::Void);
                }
                TopLevelDecl::Class(c) => {
                    symbols.insert(c.name.clone(), RjType::Class(c.name.clone()));
                }
                TopLevelDecl::Enum(e) => {
                    symbols.insert(e.name.clone(), RjType::Enum(e.name.clone()));
                }
                TopLevelDecl::Interface(i) => {
                    symbols.insert(i.name.clone(), RjType::Interface(i.name.clone()));
                }
            }
        }
    }

    fn check_unused(&mut self, file: &SourceFile, symbols: &FxHashMap<String, RjType>) {
        let mut used = HashSet::new();
        collect_used_idents(file, &mut used);

        for imp in &file.imports {
            let name = imp
                .alias
                .clone()
                .unwrap_or_else(|| imp.path.last().cloned().unwrap_or_default());
            if !used.contains(&name) && !imp.wildcard {
                self.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::W002,
                    imp.span.clone(),
                ));
            }
        }

        for (name, _) in symbols {
            if !used.contains(name) && name != "main" {
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::W001, Span::synthetic(&file.path))
                        .with_arg("name", name.clone()),
                );
            }
        }
    }
}

fn find_cycle(graph: &FxHashMap<String, Vec<String>>, start: &str) -> Option<String> {
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    let mut path = Vec::new();

    fn dfs(
        node: &str,
        graph: &FxHashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<String> {
        if stack.contains(node) {
            if !path.contains(&node.to_string()) {
                path.push(node.to_string());
            }
            return Some(path.join(" -> "));
        }
        if visited.contains(node) {
            return None;
        }
        visited.insert(node.to_string());
        stack.insert(node.to_string());
        path.push(node.to_string());
        if let Some(children) = graph.get(node) {
            for child in children {
                if let Some(cycle) = dfs(child, graph, visited, stack, path) {
                    return Some(cycle);
                }
            }
        }
        path.pop();
        stack.remove(node);
        None
    }

    dfs(start, graph, &mut visited, &mut stack, &mut path)
}

fn path_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

fn expected_package_from_path(path: &Path, _source_root: &Path) -> Option<String> {
    let components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy()).collect();
    if let Some(pos) = components.iter().position(|c| c == "rj") {
        let after: Vec<_> = components.iter().skip(pos + 1).collect();
        if after.len() > 1 {
            let pkg_parts: Vec<_> = after[..after.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            return Some(pkg_parts.join("."));
        }
    }
    None
}

fn infer_expr_type(expr: &Expr) -> RjType {
    match expr {
        Expr::IntLit { .. } => RjType::Int,
        Expr::FloatLit { .. } => RjType::Float,
        Expr::StringLit { .. } => RjType::Str,
        Expr::BoolLit { .. } => RjType::Bool,
        _ => RjType::Void,
    }
}

fn collect_used_idents(file: &SourceFile, used: &mut HashSet<String>) {
    for decl in &file.decls {
        walk_decl_uses(decl, used);
    }
}

fn walk_decl_uses(decl: &TopLevelDecl, used: &mut HashSet<String>) {
    match decl {
        TopLevelDecl::Let { value, .. } | TopLevelDecl::Var { value, .. } => {
            walk_expr_uses(value, used);
        }
        TopLevelDecl::Fn(f) => {
            for stmt in &f.body {
                walk_stmt_uses(stmt, used);
            }
            if let Some(tail) = &f.tail_expr {
                walk_expr_uses(tail, used);
            }
        }
        TopLevelDecl::Class(c) => {
            for m in &c.methods {
                for stmt in &m.body {
                    walk_stmt_uses(stmt, used);
                }
                if let Some(tail) = &m.tail_expr {
                    walk_expr_uses(tail, used);
                }
            }
        }
        _ => {}
    }
}

fn walk_stmt_uses(stmt: &Stmt, used: &mut HashSet<String>) {
    match stmt {
        Stmt::Expr { expr, .. } => walk_expr_uses(expr, used),
        Stmt::Let { value, .. } => walk_expr_uses(value, used),
        Stmt::Return { value: Some(v), .. } => walk_expr_uses(v, used),
        Stmt::Throw { value, .. } => walk_expr_uses(value, used),
        Stmt::While { cond, body, .. } => {
            walk_expr_uses(cond, used);
            walk_expr_uses(body, used);
        }
        Stmt::Loop { body, .. } => walk_expr_uses(body, used),
        Stmt::Break { .. } => {}
        Stmt::For { iter, body, .. } => {
            walk_expr_uses(iter, used);
            walk_expr_uses(body, used);
        }
        _ => {}
    }
}

fn walk_expr_uses(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Ident { name, .. } => {
            used.insert(name.clone());
        }
        Expr::Binary { left, right, .. } => {
            walk_expr_uses(left, used);
            walk_expr_uses(right, used);
        }
        Expr::Unary { expr, .. } => walk_expr_uses(expr, used),
        Expr::Call { callee, args, .. } => {
            walk_expr_uses(callee, used);
            for a in args {
                walk_expr_uses(a, used);
            }
        }
        Expr::Member { object, .. } => walk_expr_uses(object, used),
        Expr::Index { object, index, .. } => {
            walk_expr_uses(object, used);
            walk_expr_uses(index, used);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            walk_expr_uses(cond, used);
            walk_expr_uses(then_branch, used);
            if let Some(e) = else_branch {
                walk_expr_uses(e, used);
            }
        }
        Expr::Match { scrutinee, arms, .. } => {
            walk_expr_uses(scrutinee, used);
            for arm in arms {
                walk_expr_uses(&arm.body, used);
            }
        }
        Expr::Block { stmts, tail, .. } => {
            for s in stmts {
                walk_stmt_uses(s, used);
            }
            if let Some(t) = tail {
                walk_expr_uses(t, used);
            }
        }
        Expr::Lambda { body, .. } => walk_expr_uses(body, used),
        Expr::Await { expr, .. } | Expr::Try { expr, .. } => walk_expr_uses(expr, used),
        Expr::Comprehension { iter, cond, .. } => {
            walk_expr_uses(iter, used);
            if let Some(c) = cond {
                walk_expr_uses(c, used);
            }
        }
        Expr::Assign { target, value, .. } => {
            walk_expr_uses(target, used);
            walk_expr_uses(value, used);
        }
        _ => {}
    }
}
