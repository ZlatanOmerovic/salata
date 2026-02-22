---
paths:
  - "crates/salata-core/src/parser.rs"
  - "crates/salata-core/src/directives.rs"
  - "crates/salata-core/src/macros.rs"
---
# Parser Rules

- Use standard HTML parsing — runtime tags are just HTML elements named `python`, `ruby`, `javascript`, `typescript`, `php`, `shell`
- Reject nested runtime tags at parse time (e.g., `<php><python>...</python></php>`)
- `<style>` and `<script>` are client-side — pass through untouched
- Directives (`#include`, `#status`, `#content-type`, `#header`, `#cookie`, `#redirect`) appear outside runtime blocks only
- `#status` and `#content-type` can appear only once per page — multiple = parse error
- `#include` max depth is 16 levels — error on deeper recursion
- `#set`/`#get` macros appear inside runtime blocks only — expand into native code per language
- Included files participate in shared scope as if inline
