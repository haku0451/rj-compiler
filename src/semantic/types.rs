use crate::ast::Type as AstType;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RjType {
    Void,
    Int,
    Float,
    Bool,
    Str,
    Nullable(Box<RjType>),
    Array(Box<RjType>),
    Named(String),
    Result(Box<RjType>, Box<RjType>),
    Class(String),
    Enum(String),
    Interface(String),
    Lambda { params: Vec<RjType>, ret: Box<RjType> },
    Error,
}

impl RjType {
    pub fn from_ast(ty: &AstType) -> Self {
        match ty {
            AstType::Named { name, .. } => match name.as_str() {
                "int" => Self::Int,
                "float" => Self::Float,
                "bool" => Self::Bool,
                "str" => Self::Str,
                other => Self::Named(other.to_string()),
            },
            AstType::Nullable { inner, .. } => Self::Nullable(Box::new(Self::from_ast(inner))),
            AstType::Array { element, .. } => Self::Array(Box::new(Self::from_ast(element))),
            AstType::Generic { name, args, .. } if name == "Result" && args.len() == 2 => {
                Self::Result(
                    Box::new(Self::from_ast(&args[0])),
                    Box::new(Self::from_ast(&args[1])),
                )
            }
            AstType::Generic { name, .. } => Self::Named(name.clone()),
            AstType::Void { .. } => Self::Void,
        }
    }

    pub fn jvm_descriptor(&self) -> String {
        match self {
            Self::Void => "V".into(),
            Self::Int => "I".into(),
            Self::Float => "F".into(),
            Self::Bool => "Z".into(),
            Self::Str => "Ljava/lang/String;".into(),
            Self::Nullable(inner) => inner.jvm_descriptor(),
            Self::Array(inner) => format!("[{}", inner.jvm_descriptor()),
            Self::Named(name) | Self::Class(name) | Self::Enum(name) | Self::Interface(name) => {
                format!("L{};", name.replace('.', "/"))
            }
            Self::Result(_, _) => "Lrj/runtime/Result;".into(),
            Self::Lambda { .. } | Self::Error => "Ljava/lang/Object;".into(),
        }
    }

    pub fn jvm_internal_name(&self) -> String {
        match self {
            Self::Named(n) | Self::Class(n) | Self::Enum(n) | Self::Interface(n) => {
                n.replace('.', "/")
            }
            Self::Str => "java/lang/String".into(),
            Self::Result(_, _) => "rj/runtime/Result".into(),
            _ => "java/lang/Object".into(),
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }

    pub fn is_assignable_from(&self, other: &Self) -> bool {
        if self == other {
            return true;
        }
        if let Self::Nullable(inner) = self {
            return matches!(other, Self::Nullable(_)) || inner.is_assignable_from(other);
        }
        if let (Self::Int, Self::Float) = (self, other) {
            return true;
        }
        false
    }
}

impl fmt::Display for RjType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Void => write!(f, "void"),
            Self::Int => write!(f, "int"),
            Self::Float => write!(f, "float"),
            Self::Bool => write!(f, "bool"),
            Self::Str => write!(f, "str"),
            Self::Nullable(inner) => write!(f, "{inner}?"),
            Self::Array(inner) => write!(f, "[{inner}]"),
            Self::Named(n) | Self::Class(n) | Self::Enum(n) | Self::Interface(n) => write!(f, "{n}"),
            Self::Result(ok, err) => write!(f, "Result<{ok}, {err}>"),
            Self::Lambda { params, ret } => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
            }
            Self::Error => write!(f, "<error>"),
        }
    }
}
