//! CGI attack protections -- rate limiting, input validation, and request
//! sanitization.
//!
//! Implements the full set of protections defined in `specs/SECURITY.md`:
//!
//! - **Slowloris mitigation:** header timeout, body read timeout, minimum data rate
//! - **Request limits:** URL length, header size/count, query string length, body size
//! - **Process limits:** per-IP connections, total connections, execution time, response size
//! - **Path security:** directory traversal blocking, dotfile access blocking, extension blacklist
//! - **Input sanitization:** null byte detection, non-printable header rejection,
//!   Content-Length validation

use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use salata_core::config::CgiSection;

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse a human-readable size string (e.g. "50MB", "8KB") into bytes.
pub fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let idx = s
        .find(|c: char| c.is_ascii_alphabetic())
        .ok_or_else(|| format!("missing unit in size: {s}"))?;
    let (num, unit) = s.split_at(idx);
    let value: f64 = num
        .parse()
        .map_err(|_| format!("invalid number in size: {s}"))?;
    if value < 0.0 {
        return Err(format!("size cannot be negative: {s}"));
    }
    let multiplier: u64 = match unit.to_ascii_uppercase().as_str() {
        "B" => 1,
        "KB" => 1024,
        "MB" => 1024 * 1024,
        "GB" => 1024 * 1024 * 1024,
        _ => return Err(format!("unknown size unit: {unit}")),
    };
    Ok((value as u64) * multiplier)
}

/// Parse a duration string (e.g. "5s", "30s", "100ms") into a Duration.
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let idx = s
        .find(|c: char| c.is_ascii_alphabetic())
        .ok_or_else(|| format!("missing unit in duration: {s}"))?;
    let (num, unit) = s.split_at(idx);
    let value: u64 = num
        .parse()
        .map_err(|_| format!("invalid number in duration: {s}"))?;
    match unit.to_ascii_lowercase().as_str() {
        "s" => Ok(Duration::from_secs(value)),
        "ms" => Ok(Duration::from_millis(value)),
        "m" => Ok(Duration::from_secs(value * 60)),
        _ => Err(format!("unknown duration unit: {unit}")),
    }
}

/// Parse a data rate string (e.g. "100b/s") into bytes per second.
pub fn parse_data_rate(s: &str) -> Result<u64, String> {
    let s = s.trim().to_ascii_lowercase();
    let prefix = s
        .strip_suffix("/s")
        .ok_or_else(|| format!("invalid data rate format: {s} (expected e.g. '100b/s')"))?;
    parse_size(prefix)
}

// ---------------------------------------------------------------------------
// Protection error
// ---------------------------------------------------------------------------

/// Errors raised by CGI protection checks.
#[derive(Debug)]
pub enum ProtectionError {
    /// Request URL exceeds the configured maximum length (HTTP 414).
    UrlTooLong {
        length: usize,
        max: usize,
    },
    /// Query string exceeds the configured maximum length (HTTP 414).
    QueryStringTooLong {
        length: usize,
        max: usize,
    },
    /// Total header size exceeds the configured limit (HTTP 431).
    HeadersTooLarge {
        size: usize,
        max: usize,
    },
    /// Number of headers exceeds the configured limit (HTTP 431).
    TooManyHeaders {
        count: usize,
        max: usize,
    },
    /// Request body exceeds the configured maximum size (HTTP 413).
    BodyTooLarge {
        size: usize,
        max: usize,
    },
    /// Generated response exceeds the configured maximum size (HTTP 500).
    ResponseTooLarge {
        size: usize,
        max: usize,
    },
    /// Path traversal attempt detected (`..` components) (HTTP 403).
    PathTraversal {
        path: String,
    },
    /// Access to a dotfile or hidden directory blocked (HTTP 403).
    DotfileAccess {
        path: String,
    },
    /// Request targets a file with a blocked extension (HTTP 403).
    BlockedExtension {
        path: String,
        extension: String,
    },
    /// Null byte (`\0`) found in request input (HTTP 400).
    NullByteDetected {
        location: String,
    },
    /// Non-printable control character found in a header value (HTTP 400).
    NonPrintableHeader {
        header: String,
    },
    /// Declared Content-Length does not match actual body size (HTTP 400).
    ContentLengthMismatch {
        declared: usize,
        actual: usize,
    },
    /// Declared Content-Length exceeds the configured body size limit (HTTP 413).
    ContentLengthOverflow {
        declared: usize,
        max: usize,
    },
    /// Per-IP connection limit exceeded -- mitigates connection flooding (HTTP 429).
    TooManyConnectionsFromIp {
        ip: String,
        count: usize,
        max: usize,
    },
    /// Global connection limit exceeded (HTTP 503).
    TooManyTotalConnections {
        count: usize,
        max: usize,
    },
    /// Body read timed out -- mitigates slowloris attacks (HTTP 408).
    BodyTimeout,
    /// Client sending data too slowly -- mitigates slow-read attacks (HTTP 408).
    DataRateTooSlow {
        rate_bps: u64,
        min_bps: u64,
    },
    /// Underlying I/O error during body reading (HTTP 500).
    IoError(String),
}

