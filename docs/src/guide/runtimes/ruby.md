# Ruby Runtime

| Property       | Value                           |
|----------------|---------------------------------|
| Tag            | `<ruby>`                        |
| Output method  | `puts`, `print`, `STDOUT.write` |
| Default binary | `/usr/bin/ruby`                 |
| Shared scope   | `true` (default)                |

## Overview

The Ruby runtime executes code using the system Ruby interpreter. Whatever Ruby writes to stdout is captured and placed at the tag's position in the output document.

## Output

Ruby provides several ways to write to stdout:

- `puts` -- writes a string followed by a newline
- `print` -- writes a string with no trailing newline
- `STDOUT.write` -- writes raw bytes to stdout, returns the number of bytes written
- `$stdout.write` -- equivalent to `STDOUT.write`

```html
<ruby>
puts "<h1>Hello from Ruby</h1>"
print "<p>This has "
print "no newline between parts.</p>"
</ruby>
```

## Shared Scope

With shared scope enabled (the default), all `<ruby>` blocks on the same page run in a single Ruby process. Variables, methods, and classes persist across blocks:

```html
<ruby>
def greet(name)
  "Hello, #{name}!"
end

colors = %w[red green blue]
</ruby>

<div class="content">

<ruby>
colors.each do |color|
  puts %(<span style="color: #{color}">#{greet(color)}</span>)
end
</ruby>
```

## String Formatting and Interpolation

Ruby's string interpolation and formatting methods make it well-suited for generating text output:

```html
<ruby>
products = [
  { name: "Espresso", price: 3.50, stock: 42 },
  { name: "Latte",    price: 4.75, stock: 18 },
  { name: "Mocha",    price: 5.25, stock: 7  },
]

puts "<table>"
puts "  <tr><th>Product</th><th>Price</th><th>Status</th></tr>"
products.each do |p|
  status = p[:stock] < 10 ? "Low stock" : "Available"
  puts "  <tr>"
  puts "    <td>#{p[:name]}</td>"
  puts "    <td>#{'$%.2f' % p[:price]}</td>"
  puts "    <td>#{status}</td>"
  puts "  </tr>"
end
puts "</table>"
</ruby>
```

## Text Manipulation

Ruby's built-in string and enumerable methods are effective for transforming text:

```html
<ruby>
raw_text = <<~TEXT
  the quick brown fox
  jumps over the lazy dog
  pack my box with five dozen liquor jugs
TEXT

sentences = raw_text.strip.lines.map do |line|
  words = line.strip.split
  words.map.with_index { |w, i| i == 0 ? w.capitalize : w }.join(" ") + "."
end

puts "<ul>"
sentences.each { |s| puts "  <li>#{s}</li>" }
puts "</ul>"

word_count = raw_text.split.size
char_count = raw_text.gsub(/\s+/, "").size
puts "<p>Words: #{word_count}, Characters: #{char_count}</p>"
</ruby>
```

## Cross-Runtime Data Bridge

Ruby integrates with other runtimes through the `#set` and `#get` macros:

```html
<python>
menu = [
    {"dish": "Ramen", "cuisine": "Japanese", "spicy": True},
    {"dish": "Pad Thai", "cuisine": "Thai", "spicy": True},
    {"dish": "Risotto", "cuisine": "Italian", "spicy": False},
]
#set("menu", menu)
</python>

<ruby>
menu = #get("menu")
spicy = menu.select { |item| item["spicy"] }
puts "<h3>Spicy Dishes</h3>"
puts "<ul>"
spicy.each { |item| puts "  <li>#{item['dish']} (#{item['cuisine']})</li>" }
puts "</ul>"
</ruby>
```

## Configuration

```toml
[runtimes.ruby]
enabled = true
path = "/usr/bin/ruby"
shared_scope = true
```

| Field            | Type   | Default            | Description                                     |
|------------------|--------|--------------------|-------------------------------------------------|
| `enabled`        | bool   | `true`             | Enable or disable the Ruby runtime              |
| `path`           | string | `/usr/bin/ruby`    | Absolute path to the Ruby binary                |
| `shared_scope`   | bool   | `true`             | All blocks share one process per page            |
| `display_errors` | bool   | (global fallback)  | Override the global `display_errors` setting     |

## Isolated Scope

To run a block in its own process:

```html
<ruby scope="isolated">
# This block has its own Ruby process.
x = 99
puts x
</ruby>
```

Or set `shared_scope = false` in the config for all Ruby blocks.

## Tips

- Ruby's heredocs (`<<~HEREDOC`) are useful for multi-line template strings.
- Use `Struct` or hashes for lightweight data structures within templates.
- ERB is not involved -- Salata handles the template layer. Ruby blocks are plain Ruby code.
- The working directory during execution is the directory containing the `.slt` file being processed.
