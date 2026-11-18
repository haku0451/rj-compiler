use crate::ast::*;
use crate::diagnostic::{Diagnostic, DiagnosticBag, DiagnosticCode, Span};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    file: String,
    diagnostics: DiagnosticBag,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: impl Into<String>) -> Self {
        Self {
            tokens,
            pos: 0,
            file: file.into(),
            diagnostics: DiagnosticBag::new(),
        }
    }

    pub fn parse_file(mut self) -> Result<SourceFile, DiagnosticBag> {
        let path = self.file.clone();
        let mut package = None;
        let mut imports = Vec::new();
        let mut decls = Vec::new();

        if self.check(&TokenKind::Package) {
            package = Some(self.parse_package()?);
        }

        while self.check(&TokenKind::Import) {
            imports.push(self.parse_import()?);
        }

        while !self.is_at_end() {
            decls.push(self.parse_top_level_decl()?);
        }

        if package.is_none() {
            self.error(
                DiagnosticCode::E010,
                Span::synthetic(&path),
            );
        }

        if self.diagnostics.has_errors() {
            return Err(self.diagnostics);
        }

        Ok(SourceFile {
            path,
            package,
            imports,
            decls,
        })
    }

    fn parse_package(&mut self) -> Result<(Vec<String>, Span), DiagnosticBag> {
        let start = self.current_span();
        self.expect(&TokenKind::Package)?;
        let path = self.parse_qualified_name()?;
        let span = start;
        Ok((path, span))
    }

    fn parse_import(&mut self) -> Result<ImportDecl, DiagnosticBag> {
        let span = self.current_span();
        self.expect(&TokenKind::Import)?;
        let path = self.parse_qualified_name()?;
        let mut alias = None;
        let mut wildcard = false;

        if self.check(&TokenKind::As) {
            self.advance();
            alias = Some(self.expect_ident()?);
        } else if self.check(&TokenKind::Star) {
            self.advance();
            wildcard = true;
        }

        Ok(ImportDecl {
            path,
            alias,
            wildcard,
            span,
        })
    }

    fn parse_top_level_decl(&mut self) -> Result<TopLevelDecl, DiagnosticBag> {
        if self.check(&TokenKind::Let) {
            return self.parse_let_decl(false);
        }
        if self.check(&TokenKind::Var) {
            return self.parse_let_decl(true);
        }
        if self.check(&TokenKind::Async) || self.check(&TokenKind::Def) {
            return Ok(TopLevelDecl::Fn(self.parse_fn_decl(false)?));
        }
        if self.check(&TokenKind::Class) {
            return Ok(TopLevelDecl::Class(self.parse_class_decl()?));
        }
        if self.check(&TokenKind::Interface) {
            return Ok(TopLevelDecl::Interface(self.parse_interface_decl()?));
        }
        if self.check(&TokenKind::Enum) {
            return Ok(TopLevelDecl::Enum(self.parse_enum_decl()?));
        }

        self.error_at_current(
            DiagnosticCode::E200,
            "expected top-level declaration",
            None,
        );
        Err(self.diagnostics.clone())
    }

    fn parse_let_decl(&mut self, mutable: bool) -> Result<TopLevelDecl, DiagnosticBag> {
        let span = self.current_span();
        if mutable {
            self.expect(&TokenKind::Var)?;
        } else {
            self.expect(&TokenKind::Let)?;
        }
        let name = self.expect_ident()?;
        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr()?;

        if mutable {
            Ok(TopLevelDecl::Var {
                name,
                ty,
                value,
                span,
            })
        } else {
            Ok(TopLevelDecl::Let {
                name,
                ty,
                value,
                mutable: false,
                span,
            })
        }
    }

    fn parse_visibility(&mut self) -> Visibility {
        if self.check(&TokenKind::Public) {
            self.advance();
            Visibility::Public
        } else if self.check(&TokenKind::Private) {
            self.advance();
            Visibility::Private
        } else if self.check(&TokenKind::Protected) {
            self.advance();
            Visibility::Protected
        } else {
            Visibility::Public
        }
    }

    fn parse_fn_decl(&mut self, in_class: bool) -> Result<FnDecl, DiagnosticBag> {
        let span = self.current_span();
        let visibility = if in_class {
            self.parse_visibility()
        } else {
            Visibility::Public
        };
        let mut is_static = false;
        if self.check(&TokenKind::Static) {
            self.advance();
            is_static = true;
        }
        let is_async = if self.check(&TokenKind::Async) {
            self.advance();
            true
        } else {
            false
        };
        self.expect(&TokenKind::Def)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;
        let ret_ty = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::LBrace)?;
        let (body, tail_expr) = self.parse_block_contents()?;
        self.expect(&TokenKind::RBrace)?;

        Ok(FnDecl {
            name,
            params,
            ret_ty,
            body,
            tail_expr,
            visibility,
            is_static,
            is_async,
            span,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, DiagnosticBag> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let span = self.current_span();
                let name = self.expect_ident()?;
                let ty = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };
                params.push(Param { name, ty, span });
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        Ok(params)
    }

    fn parse_class_decl(&mut self) -> Result<ClassDecl, DiagnosticBag> {
        let span = self.current_span();
        let visibility = self.parse_visibility();
        self.expect(&TokenKind::Class)?;
        let name = self.expect_ident()?;
        let mut super_class = None;
        let mut interfaces = Vec::new();

        if self.check(&TokenKind::LParen) {
            self.advance();
            let first = self.expect_ident()?;
            if self.check(&TokenKind::Comma) {
                interfaces.push(first);
                while self.check(&TokenKind::Comma) {
                    self.advance();
                    interfaces.push(self.expect_ident()?);
                }
            } else {
                super_class = Some(first);
            }
            self.expect(&TokenKind::RParen)?;
        }

        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        let mut saw_method = false;
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::Let) || self.check(&TokenKind::Var) {
                if saw_method {
                    self.error_at_current(
                        DiagnosticCode::E110,
                        "fields must be declared at the beginning of the class",
                        None,
                    );
                }
                fields.push(self.parse_field_decl()?);
            } else {
                saw_method = true;
                methods.push(self.parse_fn_decl(true)?);
            }
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(ClassDecl {
            name,
            fields,
            methods,
            super_class,
            interfaces,
            visibility,
            span,
        })
    }

    fn parse_field_decl(&mut self) -> Result<FieldDecl, DiagnosticBag> {
        let span = self.current_span();
        let visibility = self.parse_visibility();
        let mutable = if self.check(&TokenKind::Var) {
            self.advance();
            true
        } else {
            self.expect(&TokenKind::Let)?;
            false
        };
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(FieldDecl {
            name,
            ty,
            mutable,
            visibility,
            span,
        })
    }

    fn parse_interface_decl(&mut self) -> Result<InterfaceDecl, DiagnosticBag> {
        let span = self.current_span();
        let visibility = self.parse_visibility();
        self.expect(&TokenKind::Interface)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let mspan = self.current_span();
            let visibility = self.parse_visibility();
            self.expect(&TokenKind::Def)?;
            let name = self.expect_ident()?;
            self.expect(&TokenKind::LParen)?;
            let params = self.parse_params()?;
            self.expect(&TokenKind::RParen)?;
            let ret_ty = if self.check(&TokenKind::Arrow) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };
            methods.push(FnDecl {
                name,
                params,
                ret_ty,
                body: vec![],
                tail_expr: None,
                visibility,
                is_static: false,
                is_async: false,
                span: mspan,
            });
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(InterfaceDecl {
            name,
            methods,
            super_interfaces: vec![],
            visibility,
            span,
        })
    }

    fn parse_enum_decl(&mut self) -> Result<EnumDecl, DiagnosticBag> {
        let span = self.current_span();
        let visibility = self.parse_visibility();
        self.expect(&TokenKind::Enum)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let vspan = self.current_span();
            let vname = self.expect_ident()?;
            variants.push(EnumVariant {
                name: vname,
                span: vspan,
            });
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(EnumDecl {
            name,
            variants,
            visibility,
            span,
        })
    }

    fn parse_block_contents(&mut self) -> Result<(Vec<Stmt>, Option<Expr>), DiagnosticBag> {
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.is_block_stmt_start() {
                stmts.push(self.parse_stmt()?);
                continue;
            }
            let span = self.current_span();
            let expr = self.parse_expr()?;
            if self.check(&TokenKind::RBrace) {
                return Ok((stmts, Some(expr)));
            }
            stmts.push(Stmt::Expr { expr, span });
        }
        Ok((stmts, None))
    }

    fn is_block_stmt_start(&self) -> bool {
        matches!(
            self.peek_kind(),
            Some(
                TokenKind::Let
                    | TokenKind::Var
                    | TokenKind::Return
                    | TokenKind::Throw
                    | TokenKind::While
                    | TokenKind::Loop
                    | TokenKind::For
                    | TokenKind::Break
            )
        )
    }

    fn parse_stmt(&mut self) -> Result<Stmt, DiagnosticBag> {
        let span = self.current_span();
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Var) {
            let mutable = if self.check(&TokenKind::Var) {
                self.advance();
                true
            } else {
                self.advance();
                false
            };
            let name = self.expect_ident()?;
            let ty = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(&TokenKind::Eq)?;
            let value = self.parse_expr()?;
            return Ok(Stmt::Let {
                name,
                ty,
                value,
                mutable,
                span,
            });
        }
        if self.check(&TokenKind::Return) {
            self.advance();
            let value = if self.is_expr_start() {
                Some(self.parse_expr()?)
            } else {
                None
            };
            return Ok(Stmt::Return { value, span });
        }
        if self.check(&TokenKind::Throw) {
            self.advance();
            let value = self.parse_expr()?;
            return Ok(Stmt::Throw { value, span });
        }
        if self.check(&TokenKind::While) {
            self.advance();
            let cond = self.parse_expr()?;
            let body = self.parse_block_expr()?;
            return Ok(Stmt::While { cond, body, span });
        }
        if self.check(&TokenKind::Loop) {
            self.advance();
            let body = self.parse_block_expr()?;
            return Ok(Stmt::Loop { body, span });
        }
        if self.check(&TokenKind::Break) {
            self.advance();
            return Ok(Stmt::Break { span });
        }
        if self.check(&TokenKind::For) {
            self.advance();
            let var = self.expect_ident()?;
            self.expect(&TokenKind::In)?;
            let iter = self.parse_expr()?;
            let body = self.parse_block_expr()?;
            return Ok(Stmt::For { var, iter, body, span });
        }

        let expr = self.parse_expr()?;
        Ok(Stmt::Expr { expr, span })
    }

    fn parse_block_expr(&mut self) -> Result<Expr, DiagnosticBag> {
        let span = self.current_span();
        self.expect(&TokenKind::LBrace)?;
        let (stmts, tail) = self.parse_block_contents()?;
        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Block {
            stmts,
            tail: tail.map(Box::new),
            span,
        })
    }

    fn parse_type(&mut self) -> Result<Type, DiagnosticBag> {
        let span = self.current_span();
        if self.check(&TokenKind::Result) {
            self.advance();
            self.expect(&TokenKind::Lt)?;
            let ok = self.parse_type()?;
            self.expect(&TokenKind::Comma)?;
            let err = self.parse_type()?;
            self.expect(&TokenKind::Gt)?;
            return Ok(Type::Generic {
                name: "Result".into(),
                args: vec![ok, err],
                span,
            });
        }

        let name = self.parse_type_name()?;
        let mut ty = Type::Named {
            name,
            span: span.clone(),
        };

        if self.check(&TokenKind::Question) {
            self.advance();
            ty = Type::Nullable {
                inner: Box::new(ty),
                span: span.clone(),
            };
        }

        if self.check(&TokenKind::LBracket) {
            self.advance();
            self.expect(&TokenKind::RBracket)?;
            ty = Type::Array {
                element: Box::new(ty),
                span,
            };
        }

        Ok(ty)
    }

    fn parse_type_name(&mut self) -> Result<String, DiagnosticBag> {
        if let Some(kind) = self.peek_kind().cloned() {
            let name = match kind {
                TokenKind::Str => "str".into(),
                TokenKind::Int => "int".into(),
                TokenKind::Float => "float".into(),
                TokenKind::Bool => "bool".into(),
                TokenKind::Ident(n) => n,
                _ => {
                    self.error_at_current(DiagnosticCode::E200, "expected type name", None);
                    return Err(self.diagnostics.clone());
                }
            };
            self.advance();
            Ok(name)
        } else {
            self.error_at_current(DiagnosticCode::E200, "expected type name", None);
            Err(self.diagnostics.clone())
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, DiagnosticBag> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr, DiagnosticBag> {
        let expr = self.parse_try()?;
        if self.check(&TokenKind::Eq) {
            let span = self.current_span();
            self.advance();
            let value = self.parse_assign()?;
            return Ok(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
                span,
            });
        }
        Ok(expr)
    }

    fn parse_try(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut expr = self.parse_or()?;
        if self.check(&TokenKind::Question) {
            let span = self.current_span();
            self.advance();
            expr = Expr::Try {
                expr: Box::new(expr),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::OrOr) {
            let span = self.current_span();
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::AndAnd) {
            let span = self.current_span();
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = if self.check(&TokenKind::EqEq) {
                self.advance();
                BinOp::Eq
            } else if self.check(&TokenKind::Ne) {
                self.advance();
                BinOp::Ne
            } else {
                break;
            };
            let span = self.current_span();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut left = self.parse_additive()?;
        loop {
            let op = if self.check(&TokenKind::Lt) {
                self.advance();
                BinOp::Lt
            } else if self.check(&TokenKind::Gt) {
                self.advance();
                BinOp::Gt
            } else if self.check(&TokenKind::Le) {
                self.advance();
                BinOp::Le
            } else if self.check(&TokenKind::Ge) {
                self.advance();
                BinOp::Ge
            } else {
                break;
            };
            let span = self.current_span();
            let right = self.parse_additive()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.check(&TokenKind::Plus) {
                self.advance();
                BinOp::Add
            } else if self.check(&TokenKind::Minus) {
                self.advance();
                BinOp::Sub
            } else if self.check(&TokenKind::Dot)
                && Self::is_string_concat_expr(&left)
                && self.is_string_concat_lookahead()
            {
                self.advance();
                BinOp::Concat
            } else {
                break;
            };
            let span = self.current_span();
            let right = if matches!(op, BinOp::Concat) {
                self.parse_concat_operand()?
            } else {
                self.parse_multiplicative()?
            };
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    /// 文字列連結の右辺: `.` は次の連結トークン用に残すためメンバーアクセスを使わない
    fn parse_concat_operand(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.check(&TokenKind::LParen) {
                let span = self.current_span();
                self.advance();
                let args = self.parse_call_args()?;
                self.expect(&TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn is_string_concat_expr(expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::StringLit { .. } | Expr::Binary { op: BinOp::Concat, .. }
        )
    }

    fn is_string_concat_lookahead(&self) -> bool {
        matches!(
            self.peek_ahead(1).map(|t| &t.kind),
            Some(TokenKind::StringLit(_) | TokenKind::Ident(_) | TokenKind::LParen)
        )
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.check(&TokenKind::Star) {
                self.advance();
                BinOp::Mul
            } else if self.check(&TokenKind::Slash) {
                self.advance();
                BinOp::Div
            } else if self.check(&TokenKind::Percent) {
                self.advance();
                BinOp::Mod
            } else {
                break;
            };
            let span = self.current_span();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, DiagnosticBag> {
        if self.check(&TokenKind::Bang) {
            let span = self.current_span();
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                span,
            });
        }
        if self.check(&TokenKind::Minus) {
            let span = self.current_span();
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
                span,
            });
        }
        if self.check(&TokenKind::Await) {
            let span = self.current_span();
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Await {
                expr: Box::new(expr),
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::Dot) {
                if Self::is_string_concat_expr(&expr) && self.is_string_concat_lookahead() {
                    break;
                }
                let span = self.current_span();
                self.advance();
                let name = self.expect_ident()?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    name,
                    span,
                };
            } else if self.check(&TokenKind::LParen) {
                let span = self.current_span();
                self.advance();
                let args = self.parse_call_args()?;
                self.expect(&TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else if self.check(&TokenKind::LBracket) {
                if matches!(expr, Expr::Ident { .. }) && self.check_ahead(1, &TokenKind::For) {
                    return self.parse_comprehension();
                }
                let span = self.current_span();
                self.advance();
                let index = self.parse_expr()?;
                self.expect(&TokenKind::RBracket)?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_comprehension(&mut self) -> Result<Expr, DiagnosticBag> {
        let span = self.current_span();
        self.expect(&TokenKind::LBracket)?;
        let elem = self.parse_expr()?;
        self.expect(&TokenKind::For)?;
        let var = self.expect_ident()?;
        self.expect(&TokenKind::In)?;
        let iter = self.parse_expr()?;
        let cond = if self.check(&TokenKind::If) {
            self.advance();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        self.expect(&TokenKind::RBracket)?;
        Ok(Expr::Comprehension {
            elem: Box::new(elem),
            var,
            iter: Box::new(iter),
            cond,
            span,
        })
    }

    fn parse_primary(&mut self) -> Result<Expr, DiagnosticBag> {
        let span = self.current_span();
        if self.check(&TokenKind::If) {
            return self.parse_if_expr();
        }
        if self.check(&TokenKind::Match) {
            return self.parse_match_expr();
        }
        if matches!(self.peek_kind(), Some(TokenKind::IntLit(_))) {
            if let TokenKind::IntLit(v) = self.advance().kind {
                return Ok(Expr::IntLit { value: v, span });
            }
        }
        if matches!(self.peek_kind(), Some(TokenKind::FloatLit(_))) {
            if let TokenKind::FloatLit(v) = self.advance().kind {
                return Ok(Expr::FloatLit { value: v, span });
            }
        }
        if matches!(self.peek_kind(), Some(TokenKind::StringLit(_))) {
            if let TokenKind::StringLit(v) = self.advance().kind {
                let unquoted = unquote_string(&v);
                return Ok(Expr::StringLit {
                    value: unquoted,
                    span,
                });
            }
        }
        if self.check(&TokenKind::True) {
            self.advance();
            return Ok(Expr::BoolLit { value: true, span });
        }
        if self.check(&TokenKind::False) {
            self.advance();
            return Ok(Expr::BoolLit { value: false, span });
        }
        if self.check(&TokenKind::SelfKw) {
            self.advance();
            return Ok(Expr::SelfExpr { span });
        }
        if self.check(&TokenKind::Result) {
            self.advance();
            return Ok(Expr::Ident {
                name: "Result".into(),
                span,
            });
        }
        if matches!(self.peek_kind(), Some(TokenKind::Ident(_))) {
            let name = self.expect_ident()?;
            return Ok(Expr::Ident { name, span });
        }
        if self.check(&TokenKind::LParen) {
            self.advance();
            if self.check(&TokenKind::RParen) || matches!(self.peek_kind(), Some(TokenKind::Ident(_))) {
                // Lambda or grouped expr
                if self.check(&TokenKind::RParen) || self.peek_ahead(1).map(|t| matches!(t.kind, TokenKind::Colon | TokenKind::FatArrow | TokenKind::RParen)).unwrap_or(false) {
                    return self.parse_lambda_from_parens(span);
                }
            }
            let expr = self.parse_expr()?;
            self.expect(&TokenKind::RParen)?;
            return Ok(expr);
        }
        if self.check(&TokenKind::LBracket) {
            return self.parse_list_or_comprehension(span);
        }
        if self.check(&TokenKind::LBrace) {
            return self.parse_block_expr();
        }

        self.error_at_current(DiagnosticCode::E200, "expected expression", None);
        Err(self.diagnostics.clone())
    }

    fn parse_lambda_from_parens(&mut self, span: Span) -> Result<Expr, DiagnosticBag> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let pspan = self.current_span();
                let name = self.expect_ident()?;
                let ty = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };
                params.push(Param { name, ty, span: pspan });
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::FatArrow)?;
        let body = self.parse_expr()?;
        Ok(Expr::Lambda {
            params,
            body: Box::new(body),
            span,
        })
    }

    fn parse_list_or_comprehension(&mut self, span: Span) -> Result<Expr, DiagnosticBag> {
        self.expect(&TokenKind::LBracket)?;
        if self.check(&TokenKind::For) {
            self.error_at_current(DiagnosticCode::E200, "invalid comprehension", None);
            return Err(self.diagnostics.clone());
        }
        let elem = self.parse_expr()?;
        if self.check(&TokenKind::For) {
            self.advance();
            let var = self.expect_ident()?;
            self.expect(&TokenKind::In)?;
            let iter = self.parse_expr()?;
            let cond = if self.check(&TokenKind::If) {
                self.advance();
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            self.expect(&TokenKind::RBracket)?;
            return Ok(Expr::Comprehension {
                elem: Box::new(elem),
                var,
                iter: Box::new(iter),
                cond,
                span,
            });
        }
        self.expect(&TokenKind::RBracket)?;
        // Treat as index on implicit list - for array literals use Call to listOf in semantic
        Ok(elem)
    }

    fn parse_if_expr(&mut self) -> Result<Expr, DiagnosticBag> {
        let span = self.current_span();
        self.expect(&TokenKind::If)?;
        let cond = self.parse_expr()?;
        let then_branch = self.parse_block_expr()?;
        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            Some(Box::new(self.parse_block_expr()?))
        } else {
            None
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch,
            span,
        })
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, DiagnosticBag> {
        let span = self.current_span();
        if matches!(self.peek_kind(), Some(TokenKind::Ident(name)) if name == "_")
            && !self.check_ahead(1, &TokenKind::Dot)
        {
            self.advance();
            return Ok(MatchPattern::Wildcard { span });
        }
        Ok(MatchPattern::Expr(self.parse_match_pattern_expr()?))
    }

    fn parse_match_pattern_expr(&mut self) -> Result<Expr, DiagnosticBag> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.check(&TokenKind::Dot) {
                let span = self.current_span();
                self.advance();
                let name = self.expect_ident()?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    name,
                    span,
                };
            } else if self.check(&TokenKind::LParen) {
                let span = self.current_span();
                self.advance();
                let args = self.parse_call_args()?;
                self.expect(&TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_match_expr(&mut self) -> Result<Expr, DiagnosticBag> {
        let span = self.current_span();
        self.expect(&TokenKind::Match)?;
        let scrutinee = self.parse_expr()?;
        self.expect(&TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let span = self.current_span();
            self.expect(&TokenKind::Case)?;
            let pattern = self.parse_match_pattern()?;
            let body = self.parse_block_expr()?;
            arms.push(MatchArm {
                pattern,
                body,
                span,
            });
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span,
        })
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, DiagnosticBag> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                if matches!(self.peek_kind(), Some(TokenKind::Ident(_))) && self.check_ahead(1, &TokenKind::FatArrow) {
                    // Single-arg lambda in call position: x => expr
                    let span = self.current_span();
                    let name = self.expect_ident()?;
                    self.expect(&TokenKind::FatArrow)?;
                    let body = self.parse_expr()?;
                    args.push(Expr::Lambda {
                        params: vec![Param {
                            name,
                            ty: None,
                            span: span.clone(),
                        }],
                        body: Box::new(body),
                        span,
                    });
                } else {
                    args.push(self.parse_expr()?);
                }
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        Ok(args)
    }

    fn parse_qualified_name(&mut self) -> Result<Vec<String>, DiagnosticBag> {
        let mut parts = vec![self.expect_ident()?];
        while self.check(&TokenKind::Dot) {
            self.advance();
            if self.check(&TokenKind::Star) {
                self.advance();
                break;
            }
            parts.push(self.expect_ident()?);
        }
        Ok(parts)
    }

    // --- helpers ---

    fn is_expr_start(&self) -> bool {
        matches!(
            self.peek_kind(),
            Some(
                TokenKind::IntLit(_)
                    | TokenKind::FloatLit(_)
                    | TokenKind::StringLit(_)
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Ident(_)
                    | TokenKind::SelfKw
                    | TokenKind::If
                    | TokenKind::Match
                    | TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::Bang
                    | TokenKind::Minus
                    | TokenKind::Await
            )
        )
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<(), DiagnosticBag> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            self.error_at_current(
                DiagnosticCode::E200,
                &format!("expected {:?}", kind),
                None,
            );
            Err(self.diagnostics.clone())
        }
    }

    fn expect_ident(&mut self) -> Result<String, DiagnosticBag> {
        if let Some(TokenKind::Ident(name)) = self.peek_kind().cloned() {
            self.advance();
            Ok(name)
        } else {
            self.error_at_current(DiagnosticCode::E200, "expected identifier", None);
            Err(self.diagnostics.clone())
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind().map(|k| discriminant_eq(k, kind)).unwrap_or(false)
    }

    fn check_ahead(&self, offset: usize, kind: &TokenKind) -> bool {
        self.peek_ahead(offset)
            .map(|t| discriminant_eq(&t.kind, kind))
            .unwrap_or(false)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }

    fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if !self.is_at_end() {
            self.pos += 1;
        }
        tok
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn current_span(&self) -> Span {
        if let Some(tok) = self.tokens.get(self.pos) {
            Span::new(&self.file, tok.line, tok.column, tok.len)
        } else if let Some(tok) = self.tokens.last() {
            Span::new(&self.file, tok.line, tok.column, tok.len)
        } else {
            Span::synthetic(&self.file)
        }
    }

    fn error(&mut self, code: DiagnosticCode, span: Span) {
        self.diagnostics.push(Diagnostic::new(code, span));
    }

    fn error_at_current(&mut self, code: DiagnosticCode, detail: &str, hint: Option<&str>) {
        let mut d = Diagnostic::new(code, self.current_span()).with_arg("detail", detail);
        if let Some(h) = hint {
            d = d.with_arg("hint", h);
        }
        self.diagnostics.push(d);
    }
}

fn discriminant_eq(a: &TokenKind, b: &TokenKind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

fn unquote_string(s: &str) -> String {
    let inner = &s[1..s.len() - 1];
    inner
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}
