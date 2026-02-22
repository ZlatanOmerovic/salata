//! Project scaffolding -- `salata init` creates a starter `.slt` project.
//!
//! Detects available language runtimes on the host system (Python, Ruby,
//! Node.js, TypeScript runners, PHP, and shell), generates a `config.toml`
//! with the correct binary paths and enabled/disabled flags, and creates
//! starter files (`index.slt`, `errors/404.slt`, `errors/500.slt`).

use std::fmt::Write as FmtWrite;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Runtime detection
// ---------------------------------------------------------------------------

/// A detected (or missing) runtime.
#[derive(Debug, Clone)]
pub struct DetectedRuntime {
    /// Runtime identifier (e.g., `"python"`, `"ruby"`, `"shell"`).
    pub name: &'static str,
    /// Absolute path to the runtime binary, or `None` if not found.
    pub path: Option<String>,
    /// Version string reported by the binary (e.g., `"Python 3.12.0"`).
    pub version: Option<String>,
}

impl DetectedRuntime {
    /// Returns `true` if this runtime was found on the system.
    pub fn found(&self) -> bool {
        self.path.is_some()
    }
}

/// Result of scanning the system for all runtimes.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Python 3 (or Python 2 as fallback) runtime.
    pub python: DetectedRuntime,
    /// Ruby runtime.
    pub ruby: DetectedRuntime,
    /// Node.js runtime (used for JavaScript blocks).
    pub node: DetectedRuntime,
    /// TypeScript runner (tsx, ts-node, or bun).
    pub typescript: DetectedRuntime,
    /// PHP CLI binary (`php`).
    pub php_cli: DetectedRuntime,
    /// PHP CGI binary (`php-cgi`).
    pub php_cgi: DetectedRuntime,
    /// Shell binary (bash, sh, zsh, fish, dash, or ash).
    pub shell: DetectedRuntime,
}

impl DetectionResult {
    /// Count how many primary runtimes (not php-cgi) were found.
    pub fn enabled_count(&self) -> usize {
        [
            &self.python,
            &self.ruby,
            &self.node,
            &self.typescript,
            &self.php_cli,
            &self.shell,
        ]
        .iter()
        .filter(|r| r.found())
        .count()
    }

    /// Total primary runtimes checked.
    pub fn total_count(&self) -> usize {
        6
    }

    /// Return the first found runtime suitable for the starter index.slt.
    pub fn first_runtime_for_starter(&self) -> Option<(&str, &str)> {
        if self.python.found() {
            return Some(("python", "print('<h1>Hello from Salata!</h1>')"));
        }
        if self.node.found() {
            return Some((
                "javascript",
                "println('<h1>Hello from Salata!</h1>');",
            ));
        }
        if self.ruby.found() {
            return Some(("ruby", "puts '<h1>Hello from Salata!</h1>'"));
        }
        if self.shell.found() {
            return Some(("shell", "echo '<h1>Hello from Salata!</h1>'"));
        }
        if self.php_cli.found() {
            return Some(("php", "echo '<h1>Hello from Salata!</h1>';"));
        }
        if self.typescript.found() {
            return Some((
                "typescript",
                "println('<h1>Hello from Salata!</h1>');",
            ));
        }
        None
    }
}

/// Try to find a binary by checking well-known paths then falling back to
/// `which` (Unix) or `where` (Windows).
fn find_binary(name: &str, candidates: &[&str]) -> Option<String> {
    for candidate in candidates {
        if Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }
    // Fall back to PATH lookup.
    let lookup_cmd = if cfg!(windows) { "where" } else { "which" };
    Command::new(lookup_cmd)
        .arg(name)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if path.is_empty() {
                    None
                } else {
                    // `which` may return multiple lines; take the first.
                    Some(
                        path.lines()
                            .next()
                            .unwrap_or(&path)
                            .to_string(),
                    )
                }
            } else {
                None
            }
        })
}

