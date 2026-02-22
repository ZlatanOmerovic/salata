//! Standalone development server for Salata with hot reload support.
//!
//! Serves directories of `.slt` template files and static assets over HTTP
//! using actix-web. Processes `.slt` files through salata-core on each
//! request; all other files (HTML, CSS, JS, images, fonts, etc.) are served
//! as-is with MIME types inferred from their extensions. Includes the full
//! set of CGI security protections from `salata-cgi` (rate limiting, input
//! validation, path traversal blocking, etc.).
//!
//! # Usage
//!
//! ```text
//! salata-server [OPTIONS] <path>
//! ```
//!
//! Where `<path>` is a directory to serve or a single `.slt` file.

mod hot_reload;
mod static_files;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use actix_web::cookie::Cookie;
use actix_web::web::{self, Bytes};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer};

use salata_cgi::protection::{CgiProtection, ProtectionError};
use salata_core::cache::ParseCache;
use salata_core::config::SalataConfig;
use salata_core::context::ExecutionContext;
use salata_core::logging::{LogLevel, Logger};
use salata_core::runtime::CgiEnv;

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// Shared application state passed to every request handler via actix-web's
/// `Data` extractor.
struct AppState {
    /// Parsed and validated Salata configuration.
    config: SalataConfig,
    /// CGI protection settings for request validation.
    protection: CgiProtection,
    /// Parsed file cache, shared with the hot-reload watcher so it can
    /// invalidate entries when files change on disk.
    #[allow(dead_code)]
    cache: Arc<ParseCache>,
    /// Logger instance for writing runtime errors to log files.
    logger: Option<Logger>,
    /// Filesystem root directory from which files are served.
    root_dir: PathBuf,
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

/// Parsed command-line arguments for the server binary.
struct CliArgs {
    /// Optional path to `config.toml`.
    config_path: Option<PathBuf>,
    /// TCP port to listen on (default: 3000).
    port: u16,
    /// Directory to serve or single `.slt` file path.
    path: PathBuf,
}

fn print_usage() {
    eprintln!("Usage: salata-server [OPTIONS] <path>");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  <path>       Directory to serve or single .slt file");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --config <path>  Path to config.toml");
    eprintln!("  --port <N>       Port to listen on (default: 3000)");
    eprintln!("  -h, --help       Print help");
    eprintln!("  -V, --version    Print version");
}

fn parse_args() -> Result<CliArgs, String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_path: Option<PathBuf> = None;
    let mut port: u16 = 3000;
    let mut positional: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                eprintln!("salata-server v{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--config" => {
                i += 1;
                if i >= args.len() {
                    return Err("--config requires a value".into());
                }
                config_path = Some(PathBuf::from(&args[i]));
            }
            "--port" => {
                i += 1;
                if i >= args.len() {
                    return Err("--port requires a value".into());
                }
                port = args[i]
                    .parse()
                    .map_err(|_| format!("invalid port: {}", args[i]))?;
            }
            other => {
                if other.starts_with('-') {
                    return Err(format!("unknown option: {other}"));
                }
                positional = Some(args.remove(i));
                continue;
            }
        }
        i += 1;
    }

    let path = positional
        .map(PathBuf::from)
        .ok_or_else(|| "missing required argument: <path>".to_string())?;

    Ok(CliArgs {
        config_path,
        port,
        path,
    })
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

/// Main request handler. Routes incoming requests to either `.slt` processing
/// or static file serving after running all CGI security protections.
async fn handle_request(req: HttpRequest, body: Bytes, data: web::Data<AppState>) -> HttpResponse {
    let state = data.get_ref();
    let display_errors = state.config.salata.display_errors;
    let request_path = req.path().to_string();

    // 1. CGI protection checks.
    if let Err(resp) = run_protections(&req, &body, &state.protection) {
        return resp;
    }

    // 2. Register connection.
    let remote_ip = req
        .peer_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_default();
    let _conn_guard = match state.protection.register_connection(&remote_ip) {
        Ok(guard) => Some(guard),
        Err(e) => {
            return error_response(e.http_status(), &e.to_string(), display_errors);
        }
    };

    // 3. Resolve filesystem path.
    let fs_path = resolve_path(&state.root_dir, &request_path);

    // If path is a directory, try index files.
    let fs_path = if fs_path.is_dir() {
        let index_slt = fs_path.join("index.slt");
        let index_html = fs_path.join("index.html");
        if index_slt.exists() {
            index_slt
        } else if index_html.exists() {
            index_html
        } else {
            return error_page(404, "Not Found", state);
        }
    } else {
        fs_path
    };

    if !fs_path.exists() {
        return error_page(404, "Not Found", state);
    }

    // 4. Route: .slt → process, else → static file.
    if fs_path.extension().and_then(|e| e.to_str()) == Some("slt") {
        process_slt(&req, &fs_path, state)
    } else {
        match static_files::serve_static_file(&fs_path) {
            Ok(resp) => resp,
            Err(_) => error_page(500, "Internal Server Error", state),
        }
    }
}

