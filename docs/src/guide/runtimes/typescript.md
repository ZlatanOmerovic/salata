# TypeScript Runtime

| Property       | Value                                               |
|----------------|-----------------------------------------------------|
| Tag            | `<typescript>`                                      |
| Output method  | `console.log()`, `process.stdout.write()`, `print()`, `println()` |
| Default binary | `/usr/bin/ts-node`                                  |
| Shared scope   | `true` (default)                                    |

## Overview

The TypeScript runtime executes code with full type-checking support. It receives the same injected `print()` and `println()` helpers as JavaScript. The key difference is the runner: by default Salata uses `ts-node`, but you can configure it to use `tsx`, `bun`, or `deno` instead.

## Injected Helpers

Identical to the JavaScript runtime, Salata injects two helpers before your code runs:

```typescript
const print = (...args: any[]) => process.stdout.write(args.join(' '));
const println = (...args: any[]) => process.stdout.write(args.join(' ') + '\n');
```

These are additive. `console.log()` and `process.stdout.write()` continue to work normally.

## Basic Usage

```html
<typescript>
interface Product {
  name: string;
  price: number;
  inStock: boolean;
}

const products: Product[] = [
  { name: "Laptop", price: 999.99, inStock: true },
  { name: "Tablet", price: 499.99, inStock: false },
  { name: "Phone", price: 699.99, inStock: true },
];

println("<ul>");
for (const p of products) {
  const cls = p.inStock ? "available" : "sold-out";
  println(`  <li class="${cls}">${p.name} - $${p.price.toFixed(2)}</li>`);
}
println("</ul>");
</typescript>
```

## Shared Scope

With shared scope enabled (the default), all `<typescript>` blocks on the same page share a single process. Interfaces, types, variables, and functions persist across blocks:

```html
<typescript>
interface User {
  name: string;
  email: string;
  role: "admin" | "editor" | "viewer";
}

function badge(role: User["role"]): string {
  const colors: Record<User["role"], string> = {
    admin: "red",
    editor: "blue",
    viewer: "gray",
  };
  return `<span style="color: ${colors[role]}">${role}</span>`;
}

const users: User[] = [
  { name: "Alice", email: "alice@example.com", role: "admin" },
  { name: "Bob", email: "bob@example.com", role: "editor" },
  { name: "Carol", email: "carol@example.com", role: "viewer" },
];
</typescript>

<h2>Team Members</h2>

<typescript>
for (const user of users) {
  println(`<div class="user">`);
  println(`  <strong>${user.name}</strong> ${badge(user.role)}`);
  println(`  <a href="mailto:${user.email}">${user.email}</a>`);
  println(`</div>`);
}
</typescript>
```

## Type Safety

TypeScript's type system catches errors at execution time rather than producing silent bugs:

```html
<typescript>
type Status = "pending" | "active" | "archived";

interface Task {
  title: string;
  status: Status;
  priority: number;
}

function renderTask(task: Task): string {
  const icons: Record<Status, string> = {
    pending: "[?]",
    active: "[*]",
    archived: "[-]",
  };
  return `${icons[task.status]} ${task.title} (priority: ${task.priority})`;
}

const tasks: Task[] = [
  { title: "Deploy v2", status: "active", priority: 1 },
  { title: "Write docs", status: "pending", priority: 2 },
  { title: "Fix bug #42", status: "archived", priority: 3 },
];

println("<pre>");
tasks
  .sort((a, b) => a.priority - b.priority)
  .forEach(t => println(renderTask(t)));
println("</pre>");
</typescript>
```

## Configurable Runner

The TypeScript runner is configurable. Set the `path` field to whichever runner you prefer:

| Runner    | Config `path` Example       | Notes                                    |
|-----------|-----------------------------|------------------------------------------|
| ts-node   | `/usr/bin/ts-node`          | Default. Widely used, requires Node.js.  |
| tsx       | `/usr/local/bin/tsx`        | Faster startup, esbuild-based.           |
| bun       | `/usr/local/bin/bun`        | All-in-one JS/TS runtime.                |
| deno      | `/usr/local/bin/deno`       | Secure-by-default runtime.               |

```toml
# Use tsx instead of ts-node
[runtimes.typescript]
enabled = true
path = "/usr/local/bin/tsx"
shared_scope = true
```

## Cross-Runtime Data Bridge

TypeScript works with the `#set` and `#get` macros just like other runtimes:

```html
<typescript>
interface Config {
  theme: string;
  maxItems: number;
  features: string[];
}

const config: Config = {
  theme: "dark",
  maxItems: 25,
  features: ["search", "export", "notifications"],
};
#set("config", config);
</typescript>

<python>
config = #get("config")
for feature in config["features"]:
    print(f'<div class="feature">{feature}</div>')
</python>
```

## Configuration

```toml
[runtimes.typescript]
enabled = true
path = "/usr/bin/ts-node"
shared_scope = true
```

| Field            | Type   | Default             | Description                                     |
|------------------|--------|---------------------|-------------------------------------------------|
| `enabled`        | bool   | `true`              | Enable or disable the TypeScript runtime        |
| `path`           | string | `/usr/bin/ts-node`  | Absolute path to the TypeScript runner           |
| `shared_scope`   | bool   | `true`              | All blocks share one process per page            |
| `display_errors` | bool   | (global fallback)   | Override the global `display_errors` setting     |

## Isolated Scope

To run a block in its own process:

```html
<typescript scope="isolated">
const x: number = 42;
println(x.toString());
</typescript>
```

Or set `shared_scope = false` in the config for all TypeScript blocks.

## Tips

- The injected `print()` and `println()` helpers are identical to those in the JavaScript runtime.
- If startup time matters, consider `tsx` or `bun` as the runner -- they are typically faster than `ts-node`.
- TypeScript blocks have access to the full Node.js ecosystem (when using ts-node or tsx).
- The working directory during execution is the directory containing the `.slt` file being processed.
