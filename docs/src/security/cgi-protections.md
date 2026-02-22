# CGI Protections

The `salata-cgi` binary includes built-in protections against common CGI attack vectors. All protections are configurable through the `[cgi]` section of `config.toml` and are enabled by default with sensible defaults.

---

## Slowloris Protection

Slowloris attacks work by opening connections and sending data extremely slowly, tying up server resources indefinitely. Salata defends against this with three settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `header_timeout` | `5s` | Maximum time allowed to receive all HTTP headers. If the client has not finished sending headers within this window, the connection is dropped. |
| `body_timeout` | `30s` | Maximum time allowed to receive the full request body. Applies after headers are received. |
| `min_data_rate` | `100b/s` | Minimum data transfer rate. If the client sends data slower than this threshold, the connection is terminated. This catches clients that technically send data but at an unusably slow rate. |

```toml
[cgi]
header_timeout = "5s"
body_timeout = "30s"
min_data_rate = "100b/s"
```

---

## Request Limits

These settings cap the size of various parts of the incoming HTTP request. Oversized requests are rejected before any processing occurs.

| Setting | Default | Description |
|---------|---------|-------------|
| `max_url_length` | `2048` | Maximum length of the request URL in characters. Prevents extremely long URLs from consuming parser resources. |
| `max_header_size` | `8KB` | Maximum total size of all HTTP headers combined. |
| `max_header_count` | `50` | Maximum number of individual HTTP headers. Prevents header flooding attacks. |
| `max_query_string_length` | `2048` | Maximum length of the query string portion of the URL. |
| `max_body_size` | `10MB` | Maximum size of the request body. Applies to POST, PUT, and PATCH requests. |

```toml
[cgi]
max_url_length = 2048
max_header_size = "8KB"
max_header_count = 50
max_query_string_length = 2048
max_body_size = "10MB"
```

---

## Process Limits

These settings control resource consumption at the process level. They prevent a single request or a single client from monopolizing server resources.

| Setting | Default | Description |
|---------|---------|-------------|
| `max_connections_per_ip` | `20` | Maximum simultaneous connections from a single IP address. Limits the impact of a single attacker. |
| `max_total_connections` | `200` | Maximum total simultaneous connections across all clients. Hard ceiling on concurrency. |
| `max_execution_time` | `30s` | Maximum time a single request is allowed to run, including all runtime block execution. Requests exceeding this are killed. |
| `max_memory_per_request` | `128MB` | Maximum memory a single request and its runtime processes may consume. |
| `max_response_size` | `50MB` | Maximum size of the generated response. Prevents runaway output from consuming disk or memory. |
| `response_timeout` | `60s` | Maximum total time from request start to response completion. A broader timeout than `max_execution_time` that includes I/O overhead. |

```toml
[cgi]
max_connections_per_ip = 20
max_total_connections = 200
max_execution_time = "30s"
max_memory_per_request = "128MB"
max_response_size = "50MB"
response_timeout = "60s"
```

---

## Path Security

These settings protect against filesystem traversal and access to sensitive files.

| Setting | Default | Description |
|---------|---------|-------------|
| `block_path_traversal` | `true` | Blocks any request URL containing `../` sequences. Prevents attackers from escaping the document root to access arbitrary files. |
| `block_dotfiles` | `true` | Blocks access to files and directories starting with a dot (e.g., `.env`, `.git`, `.htaccess`). These files often contain sensitive configuration or credentials. |
| `blocked_extensions` | `[".toml", ".env", ".git", ".log"]` | List of file extensions that cannot be served. Requests for files with these extensions are rejected with a 403 Forbidden response. |

```toml
[cgi]
block_path_traversal = true
block_dotfiles = true
blocked_extensions = [".toml", ".env", ".git", ".log"]
```

---

## Input Sanitization

These settings validate and sanitize incoming request data to prevent injection attacks.

| Setting | Default | Description |
|---------|---------|-------------|
| `block_null_bytes` | `true` | Rejects requests containing `%00` (null byte) anywhere in the URL or headers. Null byte injection is a classic attack that can cause C-based path parsers to truncate filenames. |
| `block_non_printable_headers` | `true` | Rejects requests with non-printable ASCII characters in HTTP headers. Prevents header injection and response splitting attacks. |
| `validate_content_length` | `true` | Verifies that the `Content-Length` header matches the actual body size. A mismatch indicates a malformed or malicious request. |

```toml
[cgi]
block_null_bytes = true
block_non_printable_headers = true
validate_content_length = true
```

---

## Runtime Sandboxing

These settings apply to the runtime processes spawned by CGI to execute `.slt` files.

| Setting | Default | Description |
|---------|---------|-------------|
| `max_child_processes` | `10` | Maximum number of child processes that can be spawned per request. Prevents fork bombs where a runtime block attempts to spawn an unbounded number of processes. |
| `allow_outbound_network` | `true` | Controls whether `curl` and `wget` are permitted in shell blocks. When set to `false`, these commands are added to the shell sandbox's blocked command list. Other runtimes (Python, Ruby, etc.) are not affected by this setting. |

```toml
[cgi]
max_child_processes = 10
allow_outbound_network = true
```

---

## Full Default Configuration

For reference, here is the complete `[cgi]` section with all defaults:

```toml
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
```
