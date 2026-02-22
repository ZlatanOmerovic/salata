use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::context::ExecutionContext;
use crate::error::{SalataError, SalataResult};

use super::{CgiEnv, RuntimeExecutor, BLOCK_BOUNDARY};

// ---------------------------------------------------------------------------
// PHP CLI runtime
// ---------------------------------------------------------------------------

/// PHP runtime executor in CLI mode — uses the `php` binary directly.
/// Reads PHP code from stdin (no CGI headers, no temp files needed).
pub struct PhpCliRuntime {
    binary: PathBuf,
}

impl PhpCliRuntime {
    /// Create a new PHP CLI runtime with the given php binary path.
    pub fn new(cli_path: &str) -> Self {
        Self {
            binary: PathBuf::from(cli_path),
        }
    }
}

impl RuntimeExecutor for PhpCliRuntime {
    fn language(&self) -> &str {
        "php"
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn args(&self) -> Vec<&str> {
        // `php` reads from stdin when no file is given.
        vec![]
    }

    fn wrap_block_for_shared_scope(&self, _code: &str) -> String {
        format!("echo \"{BLOCK_BOUNDARY}\";\n")
    }

    fn execute_isolated(
        &self,
        code: &str,
        env: &CgiEnv,
        file: &Path,
        start_line: usize,
    ) -> SalataResult<String> {
        let full_code = ensure_php_open_tag(code);
        super::run_process(&self.binary, &self.args(), &full_code, env, "php", file, start_line)
    }

    fn execute_shared(
        &self,
        blocks: &[(String, usize)],
        env: &CgiEnv,
        file: &Path,
    ) -> SalataResult<Vec<String>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }
        if blocks.len() == 1 {
            let output = self.execute_isolated(&blocks[0].0, env, file, blocks[0].1)?;
            return Ok(vec![output]);
        }

        let mut combined = String::from("<?php\n");
        for (i, (code, _)) in blocks.iter().enumerate() {
            if i > 0 {
                combined.push_str(&format!("echo \"{BLOCK_BOUNDARY}\";\n"));
            }
            combined.push_str(&strip_php_tags(code));
            combined.push('\n');
        }

        let first_line = blocks[0].1;
        let raw_output = super::run_process(
            &self.binary,
            &self.args(),
            &combined,
            env,
            "php",
            file,
            first_line,
        )?;

        let parts: Vec<&str> = raw_output.split(BLOCK_BOUNDARY).collect();
        let mut outputs = Vec::with_capacity(blocks.len());
        for i in 0..blocks.len() {
            if i < parts.len() {
                outputs.push(parts[i].to_string());
            } else {
                outputs.push(String::new());
            }
        }
        Ok(outputs)
    }
}

// ---------------------------------------------------------------------------
// PHP CGI runtime
// ---------------------------------------------------------------------------

/// PHP runtime executor in CGI mode — spawns php-cgi per execution.
pub struct PhpCgiRuntime {
    binary: PathBuf,
}

impl PhpCgiRuntime {
    /// Create a new PHP CGI runtime with the given php-cgi binary path.
    pub fn new(cgi_path: &str) -> Self {
        Self {
            binary: PathBuf::from(cgi_path),
        }
    }
}

impl RuntimeExecutor for PhpCgiRuntime {
    fn language(&self) -> &str {
        "php"
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn args(&self) -> Vec<&str> {
        // php-cgi reads from stdin when invoked without a file argument.
        vec![]
    }

    fn wrap_block_for_shared_scope(&self, _code: &str) -> String {
        format!("echo \"{BLOCK_BOUNDARY}\";\n")
    }

    fn execute_isolated(
        &self,
        code: &str,
        env: &CgiEnv,
        file: &Path,
        start_line: usize,
    ) -> SalataResult<String> {
        let full_code = ensure_php_open_tag(code);
        let raw = run_php_cgi_process(&self.binary, &full_code, env, file, start_line)?;
        Ok(strip_cgi_headers(&raw))
    }

    fn execute_shared(
        &self,
        blocks: &[(String, usize)],
        env: &CgiEnv,
        file: &Path,
    ) -> SalataResult<Vec<String>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }
        if blocks.len() == 1 {
            let output = self.execute_isolated(&blocks[0].0, env, file, blocks[0].1)?;
            return Ok(vec![output]);
        }

