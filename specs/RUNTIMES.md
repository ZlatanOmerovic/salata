# Runtimes — Salata

## Supported Runtimes

| Language   | Tag              | Output                          | Notes                              |
|------------|------------------|---------------------------------|------------------------------------|
| Python     | `<python>`       | `print()`                       |                                    |
| Ruby       | `<ruby>`         | `puts`                          |                                    |
| JavaScript | `<javascript>`   | `console.log()`, `process.stdout.write()`, `print()`, `println()` | `print()`/`println()` injected |
| TypeScript | `<typescript>`   | `console.log()`, `process.stdout.write()`, `print()`, `println()` | `print()`/`println()` injected |
| PHP        | `<php>`          | `echo`                          | Context-aware binary selection    |
| Shell      | `<shell>`        | `echo`                          | Hardcoded shell whitelist          |

## Output Rule

Whatever hits stdout gets captured and placed at the tag's position. The output is not restricted to HTML — it can be any text (JSON, plain text, config syntax, etc.). No magic auto-printing.

## Runtime Enable/Disable

Each runtime has `enabled = true` (default) in config. Set `enabled = false` to disable. If a `.slt` file uses a disabled runtime, salata produces a clear error. If ALL runtimes are disabled, salata prints an informative message and exits.

## Content Rules

- **No nesting runtime tags** inside other runtime tags — parse-time error
- Runtimes CAN print any text — HTML, JSON, plain text, config syntax, etc.
- `<style>` and `<script>` are client-side, passed through untouched

## JavaScript / TypeScript Extras

Salata injects before user code:

```javascript
const print = (...args) => process.stdout.write(args.join(' '));
const println = (...args) => process.stdout.write(args.join(' ') + '\n');
```

- `print()` — no newline
- `println()` — with newline
- `console.log` and `process.stdout.write` still work, nothing overridden
- Additive, not destructive

## PHP — Context-Aware Binary Selection

PHP binary is selected based on execution context (mirrors PHP's own SAPI model):

| Context | Binary Used | Config Field |
|---------|-------------|--------------|
| CLI (`salata`) | `php` | `cli_path` |
| CGI (`salata-cgi`) | `php-cgi` | `cgi_path` |
| FastCGI (`salata-fastcgi`) | php-fpm socket/TCP | `fastcgi_socket` / `fastcgi_host` |
| Server (`salata-server`) | php-fpm socket/TCP | `fastcgi_socket` / `fastcgi_host` |

```toml
# Full PHP config with all paths
[runtimes.php]
enabled = true
mode = "cgi"
cli_path = "/usr/bin/php"
cgi_path = "/usr/bin/php-cgi"
fastcgi_socket = "/run/php/php-fpm.sock"
# fastcgi_host = "127.0.0.1:9000"
shared_scope = true
```

When running via `salata` CLI, the `php` binary is used directly (no CGI overhead). When running via `salata-cgi` or `salata-server`, `php-cgi` or `php-fpm` is used as appropriate.

## Shell Runtime

### Known Limitation: #set/#get in Shell

`#set`/`#get` macros in shell blocks currently produce invalid syntax. The expansion generates `__salata_set("key", "val")` (Python-style call), but shell requires `__salata_set "key" "val"` (space-separated). Use other runtimes for cross-runtime data and keep shell blocks output-only until this is fixed.

### Allowed Shells (Hardcoded in binary)

Not configurable via config — security boundary. Only way to change: modify source and recompile.

```
/bin/sh, /bin/bash, /bin/zsh
/usr/bin/sh, /usr/bin/bash, /usr/bin/zsh, /usr/bin/fish, /usr/bin/dash, /usr/bin/ash
```

- Path in config must match whitelist
- Must be absolute path
- Non-matching = salata refuses to start

### Shell Sandbox

See specs/SECURITY.md for the full specification.
