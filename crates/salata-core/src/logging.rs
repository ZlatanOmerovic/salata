//! Logging with per-runtime log files and automatic rotation.
//!
//! Each runtime gets its own log file (e.g., `python.log`, `ruby.log`).
//! The server also maintains `access.log` and `error.log`. Log files are
//! rotated when they exceed `rotation_max_size`, keeping at most
//! `rotation_max_files` rotated copies.
//!
//! ## Log Format
//!
//! ```text
//! [2026-02-21 14:32:05] [ERROR] [python    ] [index.slt:15] NameError: name 'x' is not defined
//! ```

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::config::LoggingSection;
use crate::error::{SalataError, SalataResult};

// ---------------------------------------------------------------------------
// Log level
// ---------------------------------------------------------------------------

/// Log severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Pad to 5 chars so columns align (INFO + space, WARN + space, ERROR)
        write!(f, "{:<5}", self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Size parsing
// ---------------------------------------------------------------------------

/// Parse a human-readable size string (e.g. "50MB", "8KB", "1GB") into bytes.
fn parse_size(s: &str) -> SalataResult<u64> {
    let s = s.trim();
    let (num_part, unit) = split_size_unit(s)?;
    let value: f64 = num_part
        .parse()
        .map_err(|_| SalataError::ConfigValidation {
            field: "rotation_max_size".into(),
            message: format!("invalid numeric value in size: {s}"),
        })?;
    if value < 0.0 {
        return Err(SalataError::ConfigValidation {
            field: "rotation_max_size".into(),
            message: format!("size cannot be negative: {s}"),
        });
    }
    let multiplier: u64 = match unit.to_ascii_uppercase().as_str() {
        "B" => 1,
        "KB" => 1024,
        "MB" => 1024 * 1024,
        "GB" => 1024 * 1024 * 1024,
        _ => {
            return Err(SalataError::ConfigValidation {
                field: "rotation_max_size".into(),
                message: format!("unknown size unit: {unit}"),
            })
        }
    };
    Ok((value as u64) * multiplier)
}

/// Split "50MB" into ("50", "MB").
fn split_size_unit(s: &str) -> SalataResult<(&str, &str)> {
    let idx =
        s.find(|c: char| c.is_ascii_alphabetic())
            .ok_or_else(|| SalataError::ConfigValidation {
                field: "rotation_max_size".into(),
                message: format!("missing unit in size: {s}"),
            })?;
    Ok((&s[..idx], &s[idx..]))
}

// ---------------------------------------------------------------------------
// Timestamp formatting (no external deps)
// ---------------------------------------------------------------------------

fn format_timestamp() -> String {
    let now = SystemTime::now();
    let dur = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();

    // Convert epoch seconds to date/time components.
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Civil date from days since epoch (algorithm from Howard Hinnant).
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}:{seconds:02}")
}

// ---------------------------------------------------------------------------
// Log entry formatting
// ---------------------------------------------------------------------------

/// Format a log line per the spec:
/// `[2026-02-21 14:32:05] [ERROR] [python] [index.slt:15] NameError: ...`
pub fn format_log_entry(
    level: LogLevel,
    runtime: &str,
    file: &str,
    line: Option<usize>,
    message: &str,
) -> String {
    let ts = format_timestamp();
    let location = match line {
        Some(ln) => format!("{file}:{ln}"),
        None => file.to_string(),
    };
    format!("[{ts}] [{level}] [{runtime:<10}] [{location}] {message}\n")
}

/// Format a server access log line (combined format placeholder).
pub fn format_access_entry(message: &str) -> String {
    let ts = format_timestamp();
    format!("[{ts}] {message}\n")
}

/// Format a server error log line.
pub fn format_server_error_entry(message: &str) -> String {
    let ts = format_timestamp();
    format!("[{ts}] [ERROR] {message}\n")
}

// ---------------------------------------------------------------------------
// Logger
// ---------------------------------------------------------------------------

/// The main logger. Manages log directory, per-runtime files, rotation.
pub struct Logger {
    log_dir: PathBuf,
    max_size_bytes: u64,
    max_files: u32,
    /// Mutex-wrapped file handles, lazily opened. Keyed by filename (e.g. "python.log").
    files: Mutex<std::collections::HashMap<String, File>>,
}

impl Logger {
    /// Create a new logger from the logging config section.
    /// Creates the log directory if it doesn't exist.
    pub fn new(config: &LoggingSection) -> SalataResult<Self> {
        let log_dir = PathBuf::from(&config.directory);
        ensure_dir(&log_dir)?;

        let max_size_bytes = parse_size(&config.rotation_max_size)?;
        let max_files = config.rotation_max_files;

        Ok(Self {
            log_dir,
            max_size_bytes,
            max_files,
            files: Mutex::new(std::collections::HashMap::new()),
        })
    }

    /// Create a logger pointing at a specific directory (for testing or custom setups).
    pub fn with_dir(log_dir: PathBuf, max_size_bytes: u64, max_files: u32) -> SalataResult<Self> {
        ensure_dir(&log_dir)?;
        Ok(Self {
            log_dir,
            max_size_bytes,
            max_files,
            files: Mutex::new(std::collections::HashMap::new()),
        })
    }

