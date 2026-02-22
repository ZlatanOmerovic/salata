# Quick Start

This guide gets you from zero to running Salata output in under 5 minutes. Choose the path that suits your setup.

## Option A: Docker Playground (fastest)

The playground is a Docker container with all six runtimes, editors, and pre-built Salata binaries. No local Rust toolchain needed.

**Prerequisites:** Docker and Docker Compose v2.

### 1. Start the playground

From the Salata repository root:

```bash
cd playground
./start-playground.sh
```

On Windows CMD:

```cmd
cd playground
start-playground.bat
```

On PowerShell:

```powershell
cd playground
.\start-playground.ps1
```

The first run builds the Docker image, which takes a few minutes (installing runtimes, compiling Salata). Subsequent runs start instantly.

### 2. You are inside the container

When the container starts, you will see a welcome banner showing all detected runtimes and their versions. You land in `/home/playground` with a pre-generated `config.toml` and `index.slt`.

### 3. Run the starter file

```bash
salata index.slt
```

Expected output:

```html
<!DOCTYPE html>
<html>
<head><title>Salata</title></head>
<body>
<h1>Hello from Salata!</h1>
</body>
</html>
```

### 4. Try an example

The playground comes with pre-loaded examples:

```bash
salata --config examples/cli/hello-world/config.toml \
       examples/cli/hello-world/python.slt
```

Output:

```text
Hello from Python!
```

### 5. Start the dev server

```bash
salata-server . --port 3000
```

Open `http://localhost:3000` in your browser (port 3000 is forwarded from the container to your host).

---

## Option B: Local build

Build Salata from source and run it directly on your machine.

**Prerequisites:** Rust toolchain (`rustup`), at least one runtime installed (Python, Ruby, Node.js, PHP, or Bash).

### 1. Clone and build

```bash
git clone https://github.com/nicholasgasior/salata.git
cd salata
cargo build --release
```

### 2. Initialize a project

```bash
./target/release/salata init
```

This detects your installed runtimes, generates `config.toml`, and creates `index.slt` plus error page templates. You will see output like:

```text
Detecting runtimes...
  python         /usr/bin/python3  (Python 3.12.3)
  ruby           not found — will be disabled
  javascript     /usr/local/bin/node  (v20.11.0)
  typescript     /usr/local/bin/tsx  (tsx v4.7.0)
  php            not found — will be disabled
  php-cgi        not found — will be disabled
  shell          /bin/bash  (GNU bash, version 5.2.26)
Created config.toml with 4 of 6 runtimes enabled.
Run: salata index.slt
```

### 3. Run the starter file

```bash
./target/release/salata index.slt
```

Expected output (varies depending on which runtime was detected first):

```html
<!DOCTYPE html>
<html>
<head><title>Salata</title></head>
<body>
<h1>Hello from Salata!</h1>
</body>
</html>
```

### 4. Write your own template

Create a file called `hello.slt`:

```html
<python>
print("Hello from Salata!")
print("1 + 1 =", 1 + 1)
</python>
```

Run it:

```bash
./target/release/salata hello.slt
```

Output:

```text
Hello from Salata!
1 + 1 = 2
```

### 5. Pipe output to a file

Salata writes to stdout, so you can redirect output anywhere:

```bash
./target/release/salata hello.slt > output.txt
./target/release/salata template.slt > config.yml
./target/release/salata report.slt | less
```

---

## What just happened

When you run `salata your-file.slt`, the following pipeline executes:

1. Salata reads the `.slt` file
2. Resolves any `#include` directives (text substitution)
3. Parses the content, finding runtime blocks (`<python>`, `<ruby>`, etc.) and plain text
4. Expands `#set` / `#get` macros into native code for each runtime
5. Executes each runtime block in its native interpreter, capturing stdout
6. Splices the captured output back into the document in place of the runtime tags
7. Writes the final result to stdout

Plain text (including HTML, CSS, and anything outside runtime blocks) passes through untouched. Only the content inside runtime tags gets executed and replaced.

## Next steps

- [Your First .slt File](./first-slt-file.md) -- a step-by-step tutorial on `.slt` syntax
- [Playground Guide](./playground.md) -- full details on the Docker playground environment
- [SLT Syntax](../guide/slt-syntax.md) -- complete syntax reference
