# Single-File Web Examples

Demonstrates web-specific directives, each in its own `.slt` file.

## Files

- `hello.slt` — Basic HTML page with dynamic timestamp
- `status-codes.slt` — `#status 404` custom status page
- `redirect.slt` — `#redirect "/hello.slt"` HTTP redirect
- `headers.slt` — `#header` and `#cookie` directives
- `content-type.slt` — `#content-type application/json` returns JSON

## Run

```bash
salata-server . --port 3000
# Visit http://localhost:3000/hello.slt
```

## What It Demonstrates

- All web directives: `#status`, `#redirect`, `#header`, `#cookie`, `#content-type`
- Each file focuses on one directive for clarity
