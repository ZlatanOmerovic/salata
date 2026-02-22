# Playground Guide

The Salata Playground is a Docker container pre-loaded with all six runtimes, editors, and pre-built Salata binaries. It is the recommended way to try Salata, especially if you do not want to install language runtimes on your host system.

## What is included

The playground container is built on Ubuntu and comes with:

**Runtimes:**
- Python 3
- Ruby
- Node.js (LTS)
- TypeScript via `ts-node` and `tsx`
- PHP (CLI and CGI)
- Shell: Bash, Dash, Zsh, and Fish

**Editors:**
- nano
- Vim
- Neovim
- Emacs (terminal mode)

**Developer tools:**
- Rust stable toolchain (for recompiling Salata from source)
- Git
- `bat` (syntax-highlighted `cat` replacement)
- [Starship](https://starship.rs/) prompt

**Salata binaries (pre-built):**
- `salata` -- CLI interpreter
- `salata-cgi` -- CGI bridge
- `salata-fastcgi` -- FastCGI stub
- `salata-server` -- dev server

## Starting the playground

The playground ships with cross-platform start scripts that handle building the Docker image and launching the container.

**Linux / macOS:**

```bash
cd playground
./start-playground.sh
```

**Windows CMD:**

```cmd
cd playground
start-playground.bat
```

**PowerShell:**

```powershell
cd playground
.\start-playground.ps1
```

**Manual Docker Compose:**

If you prefer to use Docker Compose directly:

```bash
docker compose -f playground/docker-compose.playground.yml up -d
docker exec -it salata-playground-1 bash --login
```

> **Note:** The start scripts use `docker compose run` which gives you an interactive session directly. The `docker compose up -d` approach starts the container in the background, and you attach with `docker exec`.

### First run

The first time you start the playground, Docker builds the image. This takes a few minutes because it:

1. Installs all runtimes and tools from Ubuntu packages
2. Sets up Node.js via nodesource
3. Installs TypeScript tooling globally (`typescript`, `ts-node`, `tsx`)
4. Installs the Rust stable toolchain
5. Installs Starship prompt
6. Copies the Salata source and runs `cargo build --release`
7. Runs `salata init` to generate a starter project

Subsequent starts are nearly instant because the image is cached.

## The welcome banner

When you enter the container, a welcome banner displays:

```text
  ____    _    _        _  _____  _
 / ___|  / \  | |      / \|_   _|/ \
 \___ \ / _ \ | |     / _ \ | | / _ \
  ___) / ___ \| |___ / ___ \| |/ ___ \
 |____/_/   \_\_____/_/   \_\_/_/   \_\

 salata v0.1.0 — Polyglot Text Templating Engine

 Runtimes:
   Python ...... Python 3.12.3
   Ruby ........ ruby 3.2.2
   Node.js ..... v20.11.0
   TypeScript .. ts-node v10.9.2, tsx v4.7.0
   PHP ......... PHP 8.3.2
   Bash ........ GNU bash, version 5.2.26

 Quick start:
   salata index.slt              Process the starter file
   salata init --path mysite     Scaffold a new project
   salata-server . --port 3000   Start dev server (localhost:3000)
   cat index.slt                 See the starter file
```

The banner shows the exact versions of all installed runtimes, quick start commands, and a list of available examples.

## Directory layout inside the container

```text
/home/playground/              # Your working directory (HOME)
  ├── config.toml              # Pre-generated config (all runtimes enabled)
  ├── index.slt                # Starter template
  ├── errors/
  │   ├── 404.slt
  │   └── 500.slt
  ├── workspace/               # Bind-mounted to playground/workspace/ on host
  └── examples/                # Pre-loaded example projects
      ├── cli/
      │   ├── hello-world/
      │   ├── cross-runtime-pipeline/
      │   ├── scope-demo/
      │   ├── config-generator/
      │   ├── json-api-mock/
      │   ├── data-processing/
      │   ├── markdown-report/
      │   └── multi-format/
      └── web/
          └── ...

/opt/salata/                   # Salata source code (live mount from host)
/usr/local/bin/salata          # Pre-built salata binary
/usr/local/bin/salata-cgi      # Pre-built salata-cgi binary
/usr/local/bin/salata-fastcgi  # Pre-built salata-fastcgi binary
/usr/local/bin/salata-server   # Pre-built salata-server binary
/usr/local/bin/config.toml     # Symlink to /home/playground/config.toml
```

## Workspace persistence

The `playground/workspace/` directory on your host machine is bind-mounted to `/home/playground/workspace/` inside the container. Any files you create in the workspace directory persist across container restarts.

```bash
# Inside the container
cd workspace
salata init --path .
salata index.slt

# Files are visible on your host at playground/workspace/
```

This is the recommended place to put your own `.slt` projects while experimenting.

> **Tip:** Files created outside `/home/playground/workspace/` (except in `/opt/salata`) are ephemeral and will be lost when the container is removed.

## Port forwarding

Port 3000 is forwarded from the container to your host. This is used by `salata-server`:

```bash
# Inside the container
salata-server . --port 3000
```

Then open `http://localhost:3000` in your browser on your host machine. The dev server processes `.slt` files on the fly and serves static files (CSS, JS, images) as-is.

## Live source code

The Salata source code from your host repository is mounted at `/opt/salata` inside the container. This is a live bind mount -- changes you make to Rust source files on your host are immediately visible inside the container.

A named Docker volume (`cargo-target`) is used for the build artifacts so that the Linux build cache does not conflict with your host's (macOS/Windows) `target/` directory.

## Recompiling after code changes

If you modify the Salata source code (either on your host or inside the container at `/opt/salata`), use the `rebuild-salata` helper command to recompile and install the updated binaries:

```bash
rebuild-salata
```

This runs `cargo build --release` in `/opt/salata` and copies the four binaries to `/usr/local/bin/`. Output looks like:

```text
Rebuilding salata from /opt/salata ...

   Compiling salata-core v0.1.0 (/opt/salata/crates/salata-core)
   Compiling salata-cli v0.1.0 (/opt/salata/crates/salata-cli)
   ...
    Finished `release` profile [optimized] target(s) in 12.34s

Done! All 4 binaries installed:
salata v0.1.0
  salata-cgi, salata-fastcgi, salata-server
```

## Running the examples

The playground comes with pre-loaded examples in `~/examples/`. Each example has its own `config.toml` and one or more `.slt` files.

**Hello world (one file per runtime):**

```bash
salata --config examples/cli/hello-world/config.toml \
       examples/cli/hello-world/python.slt
# Output: Hello from Python!

salata --config examples/cli/hello-world/config.toml \
       examples/cli/hello-world/ruby.slt
# Output: Hello from Ruby!
```

**Cross-runtime pipeline (data flows Python -> Ruby -> JavaScript):**

```bash
salata --config examples/cli/cross-runtime-pipeline/config.toml \
       examples/cli/cross-runtime-pipeline/pipeline.slt
```

**List all available examples:**

```bash
ls examples/cli examples/web
```

## Scaffolding a new project

Use `salata init` inside the container to scaffold new projects:

```bash
# In the persistent workspace
cd workspace
salata init --path my-site
cd my-site
salata index.slt
```

Since all runtimes are available in the container, the generated `config.toml` will have all six runtimes enabled.

## Stopping the playground

If you used the start scripts (`start-playground.sh`, etc.), just type `exit` or press `Ctrl+D`. The container is removed automatically (the `--rm` flag is used).

If you started with `docker compose up -d`:

```bash
docker compose -f playground/docker-compose.playground.yml down
```

Your workspace files persist in `playground/workspace/` regardless of how you stop the container.

## Rebuilding the Docker image

If the Dockerfile changes (new runtimes, updated base image, etc.), rebuild the image:

```bash
docker compose -f playground/docker-compose.playground.yml build --no-cache
```

Or delete the image and let the start script rebuild it:

```bash
docker image rm salata-playground
./playground/start-playground.sh
```

## Next steps

- [Your First .slt File](./first-slt-file.md) -- step-by-step tutorial on `.slt` syntax
- [SLT Syntax](../guide/slt-syntax.md) -- complete syntax reference
- [Runtime Details](../guide/runtimes.md) -- language-specific behavior and configuration
