# Web Examples

These examples demonstrate using `salata-server` to serve `.slt` files over HTTP. All examples live in the `examples/web/` directory of the repository.

To run any web example, point `salata-server` at the example directory:

```bash
salata-server examples/web/<example>/ --port 3000
```

Then open `http://localhost:3000` in your browser.

---

## single-file/

Five standalone `.slt` files, each demonstrating a different web directive. These are the simplest possible web examples.

**hello.slt** -- A basic HTML page with an embedded Python block that prints the current time:

```html
<!DOCTYPE html>
<html>
<head>
    <title>Hello — Salata</title>
</head>
<body>
    <h1>Hello from Salata!</h1>
    <p>The current time is:</p>
    <python>
from datetime import datetime
print(f"<strong>{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}</strong>")
    </python>
    <p>This page was generated server-side by Salata.</p>
</body>
</html>
```

**status-codes.slt** -- Demonstrates the `#status` directive to set a custom HTTP status code:

```html
#status 404
<!DOCTYPE html>
<html>
<head>
    <title>404 — Not Found</title>
</head>
<body>
    <h1>404 — Page Not Found</h1>
    <python>
print("<p>This page intentionally returns a 404 status code.</p>")
print("<p>The <code>#status 404</code> directive sets the HTTP response status.</p>")
    </python>
</body>
</html>
```

**redirect.slt** -- A single-line file that redirects to another page:

```html
#redirect "/hello.slt"
```

**headers.slt** -- Sets custom HTTP headers and cookies using directives:

```html
#header "X-Powered-By" "Salata"
#header "X-Example" "headers-demo"
#cookie "visited" "true" httponly
<!DOCTYPE html>
<html>
<head>
    <title>Headers &amp; Cookies — Salata</title>
</head>
<body>
    <h1>Custom Headers &amp; Cookies</h1>
    <p>This page sets the following:</p>
    <ul>
        <li><code>X-Powered-By: Salata</code> (custom header)</li>
        <li><code>X-Example: headers-demo</code> (custom header)</li>
        <li><code>visited=true</code> (httponly cookie)</li>
    </ul>
    <python>
print("<p>Check the response headers in your browser's developer tools!</p>")
    </python>
</body>
</html>
```

**content-type.slt** -- Returns JSON instead of HTML using the `#content-type` directive:

```html
#content-type application/json
<python>
import json

data = {
    "message": "This page returns JSON, not HTML",
    "content_type": "application/json",
    "directive": "#content-type application/json"
}
print(json.dumps(data, indent=2))
</python>
```

Run the single-file examples:

```bash
salata-server examples/web/single-file/ --port 3000
# Visit http://localhost:3000/hello.slt
# Visit http://localhost:3000/status-codes.slt
# Visit http://localhost:3000/redirect.slt (redirects to /hello.slt)
# Visit http://localhost:3000/headers.slt
# Visit http://localhost:3000/content-type.slt
```

---

## portfolio/

A multi-page website demonstrating `#include` for shared partials and static file serving.

**Structure:**

```text
portfolio/
  index.slt           # Home page
  about.slt           # About page
  contact.slt         # Contact page
  includes/
    header.slt         # Shared header partial
    footer.slt         # Shared footer partial
  static/
    style.css          # Static CSS, served as-is
```

Each page uses `#include` to pull in the shared header and footer:

```html
#include "includes/header.slt"

<main>
    <h2>Welcome</h2>
    <python>
print("<p>This is the home page.</p>")
    </python>
</main>

#include "includes/footer.slt"
```

The `static/style.css` file is served directly by `salata-server` without any processing. Only `.slt` files are processed through the template engine.

```bash
salata-server examples/web/portfolio/ --port 3000
# Visit http://localhost:3000/index.slt
# Visit http://localhost:3000/about.slt
# Visit http://localhost:3000/contact.slt
```

---

## dashboard/

A single-page dashboard that uses multiple runtimes together, demonstrating cross-runtime data sharing on a web page.

**Structure:**

```text
dashboard/
  index.slt            # Main dashboard page
  static/
    style.css          # Dashboard styles
```

The dashboard page uses:

- **Python** to compute metrics and statistics
- **Ruby** to generate HTML tables from the data
- **Shell** to gather live server stats (hostname, uptime, platform)
- **JavaScript** to render SVG sparkline charts client-side

Runtimes share data via `#set`/`#get` -- Python computes the numbers, Ruby and JavaScript consume them to render different visualizations.

```bash
salata-server examples/web/dashboard/ --port 3000
```

---

## php-showcase/

Demonstrates PHP running alongside Python on the same page. This example highlights Salata's polyglot nature -- you can use PHP for what it does best (string manipulation, HTML generation) while using Python for data processing, all in a single `.slt` file.

```bash
salata-server examples/web/php-showcase/ --port 3000
```

---

## api-endpoint/

A JSON API endpoint built with Salata, using `#content-type` and `#status` directives. Demonstrates that Salata is not limited to generating HTML -- it can serve as a lightweight API backend.

The endpoint returns a structured JSON response with proper content type headers. It uses `#content-type application/json` to set the MIME type and runtime blocks to build the response data.

```bash
salata-server examples/web/api-endpoint/ --port 3000
```

---

## error-pages/

Demonstrates custom error pages using `.slt` files. The `config.toml` for this example points `page_404` and `page_500` to `.slt` files in the `errors/` subdirectory.

**Structure:**

```text
error-pages/
  config.toml          # Points page_404/page_500 to .slt files
  index.slt            # Main page
  errors/
    404.slt            # Custom 404 page with dynamic content
    500.slt            # Custom 500 page with dynamic content
```

The error page `.slt` files can contain runtime blocks, so your 404 and 500 pages can include dynamic content like timestamps, request information, or suggestions.

Configuration:

```toml
[errors]
page_404 = "./errors/404.slt"
page_500 = "./errors/500.slt"
```

```bash
salata-server examples/web/error-pages/ --port 3000
# Visit http://localhost:3000/nonexistent to see the custom 404 page
```

---

## blog/

A file-based blog built entirely with Salata. Python reads text files from a `posts/` directory and generates an index page with links and summaries. Shared header and footer are included via `#include`.

**Structure:**

```text
blog/
  config.toml
  index.slt              # Blog index, lists all posts
  includes/
    header.slt           # Shared header
    footer.slt           # Shared footer
  posts/
    hello-world.txt      # Blog post content
    salata-guide.txt     # Blog post content
    tips-and-tricks.txt  # Blog post content
  static/
    style.css            # Blog styles
```

The `index.slt` file uses Python to scan the `posts/` directory, read each text file, extract a title and preview, and generate the blog index HTML. The header and footer partials are shared across all pages via `#include`.

```bash
salata-server examples/web/blog/ --port 3000
```
