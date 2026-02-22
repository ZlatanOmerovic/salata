# Architecture Overview

Salata is a polyglot text templating engine built as a Rust Cargo workspace. It takes `.slt` files containing embedded runtime blocks, executes them server-side using the appropriate language interpreter, captures their stdout, and produces the final output. The output can be HTML, JSON, plain text, configuration files, or any other text format.

## Workspace Structure

The project is organized as five Rust crates in a Cargo workspace:

```text
salata/
  ├── Cargo.toml              # Workspace root
  ├── config.toml              # Mandatory configuration
  ├── crates/
  │   ├── salata-core/         # Shared library
  │   ├── salata-cli/          # CLI binary
  │   ├── salata-cgi/          # CGI binary
  │   ├── salata-fastcgi/      # FastCGI binary (stub)
  │   └── salata-server/       # Dev server binary
  ├── tests/                   # Integration tests and fixtures
  ├── errors/                  # Default error page templates
  └── logs/                    # Created at runtime
```

## salata-core: The Foundation

`salata-core` is the shared library that all other crates depend on. It contains:

| Module            | Responsibility                                               |
|-------------------|--------------------------------------------------------------|
| `config.rs`       | TOML configuration parsing and validation                    |
| `context.rs`      | `ExecutionContext` enum (Cli, Cgi, FastCgi, Server)          |
| `parser.rs`       | `.slt` file parsing, block extraction                        |
| `directives.rs`   | `#include`, `#status`, `#content-type`, `#header`, `#cookie`, `#redirect` |
| `macros.rs`       | `#set`/`#get` macro expansion into native code               |
| `runtime/`        | Runtime implementations for all six languages                |
| `scope.rs`        | Shared and isolated scope management                         |
| `cache.rs`        | Parsed file caching by path + mtime                          |
| `logging.rs`      | Log formatting and rotation                                  |
| `error.rs`        | Error types and `display_errors` logic                       |
| `security.rs`     | Shell sandbox and command blacklist                          |
| `uniform_ast/`    | Future cross-language transpilation (placeholder)            |

## Four Binaries

Salata produces four binary executables, each serving a different deployment context:

- **`salata`** -- The core interpreter. Reads a `.slt` file and writes the processed output to stdout. No networking, no HTTP. Pure file-in, text-out.

- **`salata-cgi`** -- A CGI bridge for web servers like nginx or Apache. Receives HTTP requests via the CGI protocol, processes the requested `.slt` file, and returns an HTTP response. Includes built-in security protections against CGI attack vectors.

- **`salata-fastcgi`** -- A FastCGI daemon (currently a stub that prints "not yet implemented"). Will eventually listen on a Unix socket or TCP port for persistent request handling.

- **`salata-server`** -- A standalone development server. Serves directories with `.slt` files processed on the fly and static files served as-is. Supports hot reload for development.

See the [Binaries](binaries.md) chapter for detailed descriptions of each.

## Configuration

All four binaries require a `config.toml` file. Without it, they refuse to run. The config file is looked up in this order:

1. `--config /path/to/config.toml` command-line flag
2. `config.toml` in the same directory as the binary
3. Error -- Salata exits with a message

Configuration covers runtime paths and settings, logging, CGI security limits, error pages, and server options. Every runtime can be individually enabled or disabled.

## Cross-Platform

Salata targets macOS, Linux, and Windows across x64, x86, and ARM architectures. The codebase avoids platform-specific code:

- File paths use `std::path::PathBuf`
- Line endings are handled correctly on all platforms
- Process spawning is platform-agnostic

## Encoding

UTF-8 is enforced everywhere -- all input files, all runtime output, all final output, all configuration. There is no option to change this.

## How Components Fit Together

```text
                    ┌─────────────────┐
                    │   config.toml   │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
    ┌─────────▼──┐  ┌───────▼────┐  ┌──────▼──────┐
    │ salata-cli │  │ salata-cgi │  │salata-server│
    │  (CLI)     │  │  (CGI)     │  │  (HTTP)     │
    └─────────┬──┘  └───────┬────┘  └──────┬──────┘
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────▼────────┐
                    │   salata-core   │
                    │                 │
                    │  parser         │
                    │  directives     │
                    │  macros         │
                    │  runtimes       │
                    │  scope          │
                    │  security       │
                    │  config         │
                    │  logging        │
                    └────────┬────────┘
                             │
              ┌──────┬───────┼───────┬──────┬──────┐
              │      │       │       │      │      │
            Python  Ruby    JS     TS    PHP   Shell
```

Each binary sets an `ExecutionContext` before calling into salata-core. This context flows through the entire processing pipeline and affects runtime behavior -- most notably, which PHP binary is selected. See [Execution Context](execution-context.md) for details.

## Logging

Each runtime gets its own log file (`python.log`, `ruby.log`, etc.) in the `logs/` directory next to the binary. The server also maintains `access.log` and `error.log`. Log rotation is configured via `rotation_max_size` and `rotation_max_files`.

## Caching

Salata caches the parsed structure of `.slt` files (block positions, includes) keyed by file path and modification time. This is a parse cache, not an output cache. When a file changes, the cache entry is invalidated and the file is re-parsed.

## Error Handling

Error display is controlled by the `display_errors` setting, which can be set globally and overridden per runtime. Regardless of the display setting, all errors are written to the log files. When any runtime block fails, the HTTP status code is automatically set to 500, overriding any `#status` directive.
