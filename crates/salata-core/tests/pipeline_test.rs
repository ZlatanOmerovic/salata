//! Integration tests for the full Salata processing pipeline.
//!
//! These tests use sample .slt fixture files and exercise the complete flow:
//! read → parse → resolve includes → resolve directives → expand macros →
//! execute runtimes → splice outputs.

use std::path::{Path, PathBuf};

use salata_core::config::SalataConfig;
use salata_core::context::ExecutionContext;
use salata_core::runtime::CgiEnv;
use salata_core::{process_file, process_source};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

fn find_runtime(names: &[&str]) -> Option<String> {
    for name in names {
        if Path::new(name).exists() {
            return Some(name.to_string());
        }
    }
    std::process::Command::new("which")
        .arg(names[0].rsplit('/').next().unwrap_or(names[0]))
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

fn find_python() -> Option<String> {
    find_runtime(&[
        "/usr/bin/python3",
        "/usr/local/bin/python3",
        "/opt/homebrew/bin/python3",
    ])
}

fn find_node() -> Option<String> {
    find_runtime(&[
        "/usr/bin/node",
        "/usr/local/bin/node",
        "/opt/homebrew/bin/node",
    ])
}

fn find_ruby() -> Option<String> {
    find_runtime(&[
        "/usr/bin/ruby",
        "/usr/local/bin/ruby",
        "/opt/homebrew/bin/ruby",
    ])
}

fn find_bash() -> Option<String> {
    find_runtime(&["/bin/bash", "/usr/bin/bash"])
}

/// Build a config TOML with the locally available runtimes.
fn build_config_toml() -> Option<String> {
    let python = find_python()?;
    let mut toml = format!(
        r#"
[runtimes.python]
path = "{python}"
shared_scope = true
"#
    );

    if let Some(node) = find_node() {
        toml.push_str(&format!(
            r#"
[runtimes.javascript]
path = "{node}"
shared_scope = true
"#
        ));
    }

    if let Some(ruby) = find_ruby() {
        toml.push_str(&format!(
            r#"
[runtimes.ruby]
path = "{ruby}"
shared_scope = true
"#
        ));
    }

    if let Some(bash) = find_bash() {
        toml.push_str(&format!(
            r#"
[runtimes.shell]
path = "{bash}"
shared_scope = true
"#
        ));
    }

    Some(toml)
}

fn config() -> Option<SalataConfig> {
    let toml = build_config_toml()?;
    SalataConfig::parse(&toml).ok()
}

fn env() -> CgiEnv {
    CgiEnv::default()
}

// ---------------------------------------------------------------------------
// Tests: HTML-only (no runtimes needed)
// ---------------------------------------------------------------------------

#[test]
fn html_only_passthrough() {
    let cfg = SalataConfig::parse("").unwrap();
    let file = fixtures_dir().join("html_only.slt");
    let result = process_file(&file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    assert!(result.html.contains("<p>No runtime blocks here</p>"));
    assert!(result.html.contains("<!DOCTYPE html>"));
    assert_eq!(result.directives.status, 200);
}

// ---------------------------------------------------------------------------
// Tests: Simple single-language
// ---------------------------------------------------------------------------

#[test]
fn simple_python_execution() {
    let Some(cfg) = config() else {
        eprintln!("python3 not found, skipping");
        return;
    };
    let file = fixtures_dir().join("simple_python.slt");
    let result = process_file(&file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    assert!(result.html.contains("<h1>Hello</h1>"));
    assert!(result.html.contains("World"));
    assert!(result.html.contains("<p>done</p>"));
}

// ---------------------------------------------------------------------------
// Tests: Multi-language
// ---------------------------------------------------------------------------

#[test]
fn multi_language_execution() {
    let Some(cfg) = config() else { return };
    let has_node = cfg.runtimes.javascript.is_some();
    let has_ruby = cfg.runtimes.ruby.is_some();
    let has_shell = cfg.runtimes.shell.is_some();

    let file = fixtures_dir().join("multi_language.slt");
    let result = process_file(&file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    assert!(result.html.contains("<h1>Multi-Language Page</h1>"));
    assert!(result.html.contains("<p>From Python</p>"));
    assert!(result.html.contains("<footer>End</footer>"));

    if has_ruby {
        assert!(result.html.contains("<p>From Ruby</p>"));
    }
    if has_node {
        assert!(result.html.contains("<p>From JavaScript</p>"));
    }
    if has_shell {
        assert!(result.html.contains("<p>From Shell</p>"));
    }
}

// ---------------------------------------------------------------------------
// Tests: Shared scope
// ---------------------------------------------------------------------------

#[test]
fn shared_scope_python() {
    let Some(cfg) = config() else { return };
    let file = fixtures_dir().join("shared_scope.slt");
    let result = process_file(&file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    // Python blocks share scope: x=10 in block 1 is visible in block 2.
    assert!(result.html.contains("10"));
    assert!(result.html.contains("15"));
    assert!(result.html.contains("<p>middle</p>"));
}

// ---------------------------------------------------------------------------
// Tests: Isolated scope
// ---------------------------------------------------------------------------

#[test]
fn isolated_scope_python() {
    let Some(cfg) = config() else { return };
    let file = fixtures_dir().join("isolated_scope.slt");
    let result = process_file(&file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    // First block prints 100, second block has independent scope so x is undefined.
    assert!(result.html.contains("100"));
    assert!(result.html.contains("undefined"));
}

// ---------------------------------------------------------------------------
// Tests: Directives
// ---------------------------------------------------------------------------

#[test]
fn directives_resolved() {
    let Some(cfg) = config() else { return };
    let file = fixtures_dir().join("directives.slt");
    let result = process_file(&file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    assert_eq!(result.directives.status, 201);
    assert_eq!(result.directives.content_type.as_deref(), Some("text/html"));
    assert_eq!(result.directives.headers.len(), 1);
    assert_eq!(result.directives.headers[0].0, "X-Custom");
    assert_eq!(result.directives.headers[0].1, "salata-test");
    assert!(result.html.contains("<p>directive test</p>"));
}

// ---------------------------------------------------------------------------
// Tests: #include
// ---------------------------------------------------------------------------

#[test]
fn include_resolution() {
    let Some(cfg) = config() else { return };
    let file = fixtures_dir().join("with_include.slt");
    let result = process_file(&file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    assert!(result.html.contains("<nav>Navigation Bar</nav>"));
    assert!(result.html.contains("included page content"));
}

// ---------------------------------------------------------------------------
// Tests: Cross-runtime macros (#set/#get)
// ---------------------------------------------------------------------------

#[test]
fn macros_cross_runtime_python_to_js() {
    let Some(cfg) = config() else { return };
    if cfg.runtimes.javascript.is_none() {
        eprintln!("node not found, skipping macro cross-runtime test");
        return;
    }

    let file = fixtures_dir().join("macros_cross_runtime.slt");
    let result = process_file(&file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    assert!(result.html.contains("set by python"));
    assert!(
        result.html.contains("Found 2 users: Alice"),
        "JS should read Python's macro data, got: {}",
        result.html
    );
}

// ---------------------------------------------------------------------------
// Tests: process_source (in-memory)
// ---------------------------------------------------------------------------

#[test]
fn process_source_inline() {
    let Some(cfg) = config() else { return };
    let source = "<p>start</p>\n<python>\nprint(1 + 2)\n</python>\n<p>end</p>\n";
    let file = fixtures_dir().join("inline_test.slt");

    let result = process_source(source, &file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    assert!(result.html.contains("<p>start</p>"));
    assert!(result.html.contains("3"));
    assert!(result.html.contains("<p>end</p>"));
}

// ---------------------------------------------------------------------------
// Tests: Full pipeline — multi-language + directives + macros
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_combined() {
    let Some(cfg) = config() else { return };
    let has_ruby = cfg.runtimes.ruby.is_some();

    let mut source = String::from(
        r#"#status 200
#header "X-Engine" "salata"
<html><body>
<python>
data = {"greeting": "Hello from Python"}
#set("data", data)
print(data["greeting"])
</python>
<hr>
"#,
    );

    if has_ruby {
        source.push_str(
            r#"<ruby>
d = #get("data")
puts d["greeting"].gsub("Python", "Ruby")
</ruby>
"#,
        );
    }

    source.push_str("</body></html>\n");

    let file = fixtures_dir().join("combined_test.slt");
    let result = process_source(&source, &file, &cfg, &env(), ExecutionContext::Cli).unwrap();

    assert_eq!(result.directives.status, 200);
    assert_eq!(result.directives.headers[0].0, "X-Engine");
    assert!(result.html.contains("Hello from Python"));
    if has_ruby {
        assert!(
            result.html.contains("Hello from Ruby"),
            "Ruby should have read Python macro data, got: {}",
            result.html
        );
    }
}

// ---------------------------------------------------------------------------
// Tests: Error cases
// ---------------------------------------------------------------------------

#[test]
fn runtime_not_configured_error() {
    // Config with no runtimes but source uses python.
    let cfg = SalataConfig::parse("").unwrap();
    let source = "<python>\nprint('hi')\n</python>\n";
    let file = PathBuf::from("test.slt");

    let result = process_source(source, &file, &cfg, &env(), ExecutionContext::Cli);
    assert!(result.is_err(), "should fail when runtime not configured");
}

#[test]
fn duplicate_status_error() {
    let cfg = SalataConfig::parse("").unwrap();
    let source = "#status 200\n#status 404\n<p>hi</p>\n";
    let file = PathBuf::from("test.slt");

    let result = process_source(source, &file, &cfg, &env(), ExecutionContext::Cli);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Tests: Disabled runtimes
// ---------------------------------------------------------------------------

#[test]
fn disabled_runtime_produces_clear_error() {
    let toml_str = r#"
[runtimes.python]
path = "/usr/bin/python3"
enabled = false

[runtimes.ruby]
path = "/usr/bin/ruby"
"#;
    let cfg = SalataConfig::parse(toml_str).unwrap();
    let source = "<python>print('hello')</python>\n";
    let file = PathBuf::from("test.slt");

    let result = process_source(source, &file, &cfg, &env(), ExecutionContext::Cli);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("disabled"),
        "error should mention 'disabled', got: {err}"
    );
    assert!(
        err.contains("python"),
        "error should mention 'python', got: {err}"
    );
}

#[test]
fn enabled_runtime_still_works() {
    let Some(python) = find_python() else {
        eprintln!("python3 not found, skipping");
        return;
    };
    let toml_str = format!(
        r#"
[runtimes.python]
path = "{python}"
enabled = true
"#
    );
    let cfg = SalataConfig::parse(&toml_str).unwrap();
    let source = "<python>print('enabled_ok')</python>\n";
    let file = PathBuf::from("test.slt");

    let result = process_source(source, &file, &cfg, &env(), ExecutionContext::Cli).unwrap();
    assert!(result.html.contains("enabled_ok"));
}
