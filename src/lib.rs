pub mod ast;
pub mod cast;
pub mod codegen;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod semantic;

use diagnostic::{Diagnostic, DiagnosticBag, Lang};
use semantic::SemanticChecker;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub lang: Lang,
    pub output_dir: PathBuf,
    pub source_root: PathBuf,
    pub check_only: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            lang: Lang::detect(),
            output_dir: PathBuf::from("build/classes/rj"),
            source_root: PathBuf::from("src/main/rj"),
            check_only: false,
        }
    }
}

#[derive(Debug)]
pub struct CompileOutput {
    pub class_files: Vec<PathBuf>,
    pub warnings: DiagnosticBag,
}

pub fn compile_sources(options: &CompileOptions) -> Result<CompileOutput, DiagnosticBag> {
    let mut all_diagnostics = DiagnosticBag::new();
    let mut source_files = Vec::new();

    for entry in WalkDir::new(&options.source_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rj") {
            let source = fs::read_to_string(path).map_err(|e| {
                let mut bag = DiagnosticBag::new();
                bag.push(Diagnostic::new(
                    diagnostic::DiagnosticCode::E200,
                    diagnostic::Span::synthetic(path.to_string_lossy()),
                ).with_arg("detail", e.to_string()));
                bag
            })?;

            let file_path = path.to_string_lossy().to_string();
            let tokens = match lexer::Lexer::new(&source, &file_path).tokenize() {
                Ok(t) => t,
                Err(bag) => {
                    all_diagnostics.merge(bag);
                    continue;
                }
            };

            let ast = match parser::Parser::new(tokens, &file_path).parse_file() {
                Ok(f) => f,
                Err(bag) => {
                    all_diagnostics.merge(bag);
                    continue;
                }
            };
            source_files.push(ast);
        }
    }

    if all_diagnostics.has_errors() {
        return Err(all_diagnostics);
    }

    let mut checker = SemanticChecker::new(&options.source_root);
    let typed = match checker.check_project(&source_files) {
        Ok(t) => t,
        Err(bag) => {
            all_diagnostics.merge(bag);
            return Err(all_diagnostics);
        }
    };

    let warnings = checker_warnings(&checker);
    all_diagnostics.merge(warnings.clone());

    if options.check_only {
        return Ok(CompileOutput {
            class_files: vec![],
            warnings: all_diagnostics,
        });
    }

    fs::create_dir_all(&options.output_dir).ok();

    use std::collections::HashSet;

    let interface_names: HashSet<String> = typed
        .iter()
        .flat_map(|file| {
            file.source.decls.iter().filter_map(|decl| {
                if let ast::TopLevelDecl::Interface(iface) = decl {
                    Some(iface.name.clone())
                } else {
                    None
                }
            })
        })
        .collect();

    let mut class_files = Vec::new();
    for file in &typed {
        let emitted = codegen::CodeEmitter::emit_file(file, &interface_names);
        for (simple_name, bytes) in emitted {
            let pkg_path = file.package_name.replace('.', "/");
            let out_dir = if pkg_path.is_empty() {
                options.output_dir.clone()
            } else {
                options.output_dir.join(&pkg_path)
            };
            fs::create_dir_all(&out_dir).ok();
            let out_path = out_dir.join(format!("{simple_name}.class"));
            fs::write(&out_path, &bytes).map_err(|e| {
                let mut bag = DiagnosticBag::new();
                bag.push(
                    Diagnostic::new(
                        diagnostic::DiagnosticCode::E200,
                        diagnostic::Span::synthetic(out_path.to_string_lossy()),
                    )
                    .with_arg("detail", e.to_string()),
                );
                bag
            })?;
            class_files.push(out_path);
        }
    }

    Ok(CompileOutput {
        class_files,
        warnings: all_diagnostics,
    })
}

pub fn compile_file(path: &Path, options: &CompileOptions) -> Result<CompileOutput, DiagnosticBag> {
    let mut opts = options.clone();
    if let Some(parent) = path.parent() {
        opts.source_root = parent.to_path_buf();
    }
    compile_sources(&opts)
}

fn checker_warnings(_checker: &SemanticChecker) -> DiagnosticBag {
    DiagnosticBag::new()
}

pub fn render_diagnostics(bag: &DiagnosticBag, lang: Lang) -> String {
    bag.render_all(lang)
}

pub fn print_diagnostics(bag: &DiagnosticBag, lang: Lang) {
    diagnostic::write_stderr(&render_diagnostics(bag, lang), lang);
}
