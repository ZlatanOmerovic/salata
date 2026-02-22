use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::error::{SalataError, SalataResult};

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

/// Root configuration structure parsed from `config.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SalataConfig {
    #[serde(default)]
    pub salata: SalataSection,

    #[serde(default)]
    pub server: ServerSection,

    #[serde(default)]
    pub logging: LoggingSection,

    #[serde(default)]
    pub runtimes: RuntimesSection,

    #[serde(default)]
    pub cgi: CgiSection,

    #[serde(default)]
    pub errors: ErrorsSection,
}

// ---------------------------------------------------------------------------
// [salata]
// ---------------------------------------------------------------------------

/// Global salata settings from the `[salata]` section of config.toml.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SalataSection {
    /// Whether to display runtime errors in HTML output (default: `true`).
    #[serde(default = "default_true")]
    pub display_errors: bool,

    /// Default MIME type for responses (default: `"text/html; charset=utf-8"`).
    #[serde(default = "default_content_type")]
    pub default_content_type: String,

    /// Character encoding enforced across all runtimes (default: `"utf-8"`).
    #[serde(default = "default_encoding")]
    pub encoding: String,
}

impl Default for SalataSection {
    fn default() -> Self {
        Self {
            display_errors: true,
            default_content_type: default_content_type(),
            encoding: default_encoding(),
        }
    }
}

// ---------------------------------------------------------------------------
// [server]
// ---------------------------------------------------------------------------

/// Development server settings from the `[server]` section.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSection {
    /// Enable file watcher for automatic reload on changes (default: `true`).
    #[serde(default = "default_true")]
    pub hot_reload: bool,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self { hot_reload: true }
    }
}

// ---------------------------------------------------------------------------
// [logging]
// ---------------------------------------------------------------------------

/// Logging configuration from the `[logging]` section.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSection {
    /// Directory for log files, relative to the binary (default: `"./logs"`).
    #[serde(default = "default_log_directory")]
    pub directory: String,

    /// Maximum log file size before rotation (e.g., `"50MB"`).
    #[serde(default = "default_rotation_max_size")]
    pub rotation_max_size: String,

    /// Maximum number of rotated log files to keep (default: `10`).
    #[serde(default = "default_rotation_max_files")]
    pub rotation_max_files: u32,

    #[serde(default)]
    pub server: LoggingServerSection,

    #[serde(default)]
    pub runtimes: LoggingRuntimesSection,
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self {
            directory: default_log_directory(),
            rotation_max_size: default_rotation_max_size(),
            rotation_max_files: 10,
            server: LoggingServerSection::default(),
            runtimes: LoggingRuntimesSection::default(),
        }
    }
}

/// Server-specific log file names from `[logging.server]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingServerSection {
    #[serde(default = "default_access_log")]
    pub access_log: String,

    #[serde(default = "default_error_log")]
    pub error_log: String,

    #[serde(default = "default_log_format")]
    pub format: String,
}

impl Default for LoggingServerSection {
    fn default() -> Self {
        Self {
            access_log: default_access_log(),
            error_log: default_error_log(),
            format: default_log_format(),
        }
    }
}

/// Per-runtime log file names from `[logging.runtimes]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingRuntimesSection {
    #[serde(default = "default_python_log")]
    pub python: String,

    #[serde(default = "default_ruby_log")]
    pub ruby: String,

    #[serde(default = "default_javascript_log")]
    pub javascript: String,

    #[serde(default = "default_typescript_log")]
    pub typescript: String,

    #[serde(default = "default_php_log")]
    pub php: String,

    #[serde(default = "default_shell_log")]
    pub shell: String,
}

impl Default for LoggingRuntimesSection {
    fn default() -> Self {
        Self {
            python: default_python_log(),
            ruby: default_ruby_log(),
            javascript: default_javascript_log(),
            typescript: default_typescript_log(),
            php: default_php_log(),
            shell: default_shell_log(),
        }
    }
}

// ---------------------------------------------------------------------------
// [runtimes]
// ---------------------------------------------------------------------------

/// Container for all runtime configurations from `[runtimes]`.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RuntimesSection {
    pub python: Option<RuntimeConfig>,
    pub ruby: Option<RuntimeConfig>,
    pub javascript: Option<RuntimeConfig>,
    pub typescript: Option<RuntimeConfig>,
    pub php: Option<PhpRuntimeConfig>,
    pub shell: Option<RuntimeConfig>,
}

