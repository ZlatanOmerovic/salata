use std::path::{Path, PathBuf};

use super::{RuntimeExecutor, BLOCK_BOUNDARY};

/// Python runtime executor.
pub struct PythonRuntime {
    binary: PathBuf,
}

impl PythonRuntime {
    /// Create a new Python runtime with the given interpreter path.
    pub fn new(binary_path: &str) -> Self {
        Self {
            binary: PathBuf::from(binary_path),
        }
    }
}

impl RuntimeExecutor for PythonRuntime {
    fn language(&self) -> &str {
        "python"
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn args(&self) -> Vec<&str> {
        // Python reads from stdin with `-` or no file arg + piped stdin.
        // Using `-u` for unbuffered output and `-` to read from stdin.
        vec!["-u", "-"]
    }

    fn wrap_block_for_shared_scope(&self, _code: &str) -> String {
        format!("print(\"{BLOCK_BOUNDARY}\", end=\"\")\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::CgiEnv;

    fn find_python() -> Option<String> {
        for p in &[
            "/usr/bin/python3",
            "/usr/local/bin/python3",
            "/opt/homebrew/bin/python3",
        ] {
            if Path::new(p).exists() {
                return Some(p.to_string());
            }
        }
        // Try `which`
        std::process::Command::new("which")
            .arg("python3")
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
    fn python_single_block() {
        let Some(path) = find_python() else {
            eprintln!("python3 not found, skipping test");
            return;
        };
        let rt = PythonRuntime::new(&path);
        let env = CgiEnv::default();
        let output = rt
            .execute_isolated("print('hello world')", &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "hello world");
    }

    #[test]
    fn python_multiline() {
        let Some(path) = find_python() else { return };
        let rt = PythonRuntime::new(&path);
        let env = CgiEnv::default();
        let code = "x = 2\ny = 3\nprint(x + y)";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "5");
    }

    #[test]
    fn python_shared_scope() {
        let Some(path) = find_python() else { return };
        let rt = PythonRuntime::new(&path);
        let env = CgiEnv::default();
        let blocks = vec![
            ("x = 42\nprint(x, end='')".to_string(), 1),
            ("print(x + 1, end='')".to_string(), 5),
        ];
        let outputs = rt
            .execute_shared(&blocks, &env, Path::new("test.slt"))
            .unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].trim(), "42");
        assert_eq!(outputs[1].trim(), "43");
    }

    #[test]
    fn python_error_returns_runtime_error() {
        let Some(path) = find_python() else { return };
        let rt = PythonRuntime::new(&path);
        let env = CgiEnv::default();
        let result =
            rt.execute_isolated("raise ValueError('boom')", &env, Path::new("test.slt"), 1);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("ValueError"));
    }
}
