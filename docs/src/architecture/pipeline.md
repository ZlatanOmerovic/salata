# Processing Pipeline

Every `.slt` file that Salata processes goes through the same pipeline, regardless of which binary initiated the request. This chapter describes each step in order.

## Pipeline Steps

### 1. Request Arrives

The pipeline starts when a `.slt` file needs to be processed. This can happen in several ways:

- **CLI**: the user runs `salata template.slt`
- **CGI**: a web server invokes `salata-cgi` with CGI environment variables pointing to a `.slt` file
- **Server**: `salata-server` receives an HTTP request for a `.slt` file

The invoking binary sets the `ExecutionContext` (Cli, Cgi, FastCgi, or Server) and passes it to salata-core along with the file path.

### 2. Read the .slt File

Salata reads the `.slt` file from disk. If the file does not exist, an error is returned (which becomes an HTTP 404 in web contexts or a stderr message in CLI mode).

### 3. Resolve `#include` Directives

The parser scans for `#include "file.slt"` directives and performs text substitution -- the directive is replaced with the entire contents of the referenced file. This is recursive: included files can themselves contain `#include` directives.

To prevent infinite recursion, there is a maximum depth of **16 levels**. If an include chain exceeds this depth, Salata produces a parse-time error.

Included files participate in the same processing pipeline. Their runtime blocks join the shared scope, and their directives are resolved alongside the main file's directives.

### 4. Resolve Response Directives

Salata scans for response-level directives and extracts them from the document:

| Directive       | Effect                                      | Multiplicity        |
|-----------------|---------------------------------------------|---------------------|
| `#status 404`   | Sets the HTTP response status code          | Once per page       |
| `#content-type application/json` | Sets the Content-Type header | Once per page       |
| `#header "X-Custom" "value"` | Adds a custom response header     | Multiple allowed    |
| `#cookie "name" "value" flags` | Sets a response cookie           | Multiple allowed    |
| `#redirect "/url"` | Sets a redirect response                 | Once per page       |

These directives are removed from the document content. They only affect HTTP response metadata. In CLI mode, `#status` and `#header` have no effect (there is no HTTP response), but `#content-type` can still be used to signal the intended output format.

If `#status` or `#content-type` appears more than once, it is a parse error.

### 5. Parse Content and Extract Runtime Blocks

The parser walks through the document and identifies runtime blocks:

```text
<python>code here</python>
<ruby>code here</ruby>
<javascript>code here</javascript>
<typescript>code here</typescript>
<php>code here</php>
<shell>code here</shell>
```

Each block is extracted with its position in the document (so the output can be spliced back later), the language, any attributes (like `scope="isolated"`), and the code content.

Client-side tags `<style>` and `<script>` are not runtime blocks. They are passed through untouched.

### 6. Validate: No Nested Runtime Tags

Salata checks that no runtime tag contains another runtime tag. This is a hard rule. The following is a parse-time error:

```html
<!-- INVALID: nested runtime tags -->
<python>
  print("<ruby>puts 'hello'</ruby>")
</python>
```

Nesting is never allowed, regardless of whether the inner tag appears in a string literal.

### 7. Check Runtime Enabled Status

For each runtime block found, Salata checks whether that runtime is enabled in `config.toml`. If a block uses a disabled runtime, Salata emits an error: `Runtime 'python' is disabled in config.toml`.

If every runtime is disabled, Salata prints: `No runtimes enabled. Enable at least one runtime in config.toml to process .slt files.` and exits with a non-zero status code.

### 8. Expand `#set`/`#get` Macros

Inside runtime blocks, `#set` and `#get` macros are expanded into native code for each language. For example:

```text
#set("count", 42)
```

might expand to something like (in Python):

```python
__salata_store["count"] = json.dumps(42)
```

The exact expansion is language-specific, but the effect is the same: data is serialized to JSON and stored in a shared data structure that Salata manages. `#get` expands to code that deserializes the stored value back into a native type.

### 9. Group Blocks by Language

Blocks are grouped by language. If shared scope is enabled (the default), all blocks of the same language will be sent to a single process.

### 10. Spawn or Reuse Processes

For each language with blocks to execute:

- **Shared scope (default)**: Salata spawns one process per language per page. All blocks for that language run in this single process.
- **Isolated scope**: Each isolated block gets its own process.

The process binary is determined by the runtime configuration and the current execution context (for PHP, this means selecting between `php`, `php-cgi`, or `php-fpm`).

### 11. Send Blocks with Boundary Markers

For shared-scope execution, Salata concatenates all blocks for a given language, separated by boundary markers:

```text
<code from block 1>
print("__SALATA_BLOCK_BOUNDARY__")
<code from block 2>
print("__SALATA_BLOCK_BOUNDARY__")
<code from block 3>
```

The boundary marker `__SALATA_BLOCK_BOUNDARY__` is a fixed string that Salata uses to split the output back into per-block segments. The concatenated code is sent to the runtime process's stdin.

### 12. Capture stdout Per Block

Salata reads the process's stdout and splits it on the boundary marker. This produces one output segment per block, maintaining the correct order.

If a runtime block produces an error (non-zero exit code, stderr output), Salata handles it according to the `display_errors` setting:

- **`display_errors = true`**: the error message is included in the output at the block's position
- **`display_errors = false`**: the error is suppressed in the output but still written to the log file

### 13. Splice Outputs Back Into Document

Each block's captured output replaces the original runtime tag in the document. The document is reassembled in order: static content, then block 1 output, then more static content, then block 2 output, and so on.

### 14. Send Final Output

The fully assembled document is written to stdout (in CLI mode) or sent as an HTTP response body (in CGI/Server mode), along with the response headers collected in step 4.

## Execution Order

Execution is **top-to-bottom** and **synchronous**. Each block finishes before the next one starts. Within shared scope, blocks for the same language maintain their document ordering. There is no parallel execution of blocks.

## Error Handling

### display_errors

The `display_errors` setting controls whether error messages appear in the output:

- Global setting: `[salata] display_errors = true`
- Per-runtime override: `[runtimes.python] display_errors = false`
- Resolution: runtime-specific setting takes precedence; global setting is the fallback

Regardless of the display setting, errors are always logged to the runtime's log file.

### HTTP Status on Error

If any runtime block fails during execution, the HTTP status code is automatically set to **500**, overriding any `#status` directive in the document. This ensures that errors are not silently served with a 200 status.

### Custom Error Pages

The `[errors]` section of `config.toml` allows custom error pages:

```toml
[errors]
page_404 = "./errors/404.slt"
page_500 = "./errors/500.slt"
```

These can be `.slt` files themselves, meaning error pages can contain dynamic content.

## Caching

Salata caches the parsed structure of `.slt` files -- block positions, include resolutions, and directive locations -- keyed by file path and modification time (`mtime`). When a file is modified, the cache entry is invalidated and the file is re-parsed on the next request.

This is a **parse cache**, not an output cache. The runtime blocks are always re-executed. Only the parsing work (steps 3 through 8) is cached.
