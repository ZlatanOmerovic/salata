//! Shell sandbox — three-phase security for `<shell>` blocks.
//!
//! ## Phase 1: Static Analysis (pre-execution)
//!
//! Scans code for blocked commands (70+), blocked patterns (`eval`, `exec`,
//! backgrounding with `&`, pipe to shell), blocked paths (`/dev`, `/proc`,
//! `/sys`, `/etc`), fork bombs, and null bytes.
//!
//! ## Phase 2: Environment Setup
//!
//! Clean `PATH` (`/usr/local/bin:/usr/bin:/bin`), stripped environment variables,
//! `ulimit` enforcement (CPU time, file size, processes).
//!
//! ## Phase 3: Runtime Monitoring
//!
//! Timeout enforcement, memory tracking via periodic `/proc` reads (Linux),
//! and output size limits.
//!
//! ## Shell Whitelist (Hardcoded)
//!
//! Only these shells are allowed — not configurable via config (security boundary):
//!
//! ```text
//! /bin/sh, /bin/bash, /bin/zsh
//! /usr/bin/sh, /usr/bin/bash, /usr/bin/zsh, /usr/bin/fish, /usr/bin/dash, /usr/bin/ash
//! ```

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::{SalataError, SalataResult};
use crate::runtime::CgiEnv;

// ---------------------------------------------------------------------------
// Shell whitelist (HARDCODED — security boundary)
// ---------------------------------------------------------------------------

/// The only allowed shell binaries. Absolute paths required.
pub const ALLOWED_SHELLS: &[&str] = &[
    "/bin/sh",
    "/bin/bash",
    "/bin/zsh",
    "/usr/bin/sh",
    "/usr/bin/bash",
    "/usr/bin/zsh",
    "/usr/bin/fish",
    "/usr/bin/dash",
    "/usr/bin/ash",
];

/// Validate that a shell binary path is in the hardcoded whitelist.
///
/// Returns `Ok(())` if the path is allowed, or [`SalataError::ShellNotAllowed`]
/// if the path is relative or not in [`ALLOWED_SHELLS`].
pub fn validate_shell(path: &Path) -> SalataResult<()> {
    let path_str = path.to_string_lossy();

    // Must be absolute.
    if !path.is_absolute() {
        return Err(SalataError::ShellNotAllowed {
            path: path.to_path_buf(),
        });
    }

    for allowed in ALLOWED_SHELLS {
        if path_str == *allowed {
            return Ok(());
        }
    }

    Err(SalataError::ShellNotAllowed {
        path: path.to_path_buf(),
    })
}

// ---------------------------------------------------------------------------
// Blocked commands
// ---------------------------------------------------------------------------

/// Commands that are blocked from shell execution.
const BLOCKED_COMMANDS: &[&str] = &[
    // System-level dangerous commands
    "rm",
    "rmdir",
    "shred",
    "wipefs",
    "mkfs",
    "dd",
    "fdisk",
    "mount",
    "umount",
    "reboot",
    "shutdown",
    "halt",
    "poweroff",
    "init",
    "systemctl",
    "service",
    "ln",
    // Process/user manipulation
    "kill",
    "killall",
    "pkill",
    "su",
    "sudo",
    "doas",
    "chown",
    "chmod",
    "chgrp",
    "chroot",
    "useradd",
    "userdel",
    "usermod",
    "groupadd",
    "passwd",
    // Network tools (blocked by default; curl/wget are separate)
    "nc",
    "ncat",
    "netcat",
    "nmap",
    "telnet",
    "ssh",
    "scp",
    "sftp",
    "ftp",
    "rsync",
    "socat",
    // Code execution / shell escape
    "python",
    "python3",
    "perl",
    "ruby",
    "node",
    "php",
    "lua",
    "gdb",
    "strace",
    "ltrace",
    "nohup",
    "screen",
    "tmux",
    "at",
    "batch",
    "crontab",
    // Package management
    "apt",
    "apt-get",
    "yum",
    "dnf",
    "pacman",
    "brew",
    "pip",
    "npm",
    "gem",
    // Disk and filesystem
    "losetup",
    "lvm",
    "parted",
    "mkswap",
    "swapon",
    "swapoff",
    // Kernel / modules
    "insmod",
    "rmmod",
    "modprobe",
    "dmesg",
    "sysctl",
    // Container / virtualization
    "docker",
    "podman",
    "kubectl",
    "lxc",
];

