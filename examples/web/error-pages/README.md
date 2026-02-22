# Custom Error Pages

Dynamic 404 and 500 error pages using `.slt` templates.

## Structure

```
error-pages/
├── index.slt          # Working page
├── errors/404.slt     # Custom 404 with dynamic timestamp
└── errors/500.slt     # Custom 500 with dynamic timestamp
```

## Run

```bash
salata-server . --port 3000
# Visit http://localhost:3000/index.slt       (working page)
# Visit http://localhost:3000/nonexistent.slt  (triggers 404)
```

## What It Demonstrates

- `[errors] page_404` and `page_500` config pointing to `.slt` files
- Error pages are dynamic — they can run Python, Shell, etc.
- `#status` directive in error templates
