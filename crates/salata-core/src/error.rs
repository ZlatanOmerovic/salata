//! Error types for the Salata engine.
//!
//! Uses [`thiserror`] for ergonomic error definitions. All errors include
//! context (file path, line number, runtime name) to help users locate
//! the source of problems.

use std::path::PathBuf;

/// All error types for the Salata engine.
#[derive(Debug, thiserror::Error)]
pub enum SalataError {
    // -- Config errors --

    /// Config file at the specified path does not exist.
    #[error("config file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    /// No config file found via `--config` flag or next to the binary.
    #[error("no config file found (searched --config flag and binary directory)")]
    ConfigNotLocated,

    /// Config file exists but could not be read.
    #[error("failed to read config file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Config file contains invalid TOML syntax or structure.
    #[error("failed to parse config: {message}")]
    ConfigParse { message: String },

    /// Config file has semantically invalid values.
    #[error("invalid config value: {field}: {message}")]
    ConfigValidation { field: String, message: String },

    // -- Parse errors --

    /// General parse error in a `.slt` file.
    #[error("parse error in {file} at line {line}: {message}")]
    Parse {
        file: PathBuf,
        line: usize,
        message: String,
    },

    /// A runtime tag was found inside another runtime tag.
    #[error("nested runtime tag <{tag}> inside <{outer}> in {file} at line {line}")]
    NestedRuntimeTag {
        tag: String,
        outer: String,
        file: PathBuf,
        line: usize,
    },

    // -- Runtime errors --

    /// A runtime block failed during execution.
    #[error("runtime error [{runtime}] in {file} at line {line}: {message}")]
    Runtime {
        runtime: String,
        file: PathBuf,
        line: usize,
        message: String,
    },

    /// The runtime binary does not exist at the configured path.
    #[error("runtime not found: {runtime} (path: {path})")]
    RuntimeNotFound { runtime: String, path: PathBuf },

    /// A `.slt` file uses a runtime that is disabled in config.
    #[error("runtime '{runtime}' is disabled in config.toml")]
    RuntimeDisabled { runtime: String },

    /// A runtime block exceeded its execution time limit.
    #[error("runtime timeout [{runtime}] after {seconds}s in {file}")]
    RuntimeTimeout {
        runtime: String,
        file: PathBuf,
        seconds: u64,
    },

    // -- Directive errors --

    /// A directive that can only appear once was found multiple times.
    #[error("duplicate directive #{directive} in {file} (only one allowed per page)")]
    DuplicateDirective { directive: String, file: PathBuf },

    /// A directive has invalid syntax or arguments.
    #[error("invalid directive in {file} at line {line}: {message}")]
    DirectiveInvalid {
        file: PathBuf,
        line: usize,
        message: String,
    },

    /// Recursive `#include` exceeded the maximum depth (16 levels).
    #[error("#include depth exceeded (max {max_depth}) in {file}")]
    IncludeDepthExceeded { file: PathBuf, max_depth: usize },

    /// An `#include`d file does not exist.
    #[error("included file not found: {path} (from {source_file} at line {line})")]
    IncludeNotFound {
        path: PathBuf,
        source_file: PathBuf,
        line: usize,
    },

    // -- Security errors --

    /// Shell code was rejected by the sandbox's static analysis.
    #[error("shell sandbox violation in {file} at line {line}: {message}")]
    ShellSandboxViolation {
        file: PathBuf,
        line: usize,
        message: String,
    },

    /// The configured shell binary is not in the hardcoded whitelist.
    #[error("shell not allowed: {path} (not in whitelist)")]
    ShellNotAllowed { path: PathBuf },

    // -- I/O errors --

    /// An I/O error from the standard library.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience type alias used throughout the crate.
pub type SalataResult<T> = Result<T, SalataError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_config_not_found() {
        let err = SalataError::ConfigNotFound {
            path: PathBuf::from("/missing/config.toml"),
        };
        assert_eq!(
            err.to_string(),
            "config file not found: /missing/config.toml"
        );
    }

    #[test]
    fn error_display_nested_runtime_tag() {
        let err = SalataError::NestedRuntimeTag {
            tag: "python".into(),
            outer: "ruby".into(),
            file: PathBuf::from("index.slt"),
            line: 10,
        };
        assert!(err.to_string().contains("python"));
        assert!(err.to_string().contains("ruby"));
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: SalataError = io_err.into();
        assert!(matches!(err, SalataError::Io(_)));
    }
}
