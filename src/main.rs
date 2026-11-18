use clap::{Parser as ClapParser, Subcommand};
use rjc::diagnostic::Lang;
use rjc::{compile_file, compile_sources, print_diagnostics, CompileOptions};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(ClapParser)]
#[command(name = "rjc", version, about = "RJ Language Compiler for JVM")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true, value_parser = parse_lang)]
    lang: Option<Lang>,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile RJ source files
    Build {
        /// Output directory for .class files
        #[arg(long, default_value = "build/classes/rj")]
        output: PathBuf,

        /// Source root directory
        source: PathBuf,
    },
    /// Type-check without generating bytecode
    Check {
        /// RJ source file
        file: PathBuf,
    },
}

fn parse_lang(s: &str) -> Result<Lang, String> {
    Lang::parse(s).ok_or_else(|| format!("unsupported language: {s} (use ja or en)"))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let lang = cli.lang.unwrap_or_else(Lang::detect);

    let result = match cli.command {
        Commands::Build { output, source } => {
            let options = CompileOptions {
                lang,
                output_dir: output,
                source_root: source,
                check_only: false,
            };
            compile_sources(&options)
        }
        Commands::Check { file } => {
            let options = CompileOptions {
                lang,
                check_only: true,
                ..CompileOptions::default()
            };
            compile_file(&file, &options)
        }
    };

    match result {
        Ok(output) => {
            if !output.warnings.items.is_empty() {
                print_diagnostics(&output.warnings, lang);
            }
            for path in &output.class_files {
                println!("{}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(errors) => {
            print_diagnostics(&errors, lang);
            ExitCode::FAILURE
        }
    }
}
