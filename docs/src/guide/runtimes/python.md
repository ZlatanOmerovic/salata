# Python Runtime

| Property       | Value                           |
|----------------|---------------------------------|
| Tag            | `<python>`                      |
| Output method  | `print()`                       |
| Default binary | `/usr/bin/python3`              |
| Shared scope   | `true` (default)                |

## Overview

The Python runtime executes code using Python 3. Whatever Python writes to stdout via `print()` is captured and placed at the tag's position in the output document. Python 2 is not supported.

## Output

Use `print()` to produce output. Each call to `print()` adds a newline by default. Use `end=""` to suppress it:

```html
<python>
print("<ul>")
for item in ["Apples", "Oranges", "Bananas"]:
    print(f"  <li>{item}</li>")
print("</ul>")
</python>
```

For finer control, you can also write directly to `sys.stdout`:

```python
import sys
sys.stdout.write("no trailing newline")
```

## Shared Scope

With shared scope enabled (the default), all `<python>` blocks on the same page run in a single Python process. Variables, imports, and function definitions persist across blocks:

```html
<python>
import json

def format_price(cents):
    return f"${cents / 100:.2f}"

products = [
    {"name": "Widget", "price": 1999},
    {"name": "Gadget", "price": 4550},
]
</python>

<h2>Product List</h2>

<python>
for p in products:
    print(f"<div>{p['name']}: {format_price(p['price'])}</div>")
</python>
```

Both blocks share the same process, so the second block can use `products` and `format_price` defined in the first.

## Data Processing

Python excels at data manipulation, making it a natural fit for processing data inline:

```html
<python>
import csv
import io

raw = """name,department,salary
Alice,Engineering,95000
Bob,Marketing,72000
Carol,Engineering,102000
Dave,Marketing,68000"""

reader = csv.DictReader(io.StringIO(raw))
rows = list(reader)

by_dept = {}
for row in rows:
    dept = row["department"]
    by_dept.setdefault(dept, []).append(row)

for dept, members in sorted(by_dept.items()):
    total = sum(int(m["salary"]) for m in members)
    avg = total / len(members)
    print(f"<h3>{dept}</h3>")
    print(f"<p>Headcount: {len(members)}, Average salary: ${avg:,.0f}</p>")
    print("<ul>")
    for m in members:
        print(f"  <li>{m['name']} - ${int(m['salary']):,}</li>")
    print("</ul>")
</python>
```

## Cross-Runtime Data Bridge

Python works well as a data source for other runtimes via `#set` and `#get`:

```html
<python>
stats = {
    "total_users": 1524,
    "active_today": 342,
    "conversion_rate": 0.067
}
#set("stats", stats)
</python>

<javascript>
const stats = #get("stats");
println(`<p>Conversion: ${(stats.conversion_rate * 100).toFixed(1)}%</p>`);
</javascript>
```

Salata serializes the Python dictionary to JSON and deserializes it into a JavaScript object transparently.

## Configuration

```toml
[runtimes.python]
enabled = true
path = "/usr/bin/python3"
shared_scope = true
display_errors = true
```

| Field            | Type   | Default              | Description                                     |
|------------------|--------|----------------------|-------------------------------------------------|
| `enabled`        | bool   | `true`               | Enable or disable the Python runtime            |
| `path`           | string | `/usr/bin/python3`   | Absolute path to the Python 3 binary            |
| `shared_scope`   | bool   | `true`               | All blocks share one process per page            |
| `display_errors` | bool   | (global fallback)    | Override the global `display_errors` setting     |

## Isolated Scope

To run a block in its own process, use the `scope` attribute:

```html
<python scope="isolated">
# This block has its own Python process.
# Variables from other blocks are not available here.
x = 42
print(x)
</python>
```

You can also set `shared_scope = false` in the config to make isolation the default for all Python blocks.

## Tips

- Python 3 is required. Ensure `path` points to a Python 3 interpreter.
- Use f-strings for readable output formatting.
- Heavy imports (like `pandas` or `numpy`) are fine but will increase execution time on the first block. With shared scope, the import cost is paid only once per page.
- The working directory during execution is the directory containing the `.slt` file being processed.