    /// Write a runtime log entry.
    pub fn log_runtime(
        &self,
        level: LogLevel,
        runtime: &str,
        file: &str,
        line: Option<usize>,
        message: &str,
    ) -> SalataResult<()> {
        let filename = format!("{runtime}.log");
        let entry = format_log_entry(level, runtime, file, line, message);
        self.write_to(&filename, entry.as_bytes())
    }

    /// Write a server access log entry.
    pub fn log_access(&self, message: &str) -> SalataResult<()> {
        let entry = format_access_entry(message);
        self.write_to("access.log", entry.as_bytes())
    }

    /// Write a server error log entry.
    pub fn log_server_error(&self, message: &str) -> SalataResult<()> {
        let entry = format_server_error_entry(message);
        self.write_to("error.log", entry.as_bytes())
    }

    /// Core write method. Handles rotation before writing.
    fn write_to(&self, filename: &str, data: &[u8]) -> SalataResult<()> {
        let path = self.log_dir.join(filename);

        // Check if rotation is needed before writing.
        if path.exists() {
            let meta = fs::metadata(&path)?;
            if meta.len() + data.len() as u64 > self.max_size_bytes {
                // Drop the cached file handle before rotating.
                {
                    let mut files = self.files.lock().unwrap_or_else(|e| e.into_inner());
                    files.remove(filename);
                }
                self.rotate(&path, filename)?;
            }
        }

        let mut files = self.files.lock().unwrap_or_else(|e| e.into_inner());
        let file = match files.get_mut(filename) {
            Some(f) => f,
            None => {
                let f = OpenOptions::new().create(true).append(true).open(&path)?;
                files.insert(filename.to_string(), f);
                files.get_mut(filename).expect("just inserted")
            }
        };
        file.write_all(data)?;
        Ok(())
    }

    /// Rotate log files: foo.log → foo.log.1, foo.log.1 → foo.log.2, etc.
    /// Removes files beyond `max_files`.
    fn rotate(&self, path: &Path, filename: &str) -> SalataResult<()> {
        // Remove the oldest file if it would exceed max_files.
        let oldest = self.log_dir.join(format!("{filename}.{}", self.max_files));
        if oldest.exists() {
            fs::remove_file(&oldest)?;
        }

        // Shift existing rotated files: N → N+1 (from high to low to avoid overwrites).
        for i in (1..self.max_files).rev() {
            let from = self.log_dir.join(format!("{filename}.{i}"));
            let to = self.log_dir.join(format!("{filename}.{}", i + 1));
            if from.exists() {
                fs::rename(&from, &to)?;
            }
        }

        // Move current file to .1
        let rotated = self.log_dir.join(format!("{filename}.1"));
        if path.exists() {
            fs::rename(path, &rotated)?;
        }

        Ok(())
    }

    /// Return the log directory path.
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }
}

