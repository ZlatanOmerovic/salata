# Mini Blog

A lightweight blog where Python reads post files and Ruby formats them.

## Structure

```
blog/
├── posts/
│   ├── hello-world.txt       # Raw post with title/date header
│   ├── salata-guide.txt
│   └── tips-and-tricks.txt
├── includes/
│   ├── header.slt            # Shared header
│   └── footer.slt            # Footer with Ruby date
├── index.slt                 # Post listing page
└── static/style.css          # Blog typography
```

## Run

```bash
salata-server . --port 3000
# Visit http://localhost:3000/index.slt
```

## What It Demonstrates

- Python reads files from the filesystem at request time
- Data passed to Ruby via `#set/#get` for formatting
- `#include` for shared layout
- File-based content — no database needed
