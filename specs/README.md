# Salata

**Polyglot text templating engine.** Embed Python, Ruby, JavaScript, TypeScript, PHP, and Shell directly in `.slt` files. Output can be HTML, JSON, plain text, config files — anything.

**Repository:** [github.com/ZlatanOmerovic/salata](https://github.com/ZlatanOmerovic/salata)

## What is Salata?

Salata — "salad" in Bosnian, Croatian, Serbian, and most Slavic languages — is a mix of everything thrown together. A general-purpose polyglot text templating engine. It processes `.slt` files containing embedded runtime blocks using semantic tags. Each language block executes server-side (or via CLI), and the combined output is written to stdout. The most common use case is generating HTML, but Salata is not HTML-specific — the output is whatever your code prints.

```html
<!-- index.slt -->
<!DOCTYPE html>
<html>
<body>
  <python>
    import sqlite3
    db = sqlite3.connect("app.db")
    users = db.execute("SELECT * FROM users").fetchall()
  </python>

  <h1>We have <python>print(len(users))</python> users</h1>

  <python>
  for user in users:
      print(f"<tr><td>{user['name']}</td></tr>")
  </python>

  <shell>DISK=$(df -h / | tail -1 | awk '{print $5}')</shell>
  <footer>Disk: <shell>echo $DISK</shell></footer>

  <php>echo date("H:i:s");</php>
</body>
</html>
```

## Components

| Binary | Description | Status | Depends On |
|--------|-------------|--------|------------|
| `salata` | Core interpreter — `.slt` file → evaluated output to stdout | Working | salata-core |
| `salata-cgi` | CGI bridge with built-in security protections | Built, nginx/Apache integration untested | salata-core |
| `salata-fastcgi` | FastCGI daemon (future: `salata-fpm`) | Stub | salata-core |
| `salata-server` | Standalone dev server — only tested HTTP serving method | Working | salata-cgi → salata-core |

## Quick Start

```bash
# Build
cargo build --release

# Bootstrap a new project (detects runtimes, generates config.toml + starter files)
salata init

# Run interpreter (output is whatever code prints — not limited to HTML)
salata index.slt > output.html
salata template.slt > config.yml
salata report.slt    # prints to stdout

# Start dev server
salata-server ./my-site --port 3000

# Detect available runtimes and generate config.toml (standalone script)
./scripts/detect-runtimes.sh

# Try Salata instantly with Docker (no local install needed)
./playground/start-playground.sh
```

## Supported Languages

| Language | Tag | Output |
|----------|-----|--------|
| Python | `<python>` | `print()` |
| Ruby | `<ruby>` | `puts` |
| JavaScript | `<javascript>` | `console.log()`, `print()`, `println()` |
| TypeScript | `<typescript>` | `console.log()`, `print()`, `println()` |
| PHP | `<php>` | `echo` |
| Shell | `<shell>` | `echo` |

## Configuration

Salata requires a `config.toml` file. See [specs/CONFIGURATION.md](specs/CONFIGURATION.md) for the full reference.

## Examples

See the `examples/` directory for comprehensive examples:
- `examples/cli/` — CLI usage (text output, config generation, cross-runtime pipelines)
- `examples/web/` — Web usage (mini-sites, dashboards, APIs, blogs)

Each example is self-contained with its own config.toml and README.

## Documentation

- [Architecture](specs/ARCHITECTURE.md) — components, execution model, logging
- [Configuration](specs/CONFIGURATION.md) — full config.toml reference
- [Runtimes](specs/RUNTIMES.md) — runtime details, PHP context-aware binary, shell sandbox
- [Directives & Macros](specs/DIRECTIVES_AND_MACROS.md) — #include, #status, #set/#get
- [Security](specs/SECURITY.md) — CGI protections, shell sandbox
- [Testing](specs/TESTING.md) — Docker setup, E2E tests
- [Playground](specs/PLAYGROUND.md) — Docker-based interactive playground
- [Examples](specs/EXAMPLES.md) — examples directory spec
- [Project Structure](specs/PROJECT_STRUCTURE.md) — Cargo workspace layout

## License

MIT
