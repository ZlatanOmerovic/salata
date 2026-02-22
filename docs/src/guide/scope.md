# Scope (Shared vs Isolated)

Salata supports two scope modes that control how runtime blocks of the same language interact: **shared scope** (the default) and **isolated scope**.

## Shared Scope (Default)

By default, all blocks of the same language run in a **single process**. Variables, functions, imports, and any other state defined in one block are visible to subsequent blocks of that language.

Under the hood, Salata concatenates all blocks of a given language and sends them to one process, separated by boundary markers (`__SALATA_BLOCK_BOUNDARY__`). The runtime executes the concatenated code sequentially, and Salata splits the captured output at the boundary markers to splice each block's output back into its correct position in the document.

### Example: Shared Scope

```html
<python>
  greeting = "Hello"
  count = 0
</python>

<p>Some static HTML in between.</p>

<python>
  # greeting and count are available here because both
  # Python blocks share the same process.
  count += 1
  print(f"<p>{greeting}, visitor #{count}!</p>")
</python>
```

Output:

```html

<p>Some static HTML in between.</p>

<p>Hello, visitor #1!</p>
```

The first block defines `greeting` and `count`. The second block can access and modify them because they run in the same Python process.

### Shared Scope Across Includes

Shared scope extends across `#include` boundaries. If a main file and its included files all contain Python blocks, those blocks all share one Python process:

```html
<!-- setup.slt -->
<python>
  app_name = "MyApp"
</python>
```

```html
<!-- index.slt -->
#include "setup.slt"

<python>
  print(f"<h1>Welcome to {app_name}</h1>")
</python>
```

The `app_name` variable from `setup.slt` is available in `index.slt` because both Python blocks run in the same process.

### Language Isolation

Shared scope applies **within** a language only. Different languages are always isolated from each other -- a Python block cannot see Ruby variables, and vice versa:

```html
<python>
  secret = "python-only"
</python>

<ruby>
  # `secret` is NOT available here. Ruby has its own process.
  # Use #set/#get macros for cross-language data sharing.
  puts "<p>Ruby cannot see Python variables.</p>"
</ruby>
```

To share data between languages, use the [`#set`/`#get` macro system](./macros.md).

## Isolated Scope

Isolated scope gives a block its own fresh process. No state carries over from previous blocks, and no state leaks to subsequent blocks.

There are two ways to enable isolated scope:

### Per-Block: The `scope` Attribute

Add `scope="isolated"` to the opening tag of any runtime block:

```html
<python>
  x = 42
  print(f"<p>x = {x}</p>")
</python>

<python scope="isolated">
  # This block runs in a separate, fresh Python process.
  # x is NOT defined here.
  try:
      print(f"<p>x = {x}</p>")
  except NameError:
      print("<p>x is not defined in this scope.</p>")
</python>

<python>
  # This block is back in the shared process.
  # x is still available from the first block.
  print(f"<p>x is still {x} in shared scope.</p>")
</python>
```

Output:

```html
<p>x = 42</p>

<p>x is not defined in this scope.</p>

<p>x is still 42 in shared scope.</p>
```

The `scope="isolated"` block gets a completely fresh environment. The shared-scope blocks (first and third) still share state with each other.

### Per-Runtime: Configuration

Set `shared_scope = false` in the runtime's configuration to make **all** blocks of that language use isolated scope:

```toml
[runtimes.python]
enabled = true
path = "/usr/bin/python3"
shared_scope = false
```

With this configuration, every `<python>` block runs in its own process. No state is shared between any Python blocks:

```html
<python>
  x = 100
  print(f"<p>x = {x}</p>")
</python>

<python>
  # x is NOT available here -- each block is isolated.
  # This will raise a NameError.
  print(f"<p>x = {x}</p>")
</python>
```

## When to Use Each Mode

### Use Shared Scope When:

- You want to define variables, functions, or imports once and reuse them across blocks
- You are building a page incrementally with multiple blocks of the same language
- You want included files to set up state that later blocks can use

### Use Isolated Scope When:

- A block should not be affected by (or affect) other blocks
- You need a clean environment for a specific computation
- You want to prevent variable name collisions between unrelated blocks
- You are running untrusted or experimental code that should be sandboxed from the rest

## How the Boundary Marker Works

For shared-scope blocks, Salata uses the marker `__SALATA_BLOCK_BOUNDARY__` to separate output from different blocks within a single process. The flow is:

1. Salata collects all shared-scope blocks for a given language
2. Between each block's code, Salata injects a print statement that outputs the boundary marker
3. The concatenated code is sent to one runtime process
4. The runtime executes all blocks sequentially, producing output with boundary markers between sections
5. Salata splits the output at the boundary markers
6. Each section of output is spliced back into the document at the corresponding block's position

This is transparent to you as a user -- you write individual blocks and Salata handles the concatenation and splitting behind the scenes.
