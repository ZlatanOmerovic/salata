# Your First .slt File

This chapter walks through creating, running, and understanding `.slt` template files step by step.

## What is an .slt file

An `.slt` file is a text file that can contain:

- **Plain text** -- passed through to the output unchanged (HTML, Markdown, JSON, anything)
- **Runtime blocks** -- code wrapped in language tags (`<python>...</python>`, `<ruby>...</ruby>`, etc.) that gets executed, with stdout captured and placed at the tag's position
- **Directives** -- instructions like `#include`, `#status`, and `#content-type` that control processing behavior
- **Macros** -- `#set` and `#get` calls inside runtime blocks for cross-runtime data sharing

## Step 1: A minimal .slt file

Create a file called `hello.slt` with one runtime block:

```html
<python>
print("Hello from Salata!")
</python>
```

Make sure you have a `config.toml` in the same directory (run `salata init` if you have not already). Then run it:

```bash
salata hello.slt
```

Output:

```text
Hello from Salata!
```

That is the entire flow. Salata found the `<python>` block, executed it with your system's Python interpreter, captured the `print()` output, and wrote it to stdout.

## Step 2: Mixing plain text and code

Runtime blocks can be mixed freely with plain text. Everything outside the tags passes through untouched:

```html
<!DOCTYPE html>
<html>
<head><title>My Page</title></head>
<body>
  <h1>Welcome</h1>
  <p>The current time is:
  <python>
from datetime import datetime
print(datetime.now().strftime("%Y-%m-%d %H:%M:%S"))
  </python>
  </p>
</body>
</html>
```

Output:

```html
<!DOCTYPE html>
<html>
<head><title>My Page</title></head>
<body>
  <h1>Welcome</h1>
  <p>The current time is:
  2026-02-22 14:30:45
  </p>
</body>
</html>
```

The HTML structure passes through unchanged. Only the `<python>` block is replaced by its output.

> **Note:** `<style>` and `<script>` tags are client-side HTML tags. Salata passes them through untouched -- they are not runtime blocks. Only `<python>`, `<ruby>`, `<javascript>`, `<typescript>`, `<php>`, and `<shell>` are Salata runtime tags.

## Step 3: Hello world in all six languages

Here is "Hello from [language]!" in each of the six supported runtimes. Each uses the language's native stdout mechanism:

**Python** (`hello-python.slt`):

```html
<python>
print("Hello from Python!")
</python>
```

**Ruby** (`hello-ruby.slt`):

```html
<ruby>
puts "Hello from Ruby!"
</ruby>
```

**JavaScript** (`hello-js.slt`):

```html
<javascript>
println("Hello from JavaScript!");
</javascript>
```

**TypeScript** (`hello-ts.slt`):

```html
<typescript>
const greeting: string = "Hello from TypeScript!";
println(greeting);
</typescript>
```

**PHP** (`hello-php.slt`):

```html
<php>
echo "Hello from PHP!\n";
</php>
```

**Shell** (`hello-shell.slt`):

```html
<shell>
echo "Hello from Shell!"
</shell>
```

> **Tip:** JavaScript and TypeScript get injected `print()` and `println()` helper functions. `print()` writes without a trailing newline (like `process.stdout.write()`), and `println()` adds a newline. These are additive -- `console.log()` still works as usual.

## Step 4: Output is not limited to HTML

Salata does not care what your code prints. The output format is determined entirely by the runtime blocks. Here is a `.slt` file that generates a JSON document:

```html
<python>
import json

data = {
    "name": "salata",
    "version": "0.1.0",
    "runtimes": ["python", "ruby", "javascript", "typescript", "php", "shell"]
}

print(json.dumps(data, indent=2))
</python>
```

Run it and redirect to a file:

```bash
salata api-response.slt > response.json
```

The resulting `response.json`:

```json
{
  "name": "salata",
  "version": "0.1.0",
  "runtimes": ["python", "ruby", "javascript", "typescript", "php", "shell"]
}
```

You can generate YAML, TOML, CSV, Markdown, nginx configs, Dockerfiles -- anything that can be represented as text.

