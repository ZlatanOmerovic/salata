#!/usr/bin/env bash
# detect-runtimes.sh — Detect available runtimes and generate config.toml
# Works on Linux and macOS.

set -euo pipefail

OUTPUT="config.toml"
FOUND=0
TOTAL=0

# --- Helpers ----------------------------------------------------------------

# Try to find a binary from a list of candidate paths, falling back to `which`.
# Sets RESULT to the found path or empty string.
find_binary() {
    RESULT=""
    local name="$1"
    shift
    for candidate in "$@"; do
        if [ -x "$candidate" ]; then
            RESULT="$candidate"
            return
        fi
    done
    # Fall back to which.
    RESULT=$(which "$name" 2>/dev/null || true)
}

# Get version string from a binary. Tries --version first, then -v.
get_version() {
    local bin="$1"
    local ver
    ver=$("$bin" --version 2>&1 | head -1) || ver=$("$bin" -v 2>&1 | head -1) || ver="unknown"
    echo "$ver"
}

print_found() {
    local label="$1" path="$2" version="$3"
    printf "  %-14s %-40s %s  OK\n" "$label" "$path" "$version"
}

print_missing() {
    local label="$1"
    printf "  %-14s NOT FOUND — will be disabled\n" "$label"
}

# --- Detection --------------------------------------------------------------

echo "Salata Runtime Detection"
echo "========================"
echo ""

# Python
TOTAL=$((TOTAL + 1))
find_binary python3 /usr/bin/python3 /usr/local/bin/python3 /opt/homebrew/bin/python3
PYTHON_PATH="$RESULT"
if [ -z "$PYTHON_PATH" ]; then
    # Try plain "python" as fallback.
    find_binary python /usr/bin/python /usr/local/bin/python /opt/homebrew/bin/python
    PYTHON_PATH="$RESULT"
fi
PYTHON_ENABLED=false
if [ -n "$PYTHON_PATH" ]; then
    PYTHON_VERSION=$(get_version "$PYTHON_PATH")
    print_found "python" "$PYTHON_PATH" "$PYTHON_VERSION"
    PYTHON_ENABLED=true
    FOUND=$((FOUND + 1))
else
    print_missing "python"
fi

# Ruby
TOTAL=$((TOTAL + 1))
find_binary ruby /usr/bin/ruby /usr/local/bin/ruby /opt/homebrew/bin/ruby
RUBY_PATH="$RESULT"
RUBY_ENABLED=false
if [ -n "$RUBY_PATH" ]; then
    RUBY_VERSION=$(get_version "$RUBY_PATH")
    print_found "ruby" "$RUBY_PATH" "$RUBY_VERSION"
    RUBY_ENABLED=true
    FOUND=$((FOUND + 1))
else
    print_missing "ruby"
fi

# JavaScript (node)
TOTAL=$((TOTAL + 1))
find_binary node /usr/bin/node /usr/local/bin/node /opt/homebrew/bin/node
NODE_PATH="$RESULT"
NODE_ENABLED=false
if [ -n "$NODE_PATH" ]; then
    NODE_VERSION=$(get_version "$NODE_PATH")
    print_found "javascript" "$NODE_PATH" "$NODE_VERSION"
    NODE_ENABLED=true
    FOUND=$((FOUND + 1))
else
    print_missing "javascript"
fi

# TypeScript (ts-node, tsx, bun)
TOTAL=$((TOTAL + 1))
find_binary ts-node /usr/bin/ts-node /usr/local/bin/ts-node /opt/homebrew/bin/ts-node
TS_PATH="$RESULT"
if [ -z "$TS_PATH" ]; then
    find_binary tsx /usr/bin/tsx /usr/local/bin/tsx /opt/homebrew/bin/tsx
    TS_PATH="$RESULT"
fi
if [ -z "$TS_PATH" ]; then
    find_binary bun /usr/bin/bun /usr/local/bin/bun /opt/homebrew/bin/bun
    TS_PATH="$RESULT"
fi
TS_ENABLED=false
if [ -n "$TS_PATH" ]; then
    TS_VERSION=$(get_version "$TS_PATH")
    print_found "typescript" "$TS_PATH" "$TS_VERSION"
    TS_ENABLED=true
    FOUND=$((FOUND + 1))
else
    print_missing "typescript"
fi

# PHP (cli)
TOTAL=$((TOTAL + 1))
find_binary php /usr/bin/php /usr/local/bin/php /opt/homebrew/bin/php
PHP_CLI_PATH="$RESULT"
PHP_ENABLED=false
if [ -n "$PHP_CLI_PATH" ]; then
    PHP_CLI_VERSION=$(get_version "$PHP_CLI_PATH")
    print_found "php" "$PHP_CLI_PATH" "$PHP_CLI_VERSION"
    PHP_ENABLED=true
    FOUND=$((FOUND + 1))