impl fmt::Display for ProtectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UrlTooLong { length, max } => {
                write!(f, "URL too long: {length} bytes (max {max})")
            }
            Self::QueryStringTooLong { length, max } => {
                write!(f, "query string too long: {length} bytes (max {max})")
            }
            Self::HeadersTooLarge { size, max } => {
                write!(f, "headers too large: {size} bytes (max {max})")
            }
            Self::TooManyHeaders { count, max } => {
                write!(f, "too many headers: {count} (max {max})")
            }
            Self::BodyTooLarge { size, max } => {
                write!(f, "request body too large: {size} bytes (max {max})")
            }
            Self::ResponseTooLarge { size, max } => {
                write!(f, "response too large: {size} bytes (max {max})")
            }
            Self::PathTraversal { path } => write!(f, "path traversal blocked: {path}"),
            Self::DotfileAccess { path } => write!(f, "dotfile access blocked: {path}"),
            Self::BlockedExtension { path, extension } => {
                write!(f, "blocked extension '{extension}' in: {path}")
            }
            Self::NullByteDetected { location } => {
                write!(f, "null byte detected in {location}")
            }
            Self::NonPrintableHeader { header } => {
                write!(f, "non-printable character in header: {header}")
            }
            Self::ContentLengthMismatch { declared, actual } => {
                write!(
                    f,
                    "content-length mismatch: declared {declared}, actual {actual}"
                )
            }
            Self::ContentLengthOverflow { declared, max } => {
                write!(
                    f,
                    "content-length exceeds maximum: {declared} bytes (max {max})"
                )
            }
            Self::TooManyConnectionsFromIp { ip, count, max } => {
                write!(f, "too many connections from {ip}: {count} (max {max})")
            }
            Self::TooManyTotalConnections { count, max } => {
                write!(f, "too many total connections: {count} (max {max})")
            }
            Self::BodyTimeout => write!(f, "body read timeout"),
            Self::DataRateTooSlow { rate_bps, min_bps } => {
                write!(f, "data rate too slow: {rate_bps} b/s (min {min_bps} b/s)")
            }
            Self::IoError(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}

impl ProtectionError {
    /// Map this error to an HTTP status code.
    pub fn http_status(&self) -> u16 {
        match self {
            Self::UrlTooLong { .. } => 414,
            Self::QueryStringTooLong { .. } => 414,
            Self::HeadersTooLarge { .. } => 431,
            Self::TooManyHeaders { .. } => 431,
            Self::BodyTooLarge { .. } | Self::ContentLengthOverflow { .. } => 413,
            Self::ResponseTooLarge { .. } | Self::IoError(_) => 500,
            Self::PathTraversal { .. }
            | Self::DotfileAccess { .. }
            | Self::BlockedExtension { .. } => 403,
            Self::NullByteDetected { .. }
            | Self::NonPrintableHeader { .. }
            | Self::ContentLengthMismatch { .. } => 400,
            Self::TooManyConnectionsFromIp { .. } => 429,
            Self::TooManyTotalConnections { .. } => 503,
            Self::BodyTimeout | Self::DataRateTooSlow { .. } => 408,
        }
    }
}

// ---------------------------------------------------------------------------
// Connection tracking
// ---------------------------------------------------------------------------

/// RAII guard for connection tracking. Removes the connection tracking file
/// from the filesystem when dropped, effectively deregistering the connection.
#[derive(Debug)]
pub struct ConnectionGuard {
    file_path: PathBuf,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file_path);
    }
}

/// Return the default connection tracking directory.
fn connection_tracking_dir() -> PathBuf {
    std::env::temp_dir().join("salata-cgi-connections")
}

