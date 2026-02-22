use std::path::{Path, PathBuf};

use super::{RuntimeExecutor, BLOCK_BOUNDARY};

/// JavaScript (Node.js) runtime executor.
pub struct JavaScriptRuntime {
    binary: PathBuf,
}

impl JavaScriptRuntime {
    /// Create a new JavaScript runtime with the given Node.js path.
    pub fn new(binary_path: &str) -> Self {
        Self {
            binary: PathBuf::from(binary_path),
        }
    }
}

/// JS print()/println() helpers injected before user code.
/// Additive — does not override console.log.
const JS_PREAMBLE: &str = "\
function print(...args) { process.stdout.write(args.join(' ')); }
function println(...args) { process.stdout.write(args.join(' ') + '\\n'); }
";

impl RuntimeExecutor for JavaScriptRuntime {
    fn language(&self) -> &str {
        "javascript"
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn args(&self) -> Vec<&str> {
        // Node.js uses `--input-type=module` is not needed; `-` reads stdin as CJS.
        // Just `-` for stdin or `--` + `-e` won't work. Node reads stdin with no file arg.
        // Actually node with `-` explicitly reads stdin.
        vec!["-"]
    }

    fn preamble(&self) -> Option<String> {
        Some(JS_PREAMBLE.to_string())
    }

    fn wrap_block_for_shared_scope(&self, _code: &str) -> String {
        format!("process.stdout.write(\"{BLOCK_BOUNDARY}\");\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::CgiEnv;

    fn find_node() -> Option<String> {
        for p in &[
            "/usr/bin/node",
            "/usr/local/bin/node",
            "/opt/homebrew/bin/node",
        ] {
            if Path::new(p).exists() {
                return Some(p.to_string());
            }
        }
        std::process::Command::new("which")
            .arg("node")
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
    fn js_single_block_console_log() {
        let Some(path) = find_node() else {
            eprintln!("node not found, skipping test");
            return;
        };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        let output = rt
            .execute_isolated(
                "console.log('hello world');",
                &env,
                Path::new("test.slt"),
                1,
            )
            .unwrap();
        assert_eq!(output.trim(), "hello world");
    }

    #[test]
    fn js_print_helper_no_newline() {
        let Some(path) = find_node() else { return };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        let code = "print('ab'); print('cd');";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output, "abcd");
    }

    #[test]
    fn js_println_helper_with_newline() {
        let Some(path) = find_node() else { return };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        let code = "println('line1'); println('line2');";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output, "line1\nline2\n");
    }

    #[test]
    fn js_print_multiple_args() {
        let Some(path) = find_node() else { return };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        let code = "print('a', 'b', 'c');";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output, "a b c");
    }

    #[test]
    fn js_console_log_still_works() {
        let Some(path) = find_node() else { return };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        // Both print helpers and console.log should work.
        let code = "print('A'); console.log('B');";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert!(output.contains('A'));
        assert!(output.contains('B'));
    }

    #[test]
    fn js_multiline() {
        let Some(path) = find_node() else { return };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        let code = "const x = 2;\nconst y = 3;\nconsole.log(x + y);";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output.trim(), "5");
    }

    #[test]
    fn js_shared_scope() {
        let Some(path) = find_node() else { return };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        let blocks = vec![
            ("var x = 42; print(String(x));".to_string(), 1),
            ("print(String(x + 1));".to_string(), 5),
        ];
        let outputs = rt
            .execute_shared(&blocks, &env, Path::new("test.slt"))
            .unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0], "42");
        assert_eq!(outputs[1], "43");
    }

    #[test]
    fn js_error_returns_runtime_error() {
        let Some(path) = find_node() else { return };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        let result =
            rt.execute_isolated("throw new Error('boom');", &env, Path::new("test.slt"), 1);
        assert!(result.is_err());
    }

    #[test]
    fn js_template_literals() {
        let Some(path) = find_node() else { return };
        let rt = JavaScriptRuntime::new(&path);
        let env = CgiEnv::default();
        let code = "const name = 'World'; print(`Hello ${name}`);";
        let output = rt
            .execute_isolated(code, &env, Path::new("test.slt"), 1)
            .unwrap();
        assert_eq!(output, "Hello World");
    }
}