/// Get the version string from a binary (best-effort).
fn get_version(path: &str) -> Option<String> {
    Command::new(path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let out = if o.status.success() {
                String::from_utf8_lossy(&o.stdout).to_string()
            } else {
                String::from_utf8_lossy(&o.stderr).to_string()
            };
            let line = out.lines().next().unwrap_or("").trim().to_string();
            if line.is_empty() { None } else { Some(line) }
        })
}

fn detect_one(name: &'static str, bin_name: &str, candidates: &[&str]) -> DetectedRuntime {
    match find_binary(bin_name, candidates) {
        Some(path) => {
            let version = get_version(&path);
            DetectedRuntime {
                name,
                path: Some(path),
                version,
            }
        }
        None => DetectedRuntime {
            name,
            path: None,
            version: None,
        },
    }
}

/// Detect all runtimes on the current system.
pub fn detect_runtimes() -> DetectionResult {
    let python = {
        let mut r = detect_one(
            "python",
            "python3",
            &[
                "/usr/bin/python3",
                "/usr/local/bin/python3",
                "/opt/homebrew/bin/python3",
            ],
        );
        if !r.found() {
            r = detect_one(
                "python",
                "python",
                &[
                    "/usr/bin/python",
                    "/usr/local/bin/python",
                    "/opt/homebrew/bin/python",
                ],
            );
        }
        r
    };

    let ruby = detect_one(
        "ruby",
        "ruby",
        &[
            "/usr/bin/ruby",
            "/usr/local/bin/ruby",
            "/opt/homebrew/bin/ruby",
        ],
    );

    let node = detect_one(
        "javascript",
        "node",
        &[
            "/usr/bin/node",
            "/usr/local/bin/node",
            "/opt/homebrew/bin/node",
        ],
    );

    let typescript = {
        // Prefer tsx over ts-node: tsx handles stdin/tempfiles reliably,
        // while ts-node has known issues with project detection and output.
        let mut r = detect_one(
            "typescript",
            "tsx",
            &[
                "/usr/bin/tsx",
                "/usr/local/bin/tsx",
                "/opt/homebrew/bin/tsx",
            ],
        );
        if !r.found() {
            r = detect_one(
                "typescript",
                "ts-node",
                &[
                    "/usr/bin/ts-node",
                    "/usr/local/bin/ts-node",
                    "/opt/homebrew/bin/ts-node",
                ],
            );
        }
        if !r.found() {
            r = detect_one(
                "typescript",
                "bun",
                &[
                    "/usr/bin/bun",
                    "/usr/local/bin/bun",
                    "/opt/homebrew/bin/bun",
                ],
            );
        }
        r
    };

    let php_cli = detect_one(
        "php",
        "php",
        &[
            "/usr/bin/php",
            "/usr/local/bin/php",
            "/opt/homebrew/bin/php",
        ],
    );

    let php_cgi = detect_one(
        "php-cgi",
        "php-cgi",
        &[
            "/usr/bin/php-cgi",
            "/usr/local/bin/php-cgi",
            "/opt/homebrew/bin/php-cgi",
        ],
    );

    // Shell: try whitelisted paths in order.
    let shell = {
        let shell_candidates: &[&str] = &[
            "/bin/bash",
            "/usr/bin/bash",
            "/bin/sh",
            "/usr/bin/sh",
            "/bin/zsh",
            "/usr/bin/zsh",
            "/usr/bin/fish",
            "/usr/bin/dash",
            "/usr/bin/ash",
        ];
        let mut found_shell = DetectedRuntime {
            name: "shell",
            path: None,
            version: None,
        };
        for candidate in shell_candidates {
            if Path::new(candidate).exists() {
                let version = get_version(candidate);
                found_shell = DetectedRuntime {
                    name: "shell",
                    path: Some(candidate.to_string()),
                    version,
                };
                break;
            }
        }
        found_shell
    };

    DetectionResult {
        python,
        ruby,
        node,
        typescript,
        php_cli,
        php_cgi,
        shell,
    }
}

