use rjc::{compile_sources, render_diagnostics, CompileOptions};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn fixture_source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/src/rj")
}

fn compile_fixtures(output_dir: &Path) -> rjc::CompileOutput {
    let options = CompileOptions {
        output_dir: output_dir.to_path_buf(),
        source_root: fixture_source_root(),
        ..CompileOptions::default()
    };

    compile_sources(&options).unwrap_or_else(|errors| {
        panic!(
            "compile failed:\n{}",
            render_diagnostics(&errors, rjc::diagnostic::Lang::En)
        );
    })
}

#[test]
fn compiles_fixture_main() {
    let dir = TempDir::new().expect("temp dir");
    let output = compile_fixtures(&dir.path().join("out"));

    let class_file = dir.path().join("out/com/github/haku0451/main.class");
    assert!(
        class_file.exists(),
        "expected class file at {}",
        class_file.display()
    );
    assert!(
        output.class_files.iter().any(|p| p == &class_file),
        "compile output should list {}",
        class_file.display()
    );

    let bytes = fs::read(&class_file).expect("read class file");
    assert_eq!(
        &bytes[..4],
        &[0xCA, 0xFE, 0xBA, 0xBE],
        "invalid class file magic"
    );
}

#[test]
fn compiles_fixture_math() {
    let dir = TempDir::new().expect("temp dir");
    compile_fixtures(&dir.path().join("out"));

    let class_file = dir.path().join("out/com/github/haku0451/math.class");
    assert!(
        class_file.exists(),
        "expected class file at {}",
        class_file.display()
    );
}

#[test]
fn check_only_fixture_sources() {
    let dir = TempDir::new().expect("temp dir");
    let options = CompileOptions {
        output_dir: dir.path().join("out"),
        source_root: fixture_source_root(),
        check_only: true,
        ..CompileOptions::default()
    };

    let output = compile_sources(&options).expect("check should succeed");
    assert!(output.class_files.is_empty());
    assert!(!dir.path().join("out/com/github/haku0451/main.class").exists());
}

#[test]
fn fails_without_package_declaration() {
    let dir = TempDir::new().expect("temp dir");
    let source_dir = dir.path().join("src/rj/com/github/haku0451/bad");
    fs::create_dir_all(&source_dir).expect("create dirs");
    fs::write(
        source_dir.join("Bad.rj"),
        r#"def main() {
    print("no package")
}
"#,
    )
    .expect("write source");

    let options = CompileOptions {
        output_dir: dir.path().join("out"),
        source_root: dir.path().join("src/rj"),
        ..CompileOptions::default()
    };

    let result = compile_sources(&options);
    assert!(result.is_err(), "missing package should fail");

    let errors = result.unwrap_err();
    let rendered = render_diagnostics(&errors, rjc::diagnostic::Lang::En);
    assert!(
        rendered.contains("E010"),
        "expected E010, got:\n{rendered}"
    );
}
