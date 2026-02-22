# CGI Configuration

The `[cgi]` section in `config.toml` configures the security protections and resource limits for `salata-cgi` (and by extension, `salata-server` which depends on it). These settings defend against common CGI attack vectors.

## Full Configuration

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

## Slowloris Protection

These settings defend against slowloris attacks, where a client sends data extremely slowly to tie up server resources.

### header_timeout

**Type:** string (duration)
**Default:** `"5s"`

Maximum time to wait for the client to finish sending HTTP headers. If the headers are not fully received within this window, the connection is dropped.

### body_timeout

**Type:** string (duration)
**Default:** `"30s"`

Maximum time to wait for the client to finish sending the request body. Applies to POST, PUT, and PATCH requests.

### min_data_rate

**Type:** string (rate)
**Default:** `"100b/s"`

Minimum acceptable data transfer rate from the client. If the client sends data slower than this rate, the connection is terminated. This prevents slow-rate denial-of-service attacks.

## Request Limits

These settings cap the size and complexity of incoming requests.

### max_url_length

**Type:** integer
**Default:** `2048`

Maximum length of the request URL in characters. Requests with longer URLs are rejected with a 414 status.

### max_header_size

**Type:** string (size)
**Default:** `"8KB"`

Maximum total size of all HTTP headers combined. Requests exceeding this are rejected.

### max_header_count

**Type:** integer
**Default:** `50`

Maximum number of HTTP headers in a single request. Requests with more headers are rejected.

### max_query_string_length

**Type:** integer
**Default:** `2048`

Maximum length of the query string portion of the URL. Requests with longer query strings are rejected.

### max_body_size

**Type:** string (size)
**Default:** `"10MB"`

Maximum size of the request body. Requests with larger bodies are rejected with a 413 status. This protects against memory exhaustion from large uploads.

## Process Limits

These settings control resource consumption per request and across the server.

### max_connections_per_ip

**Type:** integer
**Default:** `20`

Maximum number of simultaneous connections from a single IP address. Additional connections from the same IP are rejected. This limits the impact of a single client on server resources.

### max_total_connections

**Type:** integer
**Default:** `200`

Maximum number of simultaneous connections across all clients. When this limit is reached, new connections are rejected until existing ones complete.

### max_execution_time

**Type:** string (duration)
**Default:** `"30s"`

Maximum time a single request's runtime execution can take. If the runtime blocks in a `.slt` file take longer than this, execution is terminated and a 500 error is returned.

### max_memory_per_request

**Type:** string (size)
**Default:** `"128MB"`

Maximum memory that can be consumed by the runtime processes handling a single request. If exceeded, the processes are terminated.

### max_response_size

**Type:** string (size)
**Default:** `"50MB"`

Maximum size of the generated response. If the output from runtime blocks exceeds this, the response is truncated and a 500 error is returned.

### response_timeout

**Type:** string (duration)
**Default:** `"60s"`

Maximum total time for generating and sending a response. This is a wall-clock timeout covering the entire request lifecycle.

## Path Security

These settings protect against file system access attacks.

### block_dotfiles

**Type:** bool
**Default:** `true`

When `true`, requests for files starting with a dot (e.g., `.env`, `.htaccess`, `.git/config`) are blocked with a 403 status. This prevents accidental exposure of configuration and version control files.

### block_path_traversal

**Type:** bool
**Default:** `true`

When `true`, requests containing path traversal sequences (`..`, `%2e%2e`) are blocked. This prevents attackers from accessing files outside the document root.

### blocked_extensions

**Type:** array of strings
**Default:** `[".toml", ".env", ".git", ".log"]`

File extensions that are blocked from being served. Requests for files with these extensions return a 403 status. This prevents access to configuration files, environment files, and log files.

```toml
# Add additional blocked extensions
blocked_extensions = [".toml", ".env", ".git", ".log", ".bak", ".sql"]
```

## Input Sanitization

These settings validate and sanitize incoming request data.

### block_null_bytes

**Type:** bool
**Default:** `true`

When `true`, requests containing null bytes (`\0`, `%00`) in the URL, headers, or body are rejected. Null byte injection is a common attack vector against C-based systems.

### block_non_printable_headers

**Type:** bool
**Default:** `true`

When `true`, requests with non-printable characters in HTTP headers are rejected. This prevents header injection attacks that use control characters.

### validate_content_length

**Type:** bool
**Default:** `true`

When `true`, the `Content-Length` header is validated against the actual body size. Mismatches are rejected. This prevents request smuggling attacks.

## Runtime Sandboxing

These settings control how runtime processes are managed.

### max_child_processes

**Type:** integer
**Default:** `10`

Maximum number of child runtime processes that can run simultaneously. This limits the total system resource consumption from concurrent requests.

### allow_outbound_network

**Type:** bool
**Default:** `true`

When `true`, runtime processes are allowed to make outbound network connections (HTTP requests, database connections, etc.). Set to `false` to restrict runtimes to local-only operations.

```toml
# Lock down: no outbound network from runtime code
[cgi]
allow_outbound_network = false
```
