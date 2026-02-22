# Directives & Macros — Salata

## Directives

Pre-execution instructions resolved before any runtime blocks execute. Pattern: `#directive value`. Appear anywhere in the document but NOT inside runtime blocks.

### #include
C-style text substitution. `#include "file.slt"` pastes file contents in place.
- Included files can contain language blocks and directives
- Included blocks participate in shared scope
- Max depth: 16 levels (error on deeper recursion)

### #status
HTTP response status code. `#status 404`. Once per page (multiple = parse error). Default: 200.
- Any runtime failure → automatic 500 (overrides this)
- No runtime can set status — only this directive

### #content-type
Response MIME type. `#content-type application/json`. Once per page. Default: `text/html; charset=utf-8`.

### #header
Custom response header. `#header "X-Custom" "value"`. Can appear multiple times.

### #cookie
Response cookie. `#cookie "session" "abc123" httponly secure`. Can appear multiple times.

### #redirect
HTTP redirect. `#redirect "/other-page"`.

## Macros — Cross-Runtime Data Bridge

Work inside runtime blocks ONLY. Salata expands them into native code per language.

### #set("key", value)
Store data. Value is JSON-serialized automatically by the runtime.

### #get("key") / #get("key", default)
Retrieve data. Returns deserialized native type. Default fallback is null.

### Example

```html
<python>
  users = [{"name": "Alice"}, {"name": "Bob"}]
  #set("users", users)
  #set("count", len(users))
</python>

<javascript>
  const users = #get("users");
  const count = #get("count");
  print(`Found ${count} users`);
</javascript>

<ruby>
  users = #get("users")
  count = #get("count", 0)
  puts users.first["name"]
</ruby>
```

### How It Works
- Salata expands macros into runtime-specific native code before execution
- JSON serialization/deserialization is transparent
- Runtimes communicate through salata (the broker), never directly
- Supported types: strings, numbers, booleans, arrays/lists, objects/dicts, null

### Known Limitation: Shell Macros

`#set`/`#get` in `<shell>` blocks currently produces invalid syntax. The macro expansion does a simple text replacement (`#set(` → `__salata_set(`), which produces `__salata_set("key", "val")` — valid in Python/Ruby/JS/TS/PHP but not in shell, where function calls use space-separated arguments without parentheses. Shell expansion needs to produce `__salata_set "key" "val"` instead. Until this is fixed, use other runtimes for `#set`/`#get` and keep shell blocks for output-only tasks.
