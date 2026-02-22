# Portfolio Site

A multi-page portfolio site using `#include` for shared layout and static CSS.

## Structure

```
portfolio/
├── includes/header.slt    # Shared navigation + head
├── includes/footer.slt    # Shared footer with dynamic year
├── index.slt              # Project listing (Python)
├── about.slt              # Skills page (Ruby)
├── contact.slt            # Contact info (Python)
└── static/style.css       # Served as-is by salata-server
```

## Run

```bash
salata-server . --port 3000
# Visit http://localhost:3000/index.slt
```

## What It Demonstrates

- `#include "file.slt"` for shared layout partials
- Static file serving (CSS)
- Multiple pages sharing the same header/footer
- Python and Ruby on different pages
