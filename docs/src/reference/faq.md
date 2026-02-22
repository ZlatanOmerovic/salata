# FAQ / Troubleshooting

## General Questions

### What is Salata?

Salata is a polyglot text templating engine. It processes `.slt` template files that contain embedded runtime blocks -- `<python>`, `<ruby>`, `<javascript>`, `<typescript>`, `<php>`, and `<shell>` -- executes the code in each block server-side using the respective language interpreter, captures the stdout output, and splices it back into the document. The result is written to stdout (in CLI mode) or returned as an HTTP response (in CGI or server mode).

The output is whatever the code prints. It can be HTML, JSON, YAML, plain text, configuration files, Markdown, CSV, or any other text format.

### Is it production-ready?

No. Salata is a concept project under active development. It is suitable for experimentation, learning, and prototyping. It has not been audited for security in production environments, and some features (FastCGI, Uniform AST) are not yet implemented. Use it to explore the idea of polyglot templating, not to serve production traffic.

### Why Rust?

Rust provides several properties that are important for a templating engine that spawns and manages child processes:

- **Performance** -- parsing, process management, and I/O are fast without a garbage collector.
- **Memory safety** -- no segfaults or buffer overflows in the engine itself.
- **Cross-platform compilation** -- a single codebase compiles to Linux, macOS, and Windows on multiple architectures.
- **Single binary distribution** -- each of the four Salata binaries is a self-contained executable with no runtime dependencies (beyond the language interpreters themselves).

### Why not just use PHP/Python/etc. directly?

Salata addresses a specific niche: mixing multiple languages in a single template file and sharing data between them. If your project only needs one language, use that language directly. Salata is for cases where you want to:

- Use Python for data processing and JavaScript for formatting in the same file.
- Generate a report where Shell gathers system info, Python computes statistics, and Ruby builds a Markdown table.
- Prototype an idea using whichever language is most natural for each part of the task.

### Can I add my own runtime?

Not yet. Adding a new runtime currently requires modifying the salata-core source code and recompiling. The architecture is designed to make this straightforward in the future, but there is no plugin system today. If you want to add a runtime, the relevant code is in `crates/salata-core/src/runtime/`.

### What output formats does it support?

Any text format. Salata does not impose any structure on the output. Whatever the runtime blocks print to stdout becomes the output. Common formats include:

- HTML (the most common use case for web serving)
- JSON (using `#content-type application/json`)
- YAML, TOML, INI, and other configuration formats
- CSV and TSV
- Plain text reports
- Markdown
- Source code (you can use Salata to generate code)

### Does it work on Windows?

Salata compiles and builds on Windows. However, runtime availability varies -- you need the language interpreters installed and accessible. The shell sandbox uses Unix-specific features, so shell blocks may behave differently or not work on Windows. For a consistent experience across platforms, the Docker playground is recommended.

### Is the shell sandbox secure enough for production?

The shell sandbox implements defense-in-depth with three phases (static analysis, environment hardening, runtime monitoring), but it is not a complete security boundary. Static analysis can be bypassed by sufficiently creative encoding or obfuscation. Do not run untrusted shell code in production. The sandbox is designed to prevent accidental damage and block common attack patterns, not to withstand a determined attacker.

### Why can't I use /dev/null in shell blocks?

The shell sandbox blocks all references to the `/dev` path to prevent access to `/dev/tcp`, `/dev/udp`, and other dangerous device files that Bash can use for network connections. `/dev/null` is collateral damage of this broad block. This is an intentional trade-off -- granular path matching (allow `/dev/null` but block `/dev/tcp`) would be more complex and more likely to have bypass vulnerabilities.

Similarly, `2>&1` is blocked because the `&` character triggers the backgrounding check. The scanner does not distinguish between `&` (background a process) and `>&` (redirect file descriptors).

---

## Troubleshooting

### How do I debug .slt files?

1. **Check log files.** Each runtime has its own log file in the `logs/` directory (e.g., `python.log`, `ruby.log`, `javascript.log`). Errors, warnings, and execution details are written here regardless of the `display_errors` setting.

2. **Enable inline errors.** Set `display_errors = true` in the `[salata]` section of `config.toml` (or per-runtime in `[runtimes.*]`). When enabled, runtime errors are included in the output at the position of the failed block, making it easy to see what went wrong and where.

3. **Test with the CLI.** Use the `salata` CLI binary to process individual `.slt` files. This eliminates HTTP and server variables from the equation and shows you the raw output.

4. **Simplify.** If a multi-runtime file is failing, isolate the problem by testing each runtime block in its own `.slt` file.

### salata refuses to start -- "no config found"

All four Salata binaries require a `config.toml` file. They look for it in two places:

1. The path specified by the `--config` flag.
2. A file named `config.toml` in the same directory as the binary.

If neither exists, Salata refuses to start. Create a `config.toml` or use the `--config` flag to point to one. You can use `scripts/detect-runtimes.sh` (Linux/macOS) or `scripts/detect-runtimes.ps1` (Windows) to auto-generate a config file that detects your installed runtimes.

### "Runtime 'python' is disabled in config.toml"

The runtime you are trying to use has `enabled = false` in its `[runtimes.*]` section. Either enable it in `config.toml` or remove the corresponding blocks from your `.slt` file.

### "No runtimes enabled"

All runtimes are disabled in your `config.toml`. Enable at least one runtime to process `.slt` files.

### My shell block was rejected but the code looks safe

The shell sandbox uses broad pattern matching. Common false positives include:

- Using `>/dev/null` (blocked by the `/dev` path check)
- Using `2>&1` (blocked by the `&` character check)
- Using `command &` for backgrounding (blocked by the `&` check)
- Referencing `/etc/hostname` or other `/etc` paths (blocked by the `/etc` path check)

Consider using a Python or Ruby block instead for tasks that need these capabilities.

### Cross-runtime data is not working

Verify that:

1. The `#set` block executes before the `#get` block (execution is top-to-bottom).
2. The key names match exactly (they are case-sensitive).
3. The data is JSON-serializable (strings, numbers, booleans, arrays, objects, null).
4. You are not using `#set`/`#get` in shell blocks (shell macro expansion currently produces invalid syntax -- this is a known issue).