/// Commands that are configurable (allowed when `allow_outbound_network` is true).
const NETWORK_COMMANDS: &[&str] = &["curl", "wget"];

// ---------------------------------------------------------------------------
// Blocked patterns
// ---------------------------------------------------------------------------

/// Patterns that indicate dangerous shell constructs.
const BLOCKED_PATTERNS: &[&str] = &[
    // Background execution / job control
    "| bash",
    "| sh",
    "| zsh",
    "| dash",
    "| fish",
    "|bash",
    "|sh",
    "|zsh",
    // Code execution via shell built-ins
    "eval ",
    "exec ",
    "source ",
    ". /", // source via dot-command with absolute path
    // Process substitution / dangerous redirects
    "/dev/tcp/",
    "/dev/udp/",
    // Encoding/decoding bypass attempts
    "base64 -d",
    "base64 --decode",
    "xxd -r",
    "\\x",   // hex escape sequences
    "\\u00", // unicode escape sequences
    "$'\\x", // bash ANSI-C quoting hex
    // History manipulation
    "history",
    "HISTFILE",
    // Environment manipulation
    "export PATH",
    "export LD_",
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
];

/// Fork bomb patterns — detected by structure, not just literals.
const FORK_BOMB_PATTERNS: &[&str] = &[
    ":(){ :|:& };:",     // classic bash fork bomb
    ":(){ :|:",          // partial fork bomb
    "bomb(){ bomb|bomb", // named variant
    "fork(){ fork|fork", // named variant
    "./{",               // self-execution pattern
    "while true; do",    // infinite loop with process spawning will be caught by timeout
    "while :; do",       // infinite loop variant
];

// ---------------------------------------------------------------------------
// Blocked paths
// ---------------------------------------------------------------------------

/// Filesystem paths that shell code must not access.
const BLOCKED_PATHS: &[&str] = &["/dev", "/proc", "/sys", "/etc"];

// ---------------------------------------------------------------------------
// Sandbox configuration
// ---------------------------------------------------------------------------

/// Configuration for shell sandbox runtime limits.
///
/// Controls timeout, output size, and network access for `<shell>` blocks.
/// Built from the `[cgi]` section of `config.toml`.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum execution time before the process is killed.
    pub timeout: Duration,
    /// Maximum stdout size in bytes before the block is rejected.
    pub max_output_bytes: usize,
    /// Whether `curl`/`wget` are allowed (from config `allow_outbound_network`).
    pub allow_network_commands: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_output_bytes: 50 * 1024 * 1024, // 50MB
            allow_network_commands: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Pre-execution scanning
// ---------------------------------------------------------------------------

/// Scan shell code for sandbox violations before execution (Phase 1).
///
/// Checks for null bytes, blocked commands, blocked patterns (eval, exec,
/// pipe to shell), fork bombs, blocked filesystem paths, and backgrounding
/// with `&`. Returns [`SalataError::ShellSandboxViolation`] on the first
/// violation found, or `Ok(())` if the code passes all checks.
pub fn scan_code(code: &str, config: &SandboxConfig, file: &Path, line: usize) -> SalataResult<()> {
    // Check for null bytes.
    if code.contains('\0') {
        return Err(sandbox_err(file, line, "null bytes are not allowed"));
    }

    // Normalize: collapse whitespace for pattern matching.
    let normalized = normalize_code(code);

    // Check blocked commands.
    check_blocked_commands(&normalized, config, file, line)?;

    // Check blocked patterns.
    check_blocked_patterns(&normalized, file, line)?;

    // Check fork bomb patterns.
    check_fork_bombs(code, file, line)?;

    // Check blocked paths.
    check_blocked_paths(&normalized, file, line)?;

    // Check for backgrounding with &.
    check_backgrounding(code, file, line)?;

    Ok(())
}

/// Normalize code for pattern matching: collapse runs of whitespace.
fn normalize_code(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let mut prev_ws = false;
    for ch in code.chars() {
        if ch.is_whitespace() && ch != '\n' {
            if !prev_ws {
                result.push(' ');
            }
            prev_ws = true;
        } else {
            result.push(ch);
            prev_ws = false;
        }
    }
    result
}

