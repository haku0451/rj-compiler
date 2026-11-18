#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    E010,
    E020,
    E101,
    E102,
    E110,
    E120,
    E130,
    E200,
    E201,
    E202,
    W001,
    W002,
    W003,
}

impl DiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::E010 => "E010",
            Self::E020 => "E020",
            Self::E101 => "E101",
            Self::E102 => "E102",
            Self::E110 => "E110",
            Self::E120 => "E120",
            Self::E130 => "E130",
            Self::E200 => "E200",
            Self::E201 => "E201",
            Self::E202 => "E202",
            Self::W001 => "W001",
            Self::W002 => "W002",
            Self::W003 => "W003",
        }
    }

    pub fn is_warning(self) -> bool {
        matches!(self, Self::W001 | Self::W002 | Self::W003)
    }
}
