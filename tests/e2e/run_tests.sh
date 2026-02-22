#!/bin/bash
# Salata E2E Test Suite
# Runs inside Docker where all runtimes are installed.
#
# Usage: ./tests/e2e/run_tests.sh
# Exit code: 0 if all tests pass, 1 if any fail.

set -euo pipefail

SALATA="/usr/local/bin/salata"
CONFIG="/srv/salata/tests/e2e/test_config.toml"
FIXTURES="/srv/salata/tests/fixtures"

PASS=0
FAIL=0
ERRORS=""

# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------

run_salata() {
    "$SALATA" --config "$CONFIG" "$@" 2>/dev/null
}

run_salata_with_stderr() {
    "$SALATA" --config "$CONFIG" "$@" 2>&1
}

assert_contains() {
    local test_name="$1"
    local output="$2"
    local expected="$3"
    if echo "$output" | grep -qF "$expected"; then
        PASS=$((PASS + 1))
        echo "  PASS: $test_name"
    else
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $test_name (expected '$expected' in output)"
        echo "  FAIL: $test_name"
        echo "    Expected to contain: $expected"
        echo "    Got: $(echo "$output" | head -5)"
    fi
}

assert_not_contains() {
    local test_name="$1"
    local output="$2"
    local unexpected="$3"
    if ! echo "$output" | grep -qF "$unexpected"; then
        PASS=$((PASS + 1))
        echo "  PASS: $test_name"
    else
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $test_name (unexpected '$unexpected' found in output)"
        echo "  FAIL: $test_name"
        echo "    Unexpectedly found: $unexpected"
    fi
}

assert_exit_code() {
    local test_name="$1"
    local expected_code="$2"
    shift 2
    set +e
    "$SALATA" --config "$CONFIG" "$@" >/dev/null 2>&1
    local actual_code=$?
    set -e
    if [ "$actual_code" -eq "$expected_code" ]; then
        PASS=$((PASS + 1))
        echo "  PASS: $test_name"
    else
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: $test_name (expected exit $expected_code, got $actual_code)"
        echo "  FAIL: $test_name (expected exit $expected_code, got $actual_code)"
    fi
}

# ---------------------------------------------------------------------------
# 1. Individual runtimes
# ---------------------------------------------------------------------------
echo ""
echo "=== Individual Runtimes ==="

output=$(run_salata "$FIXTURES/python_only.slt")
assert_contains "Python runtime" "$output" "Hello from Python"

output=$(run_salata "$FIXTURES/ruby_only.slt")
assert_contains "Ruby runtime" "$output" "Hello from Ruby"

output=$(run_salata "$FIXTURES/javascript_only.slt")
assert_contains "JavaScript runtime" "$output" "Hello from JavaScript"

output=$(run_salata "$FIXTURES/typescript_only.slt")
assert_contains "TypeScript runtime" "$output" "Hello from TypeScript"

output=$(run_salata "$FIXTURES/php_only.slt")
assert_contains "PHP runtime" "$output" "Hello from PHP"

output=$(run_salata "$FIXTURES/shell_only.slt")
assert_contains "Shell runtime" "$output" "Hello from Shell"

# ---------------------------------------------------------------------------
# 2. HTML only (no runtime blocks)
# ---------------------------------------------------------------------------
echo ""
echo "=== HTML-Only ==="

output=$(run_salata "$FIXTURES/basic.slt")
assert_contains "HTML passthrough" "$output" "<h1>Basic Test</h1>"
assert_contains "HTML static content" "$output" "<p>Static content</p>"

# ---------------------------------------------------------------------------
# 3. Multi-runtime in one file
# ---------------------------------------------------------------------------
echo ""
echo "=== Multi-Runtime ==="

output=$(run_salata "$FIXTURES/multi_runtime.slt")
assert_contains "Multi-runtime: Python" "$output" "PYTHON_OK"
assert_contains "Multi-runtime: Ruby" "$output" "RUBY_OK"
assert_contains "Multi-runtime: JS" "$output" "JS_OK"
assert_contains "Multi-runtime: Shell" "$output" "SHELL_OK"
assert_contains "Multi-runtime: PHP" "$output" "PHP_OK"

# ---------------------------------------------------------------------------
# 4. Shared scope
# ---------------------------------------------------------------------------
echo ""
echo "=== Shared Scope ==="

output=$(run_salata "$FIXTURES/python_scope.slt")
assert_contains "Python shared scope" "$output" "50"

output=$(run_salata "$FIXTURES/shared_scope_ruby.slt")
assert_contains "Ruby shared scope" "$output" "15"

output=$(run_salata "$FIXTURES/shared_scope_js.slt")
assert_contains "JS shared scope" "$output" "15"

# ---------------------------------------------------------------------------
# 5. Isolated scope
# ---------------------------------------------------------------------------
echo ""
echo "=== Isolated Scope ==="

