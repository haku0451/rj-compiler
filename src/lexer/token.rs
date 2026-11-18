use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
pub enum TokenKind {
    // Keywords
    #[token("package")]
    Package,
    #[token("import")]
    Import,
    #[token("as")]
    As,
    #[token("let")]
    Let,
    #[token("var")]
    Var,
    #[token("def")]
    Def,
    #[token("class")]
    Class,
    #[token("interface")]
    Interface,
    #[token("enum")]
    Enum,
    #[token("static")]
    Static,
    #[token("public")]
    Public,
    #[token("private")]
    Private,
    #[token("protected")]
    Protected,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("match")]
    Match,
    #[token("case")]
    Case,
    #[token("loop")]
    Loop,
    #[token("break")]
    Break,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("return")]
    Return,
    #[token("throw")]
    Throw,
    #[token("self")]
    SelfKw,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("new")]
    New,
    #[token("from")]
    From,

    // Types
    #[token("str")]
    Str,
    #[token("int")]
    Int,
    #[token("float")]
    Float,
    #[token("bool")]
    Bool,
    #[token("Result")]
    Result,

    // Literals — avoid `.*` / `[^\n]*`; use bounded char classes only
    #[regex(r#""(?:[^"\\]|\\["\\/bfnrt]|\\u[0-9A-Fa-f]{4})*""#, |lex| lex.slice().to_string())]
    StringLit(String),
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().unwrap())]
    IntLit(i64),
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().unwrap())]
    FloatLit(f64),

    // Identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(".")]
    Dot,

    // Operators
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("?")]
    Question,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Ne,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("!")]
    Bang,
    #[token("=")]
    Eq,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: u32,
    pub column: u32,
    pub len: u32,
}

impl Token {
    pub fn span_len(&self) -> u32 {
        self.len
    }
}

pub const UNKNOWN_KEYWORDS: &[(&str, &str)] = &[
    ("from", "import java.util.List"),
    ("new", "User()"),
];
