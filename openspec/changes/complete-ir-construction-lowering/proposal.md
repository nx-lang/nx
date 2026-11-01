## Why

Interpreter retirement left conditional property fragments unsupported by executable lowering and
positional record calls failing at runtime. Resolve those gaps with different language decisions:
implement the existing conditional-property semantics, but require named element construction for
records instead of introducing an implicit positional constructor based on field order.

## What Changes

- **BREAKING:** Reject parenthesized construction of named records during shared source analysis,
  with a stable diagnostic, a source span and a short named-element example; no generated rewrite.
- Keep `<User name="Bob" age=42 />` and omitted-field defaults; do not change ordinary functions,
  factory functions returning records, or first-class function invocation rules.
- Compile simple conditional, condition-list and match-style property fragments with lazy active
  property selection, narrowing, evaluation order, defaults and path-sensitive validation preserved.
- Activate the five ignored conditional success tests and replace the ignored positional-record
  success test with active rejection and equivalent-markup success coverage.
- Verify Rust/TypeScript IR and generated JavaScript against explicit expectations. Update docs and
  retire the temporary construction exception from the main runtime spec when implementation lands.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `source-analysis-pipeline`: Reject positional record construction consistently before codegen.
- `executable-code-generation`: Execute conditional properties through all supported backends.
- `rust-ir-runtime`: Close the temporary retirement exception with conditional success and an
  intentional source-language rejection, rather than promising positional support.

## Impact

Shared checker/diagnostics, compiler lowering, conformance tests and documentation. Runtime/image
changes are conditional on the selected representation and must be specified before implementation.
No source-evaluation API or HIR evaluator returns. Existing source using `User("Bob", 42)` must use
`<User name="Bob" age=42 />` or an explicitly declared factory function. Broader restrictions on
parentheses versus element calls for functions are a separate language-design decision.
