# Cross-Runtime Pipeline

This chapter walks through the cross-runtime pipeline example in detail. It demonstrates Salata's core superpower: different programming languages working together in a single file, passing data between each other via the `#set`/`#get` macro system.

The example lives at `examples/cli/cross-runtime-pipeline/pipeline.slt`.

---

## The Full Pipeline

The pipeline flows through three runtimes: **Python** generates raw data, **Ruby** aggregates and transforms it, and **JavaScript** formats the final report.

```html
<!-- Step 1: Python generates raw data and stores it with #set -->
<python>
import json

# Raw sales data
sales = [
    {"product": "Widget A", "region": "North", "amount": 1200},
    {"product": "Widget B", "region": "South", "amount": 850},
    {"product": "Widget A", "region": "South", "amount": 2100},
    {"product": "Widget C", "region": "North", "amount": 675},
    {"product": "Widget B", "region": "North", "amount": 1450},
    {"product": "Widget C", "region": "South", "amount": 920},
]

# Store for the next runtime to pick up
#set("raw_sales", sales)
</python>

<!-- Step 2: Ruby transforms the data — aggregates by product -->
<ruby>
sales = #get("raw_sales")

# Aggregate totals per product
totals = {}
sales.each do |sale|
  name = sale["product"]
  totals[name] ||= 0
  totals[name] += sale["amount"]
end

# Sort by total descending
sorted = totals.sort_by { |_, v| -v }.map { |k, v| {"product" => k, "total" => v} }

# Store aggregated data for the next runtime
#set("product_totals", sorted)
#set("grand_total", totals.values.sum)
</ruby>

<!-- Step 3: JavaScript formats and presents the final output -->
<javascript>
const totals = #get("product_totals");
const grandTotal = #get("grand_total");

println("=== Sales Summary Report ===");
println("");

totals.forEach((item, i) => {
    const pct = ((item.total / grandTotal) * 100).toFixed(1);
    const bar = "#".repeat(Math.round(pct / 2));
    println(`  ${i + 1}. ${item.product.padEnd(10)} $${item.total.toString().padStart(6)}  ${pct}%  ${bar}`);
});

println("");
println(`  Grand Total: $${grandTotal}`);
println("");
println("Pipeline: Python (generate) → Ruby (aggregate) → JavaScript (format)");
</javascript>
```

---

## Step-by-Step Walkthrough

### Step 1: Python Generates the Raw Data

The Python block creates a list of sales records -- each record is a dictionary with `product`, `region`, and `amount` fields. This represents raw transactional data that needs to be processed.

At the end of the block, `#set("raw_sales", sales)` stores the Python list for other runtimes to access. Before execution, Salata expands this macro into native Python code that serializes the `sales` list to JSON and writes it to a temporary file managed by Salata.

The Python block itself produces no stdout output. Its only purpose is to generate and store data.

### Step 2: Ruby Retrieves and Transforms the Data

The Ruby block starts with `#get("raw_sales")`, which Salata expands into native Ruby code that reads the JSON file written by the Python block and deserializes it into a Ruby array of hashes. The Python list of dictionaries becomes a Ruby array of hashes automatically -- JSON is the common interchange format, and each language gets its native data types.

Ruby then aggregates the sales by product name, summing the amounts. The result is sorted in descending order by total. Two values are stored for the next runtime:

- `product_totals` -- an array of hashes with `product` and `total` keys
- `grand_total` -- a single integer representing the sum of all sales

Like the Python block, this Ruby block produces no stdout output. It only transforms data and passes it along.

### Step 3: JavaScript Formats the Final Report

The JavaScript block retrieves both `product_totals` and `grand_total` using `#get`. The Ruby array of hashes becomes a JavaScript array of objects. The Ruby integer becomes a JavaScript number.

JavaScript then formats the data into a human-readable report with aligned columns, percentage calculations, and ASCII bar charts. This block uses `println()` -- one of the helper functions Salata injects into JavaScript and TypeScript runtimes -- to produce the output.

This is the only block that writes to stdout, and its output becomes the final result of the entire `.slt` file.

---

## How Salata Brokers the Data

Runtimes never communicate with each other directly. Salata acts as the broker:

1. When a `#set("key", value)` macro is encountered, Salata expands it into native code that serializes the value to JSON and writes it to a temporary file in Salata's temp directory.
2. When a `#get("key")` macro is encountered, Salata expands it into native code that reads the JSON file and deserializes it into the runtime's native data types.
3. Execution is strictly top-to-bottom. When the Ruby block runs, the Python block has already finished and its `#set` data is available. When the JavaScript block runs, both the Python and Ruby data are available.

The JSON serialization/deserialization is transparent to the developer. You work with native types in each language:

| Data | Python type | Ruby type | JavaScript type |
|------|-------------|-----------|-----------------|
| `raw_sales` | `list` of `dict` | `Array` of `Hash` | `Array` of `Object` |
| `product_totals` | `list` of `dict` | `Array` of `Hash` | `Array` of `Object` |
| `grand_total` | `int` | `Integer` | `Number` |

Supported types for cross-runtime data: strings, numbers, booleans, arrays/lists, objects/dicts/hashes, and null/nil/None.

---

## Running the Example

```bash
salata --config examples/cli/cross-runtime-pipeline/config.toml \
       examples/cli/cross-runtime-pipeline/pipeline.slt
```

Expected output:

```text
=== Sales Summary Report ===

  1. Widget A   $ 3300  45.8%  #######################
  2. Widget B   $ 2300  31.9%  ################
  3. Widget C   $ 1595  22.1%  ###########

  Grand Total: $7195

Pipeline: Python (generate) → Ruby (aggregate) → JavaScript (format)
```

---

## Key Takeaways

- **Use the best language for each task.** Python is great for data generation and computation. Ruby shines at data transformation with its expressive enumerable methods. JavaScript excels at string formatting and template literals.
- **Data flows through `#set`/`#get`, not through stdout.** Only the last step in the pipeline needs to produce output. Earlier steps just transform and pass data.
- **Execution is sequential.** Each block finishes before the next one starts. There is no race condition or synchronization to worry about.
- **Types are preserved across runtimes.** Lists stay as lists, numbers stay as numbers, strings stay as strings. JSON handles the conversion transparently.
