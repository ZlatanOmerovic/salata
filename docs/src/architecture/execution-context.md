# Execution Context

Salata is context-aware. Each of the four binaries sets an `ExecutionContext` before invoking `salata-core`, and this context flows through the entire processing pipeline. The context is defined as a Rust enum:

```rust
enum ExecutionContext {
    Cli,
    Cgi,
    FastCgi,
    Server,
}
```

## Which Binary Sets Which Context

| Binary             | Context    |
|--------------------|------------|
| `salata` (CLI)     | `Cli`      |
| `salata-cgi`       | `Cgi`      |
| `salata-fastcgi`   | `FastCgi`  |
| `salata-server`    | `Server`   |

The context is set once at startup and does not change during the lifetime of the process.

## Why Context Matters

The primary effect of the execution context is PHP binary selection. PHP has multiple Server API (SAPI) interfaces, and using the wrong one produces incorrect behavior. Salata mirrors PHP's own model:

| Context    | PHP Binary Used             | Config Field                         |
|------------|-----------------------------|--------------------------------------|
| `Cli`      | `php` (CLI SAPI)            | `cli_path`                           |
| `Cgi`      | `php-cgi` (CGI SAPI)        | `cgi_path`                           |
| `FastCgi`  | `php-fpm` via socket or TCP  | `fastcgi_socket` / `fastcgi_host`    |
| `Server`   | `php-fpm` via socket or TCP  | `fastcgi_socket` / `fastcgi_host`    |

### Why Different PHP Binaries

PHP's SAPIs differ in how they handle input, output, and HTTP semantics:

- **`php` (CLI)** -- reads from stdin, writes to stdout, has no awareness of HTTP headers. This is the correct choice when Salata is invoked from the command line and the output is written to a file or piped to another program.

- **`php-cgi`** -- implements the CGI protocol. It reads request metadata from environment variables (`REQUEST_METHOD`, `QUERY_STRING`, etc.) and can emit HTTP headers in its output. This is necessary when Salata runs behind a web server as a CGI program.

- **`php-fpm`** -- a persistent FastCGI process manager. It maintains a pool of PHP worker processes and communicates over Unix sockets or TCP. This is the efficient choice for persistent server contexts where spawning a new process per request would be wasteful.

## Context in the Pipeline

The execution context is passed as a parameter through the processing pipeline. When salata-core encounters a `<php>` block, it checks the current context to determine which binary to invoke:

```text
salata-cli sets context = Cli
  → salata-core receives context
    → parser extracts <php> block
      → runtime module checks context
        → context is Cli → use cli_path ("/usr/bin/php")
        → context is Cgi → use cgi_path ("/usr/bin/php-cgi")
        → context is FastCgi → connect to fastcgi_socket or fastcgi_host
        → context is Server → connect to fastcgi_socket or fastcgi_host
```

## Effect on Other Runtimes

For runtimes other than PHP, the execution context currently has no effect on behavior. Python, Ruby, JavaScript, TypeScript, and Shell all use the same binary regardless of context. The context is still available to these runtimes in case future features need it.

## Request Data

In CGI, FastCGI, and Server contexts, HTTP request data is made available to runtimes through standard CGI environment variables:

| Variable             | Description                          |
|----------------------|--------------------------------------|
| `REQUEST_METHOD`     | HTTP method (GET, POST, etc.)        |
| `QUERY_STRING`       | URL query parameters                 |
| `CONTENT_TYPE`       | Request body MIME type               |
| `CONTENT_LENGTH`     | Request body size in bytes           |
| `HTTP_HOST`          | Host header value                    |
| `HTTP_COOKIE`        | Cookie header value                  |
| `REMOTE_ADDR`        | Client IP address                    |
| `REQUEST_URI`        | Full request URI                     |
| `PATH_INFO`          | Extra path information               |
| `SERVER_NAME`        | Server hostname                      |
| `SERVER_PORT`        | Server port number                   |
| `HTTP_AUTHORIZATION` | Authorization header value           |

In the CLI context, these variables are not set (there is no HTTP request).