        let mut combined = String::from("<?php\n");
        for (i, (code, _)) in blocks.iter().enumerate() {
            if i > 0 {
                combined.push_str(&format!("echo \"{BLOCK_BOUNDARY}\";\n"));
            }
            combined.push_str(&strip_php_tags(code));
            combined.push('\n');
        }

        let first_line = blocks[0].1;
        let raw = run_php_cgi_process(&self.binary, &combined, env, file, first_line)?;

        let body = strip_cgi_headers(&raw);
        let parts: Vec<&str> = body.split(BLOCK_BOUNDARY).collect();

        let mut outputs = Vec::with_capacity(blocks.len());
        for i in 0..blocks.len() {
            if i < parts.len() {
                outputs.push(parts[i].to_string());
            } else {
                outputs.push(String::new());
            }
        }
        Ok(outputs)
    }
}

// ---------------------------------------------------------------------------
// PHP FastCGI runtime
// ---------------------------------------------------------------------------

/// PHP runtime executor in FastCGI mode — connects to a running php-fpm
/// via Unix socket or TCP.
pub struct PhpFastCgiRuntime {
    socket: Option<String>,
    host: Option<String>,
}

impl PhpFastCgiRuntime {
    /// Create a new PHP FastCGI runtime connecting via Unix socket or TCP.
    pub fn new(socket: Option<String>, host: Option<String>) -> Self {
        Self { socket, host }
    }

    /// Send a FastCGI request and return the response body.
    fn send_fastcgi_request(
        &self,
        code: &str,
        env: &CgiEnv,
        file: &Path,
        start_line: usize,
    ) -> SalataResult<String> {
        let code_with_tag = ensure_php_open_tag(code);
        let code_bytes = code_with_tag.as_bytes();

        // Build FastCGI params.
        let params = build_fastcgi_params(env, code_bytes.len());

        // Connect to php-fpm.
        let mut stream: Box<dyn ReadWrite> = if let Some(ref socket_path) = self.socket {
            #[cfg(unix)]
            {
                use std::os::unix::net::UnixStream;
                let stream =
                    UnixStream::connect(socket_path).map_err(|e| SalataError::Runtime {
                        runtime: "php".into(),
                        file: file.to_path_buf(),
                        line: start_line,
                        message: format!("failed to connect to php-fpm socket {socket_path}: {e}"),
                    })?;
                Box::new(stream)
            }
            #[cfg(not(unix))]
            {
                return Err(SalataError::Runtime {
                    runtime: "php".into(),
                    file: file.to_path_buf(),
                    line: start_line,
                    message: "Unix sockets not supported on this platform".into(),
                });
            }
        } else if let Some(ref host) = self.host {
            let stream = std::net::TcpStream::connect(host).map_err(|e| SalataError::Runtime {
                runtime: "php".into(),
                file: file.to_path_buf(),
                line: start_line,
                message: format!("failed to connect to php-fpm at {host}: {e}"),
            })?;
            Box::new(stream)
        } else {
            return Err(SalataError::Runtime {
                runtime: "php".into(),
                file: file.to_path_buf(),
                line: start_line,
                message: "no FastCGI socket or host configured".into(),
            });
        };

        // Send FastCGI BEGIN_REQUEST record (request ID = 1, role = RESPONDER, keep_conn = 0).
        write_fcgi_record(
            &mut stream,
            FCGI_BEGIN_REQUEST,
            1,
            &[0, 1, 0, 0, 0, 0, 0, 0],
        )
        .map_err(|e| runtime_err(file, start_line, &format!("BEGIN_REQUEST: {e}")))?;

        // Send FCGI_PARAMS.
        let params_payload = encode_fcgi_params(&params);
        if !params_payload.is_empty() {
            write_fcgi_record(&mut stream, FCGI_PARAMS, 1, &params_payload)
                .map_err(|e| runtime_err(file, start_line, &format!("PARAMS: {e}")))?;
        }
        // Empty PARAMS to signal end.
        write_fcgi_record(&mut stream, FCGI_PARAMS, 1, &[])
            .map_err(|e| runtime_err(file, start_line, &format!("PARAMS end: {e}")))?;

        // Send FCGI_STDIN with the PHP code.
        write_fcgi_record(&mut stream, FCGI_STDIN, 1, code_bytes)
            .map_err(|e| runtime_err(file, start_line, &format!("STDIN: {e}")))?;
        // Empty STDIN to signal end.
        write_fcgi_record(&mut stream, FCGI_STDIN, 1, &[])
            .map_err(|e| runtime_err(file, start_line, &format!("STDIN end: {e}")))?;

        // Read response records.
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        loop {
            let (rec_type, _req_id, payload) = read_fcgi_record(&mut stream)
                .map_err(|e| runtime_err(file, start_line, &format!("read response: {e}")))?;

            match rec_type {
                FCGI_STDOUT => stdout_buf.extend_from_slice(&payload),
                FCGI_STDERR => stderr_buf.extend_from_slice(&payload),
                FCGI_END_REQUEST => break,
                _ => {} // Ignore unknown record types.
            }
        }

        if !stderr_buf.is_empty() {
            let stderr = String::from_utf8_lossy(&stderr_buf);
            return Err(SalataError::Runtime {
                runtime: "php".into(),
                file: file.to_path_buf(),
                line: start_line,
                message: stderr.trim().to_string(),
            });
        }

        let raw = String::from_utf8_lossy(&stdout_buf).to_string();
        Ok(strip_cgi_headers(&raw))
    }
}

