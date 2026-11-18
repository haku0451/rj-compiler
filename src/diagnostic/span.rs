#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub len: u32,
}

impl Span {
    pub fn new(file: impl Into<String>, line: u32, column: u32, len: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            len,
        }
    }

    pub fn synthetic(file: impl Into<String>) -> Self {
        Self::new(file, 1, 1, 0)
    }
}
