//! Cross-runtime data bridge via `#set`/`#get` macros.
//!
//! Macros work inside runtime blocks only. Before execution, salata expands
//! `#set("key", value)` and `#get("key")` into native function calls per
//! language, plus a preamble defining those functions. Data is exchanged via
//! JSON files in a shared temporary directory.
//!
//! # Supported Types
//!
//! Strings, numbers, booleans, arrays/lists, objects/dicts, and null.
//!
//! # Example
//!
//! ```text
//! <python>
//!   #set("users", [{"name": "Alice"}])
//! </python>
//! <javascript>
//!   const users = #get("users");
//! </javascript>
//! ```
//!
//! # Known Limitation
//!
//! Shell blocks produce invalid syntax — the expansion generates parenthesized
//! function calls (`__salata_set("key", "val")`) which is valid in Python/JS/Ruby/PHP
//! but not in shell, where function calls use space-separated arguments.

use std::path::{Path, PathBuf};

use crate::error::{SalataError, SalataResult};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Environment variable name for the macro data directory.
pub const MACRO_DIR_ENV: &str = "__SALATA_MACRO_DIR__";

// ---------------------------------------------------------------------------
// Macro data store
// ---------------------------------------------------------------------------

/// Manages a temporary directory for cross-runtime macro data exchange.
/// Each key is stored as a JSON file in this directory.
pub struct MacroDataStore {
    dir: PathBuf,
}

impl MacroDataStore {
    /// Create a new macro data store with a unique temp directory.
    pub fn new() -> SalataResult<Self> {
        let dir = std::env::temp_dir().join(format!("salata_macros_{}_{}", std::process::id(), {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            COUNTER.fetch_add(1, Ordering::Relaxed)
        }));
        std::fs::create_dir_all(&dir).map_err(SalataError::Io)?;
        Ok(Self { dir })
    }

    /// Get the directory path as a string for use in env vars.
    pub fn path_str(&self) -> &str {
        self.dir.to_str().unwrap_or("")
    }

