use super::codes::DiagnosticCode;
use super::locale::Lang;
use super::message::format_message;
use super::span::Span;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub span: Span,
    pub args: Vec<(String, String)>,
}

impl Diagnostic {
    pub fn new(code: DiagnosticCode, span: Span) -> Self {
        Self {
            code,
            span,
            args: Vec::new(),
        }
    }

    pub fn with_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.push((key.into(), value.into()));
        self
    }

    pub fn render(&self, lang: Lang) -> String {
        let arg_refs: Vec<(&str, &str)> = self
            .args
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let parts = format_message(self.code, lang, &arg_refs);

        let mut out = format!("{}: {}", self.code.as_str(), parts.primary);

        if self.span.len > 0 || self.span.line > 0 {
            out.push_str("\n\nLocation:\n");
            out.push_str(&format!(
                "  {}:{}:{}",
                self.span.file, self.span.line, self.span.column
            ));
        }

        for (label, value) in &parts.extra {
            if !value.is_empty() {
                out.push_str(&format!("\n\n{label}:\n  {value}"));
            }
        }

        if let Some(hint) = &parts.hint {
            out.push_str("\n\nHint:\n  ");
            out.push_str(hint);
        }

        out
    }
}

#[derive(Debug, Default, Clone)]
pub struct DiagnosticBag {
    pub items: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| !d.code.is_warning())
    }

    pub fn merge(&mut self, other: DiagnosticBag) {
        self.items.extend(other.items);
    }

    pub fn render_all(&self, lang: Lang) -> String {
        self.items
            .iter()
            .map(|d| d.render(lang))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
