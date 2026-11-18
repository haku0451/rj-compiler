use crate::diagnostic::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named { name: String, span: Span },
    Nullable { inner: Box<Type>, span: Span },
    Array { element: Box<Type>, span: Span },
    Generic { name: String, args: Vec<Type>, span: Span },
    Void { span: Span },
}

impl Type {
    pub fn span(&self) -> &Span {
        match self {
            Self::Named { span, .. }
            | Self::Nullable { span, .. }
            | Self::Array { span, .. }
            | Self::Generic { span, .. }
            | Self::Void { span } => span,
        }
    }

    pub fn is_nullable(&self) -> bool {
        matches!(self, Self::Nullable { .. })
    }
}
