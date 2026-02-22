// ---------------------------------------------------------------------------
// Uniform AST — Cross-Language Transpilation (Future)
// ---------------------------------------------------------------------------
//
// Status: NOT IMPLEMENTED. Placeholder module with design notes.
//
// ## Concept
//
// TypeScript is the first-class citizen. Its type system serves as the source
// of truth for classes and functions that are transpiled into other languages.
// A user defines a class or function in a <typescript> block, and Salata
// automatically makes it available in <python>, <ruby>, and <php> blocks
// within the same page.
//
// TypeScript → JavaScript is already covered (TS compiles to JS natively).
// The Uniform AST handles transpilation to the remaining target languages.
//
// ## Target Languages
//
// - Python  — class → Python class, methods → def, typed fields
// - Ruby    — class → Ruby class, methods → def, initialize constructor
// - PHP     — class → PHP class, methods → public function, typed properties
// - Shell   — EXCLUDED by design (no class/function model)
//
// ## AST Structure (Planned)
//
// The AST would represent a "Salata-compatible" subset of TypeScript:
//
// ```text
// Module
//   └── Declaration[]
//         ├── ClassDecl
//         │     ├── name: String
//         │     ├── fields: Vec<Field>
//         │     │     └── Field { name, type_ann, visibility }
//         │     ├── constructor: Option<Constructor>
//         │     │     └── Constructor { params: Vec<Param>, body: Vec<Statement> }
//         │     └── methods: Vec<Method>
//         │           └── Method { name, params, return_type, body }
//         └── FunctionDecl
//               ├── name: String
//               ├── params: Vec<Param>
//               ├── return_type: Option<TypeAnn>
//               └── body: Vec<Statement>
//
// Param { name: String, type_ann: TypeAnn }
//
// TypeAnn (type annotations)
//   ├── String
//   ├── Number
//   ├── Boolean
//   ├── Null
//   ├── Array(Box<TypeAnn>)
//   ├── Object(Vec<(String, TypeAnn)>)
//   └── Any
//
// Statement
//   ├── Assignment { target, value: Expr }
//   ├── Return { value: Expr }
//   ├── If { condition: Expr, then: Vec<Statement>, else_: Vec<Statement> }
//   ├── ForOf { item: String, iterable: Expr, body: Vec<Statement> }
//   └── ExprStatement(Expr)
//
// Expr
//   ├── Literal(Value)
//   ├── Identifier(String)
//   ├── BinaryOp { left, op, right }
//   ├── MethodCall { object, method, args }
//   ├── FieldAccess { object, field }
//   ├── TemplateLiteral { parts }
//   ├── ArrayLiteral(Vec<Expr>)
//   ├── ObjectLiteral(Vec<(String, Expr)>)
//   └── New { class_name, args }
// ```
//
// ## Transpilation Pipeline (Planned)
//
// 1. Parse TypeScript source using the `swc` crate (Rust-native TS parser)
// 2. Walk the SWC AST and convert into the Uniform AST subset above
// 3. Reject any constructs outside the "Salata-compatible" subset
// 4. For each target language, emit native source code from the Uniform AST
// 5. Prepend the generated code to the target language's runtime blocks
//    before execution (similar to how #set/#get preambles are injected)
//
// ## Limitations (By Design)
//
// These are intentional constraints, not missing features:
//
// - No language-specific features: no decorators, mixins, traits, protocols
// - No closures or higher-order functions
// - No async/await, Promises, generators, or iterators
// - No metaprogramming (eval, reflection, dynamic dispatch)
// - No standard library mapping (e.g., Array.map → list comprehension)
//   — this is a future consideration but explicitly out of scope initially
// - Only the "Salata-compatible" subset of TypeScript is supported;
//   unsupported constructs produce a clear parse-time error
// - Type annotations are used for transpilation hints, not runtime checking
//
// ## Dependencies
//
// This module CANNOT be implemented until:
//
// 1. The #set/#get macro data bridge is fully working (it is now).
//    The Uniform AST builds on the same cross-runtime communication
//    infrastructure — transpiled classes use #set/#get under the hood
//    to share state.
//
// 2. A TypeScript parser is integrated. The `swc_ecma_parser` crate
//    (from the SWC project) provides a Rust-native TypeScript parser
//    that can produce an AST without spawning Node.js. Add to Cargo.toml:
//    ```toml
//    swc_ecma_parser = "..."
//    swc_ecma_ast = "..."
//    swc_common = "..."
//    ```
//
// ## Code Organization (Planned)
//
// uniform_ast/
//   ├── mod.rs         — This file. Public API and module declarations.
//   ├── ast.rs         — Uniform AST type definitions (Module, Class, etc.)
//   ├── parse.rs       — SWC TypeScript AST → Uniform AST conversion
//   ├── validate.rs    — Reject unsupported constructs with clear errors
//   ├── emit_python.rs — Uniform AST → Python source code
//   ├── emit_ruby.rs   — Uniform AST → Ruby source code
//   └── emit_php.rs    — Uniform AST → PHP source code
//
// See specs/UNIFORM_AST.md for the full design document.
