# Contributing

Salata is open source and welcomes contributions. The repository is hosted on GitHub at [github.com/nicholasgasior/salata](https://github.com/nicholasgasior/salata).

---

## Getting Started

1. **Fork the repository** on GitHub.
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/<your-username>/salata.git
   cd salata
   ```
3. **Create a branch** for your changes:
   ```bash
   git checkout -b my-feature
   ```
4. **Make your changes**, following the code standards below.
5. **Push your branch** and open a pull request against the `main` branch.

---

## Code Standards

All contributions must follow these rules:

- **Format before committing.** Run `cargo fmt` to ensure consistent code formatting. Unformatted code will not be accepted.
- **Zero clippy warnings.** Run `cargo clippy` and fix all warnings before submitting. The CI pipeline rejects code with clippy warnings.
- **No `unwrap()` in production code.** Use proper error handling with the `thiserror` crate. `unwrap()` is acceptable only in test code (`#[cfg(test)]` modules).
- **Use `serde` + `toml` for configuration.** All config parsing goes through serde deserialization.
- **Use `std::path::PathBuf` for file paths.** Do not use string manipulation for path handling.
- **Platform-agnostic code.** Salata targets Linux, macOS, and Windows. Avoid platform-specific APIs. Use Rust's standard library abstractions for filesystem operations, process spawning, and path handling.
- **Handle line endings.** Do not assume `\n` -- use Rust's cross-platform I/O facilities.
- **UTF-8 everywhere.** All input, output, and internal strings are UTF-8.

---

## Project Structure

Salata is a Cargo workspace with five crates:

| Crate | Purpose |
|-------|---------|
| `salata-core` | Shared library: config parsing, `.slt` parser, runtime execution, security, macros, directives |
| `salata-cli` | The `salata` binary (CLI interpreter) |
| `salata-cgi` | The `salata-cgi` binary (CGI bridge with attack protections) |
| `salata-fastcgi` | The `salata-fastcgi` binary (stub, not yet implemented) |
| `salata-server` | The `salata-server` binary (standalone dev server) |

The dependency chain flows in one direction:

```text
salata-core       <-- shared library
salata-cli        <-- depends on salata-core
salata-cgi        <-- depends on salata-core
salata-fastcgi    <-- depends on salata-core
salata-server     <-- depends on salata-cgi --> salata-core
```

Most contributions will touch `salata-core`, since it contains the parser, runtime execution engine, and shared logic.

---

## Running Tests

### Unit and Integration Tests

```bash
cargo test
```

Unit tests live in `#[cfg(test)]` modules within each crate. Integration tests live in the `tests/integration/` directory and use sample `.slt` fixtures from `tests/fixtures/`.

### End-to-End Tests

E2E tests run inside Docker and must **never** assume that runtimes (Python, Ruby, Node, etc.) are installed on the host machine. All six runtimes are installed inside the Docker container.

```bash
docker compose -f docker/docker-compose.yml up --build test
```

E2E tests cover:

- Each runtime individually
- Shared and isolated scope
- `#include` directive
- `#set`/`#get` macros
- All other directives (`#status`, `#content-type`, `#header`, `#cookie`, `#redirect`)
- Error handling and `display_errors`
- Shell sandbox enforcement
- CGI protections
- Static file serving
- PHP dual mode (CLI vs CGI binary selection)

---

## Pull Request Guidelines

- **Write a clear description.** Explain what the change does and why it is needed.
- **Reference issues.** If your PR addresses a GitHub issue, reference it in the description (e.g., "Fixes #42").
- **Include tests.** New features should come with unit tests. Bug fixes should include a regression test that would have caught the bug.
- **Keep PRs focused.** One feature or fix per pull request. Large PRs that mix unrelated changes are harder to review.
- **Run the full test suite** (`cargo test` and `cargo clippy`) before submitting.

---

## Where to Start

If you are looking for a good first contribution, consider:

- Improving error messages in `crates/salata-core/src/error.rs`
- Adding test coverage for edge cases in the parser
- Improving documentation
- Fixing issues labeled `good first issue` on GitHub

For larger contributions (new runtime, FastCGI implementation, Uniform AST), please open an issue first to discuss the approach before writing code.
