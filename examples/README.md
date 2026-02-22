# Salata Examples

Practical examples demonstrating Salata's features across CLI and web use cases.

## CLI Examples

Examples that run from the command line with `salata <file.slt>`.

| Example | Description | Runtimes |
|---------|-------------|----------|
| [hello-world](cli/hello-world/) | One file per runtime, minimal "Hello from X!" | All 6 |
| [data-processing](cli/data-processing/) | CSV tables, JSON filtering, system reports | Python, Ruby, Shell |
| [config-generator](cli/config-generator/) | Generate nginx.conf from computed values | Python, Shell |
| [markdown-report](cli/markdown-report/) | Project status report as pure Markdown | Python, Ruby, Shell |
| [cross-runtime-pipeline](cli/cross-runtime-pipeline/) | Data flows between runtimes via #set/#get | Python, Ruby, JavaScript |
| [scope-demo](cli/scope-demo/) | Shared vs isolated scope behavior | Python |
| [json-api-mock](cli/json-api-mock/) | JSON API response with #content-type | Python, JavaScript |
| [multi-format](cli/multi-format/) | Same data as text, CSV, and YAML | Python |

## Web Examples

Examples designed for `salata-server <dir> --port 3000`.

| Example | Description | Runtimes |
|---------|-------------|----------|
| [single-file](web/single-file/) | Directives: status, redirect, headers, cookies | Python |
| [portfolio](web/portfolio/) | Multi-page site with #include, static CSS | Python, Ruby |
| [dashboard](web/dashboard/) | All runtimes + #set/#get on one page | Python, Ruby, JS, Shell |
| [php-showcase](web/php-showcase/) | PHP alongside Python on the same page | PHP, Python |
| [api-endpoint](web/api-endpoint/) | JSON API with #content-type and #status | Python |
| [error-pages](web/error-pages/) | Custom 404/500 with dynamic content | Python, Shell |
| [blog](web/blog/) | Mini blog: Python reads files, Ruby formats | Python, Ruby |

## Running Examples

### CLI

```bash
cd examples/cli/hello-world
salata --config config.toml python.slt
```

### Web

```bash
cd examples/web/portfolio
salata-server . --port 3000
# Open http://localhost:3000/index.slt
```

### Docker

All examples work out-of-the-box in the Docker playground since config paths use standard Linux locations.

## Each Example Contains

- `.slt` files — the templates
- `config.toml` — minimal config with only required runtimes enabled
- `README.md` — what the example demonstrates and how to run it
