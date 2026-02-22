# JSON API Mock

Generates a JSON API response using `#content-type`.

## Files

- `api.slt` — Python builds user data, JavaScript formats it as pretty-printed JSON

## Run

```bash
salata --config config.toml api.slt
```

## What It Demonstrates

- `#content-type application/json` directive
- Cross-runtime data: Python builds data, JavaScript formats with `JSON.stringify`
- Salata outputs structured data, not just HTML