    /// Get the directory path.
    pub fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for MacroDataStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Check if code contains any `#set(` or `#get(` macros.
pub fn has_macros(code: &str) -> bool {
    code.contains("#set(") || code.contains("#get(")
}

// ---------------------------------------------------------------------------
// Expansion
// ---------------------------------------------------------------------------

/// Expand `#set`/`#get` macros in a code string for the given language.
/// Returns the expanded code with the macro preamble prepended if needed.
///
/// The expansion is:
///   `#set(` → `__salata_set(`
///   `#get(` → `__salata_get(`
/// Plus a language-specific preamble defining those functions.
pub fn expand_macros(code: &str, language: &str) -> String {
    if !has_macros(code) {
        return code.to_string();
    }

    let expanded = code
        .replace("#set(", "__salata_set(")
        .replace("#get(", "__salata_get(");

    match macro_preamble(language) {
        Some(preamble) => format!("{preamble}\n{expanded}"),
        None => expanded,
    }
}

/// Get the macro helper preamble for a language.
/// Returns None if the language is not supported for macros.
pub fn macro_preamble(language: &str) -> Option<&'static str> {
    match language {
        "python" => Some(PYTHON_MACRO_PREAMBLE),
        "ruby" => Some(RUBY_MACRO_PREAMBLE),
        "javascript" => Some(JS_MACRO_PREAMBLE),
        "typescript" => Some(TS_MACRO_PREAMBLE),
        "php" => Some(PHP_MACRO_PREAMBLE),
        "shell" => Some(SHELL_MACRO_PREAMBLE),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Per-language preambles
// ---------------------------------------------------------------------------

const PYTHON_MACRO_PREAMBLE: &str = "\
import json as _sJ, os as _sO
def __salata_set(_k, _v):
    with open(_sO.path.join(_sO.environ['__SALATA_MACRO_DIR__'], _k + '.json'), 'w') as _f:
        _sJ.dump(_v, _f)
def __salata_get(_k, _d=None):
    _p = _sO.path.join(_sO.environ['__SALATA_MACRO_DIR__'], _k + '.json')
    if _sO.path.exists(_p):
        with open(_p) as _f:
            return _sJ.load(_f)
    return _d";

const RUBY_MACRO_PREAMBLE: &str = "\
require 'json'
def __salata_set(k, v)
  File.write(File.join(ENV['__SALATA_MACRO_DIR__'], \"#{k}.json\"), JSON.generate(v))
end
def __salata_get(k, d=nil)
  p = File.join(ENV['__SALATA_MACRO_DIR__'], \"#{k}.json\")
  File.exist?(p) ? JSON.parse(File.read(p)) : d
end";

const JS_MACRO_PREAMBLE: &str = "\
const __sFs = require('fs'), __sPa = require('path');
function __salata_set(k, v) { __sFs.writeFileSync(__sPa.join(process.env.__SALATA_MACRO_DIR__, k + '.json'), JSON.stringify(v)); }
function __salata_get(k, d = null) { try { return JSON.parse(__sFs.readFileSync(__sPa.join(process.env.__SALATA_MACRO_DIR__, k + '.json'), 'utf8')); } catch(e) { return d; } }";

const TS_MACRO_PREAMBLE: &str = "\
const __sFs = require('fs'), __sPa = require('path');
function __salata_set(k: string, v: any): void { __sFs.writeFileSync(__sPa.join(process.env.__SALATA_MACRO_DIR__, k + '.json'), JSON.stringify(v)); }
function __salata_get(k: string, d: any = null): any { try { return JSON.parse(__sFs.readFileSync(__sPa.join(process.env.__SALATA_MACRO_DIR__, k + '.json'), 'utf8')); } catch(e) { return d; } }";

const PHP_MACRO_PREAMBLE: &str = "\
function __salata_set($k, $v) { file_put_contents(getenv('__SALATA_MACRO_DIR__') . '/' . $k . '.json', json_encode($v)); }
function __salata_get($k, $d = null) { $p = getenv('__SALATA_MACRO_DIR__') . '/' . $k . '.json'; return file_exists($p) ? json_decode(file_get_contents($p), true) : $d; }";

const SHELL_MACRO_PREAMBLE: &str = "\
__salata_set() { printf '%s' \"$2\" > \"${__SALATA_MACRO_DIR__}/$1.json\"; }
__salata_get() { local _p=\"${__SALATA_MACRO_DIR__}/$1.json\"; if [ -f \"$_p\" ]; then cat \"$_p\"; else printf '%s' \"${2:-null}\"; fi; }";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Detection --

    #[test]
    fn detects_set_macro() {
        assert!(has_macros("x = 5\n#set(\"count\", x)"));
    }

    #[test]
    fn detects_get_macro() {
        assert!(has_macros("val = #get(\"count\")"));
    }

    #[test]
    fn detects_both_macros() {
        assert!(has_macros("#set(\"a\", 1)\n#get(\"a\")"));
    }

    #[test]
    fn no_macros_in_plain_code() {
        assert!(!has_macros("print('hello world')"));
    }

    #[test]
    fn no_macros_partial_match() {
        // #setting and #getter should not match
        assert!(!has_macros("x = #setting\ny = #getter"));
    }

    // -- Expansion (no macros) --

    #[test]
    fn expand_no_macros_returns_unchanged() {
        let code = "print('hello')";
        assert_eq!(expand_macros(code, "python"), code);
    }

    // -- Expansion (set) --

    #[test]
    fn expand_set_python() {
        let code = "#set(\"key\", value)";
        let expanded = expand_macros(code, "python");
        assert!(expanded.contains("__salata_set(\"key\", value)"));
        assert!(expanded.contains("def __salata_set("));
        assert!(expanded.contains("json"));
    }

    #[test]
    fn expand_set_ruby() {
        let code = "#set(\"key\", value)";
        let expanded = expand_macros(code, "ruby");
        assert!(expanded.contains("__salata_set(\"key\", value)"));
        assert!(expanded.contains("def __salata_set("));
        assert!(expanded.contains("JSON.generate"));
    }

    #[test]
    fn expand_set_javascript() {
        let code = "#set(\"key\", value)";
        let expanded = expand_macros(code, "javascript");
        assert!(expanded.contains("__salata_set(\"key\", value)"));
        assert!(expanded.contains("function __salata_set("));
        assert!(expanded.contains("JSON.stringify"));
    }

    #[test]
    fn expand_set_typescript() {
        let code = "#set(\"key\", value)";
        let expanded = expand_macros(code, "typescript");
        assert!(expanded.contains("__salata_set(\"key\", value)"));
        assert!(expanded.contains("function __salata_set("));
        assert!(expanded.contains("k: string"));
    }

    #[test]
    fn expand_set_php() {
        let code = "#set(\"key\", $value)";
        let expanded = expand_macros(code, "php");
        assert!(expanded.contains("__salata_set(\"key\", $value)"));
        assert!(expanded.contains("function __salata_set("));
        assert!(expanded.contains("json_encode"));
    }

    #[test]
    fn expand_set_shell() {
        let code = "#set(\"key\", \"$value\")";
        let expanded = expand_macros(code, "shell");
        assert!(expanded.contains("__salata_set(\"key\", \"$value\")"));
        assert!(expanded.contains("__salata_set()"));
    }

    // -- Expansion (get) --

    #[test]
    fn expand_get_python() {
        let code = "val = #get(\"key\")";
        let expanded = expand_macros(code, "python");
        assert!(expanded.contains("val = __salata_get(\"key\")"));
        assert!(expanded.contains("def __salata_get("));
    }

    #[test]
    fn expand_get_with_default_python() {
        let code = "val = #get(\"key\", 42)";
        let expanded = expand_macros(code, "python");
        assert!(expanded.contains("val = __salata_get(\"key\", 42)"));
    }

    #[test]
    fn expand_get_ruby() {
        let code = "val = #get(\"key\")";
        let expanded = expand_macros(code, "ruby");
        assert!(expanded.contains("val = __salata_get(\"key\")"));
        assert!(expanded.contains("def __salata_get("));
    }

    #[test]
    fn expand_get_javascript() {
        let code = "const val = #get(\"key\");";
        let expanded = expand_macros(code, "javascript");
        assert!(expanded.contains("const val = __salata_get(\"key\");"));
        assert!(expanded.contains("function __salata_get("));
    }

    #[test]
    fn expand_get_with_default_javascript() {
        let code = "const val = #get(\"key\", []);";
        let expanded = expand_macros(code, "javascript");
        assert!(expanded.contains("const val = __salata_get(\"key\", []);"));
    }

    #[test]
    fn expand_get_typescript() {
        let code = "const val = #get(\"key\");";
        let expanded = expand_macros(code, "typescript");
        assert!(expanded.contains("const val = __salata_get(\"key\");"));
        assert!(expanded.contains("d: any"));
    }

    #[test]
    fn expand_get_php() {
        let code = "$val = #get(\"key\");";
        let expanded = expand_macros(code, "php");
        assert!(expanded.contains("$val = __salata_get(\"key\");"));
        assert!(expanded.contains("function __salata_get("));
        assert!(expanded.contains("json_decode"));
    }

    #[test]
    fn expand_get_shell() {
        let code = "val=$(#get(\"key\"))";
        let expanded = expand_macros(code, "shell");
        assert!(expanded.contains("val=$(__salata_get(\"key\"))"));
    }

    // -- Multiple macros in one block --

    #[test]
    fn expand_multiple_macros() {
        let code = "x = 42\n#set(\"count\", x)\ny = #get(\"name\", \"default\")";
        let expanded = expand_macros(code, "python");
        assert!(expanded.contains("__salata_set(\"count\", x)"));
        assert!(expanded.contains("y = __salata_get(\"name\", \"default\")"));
        // Preamble appears exactly once (at the top)
        let preamble_count = expanded.matches("def __salata_set(").count();
        assert_eq!(preamble_count, 1);
    }

    // -- Unknown language --

    #[test]
    fn expand_unknown_language_still_replaces() {
        let code = "#set(\"key\", val)";
        let expanded = expand_macros(code, "unknown");
        // Replacement happens but no preamble
        assert!(expanded.contains("__salata_set(\"key\", val)"));
        assert!(!expanded.contains("def "));
        assert!(!expanded.contains("function "));
    }

    // -- Preamble content checks --

    #[test]
    fn python_preamble_has_json_import() {
        let pre = macro_preamble("python").unwrap();
        assert!(pre.contains("import json"));
        assert!(pre.contains("__SALATA_MACRO_DIR__"));
    }

    #[test]
    fn ruby_preamble_has_json_require() {
        let pre = macro_preamble("ruby").unwrap();
        assert!(pre.contains("require 'json'"));
        assert!(pre.contains("__SALATA_MACRO_DIR__"));
    }

    #[test]
    fn js_preamble_has_fs_require() {
        let pre = macro_preamble("javascript").unwrap();
        assert!(pre.contains("require('fs')"));
        assert!(pre.contains("__SALATA_MACRO_DIR__"));
    }

    #[test]
    fn ts_preamble_has_type_annotations() {
        let pre = macro_preamble("typescript").unwrap();
        assert!(pre.contains("k: string"));
        assert!(pre.contains("v: any"));
    }

    #[test]
    fn php_preamble_has_file_put_contents() {
        let pre = macro_preamble("php").unwrap();
        assert!(pre.contains("file_put_contents"));
        assert!(pre.contains("json_encode"));
        assert!(pre.contains("json_decode"));
    }

    #[test]
    fn shell_preamble_has_printf() {
        let pre = macro_preamble("shell").unwrap();
        assert!(pre.contains("printf"));
        assert!(pre.contains("__SALATA_MACRO_DIR__"));
    }

    #[test]
    fn no_preamble_for_unknown_language() {
        assert!(macro_preamble("brainfuck").is_none());
    }

    // -- MacroDataStore --

    #[test]
    fn macro_data_store_creates_dir() {
        let store = MacroDataStore::new().unwrap();
        assert!(store.path().exists());
        assert!(store.path().is_dir());
        assert!(!store.path_str().is_empty());
    }

    #[test]
    fn macro_data_store_cleanup_on_drop() {
        let path;
        {
            let store = MacroDataStore::new().unwrap();
            path = store.path().to_path_buf();
            assert!(path.exists());
        }
        // After drop, directory should be removed.
        assert!(!path.exists());
    }

    #[test]
    fn macro_data_store_unique_dirs() {
        let s1 = MacroDataStore::new().unwrap();
        let s2 = MacroDataStore::new().unwrap();
        assert_ne!(s1.path(), s2.path());
    }

    // -- Live execution: Python set/get round-trip --

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

    /// Helper: run expanded code in a runtime process with the macro env var set.
    fn run_with_macros(
        binary: &str,
        args: &[&str],
        code: &str,
        language: &str,
        store: &MacroDataStore,
        preamble: Option<&str>,
    ) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let expanded = expand_macros(code, language);
        let full_code = match preamble {
            Some(pre) => format!("{pre}\n{expanded}"),
            None => expanded,
        };

        let mut cmd = Command::new(binary);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env(MACRO_DIR_ENV, store.path_str());

        let mut child = cmd.spawn().expect("failed to spawn process");
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(full_code.as_bytes()).unwrap();
        }
        let output = child.wait_with_output().unwrap();
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("{language} failed: {stderr}\ncode:\n{full_code}");
        }
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[test]
    fn python_set_get_roundtrip() {
        let Some(py) = find_python() else {
            eprintln!("python3 not found, skipping");
            return;
        };
        let store = MacroDataStore::new().unwrap();

        // Set values
        run_with_macros(
            &py,
            &["-u", "-"],
            "#set(\"name\", \"Alice\")\n#set(\"count\", 42)\n#set(\"items\", [1, 2, 3])",
            "python",
            &store,
            None,
        );

        // Get values
        let output = run_with_macros(
            &py,
            &["-u", "-"],
            "print(#get(\"name\"))\nprint(#get(\"count\"))\nprint(#get(\"items\"))",
            "python",
            &store,
            None,
        );
        assert!(output.contains("Alice"));
        assert!(output.contains("42"));
        assert!(output.contains("[1, 2, 3]"));
    }

    #[test]
    fn python_get_default() {
        let Some(py) = find_python() else { return };
        let store = MacroDataStore::new().unwrap();

        let output = run_with_macros(
            &py,
            &["-u", "-"],
            "print(#get(\"missing\", \"fallback\"))",
            "python",
            &store,
            None,
        );
        assert_eq!(output.trim(), "fallback");
    }

    #[test]
    fn python_get_none_default() {
        let Some(py) = find_python() else { return };
        let store = MacroDataStore::new().unwrap();

        let output = run_with_macros(
            &py,
            &["-u", "-"],
            "print(#get(\"missing\"))",
            "python",
            &store,
            None,
        );
        assert_eq!(output.trim(), "None");
    }

    #[test]
    fn python_set_object() {
        let Some(py) = find_python() else { return };
        let store = MacroDataStore::new().unwrap();

        run_with_macros(
            &py,
            &["-u", "-"],
            "#set(\"user\", {\"name\": \"Bob\", \"age\": 30})",
            "python",
            &store,
            None,
        );

        let output = run_with_macros(
            &py,
            &["-u", "-"],
            "u = #get(\"user\")\nprint(u[\"name\"])\nprint(u[\"age\"])",
            "python",
            &store,
            None,
        );
        assert!(output.contains("Bob"));
        assert!(output.contains("30"));
    }

    #[test]
    fn python_set_boolean_and_null() {
        let Some(py) = find_python() else { return };
        let store = MacroDataStore::new().unwrap();

        run_with_macros(
            &py,
            &["-u", "-"],
            "#set(\"flag\", True)\n#set(\"empty\", None)",
            "python",
            &store,
            None,
        );

        let output = run_with_macros(
            &py,
            &["-u", "-"],
            "print(#get(\"flag\"))\nprint(#get(\"empty\"))",
            "python",
            &store,
            None,
        );
        assert!(output.contains("True"));
        assert!(output.contains("None"));
    }

    // -- Cross-runtime: Python → JavaScript --

    #[test]
    fn cross_runtime_python_to_js() {
        let Some(py) = find_python() else { return };
        let Some(node) = find_node() else { return };
        let store = MacroDataStore::new().unwrap();

        // Python sets data
        run_with_macros(
            &py,
            &["-u", "-"],
            "#set(\"users\", [{\"name\": \"Alice\"}, {\"name\": \"Bob\"}])\n#set(\"count\", 2)",
            "python",
            &store,
            None,
        );

        // JavaScript reads data
        let js_preamble = "function print(...a){process.stdout.write(a.join(' '));}function println(...a){process.stdout.write(a.join(' ')+'\\n');}";
        let output = run_with_macros(
            &node,
            &["-"],
            "const users = #get(\"users\");\nconst count = #get(\"count\");\nprintln(`Found ${count} users`);\nprintln(users[0].name);",
            "javascript",
            &store,
            Some(js_preamble),
        );
        assert!(output.contains("Found 2 users"));
        assert!(output.contains("Alice"));
    }

    // -- Cross-runtime: JavaScript → Python --

    #[test]
    fn cross_runtime_js_to_python() {
        let Some(py) = find_python() else { return };
        let Some(node) = find_node() else { return };
        let store = MacroDataStore::new().unwrap();

        // JavaScript sets data
        run_with_macros(
            &node,
            &["-"],
            "#set(\"greeting\", \"hello from js\");\n#set(\"nums\", [10, 20, 30]);",
            "javascript",
            &store,
            None,
        );

        // Python reads data
        let output = run_with_macros(
            &py,
            &["-u", "-"],
            "print(#get(\"greeting\"))\nprint(sum(#get(\"nums\")))",
            "python",
            &store,
            None,
        );
        assert!(output.contains("hello from js"));
        assert!(output.contains("60"));
    }

    // -- Cross-runtime: Ruby → Python --

    #[test]
    fn cross_runtime_ruby_to_python() {
        let Some(py) = find_python() else { return };
        let Some(rb) = find_ruby() else { return };
        let store = MacroDataStore::new().unwrap();

        // Ruby sets data
        run_with_macros(
            &rb,
            &[],
            "#set(\"lang\", \"ruby\")\n#set(\"version\", 3)",
            "ruby",
            &store,
            None,
        );

        // Python reads data
        let output = run_with_macros(
            &py,
            &["-u", "-"],
            "print(#get(\"lang\"))\nprint(#get(\"version\") + 1)",
            "python",
            &store,
            None,
        );
        assert!(output.contains("ruby"));
        assert!(output.contains("4"));
    }

    // -- Supported types round-trip via Python --

    #[test]
    fn python_all_types_roundtrip() {
        let Some(py) = find_python() else { return };
        let store = MacroDataStore::new().unwrap();

        run_with_macros(
            &py,
            &["-u", "-"],
            concat!(
                "#set(\"str\", \"hello\")\n",
                "#set(\"int\", 42)\n",
                "#set(\"float\", 3.14)\n",
                "#set(\"bool_t\", True)\n",
                "#set(\"bool_f\", False)\n",
                "#set(\"null\", None)\n",
                "#set(\"list\", [1, \"two\", 3])\n",
                "#set(\"dict\", {\"a\": 1, \"b\": [2, 3]})\n",
            ),
            "python",
            &store,
            None,
        );

        let output = run_with_macros(
            &py,
            &["-u", "-"],
            concat!(
                "print(type(#get(\"str\")).__name__)\n",
                "print(type(#get(\"int\")).__name__)\n",
                "print(type(#get(\"float\")).__name__)\n",
                "print(type(#get(\"bool_t\")).__name__)\n",
                "print(#get(\"bool_f\"))\n",
                "print(#get(\"null\"))\n",
                "print(len(#get(\"list\")))\n",
                "print(#get(\"dict\")[\"b\"][1])\n",
            ),
            "python",
            &store,
            None,
        );
        assert!(output.contains("str"));
        assert!(output.contains("int"));
        assert!(output.contains("float"));
        assert!(output.contains("bool"));
        assert!(output.contains("False"));
        assert!(output.contains("None"));
        assert!(output.contains("3")); // len of list
    }

    // -- JavaScript types round-trip --

    #[test]
    fn js_all_types_roundtrip() {
        let Some(node) = find_node() else { return };
        let store = MacroDataStore::new().unwrap();

        let js_preamble = "function println(...a){process.stdout.write(a.join(' ')+'\\n');}";

        run_with_macros(
            &node,
            &["-"],
            concat!(
                "#set(\"str\", \"hello\");\n",
                "#set(\"num\", 42);\n",
                "#set(\"flt\", 3.14);\n",
                "#set(\"bool\", true);\n",
                "#set(\"nil\", null);\n",
                "#set(\"arr\", [1, 2, 3]);\n",
                "#set(\"obj\", {a: 1, b: [2]});\n",
            ),
            "javascript",
            &store,
            Some(js_preamble),
        );

        let output = run_with_macros(
            &node,
            &["-"],
            concat!(
                "println(typeof #get(\"str\"));\n",
                "println(typeof #get(\"num\"));\n",
                "println(String(#get(\"bool\")));\n",
                "println(JSON.stringify(#get(\"nil\")));\n",
                "println(String(#get(\"arr\").length));\n",
                "println(String(#get(\"obj\").b[0]));\n",
            ),
            "javascript",
            &store,
            Some(js_preamble),
        );
        assert!(output.contains("string"));
        assert!(output.contains("number"));
        assert!(output.contains("true"));
        assert!(output.contains("null"));
        assert!(output.contains("3"));
        assert!(output.contains("2"));
    }

    // -- Ruby round-trip --

    #[test]
    fn ruby_set_get_roundtrip() {
        let Some(rb) = find_ruby() else { return };
        let store = MacroDataStore::new().unwrap();

        run_with_macros(
            &rb,
            &[],
            "#set(\"name\", \"Alice\")\n#set(\"nums\", [1, 2, 3])",
            "ruby",
            &store,
            None,
        );

        let output = run_with_macros(
            &rb,
            &[],
            "puts #get(\"name\")\nputs #get(\"nums\").length",
            "ruby",
            &store,
            None,
        );
        assert!(output.contains("Alice"));
        assert!(output.contains("3"));
    }

    #[test]
    fn ruby_get_default() {
        let Some(rb) = find_ruby() else { return };
        let store = MacroDataStore::new().unwrap();

        let output = run_with_macros(
            &rb,
            &[],
            "puts #get(\"missing\", \"fallback\")",
            "ruby",
            &store,
            None,
        );
        assert_eq!(output.trim(), "fallback");
    }
}
