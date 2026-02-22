//! Command-line interface for the Salata templating engine. Processes `.slt`
//! files and writes output to stdout.
//!
//! This binary is the core entry point for Salata. It parses `.slt` template
//! files containing embedded runtime blocks (`<python>`, `<ruby>`,
//! `<javascript>`, `<typescript>`, `<php>`, `<shell>`), executes them
//! server-side, and writes the combined output to stdout. The output format
//! is determined entirely by what the runtime blocks print (HTML, JSON, plain
//! text, config files, etc.).
//!
//! # Usage
//!
//! ```text
//! salata [OPTIONS] <file.slt>
//! salata init [--path <dir>]
//! ```

mod init;

use std::path::{Path, PathBuf};
use std::process;

use salata_core::config::SalataConfig;
use salata_core::context::ExecutionContext;
use salata_core::logging::{LogLevel, Logger};
use salata_core::runtime::CgiEnv;

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

/// Parsed CLI command dispatched from argument parsing.
enum CliCommand {
    /// Process a `.slt` file and write the result to stdout.
    Run(RunArgs),
    /// Initialize a new Salata project with runtime detection.
    Init(InitArgs),
    /// Print the help message and exit.
    Help,
    /// Print the version string and exit.
    Version,
}

/// Arguments for the default `run` command (processing a `.slt` file).
struct RunArgs {
    /// Optional path to `config.toml`. When `None`, the binary searches
    /// for `config.toml` next to itself.
    config_path: Option<PathBuf>,
    /// The `.slt` template file to process.
    file: PathBuf,
}

/// Arguments for the `init` subcommand.
struct InitArgs {
    /// Target directory for the new project. Defaults to the current directory.
    path: PathBuf,
}

fn parse_args() -> Result<CliCommand, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();

    if raw.is_empty() {
        return Err("no input file specified".into());
    }

    // Check for --help / --version first (they can appear anywhere).
    for arg in &raw {
        match arg.as_str() {
            "--help" | "-h" => return Ok(CliCommand::Help),
            "--version" | "-V" => return Ok(CliCommand::Version),
            _ => {}
        }
    }

    // Check for `init` subcommand.
    if raw.first().map(|s| s.as_str()) == Some("init") {
        return parse_init_args(&raw[1..]);
    }

    // Otherwise parse as a run command.
    parse_run_args(&raw)
}

fn parse_init_args(args: &[String]) -> Result<CliCommand, String> {
    let mut path: Option<PathBuf> = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                i += 1;
                if i >= args.len() {
                    return Err("--path requires a directory argument".into());
                }
                path = Some(PathBuf::from(&args[i]));
            }
            arg if arg.starts_with("--path=") => {
                let p = &arg["--path=".len()..];
                if p.is_empty() {
                    return Err("--path requires a directory argument".into());
                }
                path = Some(PathBuf::from(p));
            }
            arg if arg.starts_with('-') => {
                return Err(format!("unknown option for init: {arg}"));
            }
            _ => {
                return Err(format!("unexpected argument for init: {}", args[i]));
            }
        }
        i += 1;
    }

    Ok(CliCommand::Init(InitArgs {
        path: path.unwrap_or_else(|| PathBuf::from(".")),
    }))
}

fn parse_run_args(raw: &[String]) -> Result<CliCommand, String> {
    let mut config_path: Option<PathBuf> = None;
    let mut file: Option<PathBuf> = None;
    let mut i = 0;

    while i < raw.len() {
        match raw[i].as_str() {
            "--config" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--config requires a path argument".into());
                }
                config_path = Some(PathBuf::from(&raw[i]));
            }
            arg if arg.starts_with("--config=") => {
                let path = &arg["--config=".len()..];
                if path.is_empty() {
                    return Err("--config requires a path argument".into());
                }
                config_path = Some(PathBuf::from(path));
            }
            arg if arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            }
            _ => {
                if file.is_some() {
                    return Err(format!("unexpected argument: {}", raw[i]));
                }
                file = Some(PathBuf::from(&raw[i]));
            }
        }
        i += 1;
    }

    let file = file.ok_or_else(|| "no input file specified".to_string())?;

    Ok(CliCommand::Run(RunArgs { config_path, file }))
}

fn print_help() {
    println!(
        "\
salata v{}

Salata — polyglot text templating engine

USAGE:
    salata [OPTIONS] <file.slt>
    salata init [--path <dir>]

COMMANDS:
    init               Initialize a new salata project (detect runtimes,
                       generate config.toml, create starter files)

ARGS:
    <file.slt>         The .slt file to process

OPTIONS:
    --config <path>    Path to config.toml
    -h, --help         Print help information
    -V, --version      Print version information

INIT OPTIONS:
    --path <dir>       Target directory (default: current directory)

Processed output is written to stdout.
Errors are written to stderr and logged to the configured log directory.",
        env!("CARGO_PKG_VERSION")
    );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

/// Entry point. Parses CLI arguments, dispatches to the appropriate command
/// (run, init, help, or version), and exits with the appropriate status code.
fn main() {
    let command = match parse_args() {
        Ok(cmd) => cmd,
        Err(msg) => {
            eprintln!("salata: {msg}");
            eprintln!("Try 'salata --help' for more information.");
            process::exit(1);
        }
    };

    match command {
        CliCommand::Version => {
            println!("salata v{}", env!("CARGO_PKG_VERSION"));
        }
        CliCommand::Help => {
            print_help();
        }
        CliCommand::Init(args) => {
            let code = init::run_init(&args.path, true);
            process::exit(code);
        }
        CliCommand::Run(args) => {
            run_file(&args);
        }
    }
}

fn run_file(args: &RunArgs) {
    // Validate the input file exists.
    if !args.file.exists() {
        eprintln!("salata: file not found: {}", args.file.display());
        process::exit(1);
    }

    // Load config.
    let config = match SalataConfig::locate(args.config_path.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("salata: {e}");
            process::exit(1);
        }
    };

    // Set up logger (best-effort — don't fail if log dir can't be created).
    let logger = match Logger::new(&config.logging) {
        Ok(l) => Some(l),
        Err(e) => {
            eprintln!("salata: warning: could not initialize logging: {e}");
            None
        }
    };

    // Build CGI env (empty for CLI mode).
    let env = CgiEnv::default();

    // Process the file through the full pipeline.
    match salata_core::process_file(&args.file, &config, &env, ExecutionContext::Cli) {
        Ok(result) => {
            print!("{}", result.html);

            if result.had_runtime_errors {
                log_error(&logger, &args.file, "one or more runtime errors occurred");
                process::exit(1);
            }
        }
        Err(e) => {
            log_error(&logger, &args.file, &e.to_string());
            eprintln!("salata: {e}");
            process::exit(1);
        }
    }
}

fn log_error(logger: &Option<Logger>, file: &Path, message: &str) {
    if let Some(ref logger) = logger {
        let _ = logger.log_runtime(
            LogLevel::Error,
            "salata",
            &file.display().to_string(),
            None,
            message,
        );
    }
}
