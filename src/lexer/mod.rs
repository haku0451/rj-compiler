mod token;

pub use token::{Token, TokenKind, UNKNOWN_KEYWORDS};

use crate::cast::usize_to_u32;
use crate::diagnostic::{Diagnostic, DiagnosticBag, DiagnosticCode, Span};
use logos::Logos;

pub struct Lexer<'a> {
    source: &'a str,
    file: String,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file: impl Into<String>) -> Self {
        Self {
            source,
            file: file.into(),
        }
    }

    pub fn tokenize(&self) -> Result<Vec<Token>, DiagnosticBag> {
        let source = strip_comments(self.source);
        let mut lexer = TokenKind::lexer(&source);
        let mut tokens = Vec::new();
        let mut diagnostics = DiagnosticBag::new();
        let mut line: u32 = 1;
        let mut column: u32 = 1;

        while let Some(result) = lexer.next() {
            let slice = lexer.slice();
            let len = usize_to_u32(slice.len());

            match result {
                Ok(kind) => {
                    if matches!(kind, TokenKind::From) {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::E020,
                                Span::new(&self.file, line, column, len),
                            )
                            .with_arg("keyword", "from")
                            .with_arg("hint", "import java.util.List"),
                        );
                    }
                    if matches!(kind, TokenKind::New) {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::E130,
                                Span::new(&self.file, line, column, len),
                            )
                            .with_arg("hint", "User()"),
                        );
                    }

                    tokens.push(Token {
                        kind,
                        line,
                        column,
                        len,
                    });
                }
                Err(()) => {
                    diagnostics.push(Diagnostic::new(
                        DiagnosticCode::E200,
                        Span::new(&self.file, line, column, len),
                    ).with_arg("detail", format!("unexpected character '{slice}'")));
                }
            }

            for ch in slice.chars() {
                if ch == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
            }
        }

        if diagnostics.has_errors() {
            Err(diagnostics)
        } else {
            Ok(tokens)
        }
    }
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '/' && matches!(chars.peek(), Some('/')) {
            chars.next();
            while let Some(c) = chars.next() {
                if c == '\n' {
                    out.push(c);
                    break;
                }
            }
        } else if ch == '/' && matches!(chars.peek(), Some('*')) {
            chars.next();
            while let Some(c) = chars.next() {
                if c == '*' && matches!(chars.peek(), Some('/')) {
                    chars.next();
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_package_and_def() {
        let src = r#"package com.example

def main() {
    print("hi")
}"#;
        let tokens = Lexer::new(src, "test.rj").tokenize().unwrap();
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Package)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Def)));
    }
}
