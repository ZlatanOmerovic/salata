use std::path::{Path, PathBuf};

use super::{RuntimeExecutor, BLOCK_BOUNDARY};

/// Ruby runtime executor.
pub struct RubyRuntime {
    binary: PathBuf,
}

impl RubyRuntime {
    /// Create a new Ruby runtime with the given interpreter path.
    pub fn new(binary_path: &str) -> Self {
        Self {
            binary: PathBuf::from(binary_path),
        }
    }
}

impl RuntimeExecutor for RubyRuntime {
    fn language(&self) -> &str {
        "ruby"
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn args(&self) -> Vec<&str> {
        // Ruby reads from stdin with no file argument.
        vec![]
    }

    fn wrap_block_for_shared_scope(&self, _code: &str) -> String {
        format!("$stdout.write(\"{BLOCK_BOUNDARY}\")\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::CgiEnv;

    fn find_ruby() -> Option<String> {
        for p in &[
            "/usr/bin/ruby",
            "/usr/local/bin/ruby",
            "/opt/homebrew/bin/ruby",
        ] {
            if Path::new(p).exists() {
                return Some(p.to_string());
            }
        }
        std::process::Command::new("which")
            .arg("ruby")
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
    fn ruby_single_block() {
        let Some(path) = find_ruby() else {
            eprintln!("ruby not found, skipping test");
            return;
        };
        let rt = RubyRuntime::new(&path);
        let env = CgiEnv::default();
        let output = rt
            .execute_isolated("puts 'hello world'", &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "hello world");
    }

    #[test]
    fn ruby_multiline() {
        let Some(path) = find_ruby() else { return };
        let rt = RubyRuntime::new(&path);
        let env = CgiEnv::default();
        let code = "x = 2\ny = 3\nputs x + y";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "5");
    }

    #[test]
    fn ruby_shared_scope() {
        let Some(path) = find_ruby() else { return };
        let rt = RubyRuntime::new(&path);
        let env = CgiEnv::default();
        let blocks = vec![
            ("x = 42\n$stdout.write(x.to_s)".to_string(), 1),
            ("$stdout.write((x + 1).to_s)".to_string(), 5),
        ];
        let outputs = rt
            .execute_shared(&blocks, &env, Path::new("test.slt"))
            .unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].trim(), "42");
        assert_eq!(outputs[1].trim(), "43");
    }

    #[test]
    fn ruby_error_returns_runtime_error() {
        let Some(path) = find_ruby() else { return };
        let rt = RubyRuntime::new(&path);
        let env = CgiEnv::default();
        let result = rt.execute_isolated("raise 'boom'", &env, Path::new("test.slt"), 1);
        assert!(result.is_err());
    }
}
