# Examples — Salata

## Overview

Comprehensive examples organized into CLI and web categories. Every example is self-contained with its own `config.toml` and `README.md`.

## examples/cli/

Run with `salata <file.slt>`. Output is text to stdout — not necessarily HTML.

### hello-world/
One `.slt` file per runtime. The simplest possible example: `python.slt`, `ruby.slt`, `javascript.slt`, `typescript.slt`, `php.slt`, `shell.slt`. Each prints "Hello from <language>".

### data-processing/
- Python reads a CSV string, processes rows, outputs a formatted text table
- Ruby parses inline JSON, filters and sorts, outputs formatted text
- Shell gathers system info (uname, uptime, df), outputs a system report

### config-generator/
A `.slt` that generates an nginx.conf file. Python computes upstream server addresses, Shell detects system CPU count for worker_processes. Output is a valid nginx config file.

### markdown-report/
A `.slt` that outputs Markdown. Python computes statistics, Ruby formats Markdown tables, Shell grabs git log info. Demonstrates salata as a general text templating tool.

### cross-runtime-pipeline/
The showcase example for `#set`/`#get`. A single `.slt` file that passes data through 3 runtimes:
1. Python creates raw sales data and `#set()`s it
2. Ruby reads it, aggregates by product, `#set()`s the result
3. JavaScript reads aggregated data and formats the final output

Clear comments explain each step and the data flow.

### scope-demo/
Two `.slt` files demonstrating the difference:
- `shared-scope.slt` — default behavior, variables persist between blocks of same language
- `isolated-scope.slt` — `scope="isolated"` attribute, each block gets a fresh process

### json-api-mock/
A `.slt` with `#content-type application/json` that outputs JSON. Python builds the data structure, JavaScript formats it with `JSON.stringify` with indentation. Shows salata can produce API responses.

### multi-format/
Three `.slt` files producing the same inventory data in different formats:
- `report.txt.slt` — plain text with aligned columns
- `report.csv.slt` — CSV format
- `report.yaml.slt` — YAML format

Demonstrates salata is completely format-agnostic.

## examples/web/

Serve with `salata-server . --port 3000`. Output is HTML with web directives.

### single-file/
Individual `.slt` files demonstrating web-specific features:
- `hello.slt` — basic HTML page with a runtime block
- `status-codes.slt` — `#status 404` usage
- `redirect.slt` — `#redirect "/other-page"` usage
- `headers.slt` — `#header` and `#cookie` usage
- `content-type.slt` — `#content-type application/json`, returns JSON

### portfolio/
A mini-site with multiple pages:
- `index.slt` — home page
- `about.slt` — about page
- `contact.slt` — contact page
- `includes/header.slt` — shared header via `#include`
- `includes/footer.slt` — shared footer via `#include`
- `static/style.css` — served as-is by salata-server
- Python generates a project list, Ruby formats dates

### dashboard/
A mini-site with a single-page dashboard:
- `index.slt` — dashboard layout
- `static/style.css` — dashboard styling
- Python queries (simulated) metrics and `#set()`s them
- Ruby reads metrics and formats HTML tables
- Shell grabs server stats (uptime, memory, disk) and `#set()`s them
- JavaScript reads all data and generates inline SVG sparklines

Shows all runtimes cooperating via `#set`/`#get` on a single page.

### php-showcase/
Demonstrates PHP working naturally in salata:
- PHP for string functions (`str_word_count`, `base64_encode`) and date formatting
- Python for mathematical computation (primes, pi) alongside PHP
- Shows PHP is a first-class citizen, not an afterthought

### api-endpoint/
A `.slt` that acts as a JSON API endpoint:
- `#content-type application/json`
- `#status 200`
- Python builds the response data structure
- Shows salata can serve APIs, not just rendered pages

### error-pages/
Custom error pages with dynamic content:
- `errors/404.slt` — Shell prints timestamp, Python formats a friendly "not found" message
- `errors/500.slt` — Shell prints timestamp, Python formats an error message
- `config.toml` points to these as custom error pages

### blog/
A realistic mini-site:
- `index.slt` — lists all posts with titles, dates, and content
- `posts/` — directory of `.txt` files with `title:`/`date:`/`---` header format
- `includes/header.slt` — shared header via `#include`
- `includes/footer.slt` — shared footer via `#include` (Ruby generates date)
- `static/style.css` — blog typography
- Python reads and parses post files from the `posts/` directory, passes to Ruby via `#set/#get`
- Ruby formats post listing as HTML articles

## Requirements for All Examples

- Every example folder has its own `config.toml` with correct paths and only used runtimes enabled
- Every example folder has a `README.md` explaining what it demonstrates and how to run it
- Every `.slt` file has clear comments explaining what it does
- Every example must actually work when run