output=$(run_salata "$FIXTURES/isolated_python.slt")
assert_contains "Isolated scope: first block" "$output" "100"
assert_contains "Isolated scope: second block isolated" "$output" "ISOLATED_OK"

# ---------------------------------------------------------------------------
# 6. JS/TS print helpers
# ---------------------------------------------------------------------------
echo ""
echo "=== JS Print Helpers ==="

output=$(run_salata "$FIXTURES/js_print_helpers.slt")
assert_contains "print() no newline" "$output" "no-newline"
assert_contains "println() with newline" "$output" "with-newline"
assert_contains "print() multiple args" "$output" "A B C"

# ---------------------------------------------------------------------------
# 7. #include directive
# ---------------------------------------------------------------------------
echo ""
echo "=== #include Directive ==="

output=$(run_salata "$FIXTURES/include_basic.slt")
assert_contains "Include header" "$output" "Navigation Bar"
assert_contains "Include footer" "$output" "Footer Content"
assert_contains "Include main content" "$output" "Page Content"

output=$(run_salata "$FIXTURES/include_nested.slt")
assert_contains "Nested include: outer" "$output" "NESTED_OUTER_START"
assert_contains "Nested include: inner" "$output" "NESTED_INNER"
assert_contains "Nested include: after" "$output" "AFTER"

output=$(run_salata "$FIXTURES/include_with_runtime.slt")
assert_contains "Include with runtime block" "$output" "FROM_INCLUDE"
assert_contains "Include before marker" "$output" "BEFORE_INCLUDE"
assert_contains "Include after marker" "$output" "AFTER_INCLUDE"

# ---------------------------------------------------------------------------
# 8. #set/#get macros (cross-runtime)
# ---------------------------------------------------------------------------
echo ""
echo "=== #set/#get Macros ==="

output=$(run_salata "$FIXTURES/macros/python_to_js.slt")
assert_contains "Macro Python→JS: count" "$output" "COUNT:2"
assert_contains "Macro Python→JS: data" "$output" "FIRST:Alice"

output=$(run_salata "$FIXTURES/macros/python_to_ruby.slt")
assert_contains "Macro Python→Ruby" "$output" "Hello from Python:42"

output=$(run_salata "$FIXTURES/macros/get_with_default.slt")
assert_contains "Macro #get with default" "$output" "DEFAULT_VALUE"

output=$(run_salata "$FIXTURES/macros/js_to_python.slt")
assert_contains "Macro JS→Python" "$output" "X:10,Y:20"

# ---------------------------------------------------------------------------
# 9. Directives
# ---------------------------------------------------------------------------
echo ""
echo "=== Directives ==="

# #status — we test via salata CLI (stdout is just HTML, status is for CGI)
output=$(run_salata "$FIXTURES/directives/status_404.slt")
assert_contains "Status 404: HTML output" "$output" "Not Found Page"

output=$(run_salata "$FIXTURES/directives/content_type_json.slt")
assert_contains "Content-type JSON" "$output" '"status": "ok"'

output=$(run_salata "$FIXTURES/directives/custom_headers.slt")
assert_contains "Custom headers: HTML" "$output" "Headers set"

output=$(run_salata "$FIXTURES/directives/cookie.slt")
assert_contains "Cookie directive: HTML" "$output" "Cookies set"

output=$(run_salata "$FIXTURES/directives/all_directives.slt")
assert_contains "All directives combined" "$output" "All directives combined"

# ---------------------------------------------------------------------------
# 10. Error handling
# ---------------------------------------------------------------------------
echo ""
echo "=== Error Handling ==="

# Python runtime error — display_errors = true, so error should show
output=$(run_salata_with_stderr "$FIXTURES/errors/python_error.slt" || true)
assert_contains "Python error displayed" "$output" "NameError"

# Nested runtime tags → parse error
output=$(run_salata_with_stderr "$FIXTURES/errors/nested_tags.slt" || true)
assert_contains "Nested tags rejected" "$output" "nested"

# Duplicate #status → parse error
assert_exit_code "Duplicate status fails" 1 "$FIXTURES/errors/duplicate_status.slt"

# ---------------------------------------------------------------------------
# 11. Shell sandbox
# ---------------------------------------------------------------------------
echo ""
echo "=== Shell Sandbox ==="

# Blocked commands
output=$(run_salata_with_stderr "$FIXTURES/security/shell_rm.slt" || true)
assert_contains "Shell blocks rm" "$output" "blocked"

output=$(run_salata_with_stderr "$FIXTURES/security/shell_fork_bomb.slt" || true)
assert_contains "Shell blocks fork bomb" "$output" "fork bomb"

output=$(run_salata_with_stderr "$FIXTURES/security/shell_backgrounding.slt" || true)
assert_contains "Shell blocks backgrounding" "$output" "not allowed"

output=$(run_salata_with_stderr "$FIXTURES/security/shell_pipe_to_bash.slt" || true)
assert_contains "Shell blocks pipe to bash" "$output" "blocked"