// ---------------------------------------------------------------------------
// Config generation
// ---------------------------------------------------------------------------

fn runtime_line(enabled: bool, key: &str, value: &str) -> String {
    format!("enabled = {enabled}\n{key} = \"{value}\"")
}

/// Generate a config.toml string from detection results.
pub fn generate_config(det: &DetectionResult) -> String {
    let mut out = String::new();

    out.push_str("[salata]\n");
    out.push_str("display_errors = true\n");
    out.push_str("default_content_type = \"text/html; charset=utf-8\"\n");
    out.push_str("encoding = \"utf-8\"\n");
    out.push('\n');

    out.push_str("[server]\n");
    out.push_str("hot_reload = true\n");
    out.push('\n');

    out.push_str("[logging]\n");
    out.push_str("directory = \"./logs\"\n");
    out.push_str("rotation_max_size = \"50MB\"\n");
    out.push_str("rotation_max_files = 10\n");
    out.push('\n');

    out.push_str("[logging.server]\n");
    out.push_str("access_log = \"access.log\"\n");
    out.push_str("error_log = \"error.log\"\n");
    out.push_str("format = \"combined\"\n");
    out.push('\n');

    out.push_str("[logging.runtimes]\n");
    out.push_str("python = \"python.log\"\n");
    out.push_str("ruby = \"ruby.log\"\n");
    out.push_str("javascript = \"javascript.log\"\n");
    out.push_str("typescript = \"typescript.log\"\n");
    out.push_str("php = \"php.log\"\n");
    out.push_str("shell = \"shell.log\"\n");
    out.push('\n');

    // Python
    out.push_str("[runtimes.python]\n");
    let py_path = det.python.path.as_deref().unwrap_or("/usr/bin/python3");
    let _ = writeln!(out, "{}", runtime_line(det.python.found(), "path", py_path));
    out.push_str("shared_scope = true\n");
    out.push_str("display_errors = true\n");
    out.push('\n');

    // Ruby
    out.push_str("[runtimes.ruby]\n");
    let rb_path = det.ruby.path.as_deref().unwrap_or("/usr/bin/ruby");
    let _ = writeln!(out, "{}", runtime_line(det.ruby.found(), "path", rb_path));
    out.push_str("shared_scope = true\n");
    out.push('\n');

    // JavaScript
    out.push_str("[runtimes.javascript]\n");
    let js_path = det.node.path.as_deref().unwrap_or("/usr/bin/node");
    let _ = writeln!(out, "{}", runtime_line(det.node.found(), "path", js_path));
    out.push_str("shared_scope = true\n");
    out.push('\n');

    // TypeScript
    out.push_str("[runtimes.typescript]\n");
    let ts_path = det.typescript.path.as_deref().unwrap_or("/usr/local/bin/tsx");
    let _ = writeln!(
        out,
        "{}",
        runtime_line(det.typescript.found(), "path", ts_path)
    );
    out.push_str("shared_scope = true\n");
    out.push('\n');

    // PHP
    out.push_str("[runtimes.php]\n");
    let php_enabled = det.php_cli.found();
    let _ = writeln!(out, "enabled = {php_enabled}");
    out.push_str("mode = \"cgi\"\n");
    let cli = det.php_cli.path.as_deref().unwrap_or("/usr/bin/php");
    let _ = writeln!(out, "cli_path = \"{cli}\"");
    let cgi = det.php_cgi.path.as_deref().unwrap_or("/usr/bin/php-cgi");
    let _ = writeln!(out, "cgi_path = \"{cgi}\"");
    out.push_str("# fastcgi_socket = \"/run/php/php-fpm.sock\"\n");
    out.push_str("# fastcgi_host = \"127.0.0.1:9000\"\n");
    out.push_str("shared_scope = true\n");
    out.push('\n');

    // Shell
    out.push_str("[runtimes.shell]\n");
    let sh_path = det.shell.path.as_deref().unwrap_or("/bin/bash");
    let _ = writeln!(
        out,
        "{}",
        runtime_line(det.shell.found(), "path", sh_path)
    );
    out.push_str("shared_scope = true\n");
    out.push('\n');

    // CGI
    out.push_str("[cgi]\n");
    out.push_str("header_timeout = \"5s\"\n");
    out.push_str("body_timeout = \"30s\"\n");
    out.push_str("min_data_rate = \"100b/s\"\n");
    out.push_str("max_url_length = 2048\n");
    out.push_str("max_header_size = \"8KB\"\n");
    out.push_str("max_header_count = 50\n");
    out.push_str("max_query_string_length = 2048\n");
    out.push_str("max_body_size = \"10MB\"\n");
    out.push_str("max_connections_per_ip = 20\n");
    out.push_str("max_total_connections = 200\n");
    out.push_str("max_execution_time = \"30s\"\n");
    out.push_str("max_memory_per_request = \"128MB\"\n");
    out.push_str("max_response_size = \"50MB\"\n");
    out.push_str("response_timeout = \"60s\"\n");
    out.push_str("block_dotfiles = true\n");
    out.push_str("block_path_traversal = true\n");
    out.push_str("blocked_extensions = [\".toml\", \".env\", \".git\", \".log\"]\n");
    out.push_str("block_null_bytes = true\n");
    out.push_str("block_non_printable_headers = true\n");
    out.push_str("validate_content_length = true\n");
    out.push_str("max_child_processes = 10\n");
    out.push_str("allow_outbound_network = true\n");
    out.push('\n');

    // Errors
    out.push_str("[errors]\n");
    out.push_str("page_404 = \"./errors/404.slt\"\n");
    out.push_str("page_500 = \"./errors/500.slt\"\n");

    out
}

