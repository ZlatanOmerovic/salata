# Testing — Salata

## Unit Tests
- `#[cfg(test)]` modules in each crate
- Cover: parser, directives, macros, config validation, security, shell sandbox, scope management, runtime enable/disable
- Run: `cargo test`

## Integration Tests
- `tests/integration/` directory with sample `.slt` fixtures
- Can run locally if runtimes installed

## End-to-End Tests (Docker)

**All E2E tests run inside Docker.** Consistent, reproducible environment.

### Docker Files
```
docker/
  ├── Dockerfile          # Build salata + install Python, Ruby, Node, TS, PHP, Bash, sh, dash
  ├── Dockerfile.test     # Extends main, adds test deps, fixtures, test config.toml
  ├── docker-compose.yml  # Placeholder — will orchestrate build, test, nginx, php-fpm containers
  └── .dockerignore
```

### Test Fixtures
```
tests/fixtures/
  ├── basic.slt, python_scope.slt, multi_runtime.slt
  ├── includes/ (header.slt, footer.slt, nested_include.slt)
  ├── directives/ (status.slt, content_type.slt, redirect.slt, headers.slt)
  ├── macros/ (set_get.slt, cross_runtime.slt)
  ├── errors/ (nested_runtime.slt, invalid_shell.slt, runtime_error.slt)
  └── security/ (shell_sandbox.slt, path_traversal.slt, blocked_commands.slt)
```

### E2E Coverage

**Interpreter:** Each runtime individually, multi-runtime pages, shared/isolated scope, #include (with nesting + max depth), #set/#get macros, all directives, directive validation, display_errors on/off, nested tag rejection, UTF-8, disabled runtimes (clear error), all runtimes disabled (informative exit)

**CGI:** HTTP response formatting, CGI env vars, request methods, Slowloris protection, path traversal, dotfiles, blocked extensions, null bytes, request size limits, connections per IP, timeouts. Note: These tests cover the `salata-cgi` binary's internal logic. Integration tests with actual nginx and Apache web servers are planned but not yet in place.

**Server:** Static files, .slt processing, MIME types, 404, custom error pages, hot reload

**Security:** Every blocked shell command, fork bombs, backgrounding, pipe to shell, eval/exec, clean env, ulimit, timeout kills, allowed commands work

**PHP:** Context-aware binary selection — `php` in CLI mode, `php-cgi` in CGI mode, `php-fpm` socket mode in server mode

### Running E2E
```bash
docker compose -f docker/docker-compose.yml up --build test
# or
docker build -f docker/Dockerfile.test -t salata-test .
docker run --rm salata-test
```

## Docker Playground

Interactive playground for trying Salata. See specs/PLAYGROUND.md for full spec.

```bash
./playground/start-playground.sh    # Linux/macOS
playground\start-playground.bat     # Windows CMD
playground\start-playground.ps1     # PowerShell
```

Builds Ubuntu container with Rust, all runtimes, editors, Starship prompt, bat, and pre-built salata. Drops into bash with a welcome banner and starter files.
