use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Ja,
    En,
}

impl Lang {
    pub fn detect() -> Self {
        env::var("LANG")
            .or_else(|_| env::var("LC_ALL"))
            .or_else(|_| env::var("LC_MESSAGES"))
            .map(|v| {
                if v.to_ascii_lowercase().starts_with("ja") {
                    Self::Ja
                } else {
                    Self::En
                }
            })
            .unwrap_or(Self::Ja)
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "ja" | "ja-jp" => Some(Self::Ja),
            "en" | "en-us" => Some(Self::En),
            _ => None,
        }
    }
}
