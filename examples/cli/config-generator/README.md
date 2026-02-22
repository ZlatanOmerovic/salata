# Config Generator

Generates a valid nginx.conf using Python and Shell together.

## Files

- `nginx.slt` — Python defines upstream servers, Shell detects CPU cores, Python assembles the config

## Run

```bash
salata --config config.toml nginx.slt > nginx.conf
```

## What It Demonstrates

- Generating config files (not HTML) with Salata
- Cross-runtime data flow: Shell sets CPU cores, Python reads it via `#set/#get`
- Practical use case: infrastructure config generation
