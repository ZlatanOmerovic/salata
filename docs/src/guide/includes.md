# Includes

The `#include` directive provides C-style text substitution: the contents of the referenced file are pasted directly in place of the directive. This enables reusable templates, shared layouts, and modular `.slt` file organization.

## Syntax

```text
#include "path/to/file.slt"
```

The path is relative to the file containing the `#include` directive.

## How It Works

When Salata encounters an `#include` directive, it reads the referenced file and inserts its entire contents at that position. This happens during the pre-processing phase, before any runtime blocks are executed.

Given this file structure:

```text
project/
  index.slt
  includes/
    header.slt
    footer.slt
```

**includes/header.slt:**

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>My Site</title>
  <style>
    body { font-family: sans-serif; margin: 2rem; }
  </style>
</head>
<body>
  <nav>
    <a href="/">Home</a>
    <a href="/about">About</a>
  </nav>
```

**includes/footer.slt:**

```html
  <footer>
    <p>
      <python>
        import datetime
        print(f"&copy; {datetime.datetime.now().year} My Site")
      </python>
    </p>
  </footer>
</body>
</html>
```

**index.slt:**

```html
#include "includes/header.slt"

  <h1>Welcome</h1>

  <python>
    print("<p>This is the home page.</p>")
  </python>

#include "includes/footer.slt"
```

After include resolution, Salata processes the combined document as if it were a single file. The Python block in `footer.slt` executes just like any other block in the page.

## Included Files Can Contain Anything

Included files can contain:

- Static text and HTML
- Runtime blocks (`<python>`, `<ruby>`, etc.)
- Directives (`#status`, `#content-type`, `#header`, etc.)
- Other `#include` directives (nested includes)

```html
<!-- includes/meta.slt -->
#content-type text/html; charset=utf-8
#header "X-Powered-By" "Salata"
```

```html
#include "includes/meta.slt"
#include "includes/header.slt"

<h1>Page with included meta directives</h1>

#include "includes/footer.slt"
```

## Shared Scope Across Includes

Runtime blocks in included files participate in shared scope. Variables defined in an included file are visible to blocks in the main file (and vice versa), as long as they are the same language and shared scope is active.

**includes/setup.slt:**

```html
<python>
  site_name = "My Site"
  version = "2.1.0"
</python>
```

**index.slt:**

```html
#include "includes/setup.slt"

<python>
  # site_name and version are available here because
  # both Python blocks share the same process.
  print(f"<h1>{site_name} v{version}</h1>")
</python>
```

This works because all Python blocks (regardless of which file they originate from) run in the same process under shared scope.

## Maximum Include Depth

Includes can be nested up to **16 levels** deep. This prevents infinite recursion from circular includes. If the depth limit is exceeded, Salata produces a clear error:

```text
Error: Maximum include depth (16) exceeded. Check for circular includes.
```

For example, this chain is valid (3 levels):

```text
index.slt
  -> includes/layout.slt
    -> includes/nav.slt
      -> includes/logo.slt
```

But a circular reference is caught and rejected:

```text
<!-- a.slt -->
#include "b.slt"

<!-- b.slt -->
#include "a.slt"
```

## Common Patterns

### Shared Layout

A typical pattern is to split your layout into header and footer includes:

```html
#include "includes/header.slt"

<!-- Page-specific content -->
<h1>About Us</h1>
<p>This is the about page.</p>

#include "includes/footer.slt"
```

### Reusable Components

Create reusable blocks that generate common UI elements:

**includes/user-table.slt:**

```html
<python>
  users = #get("users", [])
  if users:
      print("<table>")
      print("<tr><th>Name</th><th>Email</th></tr>")
      for u in users:
          print(f"<tr><td>{u['name']}</td><td>{u['email']}</td></tr>")
      print("</table>")
  else:
      print("<p>No users found.</p>")
</python>
```

**page.slt:**

```html
<python>
  #set("users", [
      {"name": "Alice", "email": "alice@example.com"},
      {"name": "Bob", "email": "bob@example.com"}
  ])
</python>

#include "includes/user-table.slt"
```

### Configuration Includes

Keep directives in a shared file to ensure consistent headers across pages:

**includes/security-headers.slt:**

```text
#header "X-Content-Type-Options" "nosniff"
#header "X-Frame-Options" "DENY"
#header "Referrer-Policy" "strict-origin-when-cross-origin"
```

```html
#include "includes/security-headers.slt"
#include "includes/header.slt"

<h1>Secure Page</h1>

#include "includes/footer.slt"
```