/// Generate the starter index.slt content.
pub fn generate_index_slt(det: &DetectionResult) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n");
    out.push_str("<html>\n");
    out.push_str("<head><title>Salata</title></head>\n");
    out.push_str("<body>\n");

    if let Some((lang, code)) = det.first_runtime_for_starter() {
        let _ = writeln!(out, "<{lang}>\n{code}\n</{lang}>");
    } else {
        out.push_str("<h1>Hello from Salata!</h1>\n");
        out.push_str("<p>No runtimes detected. Edit config.toml to add runtime paths.</p>\n");
    }

    out.push_str("</body>\n");
    out.push_str("</html>\n");
    out
}

// ---------------------------------------------------------------------------
// Error page templates
// ---------------------------------------------------------------------------

/// Default HTML content for the `errors/404.slt` error page template.
pub const ERROR_404: &str = "\
<!DOCTYPE html>
<html>
<head><title>404 Not Found</title></head>
<body>
<h1>404 — Not Found</h1>
<p>The requested page could not be found.</p>
</body>
</html>
";

/// Default HTML content for the `errors/500.slt` error page template.
pub const ERROR_500: &str = "\
<!DOCTYPE html>
<html>
<head><title>500 Internal Server Error</title></head>
<body>
<h1>500 — Internal Server Error</h1>
<p>Something went wrong while processing your request.</p>
</body>
</html>
";

// ---------------------------------------------------------------------------
// Init command execution
// ---------------------------------------------------------------------------

