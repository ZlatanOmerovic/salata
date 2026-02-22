# Macros (#set / #get)

Macros are Salata's cross-runtime data bridge. They allow runtime blocks written in different languages to share data with each other. Unlike [directives](./directives.md), macros work **inside** runtime blocks only.

## Overview

- `#set("key", value)` -- store a value under a named key
- `#get("key")` -- retrieve a value by key
- `#get("key", default)` -- retrieve a value with a fallback default

Salata expands these macros into native code for each language **before** execution. The runtimes never communicate directly -- Salata acts as the broker, serializing data to JSON and deserializing it back into native types.

## #set

Stores a value under a named key. The value is JSON-serialized automatically.

**Syntax (inside a runtime block):**

```text
#set("key", value)
```

**Examples:**

```html
<python>
  users = [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]
  #set("users", users)
  #set("user_count", len(users))
</python>
```

```html
<ruby>
  config = { "debug" => false, "version" => "1.0" }
  #set("app_config", config)
</ruby>
```

## #get

Retrieves a previously stored value by key. The JSON data is deserialized into the native type for the receiving language.

**Syntax (inside a runtime block):**

```text
#get("key")
#get("key", default)
```

If the key does not exist and no default is provided, the result is `null` (or the language's equivalent: `None` in Python, `nil` in Ruby, `null` in JavaScript/PHP).

**Example with default:**

```html
<javascript>
  const count = #get("user_count", 0);
  println(`There are ${count} users.`);
</javascript>
```

## Supported Types

The macro system handles these types transparently across all runtimes:

| Type | Python | Ruby | JavaScript | TypeScript | PHP |
|------|--------|------|------------|------------|-----|
| String | `str` | `String` | `string` | `string` | `string` |
| Number (int) | `int` | `Integer` | `number` | `number` | `int` |
| Number (float) | `float` | `Float` | `number` | `number` | `float` |
| Boolean | `bool` | `TrueClass`/`FalseClass` | `boolean` | `boolean` | `bool` |
| Array/List | `list` | `Array` | `Array` | `Array` | `array` |
| Object/Dict | `dict` | `Hash` | `Object` | `Object` | `array` (assoc) |
| Null | `None` | `nil` | `null` | `null` | `null` |

## How It Works

When Salata encounters `#set` or `#get` in a runtime block, it expands them into native code before passing the block to the runtime for execution.

For example, a `#set("users", users)` in a Python block might be expanded into code that JSON-serializes the `users` variable and writes it to a temporary file. A subsequent `#get("users")` in a JavaScript block would be expanded into code that reads that file and parses the JSON back into a native JavaScript object.

The key points:

1. **Expansion happens before execution** -- the runtime sees native code, not macro syntax
2. **JSON is the interchange format** -- data is serialized to JSON by the setter and deserialized by the getter
3. **Salata is the broker** -- runtimes never communicate directly with each other
4. **Data is stored as JSON files** in a temporary directory managed by Salata

## Cross-Runtime Example

A common pattern is to generate data in one language and consume it in another:

```html
<python>
  import datetime

  report = {
      "title": "Monthly Report",
      "generated": datetime.datetime.now().isoformat(),
      "items": [
          {"name": "Revenue", "value": 50000},
          {"name": "Expenses", "value": 32000},
          {"name": "Profit", "value": 18000}
      ]
  }
  #set("report", report)
</python>

<ruby>
  report = #get("report")
  puts "<h1>#{report['title']}</h1>"
  puts "<p>Generated: #{report['generated']}</p>"
</ruby>

<javascript>
  const report = #get("report");
  println("<table>");
  println("<tr><th>Metric</th><th>Value</th></tr>");
  for (const item of report.items) {
    println(`<tr><td>${item.name}</td><td>$${item.value.toLocaleString()}</td></tr>`);
  }
  println("</table>");
</javascript>
```

In this example:

1. Python creates the report data and stores it with `#set`
2. Ruby retrieves the data and renders the heading
3. JavaScript retrieves the same data and renders the table

Each runtime gets the data as its native type -- Python `dict` becomes Ruby `Hash` becomes JavaScript `Object`.

## Multi-Step Pipeline

You can chain data through multiple runtimes, with each step transforming and re-storing:

```html
<python>
  raw_data = [5, 3, 8, 1, 9, 2, 7]
  #set("numbers", raw_data)
</python>

<ruby>
  numbers = #get("numbers")
  sorted = numbers.sort
  #set("sorted_numbers", sorted)
  puts "<p>Sorted: #{sorted.join(', ')}</p>"
</ruby>

<javascript>
  const sorted = #get("sorted_numbers");
  const sum = sorted.reduce((a, b) => a + b, 0);
  const avg = sum / sorted.length;
  println(`<p>Average: ${avg.toFixed(2)}</p>`);
</javascript>
```

## Known Limitations

**Shell blocks and #set/#get:** The macro expansion generates parenthesized function-call syntax (e.g., `#set("key", value)`), which is not valid shell syntax. Shell uses space-separated arguments, not parenthesized calls. If you need to pass data to or from shell blocks, use another runtime as an intermediary:

```html
<python>
  #set("greeting", "Hello from Python")
</python>

<!-- This works: Python sets, JavaScript gets -->
<javascript>
  const msg = #get("greeting");
  println(msg);
</javascript>

<!-- Avoid: #set/#get in shell blocks will produce syntax errors -->
```

Use Python, Ruby, JavaScript, TypeScript, or PHP for data sharing. Shell blocks are best used for standalone commands that do not need cross-runtime data.