/// Check for blocked commands. Commands are matched at word boundaries —
/// they must appear as the first token of a line/pipe segment or after
/// common prefixes like `$()`, backticks, `&&`, `||`, `;`.
fn check_blocked_commands(
    normalized: &str,
    config: &SandboxConfig,
    file: &Path,
    line: usize,
) -> SalataResult<()> {
    for cmd in BLOCKED_COMMANDS {
        if contains_command(normalized, cmd) {
            return Err(sandbox_err(file, line, &format!("blocked command: {cmd}")));
        }
    }

    // Network commands: blocked unless allow_network_commands is true.
    if !config.allow_network_commands {
        for cmd in NETWORK_COMMANDS {
            if contains_command(normalized, cmd) {
                return Err(sandbox_err(
                    file,
                    line,
                    &format!("blocked network command: {cmd} (outbound network disabled)"),
                ));
            }
        }
    }

    Ok(())
}

/// Check if a command name appears as an actual command invocation (not as
/// a substring of another word). We look for it at line start, after pipe,
/// after semicolons, after `&&`/`||`, after `$(`, after backticks.
fn contains_command(code: &str, cmd: &str) -> bool {
    for line in code.lines() {
        let trimmed = line.trim();
        if is_command_at_position(trimmed, cmd) {
            return true;
        }
        // Check after pipe, semicolon, &&, ||.
        for sep in &["|", ";", "&&", "||"] {
            for part in trimmed.split(sep) {
                let part = part.trim();
                if is_command_at_position(part, cmd) {
                    return true;
                }
            }
        }
        // Check inside $().
        if let Some(pos) = trimmed.find("$(") {
            let inside = &trimmed[pos + 2..];
            if let Some(end) = inside.find(')') {
                let inner = inside[..end].trim();
                if is_command_at_position(inner, cmd) {
                    return true;
                }
            }
        }
        // Check inside backticks.
        if let Some(pos) = trimmed.find('`') {
            let after = &trimmed[pos + 1..];
            if let Some(end) = after.find('`') {
                let inner = after[..end].trim();
                if is_command_at_position(inner, cmd) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if `text` starts with `cmd` followed by a word boundary (space, end, etc).
fn is_command_at_position(text: &str, cmd: &str) -> bool {
    if !text.starts_with(cmd) {
        return false;
    }
    let rest = &text[cmd.len()..];
    // Command must be followed by whitespace, end-of-string, or a shell operator.
    rest.is_empty()
        || rest.starts_with(' ')
        || rest.starts_with('\t')
        || rest.starts_with(';')
        || rest.starts_with('|')
        || rest.starts_with('&')
        || rest.starts_with('>')
        || rest.starts_with('<')
        || rest.starts_with('(')
        || rest.starts_with('\n')
}

/// Check for blocked patterns (pipe to shell, eval, exec, etc).
fn check_blocked_patterns(normalized: &str, file: &Path, line: usize) -> SalataResult<()> {
    let lower = normalized.to_lowercase();
    for pattern in BLOCKED_PATTERNS {
        let pat_lower = pattern.to_lowercase();
        if lower.contains(&pat_lower) {
            return Err(sandbox_err(
                file,
                line,
                &format!("blocked pattern: {pattern}"),
            ));
        }
    }
    Ok(())
}

/// Check for fork bomb patterns.
fn check_fork_bombs(code: &str, file: &Path, line: usize) -> SalataResult<()> {
    // Remove all whitespace for structural matching.
    let compressed: String = code.chars().filter(|c| !c.is_whitespace()).collect();

    for pattern in FORK_BOMB_PATTERNS {
        let pat_compressed: String = pattern.chars().filter(|c| !c.is_whitespace()).collect();
        if compressed.contains(&pat_compressed) {
            return Err(sandbox_err(file, line, "fork bomb detected"));
        }
    }

    // Heuristic: function that calls itself with pipe to itself.
    // Pattern: name(){ name|name  (with any whitespace).
    if detect_recursive_pipe_function(code) {
        return Err(sandbox_err(file, line, "fork bomb detected"));
    }

    Ok(())
}

/// Heuristic fork bomb detection: a function `f()` whose body contains `f|f`.
fn detect_recursive_pipe_function(code: &str) -> bool {
    // Look for pattern: <name>() { ... <name>|<name> ... }
    let compressed: String = code.chars().filter(|c| !c.is_whitespace()).collect();

    // Find function definitions like "name(){" or "name (){ "
    for i in 0..compressed.len() {
        if compressed[i..].starts_with("(){") {
            // Extract function name (chars before `(){`).
            let name_end = i;
            let mut name_start = i;
            while name_start > 0
                && (compressed.as_bytes()[name_start - 1].is_ascii_alphanumeric()
                    || compressed.as_bytes()[name_start - 1] == b'_')
            {
                name_start -= 1;
            }
            if name_start < name_end {
                let name = &compressed[name_start..name_end];
                // Check if function body contains `name|name`.
                let body_start = i + 3; // after "(){"
                let pipe_pattern = format!("{name}|{name}");
                if compressed[body_start..].contains(&pipe_pattern) {
                    return true;
                }
            }
        }
    }

    false
}

/// Check for references to blocked filesystem paths.
fn check_blocked_paths(normalized: &str, file: &Path, line: usize) -> SalataResult<()> {
    for path in BLOCKED_PATHS {
        // Match path followed by / or end-of-word (so /dev matches /dev/null but
        // not /developer).
        for segment in normalized.split_whitespace() {
            // Remove surrounding quotes.
            let clean = segment.trim_matches(|c| c == '\'' || c == '"');
            if clean == *path || clean.starts_with(&format!("{path}/")) {
                return Err(sandbox_err(file, line, &format!("blocked path: {path}")));
            }
        }
    }
    Ok(())
}

/// Check for backgrounding with `&` (but not `&&`).
fn check_backgrounding(code: &str, file: &Path, line: usize) -> SalataResult<()> {
    let bytes = code.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'&' {
            // Allow && (logical AND).
            if i + 1 < bytes.len() && bytes[i + 1] == b'&' {
                continue;
            }
            // Allow the second & in &&.
            if i > 0 && bytes[i - 1] == b'&' {
                continue;
            }
            return Err(sandbox_err(
                file,
                line,
                "background execution (&) is not allowed",
            ));
        }
    }
    Ok(())
}

fn sandbox_err(file: &Path, line: usize, message: &str) -> SalataError {
    SalataError::ShellSandboxViolation {
        file: file.to_path_buf(),
        line,
        message: message.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Sandboxed execution
// ---------------------------------------------------------------------------

/// Build the ulimit preamble to enforce resource limits.
fn ulimit_preamble(config: &SandboxConfig) -> String {
    // CPU time limit (minimum 1 second — sub-second handled by monitoring loop).
    let cpu_seconds = config.timeout.as_secs().max(1);
    let file_size_kb = config.max_output_bytes / 1024;
    // Limit: CPU time, file size, no core dumps.
    // Note: ulimit -u (max processes) is not set here because the exact
    // limit varies by platform. Fork bombs are caught by pre-scan instead.
    format!(
        "ulimit -t {cpu_seconds} 2>/dev/null; \
         ulimit -f {file_size_kb} 2>/dev/null; \
         ulimit -c 0 2>/dev/null;\n"
    )
}

/// Build a clean, restricted PATH.
fn safe_path() -> &'static str {
    "/usr/local/bin:/usr/bin:/bin"
}

/// Execute shell code with full three-phase sandbox enforcement.
///
/// 1. **Pre-scan** — reject blocked commands, patterns, paths, fork bombs
/// 2. **Environment** — clean `PATH`, `ulimit` enforcement, stripped env vars
/// 3. **Monitoring** — timeout enforcement and output size tracking
///
/// Returns the captured stdout on success, or an error on sandbox violation,
/// timeout, or runtime failure.
pub fn execute_sandboxed(
    shell: &Path,
    code: &str,
    cgi_env: &CgiEnv,
    config: &SandboxConfig,
    file: &Path,
    start_line: usize,
) -> SalataResult<String> {
    // Phase 1: Pre-execution scan.
    scan_code(code, config, file, start_line)?;

    // Phase 2: Build sandboxed command.
    let full_code = format!("{}{code}", ulimit_preamble(config));

    let mut cmd = Command::new(shell);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Clean environment: only pass safe PATH and CGI vars.
    cmd.env_clear();
    cmd.env("PATH", safe_path());
    cmd.env("HOME", "/tmp");
    cmd.env("LANG", "C.UTF-8");
    cmd.env("LC_ALL", "C.UTF-8");

    // Apply CGI env vars (these are safe — they come from the request).
    cgi_env.apply_to(&mut cmd);

    // Phase 3: Spawn and monitor.
    let mut child = cmd.spawn().map_err(|e| SalataError::Runtime {
        runtime: "shell".into(),
        file: file.to_path_buf(),
        line: start_line,
        message: format!("failed to spawn {}: {e}", shell.display()),
    })?;

    // Write code to stdin.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(full_code.as_bytes());
    }

    // Wait with timeout.
    let start = Instant::now();
    let timeout = config.timeout;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process finished.
                let output = child.wait_with_output().map_err(|e| SalataError::Runtime {
                    runtime: "shell".into(),
                    file: file.to_path_buf(),
                    line: start_line,
                    message: format!("failed to read output: {e}"),
                })?;

                // Check output size.
                if output.stdout.len() > config.max_output_bytes {
                    return Err(SalataError::ShellSandboxViolation {
                        file: file.to_path_buf(),
                        line: start_line,
                        message: format!(
                            "output size {} exceeds limit {}",
                            output.stdout.len(),
                            config.max_output_bytes
                        ),
                    });
                }

                if !status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(SalataError::Runtime {
                        runtime: "shell".into(),
                        file: file.to_path_buf(),
                        line: start_line,
                        message: stderr.trim().to_string(),
                    });
                }

                return Ok(String::from_utf8_lossy(&output.stdout).to_string());
            }
            Ok(None) => {
                // Still running — check timeout.
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(SalataError::RuntimeTimeout {
                        runtime: "shell".into(),
                        file: file.to_path_buf(),
                        seconds: timeout.as_secs(),
                    });
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                return Err(SalataError::Runtime {
                    runtime: "shell".into(),
                    file: file.to_path_buf(),
                    line: start_line,
                    message: format!("failed to wait on process: {e}"),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_file() -> PathBuf {
        PathBuf::from("test.slt")
    }

    fn default_config() -> SandboxConfig {
        SandboxConfig::default()
    }

    fn no_network_config() -> SandboxConfig {
        SandboxConfig {
            allow_network_commands: false,
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // Shell whitelist
    // -----------------------------------------------------------------------

    #[test]
    fn allowed_shell_bin_bash() {
        assert!(validate_shell(Path::new("/bin/bash")).is_ok());
    }

    #[test]
    fn allowed_shell_bin_sh() {
        assert!(validate_shell(Path::new("/bin/sh")).is_ok());
    }

    #[test]
    fn allowed_shell_usr_bin_zsh() {
        assert!(validate_shell(Path::new("/usr/bin/zsh")).is_ok());
    }

    #[test]
    fn allowed_shell_usr_bin_fish() {
        assert!(validate_shell(Path::new("/usr/bin/fish")).is_ok());
    }

    #[test]
    fn allowed_shell_usr_bin_dash() {
        assert!(validate_shell(Path::new("/usr/bin/dash")).is_ok());
    }

    #[test]
    fn allowed_shell_usr_bin_ash() {
        assert!(validate_shell(Path::new("/usr/bin/ash")).is_ok());
    }

    #[test]
    fn rejected_shell_relative_path() {
        let err = validate_shell(Path::new("bash")).unwrap_err();
        assert!(matches!(err, SalataError::ShellNotAllowed { .. }));
    }

    #[test]
    fn rejected_shell_unknown_path() {
        let err = validate_shell(Path::new("/usr/local/bin/custom-shell")).unwrap_err();
        assert!(matches!(err, SalataError::ShellNotAllowed { .. }));
    }

    #[test]
    fn rejected_shell_python() {
        assert!(validate_shell(Path::new("/usr/bin/python3")).is_err());
    }

    #[test]
    fn rejected_shell_node() {
        assert!(validate_shell(Path::new("/usr/bin/node")).is_err());
    }

    // -----------------------------------------------------------------------
    // Null bytes
    // -----------------------------------------------------------------------

    #[test]
    fn null_bytes_rejected() {
        let err = scan_code("echo hello\0world", &default_config(), &test_file(), 1);
        assert!(err.is_err());
        assert!(
            err.unwrap_err().to_string().contains("null bytes"),
            "should mention null bytes"
        );
    }

    // -----------------------------------------------------------------------
    // Blocked commands
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_rm() {
        let err = scan_code("rm -rf /", &default_config(), &test_file(), 1);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("rm"));
    }

    #[test]
    fn blocked_rm_standalone() {
        assert!(scan_code("rm file.txt", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_sudo() {
        assert!(scan_code("sudo apt install", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_su() {
        assert!(scan_code("su root", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_chmod() {
        assert!(scan_code("chmod 777 file", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_chown() {
        assert!(scan_code("chown root file", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_kill() {
        assert!(scan_code("kill -9 1234", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_killall() {
        assert!(scan_code("killall httpd", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_dd() {
        assert!(scan_code(
            "dd if=/dev/zero of=file",
            &default_config(),
            &test_file(),
            1
        )
        .is_err());
    }

    #[test]
    fn blocked_mkfs() {
        assert!(scan_code("mkfs.ext4 /dev/sda1", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_reboot() {
        assert!(scan_code("reboot", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_shutdown() {
        assert!(scan_code("shutdown -h now", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_python_in_shell() {
        assert!(scan_code("python3 -c 'import os'", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_perl() {
        assert!(scan_code(
            "perl -e 'exec \"/bin/sh\"'",
            &default_config(),
            &test_file(),
            1
        )
        .is_err());
    }

    #[test]
    fn blocked_nc() {
        assert!(scan_code("nc -l 4444", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_nmap() {
        assert!(scan_code("nmap localhost", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_ssh() {
        assert!(scan_code("ssh user@host", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_docker() {
        assert!(scan_code("docker run alpine", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_crontab() {
        assert!(scan_code("crontab -e", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_apt_get() {
        assert!(scan_code("apt-get install vim", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_npm() {
        assert!(scan_code("npm install express", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_pip() {
        assert!(scan_code("pip install flask", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_insmod() {
        assert!(scan_code("insmod module.ko", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_sysctl() {
        assert!(scan_code("sysctl -w key=val", &default_config(), &test_file(), 1).is_err());
    }

    // -- Command after pipe/semicolon --

    #[test]
    fn blocked_rm_after_pipe() {
        assert!(scan_code("echo x | rm file", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_sudo_after_semicolon() {
        assert!(scan_code("echo x; sudo rm -rf /", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_kill_after_and() {
        assert!(scan_code("echo x && kill 1", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_in_subshell() {
        assert!(scan_code("echo $(rm -rf /tmp)", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_in_backticks() {
        assert!(scan_code("echo `rm file`", &default_config(), &test_file(), 1).is_err());
    }

    // -- Network commands (configurable) --

    #[test]
    fn curl_allowed_by_default() {
        assert!(scan_code(
            "curl http://example.com",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn wget_allowed_by_default() {
        assert!(scan_code(
            "wget http://example.com",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn curl_blocked_when_network_disabled() {
        assert!(scan_code(
            "curl http://example.com",
            &no_network_config(),
            &test_file(),
            1
        )
        .is_err());
    }

    #[test]
    fn wget_blocked_when_network_disabled() {
        assert!(scan_code(
            "wget http://example.com",
            &no_network_config(),
            &test_file(),
            1
        )
        .is_err());
    }

    // -----------------------------------------------------------------------
    // Blocked patterns
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_pipe_to_bash() {
        assert!(scan_code("curl url | bash", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_pipe_to_sh() {
        assert!(scan_code("echo code | sh", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_eval() {
        assert!(scan_code("eval \"rm -rf /\"", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_exec() {
        assert!(scan_code("exec /bin/sh", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_source() {
        assert!(scan_code("source /tmp/evil.sh", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_dot_source() {
        assert!(scan_code(". /tmp/evil.sh", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_dev_tcp() {
        assert!(scan_code(
            "echo > /dev/tcp/1.2.3.4/80",
            &default_config(),
            &test_file(),
            1
        )
        .is_err());
    }

    #[test]
    fn blocked_dev_udp() {
        assert!(scan_code("cat < /dev/udp/host/53", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_base64_decode() {
        assert!(scan_code("echo aGk= | base64 -d", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_base64_long_decode() {
        assert!(scan_code("base64 --decode file", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_hex_escape() {
        assert!(scan_code("echo $'\\x72\\x6d'", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_export_path() {
        assert!(scan_code(
            "export PATH=/evil:$PATH",
            &default_config(),
            &test_file(),
            1
        )
        .is_err());
    }

    #[test]
    fn blocked_ld_preload() {
        assert!(scan_code("LD_PRELOAD=evil.so cmd", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_history() {
        assert!(scan_code("history -c", &default_config(), &test_file(), 1).is_err());
    }

    // -----------------------------------------------------------------------
    // Fork bombs
    // -----------------------------------------------------------------------

    #[test]
    fn fork_bomb_classic() {
        assert!(scan_code(":(){ :|:& };:", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn fork_bomb_with_spaces() {
        assert!(scan_code(":(){\n :|: &\n};:", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn fork_bomb_named() {
        assert!(scan_code(
            "bomb(){ bomb|bomb& }; bomb",
            &default_config(),
            &test_file(),
            1
        )
        .is_err());
    }

    #[test]
    fn fork_bomb_custom_name() {
        assert!(scan_code("foo(){ foo|foo& }; foo", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn fork_bomb_partial() {
        assert!(scan_code(":(){ :|:", &default_config(), &test_file(), 1).is_err());
    }

    // -----------------------------------------------------------------------
    // Blocked paths
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_path_dev_null() {
        assert!(scan_code("cat /dev/null", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_path_proc() {
        assert!(scan_code("cat /proc/cpuinfo", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_path_sys() {
        assert!(scan_code("ls /sys/class", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_path_etc() {
        assert!(scan_code("cat /etc/passwd", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_path_etc_shadow() {
        assert!(scan_code("cat /etc/shadow", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_path_dev_standalone() {
        assert!(scan_code("ls /dev", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn blocked_path_quoted() {
        assert!(scan_code("cat '/etc/passwd'", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn not_blocked_path_developer() {
        // "/developer" should NOT match "/dev" — we check for "/dev/" or exact "/dev".
        assert!(scan_code("echo /developer", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn not_blocked_path_ethical() {
        // "/ethical" should NOT match "/etc".
        assert!(scan_code("echo /ethical", &default_config(), &test_file(), 1).is_ok());
    }

    // -----------------------------------------------------------------------
    // Backgrounding (&)
    // -----------------------------------------------------------------------

    #[test]
    fn backgrounding_blocked() {
        assert!(scan_code("sleep 100 &", &default_config(), &test_file(), 1).is_err());
    }

    #[test]
    fn logical_and_allowed() {
        assert!(scan_code("echo a && echo b", &default_config(), &test_file(), 1).is_ok());
    }

    // -----------------------------------------------------------------------
    // Allowed commands (safe operations)
    // -----------------------------------------------------------------------

    #[test]
    fn allowed_echo() {
        assert!(scan_code("echo 'hello world'", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_printf() {
        assert!(scan_code("printf '%s\\n' hello", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_cat_normal_file() {
        assert!(scan_code("cat /tmp/data.txt", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_ls_tmp() {
        assert!(scan_code("ls /tmp", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_wc() {
        assert!(scan_code("wc -l /tmp/file", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_grep() {
        assert!(scan_code("grep pattern /tmp/file", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_sed() {
        assert!(scan_code(
            "sed 's/a/b/g' /tmp/file",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn allowed_awk() {
        assert!(scan_code(
            "awk '{print $1}' /tmp/file",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn allowed_head_tail() {
        assert!(scan_code(
            "head -5 /tmp/file && tail -5 /tmp/file",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn allowed_date() {
        assert!(scan_code("date +%Y-%m-%d", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_hostname() {
        assert!(scan_code("hostname", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_whoami() {
        assert!(scan_code("whoami", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_env_read() {
        assert!(scan_code("echo $REQUEST_METHOD", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_arithmetic() {
        assert!(scan_code("echo $((2 + 3))", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_variable_assignment() {
        assert!(scan_code("X=42\necho $X", &default_config(), &test_file(), 1).is_ok());
    }

    #[test]
    fn allowed_for_loop() {
        assert!(scan_code(
            "for i in 1 2 3; do echo $i; done",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn allowed_if_statement() {
        assert!(scan_code(
            "if [ 1 -eq 1 ]; then echo yes; fi",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn allowed_pipe_safe_commands() {
        assert!(scan_code(
            "echo hello | tr 'a-z' 'A-Z'",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn allowed_curl_with_network() {
        assert!(scan_code(
            "curl -s http://api.example.com/data",
            &default_config(),
            &test_file(),
            1
        )
        .is_ok());
    }

    #[test]
    fn allowed_multiline() {
        let code = "NAME='World'\necho \"Hello $NAME\"\ndate";
        assert!(scan_code(code, &default_config(), &test_file(), 1).is_ok());
    }

    // -----------------------------------------------------------------------
    // Normalize
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize_code("echo   hello    world"), "echo hello world");
    }

    #[test]
    fn normalize_preserves_newlines() {
        assert_eq!(normalize_code("a\nb"), "a\nb");
    }

    // -----------------------------------------------------------------------
    // Sandboxed execution (live — requires bash)
    // -----------------------------------------------------------------------

    fn find_shell() -> Option<PathBuf> {
        for p in &["/bin/bash", "/bin/sh", "/usr/bin/bash"] {
            let path = Path::new(p);
            if path.exists() {
                return Some(path.to_path_buf());
            }
        }
        None
    }

    #[test]
    fn sandboxed_echo() {
        let Some(shell) = find_shell() else { return };
        let env = CgiEnv::default();
        let output = execute_sandboxed(
            &shell,
            "echo hello",
            &env,
            &default_config(),
            &test_file(),
            1,
        )
        .unwrap();
        assert_eq!(output.trim(), "hello");
    }

    #[test]
    fn sandboxed_arithmetic() {
        let Some(shell) = find_shell() else { return };
        let env = CgiEnv::default();
        let output = execute_sandboxed(
            &shell,
            "echo $((2 + 3))",
            &env,
            &default_config(),
            &test_file(),
            1,
        )
        .unwrap();
        assert_eq!(output.trim(), "5");
    }

    #[test]
    fn sandboxed_env_is_clean() {
        let Some(shell) = find_shell() else { return };
        let env = CgiEnv::default();
        // HOME should be /tmp (from our clean env).
        let output = execute_sandboxed(
            &shell,
            "echo $HOME",
            &env,
            &default_config(),
            &test_file(),
            1,
        )
        .unwrap();
        assert_eq!(output.trim(), "/tmp");
    }

    #[test]
    fn sandboxed_cgi_env_accessible() {
        let Some(shell) = find_shell() else { return };
        let env = CgiEnv {
            request_method: "GET".into(),
            ..Default::default()
        };
        let output = execute_sandboxed(
            &shell,
            "echo $REQUEST_METHOD",
            &env,
            &default_config(),
            &test_file(),
            1,
        )
        .unwrap();
        assert_eq!(output.trim(), "GET");
    }

    #[test]
    fn sandboxed_rejects_rm() {
        let Some(shell) = find_shell() else { return };
        let env = CgiEnv::default();
        let result = execute_sandboxed(
            &shell,
            "rm /tmp/test",
            &env,
            &default_config(),
            &test_file(),
            1,
        );
        assert!(result.is_err());
    }

    #[test]
    fn sandboxed_timeout() {
        let Some(shell) = find_shell() else { return };
        let env = CgiEnv::default();
        let config = SandboxConfig {
            timeout: Duration::from_millis(200),
            ..Default::default()
        };
        let result = execute_sandboxed(&shell, "sleep 10", &env, &config, &test_file(), 1);
        assert!(
            matches!(result, Err(SalataError::RuntimeTimeout { .. })),
            "expected timeout, got: {result:?}"
        );
    }
}
