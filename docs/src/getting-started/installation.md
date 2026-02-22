# Installation

Salata is built from source using the Rust toolchain. There are no pre-built binaries or package manager packages at this time.

## Prerequisites

### Rust toolchain

Salata requires a working Rust installation with `cargo`. The recommended way to install Rust is through [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On Windows, download and run the installer from [rustup.rs](https://rustup.rs/).

Salata targets the **stable** Rust toolchain. Any recent stable version (1.70+) should work. Verify your installation:

```bash
rustc --version
cargo --version
```

### At least one runtime

Salata needs at least one language runtime installed on your system to be useful. The supported runtimes are:

| Runtime | Binary | Common locations |
|---------|--------|-----------------|
| Python | `python3` (or `python`) | `/usr/bin/python3`, `/usr/local/bin/python3`, `/opt/homebrew/bin/python3` |
| Ruby | `ruby` | `/usr/bin/ruby`, `/usr/local/bin/ruby`, `/opt/homebrew/bin/ruby` |
| JavaScript | `node` | `/usr/bin/node`, `/usr/local/bin/node`, `/opt/homebrew/bin/node` |
| TypeScript | `tsx`, `ts-node`, or `bun` | `/usr/local/bin/tsx`, `/usr/local/bin/ts-node` |
| PHP | `php` (CLI), `php-cgi` (CGI) | `/usr/bin/php`, `/usr/bin/php-cgi`, `/opt/homebrew/bin/php` |
| Shell | `bash`, `sh`, `zsh`, `fish`, `dash`, `ash` | `/bin/bash`, `/bin/sh`, `/usr/bin/zsh` |

You do not need all six. Salata detects which runtimes are available and disables the rest. If your `.slt` file uses a disabled runtime, Salata will report a clear error.

> **Tip:** If you do not want to install runtimes locally, use the [Docker Playground](./playground.md) instead. It comes with all six runtimes pre-installed.

## Building from source

Clone the repository and build in release mode:

```bash
git clone https://github.com/nicholasgasior/salata.git
cd salata
cargo build --release
```

This produces four binaries in `target/release/`:

| Binary | Purpose |
|--------|---------|
| `salata` | Core CLI interpreter. Processes `.slt` files and writes output to stdout. |
| `salata-cgi` | CGI bridge with attack protections. Built, but nginx/Apache integration not yet tested. |
| `salata-fastcgi` | FastCGI daemon (stub -- not yet implemented). |
| `salata-server` | Standalone dev server -- the only tested way to serve `.slt` over HTTP right now. |

You can copy these binaries wherever you like. The only requirement is that a `config.toml` file must be present next to the binary or specified via the `--config` flag. Without a config file, none of the binaries will run.

> **Note:** The build produces all four binaries from a Cargo workspace. You cannot build them individually without the workspace root `Cargo.toml`.

## Initializing a project

After building, the fastest way to get started is the `salata init` command. It scans your system for available runtimes, generates a `config.toml` with the correct binary paths and enabled/disabled flags, and creates starter files:

```bash
./target/release/salata init
```

Or specify a target directory:

```bash
./target/release/salata init --path ./my-project
```

The init command:

1. **Detects runtimes** -- checks well-known paths and falls back to `which` (Unix) or `where` (Windows) for each of the six runtimes
2. **Generates `config.toml`** -- with detected paths, `enabled = true` for found runtimes and `enabled = false` for missing ones
3. **Creates `index.slt`** -- a starter template using the first available runtime (prefers Python, then Node.js, Ruby, Shell, PHP, TypeScript)
4. **Creates `errors/404.slt` and `errors/500.slt`** -- default error page templates

Example output:

```text
Detecting runtimes...
  python         /usr/bin/python3  (Python 3.12.3)
  ruby           /usr/bin/ruby  (ruby 3.2.2)
  javascript     /usr/local/bin/node  (v20.11.0)
  typescript     /usr/local/bin/tsx  (tsx v4.7.0)
  php            /usr/bin/php  (PHP 8.3.2)
  php-cgi        /usr/bin/php-cgi  (PHP 8.3.2)
  shell          /bin/bash  (GNU bash, version 5.2.26)
Created config.toml with 6 of 6 runtimes enabled.
Run: salata index.slt
```

If a runtime is not found, it will show:

```text
  typescript     not found — will be disabled
```

The generated `config.toml` will have `enabled = false` for that runtime.

## Runtime discovery scripts

For environments where `salata init` is not available (or if you prefer a shell-based approach), Salata includes standalone runtime discovery scripts in the `scripts/` directory:

| Script | Platform |
|--------|----------|
| `scripts/detect-runtimes.sh` | Linux, macOS (Bash) |
| `scripts/detect-runtimes.bat` | Windows (CMD) |
| `scripts/detect-runtimes.ps1` | Windows (PowerShell) |

These scripts scan for the same runtimes as `salata init` and generate a `config.toml` file. They are useful if you want to generate a config without building Salata first, or if you are setting up a deployment environment.

```bash
# Linux / macOS
bash scripts/detect-runtimes.sh > config.toml

# Windows CMD
scripts\detect-runtimes.bat > config.toml

# PowerShell
powershell -File scripts\detect-runtimes.ps1 > config.toml
```

## Cross-platform notes

Salata is designed to run on macOS, Linux, and Windows across x64, x86, and ARM architectures. There is no platform-specific code in the Salata codebase itself.

Platform considerations:

- **macOS (including Apple Silicon):** Homebrew-installed runtimes are detected at `/opt/homebrew/bin/`. System Python at `/usr/bin/python3` is typically available.
- **Linux:** Most distributions include Python and Bash by default. Other runtimes can be installed via your package manager (`apt`, `dnf`, `pacman`, etc.).
- **Windows:** Runtime detection uses `where` instead of `which`. Shell paths must still be absolute. Common paths differ (`C:\Python312\python.exe`, etc.). The detection scripts handle these differences.

> **Tip:** The `config.toml` file uses absolute paths to runtime binaries. If you move runtimes or switch between system and Homebrew installations, re-run `salata init` or update the paths manually.

## Alternative: Docker playground

If you want to try Salata without installing anything locally (beyond Docker), the playground container has everything pre-configured:

```bash
cd playground
./start-playground.sh
```

See the [Playground Guide](./playground.md) for full details.

## Verifying the installation

After building and initializing, verify everything works:

```bash
# Check the version
./target/release/salata --version

# Process the starter file
./target/release/salata index.slt
```

You should see HTML output with "Hello from Salata!" (the exact runtime used depends on what was detected on your system).

## Next steps

- [Quick Start](./quick-start.md) -- go from zero to running output in 5 minutes
- [Your First .slt File](./first-slt-file.md) -- understand `.slt` syntax step by step
- [Playground Guide](./playground.md) -- try Salata in Docker with all runtimes
