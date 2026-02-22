# Cross-Runtime Pipeline

The showcase example for `#set/#get` — data flows through three runtimes.

## Files

- `pipeline.slt` — Python generates sales data, Ruby aggregates it, JavaScript formats the output

## Run

```bash
salata --config config.toml pipeline.slt
```

## What It Demonstrates

- `#set("key", value)` stores data from one runtime
- `#get("key")` retrieves it in another runtime
- JSON serialization/deserialization is transparent
- Runtimes communicate through Salata as broker, never directly
- Heavily commented to explain each step