## Step 5: Multiple runtimes in one file

The real power of Salata shows when you combine runtimes. Each runtime block executes in order, top to bottom:

```html
<python>
print("Step 1 (Python): Generating data...")
</python>

---

<ruby>
puts "Step 2 (Ruby): Processing..."
</ruby>

---

<javascript>
println("Step 3 (JavaScript): Formatting output.");
</javascript>
```

Output:

```text
Step 1 (Python): Generating data...

---

Step 2 (Ruby): Processing...

---

Step 3 (JavaScript): Formatting output.
```

Each block runs in its own runtime's interpreter. Python blocks run in Python, Ruby blocks in Ruby, and so on. The plain text between blocks (the `---` lines) passes through unchanged.

## Step 6: Sharing data between runtimes

Runtimes are isolated from each other -- a Python variable is not visible in Ruby. To pass data between runtimes, use the `#set` / `#get` macros:

```html
<python>
# Store data for other runtimes
items = ["apple", "banana", "cherry"]
#set("fruits", items)
#set("count", len(items))
</python>

<ruby>
# Retrieve data from Python
fruits = #get("fruits")
count = #get("count")

puts "Ruby received #{count} fruits:"
fruits.each { |f| puts "  - #{f}" }
</ruby>

<javascript>
// Retrieve the same data
const fruits = #get("fruits");
const count = #get("count");

println(`JavaScript confirms: ${count} fruits total.`);
println(`First fruit: ${fruits[0]}`);
</javascript>
```

Output:

```text
Ruby received 3 fruits:
  - apple
  - banana
  - cherry
JavaScript confirms: 3 fruits total.
First fruit: apple
```

Salata acts as the data broker. Values are JSON-serialized when stored with `#set` and deserialized back into native types when retrieved with `#get`. Strings, numbers, booleans, arrays/lists, objects/dicts, and null are all supported.

> **Note:** `#set` and `#get` are macros, not function calls. Salata expands them into runtime-specific native code before execution. They can only be used inside runtime blocks.

## Step 7: Using #get with defaults

The `#get` macro accepts an optional default value for when a key has not been set:

```html
<javascript>
const name = #get("username", "anonymous");
const theme = #get("theme", "dark");

println(`Welcome, ${name}! (theme: ${theme})`);
</javascript>
```

Output:

```text
Welcome, anonymous! (theme: dark)
```

If `"username"` or `"theme"` had been set by a previous runtime block, those values would be used instead.

## Step 8: Shared scope within a runtime

By default, all blocks of the same language share a single process. This means variables defined in one block are visible in later blocks of the same language:

```html
<python>
x = 42
</python>

<p>Some plain text in between.</p>

<python>
# x is still defined because both Python blocks share the same process
print(f"x is {x}")
</python>
```

Output:

```html

<p>Some plain text in between.</p>

x is 42
```

This is called **shared scope** and it is the default behavior. You can opt into isolated scope per runtime (via `config.toml`) or per block (via the `scope="isolated"` attribute) when you want blocks to run in separate processes.

## What to try next

Now that you understand the basics of `.slt` files, here are some things to explore:

- **Pipe to files:** `salata template.slt > output.html` to save the result
- **Use as a config generator:** Write a `.slt` file that generates nginx or Docker configs
- **Explore the examples:** The `examples/cli/` directory has ready-to-run examples covering hello-world, cross-runtime pipelines, scope demos, config generation, JSON API mocking, and more
- **Start the dev server:** `salata-server . --port 3000` serves `.slt` files over HTTP with hot reload

## Next steps

- [Playground Guide](./playground.md) -- try all runtimes in Docker
- [SLT Syntax](../guide/slt-syntax.md) -- complete syntax reference
- [Directives](../guide/directives.md) -- `#include`, `#status`, `#content-type`, and more
- [Macros (#set / #get)](../guide/macros.md) -- cross-runtime data sharing in depth
- [Scope (Shared vs Isolated)](../guide/scope.md) -- controlling process sharing
