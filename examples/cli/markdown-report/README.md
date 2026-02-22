# Markdown Report

Generates a project status report in pure Markdown format.

## Files

- `report.slt` — Python computes stats, Ruby formats a Markdown table, Shell adds build info

## Run

```bash
salata --config config.toml report.slt
salata --config config.toml report.slt > report.md
```

## What It Demonstrates

- Salata as a Markdown generator
- Three runtimes cooperating via `#set/#get`
- Data flows: Python (compute) -> Ruby (format) -> Shell (metadata)