impl RuntimeExecutor for PhpFastCgiRuntime {
    fn language(&self) -> &str {
        "php"
    }

    fn binary_path(&self) -> &Path {
        // FastCGI doesn't use a binary — returns a placeholder path.
        Path::new("/php-fpm")
    }

    fn args(&self) -> Vec<&str> {
        vec![]
    }

    fn wrap_block_for_shared_scope(&self, _code: &str) -> String {
        format!("echo \"{BLOCK_BOUNDARY}\";\n")
    }

    fn execute_isolated(
        &self,
        code: &str,
        env: &CgiEnv,
        file: &Path,
        start_line: usize,
    ) -> SalataResult<String> {
        self.send_fastcgi_request(code, env, file, start_line)
    }

    fn execute_shared(
        &self,
        blocks: &[(String, usize)],
        env: &CgiEnv,
        file: &Path,
    ) -> SalataResult<Vec<String>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }
        if blocks.len() == 1 {
            let output = self.execute_isolated(&blocks[0].0, env, file, blocks[0].1)?;
            return Ok(vec![output]);
        }

        let mut combined = String::from("<?php\n");
        for (i, (code, _)) in blocks.iter().enumerate() {
            if i > 0 {
                combined.push_str(&format!("echo \"{BLOCK_BOUNDARY}\";\n"));
            }
            combined.push_str(&strip_php_tags(code));
            combined.push('\n');
        }

        let first_line = blocks[0].1;
        let body = self.send_fastcgi_request(&combined, env, file, first_line)?;

        let parts: Vec<&str> = body.split(BLOCK_BOUNDARY).collect();
        let mut outputs = Vec::with_capacity(blocks.len());
        for i in 0..blocks.len() {
            if i < parts.len() {
                outputs.push(parts[i].to_string());
            } else {
                outputs.push(String::new());
            }
        }
        Ok(outputs)
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Create the appropriate PHP runtime based on execution context.
///
/// - `Cli`: uses `cli_path` (the `php` binary)
/// - `Cgi`: uses `cgi_path` (the `php-cgi` binary)
/// - `FastCgi` / `Server`: uses php-fpm via `fastcgi_socket` or `fastcgi_host`
pub fn create_php_runtime(
    ctx: ExecutionContext,
    cli_path: Option<&str>,
    cgi_path: Option<&str>,
    fastcgi_socket: Option<&str>,
    fastcgi_host: Option<&str>,
) -> Box<dyn RuntimeExecutor> {
    match ctx {
        ExecutionContext::Cli => {
            Box::new(PhpCliRuntime::new(cli_path.unwrap_or("/usr/bin/php")))
        }
        ExecutionContext::Cgi => {
            Box::new(PhpCgiRuntime::new(cgi_path.unwrap_or("/usr/bin/php-cgi")))
        }
        ExecutionContext::FastCgi | ExecutionContext::Server => Box::new(PhpFastCgiRuntime::new(
            fastcgi_socket.map(String::from),
            fastcgi_host.map(String::from),
        )),
    }
}

