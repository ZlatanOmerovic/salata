//! CGI bridge for Salata -- connects web servers (nginx, Apache) to the Salata
//! engine via the Common Gateway Interface protocol.
//!
//! Receives a request through CGI environment variables (`SCRIPT_FILENAME`,
//! `PATH_INFO`, `QUERY_STRING`, etc.), runs the targeted `.slt` file through
//! salata-core, and writes a complete HTTP response (status line, headers, body)
//! to stdout. Includes built-in security protections against common CGI attack
//! vectors (slowloris, path traversal, null bytes, request size limits, and
//! connection rate limiting).

use std::path::{Path, PathBuf};
use std::process;

use salata_core::config::SalataConfig;
use salata_core::context::ExecutionContext;
use salata_core::logging::{LogLevel, Logger};
use salata_core::runtime::CgiEnv;

use salata_cgi::protection::{self, status_text, CgiProtection, ProtectionError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a CGI environment variable, returning empty string if unset.
fn env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Output a CGI error response and exit.
fn output_error_response(status: u16, message: &str, display_errors: bool) {
    let text = status_text(status);
    println!("Status: {status} {text}");
    println!("Content-Type: text/html; charset=utf-8");
    println!();
    if display_errors {
        println!(
            "<html><body><h1>{status} {text}</h1><p>{}</p></body></html>",
            html_escape(message)
        );
    } else {
        println!("<html><body><h1>{status} {text}</h1></body></html>");
    }
}

/// Output a protection error as a CGI response and exit.
fn handle_protection_error(err: &ProtectionError, display_errors: bool) -> ! {
    output_error_response(err.http_status(), &err.to_string(), display_errors);
    process::exit(0) // CGI exits cleanly — the error is in the response.
}

/// Output a successful CGI response with resolved directives.
fn output_response(
    status: u16,
    directives: &salata_core::directives::ResolvedDirectives,
    html: &str,
    default_content_type: &str,
) {
    let text = status_text(status);
    println!("Status: {status} {text}");

    // Content-Type header.
    let ct = directives
        .content_type
        .as_deref()
        .unwrap_or(default_content_type);
    println!("Content-Type: {ct}");

    // Redirect.
    if let Some(ref location) = directives.redirect {
        // Override status for redirect if not already a 3xx.
        if !(300..400).contains(&status) {
            println!("Status: 302 Found");
        }
        println!("Location: {location}");
    }

    // Custom headers.
    for (name, value) in &directives.headers {
        println!("{name}: {value}");
    }

    // Cookies.
    for cookie in &directives.cookies {
        println!("Set-Cookie: {cookie}");
    }

    // Blank line separates headers from body.
    println!();
    print!("{html}");
}

/// Determine the .slt file to process from CGI environment variables.
fn determine_slt_file() -> Option<PathBuf> {
    // 1. SCRIPT_FILENAME (set by nginx/Apache).
    let script_filename = env_var("SCRIPT_FILENAME");
    if !script_filename.is_empty() {
        return Some(PathBuf::from(script_filename));
    }

    // 2. PATH_TRANSLATED.
    let path_translated = env_var("PATH_TRANSLATED");
    if !path_translated.is_empty() {
        return Some(PathBuf::from(path_translated));
    }

    // 3. DOCUMENT_ROOT + PATH_INFO.
    let document_root = env_var("DOCUMENT_ROOT");
    let path_info = env_var("PATH_INFO");
    if !document_root.is_empty() && !path_info.is_empty() {
        return Some(PathBuf::from(document_root).join(path_info.trim_start_matches('/')));
    }

    None
}

/// Build a CgiEnv from the current CGI environment variables.
fn build_cgi_env() -> CgiEnv {
    CgiEnv {
        request_method: env_var("REQUEST_METHOD"),
        query_string: env_var("QUERY_STRING"),
        content_type: env_var("CONTENT_TYPE"),
        content_length: env_var("CONTENT_LENGTH"),
        http_host: env_var("HTTP_HOST"),
        http_cookie: env_var("HTTP_COOKIE"),
        remote_addr: env_var("REMOTE_ADDR"),
        request_uri: env_var("REQUEST_URI"),
        path_info: env_var("PATH_INFO"),
        server_name: env_var("SERVER_NAME"),
        server_port: env_var("SERVER_PORT"),
        http_authorization: env_var("HTTP_AUTHORIZATION"),
        macro_data_dir: None,
    }
}

fn log_error(logger: &Option<Logger>, file: &Path, message: &str) {
    if let Some(ref logger) = logger {
        let _ = logger.log_runtime(
            LogLevel::Error,
            "salata-cgi",
            &file.display().to_string(),
            None,
            message,
        );
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

/// Entry point for the CGI binary.
///
/// Loads configuration, initializes protections, validates the incoming
/// request against all security checks, determines the `.slt` file to
/// process from CGI environment variables, runs it through salata-core,
/// and writes the HTTP response to stdout.
fn main() {
    // 1. Load config.
    let config_path: Option<PathBuf> = std::env::args()
        .nth(1)
        .filter(|a| !a.is_empty())
        .map(PathBuf::from);
    let config = match SalataConfig::locate(config_path.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            output_error_response(500, &format!("configuration error: {e}"), true);
            process::exit(0);
        }
    };

    let display_errors = config.salata.display_errors;

    // 2. Set up logger (best-effort).
    let logger = Logger::new(&config.logging).ok();

    // 3. Create CGI protections from config.
    let protection = match CgiProtection::from_config(&config.cgi) {
        Ok(p) => p,
        Err(e) => {
            output_error_response(
                500,
                &format!("protection config error: {e}"),
                display_errors,
            );
            process::exit(0);
        }
    };

    // 4. Set up execution time limit.
    let exec_timeout = protection.max_execution_time;
    std::thread::spawn(move || {
        std::thread::sleep(exec_timeout);
        // Output timeout error — may corrupt in-progress output, but that's
        // expected for a timeout. The web server handles incomplete responses.
        eprintln!("salata-cgi: execution time limit exceeded ({exec_timeout:?})");
        process::exit(1);
    });

    // 5. Read CGI environment variables.
    let request_uri = env_var("REQUEST_URI");
    let query_string = env_var("QUERY_STRING");
    let remote_addr = env_var("REMOTE_ADDR");
    let path_info = env_var("PATH_INFO");
    let content_length_str = env_var("CONTENT_LENGTH");
    let content_length: Option<usize> = content_length_str.parse().ok();

    // 6. Run request protections.
    // Null byte checks.
    if let Err(e) = protection.validate_null_bytes(&request_uri, "URL") {
        handle_protection_error(&e, display_errors);
    }
    if let Err(e) = protection.validate_null_bytes(&query_string, "query string") {
        handle_protection_error(&e, display_errors);
    }
    if let Err(e) = protection.validate_null_bytes(&path_info, "path") {
        handle_protection_error(&e, display_errors);
    }

    // URL and query string length.
    if let Err(e) = protection.validate_url(&request_uri) {
        handle_protection_error(&e, display_errors);
    }
    if let Err(e) = protection.validate_query_string(&query_string) {
        handle_protection_error(&e, display_errors);
    }

    // Path security.
    if let Err(e) = protection.validate_path(&path_info) {
        handle_protection_error(&e, display_errors);
    }

    // Header validation.
    let headers = protection::collect_http_headers();
    if let Err(e) = protection.validate_headers(&headers) {
        handle_protection_error(&e, display_errors);
    }

    // Content-Length pre-check (overflow).
    if let Some(cl) = content_length {
        if let Err(e) = protection.validate_content_length(Some(cl), cl) {
            handle_protection_error(&e, display_errors);
        }
    }

    // 7. Register connection.
    let _connection_guard = match protection.register_connection(&remote_addr) {
        Ok(guard) => Some(guard),
        Err(e) => {
            if let Some(ref logger) = logger {
                let _ = logger.log_runtime(
                    LogLevel::Warn,
                    "salata-cgi",
                    &request_uri,
                    None,
                    &e.to_string(),
                );
            }
            handle_protection_error(&e, display_errors);
        }
    };

    // 8. Read request body with timeout and data rate enforcement.
    let body = match protection::read_request_body(
        content_length,
        protection.max_body_size,
        protection.body_timeout,
        protection.min_data_rate_bps,
    ) {
        Ok(b) => b,
        Err(e) => {
            if let Some(ref logger) = logger {
                let _ = logger.log_runtime(
                    LogLevel::Error,
                    "salata-cgi",
                    &request_uri,
                    None,
                    &e.to_string(),
                );
            }
            handle_protection_error(&e, display_errors);
        }
    };

    // Validate body size and content-length match.
    if let Err(e) = protection.validate_body_size(body.len()) {
        handle_protection_error(&e, display_errors);
    }
    if let Err(e) = protection.validate_content_length(content_length, body.len()) {
        handle_protection_error(&e, display_errors);
    }

    // 9. Determine which .slt file to process.
    let slt_file = match determine_slt_file() {
        Some(f) => f,
        None => {
            output_error_response(
                500,
                "cannot determine .slt file path (SCRIPT_FILENAME, PATH_TRANSLATED, and DOCUMENT_ROOT+PATH_INFO are all empty)",
                display_errors,
            );
            process::exit(0);
        }
    };

    if !slt_file.exists() {
        output_error_response(
            404,
            &format!("file not found: {}", slt_file.display()),
            display_errors,
        );
        process::exit(0);
    }

    // 10. Build CgiEnv and process the file.
    let cgi_env = build_cgi_env();
    let default_ct = config.salata.default_content_type.clone();

    match salata_core::process_file(&slt_file, &config, &cgi_env, ExecutionContext::Cgi) {
        Ok(result) => {
            // Per spec: any runtime failure → 500 (overrides #status).
            let status = if result.had_runtime_errors {
                500
            } else {
                result.directives.status
            };

            // Validate response size.
            if let Err(e) = protection.validate_response_size(result.html.len()) {
                log_error(&logger, &slt_file, &e.to_string());
                handle_protection_error(&e, display_errors);
            }

            output_response(status, &result.directives, &result.html, &default_ct);

            if result.had_runtime_errors {
                log_error(&logger, &slt_file, "one or more runtime errors occurred");
            }
        }
        Err(e) => {
            log_error(&logger, &slt_file, &e.to_string());
            output_error_response(500, &e.to_string(), display_errors);
        }
    }

    // Connection guard is dropped here, deregistering the connection.
}
