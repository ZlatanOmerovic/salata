# Multi-Format Output

Same inventory data rendered as three different formats.

## Files

- `report.txt.slt` — Plain text with aligned columns
- `report.csv.slt` — CSV output
- `report.yaml.slt` — YAML output

## Run

```bash
salata --config config.toml report.txt.slt
salata --config config.toml report.csv.slt > inventory.csv
salata --config config.toml report.yaml.slt > inventory.yaml
```

## What It Demonstrates

- Salata is format-agnostic — the output is whatever your code prints
- Same data source, three different output formats
- Practical use: generate reports in the format consumers need
