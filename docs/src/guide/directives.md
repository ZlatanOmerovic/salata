# Directives

Directives are pre-execution instructions that Salata resolves before any runtime blocks execute. They control HTTP response metadata, file inclusion, and routing behavior.

**Key rule:** Directives appear **outside** runtime blocks only. Placing a directive inside a `<python>`, `<ruby>`, or any other runtime block is invalid. For data sharing between runtime blocks, use [macros](./macros.md) instead.

## #include

C-style text substitution. The referenced file's contents are pasted in place of the directive.

**Syntax:**

```text
#include "path/to/file.slt"
```

**Rules:**

- Included files can contain runtime blocks, directives, and static text
- Included runtime blocks participate in shared scope (variables are visible)
- Maximum include depth: 16 levels (deeper recursion produces an error)
- Paths are relative to the file containing the `#include`

**Example:**

```html
#include "includes/header.slt"

<h1>Page Content</h1>

<python>
  print("<p>Dynamic content here.</p>")
</python>

#include "includes/footer.slt"
```

See [Includes](./includes.md) for detailed usage patterns.

## #status

Sets the HTTP response status code.

**Syntax:**

```text
#status CODE
```

**Rules:**

- Only once per page (multiple `#status` directives produce a parse error)
- Default: 200
- Any runtime failure automatically overrides this to 500
- No runtime block can set the status code -- only this directive can

**Examples:**

```html
#status 404

<h1>Page Not Found</h1>
<p>The requested page does not exist.</p>
```

```html
#status 201

<python>
  import json
  print(json.dumps({"created": True}))
</python>
```

## #content-type

Sets the response MIME type.

**Syntax:**

```text
#content-type MIME_TYPE
```

**Rules:**

- Only once per page (multiple produces a parse error)
- Default: `text/html; charset=utf-8`

**Examples:**

```html
#content-type application/json

<python>
  import json
  data = {"users": [{"name": "Alice"}, {"name": "Bob"}]}
  print(json.dumps(data, indent=2))
</python>
```

```html
#content-type text/plain

This is a plain text response.
<shell>echo "Generated at: $(date)"</shell>
```

```html
#content-type text/csv

<ruby>
  puts "name,email,age"
  puts "Alice,alice@example.com,30"
  puts "Bob,bob@example.com,25"
</ruby>
```

## #header

Adds a custom HTTP response header.

**Syntax:**

```text
#header "Header-Name" "value"
```

**Rules:**

- Can appear multiple times (each adds a header)
- Both the header name and value must be quoted

**Examples:**

```html
#header "X-Powered-By" "Salata"
#header "Cache-Control" "no-cache, no-store, must-revalidate"
#header "X-Request-Id" "abc-123"

<h1>Hello</h1>
```

## #cookie

Sets a response cookie.

**Syntax:**

```text
#cookie "name" "value" [flags...]
```

**Rules:**

- Can appear multiple times (each sets a different cookie)
- The cookie name and value must be quoted
- Optional flags follow the value, space-separated: `httponly`, `secure`, `samesite=Strict`, `samesite=Lax`, `samesite=None`, `path=/...`, `max-age=SECONDS`, `domain=...`

**Examples:**

```html
#cookie "session" "abc123" httponly secure

<h1>Welcome back</h1>
```

```html
#cookie "theme" "dark" path=/ max-age=31536000
#cookie "lang" "en" path=/ samesite=Lax

<python>
  print("<p>Preferences saved.</p>")
</python>
```

## #redirect

Issues an HTTP redirect response.

**Syntax:**

```text
#redirect "destination"
```

**Examples:**

```html
#redirect "/login"
```

```html
#redirect "/dashboard"
```

When a `#redirect` directive is present, the response body is typically empty since the client will follow the redirect.

## Summary

| Directive | Repeatable | Where | Purpose |
|-----------|-----------|-------|---------|
| `#include` | Yes | Outside blocks | Paste file contents in place |
| `#status` | No | Outside blocks | Set HTTP status code |
| `#content-type` | No | Outside blocks | Set response MIME type |
| `#header` | Yes | Outside blocks | Add custom response header |
| `#cookie` | Yes | Outside blocks | Set response cookie |
| `#redirect` | No | Outside blocks | Issue HTTP redirect |
