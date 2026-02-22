# Salata Playground

A one-command interactive environment with all runtimes, editors, and pre-built
salata binaries ready to use.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) with
  [Compose v2](https://docs.docker.com/compose/install/)

## Quick Start

**Linux / macOS:**

```bash
./playground/start-playground.sh
```

**Windows CMD:**

```cmd
playground\start-playground.bat
```

**PowerShell:**

```powershell
playground/start-playground.ps1
```

The first run builds the Docker image (installs Rust, all runtimes, editors,
and compiles salata from source). Subsequent starts reuse the cached image.

## What's Included

### Runtimes

| Runtime    | Binary     |
|------------|------------|
| Python 3   | `python3`  |
| Ruby       | `ruby`     |
| Node.js    | `node`     |
| TypeScript | `ts-node`, `tsx` |
| PHP        | `php`, `php-cgi` |
| Bash       | `bash`     |
| Dash       | `dash`     |
| Zsh        | `zsh`      |
| Fish       | `fish`     |

### Editors

`nano`, `vim`, `neovim` (`nvim`), `emacs` (terminal mode)

### Salata Binaries

| Binary           | Description                           |
|------------------|---------------------------------------|
| `salata`         | Core interpreter (`.slt` to stdout)   |
| `salata-cgi`     | CGI bridge for nginx/Apache           |
| `salata-fastcgi` | FastCGI daemon (stub)                 |
| `salata-server`  | Standalone HTTP dev server            |

### Pre-generated Files

The playground starts with a ready-to-run project in `/home/playground`:

- `config.toml` — auto-detected runtime paths
- `index.slt` — starter template using Python
- `errors/404.slt` — custom 404 page
- `errors/500.slt` — custom 500 page
- `workspace/` — mounted from host for persisting files

## Usage Examples

Process the starter file:

```bash
salata index.slt
```

Start the dev server:

```bash
salata-server . --port 3000
```

Then open http://localhost:3000 in your browser on the host machine (port 3000
is forwarded from the container).

Create a new `.slt` file:

```bash
cat > hello.slt <<'EOF'
<!DOCTYPE html>
<html>
<body>
<python>
print('<h1>Hello from Python!</h1>')
</python>
<ruby>
puts '<p>Hello from Ruby!</p>'
</ruby>
<javascript>
println('<p>Hello from JavaScript!</p>');
</javascript>
</body>
</html>
EOF

salata hello.slt
```

## Persisting Files

The `playground/workspace/` directory on the host is mounted at
`/home/playground/workspace` inside the container. Files saved there persist
across playground sessions.

## Rebuilding

To rebuild the image after source changes:

```bash
docker compose -f playground/docker-compose.playground.yml build --no-cache
```
