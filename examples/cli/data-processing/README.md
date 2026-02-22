# Data Processing

Demonstrates Salata as a data processing tool — not just for HTML.

## Files

- `csv-table.slt` — Python parses inline CSV data into a formatted ASCII table
- `json-filter.slt` — Ruby filters and sorts a JSON array, outputs formatted text
- `system-report.slt` — Shell gathers system info (hostname, disk, memory)

## Run

```bash
salata --config config.toml csv-table.slt
salata --config config.toml json-filter.slt
salata --config config.toml system-report.slt
```

## What It Demonstrates

- Salata outputs any text format, not just HTML
- Each runtime's strengths: Python for CSV, Ruby for JSON, Shell for system info
- Only the runtimes used are enabled in config