/// Standard runtime config (python, ruby, javascript, typescript, shell).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    /// Whether this runtime is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Absolute path to the runtime binary (e.g., `"/usr/bin/python3"`).
    pub path: String,

    /// Whether blocks of this language share a single process (default: `true`).
    #[serde(default = "default_true")]
    pub shared_scope: bool,

    /// Per-runtime override for `display_errors` (falls back to global setting).
    pub display_errors: Option<bool>,
}

/// PHP-specific runtime configuration with context-aware binary selection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhpRuntimeConfig {
    /// Whether this runtime is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// PHP execution mode: `"cgi"` or `"fastcgi"`.
    #[serde(default = "default_php_mode")]
    pub mode: PhpMode,

    /// Path to the `php` CLI binary (used by `salata` CLI).
    pub cli_path: Option<String>,
    /// Path to the `php-cgi` binary (used by `salata-cgi`).
    pub cgi_path: Option<String>,
    /// Unix socket for php-fpm (used by `salata-fastcgi` / `salata-server`).
    pub fastcgi_socket: Option<String>,
    /// TCP host:port for php-fpm (used by `salata-fastcgi` / `salata-server`).
    pub fastcgi_host: Option<String>,

    /// Whether PHP blocks share a single process (default: `true`).
    #[serde(default = "default_true")]
    pub shared_scope: bool,

    /// Per-runtime override for `display_errors`.
    pub display_errors: Option<bool>,
}

/// PHP execution mode — determines which SAPI interface to use.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PhpMode {
    /// CGI mode — uses `php-cgi` binary with CGI environment variables.
    Cgi,
    /// FastCGI mode — connects to a running `php-fpm` process.
    Fastcgi,
}

// ---------------------------------------------------------------------------
// [cgi]
// ---------------------------------------------------------------------------

/// CGI security and resource limit settings from the `[cgi]` section.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CgiSection {
    /// Maximum time to receive request headers (default: `"5s"`).
    #[serde(default = "default_header_timeout")]
    pub header_timeout: String,

    /// Maximum time to receive request body (default: `"30s"`).
    #[serde(default = "default_body_timeout")]
    pub body_timeout: String,

    /// Minimum data transfer rate before slowloris protection triggers (default: `"100b/s"`).
    #[serde(default = "default_min_data_rate")]
    pub min_data_rate: String,

    /// Maximum URL length in bytes (default: 2048).
    #[serde(default = "default_max_url_length")]
    pub max_url_length: usize,

    /// Maximum total size of all request headers (default: `"8KB"`).
    #[serde(default = "default_max_header_size")]
    pub max_header_size: String,

    /// Maximum number of request headers (default: 50).
    #[serde(default = "default_max_header_count")]
    pub max_header_count: usize,

    /// Maximum query string length in bytes (default: 2048).
    #[serde(default = "default_max_query_string_length")]
    pub max_query_string_length: usize,

    /// Maximum request body size (default: `"10MB"`).
    #[serde(default = "default_max_body_size")]
    pub max_body_size: String,

    /// Maximum concurrent connections from a single IP (default: 20).
    #[serde(default = "default_max_connections_per_ip")]
    pub max_connections_per_ip: usize,

    /// Maximum total concurrent connections (default: 200).
    #[serde(default = "default_max_total_connections")]
    pub max_total_connections: usize,

    /// Maximum execution time per request (default: `"30s"`).
    #[serde(default = "default_max_execution_time")]
    pub max_execution_time: String,

    /// Maximum memory per request (default: `"128MB"`).
    #[serde(default = "default_max_memory_per_request")]
    pub max_memory_per_request: String,

    /// Maximum response body size (default: `"50MB"`).
    #[serde(default = "default_max_response_size")]
    pub max_response_size: String,

    /// Maximum time to produce a response (default: `"60s"`).
    #[serde(default = "default_response_timeout")]
    pub response_timeout: String,

    /// Block access to dotfiles (e.g., `.env`, `.git`) (default: `true`).
    #[serde(default = "default_true")]
    pub block_dotfiles: bool,

    /// Block `../` path traversal attempts (default: `true`).
    #[serde(default = "default_true")]
    pub block_path_traversal: bool,

    /// File extensions that are blocked from being served (default: `.toml`, `.env`, `.git`, `.log`).
    #[serde(default = "default_blocked_extensions")]
    pub blocked_extensions: Vec<String>,

    /// Block null bytes in request paths and headers (default: `true`).
    #[serde(default = "default_true")]
    pub block_null_bytes: bool,

    /// Block non-printable characters in request headers (default: `true`).
    #[serde(default = "default_true")]
    pub block_non_printable_headers: bool,

    /// Validate that `Content-Length` matches actual body size (default: `true`).
    #[serde(default = "default_true")]
    pub validate_content_length: bool,

    /// Maximum number of child processes for CGI execution (default: 10).
    #[serde(default = "default_max_child_processes")]
    pub max_child_processes: usize,

    /// Whether to allow outbound network access from runtime blocks (default: `true`).
    #[serde(default = "default_true")]
    pub allow_outbound_network: bool,
}