/// Ask for confirmation on stdin. Returns true if user enters y/Y/yes.
pub fn confirm_overwrite(path: &Path) -> bool {
    eprint!(
        "{} already exists. Overwrite? [y/N] ",
        path.display()
    );
    let _ = io::stderr().flush();
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

fn print_runtime(rt: &DetectedRuntime) {
    if let Some(ref path) = rt.path {
        let ver = rt.version.as_deref().unwrap_or("unknown");
        eprintln!("  {:<14} {path}  ({ver})", rt.name);
    } else {
        eprintln!("  {:<14} not found — will be disabled", rt.name);
    }
}

/// Run the init command. Returns an exit code.
pub fn run_init(target_dir: &Path, interactive: bool) -> i32 {
    // Ensure the target directory exists.
    if let Err(e) = std::fs::create_dir_all(target_dir) {
        eprintln!("salata init: cannot create directory {}: {e}", target_dir.display());
        return 1;
    }

    let config_path = target_dir.join("config.toml");

    // Check for existing config.toml.
    if config_path.exists() && interactive && !confirm_overwrite(&config_path) {
        eprintln!("Aborted.");
        return 1;
    }

    // Detect runtimes.
    eprintln!("Detecting runtimes...");
    let det = detect_runtimes();
    print_runtime(&det.python);
    print_runtime(&det.ruby);
    print_runtime(&det.node);
    print_runtime(&det.typescript);
    print_runtime(&det.php_cli);
    print_runtime(&det.php_cgi);
    print_runtime(&det.shell);

    // Write config.toml.
    let config_content = generate_config(&det);
    if let Err(e) = std::fs::write(&config_path, &config_content) {
        eprintln!("salata init: cannot write {}: {e}", config_path.display());
        return 1;
    }

    // Write index.slt.
    let index_path = target_dir.join("index.slt");
    let index_content = generate_index_slt(&det);
    if let Err(e) = std::fs::write(&index_path, &index_content) {
        eprintln!("salata init: cannot write {}: {e}", index_path.display());
        return 1;
    }

    // Create errors/ directory with templates.
    let errors_dir = target_dir.join("errors");
    if let Err(e) = std::fs::create_dir_all(&errors_dir) {
        eprintln!(
            "salata init: cannot create directory {}: {e}",
            errors_dir.display()
        );
        return 1;
    }
    if let Err(e) = std::fs::write(errors_dir.join("404.slt"), ERROR_404) {
        eprintln!("salata init: cannot write errors/404.slt: {e}");
        return 1;
    }
    if let Err(e) = std::fs::write(errors_dir.join("500.slt"), ERROR_500) {
        eprintln!("salata init: cannot write errors/500.slt: {e}");
        return 1;
    }

    // Print summary.
    let found = det.enabled_count();
    let total = det.total_count();
    println!("Created config.toml with {found} of {total} runtimes enabled.");
    println!("Run: salata index.slt");

    0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "salata_init_test_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    // -- Runtime detection tests --

    #[test]
    fn detect_runtimes_returns_results() {
        let det = detect_runtimes();
        // At minimum, on any CI or dev box, we expect the struct is populated.
        // We can't assert specific runtimes exist, but we can check structure.
        assert_eq!(det.total_count(), 6);
        assert!(det.enabled_count() <= det.total_count());
    }

    #[test]
    fn detect_finds_at_least_one_runtime() {
        // On any reasonable system (macOS/Linux), at least bash or sh exists.
        let det = detect_runtimes();
        assert!(
            det.enabled_count() > 0,
            "expected at least one runtime on this system"
        );
    }

    #[test]
    fn detected_runtime_found_flag() {
        let found = DetectedRuntime {
            name: "test",
            path: Some("/usr/bin/test".into()),
            version: Some("1.0".into()),
        };
        assert!(found.found());

        let missing = DetectedRuntime {
            name: "test",
            path: None,
            version: None,
        };
        assert!(!missing.found());
    }

    #[test]
    fn detection_result_counts() {
        // Construct a result with 3 found runtimes.
        let det = DetectionResult {
            python: DetectedRuntime {
                name: "python",
                path: Some("/usr/bin/python3".into()),
                version: None,
            },
            ruby: DetectedRuntime {
                name: "ruby",
                path: None,
                version: None,
            },
            node: DetectedRuntime {
                name: "javascript",
                path: Some("/usr/bin/node".into()),
                version: None,
            },
            typescript: DetectedRuntime {
                name: "typescript",
                path: None,
                version: None,
            },
            php_cli: DetectedRuntime {
                name: "php",
                path: None,
                version: None,
            },
            php_cgi: DetectedRuntime {
                name: "php-cgi",
                path: None,
                version: None,
            },
            shell: DetectedRuntime {
                name: "shell",
                path: Some("/bin/bash".into()),
                version: None,
            },
        };
        assert_eq!(det.enabled_count(), 3);
        assert_eq!(det.total_count(), 6);
    }

    #[test]
    fn first_runtime_prefers_python() {
        let det = DetectionResult {
            python: DetectedRuntime {
                name: "python",
                path: Some("/usr/bin/python3".into()),
                version: None,
            },
            ruby: DetectedRuntime {
                name: "ruby",
                path: Some("/usr/bin/ruby".into()),
                version: None,
            },
            node: DetectedRuntime {
                name: "javascript",
                path: Some("/usr/bin/node".into()),
                version: None,
            },
            typescript: DetectedRuntime {
                name: "typescript",
                path: None,
                version: None,
            },
            php_cli: DetectedRuntime {
                name: "php",
                path: None,
                version: None,
            },
            php_cgi: DetectedRuntime {
                name: "php-cgi",
                path: None,
                version: None,
            },
            shell: DetectedRuntime {
                name: "shell",
                path: None,
                version: None,
            },
        };
        let (lang, _) = det.first_runtime_for_starter().unwrap();
        assert_eq!(lang, "python");
    }

    #[test]
    fn first_runtime_falls_back_to_node() {
        let det = DetectionResult {
            python: DetectedRuntime {
                name: "python",
                path: None,
                version: None,
            },
            ruby: DetectedRuntime {
                name: "ruby",
                path: None,
                version: None,
            },
            node: DetectedRuntime {
                name: "javascript",
                path: Some("/usr/bin/node".into()),
                version: None,
            },
            typescript: DetectedRuntime {
                name: "typescript",
                path: None,
                version: None,
            },
            php_cli: DetectedRuntime {
                name: "php",
                path: None,
                version: None,
            },
            php_cgi: DetectedRuntime {
                name: "php-cgi",
                path: None,
                version: None,
            },
            shell: DetectedRuntime {
                name: "shell",
                path: None,
                version: None,
            },
        };
        let (lang, _) = det.first_runtime_for_starter().unwrap();
        assert_eq!(lang, "javascript");
    }

    #[test]
    fn first_runtime_none_when_all_missing() {
        let det = DetectionResult {
            python: DetectedRuntime { name: "python", path: None, version: None },
            ruby: DetectedRuntime { name: "ruby", path: None, version: None },
            node: DetectedRuntime { name: "javascript", path: None, version: None },
            typescript: DetectedRuntime { name: "typescript", path: None, version: None },
            php_cli: DetectedRuntime { name: "php", path: None, version: None },
            php_cgi: DetectedRuntime { name: "php-cgi", path: None, version: None },
            shell: DetectedRuntime { name: "shell", path: None, version: None },
        };
        assert!(det.first_runtime_for_starter().is_none());
    }

    // -- Config generation tests --

    #[test]
    fn generate_config_contains_all_sections() {
        let det = DetectionResult {
            python: DetectedRuntime {
                name: "python",
                path: Some("/usr/bin/python3".into()),
                version: None,
            },
            ruby: DetectedRuntime {
                name: "ruby",
                path: None,
                version: None,
            },
            node: DetectedRuntime {
                name: "javascript",
                path: Some("/usr/local/bin/node".into()),
                version: None,
            },
            typescript: DetectedRuntime {
                name: "typescript",
                path: None,
                version: None,
            },
            php_cli: DetectedRuntime {
                name: "php",
                path: None,
                version: None,
            },
            php_cgi: DetectedRuntime {
                name: "php-cgi",
                path: None,
                version: None,
            },
            shell: DetectedRuntime {
                name: "shell",
                path: Some("/bin/bash".into()),
                version: None,
            },
        };

        let config = generate_config(&det);

        // Sections present.
        assert!(config.contains("[salata]"));
        assert!(config.contains("[server]"));
        assert!(config.contains("[logging]"));
        assert!(config.contains("[runtimes.python]"));
        assert!(config.contains("[runtimes.ruby]"));
        assert!(config.contains("[runtimes.javascript]"));
        assert!(config.contains("[runtimes.typescript]"));
        assert!(config.contains("[runtimes.php]"));
        assert!(config.contains("[runtimes.shell]"));
        assert!(config.contains("[cgi]"));
        assert!(config.contains("[errors]"));
    }

    #[test]
    fn generate_config_enabled_for_found_runtimes() {
        let det = DetectionResult {
            python: DetectedRuntime {
                name: "python",
                path: Some("/usr/bin/python3".into()),
                version: None,
            },
            ruby: DetectedRuntime { name: "ruby", path: None, version: None },
            node: DetectedRuntime { name: "javascript", path: None, version: None },
            typescript: DetectedRuntime { name: "typescript", path: None, version: None },
            php_cli: DetectedRuntime { name: "php", path: None, version: None },
            php_cgi: DetectedRuntime { name: "php-cgi", path: None, version: None },
            shell: DetectedRuntime { name: "shell", path: None, version: None },
        };

        let config = generate_config(&det);

        // Extract lines around [runtimes.python] — should have enabled = true.
        let python_section = extract_section(&config, "[runtimes.python]");
        assert!(
            python_section.contains("enabled = true"),
            "python should be enabled, section: {python_section}"
        );
        assert!(python_section.contains("path = \"/usr/bin/python3\""));

        // Ruby should be disabled.
        let ruby_section = extract_section(&config, "[runtimes.ruby]");
        assert!(
            ruby_section.contains("enabled = false"),
            "ruby should be disabled, section: {ruby_section}"
        );
    }

    #[test]
    fn generate_config_uses_detected_paths() {
        let det = DetectionResult {
            python: DetectedRuntime { name: "python", path: None, version: None },
            ruby: DetectedRuntime { name: "ruby", path: None, version: None },
            node: DetectedRuntime {
                name: "javascript",
                path: Some("/opt/homebrew/bin/node".into()),
                version: None,
            },
            typescript: DetectedRuntime { name: "typescript", path: None, version: None },
            php_cli: DetectedRuntime {
                name: "php",
                path: Some("/opt/homebrew/bin/php".into()),
                version: None,
            },
            php_cgi: DetectedRuntime {
                name: "php-cgi",
                path: Some("/opt/homebrew/bin/php-cgi".into()),
                version: None,
            },
            shell: DetectedRuntime { name: "shell", path: None, version: None },
        };

        let config = generate_config(&det);
        assert!(config.contains("path = \"/opt/homebrew/bin/node\""));
        assert!(config.contains("cli_path = \"/opt/homebrew/bin/php\""));
        assert!(config.contains("cgi_path = \"/opt/homebrew/bin/php-cgi\""));
    }

    #[test]
    fn generate_config_is_valid_toml() {
        let det = detect_runtimes();
        let config = generate_config(&det);
        // Should parse without error.
        let parsed: Result<toml::Value, _> = toml::from_str(&config);
        assert!(
            parsed.is_ok(),
            "generated config is not valid TOML: {:?}\n---\n{config}",
            parsed.err()
        );
    }

    // -- Index.slt generation tests --

    #[test]
    fn generate_index_slt_with_python() {
        let det = DetectionResult {
            python: DetectedRuntime {
                name: "python",
                path: Some("/usr/bin/python3".into()),
                version: None,
            },
            ruby: DetectedRuntime { name: "ruby", path: None, version: None },
            node: DetectedRuntime { name: "javascript", path: None, version: None },
            typescript: DetectedRuntime { name: "typescript", path: None, version: None },
            php_cli: DetectedRuntime { name: "php", path: None, version: None },
            php_cgi: DetectedRuntime { name: "php-cgi", path: None, version: None },
            shell: DetectedRuntime { name: "shell", path: None, version: None },
        };

        let index = generate_index_slt(&det);
        assert!(index.contains("<python>"));
        assert!(index.contains("</python>"));
        assert!(index.contains("Hello from Salata"));
    }

    #[test]
    fn generate_index_slt_no_runtimes() {
        let det = DetectionResult {
            python: DetectedRuntime { name: "python", path: None, version: None },
            ruby: DetectedRuntime { name: "ruby", path: None, version: None },
            node: DetectedRuntime { name: "javascript", path: None, version: None },
            typescript: DetectedRuntime { name: "typescript", path: None, version: None },
            php_cli: DetectedRuntime { name: "php", path: None, version: None },
            php_cgi: DetectedRuntime { name: "php-cgi", path: None, version: None },
            shell: DetectedRuntime { name: "shell", path: None, version: None },
        };

        let index = generate_index_slt(&det);
        assert!(index.contains("Hello from Salata"));
        assert!(index.contains("No runtimes detected"));
    }

    // -- No overwrite without confirmation tests --

    #[test]
    fn run_init_creates_all_files() {
        let dir = temp_dir("creates_all");

        let code = run_init(&dir, false);

        assert_eq!(code, 0);
        assert!(dir.join("config.toml").exists());
        assert!(dir.join("index.slt").exists());
        assert!(dir.join("errors/404.slt").exists());
        assert!(dir.join("errors/500.slt").exists());

        // Verify config.toml is valid TOML.
        let config = fs::read_to_string(dir.join("config.toml")).unwrap();
        let parsed: Result<toml::Value, _> = toml::from_str(&config);
        assert!(parsed.is_ok(), "config.toml is not valid TOML: {:?}", parsed.err());

        // Verify index.slt has content.
        let index = fs::read_to_string(dir.join("index.slt")).unwrap();
        assert!(index.contains("Hello from Salata"));

        // Verify error pages.
        let e404 = fs::read_to_string(dir.join("errors/404.slt")).unwrap();
        assert!(e404.contains("404"));
        let e500 = fs::read_to_string(dir.join("errors/500.slt")).unwrap();
        assert!(e500.contains("500"));

        cleanup(&dir);
    }

    #[test]
    fn run_init_non_interactive_overwrites() {
        let dir = temp_dir("overwrite_nonint");

        // Create an existing config.toml.
        fs::write(dir.join("config.toml"), "# old config").unwrap();

        // Non-interactive mode should overwrite without asking.
        let code = run_init(&dir, false);
        assert_eq!(code, 0);

        let config = fs::read_to_string(dir.join("config.toml")).unwrap();
        assert!(
            config.contains("[salata]"),
            "should have been overwritten with new config"
        );

        cleanup(&dir);
    }

    #[test]
    fn config_exists_blocks_interactive_without_confirmation() {
        // We can't easily simulate stdin in a unit test, but we can verify
        // that confirm_overwrite returns false for empty input.
        // The actual interactive test is done via CLI integration test.

        // Test the confirm logic indirectly: if config exists and interactive
        // is true, run_init would call confirm_overwrite which reads stdin.
        // In a test environment with no stdin, it should return false / abort.
        // We test this through the CLI integration test instead.

        // Here we just verify the function signature works.
        let dir = temp_dir("confirm_check");
        fs::write(dir.join("config.toml"), "# existing").unwrap();

        // confirm_overwrite with a path that exists — we can't feed stdin,
        // so just verify it returns false (EOF on stdin).
        let result = confirm_overwrite(&dir.join("config.toml"));
        assert!(!result, "should return false when stdin is empty/EOF");

        cleanup(&dir);
    }

    // Helper: extract lines from a section header until the next section.
    fn extract_section(config: &str, header: &str) -> String {
        let mut in_section = false;
        let mut lines = Vec::new();
        for line in config.lines() {
            if line == header {
                in_section = true;
                continue;
            }
            if in_section {
                if line.starts_with('[') {
                    break;
                }
                lines.push(line);
            }
        }
        lines.join("\n")
    }
}