else
    print_missing "php"
fi

# PHP-CGI
find_binary php-cgi /usr/bin/php-cgi /usr/local/bin/php-cgi /opt/homebrew/bin/php-cgi
PHP_CGI_PATH="$RESULT"
if [ -n "$PHP_CGI_PATH" ]; then
    PHP_CGI_VERSION=$(get_version "$PHP_CGI_PATH")
    print_found "php-cgi" "$PHP_CGI_PATH" "$PHP_CGI_VERSION"
else
    print_missing "php-cgi"
fi

# Shell — check all whitelisted paths
echo ""
echo "Shells (whitelisted):"

SHELL_PATH=""
SHELL_CANDIDATES=(
    /bin/bash
    /bin/sh
    /bin/zsh
    /usr/bin/bash
    /usr/bin/sh
    /usr/bin/zsh
    /usr/bin/fish
    /usr/bin/dash
    /usr/bin/ash
)

SHELLS_FOUND=0
for candidate in "${SHELL_CANDIDATES[@]}"; do
    TOTAL=$((TOTAL + 1))
    if [ -x "$candidate" ]; then
        ver=$("$candidate" --version 2>&1 | head -1 || echo "unknown")
        print_found "$(basename "$candidate")" "$candidate" "$ver"
        SHELLS_FOUND=$((SHELLS_FOUND + 1))
        FOUND=$((FOUND + 1))
        # Use the first found shell as the config default.
        if [ -z "$SHELL_PATH" ]; then
            SHELL_PATH="$candidate"
        fi
    else
        print_missing "$(basename "$candidate")"
    fi
done

SHELL_ENABLED=false
if [ -n "$SHELL_PATH" ]; then
    SHELL_ENABLED=true
fi

# --- Generate config.toml ---------------------------------------------------

echo ""
echo "Generating $OUTPUT ..."

cat > "$OUTPUT" <<TOMLEOF
[salata]
display_errors = true
default_content_type = "text/html; charset=utf-8"
encoding = "utf-8"

[server]
hot_reload = true

[logging]
directory = "./logs"
rotation_max_size = "50MB"
rotation_max_files = 10

[logging.server]
access_log = "access.log"
error_log = "error.log"
format = "combined"

[logging.runtimes]
python = "python.log"
ruby = "ruby.log"
javascript = "javascript.log"
typescript = "typescript.log"
php = "php.log"
shell = "shell.log"

[runtimes.python]
enabled = ${PYTHON_ENABLED}
path = "${PYTHON_PATH:-/usr/bin/python3}"
shared_scope = true
display_errors = true

[runtimes.ruby]
enabled = ${RUBY_ENABLED}
path = "${RUBY_PATH:-/usr/bin/ruby}"
shared_scope = true

[runtimes.javascript]
enabled = ${NODE_ENABLED}
path = "${NODE_PATH:-/usr/bin/node}"
shared_scope = true

[runtimes.typescript]
enabled = ${TS_ENABLED}
path = "${TS_PATH:-/usr/bin/ts-node}"
shared_scope = true

[runtimes.php]
enabled = ${PHP_ENABLED}
mode = "cgi"
cli_path = "${PHP_CLI_PATH:-/usr/bin/php}"
cgi_path = "${PHP_CGI_PATH:-/usr/bin/php-cgi}"
# fastcgi_socket = "/run/php/php-fpm.sock"
# fastcgi_host = "127.0.0.1:9000"
shared_scope = true

[runtimes.shell]
enabled = ${SHELL_ENABLED}
path = "${SHELL_PATH:-/bin/bash}"
shared_scope = true

[cgi]
header_timeout = "5s"
body_timeout = "30s"
min_data_rate = "100b/s"
max_url_length = 2048
max_header_size = "8KB"
max_header_count = 50
max_query_string_length = 2048
max_body_size = "10MB"
max_connections_per_ip = 20
max_total_connections = 200
max_execution_time = "30s"
max_memory_per_request = "128MB"
max_response_size = "50MB"
response_timeout = "60s"
block_dotfiles = true
block_path_traversal = true
blocked_extensions = [".toml", ".env", ".git", ".log"]
block_null_bytes = true
block_non_printable_headers = true
validate_content_length = true
max_child_processes = 10
allow_outbound_network = true

[errors]
page_404 = "./errors/404.slt"
page_500 = "./errors/500.slt"
TOMLEOF

echo ""
echo "Found $FOUND of $TOTAL runtimes. Config written to $OUTPUT."
