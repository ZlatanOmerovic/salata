use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{CgiEnv, RuntimeExecutor, BLOCK_BOUNDARY};
use crate::error::{SalataError, SalataResult};

/// TypeScript runtime executor.
/// Runner is configurable: ts-node, tsx, bun, deno.
///
/// Unlike other runtimes, TypeScript uses temp files instead of stdin piping
/// because ts-node does not support `-` for stdin input.
pub struct TypeScriptRuntime {
    binary: PathBuf,
}

impl TypeScriptRuntime {
    /// Create a new TypeScript runtime with the given runner path (ts-node, tsx, etc.).
    pub fn new(binary_path: &str) -> Self {
        Self {
            binary: PathBuf::from(binary_path),
        }
    }

    /// Run TypeScript code via a temp file (works with all TS runners).
    fn run_via_tempfile(
        &self,
        code: &str,
        env: &CgiEnv,
        file: &Path,
        start_line: usize,
    ) -> SalataResult<String> {
        if !self.binary.exists() {
            return Err(SalataError::RuntimeNotFound {
                runtime: "typescript".to_string(),
                path: self.binary.clone(),
            });
        }

        let mut tmp = tempfile::Builder::new()
            .suffix(".ts")
            .tempfile()
            .map_err(|e| SalataError::Runtime {
                runtime: "typescript".to_string(),
                file: file.to_path_buf(),
                line: start_line,
                message: format!("failed to create temp file: {e}"),
            })?;

        tmp.write_all(code.as_bytes())
            .map_err(|e| SalataError::Runtime {
                runtime: "typescript".to_string(),
                file: file.to_path_buf(),
                line: start_line,
                message: format!("failed to write temp file: {e}"),
            })?;

        let mut cmd = Command::new(&self.binary);
        cmd.arg(tmp.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        env.apply_to(&mut cmd);

        let output = cmd.output().map_err(|e| SalataError::Runtime {
            runtime: "typescript".to_string(),
            file: file.to_path_buf(),
            line: start_line,
            message: format!("failed to spawn {}: {e}", self.binary.display()),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SalataError::Runtime {
                runtime: "typescript".to_string(),
                file: file.to_path_buf(),
                line: start_line,
                message: stderr.trim().to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// TS print()/println() helpers — same as JS, TS runners support process.stdout.
/// Uses `any[]` type annotations for TypeScript compatibility.
const TS_PREAMBLE: &str = "\
function print(...args: any[]) { process.stdout.write(args.join(' ')); }
function println(...args: any[]) { process.stdout.write(args.join(' ') + '\\n'); }
";

impl RuntimeExecutor for TypeScriptRuntime {
    fn language(&self) -> &str {
        "typescript"
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn preamble(&self) -> Option<String> {
        Some(TS_PREAMBLE.to_string())
    }

    fn wrap_block_for_shared_scope(&self, _code: &str) -> String {
        format!("process.stdout.write(\"{BLOCK_BOUNDARY}\");\n")
    }

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
        self.run_via_tempfile(&full_code, env, file, start_line)
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
        let raw_output = self.run_via_tempfile(&combined, env, file, first_line)?;

        let parts: Vec<&str> = raw_output.split(BLOCK_BOUNDARY).collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ts_preamble_contains_print_helpers() {
        let rt = TypeScriptRuntime::new("/usr/bin/ts-node");
        let pre = rt.preamble().unwrap();
        assert!(pre.contains("function print("));
        assert!(pre.contains("function println("));
        assert!(pre.contains("any[]"));
    }

    #[test]
    fn ts_boundary_marker() {
        let rt = TypeScriptRuntime::new("/usr/bin/ts-node");
        let boundary = rt.wrap_block_for_shared_scope("");
        assert!(boundary.contains(BLOCK_BOUNDARY));
        assert!(boundary.contains("process.stdout.write"));
    }

    #[test]
    fn ts_language_name() {
        let rt = TypeScriptRuntime::new("/usr/bin/ts-node");
        assert_eq!(rt.language(), "typescript");
    }

    // Note: live execution tests for TypeScript require ts-node/tsx/bun installed,
    // which is not guaranteed locally. Those are covered by E2E tests in Docker.
}
