# Hello World

The simplest possible Salata example — one file per runtime, each printing a greeting.

## Files

- `python.slt` — Python: `print()`
- `ruby.slt` — Ruby: `puts`
- `javascript.slt` — JavaScript: `println()` (Salata-injected helper)
- `typescript.slt` — TypeScript: `println()` with type annotation
- `php.slt` — PHP: `echo`
- `shell.slt` — Shell: `echo`

## Run

```bash
salata --config config.toml python.slt
salata --config config.toml ruby.slt
salata --config config.toml javascript.slt
# ... etc
```

## What It Demonstrates

- Basic runtime block syntax: `<python>`, `<ruby>`, `<javascript>`, `<typescript>`, `<php>`, `<shell>`
- Each runtime's native stdout function
- Minimal config with all runtimes enabled
