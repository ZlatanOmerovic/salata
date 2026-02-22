# Uniform AST — Salata (Future)

**Status: Not implemented. TODO with comments and placeholder module.**

## Concept

Cross-language function/class transpilation with **TypeScript as first-class citizen**. TypeScript's type system is the source of truth for classes/functions transpiled to other languages.

- TypeScript → JavaScript covered (TS compiles to JS)
- Target languages: Python, Ruby, PHP
- Shell excluded

## How It Would Work

Define in TypeScript, use everywhere:

```html
<typescript>
  class User {
    name: string;
    email: string;
    constructor(name: string, email: string) { this.name = name; this.email = email; }
    greet(): string { return `Hello, ${this.name}`; }
  }
</typescript>

<python>
  user = User("Alice", "alice@test.com")
  print(user.greet())
</python>

<ruby>
  user = User.new("Alice", "alice@test.com")
  puts user.greet()
</ruby>
```

## Limitations (By Design)
- No language-specific features (decorators, mixins, traits)
- No closures, async, metaprogramming
- No stdlib mapping (future consideration)
- Only "Salata-compatible" subset

## Dependencies
- `#set`/`#get` data bridge must be implemented first
- TypeScript parser integration needed (e.g., `swc` crate)

## Codebase
Placeholder: `crates/salata-core/src/uniform_ast/mod.rs` with comprehensive TODO comments.
