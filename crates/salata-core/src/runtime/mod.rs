//! Runtime execution engine — spawns language interpreters and captures output.
//!
//! Each supported language has its own module implementing [`RuntimeExecutor`].
//! The engine supports two scope modes:
//!
//! - **Shared scope** (default): all blocks of the same language run in one process,
//!   separated by boundary markers (`__SALATA_BLOCK_BOUNDARY__`).
//! - **Isolated scope**: each block gets its own process (`scope="isolated"` attribute
//!   or `shared_scope = false` in config).
//!
//! Cross-runtime communication uses the `#set`/`#get` macro system — runtimes
//! never communicate directly with each other.

pub mod javascript;
pub mod php;
pub mod python;
pub mod ruby;
pub mod shell;
pub mod typescript;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::SalataConfig;
use crate::error::{SalataError, SalataResult};
use crate::parser::{BlockScope, Segment};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Boundary marker placed between concatenated blocks in shared scope mode.
pub const BLOCK_BOUNDARY: &str = "__SALATA_BLOCK_BOUNDARY__";

// ---------------------------------------------------------------------------
// CGI environment
// ---------------------------------------------------------------------------

/// Standard CGI environment variables passed to runtime processes.
/// Values come from the HTTP request context (or empty strings for CLI mode).
#[derive(Debug, Clone, Default)]
pub struct CgiEnv {
    pub request_method: String,
    pub query_string: String,
    pub content_type: String,
    pub content_length: String,
    pub http_host: String,
    pub http_cookie: String,
    pub remote_addr: String,
    pub request_uri: String,
    pub path_info: String,
    pub server_name: String,
    pub server_port: String,
    pub http_authorization: String,
    /// Path to the macro data directory (set when macros are in use).
    pub macro_data_dir: Option<String>,
}