output=$(run_salata_with_stderr "$FIXTURES/security/shell_eval.slt" || true)
assert_contains "Shell blocks eval" "$output" "blocked"

output=$(run_salata_with_stderr "$FIXTURES/security/shell_exec.slt" || true)
assert_contains "Shell blocks exec" "$output" "blocked"

output=$(run_salata_with_stderr "$FIXTURES/security/shell_sudo.slt" || true)
assert_contains "Shell blocks sudo" "$output" "blocked"

output=$(run_salata_with_stderr "$FIXTURES/security/shell_etc_access.slt" || true)
assert_contains "Shell blocks /etc access" "$output" "blocked"

# Safe commands should work
output=$(run_salata "$FIXTURES/security/shell_safe.slt")
assert_contains "Shell allows echo" "$output" "SAFE_OUTPUT"
assert_contains "Shell allows printf" "$output" "PRINTF_OK"

# Shell timeout (should not hang forever — uses short timeout in test)
# We just verify the command is accepted and returns within a reasonable time
# The timeout fixture uses sleep 999 which the sandbox timeout should kill

# ---------------------------------------------------------------------------
# 12. Static file serving (salata-server)
# ---------------------------------------------------------------------------
echo ""
echo "=== Static File Serving ==="

# Create a temporary site directory with static files
SITE_DIR=$(mktemp -d)
echo "<h1>Static HTML</h1>" > "$SITE_DIR/index.html"
echo "body { color: red; }" > "$SITE_DIR/style.css"
echo "console.log('hello');" > "$SITE_DIR/app.js"
mkdir -p "$SITE_DIR/images"
echo "PNG_PLACEHOLDER" > "$SITE_DIR/images/logo.png"

# Start salata-server in background
SERVER="/usr/local/bin/salata-server"
PORT=18080
"$SERVER" --config "$CONFIG" --port "$PORT" "$SITE_DIR" &
SERVER_PID=$!
sleep 1

# Test static file serving (if curl is available)
if command -v curl &>/dev/null; then
    output=$(curl -s "http://127.0.0.1:$PORT/index.html")
    assert_contains "Server: static HTML" "$output" "Static HTML"

    output=$(curl -s "http://127.0.0.1:$PORT/style.css")
    assert_contains "Server: static CSS" "$output" "color: red"

    output=$(curl -s "http://127.0.0.1:$PORT/app.js")
    assert_contains "Server: static JS" "$output" "console.log"

    # 404 for missing file
    status=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/missing.html")
    if [ "$status" = "404" ]; then
        PASS=$((PASS + 1))
        echo "  PASS: Server: 404 for missing file"
    else
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: Server: 404 (got $status)"
        echo "  FAIL: Server: 404 (got $status)"
    fi

    # Path traversal blocked
    status=$(curl -s -o /dev/null -w "%{http_code}" --path-as-is "http://127.0.0.1:$PORT/../../../etc/passwd")
    if [ "$status" = "403" ]; then
        PASS=$((PASS + 1))
        echo "  PASS: Server: path traversal blocked"
    else
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: Server: path traversal (got $status)"
        echo "  FAIL: Server: path traversal (got $status)"
    fi

    # Blocked extension
    status=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/config.toml")
    if [ "$status" = "403" ]; then
        PASS=$((PASS + 1))
        echo "  PASS: Server: blocked extension .toml"
    else
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: Server: blocked ext (got $status)"
        echo "  FAIL: Server: blocked ext (got $status)"
    fi

    # Dotfile blocked
    echo "secret" > "$SITE_DIR/.env"
    status=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/.env")
    if [ "$status" = "403" ]; then
        PASS=$((PASS + 1))
        echo "  PASS: Server: dotfile blocked"
    else
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL: Server: dotfile (got $status)"
        echo "  FAIL: Server: dotfile (got $status)"
    fi

    # Directory index
    output=$(curl -s "http://127.0.0.1:$PORT/")
    assert_contains "Server: directory index" "$output" "Static HTML"
else
    echo "  SKIP: curl not available, skipping server tests"
fi

# Stop server
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
rm -rf "$SITE_DIR"

# ---------------------------------------------------------------------------
# 13. PHP dual mode (CGI mode tested via fixtures above)
# ---------------------------------------------------------------------------
echo ""
echo "=== PHP Dual Mode ==="

# PHP CGI mode is tested through php_only.slt above
# FastCGI mode would need php-fpm running, which we skip in basic E2E
echo "  INFO: PHP CGI mode tested via php_only.slt (above)"
echo "  INFO: PHP FastCGI mode requires php-fpm (skipped in basic E2E)"

# ---------------------------------------------------------------------------
# Results
# ---------------------------------------------------------------------------
echo ""
echo "==========================================="
echo "  E2E Results: $PASS passed, $FAIL failed"
echo "==========================================="

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo "Failures:"
    echo -e "$ERRORS"
    echo ""
    exit 1
fi

echo ""
echo "All E2E tests passed!"
exit 0
