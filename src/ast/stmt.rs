use super::expr::Expr;
use super::types::Type;
use crate::diagnostic::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr { expr: Expr, span: Span },
    Let {
        name: String,
        ty: Option<Type>,
        value: Expr,
        mutable: bool,
        span: Span,
    },
    Return { value: Option<Expr>, span: Span },
    Throw { value: Expr, span: Span },
    While {
        cond: Expr,
        body: Expr,
        span: Span,
    },
    Loop { body: Expr, span: Span },
    Break { span: Span },
    For {
        var: String,
        iter: Expr,
        body: Expr,
        span: Span,
    },
}
