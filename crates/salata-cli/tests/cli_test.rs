//! Integration tests for the `salata` CLI binary.
//!
//! These tests invoke the compiled binary and verify output, exit codes,
//! and error handling.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn salata_bin() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_salata"));
    assert!(
        path.exists(),
        "salata binary not found at {}",
        path.display()
    );
    path
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "salata_cli_test_{name}_{}_{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn write_file(&self, name: &str, content: &str) -> PathBuf {
        let file_path = self.path.join(name);
        fs::write(&file_path, content).unwrap();
        file_path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

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
    Command::new("which")
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

fn find_bash() -> Option<String> {
    for p in &["/bin/bash", "/usr/bin/bash"] {
        if Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    None
}

fn make_config(python: Option<&str>, bash: Option<&str>) -> String {
    let mut toml = String::from("[salata]\ndisplay_errors = true\n\n");

    if let Some(py) = python {
        toml.push_str(&format!(
            "[runtimes.python]\npath = \"{py}\"\nshared_scope = true\n\n"
        ));
    }
    if let Some(sh) = bash {
        toml.push_str(&format!(
            "[runtimes.shell]\npath = \"{sh}\"\nshared_scope = true\n\n"
        ));
    }

    toml
}

fn run_salata(args: &[&str]) -> std::process::Output {
    Command::new(salata_bin())
        .args(args)
        .output()
        .expect("failed to execute salata binary")
}

// ---------------------------------------------------------------------------
// Tests: CLI flags
// ---------------------------------------------------------------------------

#[test]
fn cli_version() {
    let output = run_salata(&["--version"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("salata v"), "got: {stdout}");
}

#[test]
fn cli_version_short() {
    let output = run_salata(&["-V"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("salata v"));
}

#[test]
fn cli_help() {
    let output = run_salata(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("USAGE:"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("<file.slt>"));
}

#[test]
fn cli_help_short() {
    let output = run_salata(&["-h"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("USAGE:"));
}

#[test]
fn cli_no_args_fails() {
    let output = run_salata(&[]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no input file"), "got: {stderr}");
}

#[test]
fn cli_missing_file_fails() {
    let output = run_salata(&["--config", "/dev/null", "/nonexistent/file.slt"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("file not found"), "got: {stderr}");
}

#[test]
fn cli_unknown_option_fails() {
    let output = run_salata(&["--bogus"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown option"), "got: {stderr}");
}

#[test]
fn cli_config_missing_value_fails() {
    let output = run_salata(&["--config"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--config requires"), "got: {stderr}");
}

// ---------------------------------------------------------------------------
// Tests: HTML-only processing (no runtimes)
// ---------------------------------------------------------------------------

#[test]
fn cli_html_only() {
    let tmp = TempDir::new("html_only");
    let config_path = tmp.write_file("config.toml", "[salata]\n");
    let slt_path = tmp.write_file("test.slt", "<h1>Hello World</h1>\n<p>Static content</p>\n");

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<h1>Hello World</h1>"));
    assert!(stdout.contains("<p>Static content</p>"));
}

// ---------------------------------------------------------------------------
// Tests: Python execution
// ---------------------------------------------------------------------------

#[test]
fn cli_python_execution() {
    let Some(python) = find_python() else {
        eprintln!("python3 not found, skipping");
        return;
    };

    let tmp = TempDir::new("python_exec");
    let config_path = tmp.write_file("config.toml", &make_config(Some(&python), None));
    let slt_path = tmp.write_file(
        "test.slt",
        "<p>before</p>\n<python>\nprint('Hello from Python')\n</python>\n<p>after</p>\n",
    );

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<p>before</p>"));
    assert!(stdout.contains("Hello from Python"));
    assert!(stdout.contains("<p>after</p>"));
}

#[test]
fn cli_python_shared_scope() {
    let Some(python) = find_python() else { return };

    let tmp = TempDir::new("python_shared");
    let config_path = tmp.write_file("config.toml", &make_config(Some(&python), None));
    let slt_path = tmp.write_file(
        "test.slt",
        "<python>\nx = 42\nprint(x)\n</python>\n<p>mid</p>\n<python>\nprint(x + 8)\n</python>\n",
    );

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("42"));
    assert!(stdout.contains("50"));
    assert!(stdout.contains("<p>mid</p>"));
}

// ---------------------------------------------------------------------------
// Tests: display_errors
// ---------------------------------------------------------------------------

#[test]
fn cli_display_errors_enabled() {
    let Some(python) = find_python() else { return };

    let tmp = TempDir::new("display_errors_on");
    let config_path = tmp.write_file(
        "config.toml",
        &format!(
            "[salata]\ndisplay_errors = true\n\n[runtimes.python]\npath = \"{python}\"\ndisplay_errors = true\n"
        ),
    );
    let slt_path = tmp.write_file(
        "test.slt",
        "<p>before</p>\n<python>\nraise ValueError('test error')\n</python>\n<p>after</p>\n",
    );

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    // Should exit with non-zero (runtime error).
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Error should appear in HTML output.
    assert!(
        stdout.contains("Salata Error")
            || stdout.contains("ValueError")
            || stdout.contains("test error"),
        "display_errors=true should show error in HTML, got: {stdout}"
    );
    assert!(stdout.contains("<p>before</p>"));
    assert!(stdout.contains("<p>after</p>"));
}

#[test]
fn cli_display_errors_disabled() {
    let Some(python) = find_python() else { return };

    let tmp = TempDir::new("display_errors_off");
    let config_path = tmp.write_file(
        "config.toml",
        &format!("[salata]\ndisplay_errors = false\n\n[runtimes.python]\npath = \"{python}\"\n"),
    );
    let slt_path = tmp.write_file(
        "test.slt",
        "<p>before</p>\n<python>\nraise ValueError('test error')\n</python>\n<p>after</p>\n",
    );

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    // Should exit with non-zero (runtime error).
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Error should NOT appear in HTML output.
    assert!(
        !stdout.contains("ValueError"),
        "display_errors=false should hide error, got: {stdout}"
    );
    assert!(
        !stdout.contains("test error"),
        "display_errors=false should hide error, got: {stdout}"
    );
    assert!(stdout.contains("<p>before</p>"));
    assert!(stdout.contains("<p>after</p>"));
}

#[test]
fn cli_display_errors_per_runtime_override() {
    let Some(python) = find_python() else { return };

    let tmp = TempDir::new("display_errors_override");
    // Global display_errors=false, but python overrides to true.
    let config_path = tmp.write_file(
        "config.toml",
        &format!(
            "[salata]\ndisplay_errors = false\n\n[runtimes.python]\npath = \"{python}\"\ndisplay_errors = true\n"
        ),
    );
    let slt_path = tmp.write_file(
        "test.slt",
        "<python>\nraise ValueError('visible error')\n</python>\n",
    );

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Python's display_errors=true overrides global false.
    assert!(
        stdout.contains("visible error"),
        "per-runtime override should show error, got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Tests: Exit codes
// ---------------------------------------------------------------------------

#[test]
fn cli_exit_zero_on_success() {
    let tmp = TempDir::new("exit_zero");
    let config_path = tmp.write_file("config.toml", "[salata]\n");
    let slt_path = tmp.write_file("test.slt", "<p>ok</p>\n");

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    assert!(output.status.success());
}

#[test]
fn cli_exit_nonzero_on_runtime_error() {
    let Some(python) = find_python() else { return };

    let tmp = TempDir::new("exit_nonzero");
    let config_path = tmp.write_file("config.toml", &make_config(Some(&python), None));
    let slt_path = tmp.write_file(
        "test.slt",
        "<python>\nraise RuntimeError('boom')\n</python>\n",
    );

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    assert!(
        !output.status.success(),
        "should exit non-zero on runtime error"
    );
}

#[test]
fn cli_exit_nonzero_on_missing_config() {
    let tmp = TempDir::new("no_config");
    let slt_path = tmp.write_file("test.slt", "<p>ok</p>\n");

    let output = run_salata(&[
        "--config",
        "/nonexistent/config.toml",
        slt_path.to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("config"), "got: {stderr}");
}

// ---------------------------------------------------------------------------
// Tests: Shell execution (through sandbox)
// ---------------------------------------------------------------------------

#[test]
fn cli_shell_execution() {
    let Some(bash) = find_bash() else { return };

    let tmp = TempDir::new("shell_exec");
    let config_path = tmp.write_file("config.toml", &make_config(None, Some(&bash)));
    let slt_path = tmp.write_file("test.slt", "<shell>\necho 'hello from shell'\n</shell>\n");

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello from shell"));
}

// ---------------------------------------------------------------------------
// Tests: Directives in CLI output
// ---------------------------------------------------------------------------

#[test]
fn cli_directives_do_not_appear_in_html() {
    let tmp = TempDir::new("directives");
    let config_path = tmp.write_file("config.toml", "[salata]\n");
    let slt_path = tmp.write_file(
        "test.slt",
        "#status 201\n#header \"X-Test\" \"value\"\n<p>content</p>\n",
    );

    let output = run_salata(&[
        "--config",
        config_path.to_str().unwrap(),
        slt_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Directive lines should be consumed by the parser, not in output.
    assert!(!stdout.contains("#status"));
    assert!(!stdout.contains("#header"));
    assert!(stdout.contains("<p>content</p>"));
}

// ---------------------------------------------------------------------------
// Tests: salata init
// ---------------------------------------------------------------------------

#[test]
fn cli_init_creates_project() {
    let tmp = TempDir::new("init_creates");

    let output = run_salata(&["init", "--path", tmp.path.to_str().unwrap()]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created config.toml"), "got: {stdout}");
    assert!(stdout.contains("runtimes enabled"), "got: {stdout}");
    assert!(stdout.contains("Run: salata index.slt"), "got: {stdout}");

    // Verify files exist.
    assert!(tmp.path.join("config.toml").exists());
    assert!(tmp.path.join("index.slt").exists());
    assert!(tmp.path.join("errors/404.slt").exists());
    assert!(tmp.path.join("errors/500.slt").exists());

    // Verify config.toml contains expected sections.
    let config = fs::read_to_string(tmp.path.join("config.toml")).unwrap();
    assert!(config.contains("[salata]"));
    assert!(config.contains("[runtimes.python]"));
    assert!(config.contains("[cgi]"));
    assert!(config.contains("[errors]"));
}

#[test]
fn cli_init_no_overwrite_without_confirmation() {
    let tmp = TempDir::new("init_no_overwrite");

    // Write an existing config.toml.
    let original = "# my existing config\n";
    fs::write(tmp.path.join("config.toml"), original).unwrap();

    // Run init in non-interactive mode (piped stdin = EOF → no confirmation).
    let output = Command::new(salata_bin())
        .args(["init", "--path", tmp.path.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    // Should have aborted.
    assert!(
        !output.status.success(),
        "should fail without confirmation, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Original config should be preserved.
    let config = fs::read_to_string(tmp.path.join("config.toml")).unwrap();
    assert_eq!(config, original, "config.toml should not be overwritten");
}

#[test]
fn cli_init_help_mentions_init() {
    let output = run_salata(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("init"), "help should mention init command");
}

// ---------------------------------------------------------------------------
// Tests: config.toml with --config= (equals syntax)
// ---------------------------------------------------------------------------

#[test]
fn cli_config_equals_syntax() {
    let tmp = TempDir::new("config_eq");
    let config_path = tmp.write_file("config.toml", "[salata]\n");
    let slt_path = tmp.write_file("test.slt", "<p>ok</p>\n");

    let output = run_salata(&[
        &format!("--config={}", config_path.to_str().unwrap()),
        slt_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
