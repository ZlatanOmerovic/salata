# Logging Configuration

The `[logging]` section in `config.toml` controls where and how Salata writes log files. Logging is always active -- errors are written to log files regardless of the `display_errors` setting.

## Configuration

```toml
[logging]
directory = "./logs"
rotation_max_size = "50MB"
rotation_max_files = 10

[logging.server]
access_log = "access.log"
error_log = "error.log"
format = "combined"

[logging.runtimes]
python = "python.log"
ruby = "ruby.log"
javascript = "javascript.log"
typescript = "typescript.log"
php = "php.log"
shell = "shell.log"
```

## General Settings

### directory

**Type:** string
**Default:** `"./logs"`

The directory where all log files are stored, relative to the binary location. This directory is **created automatically** on first run. If the directory cannot be created, Salata reports an error and exits.

```toml
[logging]
directory = "/var/log/salata"
```

### rotation_max_size

**Type:** string (size)
**Default:** `"50MB"`

Maximum size of a single log file before it is rotated. When a log file reaches this size, it is renamed (e.g., `python.log` becomes `python.log.1`) and a new log file is started.

### rotation_max_files

**Type:** integer
**Default:** `10`

Maximum number of rotated log files to keep. Older rotated files beyond this count are deleted. With the default setting of 10, you will have at most `python.log` plus `python.log.1` through `python.log.10`.

## Per-Runtime Log Files

Each runtime writes to its own log file within the logging directory. This separation makes it straightforward to diagnose issues with a specific runtime.

```toml
[logging.runtimes]
python = "python.log"
ruby = "ruby.log"
javascript = "javascript.log"
typescript = "typescript.log"
php = "php.log"
shell = "shell.log"
```

All runtime errors, warnings, and informational messages for a given language go to its dedicated log file. With the default `directory = "./logs"`, the full paths would be:

```text
logs/
  python.log
  ruby.log
  javascript.log
  typescript.log
  php.log
  shell.log
```

## Server Log Files

When running `salata-server`, two additional log files are written:

```toml
[logging.server]
access_log = "access.log"
error_log = "error.log"
format = "combined"
```

### access_log

**Type:** string
**Default:** `"access.log"`

Records every HTTP request handled by `salata-server`. Written in the format specified by the `format` field.

### error_log

**Type:** string
**Default:** `"error.log"`

Records server-level errors (startup failures, connection errors, etc.). Runtime errors from `.slt` processing go to the per-runtime log files, not this file.

### format

**Type:** string
**Default:** `"combined"`

The format for access log entries. The `"combined"` format follows the standard Apache/nginx combined log format.

## Log Entry Format

Runtime log entries follow this format:

```text
[TIMESTAMP] [LEVEL] [RUNTIME] [FILE:LINE] MESSAGE
```

Examples:

```text
[2026-02-21 14:32:05] [ERROR] [python] [index.slt:15] NameError: name 'x' is not defined
[2026-02-21 14:32:05] [INFO]  [shell]  [index.slt:42] Block executed successfully (12ms)
[2026-02-21 14:32:06] [ERROR] [ruby]   [report.slt:8] undefined method 'foo' for nil (NoMethodError)
[2026-02-21 14:32:07] [INFO]  [javascript] [app.slt:20] Block executed successfully (3ms)
```

The fields are:

- **TIMESTAMP** -- date and time in `YYYY-MM-DD HH:MM:SS` format
- **LEVEL** -- `ERROR`, `WARN`, or `INFO`
- **RUNTIME** -- which language runtime produced the log entry
- **FILE:LINE** -- the `.slt` file and line number where the block starts
- **MESSAGE** -- the error message or status information

## Relationship to display_errors

The `display_errors` setting (global or per-runtime) controls whether errors appear in the **output**. It does not affect logging. Errors are **always** written to log files regardless of the `display_errors` setting.

| `display_errors` | Output | Log file |
|-------------------|--------|----------|
| `true` | Error shown in output | Error logged |
| `false` | Error hidden from output | Error logged |

This means you can safely set `display_errors = false` in production to hide error details from users while still having full error information in the log files for debugging.