impl CgiEnv {
    /// Apply CGI env vars to a Command.
    pub fn apply_to(&self, cmd: &mut Command) {
        cmd.env("REQUEST_METHOD", &self.request_method)
            .env("QUERY_STRING", &self.query_string)
            .env("CONTENT_TYPE", &self.content_type)
            .env("CONTENT_LENGTH", &self.content_length)
            .env("HTTP_HOST", &self.http_host)
            .env("HTTP_COOKIE", &self.http_cookie)
            .env("REMOTE_ADDR", &self.remote_addr)
            .env("REQUEST_URI", &self.request_uri)
            .env("PATH_INFO", &self.path_info)
            .env("SERVER_NAME", &self.server_name)
            .env("SERVER_PORT", &self.server_port)
            .env("HTTP_AUTHORIZATION", &self.http_authorization);

        if let Some(ref dir) = self.macro_data_dir {
            cmd.env(crate::macros::MACRO_DIR_ENV, dir);
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime trait
// ---------------------------------------------------------------------------

/// Common interface for all language runtimes.
pub trait RuntimeExecutor {
    /// The language name (e.g. "python", "ruby", "shell").
    fn language(&self) -> &str;

    /// The path to the runtime binary.
    fn binary_path(&self) -> &Path;

    /// Build command-line arguments for the runtime.
    /// Most runtimes use `-` (read from stdin); override if different.
    fn args(&self) -> Vec<&str> {
        vec!["-"]
    }

    /// Optional preamble code injected before user code.
    /// Used by JS/TS for print()/println() helpers.
    fn preamble(&self) -> Option<String> {
        None
    }

    /// Wrap user code with boundary print statements for shared scope.
    fn wrap_block_for_shared_scope(&self, code: &str) -> String;

    /// Execute a single code block in isolated scope.
    fn execute_isolated(
        &self,
        code: &str,
        env: &CgiEnv,
        file: &Path,
        start_line: usize,
    ) -> SalataResult<String> {
        let full_code = match self.preamble() {
            Some(pre) => format!("{pre}\n{code}"),
            None => code.to_string(),
        };
        run_process(
            self.binary_path(),
            &self.args(),
            &full_code,
            env,
            self.language(),
            file,
            start_line,
        )
    }

    /// Execute multiple blocks in shared scope (one process).
    /// Returns per-block outputs in the same order as the input blocks.
    fn execute_shared(
        &self,
        blocks: &[(String, usize)], // (code, start_line)
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

        // Concatenate blocks with boundary markers.
        let mut combined = String::new();
        if let Some(pre) = self.preamble() {
            combined.push_str(&pre);
            combined.push('\n');
        }
        for (i, (code, _)) in blocks.iter().enumerate() {
            if i > 0 {
                combined.push_str(&self.wrap_block_for_shared_scope(""));
            }
            combined.push_str(code);
            combined.push('\n');
        }

        let first_line = blocks[0].1;
        let raw_output = run_process(
            self.binary_path(),
            &self.args(),
            &combined,
            env,
            self.language(),
            file,
            first_line,
        )?;

        // Split output by boundary markers.
        let parts: Vec<&str> = raw_output.split(BLOCK_BOUNDARY).collect();

        // We expect exactly `blocks.len()` parts.
        // If there are fewer, pad with empty strings.
        // If there are more, join excess into the last block.
        let mut outputs: Vec<String> = Vec::with_capacity(blocks.len());
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
// Process execution
// ---------------------------------------------------------------------------

/// Spawn a runtime process, send code via stdin, capture stdout.
pub(crate) fn run_process(
    binary: &Path,
    args: &[&str],
    code: &str,
    env: &CgiEnv,
    language: &str,
    file: &Path,
    start_line: usize,
) -> SalataResult<String> {
    if !binary.exists() {
        return Err(SalataError::RuntimeNotFound {
            runtime: language.to_string(),
            path: binary.to_path_buf(),
        });
    }

    let mut cmd = Command::new(binary);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    env.apply_to(&mut cmd);

    let mut child = cmd.spawn().map_err(|e| SalataError::Runtime {
        runtime: language.to_string(),
        file: file.to_path_buf(),
        line: start_line,
        message: format!("failed to spawn {}: {e}", binary.display()),
    })?;

    // Write code to stdin.
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(code.as_bytes())
            .map_err(|e| SalataError::Runtime {
                runtime: language.to_string(),
                file: file.to_path_buf(),
                line: start_line,
                message: format!("failed to write to stdin: {e}"),
            })?;
    }

    let output = child.wait_with_output().map_err(|e| SalataError::Runtime {
        runtime: language.to_string(),
        file: file.to_path_buf(),
        line: start_line,
        message: format!("failed to read output: {e}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SalataError::Runtime {
            runtime: language.to_string(),
            file: file.to_path_buf(),
            line: start_line,
            message: stderr.trim().to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ---------------------------------------------------------------------------
// Error display helpers
// ---------------------------------------------------------------------------

/// Escape HTML special characters in error messages.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Format a runtime error as visible HTML for display_errors mode.
pub fn format_error_html(error: &SalataError) -> String {
    let msg = html_escape(&error.to_string());
    format!(
        "<pre style=\"color:red;background:#fff0f0;padding:8px;border:1px solid red\">{msg}</pre>"
    )
}

// ---------------------------------------------------------------------------
// Execution engine
// ---------------------------------------------------------------------------

/// Execute all runtime blocks in a parsed document, returning per-block outputs
/// keyed by their index in the segments list, plus a flag indicating whether
/// any runtime errors occurred.
///
/// `shared_scope_config` maps language name → whether shared scope is enabled
/// in config. Per-block `scope="isolated"` overrides this.
///
/// When `sequential` is true, blocks execute one-by-one in document order
/// (needed when macros bridge data between runtimes). When false, blocks are
/// grouped by language for shared-scope batch execution.
///
/// When `config` is `Some`, runtime errors are caught per-block: if
/// `display_errors` is enabled for the runtime, the error is formatted as HTML;
/// otherwise the block produces empty output. Errors always set `had_errors`.
/// When `config` is `None`, errors propagate immediately (fail-fast).
pub fn execute_blocks(
    segments: &[Segment],
    executors: &HashMap<String, Box<dyn RuntimeExecutor>>,
    shared_scope_config: &HashMap<String, bool>,
    env: &CgiEnv,
    file: &Path,
    sequential: bool,
    config: Option<&SalataConfig>,
) -> SalataResult<(HashMap<usize, String>, bool)> {
    if sequential {
        return execute_blocks_sequential(segments, executors, env, file, config);
    }

    execute_blocks_grouped(segments, executors, shared_scope_config, env, file, config)
}

/// Execute blocks one-by-one in document order (for macro cross-runtime data).
fn execute_blocks_sequential(
    segments: &[Segment],
    executors: &HashMap<String, Box<dyn RuntimeExecutor>>,
    env: &CgiEnv,
    file: &Path,
    config: Option<&SalataConfig>,
) -> SalataResult<(HashMap<usize, String>, bool)> {
    let mut outputs: HashMap<usize, String> = HashMap::new();
    let mut had_errors = false;

    for (i, seg) in segments.iter().enumerate() {
        if let Segment::RuntimeBlock(block) = seg {
            let executor = executors.get(block.language.as_str()).ok_or_else(|| {
                SalataError::RuntimeNotFound {
                    runtime: block.language.clone(),
                    path: PathBuf::from("(not configured)"),
                }
            })?;

            match executor.execute_isolated(&block.code, env, file, block.start_line) {
                Ok(output) => {
                    outputs.insert(i, output);
                }
                Err(e) => {
                    had_errors = true;
                    match config {
                        Some(cfg) if cfg.display_errors_for(&block.language) => {
                            outputs.insert(i, format_error_html(&e));
                        }
                        Some(_) => {
                            outputs.insert(i, String::new());
                        }
                        None => return Err(e),
                    }
                }
            }
        }
    }

    Ok((outputs, had_errors))
}

/// Execute blocks grouped by language (optimized — shared scope batches).
fn execute_blocks_grouped(
    segments: &[Segment],
    executors: &HashMap<String, Box<dyn RuntimeExecutor>>,
    shared_scope_config: &HashMap<String, bool>,
    env: &CgiEnv,
    file: &Path,
    config: Option<&SalataConfig>,
) -> SalataResult<(HashMap<usize, String>, bool)> {
    struct BlockInfo {
        seg_index: usize,
        code: String,
        start_line: usize,
        isolated: bool,
    }

    let mut blocks_by_lang: HashMap<String, Vec<BlockInfo>> = HashMap::new();

    for (i, seg) in segments.iter().enumerate() {
        if let Segment::RuntimeBlock(block) = seg {
            let isolated = block.scope == Some(BlockScope::Isolated)
                || !shared_scope_config
                    .get(&block.language)
                    .copied()
                    .unwrap_or(true);

            blocks_by_lang
                .entry(block.language.clone())
                .or_default()
                .push(BlockInfo {
                    seg_index: i,
                    code: block.code.clone(),
                    start_line: block.start_line,
                    isolated,
                });
        }
    }

    let mut outputs: HashMap<usize, String> = HashMap::new();
    let mut had_errors = false;

    for (lang, blocks) in &blocks_by_lang {
        let executor =
            executors
                .get(lang.as_str())
                .ok_or_else(|| SalataError::RuntimeNotFound {
                    runtime: lang.clone(),
                    path: PathBuf::from("(not configured)"),
                })?;

        let mut shared_blocks: Vec<(usize, String, usize)> = Vec::new();
        let mut isolated_blocks: Vec<(usize, String, usize)> = Vec::new();

        for b in blocks {
            if b.isolated {
                isolated_blocks.push((b.seg_index, b.code.clone(), b.start_line));
            } else {
                shared_blocks.push((b.seg_index, b.code.clone(), b.start_line));
            }
        }

        for (seg_idx, code, start_line) in &isolated_blocks {
            match executor.execute_isolated(code, env, file, *start_line) {
                Ok(output) => {
                    outputs.insert(*seg_idx, output);
                }
                Err(e) => {
                    had_errors = true;
                    match config {
                        Some(cfg) if cfg.display_errors_for(lang) => {
                            outputs.insert(*seg_idx, format_error_html(&e));
                        }
                        Some(_) => {
                            outputs.insert(*seg_idx, String::new());
                        }
                        None => return Err(e),
                    }
                }
            }
        }

        if !shared_blocks.is_empty() {
            let code_and_lines: Vec<(String, usize)> = shared_blocks
                .iter()
                .map(|(_, code, line)| (code.clone(), *line))
                .collect();

            match executor.execute_shared(&code_and_lines, env, file) {
                Ok(shared_outputs) => {
                    for (i, (seg_idx, _, _)) in shared_blocks.iter().enumerate() {
                        if let Some(output) = shared_outputs.get(i) {
                            outputs.insert(*seg_idx, output.clone());
                        }
                    }
                }
                Err(e) => {
                    had_errors = true;
                    match config {
                        Some(cfg) if cfg.display_errors_for(lang) => {
                            // Show error on first block, empty on rest.
                            for (i, (seg_idx, _, _)) in shared_blocks.iter().enumerate() {
                                if i == 0 {
                                    outputs.insert(*seg_idx, format_error_html(&e));
                                } else {
                                    outputs.insert(*seg_idx, String::new());
                                }
                            }
                        }
                        Some(_) => {
                            for (seg_idx, _, _) in &shared_blocks {
                                outputs.insert(*seg_idx, String::new());
                            }
                        }
                        None => return Err(e),
                    }
                }
            }
        }
    }

    Ok((outputs, had_errors))
}

/// Splice block outputs back into the document, producing the final HTML.
pub fn splice_outputs(segments: &[Segment], outputs: &HashMap<usize, String>) -> String {
    let mut result = String::new();
    for (i, seg) in segments.iter().enumerate() {
        match seg {
            Segment::Html(html) => result.push_str(html),
            Segment::RuntimeBlock(_) => {
                if let Some(output) = outputs.get(&i) {
                    result.push_str(output);
                }
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::RuntimeBlock;

    #[test]
    fn cgi_env_default() {
        let env = CgiEnv::default();
        assert_eq!(env.request_method, "");
        assert_eq!(env.query_string, "");
    }

    #[test]
    fn cgi_env_apply() {
        let env = CgiEnv {
            request_method: "GET".into(),
            http_host: "localhost".into(),
            ..Default::default()
        };
        let mut cmd = Command::new("echo");
        env.apply_to(&mut cmd);
        // Can't easily inspect env vars on Command, but this shouldn't panic.
    }

    #[test]
    fn splice_outputs_basic() {
        let segments = vec![
            Segment::Html("<h1>".into()),
            Segment::RuntimeBlock(RuntimeBlock {
                language: "python".into(),
                code: "print('hello')".into(),
                start_line: 2,
                scope: None,
            }),
            Segment::Html("</h1>\n".into()),
        ];

        let mut outputs = HashMap::new();
        outputs.insert(1, "hello".to_string());

        let result = splice_outputs(&segments, &outputs);
        assert_eq!(result, "<h1>hello</h1>\n");
    }

    #[test]
    fn splice_outputs_multiple_blocks() {
        let segments = vec![
            Segment::Html("<div>".into()),
            Segment::RuntimeBlock(RuntimeBlock {
                language: "python".into(),
                code: "".into(),
                start_line: 1,
                scope: None,
            }),
            Segment::Html("</div><div>".into()),
            Segment::RuntimeBlock(RuntimeBlock {
                language: "ruby".into(),
                code: "".into(),
                start_line: 3,
                scope: None,
            }),
            Segment::Html("</div>".into()),
        ];

        let mut outputs = HashMap::new();
        outputs.insert(1, "A".to_string());
        outputs.insert(3, "B".to_string());

        let result = splice_outputs(&segments, &outputs);
        assert_eq!(result, "<div>A</div><div>B</div>");
    }

    #[test]
    fn splice_outputs_missing_block_output() {
        let segments = vec![
            Segment::Html("<p>".into()),
            Segment::RuntimeBlock(RuntimeBlock {
                language: "python".into(),
                code: "".into(),
                start_line: 1,
                scope: None,
            }),
            Segment::Html("</p>".into()),
        ];

        let outputs = HashMap::new(); // No output for block
        let result = splice_outputs(&segments, &outputs);
        assert_eq!(result, "<p></p>");
    }

    #[test]
    fn boundary_marker_constant() {
        assert_eq!(BLOCK_BOUNDARY, "__SALATA_BLOCK_BOUNDARY__");
    }
}
