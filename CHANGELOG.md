# Changelog

## 0.1.0 (Unreleased)

Initial release.

### Added
- Core interpreter (`salata`) — processes `.slt` files with embedded runtime blocks
- `salata init` subcommand — detects runtimes, generates config.toml and starter files
- `salata-cgi` — CGI bridge with built-in security protections
- `salata-fastcgi` — stub (not yet implemented)
- `salata-server` — standalone dev server with static file serving and hot reload
- 6 runtime blocks: `<python>`, `<ruby>`, `<javascript>`, `<typescript>`, `<php>`, `<shell>`
- Directives: `#include`, `#status`, `#content-type`, `#header`, `#cookie`, `#redirect`
- Cross-runtime data bridge: `#set`/`#get` macros with JSON serialization
- Shared and isolated scope management (`scope="isolated"` attribute)
- Execution context system (CLI, CGI, FastCGI, Server) — affects PHP binary selection
- Shell sandbox with static analysis, blocked commands, blocked paths, and runtime monitoring
- Per-runtime enable/disable via config
- Per-runtime logging with rotation
- Custom error pages (404, 500) supporting `.slt` templates
- Docker playground with all runtimes, editors, Starship prompt, and examples
- Comprehensive examples directory (8 CLI + 7 web examples)
- Runtime detection scripts for Linux/macOS/Windows
