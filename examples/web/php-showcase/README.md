# PHP Showcase

PHP and Python running side-by-side on the same page.

## Files

- `index.slt` — PHP handles string/date formatting, Python handles computation

## Run

```bash
salata-server . --port 3000
# Visit http://localhost:3000/index.slt
```

## What It Demonstrates

- PHP and Python coexisting in one `.slt` file
- PHP's string functions (`str_word_count`, `base64_encode`, `date`)
- Python's math capabilities alongside PHP
- Each runtime does what it's best at
