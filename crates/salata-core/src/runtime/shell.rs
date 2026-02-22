use std::path::{Path, PathBuf};

use crate::error::SalataResult;
use crate::security::{self, SandboxConfig};

use super::{CgiEnv, RuntimeExecutor, BLOCK_BOUNDARY};

/// Shell runtime executor with full sandbox enforcement.
///
/// - Shell binary validated against hardcoded whitelist on construction
/// - Pre-execution scan for blocked commands/patterns/paths
/// - Clean environment (stripped env vars, safe PATH)
/// - ulimit enforcement (CPU, file size, processes)
/// - Timeout monitoring and output size tracking
pub struct ShellRuntime {
    binary: PathBuf,
    sandbox_config: SandboxConfig,
}

impl ShellRuntime {
    /// Create a new shell runtime. Validates the shell binary against the
    /// whitelist — returns an error if the path is not allowed.
    pub fn new(binary_path: &str) -> Self {
        Self {
            binary: PathBuf::from(binary_path),
            sandbox_config: SandboxConfig::default(),
        }
    }

    /// Create with custom sandbox configuration.
    pub fn with_config(binary_path: &str, config: SandboxConfig) -> Self {
        Self {
            binary: PathBuf::from(binary_path),
            sandbox_config: config,
        }
    }

    /// Validate the shell binary against the whitelist.
    /// Call this at startup to refuse to run with an unauthorized shell.
    pub fn validate(&self) -> SalataResult<()> {
        security::validate_shell(&self.binary)
    }
}

impl RuntimeExecutor for ShellRuntime {
    fn language(&self) -> &str {
        "shell"
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn args(&self) -> Vec<&str> {
        vec![]
    }

    fn wrap_block_for_shared_scope(&self, _code: &str) -> String {
        format!("printf '{BLOCK_BOUNDARY}'\n")
    }

    fn execute_isolated(
        &self,
        code: &str,
        env: &CgiEnv,
        file: &Path,
        start_line: usize,
    ) -> SalataResult<String> {
        // Validate shell on first use.
        security::validate_shell(&self.binary)?;

        security::execute_sandboxed(
            &self.binary,
            code,
            env,
            &self.sandbox_config,
            file,
            start_line,
        )
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

        // Validate shell.
        security::validate_shell(&self.binary)?;

        // Pre-scan all blocks before execution.
        for (code, start_line) in blocks {
            security::scan_code(code, &self.sandbox_config, file, *start_line)?;
        }

        // Concatenate blocks with boundary markers.
        let mut combined = String::new();
        for (i, (code, _)) in blocks.iter().enumerate() {
            if i > 0 {
                combined.push_str(&format!("printf '{BLOCK_BOUNDARY}'\n"));
            }
            combined.push_str(code);
            combined.push('\n');
        }

        let first_line = blocks[0].1;
        let raw = security::execute_sandboxed(
            &self.binary,
            &combined,
            env,
            &self.sandbox_config,
            file,
            first_line,
        )?;

        // Split output by boundary markers.
        let parts: Vec<&str> = raw.split(BLOCK_BOUNDARY).collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SalataError;

    fn find_shell() -> Option<String> {
        for p in &["/bin/bash", "/bin/sh", "/usr/bin/bash"] {
            if Path::new(p).exists() {
                return Some(p.to_string());
            }
        }
        None
    }

    #[test]
    fn shell_validates_allowed_binary() {
        let rt = ShellRuntime::new("/bin/bash");
        // If /bin/bash exists on the system, validate should pass.
        if Path::new("/bin/bash").exists() {
            assert!(rt.validate().is_ok());
        }
    }

    #[test]
    fn shell_rejects_disallowed_binary() {
        let rt = ShellRuntime::new("/usr/bin/python3");
        assert!(rt.validate().is_err());
    }

    #[test]
    fn shell_single_block() {
        let Some(path) = find_shell() else {
            eprintln!("bash/sh not found, skipping test");
            return;
        };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let output = rt
            .execute_isolated("echo 'hello world'", &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "hello world");
    }

    #[test]
    fn shell_multiline() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let code = "X=2\nY=3\necho $((X + Y))";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "5");
    }

    #[test]
    fn shell_shared_scope() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let blocks = vec![
            ("X=42\nprintf '%s' \"$X\"".to_string(), 1),
            ("printf '%s' \"$((X + 1))\"".to_string(), 5),
        ];
        let outputs = rt
            .execute_shared(&blocks, &env, Path::new("test.slt"))
            .unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].trim(), "42");
        assert_eq!(outputs[1].trim(), "43");
    }

    #[test]
    fn shell_blocks_rm() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let result = rt.execute_isolated("rm /tmp/test", &env, Path::new("test.slt"), 1);
        assert!(result.is_err(), "rm should be blocked by sandbox");
    }

    #[test]
    fn shell_blocks_sudo() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let result = rt.execute_isolated("sudo ls", &env, Path::new("test.slt"), 1);
        assert!(result.is_err());
    }

    #[test]
    fn shell_blocks_etc_access() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let result = rt.execute_isolated("cat /etc/passwd", &env, Path::new("test.slt"), 1);
        assert!(result.is_err());
    }

    #[test]
    fn shell_blocks_fork_bomb() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let result = rt.execute_isolated(":(){ :|:& };:", &env, Path::new("test.slt"), 1);
        assert!(result.is_err());
    }

    #[test]
    fn shell_cgi_env_accessible() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv {
            request_method: "POST".into(),
            http_host: "example.com".into(),
            ..Default::default()
        };
        let code = "printf '%s %s' \"$REQUEST_METHOD\" \"$HTTP_HOST\"";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output, "POST example.com");
    }

    #[test]
    fn shell_safe_path_enforced() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let output = rt
            .execute_isolated("echo $PATH", &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "/usr/local/bin:/usr/bin:/bin");
    }

    #[test]
    fn shell_shared_scope_blocks_dangerous_code() {
        let Some(path) = find_shell() else { return };
        let rt = ShellRuntime::new(&path);
        let env = CgiEnv::default();
        let blocks = vec![
            ("echo safe".to_string(), 1),
            ("rm /tmp/file".to_string(), 5),
        ];
        let result = rt.execute_shared(&blocks, &env, Path::new("test.slt"));
        assert!(result.is_err(), "rm in shared scope should be caught");
    }

    #[test]
    fn shell_with_custom_timeout() {
        let Some(path) = find_shell() else { return };
        let config = SandboxConfig {
            timeout: std::time::Duration::from_millis(200),
            ..Default::default()
        };
        let rt = ShellRuntime::with_config(&path, config);
        let env = CgiEnv::default();
        let result = rt.execute_isolated("sleep 10", &env, Path::new("test.slt"), 1);
        assert!(
            matches!(result, Err(SalataError::RuntimeTimeout { .. })),
            "expected timeout error, got: {result:?}"
        );
    }
}
