# Server Configuration

The `[server]` section in `config.toml` controls the behavior of `salata-server`, the standalone development and lightweight production server.

## Configuration

```toml
[server]
hot_reload = true
```

### hot_reload

**Type:** bool
**Default:** `true`

When enabled, `salata-server` watches for file changes in the served directory and automatically reparses `.slt` files when they are modified. This also invalidates the parsed file cache, ensuring that the next request picks up the latest changes.

Set to `false` in production to avoid the overhead of file watching:

```toml
[server]
hot_reload = false
```

## Usage

`salata-server` serves directories or individual `.slt` files over HTTP:

```bash
# Serve a directory on port 3000
salata-server ./my-site --port 3000

# Serve a single file
salata-server index.slt --port 3000
```

When serving a directory:

- `.slt` files are processed through the Salata engine and the output is returned as the response
- All other files (HTML, CSS, JavaScript, images, fonts, media) are served as static files with correct MIME types
- Directory index files (e.g., `index.slt`) are served automatically when a directory path is requested

## Framework Capabilities

`salata-server` is built on a mature Rust web framework (actix-web) that provides:

- **Cookies and headers** -- full HTTP cookie and header handling
- **Sessions** -- server-side session management
- **Redirects** -- HTTP redirect responses
- **Compression** -- gzip/brotli response compression
- **TLS/HTTPS** -- built-in TLS support for secure connections
- **Keep-alive** -- persistent HTTP connections
- **Chunked transfer** -- streaming responses for large outputs
- **Content negotiation** -- automatic content type handling
- **Static file serving** -- efficient static file delivery with proper MIME types

## Relationship to Other Binaries

`salata-server` depends on `salata-cgi`, which in turn depends on `salata-core`. The dependency chain is:

```text
salata-server → salata-cgi → salata-core
```

This means `salata-server` includes all of the CGI security protections. The CGI configuration in `[cgi]` applies to requests processed by `salata-server` as well. See [CGI Configuration](./cgi-config.md) for those settings.

## Logging

When `salata-server` is running, it writes to the server log files configured in `[logging.server]`:

```toml
[logging.server]
access_log = "access.log"
error_log = "error.log"
format = "combined"
```

The access log records every request in the configured format. The error log captures server-level errors. Runtime errors from `.slt` file processing are written to the per-runtime log files (see [Logging Configuration](./logging-config.md)).
