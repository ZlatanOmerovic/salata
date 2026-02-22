# Salata — Polyglot Text Templating Engine

## What This Is
Salata ("salad" in Bosnian/Croatian/Serbian and most Slavic languages) is a general-purpose polyglot text templating engine. It processes `.slt` files containing embedded runtime blocks (`<python>`, `<ruby>`, `<javascript>`, `<typescript>`, `<php>`, `<shell>`), executes them, and outputs the result to stdout. The output is whatever the code prints — HTML, JSON, plain text, config files, anything.

## Tech Stack
- **Language:** Rust (Cargo workspace)
- **Cross-platform:** macOS, Linux, Windows — x64, x86, ARM. No platform-specific code.
- **Config format:** TOML (`config.toml`)
- **License:** MIT

## Architecture — 4 Binaries
- `salata` — core interpreter: `.slt` → evaluated output to stdout. No HTTP. Not HTML-specific.
  - `salata init` — subcommand: detects runtimes, generates config.toml + starter files
- `salata-cgi` — CGI bridge with attack protections. Built and unit-tested, but nginx/Apache integration not yet tested.
- `salata-fastcgi` — **stub only for now**, prints "not yet implemented". Will become `salata-fpm` for persistent FastCGI connections.
- `salata-server` — standalone dev server. **Only tested way to serve `.slt` over HTTP right now.** Depends on salata-cgi (and therefore salata-core).
- All binaries require `config.toml` next to them or via `--config` flag. No config = refuse to run.
- `logs/` directory lives next to binaries. Created on first run.

## Dependency Chain
```
salata-core       ← shared library (config, parser, runtimes, security)
salata-cli        ← depends on salata-core
salata-cgi        ← depends on salata-core
salata-fastcgi    ← depends on salata-core
salata-server     ← depends on salata-cgi → salata-core
```

## Execution Context
Salata is context-aware. Each binary sets its execution context:
- `salata` (CLI) → `ExecutionContext::Cli`
- `salata-cgi` → `ExecutionContext::Cgi`
- `salata-fastcgi` → `ExecutionContext::FastCgi`
- `salata-server` → `ExecutionContext::Server`

This affects runtime behavior (e.g., PHP uses `php` in CLI mode, `php-cgi` in CGI mode, `php-fpm` in FastCGI/Server mode).

## Runtime Enable/Disable
- Each runtime has an `enabled` field in config (default `true`)
- Disabled runtimes are skipped — using a disabled runtime in `.slt` produces a clear error
- If ALL runtimes are disabled: print informative message and exit non-zero

## Project Structure
See @specs/PROJECT_STRUCTURE.md for the full Cargo workspace layout.

## Key Design Rules
- Runtime tags: `<python>`, `<ruby>`, `<javascript>`, `<typescript>`, `<php>`, `<shell>`
- **No nesting runtime tags** inside other runtime tags. Parse-time error.
- Runtimes CAN print any text (HTML, JSON, plain text, etc.) via stdout — output is not restricted to HTML
- `<style>` and `<script>` are client-side, passed through untouched.
- Each language uses native stdout: `print()`, `puts`, `echo`, `console.log()`
- JS/TS get injected `print()`/`println()` helpers (additive, nothing overridden)
- Shared scope per language by default (one process per language per page)
- Each language is isolated from others — use `#set`/`#get` macros for cross-runtime data
- Execution is top-to-bottom, synchronous
- UTF-8 enforced everywhere

## Directives (outside runtime blocks only)
- `#include "file.slt"` — text substitution, max depth 16
- `#status 404` — once per page, default 200
- `#content-type application/json` — once per page
- `#header "X-Custom" "value"` — custom response headers
- `#cookie "session" "abc123" httponly secure`
- `#redirect "/other-page"`

## Macros (inside runtime blocks only)
- `#set("key", value)` — store JSON-serialized data
- `#get("key")` / `#get("key", default)` — retrieve deserialized native data

## CLI Subcommands
- `salata <file.slt>` — process a .slt file, output to stdout
- `salata init [--path dir]` — detect runtimes, generate config.toml + starter index.slt + errors/

## Commands
- `cargo fmt` — format before committing
- `cargo clippy` — must have zero warnings
- `cargo test` — unit + integration tests
- E2E tests run in Docker only: `docker compose -f docker/docker-compose.yml up --build test`

## Code Standards
- No `unwrap()` in production code — proper error handling with `thiserror`
- Use `serde` + `toml` crate for config parsing
- Use `std::path::PathBuf`, handle line endings, platform-agnostic process spawning
- Unit tests in `#[cfg(test)]` modules for all core logic

## Documentation
- **User docs (mdBook):** `docs/` — deployed to GitHub Pages via `.github/workflows/docs.yml`
- **API docs:** `cargo doc --no-deps --document-private-items`
- **Internal design specs:** `specs/` directory (source of truth for architecture decisions)

## CI/CD
- `.github/workflows/release.yml` — manual trigger, builds 8 targets (Linux/macOS/Windows), creates GitHub Release with 32 archives + SHA256 checksums
- `.github/workflows/docs.yml` — auto-deploys mdBook to GitHub Pages on push to main (`docs/**` paths)

## Key References
- Architecture details: @specs/ARCHITECTURE.md
- Full config reference: @specs/CONFIGURATION.md
- Runtime details (PHP context-aware binary, shell sandbox, JS/TS extras): @specs/RUNTIMES.md
- Directives & macros: @specs/DIRECTIVES_AND_MACROS.md
- Security (CGI protections, shell sandbox): @specs/SECURITY.md
- Testing & Docker setup: @specs/TESTING.md
- Docker playground: @specs/PLAYGROUND.md
- Examples: @specs/EXAMPLES.md
- Future uniform AST vision: @specs/UNIFORM_AST.md

## Helper Scripts
- `scripts/detect-runtimes.sh` — Linux/macOS runtime discovery, generates config.toml
- `scripts/detect-runtimes.bat` — Windows CMD equivalent
- `scripts/detect-runtimes.ps1` — PowerShell equivalent
