# Runtime Configuration

Each runtime is configured under its own `[runtimes.*]` section in `config.toml`. This chapter covers the common configuration fields and the specifics for each runtime.

## Common Fields

All runtimes share these configuration fields:

### enabled

**Type:** bool
**Default:** `true`

Enables or disables the runtime. When a runtime is disabled:

- Salata skips it during execution
- If a `.slt` file uses a disabled runtime's tag, Salata produces a clear error: *"Runtime 'python' is disabled in config.toml"*
- If **all** runtimes are disabled, Salata prints *"No runtimes enabled. Enable at least one runtime in config.toml to process .slt files."* and exits with a non-zero status

```toml
[runtimes.ruby]
enabled = false  # Ruby blocks will produce an error
```

### path

**Type:** string
**Default:** varies by runtime

Absolute path to the runtime binary. This must point to a valid executable on the system.

```toml
[runtimes.python]
path = "/usr/local/bin/python3.12"
```

### shared_scope

**Type:** bool
**Default:** `true`

When `true`, all blocks of this language run in a single process and share state (variables, imports, functions). When `false`, each block runs in its own fresh process.

```toml
[runtimes.python]
shared_scope = false  # Every <python> block gets a fresh process
```

See [Scope (Shared vs Isolated)](./scope.md) for detailed behavior.

### display_errors

**Type:** bool
**Default:** inherited from `[salata] display_errors`

Override the global error display setting for this specific runtime. When `true`, runtime errors are included in the output. When `false`, errors are logged but the output shows nothing (or an empty string) for the failed block.

Resolution order: runtime-specific `display_errors` -> global `[salata] display_errors` fallback.

```toml
[salata]
display_errors = false  # Global: hide errors

[runtimes.python]
display_errors = true   # Override: show Python errors anyway
```

## Per-Runtime Configuration

### Python

```toml
[runtimes.python]
enabled = true
path = "/usr/bin/python3"
shared_scope = true
display_errors = true
```

The `path` should point to a Python 3 interpreter. Common locations:

- Linux: `/usr/bin/python3`
- macOS (Homebrew): `/usr/local/bin/python3` or `/opt/homebrew/bin/python3`
- Custom virtualenv: `/path/to/venv/bin/python3`

### Ruby

```toml
[runtimes.ruby]
enabled = true
path = "/usr/bin/ruby"
shared_scope = true
```

Common locations:

- Linux: `/usr/bin/ruby`
- macOS (Homebrew): `/usr/local/bin/ruby` or `/opt/homebrew/bin/ruby`
- rbenv: `~/.rbenv/shims/ruby`

### JavaScript

```toml
[runtimes.javascript]
enabled = true
path = "/usr/bin/node"
shared_scope = true
```

The `path` should point to a Node.js binary. Common locations:

- Linux: `/usr/bin/node` or `/usr/local/bin/node`
- macOS (Homebrew): `/usr/local/bin/node` or `/opt/homebrew/bin/node`
- nvm: `~/.nvm/versions/node/v20.x.x/bin/node`

### TypeScript

```toml
[runtimes.typescript]
enabled = true
path = "/usr/bin/ts-node"
shared_scope = true
```

The TypeScript runner is configurable. You can use any of these by changing the `path`:

- `ts-node` -- the traditional TypeScript runner
- `tsx` -- a faster alternative to ts-node
- `bun` -- Bun's built-in TypeScript support
- `deno` -- Deno's built-in TypeScript support

```toml
# Using tsx instead of ts-node
[runtimes.typescript]
path = "/usr/local/bin/tsx"

# Using bun
[runtimes.typescript]
path = "/usr/local/bin/bun"
```

### PHP

PHP has the most complex configuration because it is context-aware -- different binaries are used depending on the execution context.

```toml
[runtimes.php]
enabled = true
mode = "cgi"
cli_path = "/usr/bin/php"
cgi_path = "/usr/bin/php-cgi"
# fastcgi_socket = "/run/php/php-fpm.sock"
# fastcgi_host = "127.0.0.1:9000"
shared_scope = true
```

| Field | Used When | Description |
|-------|-----------|-------------|
| `mode` | Always | `"cgi"` or `"fastcgi"` -- determines how PHP is invoked |
| `cli_path` | `salata` (CLI context) | Path to the `php` binary for command-line use |
| `cgi_path` | `salata-cgi` (CGI context) | Path to the `php-cgi` binary |
| `fastcgi_socket` | `salata-fastcgi` / `salata-server` | Unix socket path for php-fpm |
| `fastcgi_host` | `salata-fastcgi` / `salata-server` | TCP address for php-fpm (e.g., `"127.0.0.1:9000"`) |

The binary selection follows the execution context:

| Binary | Context | PHP Binary Used |
|--------|---------|-----------------|
| `salata` | Cli | `cli_path` (`php`) |
| `salata-cgi` | Cgi | `cgi_path` (`php-cgi`) |
| `salata-fastcgi` | FastCgi | `fastcgi_socket` or `fastcgi_host` (`php-fpm`) |
| `salata-server` | Server | `fastcgi_socket` or `fastcgi_host` (`php-fpm`) |

### Shell

```toml
[runtimes.shell]
enabled = true
path = "/bin/bash"
shared_scope = true
```

The shell runtime is the most restricted. The `path` must be one of the hardcoded allowed shells:

- `/bin/sh`
- `/bin/bash`
- `/bin/zsh`
- `/usr/bin/sh`
- `/usr/bin/bash`
- `/usr/bin/zsh`
- `/usr/bin/fish`
- `/usr/bin/dash`
- `/usr/bin/ash`

Setting `path` to any other value will be rejected. This is a security boundary -- the whitelist is hardcoded in the binary and cannot be changed via configuration.

See the [Shell Sandbox](../security/shell-sandbox.md) documentation for full details on shell security restrictions.

## Runtime Detection Scripts

Salata ships with helper scripts that detect installed runtimes and generate a `config.toml` with the correct paths for your system:

- **Linux / macOS:** `scripts/detect-runtimes.sh`
- **Windows CMD:** `scripts/detect-runtimes.bat`
- **PowerShell:** `scripts/detect-runtimes.ps1`

```bash
# Generate config.toml with detected runtime paths
./scripts/detect-runtimes.sh > config.toml
```

These scripts check standard locations for each runtime binary and produce a valid configuration file with `enabled = false` for any runtimes not found on the system.
