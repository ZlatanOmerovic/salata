# Configuration — Salata

## config.toml (Mandatory)

No config = nothing runs. None of the four binaries start without a valid config.

### Lookup Order

1. `--config /path/to/config.toml` flag
2. `config.toml` next to the binary
3. Error out, refuse to run

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
# fastcgi_socket = "/run/php/php-fpm.sock"
# fastcgi_host = "127.0.0.1:9000"
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
- `display_errors` (bool, default `true`) — global error display in output
- `default_content_type` (string) — default MIME type
- `encoding` (string) — enforced encoding

### [server]
- `hot_reload` (bool, default `true`) — file watcher for dev mode

### [logging]
- `directory` (string, default `"./logs"`) — relative to binary
- `rotation_max_size` (string) — max size before rotation
- `rotation_max_files` (int) — max rotated files

### [runtimes.*] (common fields)
- `enabled` (bool, default `true`) — enable/disable this runtime
- `path` (string) — absolute path to runtime binary
- `shared_scope` (bool, default `true`) — all blocks share one process
- `display_errors` (bool, optional) — override global setting

### [runtimes.php] (additional)
- `mode` ("cgi" | "fastcgi")
- `cli_path` — path to `php` binary (used when execution context is CLI)
- `cgi_path` — path to php-cgi binary (used when execution context is CGI)
- `fastcgi_socket` — unix socket for php-fpm (used when context is FastCGI/Server)
- `fastcgi_host` — TCP host:port for php-fpm (used when context is FastCGI/Server)

### [cgi]
See specs/SECURITY.md for descriptions of each protection setting.

### [errors]
- `page_404` / `page_500` — custom error pages (can be .slt files)
