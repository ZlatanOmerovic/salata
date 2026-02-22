# Project Structure — Salata

```
salata/
  ├── Cargo.toml                    # Workspace root
  ├── config.toml                   # Default config
  ├── CLAUDE.md                     # Claude Code project memory
  ├── CHANGELOG.md
  ├── .github/
  │   └── workflows/
  │       ├── release.yml           # Manual: build 8 targets, create GitHub Release
  │       └── docs.yml              # Auto: deploy mdBook to GitHub Pages on push to main
  ├── docs/
  │   ├── book.toml                 # mdBook configuration
  │   └── src/
  │       ├── SUMMARY.md            # Table of contents
  │       ├── introduction.md
  │       ├── getting-started/      # Installation, quick start, playground
  │       ├── guide/                # Syntax, directives, macros, config, runtimes
  │       ├── architecture/         # Overview, binaries, pipeline, dependencies
  │       ├── security/             # Shell sandbox, CGI protections
  │       ├── examples/             # CLI and web example walkthroughs
  │       └── reference/            # FAQ, known issues, changelog, contributing
  ├── .claude/
  │   └── rules/                    # Auto-loaded rules per file pattern
  │       ├── rust.md
  │       ├── parser.md
  │       ├── security.md
  │       ├── runtimes.md
  │       └── testing.md
  ├── README.md
  ├── scripts/
  │   ├── detect-runtimes.sh        # Linux/macOS runtime discovery
  │   ├── detect-runtimes.bat       # Windows CMD runtime discovery
  │   └── detect-runtimes.ps1       # PowerShell runtime discovery
  ├── playground/
  │   ├── Dockerfile.playground     # Ubuntu + Rust + all runtimes + editors + Starship + bat
  │   ├── docker-compose.playground.yml  # Single interactive service
  │   ├── start-playground.sh       # Linux/macOS launcher
  │   ├── start-playground.bat      # Windows CMD launcher
  │   ├── start-playground.ps1      # PowerShell launcher
  │   └── README.md                 # Playground usage docs
  ├── examples/
  │   ├── cli/                      # CLI examples (salata file.slt → stdout)
  │   │   ├── hello-world/          # One .slt per runtime, simplest possible
  │   │   ├── data-processing/      # Python CSV, Ruby JSON, Shell system info
  │   │   ├── config-generator/     # Generate nginx.conf with Python + Shell
  │   │   ├── markdown-report/      # Output Markdown with Python + Ruby + Shell
  │   │   ├── cross-runtime-pipeline/  # #set/#get across all 6 runtimes
  │   │   ├── scope-demo/           # Shared vs isolated scope comparison
  │   │   ├── json-api-mock/        # Output JSON with #content-type
  │   │   └── multi-format/         # Same data as text, CSV, and YAML
  │   └── web/                      # Web examples (salata-server . --port 3000)
  │       ├── single-file/          # Individual .slt files with web directives
  │       ├── portfolio/            # Mini-site: pages, #include, static CSS
  │       ├── dashboard/            # All runtimes cooperating via #set/#get
  │       ├── php-showcase/         # PHP session, forms, dates + Python
  │       ├── api-endpoint/         # JSON API with #content-type
  │       ├── error-pages/          # Custom 404.slt and 500.slt
  │       └── blog/                 # Mini-site: posts from .txt files
  ├── docker/
  │   ├── Dockerfile                # Main build + all runtimes
  │   ├── Dockerfile.test           # E2E test runner
  │   ├── docker-compose.yml        # Orchestration (placeholder)
  │   └── .dockerignore
  ├── specs/
  │   ├── PROJECT_STRUCTURE.md      # This file
  │   ├── ARCHITECTURE.md           # Components, execution model, scope, logging
  │   ├── CONFIGURATION.md          # Full config.toml reference
  │   ├── RUNTIMES.md               # Runtime details, PHP context-aware, shell, JS/TS
  │   ├── DIRECTIVES_AND_MACROS.md  # #include, #status, #set, #get, etc.
  │   ├── SECURITY.md               # CGI protections, shell sandbox
  │   ├── TESTING.md                # Docker setup, E2E coverage
  │   ├── PLAYGROUND.md             # Docker playground spec
  │   ├── EXAMPLES.md               # Examples directory spec
  │   ├── README.md                 # Quick project overview
  │   └── UNIFORM_AST.md           # Future transpilation vision
  ├── crates/
  │   ├── salata-core/              # Shared: config, parser, runtimes, security
  │   │   ├── Cargo.toml
  │   │   └── src/
  │   │       ├── lib.rs
  │   │       ├── config.rs         # TOML config parsing and validation
  │   │       ├── context.rs        # ExecutionContext enum (Cli, Cgi, FastCgi, Server)
  │   │       ├── parser.rs         # .slt parser, block extraction
  │   │       ├── directives.rs     # #include, #status, #content-type, #header, #cookie, #redirect
  │   │       ├── macros.rs         # #set/#get macro expansion
  │   │       ├── runtime/
  │   │       │   ├── mod.rs
  │   │       │   ├── python.rs
  │   │       │   ├── ruby.rs
  │   │       │   ├── javascript.rs
  │   │       │   ├── typescript.rs
  │   │       │   ├── php.rs
  │   │       │   └── shell.rs      # Includes full sandbox implementation
  │   │       ├── scope.rs          # Shared/isolated scope management
  │   │       ├── cache.rs          # Parsed file caching
  │   │       ├── logging.rs        # Log formatting and rotation
  │   │       ├── error.rs          # Error types and display_errors logic
  │   │       ├── security.rs       # Shell sandbox, command blacklist
  │   │       └── uniform_ast/      # Future: cross-language transpilation
  │   │           └── mod.rs        # TODO placeholder with comments
  │   ├── salata-cli/               # The `salata` binary
  │   │   ├── Cargo.toml
  │   │   └── src/
  │   │       ├── main.rs
  │   │       └── init.rs           # `salata init` — runtime detection + project scaffolding
  │   ├── salata-cgi/               # The `salata-cgi` binary
  │   │   ├── Cargo.toml
  │   │   └── src/
  │   │       ├── main.rs
  │   │       └── protection.rs     # All CGI attack protections
  │   ├── salata-fastcgi/           # The `salata-fastcgi` binary (stub)
  │   │   ├── Cargo.toml
  │   │   └── src/
  │   │       └── main.rs           # Hello world / not yet implemented
  │   └── salata-server/            # The `salata-server` binary
  │       ├── Cargo.toml
  │       └── src/
  │           ├── main.rs
  │           ├── static_files.rs   # Static file serving
  │           └── hot_reload.rs     # File watcher for dev mode
  ├── tests/
  │   ├── fixtures/                 # Sample .slt files for testing
  │   ├── e2e/                      # E2E tests (run inside Docker)
  │   └── integration/              # Rust integration tests
  └── errors/                       # Default error page templates
      ├── 404.slt
      └── 500.slt
```