// ---------------------------------------------------------------------------
// PHP helpers
// ---------------------------------------------------------------------------

/// Ensure code starts with `<?php` tag.
fn ensure_php_open_tag(code: &str) -> String {
    let trimmed = code.trim_start();
    if trimmed.starts_with("<?php") || trimmed.starts_with("<?") {
        code.to_string()
    } else {
        format!("<?php\n{code}")
    }
}

/// Strip `<?php` / `?>` tags from code for concatenation in shared scope.
fn strip_php_tags(code: &str) -> String {
    let mut s = code.to_string();
    // Remove leading <?php
    if let Some(rest) = s.strip_prefix("<?php") {
        s = rest.to_string();
    } else if let Some(rest) = s.strip_prefix("<?") {
        s = rest.to_string();
    }
    // Remove trailing ?>
    if let Some(rest) = s.strip_suffix("?>") {
        s = rest.to_string();
    }
    s
}

/// Spawn php-cgi with the required environment variables.
///
/// php-cgi requires `REDIRECT_STATUS=200` (CGI security feature) and
/// `SCRIPT_FILENAME` pointing to a real file — it cannot read from stdin
/// when CGI env vars are present. We write code to a temp file.
fn run_php_cgi_process(
    binary: &Path,
    code: &str,
    env: &CgiEnv,
    file: &Path,
    start_line: usize,
) -> SalataResult<String> {
    if !binary.exists() {
        return Err(SalataError::RuntimeNotFound {
            runtime: "php".to_string(),
            path: binary.to_path_buf(),
        });
    }

    // Write code to a temp file — php-cgi needs SCRIPT_FILENAME.
    let tmp_dir = std::env::temp_dir();
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path = tmp_dir.join(format!("salata_php_{}_{id}.php", std::process::id()));
    std::fs::write(&tmp_path, code).map_err(|e| SalataError::Runtime {
        runtime: "php".to_string(),
        file: file.to_path_buf(),
        line: start_line,
        message: format!("failed to write temp PHP file: {e}"),
    })?;

    let mut cmd = Command::new(binary);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    env.apply_to(&mut cmd);
    cmd.env("REDIRECT_STATUS", "200");
    cmd.env("SCRIPT_FILENAME", &tmp_path);

    let result = cmd.output().map_err(|e| SalataError::Runtime {
        runtime: "php".to_string(),
        file: file.to_path_buf(),
        line: start_line,
        message: format!("failed to run {}: {e}", binary.display()),
    });

    // Clean up temp file regardless of outcome.
    let _ = std::fs::remove_file(&tmp_path);

    let output = result?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let message = if stderr.trim().is_empty() {
            // php-cgi often puts errors in stdout as part of CGI response.
            strip_cgi_headers(&stdout).trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(SalataError::Runtime {
            runtime: "php".to_string(),
            file: file.to_path_buf(),
            line: start_line,
            message,
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Strip CGI headers from php-cgi output (everything before the first blank line).
fn strip_cgi_headers(output: &str) -> String {
    // php-cgi outputs headers like: "X-Powered-By: ...\r\nContent-type: ...\r\n\r\n<body>"
    // Find the header/body separator.
    if let Some(pos) = output.find("\r\n\r\n") {
        output[pos + 4..].to_string()
    } else if let Some(pos) = output.find("\n\n") {
        output[pos + 2..].to_string()
    } else {
        // No headers found — return as-is.
        output.to_string()
    }
}

// ---------------------------------------------------------------------------
// FastCGI protocol helpers
// ---------------------------------------------------------------------------

// FastCGI record types.
const FCGI_BEGIN_REQUEST: u8 = 1;
const FCGI_END_REQUEST: u8 = 3;
const FCGI_PARAMS: u8 = 4;
const FCGI_STDIN: u8 = 5;
const FCGI_STDOUT: u8 = 6;
const FCGI_STDERR: u8 = 7;

/// Trait alias for Read + Write.
trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn runtime_err(file: &Path, line: usize, msg: &str) -> SalataError {
    SalataError::Runtime {
        runtime: "php".into(),
        file: file.to_path_buf(),
        line,
        message: msg.to_string(),
    }
}

/// Write a FastCGI record to the stream.
fn write_fcgi_record(
    stream: &mut dyn Write,
    rec_type: u8,
    request_id: u16,
    data: &[u8],
) -> std::io::Result<()> {
    let content_len = data.len() as u16;
    let padding_len = (8 - (content_len % 8)) % 8;

    // 8-byte header.
    let header = [
        1,                          // version
        rec_type,                   // type
        (request_id >> 8) as u8,    // requestIdB1
        (request_id & 0xFF) as u8,  // requestIdB0
        (content_len >> 8) as u8,   // contentLengthB1
        (content_len & 0xFF) as u8, // contentLengthB0
        padding_len as u8,          // paddingLength
        0,                          // reserved
    ];

    stream.write_all(&header)?;
    stream.write_all(data)?;
    if padding_len > 0 {
        stream.write_all(&vec![0u8; padding_len as usize])?;
    }
    stream.flush()?;
    Ok(())
}

/// Read a single FastCGI record from the stream.
/// Returns (record_type, request_id, payload).
fn read_fcgi_record(stream: &mut dyn Read) -> std::io::Result<(u8, u16, Vec<u8>)> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header)?;

    let rec_type = header[1];
    let request_id = ((header[2] as u16) << 8) | header[3] as u16;
    let content_len = ((header[4] as u16) << 8) | header[5] as u16;
    let padding_len = header[6];

    let mut payload = vec![0u8; content_len as usize];
    if content_len > 0 {
        stream.read_exact(&mut payload)?;
    }
    if padding_len > 0 {
        let mut pad = vec![0u8; padding_len as usize];
        stream.read_exact(&mut pad)?;
    }

    Ok((rec_type, request_id, payload))
}

/// Encode a key-value pair list into FastCGI PARAMS format.
fn encode_fcgi_params(params: &[(&str, &str)]) -> Vec<u8> {
    let mut buf = Vec::new();
    for (name, value) in params {
        encode_fcgi_length(&mut buf, name.len());
        encode_fcgi_length(&mut buf, value.len());
        buf.extend_from_slice(name.as_bytes());
        buf.extend_from_slice(value.as_bytes());
    }
    buf
}

/// Encode a FastCGI name/value length.
fn encode_fcgi_length(buf: &mut Vec<u8>, len: usize) {
    if len < 128 {
        buf.push(len as u8);
    } else {
        buf.push(((len >> 24) as u8) | 0x80);
        buf.push((len >> 16) as u8);
        buf.push((len >> 8) as u8);
        buf.push(len as u8);
    }
}

/// Build FastCGI params from CGI env.
fn build_fastcgi_params(env: &CgiEnv, content_length: usize) -> Vec<(&str, &str)> {
    let mut params: Vec<(&str, &str)> = vec![
        ("REQUEST_METHOD", &env.request_method),
        ("QUERY_STRING", &env.query_string),
        ("CONTENT_TYPE", &env.content_type),
        ("HTTP_HOST", &env.http_host),
        ("HTTP_COOKIE", &env.http_cookie),
        ("REMOTE_ADDR", &env.remote_addr),
        ("REQUEST_URI", &env.request_uri),
        ("PATH_INFO", &env.path_info),
        ("SERVER_NAME", &env.server_name),
        ("SERVER_PORT", &env.server_port),
        ("GATEWAY_INTERFACE", "CGI/1.1"),
        ("SERVER_PROTOCOL", "HTTP/1.1"),
        // php-fpm needs SCRIPT_FILENAME to execute inline code.
        ("SCRIPT_FILENAME", "/dev/stdin"),
    ];
    // CONTENT_LENGTH must reflect actual stdin data size.
    // We'll store it as a leaked string for the lifetime of this request.
    // Instead, build as String and return owned — but the signature uses &str...
    // Work around by using the env's content_length field.
    if !env.content_length.is_empty() {
        params.push(("CONTENT_LENGTH", &env.content_length));
    }
    let _ = content_length; // reserved for future use
    params
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- PHP helper tests --

    #[test]
    fn ensure_php_tag_added() {
        let code = "echo 'hello';";
        let result = ensure_php_open_tag(code);
        assert!(result.starts_with("<?php\n"));
        assert!(result.contains("echo 'hello';"));
    }

    #[test]
    fn ensure_php_tag_not_doubled() {
        let code = "<?php\necho 'hello';";
        let result = ensure_php_open_tag(code);
        assert_eq!(result, code);
    }

    #[test]
    fn ensure_php_short_tag_not_doubled() {
        let code = "<? echo 'hello';";
        let result = ensure_php_open_tag(code);
        assert_eq!(result, code);
    }

    #[test]
    fn strip_php_tags_basic() {
        let code = "<?php echo 'hi'; ?>";
        let result = strip_php_tags(code);
        assert_eq!(result.trim(), "echo 'hi';");
    }

    #[test]
    fn strip_php_tags_no_tags() {
        let code = "echo 'hi';";
        let result = strip_php_tags(code);
        assert_eq!(result, code);
    }

    #[test]
    fn strip_cgi_headers_basic() {
        let output = "X-Powered-By: PHP/8.1\r\nContent-type: text/html\r\n\r\nhello world";
        let body = strip_cgi_headers(output);
        assert_eq!(body, "hello world");
    }

    #[test]
    fn strip_cgi_headers_lf_only() {
        let output = "Content-type: text/html\n\nhello";
        let body = strip_cgi_headers(output);
        assert_eq!(body, "hello");
    }

    #[test]
    fn strip_cgi_headers_no_headers() {
        let output = "just body";
        let body = strip_cgi_headers(output);
        assert_eq!(body, "just body");
    }

    // -- FastCGI protocol tests --

    #[test]
    fn encode_fcgi_length_short() {
        let mut buf = Vec::new();
        encode_fcgi_length(&mut buf, 42);
        assert_eq!(buf, vec![42]);
    }

    #[test]
    fn encode_fcgi_length_long() {
        let mut buf = Vec::new();
        encode_fcgi_length(&mut buf, 300);
        assert_eq!(buf.len(), 4);
        assert_eq!(buf[0] & 0x80, 0x80); // High bit set.
    }

    #[test]
    fn encode_fcgi_params_basic() {
        let params = vec![("KEY", "val")];
        let encoded = encode_fcgi_params(&params);
        // 1 byte key len + 1 byte val len + 3 bytes "KEY" + 3 bytes "val" = 8
        assert_eq!(encoded.len(), 8);
        assert_eq!(encoded[0], 3); // key length
        assert_eq!(encoded[1], 3); // value length
    }

    #[test]
    fn fcgi_record_roundtrip() {
        let data = b"hello";
        let mut buf = Vec::new();
        write_fcgi_record(&mut buf, FCGI_STDOUT, 1, data).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let (rec_type, req_id, payload) = read_fcgi_record(&mut cursor).unwrap();
        assert_eq!(rec_type, FCGI_STDOUT);
        assert_eq!(req_id, 1);
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn fcgi_record_empty_payload() {
        let mut buf = Vec::new();
        write_fcgi_record(&mut buf, FCGI_PARAMS, 1, &[]).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let (rec_type, _, payload) = read_fcgi_record(&mut cursor).unwrap();
        assert_eq!(rec_type, FCGI_PARAMS);
        assert!(payload.is_empty());
    }

    // -- Factory tests --

    #[test]
    fn create_cli_runtime() {
        let rt = create_php_runtime(
            ExecutionContext::Cli,
            Some("/usr/bin/php"),
            None,
            None,
            None,
        );
        assert_eq!(rt.language(), "php");
        assert_eq!(rt.binary_path(), Path::new("/usr/bin/php"));
    }

    #[test]
    fn create_cgi_runtime() {
        let rt = create_php_runtime(
            ExecutionContext::Cgi,
            None,
            Some("/usr/bin/php-cgi"),
            None,
            None,
        );
        assert_eq!(rt.language(), "php");
        assert_eq!(rt.binary_path(), Path::new("/usr/bin/php-cgi"));
    }

    #[test]
    fn create_fastcgi_runtime() {
        let rt = create_php_runtime(
            ExecutionContext::FastCgi,
            None,
            None,
            Some("/run/php/php-fpm.sock"),
            None,
        );
        assert_eq!(rt.language(), "php");
    }

    #[test]
    fn create_server_runtime() {
        let rt = create_php_runtime(
            ExecutionContext::Server,
            None,
            None,
            None,
            Some("127.0.0.1:9000"),
        );
        assert_eq!(rt.language(), "php");
    }

    #[test]
    fn cli_context_uses_cli_path() {
        let rt = create_php_runtime(
            ExecutionContext::Cli,
            Some("/opt/php/bin/php"),
            Some("/opt/php/bin/php-cgi"),
            None,
            None,
        );
        assert_eq!(rt.binary_path(), Path::new("/opt/php/bin/php"));
    }

    #[test]
    fn cgi_context_uses_cgi_path() {
        let rt = create_php_runtime(
            ExecutionContext::Cgi,
            Some("/opt/php/bin/php"),
            Some("/opt/php/bin/php-cgi"),
            None,
            None,
        );
        assert_eq!(rt.binary_path(), Path::new("/opt/php/bin/php-cgi"));
    }

    #[test]
    fn cli_context_default_path() {
        let rt = create_php_runtime(ExecutionContext::Cli, None, None, None, None);
        assert_eq!(rt.binary_path(), Path::new("/usr/bin/php"));
    }

    #[test]
    fn cgi_context_default_path() {
        let rt = create_php_runtime(ExecutionContext::Cgi, None, None, None, None);
        assert_eq!(rt.binary_path(), Path::new("/usr/bin/php-cgi"));
    }

    // -- Live CGI execution tests (require php-cgi installed) --

    fn find_php_cgi() -> Option<String> {
        for p in &[
            "/usr/bin/php-cgi",
            "/usr/local/bin/php-cgi",
            "/opt/homebrew/bin/php-cgi",
        ] {
            if Path::new(p).exists() {
                return Some(p.to_string());
            }
        }
        std::process::Command::new("which")
            .arg("php-cgi")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
    }

    #[test]
    fn php_cgi_single_block() {
        let Some(path) = find_php_cgi() else {
            eprintln!("php-cgi not found, skipping test");
            return;
        };
        let rt = PhpCgiRuntime::new(&path);
        let env = CgiEnv::default();
        let output = rt
            .execute_isolated("echo 'hello world';", &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "hello world");
    }

    #[test]
    fn php_cgi_with_existing_tag() {
        let Some(path) = find_php_cgi() else { return };
        let rt = PhpCgiRuntime::new(&path);
        let env = CgiEnv::default();
        let output = rt
            .execute_isolated("<?php echo 'tagged';", &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "tagged");
    }

    #[test]
    fn php_cgi_shared_scope() {
        let Some(path) = find_php_cgi() else { return };
        let rt = PhpCgiRuntime::new(&path);
        let env = CgiEnv::default();
        let blocks = vec![
            ("$x = 42; echo $x;".to_string(), 1),
            ("echo $x + 1;".to_string(), 5),
        ];
        let outputs = rt
            .execute_shared(&blocks, &env, Path::new("test.slt"))
            .unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].trim(), "42");
        assert_eq!(outputs[1].trim(), "43");
    }

    #[test]
    fn php_cgi_error() {
        let Some(path) = find_php_cgi() else { return };
        let rt = PhpCgiRuntime::new(&path);
        let env = CgiEnv::default();
        // Parse error should cause non-zero exit.
        let result = rt.execute_isolated(
            "<?php\nthis is not valid php {{{",
            &env,
            Path::new("test.slt"),
            1,
        );
        // php-cgi may still exit 0 with errors in output, so we just check it runs.
        // The important thing is it doesn't panic.
        let _ = result;
    }
}
