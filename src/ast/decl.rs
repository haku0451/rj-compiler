use super::expr::{Expr, Param};
use super::stmt::Stmt;
use super::types::Type;
use crate::diagnostic::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Public
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub wildcard: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<Type>,
    pub body: Vec<Stmt>,
    pub tail_expr: Option<Expr>,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_async: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<FnDecl>,
    pub super_class: Option<String>,
    pub interfaces: Vec<String>,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: String,
    pub methods: Vec<FnDecl>,
    pub super_interfaces: Vec<String>,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopLevelDecl {
    Let {
        name: String,
        ty: Option<Type>,
        value: Expr,
        mutable: bool,
        span: Span,
    },
    Var {
        name: String,
        ty: Option<Type>,
        value: Expr,
        span: Span,
    },
    Fn(FnDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Enum(EnumDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub package: Option<(Vec<String>, Span)>,
    pub imports: Vec<ImportDecl>,
    pub decls: Vec<TopLevelDecl>,
}
