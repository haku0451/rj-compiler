use super::codes::DiagnosticCode;
use super::locale::Lang;

pub struct MessageParts<'a> {
    pub primary: String,
    pub hint: Option<String>,
    pub extra: Vec<(&'a str, String)>,
}

pub fn format_message<'a>(
    code: DiagnosticCode,
    lang: Lang,
    args: &'a [(&'a str, &'a str)],
) -> MessageParts<'a> {
    let lookup = |key: &str| -> Option<String> {
        args.iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| (*v).to_string())
    };

    match (code, lang) {
        (DiagnosticCode::E010, Lang::Ja) => MessageParts {
            primary: "package宣言が必要です".into(),
            hint: Some("package com.example".into()),
            extra: vec![],
        },
        (DiagnosticCode::E010, Lang::En) => MessageParts {
            primary: "package declaration is required".into(),
            hint: Some("package com.example".into()),
            extra: vec![],
        },
        (DiagnosticCode::E020, Lang::Ja) => MessageParts {
            primary: format!(
                "未知のキーワード '{}'",
                lookup("keyword").unwrap_or_default()
            ),
            hint: lookup("hint"),
            extra: vec![],
        },
        (DiagnosticCode::E020, Lang::En) => MessageParts {
            primary: format!(
                "unknown keyword '{}'",
                lookup("keyword").unwrap_or_default()
            ),
            hint: lookup("hint"),
            extra: vec![],
        },
        (DiagnosticCode::E101, Lang::Ja) => MessageParts {
            primary: "ファイル名とクラス名が一致していません".into(),
            hint: None,
            extra: vec![
                ("File", lookup("file").unwrap_or_default()),
                ("Class", lookup("class").unwrap_or_default()),
            ],
        },
        (DiagnosticCode::E101, Lang::En) => MessageParts {
            primary: "file name does not match class name".into(),
            hint: None,
            extra: vec![
                ("File", lookup("file").unwrap_or_default()),
                ("Class", lookup("class").unwrap_or_default()),
            ],
        },
        (DiagnosticCode::E102, Lang::Ja) => MessageParts {
            primary: "パッケージ構造が一致しません".into(),
            hint: None,
            extra: vec![
                ("Expected", lookup("expected").unwrap_or_default()),
                ("Found", lookup("found").unwrap_or_default()),
            ],
        },
        (DiagnosticCode::E102, Lang::En) => MessageParts {
            primary: "package structure does not match directory".into(),
            hint: None,
            extra: vec![
                ("Expected", lookup("expected").unwrap_or_default()),
                ("Found", lookup("found").unwrap_or_default()),
            ],
        },
        (DiagnosticCode::E110, Lang::Ja) => MessageParts {
            primary: "フィールドはクラス先頭に定義してください".into(),
            hint: None,
            extra: vec![],
        },
        (DiagnosticCode::E110, Lang::En) => MessageParts {
            primary: "fields must be declared at the beginning of the class".into(),
            hint: None,
            extra: vec![],
        },
        (DiagnosticCode::E120, Lang::Ja) => MessageParts {
            primary: "循環参照を検出しました".into(),
            hint: None,
            extra: vec![("Path", lookup("path").unwrap_or_default())],
        },
        (DiagnosticCode::E120, Lang::En) => MessageParts {
            primary: "circular import detected".into(),
            hint: None,
            extra: vec![("Path", lookup("path").unwrap_or_default())],
        },
        (DiagnosticCode::E130, Lang::Ja) => MessageParts {
            primary: "'new' キーワードは存在しません".into(),
            hint: Some(lookup("hint").unwrap_or_else(|| "User()".into())),
            extra: vec![],
        },
        (DiagnosticCode::E130, Lang::En) => MessageParts {
            primary: "keyword 'new' does not exist".into(),
            hint: Some(lookup("hint").unwrap_or_else(|| "User()".into())),
            extra: vec![],
        },
        (DiagnosticCode::E200, Lang::Ja) => MessageParts {
            primary: format!("構文エラー: {}", lookup("detail").unwrap_or_default()),
            hint: lookup("hint"),
            extra: vec![],
        },
        (DiagnosticCode::E200, Lang::En) => MessageParts {
            primary: format!("syntax error: {}", lookup("detail").unwrap_or_default()),
            hint: lookup("hint"),
            extra: vec![],
        },
        (DiagnosticCode::E201, Lang::Ja) => MessageParts {
            primary: format!("型エラー: {}", lookup("detail").unwrap_or_default()),
            hint: lookup("hint"),
            extra: vec![],
        },
        (DiagnosticCode::E201, Lang::En) => MessageParts {
            primary: format!("type error: {}", lookup("detail").unwrap_or_default()),
            hint: lookup("hint"),
            extra: vec![],
        },
        (DiagnosticCode::E202, Lang::Ja) => MessageParts {
            primary: format!("未解決のシンボル: {}", lookup("name").unwrap_or_default()),
            hint: lookup("hint"),
            extra: vec![],
        },
        (DiagnosticCode::E202, Lang::En) => MessageParts {
            primary: format!("unresolved symbol: {}", lookup("name").unwrap_or_default()),
            hint: lookup("hint"),
            extra: vec![],
        },
        (DiagnosticCode::W001, Lang::Ja) => MessageParts {
            primary: format!(
                "変数 '{}' は使用されていません",
                lookup("name").unwrap_or_default()
            ),
            hint: None,
            extra: vec![],
        },
        (DiagnosticCode::W001, Lang::En) => MessageParts {
            primary: format!(
                "variable '{}' is unused",
                lookup("name").unwrap_or_default()
            ),
            hint: None,
            extra: vec![],
        },
        (DiagnosticCode::W002, Lang::Ja) => MessageParts {
            primary: "importされたシンボルが使用されていません".into(),
            hint: None,
            extra: vec![],
        },
        (DiagnosticCode::W002, Lang::En) => MessageParts {
            primary: "imported symbol is unused".into(),
            hint: None,
            extra: vec![],
        },
        (DiagnosticCode::W003, Lang::Ja) => MessageParts {
            primary: "このコードは実行されません".into(),
            hint: None,
            extra: vec![],
        },
        (DiagnosticCode::W003, Lang::En) => MessageParts {
            primary: "this code is unreachable".into(),
            hint: None,
            extra: vec![],
        },
    }
}