impl Default for CgiSection {
    fn default() -> Self {
        Self {
            header_timeout: default_header_timeout(),
            body_timeout: default_body_timeout(),
            min_data_rate: default_min_data_rate(),
            max_url_length: 2048,
            max_header_size: default_max_header_size(),
            max_header_count: 50,
            max_query_string_length: 2048,
            max_body_size: default_max_body_size(),
            max_connections_per_ip: 20,
            max_total_connections: 200,
            max_execution_time: default_max_execution_time(),
            max_memory_per_request: default_max_memory_per_request(),
            max_response_size: default_max_response_size(),
            response_timeout: default_response_timeout(),
            block_dotfiles: true,
            block_path_traversal: true,
            blocked_extensions: default_blocked_extensions(),
            block_null_bytes: true,
            block_non_printable_headers: true,
            validate_content_length: true,
            max_child_processes: 10,
            allow_outbound_network: true,
        }
    }
}

// ---------------------------------------------------------------------------
// [errors]
// ---------------------------------------------------------------------------

/// Custom error page paths from the `[errors]` section.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorsSection {
    /// Path to custom 404 error page (can be a `.slt` file).
    pub page_404: Option<String>,
    /// Path to custom 500 error page (can be a `.slt` file).
    pub page_500: Option<String>,
}

