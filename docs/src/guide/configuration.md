# Configuration Reference

Salata uses a single `config.toml` file for all configuration. This file is **mandatory** -- none of the four binaries (`salata`, `salata-cgi`, `salata-fastcgi`, `salata-server`) will start without a valid config.

## Config Lookup Order

1. `--config /path/to/config.toml` flag (explicit path)
2. `config.toml` in the same directory as the binary
3. Error -- Salata refuses to run

```bash
# Explicit config path
salata --config /etc/salata/config.toml index.slt

# Looks for config.toml next to the salata binary
salata index.slt
```

## Full Default config.toml

```toml
[salata]
display_errors = true
default_content_type = "text/html; charset=utf-8"
encoding = "utf-8"

[server]
hot_reload = true

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

[runtimes.python]
enabled = true
path = "/usr/bin/python3"
shared_scope = true
display_errors = true

[runtimes.ruby]
enabled = true
path = "/usr/bin/ruby"
shared_scope = true

[runtimes.javascript]
enabled = true
path = "/usr/bin/node"
shared_scope = true

[runtimes.typescript]
enabled = true
path = "/usr/bin/ts-node"
shared_scope = true

[runtimes.php]
enabled = true
mode = "cgi"
cli_path = "/usr/bin/php"
cgi_path = "/usr/bin/php-cgi"
shared_scope = true

[runtimes.shell]
enabled = true
path = "/bin/bash"
shared_scope = true

[cgi]
header_timeout = "5s"
body_timeout = "30s"
min_data_rate = "100b/s"
max_url_length = 2048
max_header_size = "8KB"
max_header_count = 50
max_query_string_length = 2048
max_body_size = "10MB"
max_connections_per_ip = 20
max_total_connections = 200
max_execution_time = "30s"
max_memory_per_request = "128MB"
max_response_size = "50MB"
response_timeout = "60s"
block_dotfiles = true
block_path_traversal = true
blocked_extensions = [".toml", ".env", ".git", ".log"]
block_null_bytes = true
block_non_printable_headers = true
validate_content_length = true
max_child_processes = 10
allow_outbound_network = true

[errors]
page_404 = "./errors/404.slt"
page_500 = "./errors/500.slt"
```

## Section Reference

### [salata]

Global settings that apply across all binaries and runtimes.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `display_errors` | bool | `true` | Show runtime errors in the output. When `false`, errors are logged but not displayed. Individual runtimes can override this. |
| `default_content_type` | string | `"text/html; charset=utf-8"` | Default MIME type for responses when no `#content-type` directive is used. |
| `encoding` | string | `"utf-8"` | Enforced character encoding for all input and output. |

### [server]

Settings specific to `salata-server`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `hot_reload` | bool | `true` | Watch for file changes and trigger reparse in dev mode. |

See [Server Configuration](./server-config.md) for details.

### [logging]

Log file management.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `directory` | string | `"./logs"` | Log directory, relative to the binary location. Created on first run. |
| `rotation_max_size` | string | `"50MB"` | Maximum size of a log file before rotation. |
| `rotation_max_files` | int | `10` | Maximum number of rotated log files to keep. |

See [Logging Configuration](./logging-config.md) for the full logging reference.

### [runtimes.*]

Each runtime has its own configuration section. Common fields shared by all runtimes:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable or disable this runtime. |
| `path` | string | varies | Absolute path to the runtime binary. |
| `shared_scope` | bool | `true` | All blocks of this language share one process. |
| `display_errors` | bool | (inherited) | Override the global `display_errors` setting for this runtime. |

PHP has additional fields for its context-aware binary selection:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | string | `"cgi"` | PHP execution mode: `"cgi"` or `"fastcgi"`. |
| `cli_path` | string | `"/usr/bin/php"` | Path to the PHP CLI binary (used in CLI context). |
| `cgi_path` | string | `"/usr/bin/php-cgi"` | Path to php-cgi (used in CGI context). |
| `fastcgi_socket` | string | (none) | Unix socket path for php-fpm (FastCGI/Server context). |
| `fastcgi_host` | string | (none) | TCP host:port for php-fpm (FastCGI/Server context). |

See [Runtime Configuration](./runtime-config.md) for detailed per-runtime settings.

### [cgi]

Security and resource limits for `salata-cgi`. These settings protect against common CGI attack vectors.

See [CGI Configuration](./cgi-config.md) for the full reference with descriptions of each field.

### [errors]

Custom error page templates.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_404` | string | `"./errors/404.slt"` | Path to the 404 error page. Can be a `.slt` file. |
| `page_500` | string | `"./errors/500.slt"` | Path to the 500 error page. Can be a `.slt` file. |

Error pages can be `.slt` files with runtime blocks, so you can generate dynamic error pages. Paths are relative to the binary location.
