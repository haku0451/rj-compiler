use std::io::{self, Write};

use super::locale::Lang;

pub fn write_stderr(text: &str, lang: Lang) {
    #[cfg(windows)]
    {
        let encoding = windows_stderr_encoding(lang);
        let (encoded, _, had_errors) = encoding.encode(text);
        debug_assert!(!had_errors, "diagnostic message contains unencodable characters");
        let mut lock = io::stderr().lock();
        let _ = lock.write_all(&encoded);
        let _ = lock.write_all(b"\n");
        return;
    }
    #[cfg(not(windows))]
    {
        let _ = writeln!(io::stderr(), "{text}");
    }
}

#[cfg(windows)]
fn windows_stderr_encoding(lang: Lang) -> &'static encoding_rs::Encoding {
    match lang {
        Lang::Ja => encoding_rs::SHIFT_JIS,
        Lang::En => encoding_rs::UTF_8,
    }
}
