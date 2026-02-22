# Binaries

Salata produces four executable binaries. Each one serves a different deployment scenario, but they all share the same core engine (`salata-core`) for parsing and executing `.slt` files.

## salata (CLI Interpreter)

The core interpreter. It reads a `.slt` file, processes all runtime blocks, and writes the final output to stdout. There is no networking, no HTTP, and no web server involved. It is a pure file-in, text-out tool.

```bash
# Process a template and write output to a file
salata index.slt > output.html

# Generate JSON output
salata api-response.slt > data.json

# Generate a config file from a template
salata nginx.conf.slt > /etc/nginx/sites-available/mysite

# Use a specific config file
salata --config /path/to/config.toml template.slt
```

The output format is determined entirely by what the runtime blocks print. Salata does not impose any format.

### `salata init`

The CLI includes a project scaffolding command:

```bash
salata init my-project
```

This creates a new directory with a starter `.slt` file, a default `config.toml`, and the standard directory structure to get started quickly.

### Execution Context

The CLI binary sets `ExecutionContext::Cli`. This means PHP blocks use the `php` CLI binary (configured via `cli_path`).

## salata-cgi (CGI Bridge)

A traditional CGI bridge designed to sit between a web server (nginx, Apache, etc.) and the Salata interpreter. The web server invokes `salata-cgi` for each request, passing request data through CGI environment variables. `salata-cgi` determines which `.slt` file to process, runs it through salata-core, and returns the result as an HTTP response.

> **Note:** The `salata-cgi` binary and all its security protections are fully built and unit-tested. However, integration with actual nginx and Apache web servers has not been tested yet. Testing and configuration documentation for real web server setups are coming. For now, use `salata-server` to serve `.slt` files over HTTP.

### Security Protections

`salata-cgi` includes built-in protections against common CGI attack vectors:

- **Slowloris defense** -- configurable timeouts for headers and body, minimum data rate enforcement
- **Request limits** -- maximum URL length, header size, header count, query string length, body size
- **Process limits** -- connections per IP, total connections, execution time, memory per request, response size
- **Path security** -- blocks path traversal attempts, dotfile access, and dangerous file extensions
- **Input validation** -- blocks null bytes, non-printable characters in headers, validates content-length

All of these are configurable through the `[cgi]` section of `config.toml`.

### Execution Context

The CGI binary sets `ExecutionContext::Cgi`. This means PHP blocks use the `php-cgi` binary (configured via `cgi_path`).

## salata-fastcgi (Stub)

The FastCGI binary is a placeholder for future development. Currently, running it produces:

```text
$ salata-fastcgi
Salata FastCGI v0.1.0 — not yet implemented
```

When implemented (planned as `salata-fpm`), it will be a persistent FastCGI daemon that listens on a Unix socket or TCP port for integration with nginx and Apache. Unlike CGI (which spawns a new process per request), FastCGI keeps a persistent process that handles multiple requests, reducing overhead.

### Execution Context

When implemented, the FastCGI binary will set `ExecutionContext::FastCgi`. PHP blocks will use `php-fpm` via socket or TCP (configured via `fastcgi_socket` or `fastcgi_host`).

## salata-server (Development Server)

A standalone HTTP server for development and lightweight production use. This is currently the only tested way to serve `.slt` files over HTTP. It depends on `salata-cgi` (and transitively on `salata-core`) and uses a Rust web framework (actix-web) for HTTP handling.

```bash
# Serve a directory on the default port
salata-server ./my-site

# Serve on a specific port
salata-server ./my-site --port 3000

# Serve a single file
salata-server index.slt --port 8080
```

### Directory Serving

When pointed at a directory, `salata-server` handles files based on their type:

- **`.slt` files** -- processed through the Salata engine, output sent as the HTTP response
- **Everything else** -- served as static files with proper MIME types (HTML, CSS, JavaScript, images, fonts, media, etc.)

### Framework Features

Because `salata-server` uses a mature Rust web framework, it inherits full HTTP server capabilities:

- Cookies, headers, and sessions
- Redirects and content negotiation
- Compression (gzip, brotli)
- TLS/HTTPS support
- Keep-alive connections
- Chunked transfer encoding
- Static file serving with correct MIME types

### Hot Reload

When `hot_reload = true` in the `[server]` section of `config.toml` (the default), the server watches for file changes and triggers a reparse. This means you can edit `.slt` files and see the results immediately without restarting the server.

### Execution Context

The server binary sets `ExecutionContext::Server`. PHP blocks use `php-fpm` via socket or TCP (configured via `fastcgi_socket` or `fastcgi_host`).

## Configuration Requirement

All four binaries require a `config.toml` file to run. If no config is found, the binary prints an error and exits. The lookup order is:

1. `--config /path/to/config.toml` flag (explicit path)
2. `config.toml` in the same directory as the binary
3. Error -- refuse to run

## Log Directory

All binaries write logs to a `logs/` directory next to the binary. This directory is created automatically on first run. Each runtime gets its own log file, and `salata-server` additionally maintains `access.log` and `error.log`.
