use super::types::Type;
use crate::diagnostic::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Concat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLit { value: i64, span: Span },
    FloatLit { value: f64, span: Span },
    StringLit { value: String, span: Span },
    BoolLit { value: bool, span: Span },
    Ident { name: String, span: Span },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Member {
        object: Box<Expr>,
        name: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Block {
        stmts: Vec<super::stmt::Stmt>,
        tail: Option<Box<Expr>>,
        span: Span,
    },
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
        span: Span,
    },
    Await {
        expr: Box<Expr>,
        span: Span,
    },
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    Comprehension {
        elem: Box<Expr>,
        var: String,
        iter: Box<Expr>,
        cond: Option<Box<Expr>>,
        span: Span,
    },
    SelfExpr { span: Span },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Expr(Expr),
    Wildcard { span: Span },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub span: Span,
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Self::IntLit { span, .. }
            | Self::FloatLit { span, .. }
            | Self::StringLit { span, .. }
            | Self::BoolLit { span, .. }
            | Self::Ident { span, .. }
            | Self::Binary { span, .. }
            | Self::Unary { span, .. }
            | Self::Call { span, .. }
            | Self::Member { span, .. }
            | Self::Index { span, .. }
            | Self::If { span, .. }
            | Self::Match { span, .. }
            | Self::Block { span, .. }
            | Self::Lambda { span, .. }
            | Self::Await { span, .. }
            | Self::Try { span, .. }
            | Self::Comprehension { span, .. }
            | Self::SelfExpr { span }
            | Self::Assign { span, .. } => span,
        }
    }
}
