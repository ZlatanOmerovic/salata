# Introduction

Salata -- "salad" in Bosnian, Croatian, Serbian, and most Slavic languages -- is a polyglot text templating engine written in Rust. Like its namesake, it's a mix of everything thrown together: it processes `.slt` template files containing embedded runtime blocks -- `<python>`, `<ruby>`, `<javascript>`, `<typescript>`, `<php>`, and `<shell>` -- executes them server-side, and writes the combined output to stdout. The output is whatever the code prints: HTML, JSON, plain text, YAML, config files, Markdown, or anything else. HTML is the most common use case, but Salata is not limited to it.

> **Warning:** Salata is a concept project under active development. It is not production-ready. APIs, configuration format, and behavior may change between versions. Use it for experimentation, learning, and prototyping -- not for production workloads.

## What makes Salata different

Most templating engines are tied to a single language and a single output format. Salata takes a different approach:

- **Six runtimes in one file.** Python, Ruby, JavaScript, TypeScript, PHP, and Shell can all appear in the same `.slt` template. Each block runs in its native interpreter -- no transpilation, no emulation.

- **Cross-runtime data sharing.** The `#set` / `#get` macro system lets runtimes pass data to each other through Salata as a broker. Python can generate data, Ruby can transform it, and JavaScript can format the final output -- all in the same file, with JSON serialization handled transparently.

- **Output-agnostic.** Salata does not assume you are generating HTML. Runtime blocks print to stdout, and whatever they print becomes the output. Generate an nginx config with Python, a JSON API response with JavaScript, or a Markdown report with Ruby -- it all works the same way.

- **Context-aware PHP.** Salata automatically selects the right PHP binary based on the execution context: `php` for CLI, `php-cgi` for CGI, and `php-fpm` for FastCGI and the dev server. This mirrors how PHP itself works with different SAPIs.

- **Built-in shell sandbox.** The shell runtime includes a hardcoded security sandbox with command blacklists, blocked patterns, path restrictions, environment stripping, ulimit enforcement, and timeout monitoring. No external sandboxing tools required.

- **Four binaries, one core.** The `salata` CLI interpreter, `salata-cgi` bridge, `salata-fastcgi` daemon (stub), and `salata-server` dev server all share the same core library. Build once, deploy however you need.

## Who is Salata for

Salata is aimed at developers who want to:

- Experiment with polyglot templating and see how different languages can cooperate in a single document
- Learn about language interoperability, process management, and cross-runtime data exchange
- Build text-processing pipelines where each stage uses the best language for the job
- Prototype web pages that combine server-side logic from multiple languages
- Generate any kind of text output (configs, reports, data files) using familiar languages

## A quick look

Here is a `.slt` file that uses three runtimes to build a sales report. Python generates raw data, Ruby aggregates it, and JavaScript formats the output:

```html
<!-- Python generates the raw data -->
<python>
import json

sales = [
    {"product": "Widget A", "region": "North", "amount": 1200},
    {"product": "Widget B", "region": "South", "amount": 850},
    {"product": "Widget A", "region": "South", "amount": 2100},
    {"product": "Widget C", "region": "North", "amount": 675},
]

#set("raw_sales", sales)
</python>

<!-- Ruby aggregates by product -->
<ruby>
sales = #get("raw_sales")

totals = {}
sales.each do |sale|
  name = sale["product"]
  totals[name] ||= 0
  totals[name] += sale["amount"]
end

sorted = totals.sort_by { |_, v| -v }.map { |k, v| {"product" => k, "total" => v} }
#set("product_totals", sorted)
</ruby>

<!-- JavaScript formats the final report -->
<javascript>
const totals = #get("product_totals");

println("=== Sales Summary ===");
println("");
totals.forEach((item, i) => {
    println(`  ${i + 1}. ${item.product.padEnd(10)} $${item.total}`);
});
</javascript>
```

Run it:

```bash
salata report.slt
```

Output:

```text
=== Sales Summary ===

  1. Widget A   $3300
  2. Widget B   $850
  3. Widget C   $675
```

Three languages, one file, one command. Each runtime does what it does best.

## Project links

- **GitHub:** [github.com/nicholasgasior/salata](https://github.com/nicholasgasior/salata)
- **License:** See the repository for license details

## Next steps

- [Installation](./getting-started/installation.md) -- build Salata from source
- [Quick Start](./getting-started/quick-start.md) -- get running in 5 minutes
- [Your First .slt File](./getting-started/first-slt-file.md) -- write and run your first template
- [Playground Guide](./getting-started/playground.md) -- try Salata in a Docker container with all runtimes pre-installed
