# Runtimes

Salata ships with support for six server-side runtimes. Each runtime corresponds to a tag you embed in your `.slt` files. When Salata encounters a runtime tag, it executes the code inside it using the appropriate interpreter, captures whatever the code writes to stdout, and splices that output back into the document at the tag's position.

The output is not restricted to HTML. Runtimes can print JSON, plain text, YAML, CSV, or any other text format. What matters is that it goes to stdout.

## Supported Runtimes

| Language   | Tag              | Output Method                              | Notes                              |
|------------|------------------|--------------------------------------------|------------------------------------|
| Python     | `<python>`       | `print()`                                  | Python 3 required                  |
| Ruby       | `<ruby>`         | `puts`, `print`, `STDOUT.write`            |                                    |
| JavaScript | `<javascript>`   | `console.log()`, `print()`, `println()`    | `print()`/`println()` injected     |
| TypeScript | `<typescript>`   | `console.log()`, `print()`, `println()`    | `print()`/`println()` injected     |
| PHP        | `<php>`          | `echo`                                     | Context-aware binary selection     |
| Shell      | `<shell>`        | `echo`, `printf`                           | Sandboxed, hardcoded shell whitelist|

## Enabling and Disabling Runtimes

Every runtime has an `enabled` field in `config.toml` that defaults to `true`. You can disable any runtime you do not need:

```toml
[runtimes.ruby]
enabled = false
```

When a `.slt` file contains a tag for a disabled runtime, Salata produces a clear error message: `Runtime 'ruby' is disabled in config.toml`. If every runtime is disabled, Salata prints an informative message and exits with a non-zero status code.

## Shared Scope

By default, all blocks of the same language within a single `.slt` file run in one process. This means variables, functions, and state persist across blocks of the same language on the same page:

```html
<python>
name = "Alice"
</python>

<p>Some HTML in between.</p>

<python>
print(f"Hello, {name}!")
</python>
```

The second `<python>` block can access `name` because both blocks share the same Python process. You can disable shared scope globally per runtime (`shared_scope = false` in config) or per block (`scope="isolated"` attribute on the tag).

## Cross-Runtime Communication

Each language is isolated from every other language. A Python block cannot directly access a variable defined in a Ruby block. To pass data between runtimes, use the `#set` and `#get` macros:

```html
<python>
users = [{"name": "Alice"}, {"name": "Bob"}]
#set("users", users)
</python>

<javascript>
const users = #get("users");
println(`Found ${users.length} users`);
</javascript>
```

Salata acts as the broker. It expands the macros into native code for each language and handles JSON serialization and deserialization transparently. See the [Directives and Macros](../getting-started/directives-and-macros.md) chapter for full details.

## Encoding

UTF-8 is enforced everywhere: all input files, all runtime output, all final output. There is no option to change this.

## Configuration

Each runtime is configured under `[runtimes.<name>]` in `config.toml`. Common fields shared by all runtimes:

| Field            | Type   | Default | Description                                       |
|------------------|--------|---------|---------------------------------------------------|
| `enabled`        | bool   | `true`  | Whether this runtime is available                  |
| `path`           | string | varies  | Absolute path to the runtime binary                |
| `shared_scope`   | bool   | `true`  | Whether blocks share one process per page          |
| `display_errors` | bool   | (unset) | Override the global `display_errors` setting       |

PHP has additional fields for context-aware binary selection. See the [PHP runtime page](runtimes/php.md) for details.

## Runtime Pages

Each runtime has its own page with language-specific details, examples, and configuration:

- [Python](runtimes/python.md)
- [Ruby](runtimes/ruby.md)
- [JavaScript](runtimes/javascript.md)
- [TypeScript](runtimes/typescript.md)
- [PHP](runtimes/php.md)
- [Shell](runtimes/shell.md)
