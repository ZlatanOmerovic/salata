# Architecture — Salata

## Components

```
/usr/local/bin/
  ├── salata              # Core interpreter
  ├── salata-cgi          # Traditional CGI bridge with security protections
  ├── salata-fastcgi      # FastCGI daemon (stub/hello-world for now)
  ├── salata-server       # Standalone dev/lightweight production server
  ├── config.toml         # Mandatory configuration file
  └── logs/               # Runtime and server logs
```

## Dependency Chain

```
salata-core       ← shared library (config, parser, runtimes, security)
salata-cli        ← depends on salata-core
salata-cgi        ← depends on salata-core
salata-fastcgi    ← depends on salata-core
salata-server     ← depends on salata-cgi → salata-core
```

## salata (Core Interpreter)

The foundation. A general-purpose polyglot text templating engine. Takes a `.slt` file, parses it, finds language blocks, executes them in their respective runtimes, captures stdout, replaces the tags with output, and writes the result to stdout. The output is whatever the code prints — HTML, JSON, plain text, config files, or anything else.

```bash
salata index.slt > output.html
salata template.slt > config.yml
salata report.slt > report.txt
salata --config /path/to/config.toml index.slt
```

- No networking, no HTTP. Pure file-in → text-out.
- Output format is determined entirely by what the runtime blocks print.
- All other components depend on salata-core (the shared library).

### salata init

Built-in project bootstrapper. Detects available runtimes on the system, generates a config.toml with correct paths and disables missing runtimes, creates a starter `index.slt`, and creates `errors/` with default 404.slt and 500.slt.

```bash
salata init                    # in current directory
salata init --path ./my-site   # in specified directory
```

- Checks PATH for: python3/python, ruby, node, ts-node/tsx, php/php-cgi, whitelisted shells
- Prints summary: "Created config.toml with X of Y runtimes enabled."
- Will not overwrite existing config.toml without confirmation

## Execution Context

Salata is context-aware. An `ExecutionContext` enum is passed through the processing pipeline:

| Binary | Context | Effect |
|--------|---------|--------|
| `salata` (CLI) | `Cli` | PHP uses `php` binary |
| `salata-cgi` | `Cgi` | PHP uses `php-cgi` binary |
| `salata-fastcgi` | `FastCgi` | PHP uses `php-fpm` socket/TCP |
| `salata-server` | `Server` | PHP uses `php-fpm` socket/TCP |

Each binary sets its context before invoking salata-core. This mirrors how PHP itself works with different SAPIs.

## salata-cgi (CGI Bridge)

Traditional CGI bridge between web servers (nginx, Apache) and the salata interpreter. Receives a request, determines which `.slt` file to run, calls the interpreter, returns an HTTP response.

- Depends on salata-core only (not on the salata binary).
- Has built-in security protections against CGI attack vectors (see specs/SECURITY.md).
- **Current status:** The binary and all protections are fully implemented and unit-tested. However, integration with actual nginx and Apache web servers has not been tested yet. Testing and configuration documentation are planned.

## salata-fastcgi (Future — Stub for Now)

Persistent FastCGI daemon that will listen on a Unix socket or TCP port. For now, it is a hello-world stub:

```
$ salata-fastcgi
Salata FastCGI v0.1.0 — not yet implemented
```

- Depends on salata-core only.
- Will be implemented as `salata-fpm` — a persistent FastCGI process for nginx and Apache integration, avoiding the per-request process spawning overhead of traditional CGI.
- Placeholder module structure with TODO comments.

## salata-server (Standalone Server)

A standalone development/lightweight production server. This is currently the only tested way to serve `.slt` files over HTTP. Depends on salata-cgi (and therefore transitively on salata-core).

```bash
salata-server ./my-site --port 3000
salata-server index.slt --port 3000
```

- Serves full directories: `.slt` files processed, everything else (HTML, CSS, JS, images, fonts, media) served as-is.
- Uses a mature Rust web framework (e.g., `actix-web` or `hyper`) for HTTP handling.
- Framework provides: cookies, headers, sessions, redirects, compression, TLS/HTTPS, keep-alive, chunked transfer, content negotiation, static file serving with proper MIME types.
- **Hot reload:** In dev mode (`hot_reload = true`), watches for file changes and triggers reparse.

## Execution Model

### Processing Pipeline

```
Request comes in (or CLI invocation)
  → Read the .slt file
  → Resolve #include directives (text substitution, max depth 16)
  → Resolve #status, #content-type, #header, #cookie, #redirect directives
  → Parse content, extract runtime blocks
  → Validate: no nested runtime tags
  → Check runtime enabled status (error if disabled, exit if all disabled)
  → Expand #set/#get macros into native code per runtime
  → Group blocks by language
  → For each language: spawn/reuse one process (if shared_scope)
  → Send blocks with boundary markers
  → Capture stdout per block
  → Splice outputs back into document positions
  → Send final output (HTML, JSON, plain text, or whatever the code produced)
```

### Runtime Enable/Disable

Each runtime has an `enabled` field in config (default `true`). Disabled runtimes are skipped during execution. If a `.slt` file uses a disabled runtime tag, salata emits a clear error: "Runtime 'python' is disabled in config.toml". If ALL runtimes are disabled, salata prints "No runtimes enabled. Enable at least one runtime in config.toml to process .slt files." and exits with non-zero status.

### Execution Order

- Top-to-bottom. Each block executes in document order.
- Within shared scope, blocks for the same language maintain ordering.
- Synchronous. Each block finishes before the next one starts.

## Request Data

Runtimes access HTTP request data through standard CGI environment variables:
`REQUEST_METHOD`, `QUERY_STRING`, `CONTENT_TYPE`, `CONTENT_LENGTH`, `HTTP_HOST`, `HTTP_COOKIE`, `REMOTE_ADDR`, `REQUEST_URI`, `PATH_INFO`, `SERVER_NAME`, `SERVER_PORT`, `HTTP_AUTHORIZATION`

## Scope Management

### Shared Scope (Default)

All blocks of same language run in one process. Salata concatenates blocks with boundary markers (`__SALATA_BLOCK_BOUNDARY__`), splits output, splices back.

### Isolated Scope

Per-runtime: `shared_scope = false` in config. Per-block: `scope="isolated"` attribute.

## Error Handling

### display_errors

Global `[salata] display_errors = true` with per-runtime override. Resolution: runtime setting → global fallback. Errors always go to log files regardless.

### HTTP Status on Error

Default 200. Any runtime failure → 500 (overrides #status).

### Custom Error Pages

`[errors] page_404` and `page_500` in config — can be `.slt` files.

## Logging

### Log Directory

Default `./logs/` next to binaries. Created on first run or error out.

### Log Files

Per-runtime: `python.log`, `ruby.log`, `javascript.log`, `typescript.log`, `php.log`, `shell.log`
Server: `access.log`, `error.log`

### Log Format (CLI/Interpreter)

```
[2026-02-21 14:32:05] [ERROR] [python] [index.slt:15] NameError: name 'x' is not defined
[2026-02-21 14:32:05] [INFO]  [shell]  [index.slt:42] Block executed successfully (12ms)
```

### Log Rotation

`rotation_max_size = "50MB"`, `rotation_max_files = 10`

## Caching

Simple: cache parsed `.slt` structure (block positions, includes) by file path + mtime. Not output caching. Extensible later.

## Signal Handling

On SIGTERM/SIGINT/SIGQUIT: stop accepting requests → kill runtime processes → wait briefly → force kill → flush logs → exit cleanly. Errors during cleanup go to error log.

## Encoding

UTF-8 enforced everywhere. All runtimes, all output, all input.