/// Run CGI protection validations on the incoming request.
fn run_protections(
    req: &HttpRequest,
    body: &Bytes,
    protection: &CgiProtection,
) -> Result<(), HttpResponse> {
    let uri = req.uri().to_string();
    let query = req.query_string();
    let path = req.path();

    // Null byte checks.
    check_protection(protection.validate_null_bytes(&uri, "URL"))?;
    check_protection(protection.validate_null_bytes(query, "query string"))?;
    check_protection(protection.validate_null_bytes(path, "path"))?;

    // URL and query string length.
    check_protection(protection.validate_url(&uri))?;
    check_protection(protection.validate_query_string(query))?;

    // Path security.
    check_protection(protection.validate_path(path))?;

    // Header validation.
    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    check_protection(protection.validate_headers(&headers))?;

    // Body size.
    check_protection(protection.validate_body_size(body.len()))?;

    // Content-Length validation.
    let content_length: Option<usize> = req
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    check_protection(protection.validate_content_length(content_length, body.len()))?;

    Ok(())
}

fn check_protection(result: Result<(), ProtectionError>) -> Result<(), HttpResponse> {
    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(error_response(e.http_status(), &e.to_string(), true)),
    }
}

/// Resolve a URL path to a filesystem path under root_dir.
fn resolve_path(root: &Path, url_path: &str) -> PathBuf {
    let cleaned = url_path.trim_start_matches('/');
    root.join(cleaned)
}

// ---------------------------------------------------------------------------
// SLT processing
// ---------------------------------------------------------------------------

fn process_slt(req: &HttpRequest, file: &Path, state: &AppState) -> HttpResponse {
    let cgi_env = build_cgi_env(req);
    let default_ct = &state.config.salata.default_content_type;

    match salata_core::process_file(file, &state.config, &cgi_env, ExecutionContext::Server) {
        Ok(result) => {
            // Per spec: any runtime failure → 500 (overrides #status).
            let status_code = if result.had_runtime_errors {
                500
            } else {
                result.directives.status
            };

            // Validate response size.
            if let Err(e) = state.protection.validate_response_size(result.html.len()) {
                log_error(&state.logger, file, &e.to_string());
                return error_response(
                    e.http_status(),
                    &e.to_string(),
                    state.config.salata.display_errors,
                );
            }

            // Build response.
            let actix_status = actix_web::http::StatusCode::from_u16(status_code)
                .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
            let mut resp = HttpResponse::build(actix_status);

            // Content-Type.
            let ct = result
                .directives
                .content_type
                .as_deref()
                .unwrap_or(default_ct);
            resp.content_type(ct);

            // Redirect.
            if let Some(ref location) = result.directives.redirect {
                let redirect_status = if (300..400).contains(&status_code) {
                    actix_status
                } else {
                    actix_web::http::StatusCode::FOUND
                };
                return HttpResponse::build(redirect_status)
                    .insert_header(("Location", location.as_str()))
                    .content_type(ct)
                    .finish();
            }

            // Custom headers.
            for (name, value) in &result.directives.headers {
                resp.insert_header((name.as_str(), value.as_str()));
            }

            // Cookies.
            for cookie_str in &result.directives.cookies {
                if let Ok(cookie) = Cookie::parse_encoded(cookie_str.as_str()) {
                    resp.cookie(cookie);
                }
            }

            if result.had_runtime_errors {
                log_error(&state.logger, file, "one or more runtime errors occurred");
            }

            resp.body(result.html)
        }
        Err(e) => {
            log_error(&state.logger, file, &e.to_string());
            error_page(500, &e.to_string(), state)
        }
    }
}

/// Build a CgiEnv from the actix HttpRequest.
fn build_cgi_env(req: &HttpRequest) -> CgiEnv {
    let peer = req.peer_addr();
    CgiEnv {
        request_method: req.method().to_string(),
        query_string: req.query_string().to_string(),
        content_type: req
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string(),
        content_length: req
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string(),
        http_host: req
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string(),
        http_cookie: req
            .headers()
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string(),
        remote_addr: peer.map(|a| a.ip().to_string()).unwrap_or_default(),
        request_uri: req.uri().to_string(),
        path_info: req.path().to_string(),
        server_name: req
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.split(':').next())
            .unwrap_or("127.0.0.1")
            .to_string(),
        server_port: req
            .connection_info()
            .host()
            .rsplit(':')
            .next()
            .unwrap_or("80")
            .to_string(),
        http_authorization: req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string(),
        macro_data_dir: None,
    }
}