/// Create the log directory if it doesn't exist.
fn ensure_dir(dir: &Path) -> SalataResult<()> {
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(SalataError::Io)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("salata_log_test_{}", std::process::id()));
        // Use a sub-path with random-ish name to avoid collisions.
        let dir = dir.join(format!(
            "{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_size_megabytes() {
        assert_eq!(parse_size("50MB").unwrap(), 50 * 1024 * 1024);
    }

    #[test]
    fn parse_size_kilobytes() {
        assert_eq!(parse_size("8KB").unwrap(), 8 * 1024);
    }

    #[test]
    fn parse_size_gigabytes() {
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_size_bytes() {
        assert_eq!(parse_size("100B").unwrap(), 100);
    }

    #[test]
    fn parse_size_invalid_unit() {
        assert!(parse_size("50XB").is_err());
    }

    #[test]
    fn parse_size_no_unit() {
        assert!(parse_size("50").is_err());
    }

    #[test]
    fn format_log_entry_with_line() {
        let entry = format_log_entry(
            LogLevel::Error,
            "python",
            "index.slt",
            Some(15),
            "NameError: name 'x' is not defined",
        );
        assert!(entry.contains("[ERROR]"));
        assert!(entry.contains("[python"));
        assert!(entry.contains("[index.slt:15]"));
        assert!(entry.contains("NameError"));
        assert!(entry.ends_with('\n'));
    }

    #[test]
    fn format_log_entry_without_line() {
        let entry = format_log_entry(LogLevel::Info, "shell", "test.slt", None, "done");
        assert!(entry.contains("[INFO"));
        assert!(entry.contains("[shell"));
        assert!(entry.contains("[test.slt]"));
        // No "test.slt:N" pattern — just the filename with no line number.
        assert!(!entry.contains("test.slt:"));
    }

    #[test]
    fn log_level_ordering() {
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn log_level_display_alignment() {
        assert_eq!(format!("{}", LogLevel::Info), "INFO ");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN ");
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
    }

    #[test]
    fn logger_creates_directory() {
        let dir = temp_dir();
        assert!(!dir.exists());

        let logger = Logger::with_dir(dir.clone(), 1024 * 1024, 5).unwrap();
        assert!(dir.exists());
        assert_eq!(logger.log_dir(), dir);

        cleanup(&dir);
    }

    #[test]
    fn logger_writes_runtime_log() {
        let dir = temp_dir();
        let logger = Logger::with_dir(dir.clone(), 1024 * 1024, 5).unwrap();

        logger
            .log_runtime(LogLevel::Error, "python", "index.slt", Some(15), "boom")
            .unwrap();

        let content = fs::read_to_string(dir.join("python.log")).unwrap();
        assert!(content.contains("[ERROR]"));
        assert!(content.contains("[python"));
        assert!(content.contains("[index.slt:15]"));
        assert!(content.contains("boom"));

        cleanup(&dir);
    }

    #[test]
    fn logger_writes_access_log() {
        let dir = temp_dir();
        let logger = Logger::with_dir(dir.clone(), 1024 * 1024, 5).unwrap();

        logger.log_access("GET /index.slt 200").unwrap();

        let content = fs::read_to_string(dir.join("access.log")).unwrap();
        assert!(content.contains("GET /index.slt 200"));

        cleanup(&dir);
    }

    #[test]
    fn logger_writes_server_error_log() {
        let dir = temp_dir();
        let logger = Logger::with_dir(dir.clone(), 1024 * 1024, 5).unwrap();

        logger.log_server_error("bind failed").unwrap();

        let content = fs::read_to_string(dir.join("error.log")).unwrap();
        assert!(content.contains("[ERROR]"));
        assert!(content.contains("bind failed"));

        cleanup(&dir);
    }

    #[test]
    fn logger_appends_multiple_entries() {
        let dir = temp_dir();
        let logger = Logger::with_dir(dir.clone(), 1024 * 1024, 5).unwrap();

        logger
            .log_runtime(LogLevel::Info, "ruby", "a.slt", Some(1), "first")
            .unwrap();
        logger
            .log_runtime(LogLevel::Warn, "ruby", "a.slt", Some(2), "second")
            .unwrap();

        let content = fs::read_to_string(dir.join("ruby.log")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("first"));
        assert!(lines[1].contains("second"));

        cleanup(&dir);
    }

    #[test]
    fn logger_rotation_triggers() {
        let dir = temp_dir();
        // Tiny max size to force rotation quickly.
        let logger = Logger::with_dir(dir.clone(), 50, 3).unwrap();

        // Write enough to trigger rotation (each entry is well over 50 bytes).
        logger
            .log_runtime(LogLevel::Info, "python", "a.slt", Some(1), "aaaaaaaaaa")
            .unwrap();
        // First entry creates python.log. It's now > 50 bytes.
        // Next write should trigger rotation.
        logger
            .log_runtime(LogLevel::Info, "python", "b.slt", Some(2), "bbbbbbbbbb")
            .unwrap();

        // python.log.1 should exist (the rotated first entry).
        assert!(
            dir.join("python.log.1").exists(),
            "python.log.1 should exist after rotation"
        );
        // Current python.log should contain the second entry.
        let current = fs::read_to_string(dir.join("python.log")).unwrap();
        assert!(current.contains("bbbbbbbbbb"));

        // Write a third entry to trigger another rotation.
        logger
            .log_runtime(LogLevel::Info, "python", "c.slt", Some(3), "cccccccccc")
            .unwrap();

        assert!(dir.join("python.log.2").exists());
        assert!(dir.join("python.log.1").exists());

        cleanup(&dir);
    }

    #[test]
    fn logger_rotation_respects_max_files() {
        let dir = temp_dir();
        // max_files = 2, so only .1 and .2 should survive.
        let logger = Logger::with_dir(dir.clone(), 50, 2).unwrap();

        for i in 0..5 {
            logger
                .log_runtime(
                    LogLevel::Info,
                    "shell",
                    "x.slt",
                    Some(i),
                    "padding_data_to_exceed_50_bytes_limit_for_rotation",
                )
                .unwrap();
        }

        // .1 and .2 should exist, .3 should not.
        assert!(dir.join("shell.log").exists());
        assert!(dir.join("shell.log.1").exists());
        assert!(dir.join("shell.log.2").exists());
        assert!(
            !dir.join("shell.log.3").exists(),
            "shell.log.3 should not exist with max_files=2"
        );

        cleanup(&dir);
    }

    #[test]
    fn logger_from_config() {
        let dir = temp_dir();
        let config = LoggingSection {
            directory: dir.to_str().unwrap().to_string(),
            rotation_max_size: "10MB".to_string(),
            rotation_max_files: 5,
            ..Default::default()
        };

        let logger = Logger::new(&config).unwrap();
        logger
            .log_runtime(LogLevel::Info, "javascript", "app.slt", Some(1), "ok")
            .unwrap();

        assert!(dir.join("javascript.log").exists());

        cleanup(&dir);
    }

    #[test]
    fn timestamp_format_looks_correct() {
        let ts = format_timestamp();
        // Should match YYYY-MM-DD HH:MM:SS pattern.
        assert_eq!(ts.len(), 19);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], " ");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
    }
}
