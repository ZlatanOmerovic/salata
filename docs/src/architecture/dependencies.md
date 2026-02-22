# Dependency Chain

Salata is structured as a Cargo workspace with five crates. The dependency relationships between them are intentionally simple and linear, with no circular dependencies.

## Dependency Graph

```text
salata-core       ← shared library (config, parser, runtimes, security)
    │
    ├── salata-cli        ← depends on salata-core
    │
    ├── salata-cgi        ← depends on salata-core
    │
    ├── salata-fastcgi    ← depends on salata-core
    │
    └── salata-server     ← depends on salata-cgi → salata-core
```

## salata-core

The foundation of the entire project. Every other crate depends on it, either directly or transitively. It is a library crate (no `main.rs`, no binary output).

salata-core contains:

- **Configuration** -- TOML parsing, validation, and the `Config` struct
- **Parser** -- `.slt` file parsing and block extraction
- **Runtimes** -- process spawning and stdout capture for all six languages (Python, Ruby, JavaScript, TypeScript, PHP, Shell)
- **Directives** -- `#include`, `#status`, `#content-type`, `#header`, `#cookie`, `#redirect`
- **Macros** -- `#set`/`#get` expansion into native code per language
- **Scope** -- shared and isolated scope management, boundary markers
- **Security** -- shell sandbox implementation, command blacklisting
- **Logging** -- log formatting, file writing, rotation
- **Error handling** -- error types, `display_errors` resolution
- **Caching** -- parsed file cache by path + mtime
- **Context** -- the `ExecutionContext` enum

No binary crate reimplements any of this logic. They all call into salata-core.

## salata-cli

Depends on: **salata-core**

The CLI interpreter binary. Its `main.rs` handles argument parsing, loads `config.toml`, sets `ExecutionContext::Cli`, and calls salata-core to process the `.slt` file. The result is written to stdout.

```toml
# crates/salata-cli/Cargo.toml
[dependencies]
salata-core = { path = "../salata-core" }
```

## salata-cgi

Depends on: **salata-core**

The CGI bridge binary. Its `main.rs` reads CGI environment variables, applies security protections (implemented in `protection.rs`), sets `ExecutionContext::Cgi`, and calls salata-core. The security protections (slowloris defense, request limits, path traversal blocking, etc.) are implemented within this crate, not in salata-core, because they are specific to the CGI deployment model.

```toml
# crates/salata-cgi/Cargo.toml
[dependencies]
salata-core = { path = "../salata-core" }
```

## salata-fastcgi

Depends on: **salata-core**

Currently a stub. When implemented, it will be a persistent FastCGI daemon. It depends on salata-core directly (not on salata-cgi), because FastCGI has its own protocol and process model distinct from CGI.

```toml
# crates/salata-fastcgi/Cargo.toml
[dependencies]
salata-core = { path = "../salata-core" }
```

## salata-server

Depends on: **salata-cgi** (which transitively depends on salata-core)

The standalone development server. This is the only binary that depends on another binary crate. It depends on salata-cgi because internally it processes `.slt` requests using the CGI pipeline -- the server receives an HTTP request, translates it into CGI-style invocation, and uses salata-cgi's processing logic to handle the request.

```toml
# crates/salata-server/Cargo.toml
[dependencies]
salata-cgi = { path = "../salata-cgi" }
```

Through salata-cgi, the server transitively depends on salata-core as well. This means salata-server has access to all configuration, parsing, runtime, and security functionality.

The server adds its own modules on top:

- `static_files.rs` -- serves non-`.slt` files (HTML, CSS, JS, images, fonts, media) with correct MIME types
- `hot_reload.rs` -- file watcher that triggers reparse when files change during development

## Why This Structure

### salata-core as a library

Keeping all shared logic in a single library crate means there is exactly one implementation of the parser, the runtimes, the macro expander, and everything else. Bug fixes and improvements in salata-core automatically apply to all four binaries.

### salata-server depending on salata-cgi

The server needs CGI-style request processing (translating HTTP requests into `.slt` file processing) and CGI-specific security protections (request limits, path traversal blocking). Rather than reimplementing these, it reuses salata-cgi's implementation.

### No circular dependencies

The dependency graph is a tree, not a graph with cycles. salata-core depends on no other project crates. The binary crates depend on salata-core (and in salata-server's case, also on salata-cgi). No crate depends on itself or on a crate that depends on it.

## External Dependencies

Beyond the internal crate dependencies, the project uses standard Rust ecosystem crates:

| Crate       | Purpose                                    |
|-------------|--------------------------------------------|
| `serde`     | Serialization/deserialization framework     |
| `toml`      | TOML configuration file parsing             |
| `thiserror` | Ergonomic error type definitions            |
| `actix-web` | HTTP framework (used by salata-server)      |

The project avoids `unwrap()` in production code and uses `thiserror` for proper error propagation throughout the crate hierarchy.