// ---------------------------------------------------------------------------
// Error responses
// ---------------------------------------------------------------------------

fn error_response(status: u16, message: &str, display_errors: bool) -> HttpResponse {
    let actix_status = actix_web::http::StatusCode::from_u16(status)
        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
    let body = if display_errors {
        format!(
            "<html><body><h1>{status} {}</h1><p>{}</p></body></html>",
            actix_status.canonical_reason().unwrap_or("Error"),
            html_escape(message)
        )
    } else {
        format!(
            "<html><body><h1>{status} {}</h1></body></html>",
            actix_status.canonical_reason().unwrap_or("Error")
        )
    };
    HttpResponse::build(actix_status)
        .content_type("text/html; charset=utf-8")
        .body(body)
}

/// Try to render a custom error page from config, falling back to plain HTML.
fn error_page(status: u16, message: &str, state: &AppState) -> HttpResponse {
    let error_page_path = match status {
        404 => state.config.errors.page_404.as_deref(),
        500 => state.config.errors.page_500.as_deref(),
        _ => None,
    };

    if let Some(page_path) = error_page_path {
        let path = PathBuf::from(page_path);
        if path.exists() {
            if path.extension().and_then(|e| e.to_str()) == Some("slt") {
                // Process .slt error page — but don't recurse on failure.
                let env = CgiEnv::default();
                if let Ok(result) =
                    salata_core::process_file(&path, &state.config, &env, ExecutionContext::Server)
                {
                    let actix_status = actix_web::http::StatusCode::from_u16(status)
                        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
                    return HttpResponse::build(actix_status)
                        .content_type("text/html; charset=utf-8")
                        .body(result.html);
                }
            } else if let Ok(contents) = std::fs::read_to_string(&path) {
                let actix_status = actix_web::http::StatusCode::from_u16(status)
                    .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
                return HttpResponse::build(actix_status)
                    .content_type("text/html; charset=utf-8")
                    .body(contents);
            }
        }
    }

    error_response(status, message, state.config.salata.display_errors)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn log_error(logger: &Option<Logger>, file: &Path, message: &str) {
    if let Some(ref logger) = logger {
        let _ = logger.log_runtime(
            LogLevel::Error,
            "salata-server",
            &file.display().to_string(),
            None,
            message,
        );
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

/// Entry point. Parses CLI arguments, loads configuration, initializes CGI
/// protections and the hot-reload file watcher, then starts the actix-web
/// HTTP server on the configured port.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("error: {e}");
            print_usage();
            std::process::exit(1);
        }
    };

    // 1. Load config.
    let config = match SalataConfig::locate(cli.config_path.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: configuration: {e}");
            std::process::exit(1);
        }
    };

    // 2. Set up logger (best-effort).
    let logger = Logger::new(&config.logging).ok();

    // 3. Create CGI protections.
    let protection = match CgiProtection::from_config(&config.cgi) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: protection config: {e}");
            std::process::exit(1);
        }
    };

    // 4. Determine root directory.
    let root_dir = if cli.path.is_dir() {
        cli.path.canonicalize().unwrap_or(cli.path.clone())
    } else {
        // Single file mode: serve from its parent directory.
        cli.path
            .parent()
            .unwrap_or(Path::new("."))
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("."))
    };

    // 5. Set up parse cache and hot reload watcher.
    let cache = Arc::new(ParseCache::new());
    let _watcher = if config.server.hot_reload {
        match hot_reload::start_watcher(&root_dir, Arc::clone(&cache)) {
            Ok(w) => {
                eprintln!(
                    "salata-server: hot reload enabled, watching {}",
                    root_dir.display()
                );
                Some(w)
            }
            Err(e) => {
                eprintln!("salata-server: warning: hot reload failed: {e}");
                None
            }
        }
    } else {
        None
    };

    let port = cli.port;
    eprintln!(
        "salata-server v{} listening on http://127.0.0.1:{port}",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("  serving: {}", root_dir.display());

    // 6. Start actix-web HTTP server.
    let state = web::Data::new(AppState {
        config,
        protection,
        cache,
        logger,
        root_dir,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .default_service(web::route().to(handle_request))
    })
    .bind(("127.0.0.1", port))?
    .shutdown_timeout(5)
    .run()
    .await
}
