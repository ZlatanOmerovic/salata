# Runtime Blocks

Salata supports six server-side runtime blocks. Each block is delimited by its opening and closing tag, and the code inside is executed by the corresponding language runtime. The stdout output of each block replaces the block in the final output.

## Python

```html
<python>
  name = "world"
  print(f"<h1>Hello, {name}!</h1>")
</python>
```

Python blocks are executed by the Python 3 interpreter (configured via `runtimes.python.path`). Use `print()` for output. Multiple `print()` calls accumulate output as expected.

## Ruby

```html
<ruby>
  items = %w[apple banana cherry]
  items.each do |item|
    puts "<li>#{item}</li>"
  end
</ruby>
```

Ruby blocks are executed by the Ruby interpreter (configured via `runtimes.ruby.path`). Use `puts` for output with a trailing newline, or `print` for output without one.

## JavaScript

```html
<javascript>
  const colors = ["red", "green", "blue"];
  colors.forEach(c => {
    println(`<span style="color:${c}">${c}</span>`);
  });
</javascript>
```

JavaScript blocks are executed by Node.js (configured via `runtimes.javascript.path`). You have several output options:

- `console.log()` -- standard Node.js, appends a newline
- `process.stdout.write()` -- standard Node.js, no newline
- `print()` -- Salata-injected helper, no newline (equivalent to `process.stdout.write()` with space-joined arguments)
- `println()` -- Salata-injected helper, appends a newline

The `print()` and `println()` helpers are **additive** -- they do not override or replace `console.log` or any other built-in. They are injected before your code runs for convenience.

## TypeScript

```html
<typescript>
  interface User {
    name: string;
    age: number;
  }
  const user: User = { name: "Alice", age: 30 };
  println(`<p>${user.name} is ${user.age} years old.</p>`);
</typescript>
```

TypeScript blocks work the same as JavaScript blocks, with the same `print()`/`println()` helpers injected. The TypeScript runner is configurable -- you can use `ts-node`, `tsx`, `bun`, or `deno` by setting the `runtimes.typescript.path` in your config.

## PHP

```html
<php>
  $items = ["one", "two", "three"];
  foreach ($items as $item) {
      echo "<li>$item</li>\n";
  }
</php>
```

PHP blocks use `echo` for output. PHP is context-aware: the binary used depends on the execution context:

| Context | Binary |
|---------|--------|
| CLI (`salata`) | `php` (via `cli_path`) |
| CGI (`salata-cgi`) | `php-cgi` (via `cgi_path`) |
| FastCGI / Server | `php-fpm` (via `fastcgi_socket` or `fastcgi_host`) |

See [PHP runtime configuration](./runtime-config.md) for details on configuring the PHP paths.

## Shell

```html
<shell>
  echo "System uptime:"
  uptime
</shell>
```

Shell blocks are executed by the configured shell (default: `/bin/bash`). Use `echo` for output. Shell is the **most restricted** runtime -- it runs inside a sandbox with:

- A hardcoded whitelist of allowed shell paths
- Pre-execution scanning for blocked commands and patterns
- A cleaned environment with stripped variables
- Timeout, memory, and output size limits

See the [Shell Sandbox](../security/shell-sandbox.md) documentation for full details.

## Output Methods Summary

| Runtime | Primary Output | Alternatives |
|---------|---------------|-------------|
| Python | `print()` | `sys.stdout.write()` |
| Ruby | `puts` | `print`, `$stdout.write()` |
| JavaScript | `console.log()` | `print()`, `println()`, `process.stdout.write()` |
| TypeScript | `console.log()` | `print()`, `println()`, `process.stdout.write()` |
| PHP | `echo` | `print`, `printf()` |
| Shell | `echo` | `printf` |

## Nesting Rules

Runtime tags **cannot** be nested inside other runtime tags. The following is a parse-time error:

```html
<!-- INVALID: nested runtime tags -->
<python>
  print("<javascript>console.log('no')</javascript>")
</python>
```

Salata detects this during parsing and rejects the file before any execution occurs. The output of `print()` is treated as plain text, not parsed for additional runtime tags.

To pass data between runtimes, use the [`#set`/`#get` macro system](./macros.md).

## The `scope` Attribute

By default, all blocks of the same language share a single process (shared scope). You can opt a specific block out of shared scope by adding the `scope="isolated"` attribute to its opening tag:

```html
<python>
  x = 42
  print(f"x = {x}")
</python>

<python scope="isolated">
  # This block runs in a fresh process.
  # The variable x from the previous block is NOT available here.
  print("This is isolated.")
</python>
```

See [Scope (Shared vs Isolated)](./scope.md) for a full explanation of scope behavior.

## Automatic Dedenting

Code inside runtime blocks is automatically dedented before execution. Salata strips the common leading whitespace from all lines in a block. This allows you to indent code naturally within your document structure:

```html
<div>
  <ul>
    <python>
      for i in range(5):
          print(f"<li>Item {i}</li>")
    </python>
  </ul>
</div>
```

The Python code is dedented so the `for` statement starts at column 0 from Python's perspective, avoiding indentation errors.
