---
paths:
  - "**/*.rs"
---
# Rust Code Rules

- Always `cargo fmt` before committing
- `cargo clippy` must produce zero warnings
- No `unwrap()` in production code — use `?` operator, `thiserror` error types, or `anyhow`
- Use `std::path::PathBuf` for all file paths — never string concatenation
- Handle line endings cross-platform (no hardcoded `\n` for file I/O)
- Platform-agnostic process spawning — use `std::process::Command` properly
- Unit tests in `#[cfg(test)]` modules within each source file
- Use `serde` + `toml` crate for config parsing
- Prefer `thiserror` for library error types, `anyhow` only in binary crates
- All public functions must have doc comments