impl Default for ErrorsSection {
    fn default() -> Self {
        Self {
            page_404: Some("./errors/404.slt".into()),
            page_500: Some("./errors/500.slt".into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

impl SalataConfig {
    /// Parse a TOML string into a validated `SalataConfig`.
    pub fn parse(toml_str: &str) -> SalataResult<Self> {
        let config: SalataConfig =
            toml::from_str(toml_str).map_err(|e| SalataError::ConfigParse {
                message: e.to_string(),
            })?;
        config.validate()?;
        Ok(config)
    }

    /// Load config from a file path.
    pub fn from_file(path: &Path) -> SalataResult<Self> {
        if !path.exists() {
            return Err(SalataError::ConfigNotFound {
                path: path.to_path_buf(),
            });
        }
        let contents = std::fs::read_to_string(path).map_err(|e| SalataError::ConfigRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::parse(&contents)
    }

    /// Locate and load config using the standard lookup order:
    /// 1. Explicit path from `--config` flag (if `Some`)
    /// 2. `config.toml` next to the current executable
    /// 3. Error
    pub fn locate(explicit_path: Option<&Path>) -> SalataResult<Self> {
        if let Some(path) = explicit_path {
            return Self::from_file(path);
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join("config.toml");
                if candidate.exists() {
                    return Self::from_file(&candidate);
                }
            }
        }

        Err(SalataError::ConfigNotLocated)
    }

    /// Check whether a specific runtime is enabled.
    /// Returns `false` if the runtime is configured with `enabled = false`,
    /// and `true` if it is enabled or not configured at all.
    pub fn is_runtime_enabled(&self, name: &str) -> bool {
        match name {
            "python" => self.runtimes.python.as_ref().is_none_or(|r| r.enabled),
            "ruby" => self.runtimes.ruby.as_ref().is_none_or(|r| r.enabled),
            "javascript" => self.runtimes.javascript.as_ref().is_none_or(|r| r.enabled),
            "typescript" => self.runtimes.typescript.as_ref().is_none_or(|r| r.enabled),
            "php" => self.runtimes.php.as_ref().is_none_or(|r| r.enabled),
            "shell" => self.runtimes.shell.as_ref().is_none_or(|r| r.enabled),
            _ => true,
        }
    }

    /// Check whether at least one configured runtime is enabled.
    /// Returns `true` if no runtimes are configured (HTML-only mode is valid).
    /// Returns `false` only when runtimes are configured but ALL are disabled.
    pub fn has_any_runtime_enabled(&self) -> bool {
        let configured: Vec<bool> = [
            self.runtimes.python.as_ref().map(|r| r.enabled),
            self.runtimes.ruby.as_ref().map(|r| r.enabled),
            self.runtimes.javascript.as_ref().map(|r| r.enabled),
            self.runtimes.typescript.as_ref().map(|r| r.enabled),
            self.runtimes.php.as_ref().map(|r| r.enabled),
            self.runtimes.shell.as_ref().map(|r| r.enabled),
        ]
        .into_iter()
        .flatten()
        .collect();

        // No runtimes configured at all → fine (HTML-only mode).
        if configured.is_empty() {
            return true;
        }

        // At least one must be enabled.
        configured.iter().any(|&e| e)
    }

    /// Validate the parsed config for semantic correctness.
    fn validate(&self) -> SalataResult<()> {
        self.validate_php()?;
        self.validate_logging()?;
        self.validate_runtimes_enabled()?;
        Ok(())
    }

    fn validate_php(&self) -> SalataResult<()> {
        if let Some(php) = &self.runtimes.php {
            match php.mode {
                PhpMode::Cgi => {
                    if php.cgi_path.is_none() {
                        return Err(SalataError::ConfigValidation {
                            field: "runtimes.php.cgi_path".into(),
                            message: "cgi_path is required when mode = \"cgi\"".into(),
                        });
                    }
                }
                PhpMode::Fastcgi => {
                    if php.fastcgi_socket.is_none() && php.fastcgi_host.is_none() {
                        return Err(SalataError::ConfigValidation {
                            field: "runtimes.php".into(),
                            message:
                                "fastcgi_socket or fastcgi_host is required when mode = \"fastcgi\""
                                    .into(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_logging(&self) -> SalataResult<()> {
        if self.logging.rotation_max_files == 0 {
            return Err(SalataError::ConfigValidation {
                field: "logging.rotation_max_files".into(),
                message: "must be at least 1".into(),
            });
        }
        Ok(())
    }

    fn validate_runtimes_enabled(&self) -> SalataResult<()> {
        if !self.has_any_runtime_enabled() {
            return Err(SalataError::ConfigValidation {
                field: "runtimes".into(),
                message: "No runtimes enabled. Enable at least one runtime in config.toml to process .slt files.".into(),
            });
        }
        Ok(())
    }

    /// Resolve the display_errors setting for a given runtime, falling back
    /// to the global `[salata]` setting.
    pub fn display_errors_for(&self, runtime: &str) -> bool {
        let override_val = match runtime {
            "python" => self.runtimes.python.as_ref().and_then(|r| r.display_errors),
            "ruby" => self.runtimes.ruby.as_ref().and_then(|r| r.display_errors),
            "javascript" => self
                .runtimes
                .javascript
                .as_ref()
                .and_then(|r| r.display_errors),
            "typescript" => self
                .runtimes
                .typescript
                .as_ref()
                .and_then(|r| r.display_errors),
            "php" => self.runtimes.php.as_ref().and_then(|r| r.display_errors),
            "shell" => self.runtimes.shell.as_ref().and_then(|r| r.display_errors),
            _ => None,
        };
        override_val.unwrap_or(self.salata.display_errors)
    }

    /// Get a map of all configured and enabled runtimes and their paths.
    pub fn runtime_paths(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        if let Some(r) = &self.runtimes.python {
            if r.enabled {
                map.insert("python", r.path.as_str());
            }
        }
        if let Some(r) = &self.runtimes.ruby {
            if r.enabled {
                map.insert("ruby", r.path.as_str());
            }
        }
        if let Some(r) = &self.runtimes.javascript {
            if r.enabled {
                map.insert("javascript", r.path.as_str());
            }
        }
        if let Some(r) = &self.runtimes.typescript {
            if r.enabled {
                map.insert("typescript", r.path.as_str());
            }
        }
        if let Some(r) = &self.runtimes.php {
            if r.enabled {
                if let Some(p) = &r.cgi_path {
                    map.insert("php", p.as_str());
                }
            }
        }
        if let Some(r) = &self.runtimes.shell {
            if r.enabled {
                map.insert("shell", r.path.as_str());
            }
        }
        map
    }
}

// ---------------------------------------------------------------------------
// Default value helpers
// ---------------------------------------------------------------------------

fn default_true() -> bool {
    true
}

fn default_content_type() -> String {
    "text/html; charset=utf-8".into()
}

fn default_encoding() -> String {
    "utf-8".into()
}

fn default_log_directory() -> String {
    "./logs".into()
}

fn default_rotation_max_size() -> String {
    "50MB".into()
}

fn default_rotation_max_files() -> u32 {
    10
}

fn default_access_log() -> String {
    "access.log".into()
}

fn default_error_log() -> String {
    "error.log".into()
}

fn default_log_format() -> String {
    "combined".into()
}

fn default_python_log() -> String {
    "python.log".into()
}

fn default_ruby_log() -> String {
    "ruby.log".into()
}

fn default_javascript_log() -> String {
    "javascript.log".into()
}

fn default_typescript_log() -> String {
    "typescript.log".into()
}

fn default_php_log() -> String {
    "php.log".into()
}

fn default_shell_log() -> String {
    "shell.log".into()
}

fn default_php_mode() -> PhpMode {
    PhpMode::Cgi
}

fn default_header_timeout() -> String {
    "5s".into()
}

fn default_body_timeout() -> String {
    "30s".into()
}

fn default_min_data_rate() -> String {
    "100b/s".into()
}

fn default_max_url_length() -> usize {
    2048
}

fn default_max_header_size() -> String {
    "8KB".into()
}

fn default_max_header_count() -> usize {
    50
}

fn default_max_query_string_length() -> usize {
    2048
}

fn default_max_body_size() -> String {
    "10MB".into()
}

fn default_max_connections_per_ip() -> usize {
    20
}

fn default_max_total_connections() -> usize {
    200
}

fn default_max_execution_time() -> String {
    "30s".into()
}

fn default_max_memory_per_request() -> String {
    "128MB".into()
}

fn default_max_response_size() -> String {
    "50MB".into()
}

fn default_response_timeout() -> String {
    "60s".into()
}

fn default_blocked_extensions() -> Vec<String> {
    vec![".toml".into(), ".env".into(), ".git".into(), ".log".into()]
}

fn default_max_child_processes() -> usize {
    10
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The full default config.toml from the spec should parse without error.
    #[test]
    fn parse_full_default_config() {
        let toml_str = include_str!("../../../config.toml");
        let config = SalataConfig::parse(toml_str).expect("should parse default config");

        // [salata]
        assert!(config.salata.display_errors);
        assert_eq!(
            config.salata.default_content_type,
            "text/html; charset=utf-8"
        );
        assert_eq!(config.salata.encoding, "utf-8");

        // [server]
        assert!(config.server.hot_reload);

        // [logging]
        assert_eq!(config.logging.directory, "./logs");
        assert_eq!(config.logging.rotation_max_size, "50MB");
        assert_eq!(config.logging.rotation_max_files, 10);
        assert_eq!(config.logging.server.access_log, "access.log");
        assert_eq!(config.logging.server.error_log, "error.log");
        assert_eq!(config.logging.server.format, "combined");
        assert_eq!(config.logging.runtimes.python, "python.log");

        // [runtimes]
        let py = config.runtimes.python.as_ref().unwrap();
        assert_eq!(py.path, "/usr/bin/python3");
        assert!(py.shared_scope);
        assert_eq!(py.display_errors, Some(true));

        let js = config.runtimes.javascript.as_ref().unwrap();
        assert_eq!(js.path, "/usr/bin/node");

        let php = config.runtimes.php.as_ref().unwrap();
        assert_eq!(php.mode, PhpMode::Cgi);
        assert_eq!(php.cgi_path.as_deref(), Some("/usr/bin/php-cgi"));

        // [cgi]
        assert_eq!(config.cgi.header_timeout, "5s");
        assert_eq!(config.cgi.max_url_length, 2048);
        assert!(config.cgi.block_dotfiles);
        assert_eq!(config.cgi.blocked_extensions.len(), 4);
        assert_eq!(config.cgi.max_child_processes, 10);

        // [errors]
        assert_eq!(config.errors.page_404.as_deref(), Some("./errors/404.slt"));
        assert_eq!(config.errors.page_500.as_deref(), Some("./errors/500.slt"));
    }

    /// An empty TOML should parse and fill all defaults.
    #[test]
    fn empty_config_uses_defaults() {
        let config = SalataConfig::parse("").expect("empty config should parse");

        assert!(config.salata.display_errors);
        assert_eq!(
            config.salata.default_content_type,
            "text/html; charset=utf-8"
        );
        assert!(config.server.hot_reload);
        assert_eq!(config.logging.directory, "./logs");
        assert_eq!(config.logging.rotation_max_files, 10);
        assert_eq!(config.logging.server.access_log, "access.log");
        assert_eq!(config.logging.runtimes.shell, "shell.log");
        assert!(config.runtimes.python.is_none());
        assert!(config.runtimes.php.is_none());
        assert_eq!(config.cgi.max_url_length, 2048);
        assert!(config.cgi.block_path_traversal);
        assert_eq!(config.errors.page_404.as_deref(), Some("./errors/404.slt"));
    }

    /// Partial config: only set some fields, rest should default.
    #[test]
    fn partial_config_fills_defaults() {
        let toml_str = r#"
[salata]
display_errors = false

[cgi]
max_url_length = 4096
block_dotfiles = false
"#;
        let config = SalataConfig::parse(toml_str).expect("should parse partial config");

        assert!(!config.salata.display_errors);
        assert_eq!(
            config.salata.default_content_type,
            "text/html; charset=utf-8"
        );
        assert_eq!(config.cgi.max_url_length, 4096);
        assert!(!config.cgi.block_dotfiles);
        // Other CGI fields still default
        assert_eq!(config.cgi.header_timeout, "5s");
        assert_eq!(config.cgi.max_header_count, 50);
    }

    /// Unknown fields should be rejected (deny_unknown_fields).
    #[test]
    fn unknown_field_rejected() {
        let toml_str = r#"
[salata]
display_errors = true
bogus_field = "nope"
"#;
        let result = SalataConfig::parse(toml_str);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SalataError::ConfigParse { .. }),
            "expected ConfigParse, got: {err}"
        );
    }

    /// PHP CGI mode requires cgi_path.
    #[test]
    fn php_cgi_requires_cgi_path() {
        let toml_str = r#"
[runtimes.php]
mode = "cgi"
"#;
        let result = SalataConfig::parse(toml_str);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SalataError::ConfigValidation { ref field, .. } if field == "runtimes.php.cgi_path"),
            "expected ConfigValidation for cgi_path, got: {err}"
        );
    }

    /// PHP FastCGI mode requires socket or host.
    #[test]
    fn php_fastcgi_requires_socket_or_host() {
        let toml_str = r#"
[runtimes.php]
mode = "fastcgi"
"#;
        let result = SalataConfig::parse(toml_str);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SalataError::ConfigValidation { ref field, .. } if field == "runtimes.php"),
            "expected ConfigValidation for php, got: {err}"
        );
    }

    /// PHP FastCGI with socket should pass.
    #[test]
    fn php_fastcgi_with_socket_valid() {
        let toml_str = r#"
[runtimes.php]
mode = "fastcgi"
fastcgi_socket = "/run/php/php-fpm.sock"
"#;
        let config = SalataConfig::parse(toml_str).expect("should parse");
        let php = config.runtimes.php.unwrap();
        assert_eq!(php.mode, PhpMode::Fastcgi);
        assert_eq!(php.fastcgi_socket.as_deref(), Some("/run/php/php-fpm.sock"));
    }

    /// Invalid TOML syntax should produce ConfigParse error.
    #[test]
    fn invalid_toml_syntax() {
        let result = SalataConfig::parse("[salata\nbad syntax");
        assert!(matches!(result, Err(SalataError::ConfigParse { .. })));
    }

    /// Wrong type for a field should produce ConfigParse error.
    #[test]
    fn wrong_type_rejected() {
        let toml_str = r#"
[salata]
display_errors = "not a bool"
"#;
        let result = SalataConfig::parse(toml_str);
        assert!(matches!(result, Err(SalataError::ConfigParse { .. })));
    }

    /// rotation_max_files = 0 should fail validation.
    #[test]
    fn zero_rotation_files_rejected() {
        let toml_str = r#"
[logging]
rotation_max_files = 0
"#;
        let result = SalataConfig::parse(toml_str);
        assert!(matches!(
            result,
            Err(SalataError::ConfigValidation { ref field, .. }) if field == "logging.rotation_max_files"
        ));
    }

    /// display_errors_for uses runtime override when present.
    #[test]
    fn display_errors_runtime_override() {
        let toml_str = r#"
[salata]
display_errors = false

[runtimes.python]
path = "/usr/bin/python3"
display_errors = true
"#;
        let config = SalataConfig::parse(toml_str).expect("should parse");
        assert!(config.display_errors_for("python"));
        assert!(!config.display_errors_for("ruby"));
        assert!(!config.display_errors_for("unknown"));
    }

    /// runtime_paths returns all configured runtimes.
    #[test]
    fn runtime_paths_map() {
        let toml_str = r#"
[runtimes.python]
path = "/usr/bin/python3"

[runtimes.shell]
path = "/bin/bash"
"#;
        let config = SalataConfig::parse(toml_str).expect("should parse");
        let paths = config.runtime_paths();
        assert_eq!(paths.get("python"), Some(&"/usr/bin/python3"));
        assert_eq!(paths.get("shell"), Some(&"/bin/bash"));
        assert!(!paths.contains_key("ruby"));
    }

    /// Loading a nonexistent file should produce ConfigNotFound.
    #[test]
    fn missing_file_errors() {
        let result = SalataConfig::from_file(Path::new("/nonexistent/config.toml"));
        assert!(matches!(result, Err(SalataError::ConfigNotFound { .. })));
    }

    /// locate with no explicit path and no config next to binary → ConfigNotLocated.
    #[test]
    fn locate_no_config_errors() {
        // The test binary won't have config.toml next to it
        let result = SalataConfig::locate(None);
        assert!(matches!(result, Err(SalataError::ConfigNotLocated)));
    }

    /// locate with explicit nonexistent path → ConfigNotFound.
    #[test]
    fn locate_explicit_missing() {
        let result = SalataConfig::locate(Some(Path::new("/tmp/no-such-config.toml")));
        assert!(matches!(result, Err(SalataError::ConfigNotFound { .. })));
    }

    /// Runtime with enabled = true (default) is enabled.
    #[test]
    fn runtime_enabled_default() {
        let toml_str = r#"
[runtimes.python]
path = "/usr/bin/python3"
"#;
        let config = SalataConfig::parse(toml_str).expect("should parse");
        assert!(config.is_runtime_enabled("python"));
        assert!(config.runtimes.python.as_ref().unwrap().enabled);
    }

    /// Runtime with enabled = false is disabled.
    #[test]
    fn runtime_disabled_single() {
        let toml_str = r#"
[runtimes.python]
path = "/usr/bin/python3"
enabled = false

[runtimes.ruby]
path = "/usr/bin/ruby"
"#;
        let config = SalataConfig::parse(toml_str).expect("should parse");
        assert!(!config.is_runtime_enabled("python"));
        assert!(config.is_runtime_enabled("ruby"));

        // runtime_paths should exclude disabled runtimes
        let paths = config.runtime_paths();
        assert!(!paths.contains_key("python"));
        assert!(paths.contains_key("ruby"));
    }

    /// All runtimes disabled → config validation error.
    #[test]
    fn all_runtimes_disabled_rejected() {
        let toml_str = r#"
[runtimes.python]
path = "/usr/bin/python3"
enabled = false

[runtimes.ruby]
path = "/usr/bin/ruby"
enabled = false
"#;
        let result = SalataConfig::parse(toml_str);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SalataError::ConfigValidation { ref field, ref message } if field == "runtimes" && message.contains("No runtimes enabled")),
            "expected validation error about no runtimes enabled, got: {err}"
        );
    }

    /// No runtimes configured is valid (HTML-only mode).
    #[test]
    fn no_runtimes_configured_is_valid() {
        let config = SalataConfig::parse("").expect("empty config is valid");
        assert!(config.has_any_runtime_enabled());
    }

    /// PHP runtime respects enabled field.
    #[test]
    fn php_runtime_enabled_field() {
        let toml_str = r#"
[runtimes.php]
mode = "cgi"
cgi_path = "/usr/bin/php-cgi"
enabled = false

[runtimes.python]
path = "/usr/bin/python3"
"#;
        let config = SalataConfig::parse(toml_str).expect("should parse");
        assert!(!config.is_runtime_enabled("php"));
        assert!(config.is_runtime_enabled("python"));
        assert!(!config.runtime_paths().contains_key("php"));
    }
}
