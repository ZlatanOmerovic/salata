# Changelog

## v0.1.0 -- Initial Release

First public release of the Salata polyglot text templating engine.

### Features

- **6 runtimes**: Python, Ruby, JavaScript, TypeScript, PHP, Shell
- **4 binaries**: `salata` (CLI), `salata-cgi` (CGI bridge), `salata-fastcgi` (stub), `salata-server` (dev server)
- **Cross-runtime data**: `#set`/`#get` macros for sharing data between runtimes via JSON serialization
- **Directives**: `#include`, `#status`, `#content-type`, `#header`, `#cookie`, `#redirect`
- **Scope management**: Shared (default) and isolated scope per block or per runtime
- **Shell sandbox**: Three-phase security (static analysis, environment setup, runtime monitoring)
- **CGI protections**: Slowloris, path traversal, dotfiles, request limits, null bytes
- **Context-aware PHP**: Automatic binary selection (php/php-cgi/php-fpm) based on execution context
- **JS/TS helpers**: Injected `print()` and `println()` functions
- **Hot reload**: File watcher in `salata-server` for development
- **Project scaffolding**: `salata init` detects runtimes and generates config
- **Docker playground**: Interactive container with all runtimes and editors
- **Automatic dedent**: Code inside runtime blocks is automatically dedented
- **8-platform builds**: Linux (x86_64, ARM64, i686), macOS (x86_64, ARM64), Windows (x86_64, i686, ARM64)
- **Comprehensive examples**: 15 example projects covering CLI and web use cases

### Known Limitations

- FastCGI daemon is a stub
- Shell `#set`/`#get` macros produce invalid syntax
- Windows builds untested
- Uniform AST not yet implemented
