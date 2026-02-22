# PHP Runtime

| Property       | Value                           |
|----------------|---------------------------------|
| Tag            | `<php>`                         |
| Output method  | `echo`                          |
| Shared scope   | `true` (default)                |

## Overview

The PHP runtime is unique among Salata's runtimes because it uses context-aware binary selection. PHP itself has multiple Server API (SAPI) interfaces -- CLI, CGI, and FPM -- and they behave differently. Salata mirrors this model: depending on which Salata binary is running, a different PHP binary is used.

## Output

Use `echo` to produce output. PHP's string interpolation and heredoc syntax also work:

```html
<php>
$items = ["Apples", "Oranges", "Bananas"];

echo "<ul>\n";
foreach ($items as $item) {
    echo "  <li>$item</li>\n";
}
echo "</ul>\n";
</php>
```

Other output functions like `print`, `printf`, and `var_dump` write to stdout as well, but `echo` is the standard approach.

## Context-Aware Binary Selection

This is the defining feature of the PHP runtime. Each Salata binary sets an execution context, and that context determines which PHP binary handles `<php>` blocks:

| Salata Binary      | Execution Context | PHP Binary Used      | Config Field        |
|--------------------|-------------------|----------------------|---------------------|
| `salata` (CLI)     | `Cli`             | `php`                | `cli_path`          |
| `salata-cgi`       | `Cgi`             | `php-cgi`            | `cgi_path`          |
| `salata-fastcgi`   | `FastCgi`         | `php-fpm` (socket/TCP) | `fastcgi_socket` / `fastcgi_host` |
| `salata-server`    | `Server`          | `php-fpm` (socket/TCP) | `fastcgi_socket` / `fastcgi_host` |

### Why This Matters

PHP's different SAPIs handle things like headers, input, and environment variables differently:

- **`php` (CLI)** reads from stdin, writes to stdout, and has no concept of HTTP headers. This is what you want when running `salata template.slt > output.html` from the command line.

- **`php-cgi`** follows the CGI protocol. It reads request data from environment variables (`REQUEST_METHOD`, `QUERY_STRING`, etc.) and can set HTTP headers through its output. This is the correct binary when Salata is running as a CGI program behind nginx or Apache.

- **`php-fpm`** is a long-running FastCGI process manager. It listens on a Unix socket or TCP port and handles requests persistently. This is used when Salata runs as a FastCGI daemon or standalone server, where a persistent PHP process pool is more efficient than spawning a new process per request.

Salata handles this selection automatically. You configure the paths once, and the correct binary is chosen based on which Salata binary is invoked.

## Configuration

```toml
[runtimes.php]
enabled = true
mode = "cgi"
cli_path = "/usr/bin/php"
cgi_path = "/usr/bin/php-cgi"
# fastcgi_socket = "/run/php/php-fpm.sock"
# fastcgi_host = "127.0.0.1:9000"
shared_scope = true
```

| Field              | Type   | Default                    | Description                                        |
|--------------------|--------|----------------------------|----------------------------------------------------|
| `enabled`          | bool   | `true`                     | Enable or disable the PHP runtime                  |
| `mode`             | string | `"cgi"`                    | PHP mode: `"cgi"` or `"fastcgi"`                   |
| `cli_path`         | string | `/usr/bin/php`             | Path to `php` binary (used in CLI context)         |
| `cgi_path`         | string | `/usr/bin/php-cgi`         | Path to `php-cgi` binary (used in CGI context)     |
| `fastcgi_socket`   | string | (unset)                    | Unix socket path for php-fpm                       |
| `fastcgi_host`     | string | (unset)                    | TCP host:port for php-fpm (e.g., `127.0.0.1:9000`) |
| `shared_scope`     | bool   | `true`                     | All blocks share one process per page               |
| `display_errors`   | bool   | (global fallback)          | Override the global `display_errors` setting        |

### The `mode` Field

The `mode` field has two values:

- **`"cgi"`** -- Used for CLI and CGI contexts. Salata spawns `php` or `php-cgi` as a child process.
- **`"fastcgi"`** -- Used for FastCGI and Server contexts. Salata connects to an already-running `php-fpm` process via socket or TCP.

When using `"fastcgi"` mode, you must configure either `fastcgi_socket` or `fastcgi_host` (not both). A Unix socket is generally preferred for same-machine setups:

```toml
[runtimes.php]
enabled = true
mode = "fastcgi"
cli_path = "/usr/bin/php"
cgi_path = "/usr/bin/php-cgi"
fastcgi_socket = "/run/php/php-fpm.sock"
shared_scope = true
```

For remote or containerized php-fpm:

```toml
[runtimes.php]
enabled = true
mode = "fastcgi"
cli_path = "/usr/bin/php"
cgi_path = "/usr/bin/php-cgi"
fastcgi_host = "127.0.0.1:9000"
shared_scope = true
```

## Shared Scope

With shared scope enabled (the default), all `<php>` blocks on the same page share a single PHP process. Variables and functions defined in one block are available in later blocks:

```html
<php>
function formatCurrency($amount) {
    return '$' . number_format($amount, 2);
}

$products = [
    ['name' => 'Widget', 'price' => 19.99],
    ['name' => 'Gadget', 'price' => 45.50],
    ['name' => 'Doohickey', 'price' => 8.75],
];
</php>

<h2>Products</h2>

<php>
echo "<table>\n";
echo "  <tr><th>Product</th><th>Price</th></tr>\n";
foreach ($products as $p) {
    echo "  <tr><td>{$p['name']}</td><td>" . formatCurrency($p['price']) . "</td></tr>\n";
}
$total = array_sum(array_column($products, 'price'));
echo "  <tr><td><strong>Total</strong></td><td><strong>" . formatCurrency($total) . "</strong></td></tr>\n";
echo "</table>\n";
</php>
```

## Cross-Runtime Data Bridge

PHP works with `#set` and `#get` macros for cross-runtime data sharing:

```html
<python>
config = {
    "site_name": "My Store",
    "currency": "USD",
    "tax_rate": 0.08
}
#set("config", config)
</python>

<php>
$config = #get("config");
$price = 29.99;
$tax = $price * $config['tax_rate'];
$total = $price + $tax;

echo "<p>{$config['site_name']}</p>\n";
echo "<p>Price: $" . number_format($price, 2) . "</p>\n";
echo "<p>Tax: $" . number_format($tax, 2) . "</p>\n";
echo "<p>Total: $" . number_format($total, 2) . "</p>\n";
</php>
```

## Isolated Scope

To run a block in its own process:

```html
<php scope="isolated">
// This block has its own PHP process.
$x = 42;
echo $x;
</php>
```

Or set `shared_scope = false` in the config for all PHP blocks.

## Tips

- The `<php>` tag in Salata is not the same as `<?php`. Salata's PHP blocks do not use PHP's opening/closing tags -- the code inside `<php>` is pure PHP code executed directly.
- When testing locally with `salata` (CLI), the `cli_path` binary is used. You do not need `php-cgi` or `php-fpm` installed for CLI-only usage.
- For production CGI setups, ensure `php-cgi` is installed and the `cgi_path` is correct.
- The working directory during execution is the directory containing the `.slt` file being processed.
