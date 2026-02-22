# Known Issues / TODO

This page documents known limitations, incomplete features, and planned work.

---

## nginx / Apache Integration Untested

The `salata-cgi` binary and all its security protections (Slowloris defense, path traversal blocking, input sanitization, etc.) are fully built and unit-tested. However, integration with actual nginx and Apache web servers has not been tested yet. Configuration examples and integration testing are planned.

For now, `salata-server` is the only tested way to serve `.slt` files over HTTP.

---

## FastCGI Stub

The `salata-fastcgi` binary is currently a stub. Running it prints:

```text
Salata FastCGI v0.1.0 — not yet implemented
```

The full FastCGI daemon (planned as `salata-fpm`) would listen on a Unix socket or TCP port for persistent connections with nginx and Apache, avoiding per-request process spawning overhead. The module structure and placeholder code exist at `crates/salata-fastcgi/`.

---

## Shell #set/#get Macros

The `#set` and `#get` macro expansion for shell blocks currently produces invalid syntax. Shell's string handling and lack of native JSON support make the expansion non-trivial. For now, use other runtimes (Python, Ruby, JavaScript, TypeScript, PHP) for cross-runtime data sharing. Shell blocks can still read and write files directly, but they cannot participate in the `#set`/`#get` data bridge.

---

## Shell Sandbox Side Effects

The shell sandbox's security restrictions have some side effects that affect legitimate use cases:

- **No `/dev/null` redirects** -- the `/dev` path block prevents `>/dev/null`, `2>/dev/null`, and any other reference to device files. This is collateral damage from blocking `/dev/tcp` and `/dev/udp`.
- **No `2>&1`** -- the `&` character check does not distinguish between backgrounding (`command &`) and file descriptor redirection (`2>&1`).
- **No backgrounding** -- `command &` is blocked. Long-running background tasks cannot be started from shell blocks.
- **No `/etc` access** -- reading configuration files like `/etc/hostname` or `/etc/os-release` is blocked.

These restrictions are by design. See the [Shell Sandbox](../security/shell-sandbox.md) chapter for details.

---

## Windows Support

Salata compiles and builds on Windows (x86_64, i686, ARM64), but it has not been extensively tested on Windows. Known concerns:

- The shell sandbox uses Unix-specific path conventions (`/bin/bash`, `/usr/bin/sh`, etc.) and `ulimit`. These do not apply on Windows.
- Runtime binary paths in the default `config.toml` use Unix paths. Windows users need to update these to point to their installed interpreters.
- Line ending handling (CRLF vs LF) has not been thoroughly tested.

The Docker playground is recommended for a consistent cross-platform experience.

---

## Uniform AST (Future Vision)

The Uniform AST is a planned feature for cross-language function and class transpilation, with TypeScript as the first-class citizen. The idea is that you would define a class or function in TypeScript, and Salata would transpile it to equivalent Python, Ruby, and PHP code so that all runtimes can use it natively.

**Current status:** Not implemented. A placeholder module exists at `crates/salata-core/src/uniform_ast/mod.rs` with comprehensive TODO comments describing the design.

**Dependencies:** The `#set`/`#get` data bridge must be fully implemented and stable first. TypeScript parsing would use the `swc` crate.

**Design constraints:** Only a "Salata-compatible" subset of TypeScript would be supported -- no decorators, no mixins, no closures, no async, no metaprogramming, no stdlib mapping. Shell is excluded from transpilation targets.

---

## No Async Execution

All runtime blocks execute synchronously, top-to-bottom. Block 1 must finish before block 2 starts. There is no parallel execution of runtime blocks, even when blocks use different runtimes that could theoretically run concurrently. This simplifies the execution model and guarantees deterministic output, but it means performance scales linearly with the number of blocks.

---

## No Output Caching

Salata caches the parsed structure of `.slt` files (block positions, include resolutions) by file path and modification time. However, runtime output is never cached -- every request re-executes all runtime blocks. This means the output is always fresh, but it also means repeated requests for the same page do the same work every time.

---

## PHP FastCGI Mode

The `php-fpm` socket/TCP connection for PHP in FastCGI and Server execution contexts is not yet implemented. PHP blocks currently work in CLI mode (`php` binary) and CGI mode (`php-cgi` binary), but the FastCGI mode that would use `php-fpm` via a Unix socket or TCP connection is planned but not built.

The configuration fields (`fastcgi_socket` and `fastcgi_host` in `[runtimes.php]`) exist and are parsed, but they are not used yet.

---

## Single-Threaded Per Request

Each request is processed sequentially through all runtime blocks. Within a single request, there is no parallelism. If a page has five runtime blocks, they execute one after another. This is a consequence of the synchronous, top-to-bottom execution model and the shared scope system (where blocks of the same language share a single process).