/// Sanitize an IP address for use as a filename component.
fn sanitize_ip(ip: &str) -> String {
    ip.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Remove stale connection files (older than max_age).
fn cleanup_stale_connections(dir: &Path, max_age: Duration) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let age = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .unwrap_or(Duration::ZERO);
        if age > max_age {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Count connection files, optionally filtering by IP prefix.
fn count_connections(dir: &Path, ip_prefix: Option<&str>) -> usize {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        match ip_prefix {
            Some(prefix) if name.starts_with(prefix) => count += 1,
            Some(_) => {}
            None => count += 1,
        }
    }
    count
}

// ---------------------------------------------------------------------------
// CGI Protection
// ---------------------------------------------------------------------------

/// Holds parsed CGI protection configuration and provides validation methods.
pub struct CgiProtection {
    /// Maximum allowed URL length in bytes.
    pub max_url_length: usize,
    /// Maximum total size of all HTTP headers combined, in bytes.
    pub max_header_size: usize,
    /// Maximum number of HTTP headers allowed per request.
    pub max_header_count: usize,
    /// Maximum allowed query string length in bytes.
    pub max_query_string_length: usize,
    /// Maximum allowed request body size in bytes.
    pub max_body_size: usize,
    /// Maximum allowed response body size in bytes.
    pub max_response_size: usize,
    /// Whether to block access to dotfiles and hidden directories.
    pub block_dotfiles: bool,
    /// Whether to block path traversal attempts (`..` in URL paths).
    pub block_path_traversal: bool,
    /// File extensions that are denied access (e.g., `.toml`, `.env`, `.git`).
    pub blocked_extensions: Vec<String>,
    /// Whether to reject requests containing null bytes in URL, path, or query.
    pub block_null_bytes: bool,
    /// Whether to reject headers containing non-printable control characters.
    pub block_non_printable_headers: bool,
    /// Whether to validate that Content-Length matches the actual body size.
    pub validate_content_length_enabled: bool,
    /// Header timeout -- stored for completeness but unused in CGI mode
    /// since the web server handles header reading.
    pub _header_timeout: Duration,
    /// Maximum time allowed for reading the request body (anti-slowloris).
    pub body_timeout: Duration,
    /// Minimum acceptable data rate in bytes per second (anti-slow-read).
    pub min_data_rate_bps: u64,
    /// Maximum total execution time for a single request.
    pub max_execution_time: Duration,
    /// Maximum concurrent connections allowed from a single IP address.
    pub max_connections_per_ip: usize,
    /// Maximum total concurrent connections across all IPs.
    pub max_total_connections: usize,
    /// Maximum time allowed for sending the response.
    pub response_timeout: Duration,
    connection_dir: PathBuf,
}

impl CgiProtection {
    /// Create a `CgiProtection` from the config `[cgi]` section.
    pub fn from_config(cgi: &CgiSection) -> Result<Self, String> {
        Ok(Self {
            max_url_length: cgi.max_url_length,
            max_header_size: parse_size(&cgi.max_header_size)? as usize,
            max_header_count: cgi.max_header_count,
            max_query_string_length: cgi.max_query_string_length,
            max_body_size: parse_size(&cgi.max_body_size)? as usize,
            max_response_size: parse_size(&cgi.max_response_size)? as usize,
            block_dotfiles: cgi.block_dotfiles,
            block_path_traversal: cgi.block_path_traversal,
            blocked_extensions: cgi.blocked_extensions.clone(),
            block_null_bytes: cgi.block_null_bytes,
            block_non_printable_headers: cgi.block_non_printable_headers,
            validate_content_length_enabled: cgi.validate_content_length,
            _header_timeout: parse_duration(&cgi.header_timeout)?,
            body_timeout: parse_duration(&cgi.body_timeout)?,
            min_data_rate_bps: parse_data_rate(&cgi.min_data_rate)?,
            max_execution_time: parse_duration(&cgi.max_execution_time)?,
            max_connections_per_ip: cgi.max_connections_per_ip,
            max_total_connections: cgi.max_total_connections,
            response_timeout: parse_duration(&cgi.response_timeout)?,
            connection_dir: connection_tracking_dir(),
        })
    }

    /// Create with a custom connection tracking directory (for testing).
    #[cfg(test)]
    pub fn with_connection_dir(mut self, dir: PathBuf) -> Self {
        self.connection_dir = dir;
        self
    }

    // -- URL & query string --

    /// Validate URL length.
    pub fn validate_url(&self, url: &str) -> Result<(), ProtectionError> {
        if url.len() > self.max_url_length {
            return Err(ProtectionError::UrlTooLong {
                length: url.len(),
                max: self.max_url_length,
            });
        }
        Ok(())
    }

    /// Validate query string length.
    pub fn validate_query_string(&self, qs: &str) -> Result<(), ProtectionError> {
        if qs.len() > self.max_query_string_length {
            return Err(ProtectionError::QueryStringTooLong {
                length: qs.len(),
                max: self.max_query_string_length,
            });
        }
        Ok(())
    }

    // -- Path security --

    /// Validate a request path for traversal, dotfiles, and blocked extensions.
    pub fn validate_path(&self, path: &str) -> Result<(), ProtectionError> {
        // Path traversal: check for ".." components.
        if self.block_path_traversal && has_path_traversal(path) {
            return Err(ProtectionError::PathTraversal {
                path: path.to_string(),
            });
        }

        // Dotfile access: check for path components starting with '.'.
        if self.block_dotfiles && has_dotfile(path) {
            return Err(ProtectionError::DotfileAccess {
                path: path.to_string(),
            });
        }

        // Blocked extensions.
        for ext in &self.blocked_extensions {
            if path.ends_with(ext.as_str()) {
                return Err(ProtectionError::BlockedExtension {
                    path: path.to_string(),
                    extension: ext.clone(),
                });
            }
        }

        Ok(())
    }

    // -- Headers --

    /// Validate header count and total size.
    /// `headers` is a list of (name, value) pairs from the HTTP_* env vars.
    pub fn validate_headers(&self, headers: &[(String, String)]) -> Result<(), ProtectionError> {
        if headers.len() > self.max_header_count {
            return Err(ProtectionError::TooManyHeaders {
                count: headers.len(),
                max: self.max_header_count,
            });
        }

        let total_size: usize = headers.iter().map(|(k, v)| k.len() + v.len() + 4).sum();
        if total_size > self.max_header_size {
            return Err(ProtectionError::HeadersTooLarge {
                size: total_size,
                max: self.max_header_size,
            });
        }

        // Non-printable header check.
        if self.block_non_printable_headers {
            for (name, value) in headers {
                if has_non_printable(value) {
                    return Err(ProtectionError::NonPrintableHeader {
                        header: name.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    // -- Null bytes --

    /// Check input for null bytes.
    pub fn validate_null_bytes(&self, input: &str, location: &str) -> Result<(), ProtectionError> {
        if self.block_null_bytes && input.contains('\0') {
            return Err(ProtectionError::NullByteDetected {
                location: location.to_string(),
            });
        }
        Ok(())
    }

    // -- Content-Length --

    /// Validate that declared Content-Length matches the actual body size
    /// and does not exceed max_body_size.
    pub fn validate_content_length(
        &self,
        declared: Option<usize>,
        actual: usize,
    ) -> Result<(), ProtectionError> {
        if let Some(declared) = declared {
            if declared > self.max_body_size {
                return Err(ProtectionError::ContentLengthOverflow {
                    declared,
                    max: self.max_body_size,
                });
            }
            if self.validate_content_length_enabled && declared != actual {
                return Err(ProtectionError::ContentLengthMismatch { declared, actual });
            }
        }
        Ok(())
    }

    // -- Body size --

    /// Validate request body size.
    pub fn validate_body_size(&self, size: usize) -> Result<(), ProtectionError> {
        if size > self.max_body_size {
            return Err(ProtectionError::BodyTooLarge {
                size,
                max: self.max_body_size,
            });
        }
        Ok(())
    }

    // -- Response size --

    /// Validate response body size.
    pub fn validate_response_size(&self, size: usize) -> Result<(), ProtectionError> {
        if size > self.max_response_size {
            return Err(ProtectionError::ResponseTooLarge {
                size,
                max: self.max_response_size,
            });
        }
        Ok(())
    }

    // -- Connection tracking --

    /// Register a connection from `remote_ip`. Returns a guard that
    /// deregisters the connection on drop. Fails if per-IP or total
    /// connection limits are exceeded.
    pub fn register_connection(&self, remote_ip: &str) -> Result<ConnectionGuard, ProtectionError> {
        let _ = fs::create_dir_all(&self.connection_dir);

        // Clean up stale entries (older than execution_time + response_timeout + buffer).
        let stale_age = self.max_execution_time + self.response_timeout + Duration::from_secs(10);
        cleanup_stale_connections(&self.connection_dir, stale_age);

        let sanitized_ip = sanitize_ip(remote_ip);
        let ip_prefix = format!("{sanitized_ip}_");

        // Check per-IP limit.
        let ip_count = count_connections(&self.connection_dir, Some(&ip_prefix));
        if ip_count >= self.max_connections_per_ip {
            return Err(ProtectionError::TooManyConnectionsFromIp {
                ip: remote_ip.to_string(),
                count: ip_count,
                max: self.max_connections_per_ip,
            });
        }

        // Check total limit.
        let total_count = count_connections(&self.connection_dir, None);
        if total_count >= self.max_total_connections {
            return Err(ProtectionError::TooManyTotalConnections {
                count: total_count,
                max: self.max_total_connections,
            });
        }

        // Create connection file.
        let pid = std::process::id();
        let file_name = format!("{sanitized_ip}_{pid}");
        let file_path = self.connection_dir.join(file_name);
        let _ = fs::write(&file_path, b"");

        Ok(ConnectionGuard { file_path })
    }
}

// ---------------------------------------------------------------------------
// Body reading with timeout and data rate enforcement
// ---------------------------------------------------------------------------

/// Read the request body from stdin with timeout and data rate enforcement.
pub fn read_request_body(
    content_length: Option<usize>,
    max_body_size: usize,
    body_timeout: Duration,
    min_data_rate_bps: u64,
) -> Result<Vec<u8>, ProtectionError> {
    let to_read = match content_length {
        Some(len) => {
            if len > max_body_size {
                return Err(ProtectionError::BodyTooLarge {
                    size: len,
                    max: max_body_size,
                });
            }
            len
        }
        None => return Ok(Vec::new()),
    };

    if to_read == 0 {
        return Ok(Vec::new());
    }

    // Use a channel for timeout-based reading.
    let (tx, rx) = std::sync::mpsc::channel();
    let max_size = max_body_size;
    let min_rate = min_data_rate_bps;

    std::thread::spawn(move || {
        let result = read_stdin_with_rate_check(to_read, max_size, min_rate);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(body_timeout) {
        Ok(result) => result,
        Err(_) => Err(ProtectionError::BodyTimeout),
    }
}

/// Read from stdin with size limit and data rate checking.
fn read_stdin_with_rate_check(
    to_read: usize,
    max_size: usize,
    min_rate_bps: u64,
) -> Result<Vec<u8>, ProtectionError> {
    let mut body = Vec::with_capacity(to_read.min(64 * 1024));
    let mut buf = [0u8; 8192];
    let start = Instant::now();

    let mut stdin = io::stdin().lock();
    while body.len() < to_read {
        let remaining = to_read - body.len();
        let chunk_size = buf.len().min(remaining);
        let n = stdin
            .read(&mut buf[..chunk_size])
            .map_err(|e| ProtectionError::IoError(e.to_string()))?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);

        if body.len() > max_size {
            return Err(ProtectionError::BodyTooLarge {
                size: body.len(),
                max: max_size,
            });
        }

        // Check data rate after 2 seconds of reading.
        let elapsed = start.elapsed();
        if elapsed.as_secs() >= 2 && min_rate_bps > 0 {
            let rate = body.len() as u64 / elapsed.as_secs().max(1);
            if rate < min_rate_bps {
                return Err(ProtectionError::DataRateTooSlow {
                    rate_bps: rate,
                    min_bps: min_rate_bps,
                });
            }
        }
    }

    Ok(body)
}

// ---------------------------------------------------------------------------
// HTTP header collection from CGI env
// ---------------------------------------------------------------------------

/// Collect HTTP headers from CGI environment variables.
/// In CGI, headers are passed as env vars prefixed with `HTTP_`.
pub fn collect_http_headers() -> Vec<(String, String)> {
    let mut headers = Vec::new();
    for (key, value) in std::env::vars() {
        if let Some(header_name) = key.strip_prefix("HTTP_") {
            // Convert HTTP_ACCEPT_LANGUAGE → Accept-Language.
            let name = header_name
                .split('_')
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join("-");
            headers.push((name, value));
        }
    }
    headers
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Check if a path contains traversal sequences ("..").
fn has_path_traversal(path: &str) -> bool {
    // Normalize separators.
    let normalized = path.replace('\\', "/");
    for component in normalized.split('/') {
        if component == ".." {
            return true;
        }
    }
    // Also check URL-encoded traversal.
    let decoded = normalized.replace("%2e", ".").replace("%2E", ".");
    if decoded != normalized {
        for component in decoded.split('/') {
            if component == ".." {
                return true;
            }
        }
    }
    false
}

/// Check if any path component starts with a dot (dotfile).
/// Ignores "." and ".." which are navigation, not dotfiles.
fn has_dotfile(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    for component in normalized.split('/') {
        if component.starts_with('.') && component.len() > 1 && component != ".." {
            return true;
        }
    }
    false
}

/// Check if a string contains non-printable characters (control chars except
/// tab, carriage return, and newline).
fn has_non_printable(s: &str) -> bool {
    s.bytes()
        .any(|b| b < 0x20 && b != b'\t' && b != b'\r' && b != b'\n')
}

/// HTTP status reason phrase for common codes.
pub fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Content Too Large",
        414 => "URI Too Long",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Parsing tests --

    #[test]
    fn parse_size_bytes() {
        assert_eq!(parse_size("100B").unwrap(), 100);
    }

    #[test]
    fn parse_size_kilobytes() {
        assert_eq!(parse_size("8KB").unwrap(), 8 * 1024);
    }

    #[test]
    fn parse_size_megabytes() {
        assert_eq!(parse_size("10MB").unwrap(), 10 * 1024 * 1024);
    }

    #[test]
    fn parse_size_gigabytes() {
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_size_invalid_unit() {
        assert!(parse_size("50XB").is_err());
    }

    #[test]
    fn parse_size_no_unit() {
        assert!(parse_size("50").is_err());
    }

    #[test]
    fn parse_size_case_insensitive() {
        assert_eq!(parse_size("10mb").unwrap(), 10 * 1024 * 1024);
    }

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
    }

    #[test]
    fn parse_duration_milliseconds() {
        assert_eq!(parse_duration("100ms").unwrap(), Duration::from_millis(100));
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
    }

    #[test]
    fn parse_duration_invalid_unit() {
        assert!(parse_duration("5h").is_err());
    }

    #[test]
    fn parse_duration_no_unit() {
        assert!(parse_duration("5").is_err());
    }

    #[test]
    fn parse_data_rate_bytes_per_sec() {
        assert_eq!(parse_data_rate("100b/s").unwrap(), 100);
    }

    #[test]
    fn parse_data_rate_kilobytes_per_sec() {
        assert_eq!(parse_data_rate("1KB/s").unwrap(), 1024);
    }

    #[test]
    fn parse_data_rate_invalid_format() {
        assert!(parse_data_rate("100b").is_err());
    }

    // -- from_config --

    #[test]
    fn from_default_config() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert_eq!(prot.max_url_length, 2048);
        assert_eq!(prot.max_header_size, 8 * 1024);
        assert_eq!(prot.max_header_count, 50);
        assert_eq!(prot.max_query_string_length, 2048);
        assert_eq!(prot.max_body_size, 10 * 1024 * 1024);
        assert_eq!(prot.max_response_size, 50 * 1024 * 1024);
        assert!(prot.block_dotfiles);
        assert!(prot.block_path_traversal);
        assert_eq!(prot.blocked_extensions.len(), 4);
        assert!(prot.block_null_bytes);
        assert!(prot.block_non_printable_headers);
        assert!(prot.validate_content_length_enabled);
        assert_eq!(prot._header_timeout, Duration::from_secs(5));
        assert_eq!(prot.body_timeout, Duration::from_secs(30));
        assert_eq!(prot.min_data_rate_bps, 100);
        assert_eq!(prot.max_execution_time, Duration::from_secs(30));
        assert_eq!(prot.max_connections_per_ip, 20);
        assert_eq!(prot.max_total_connections, 200);
        assert_eq!(prot.response_timeout, Duration::from_secs(60));
    }

    // -- URL validation --

    #[test]
    fn url_within_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_url("/index.slt").is_ok());
    }

    #[test]
    fn url_exceeds_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let long_url = "a".repeat(3000);
        let err = prot.validate_url(&long_url).unwrap_err();
        assert_eq!(err.http_status(), 414);
    }

    // -- Query string validation --

    #[test]
    fn query_string_within_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_query_string("foo=bar&baz=qux").is_ok());
    }

    #[test]
    fn query_string_exceeds_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let long_qs = "x=".to_string() + &"a".repeat(3000);
        let err = prot.validate_query_string(&long_qs).unwrap_err();
        assert_eq!(err.http_status(), 414);
    }

    // -- Path security --

    #[test]
    fn path_normal() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/pages/index.slt").is_ok());
    }

    #[test]
    fn path_traversal_blocked() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let err = prot
            .validate_path("/pages/../../../etc/passwd")
            .unwrap_err();
        assert_eq!(err.http_status(), 403);
        assert!(matches!(err, ProtectionError::PathTraversal { .. }));
    }

    #[test]
    fn path_traversal_simple() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/pages/..").is_err());
    }

    #[test]
    fn path_traversal_encoded() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/pages/%2e%2e/secret").is_err());
    }

    #[test]
    fn path_traversal_disabled() {
        let cgi = CgiSection {
            block_path_traversal: false,
            ..Default::default()
        };
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/pages/../secret").is_ok());
    }

    #[test]
    fn dotfile_blocked() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let err = prot.validate_path("/pages/.env").unwrap_err();
        assert_eq!(err.http_status(), 403);
        assert!(matches!(err, ProtectionError::DotfileAccess { .. }));
    }

    #[test]
    fn dotfile_hidden_directory() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/.git/config").is_err());
    }

    #[test]
    fn dotfile_disabled() {
        let cgi = CgiSection {
            block_dotfiles: false,
            ..Default::default()
        };
        let prot = CgiProtection::from_config(&cgi).unwrap();
        // .hidden is a dotfile but not a blocked extension.
        assert!(prot.validate_path("/.hidden").is_ok());
        assert!(prot.validate_path("/.gitignore").is_ok());
    }

    #[test]
    fn blocked_extension_toml() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let err = prot.validate_path("/config.toml").unwrap_err();
        assert_eq!(err.http_status(), 403);
        assert!(matches!(err, ProtectionError::BlockedExtension { .. }));
    }

    #[test]
    fn blocked_extension_env() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/app/.env").is_err());
    }

    #[test]
    fn blocked_extension_git() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/.git").is_err());
    }

    #[test]
    fn blocked_extension_log() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/logs/error.log").is_err());
    }

    #[test]
    fn allowed_extension_slt() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/index.slt").is_ok());
    }

    #[test]
    fn allowed_extension_html() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_path("/page.html").is_ok());
    }

    // -- Null bytes --

    #[test]
    fn null_bytes_clean() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_null_bytes("/index.slt", "URL").is_ok());
    }

    #[test]
    fn null_bytes_detected() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let err = prot
            .validate_null_bytes("/index.slt\0.jpg", "URL")
            .unwrap_err();
        assert_eq!(err.http_status(), 400);
    }

    #[test]
    fn null_bytes_disabled() {
        let cgi = CgiSection {
            block_null_bytes: false,
            ..Default::default()
        };
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_null_bytes("/index\0", "URL").is_ok());
    }

    // -- Non-printable headers --

    #[test]
    fn headers_clean() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("Accept".to_string(), "text/html".to_string()),
        ];
        assert!(prot.validate_headers(&headers).is_ok());
    }

    #[test]
    fn headers_with_tab_ok() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let headers = vec![("Accept".to_string(), "text/html\t, text/plain".to_string())];
        assert!(prot.validate_headers(&headers).is_ok());
    }

    #[test]
    fn headers_with_control_char() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let headers = vec![("Accept".to_string(), "text/html\x01".to_string())];
        let err = prot.validate_headers(&headers).unwrap_err();
        assert_eq!(err.http_status(), 400);
        assert!(matches!(err, ProtectionError::NonPrintableHeader { .. }));
    }

    #[test]
    fn headers_non_printable_disabled() {
        let cgi = CgiSection {
            block_non_printable_headers: false,
            ..Default::default()
        };
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let headers = vec![("Accept".to_string(), "text\x01html".to_string())];
        assert!(prot.validate_headers(&headers).is_ok());
    }

    // -- Header count & size --

    #[test]
    fn too_many_headers() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let headers: Vec<(String, String)> = (0..51)
            .map(|i| (format!("X-Header-{i}"), "value".to_string()))
            .collect();
        let err = prot.validate_headers(&headers).unwrap_err();
        assert!(matches!(err, ProtectionError::TooManyHeaders { .. }));
    }

    #[test]
    fn headers_within_count_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let headers: Vec<(String, String)> = (0..50)
            .map(|i| (format!("X-H-{i}"), "v".to_string()))
            .collect();
        assert!(prot.validate_headers(&headers).is_ok());
    }

    #[test]
    fn headers_too_large() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        // Default max_header_size = 8KB = 8192 bytes.
        let big_value = "x".repeat(9000);
        let headers = vec![("Host".to_string(), big_value)];
        let err = prot.validate_headers(&headers).unwrap_err();
        assert!(matches!(err, ProtectionError::HeadersTooLarge { .. }));
    }

    // -- Content-Length validation --

    #[test]
    fn content_length_matching() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_content_length(Some(100), 100).is_ok());
    }

    #[test]
    fn content_length_mismatch() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let err = prot.validate_content_length(Some(100), 50).unwrap_err();
        assert!(matches!(err, ProtectionError::ContentLengthMismatch { .. }));
    }

    #[test]
    fn content_length_overflow() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let err = prot
            .validate_content_length(Some(100 * 1024 * 1024), 0)
            .unwrap_err();
        assert!(matches!(err, ProtectionError::ContentLengthOverflow { .. }));
    }

    #[test]
    fn content_length_none_ok() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_content_length(None, 0).is_ok());
    }

    #[test]
    fn content_length_validation_disabled() {
        let cgi = CgiSection {
            validate_content_length: false,
            ..Default::default()
        };
        let prot = CgiProtection::from_config(&cgi).unwrap();
        // Mismatch allowed when validation disabled.
        assert!(prot.validate_content_length(Some(100), 50).is_ok());
    }

    // -- Body size --

    #[test]
    fn body_size_within_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_body_size(1024).is_ok());
    }

    #[test]
    fn body_size_exceeds_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let err = prot.validate_body_size(20 * 1024 * 1024).unwrap_err();
        assert!(matches!(err, ProtectionError::BodyTooLarge { .. }));
    }

    // -- Response size --

    #[test]
    fn response_size_within_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        assert!(prot.validate_response_size(1024).is_ok());
    }

    #[test]
    fn response_size_exceeds_limit() {
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi).unwrap();
        let err = prot.validate_response_size(100 * 1024 * 1024).unwrap_err();
        assert!(matches!(err, ProtectionError::ResponseTooLarge { .. }));
    }

    // -- Connection tracking --

    fn temp_conn_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "salata_cgi_test_{name}_{}_{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn cleanup_dir(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn connection_register_and_deregister() {
        let dir = temp_conn_dir("register");
        let cgi = CgiSection::default();
        let prot = CgiProtection::from_config(&cgi)
            .unwrap()
            .with_connection_dir(dir.clone());

        {
            let _guard = prot.register_connection("192.168.1.1").unwrap();
            assert_eq!(count_connections(&dir, None), 1);
        }
        // Guard dropped — file should be removed.
        assert_eq!(count_connections(&dir, None), 0);

        cleanup_dir(&dir);
    }

    #[test]
    fn connection_per_ip_limit() {
        let dir = temp_conn_dir("per_ip");
        let cgi = CgiSection {
            max_connections_per_ip: 2,
            ..Default::default()
        };
        let prot = CgiProtection::from_config(&cgi)
            .unwrap()
            .with_connection_dir(dir.clone());

        // Create fake connection files for the same IP.
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("192_168_1_1_99990"), b"").unwrap();
        fs::write(dir.join("192_168_1_1_99991"), b"").unwrap();

        let err = prot.register_connection("192.168.1.1").unwrap_err();
        assert!(matches!(
            err,
            ProtectionError::TooManyConnectionsFromIp { .. }
        ));
        assert_eq!(err.http_status(), 429);

        // Different IP should work.
        let _guard = prot.register_connection("10.0.0.1").unwrap();

        cleanup_dir(&dir);
    }

    #[test]
    fn connection_total_limit() {
        let dir = temp_conn_dir("total");
        let cgi = CgiSection {
            max_total_connections: 3,
            ..Default::default()
        };
        let prot = CgiProtection::from_config(&cgi)
            .unwrap()
            .with_connection_dir(dir.clone());

        // Create fake connection files.
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("10_0_0_1_99990"), b"").unwrap();
        fs::write(dir.join("10_0_0_2_99991"), b"").unwrap();
        fs::write(dir.join("10_0_0_3_99992"), b"").unwrap();

        let err = prot.register_connection("192.168.1.1").unwrap_err();
        assert!(matches!(
            err,
            ProtectionError::TooManyTotalConnections { .. }
        ));
        assert_eq!(err.http_status(), 503);

        cleanup_dir(&dir);
    }

    #[test]
    fn connection_stale_cleanup() {
        let dir = temp_conn_dir("stale");
        let _ = fs::create_dir_all(&dir);

        // Create a stale file (set modified time to past by creating it normally).
        let stale_file = dir.join("old_ip_99999");
        fs::write(&stale_file, b"").unwrap();

        // Clean up with zero max age — everything is stale.
        cleanup_stale_connections(&dir, Duration::ZERO);

        assert!(!stale_file.exists());

        cleanup_dir(&dir);
    }

    // -- Helper function tests --

    #[test]
    fn path_traversal_detection() {
        assert!(has_path_traversal("/foo/../bar"));
        assert!(has_path_traversal("/../"));
        assert!(has_path_traversal("/foo/.."));
        assert!(!has_path_traversal("/foo/bar"));
        assert!(!has_path_traversal("/foo/...bar")); // triple dot is not traversal
    }

    #[test]
    fn path_traversal_encoded_detection() {
        assert!(has_path_traversal("/foo/%2e%2e/bar"));
        assert!(has_path_traversal("/foo/%2E%2E/bar"));
    }

    #[test]
    fn dotfile_detection() {
        assert!(has_dotfile("/.env"));
        assert!(has_dotfile("/foo/.gitignore"));
        assert!(has_dotfile("/.git/config"));
        assert!(!has_dotfile("/foo/bar"));
        assert!(!has_dotfile("/index.slt"));
    }

    #[test]
    fn non_printable_detection() {
        assert!(!has_non_printable("normal text"));
        assert!(!has_non_printable("text\twith\ttabs"));
        assert!(!has_non_printable("text\r\nwith\nnewlines"));
        assert!(has_non_printable("text\x01control"));
        assert!(has_non_printable("text\x00null"));
        assert!(has_non_printable("\x07bell"));
    }

    #[test]
    fn sanitize_ip_v4() {
        assert_eq!(sanitize_ip("192.168.1.1"), "192_168_1_1");
    }

    #[test]
    fn sanitize_ip_v6() {
        assert_eq!(sanitize_ip("::1"), "__1");
        assert_eq!(sanitize_ip("2001:db8::1"), "2001_db8__1");
    }

    // -- Status text --

    #[test]
    fn status_text_common_codes() {
        assert_eq!(status_text(200), "OK");
        assert_eq!(status_text(403), "Forbidden");
        assert_eq!(status_text(404), "Not Found");
        assert_eq!(status_text(500), "Internal Server Error");
    }

    // -- Error HTTP status mapping --

    #[test]
    fn error_status_codes() {
        assert_eq!(
            ProtectionError::UrlTooLong {
                length: 3000,
                max: 2048
            }
            .http_status(),
            414
        );
        assert_eq!(
            ProtectionError::BodyTooLarge { size: 100, max: 50 }.http_status(),
            413
        );
        assert_eq!(
            ProtectionError::PathTraversal { path: "..".into() }.http_status(),
            403
        );
        assert_eq!(
            ProtectionError::NullByteDetected {
                location: "url".into()
            }
            .http_status(),
            400
        );
        assert_eq!(ProtectionError::BodyTimeout.http_status(), 408);
        assert_eq!(
            ProtectionError::TooManyConnectionsFromIp {
                ip: "x".into(),
                count: 1,
                max: 1
            }
            .http_status(),
            429
        );
        assert_eq!(
            ProtectionError::TooManyTotalConnections { count: 1, max: 1 }.http_status(),
            503
        );
    }

    // -- Error Display --

    #[test]
    fn error_display_messages() {
        let err = ProtectionError::UrlTooLong {
            length: 3000,
            max: 2048,
        };
        let msg = err.to_string();
        assert!(msg.contains("3000"));
        assert!(msg.contains("2048"));

        let err = ProtectionError::PathTraversal {
            path: "/foo/..".into(),
        };
        assert!(err.to_string().contains("/foo/.."));
    }
}
