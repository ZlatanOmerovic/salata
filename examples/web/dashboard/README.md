# Dashboard

A single-page dashboard using four runtimes and `#set/#get` for data sharing.

## Structure

```
dashboard/
├── index.slt          # Python metrics, Ruby tables, Shell info, JS SVG chart
└── static/style.css   # Dashboard grid styling
```

## Run

```bash
salata-server . --port 3000
# Visit http://localhost:3000/index.slt
```

## What It Demonstrates

- Four runtimes on one page: Python, Ruby, Shell, JavaScript
- `#set/#get` passes data between runtimes (Ruby sets page data, JS reads it)
- SVG generation from JavaScript
- CSS grid layout served as static file
