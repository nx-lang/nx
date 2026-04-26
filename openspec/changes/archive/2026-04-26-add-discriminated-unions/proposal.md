## Why

NX currently models closed symbolic choices with enums and polymorphic payloads with abstract record
inheritance, but it lacks a single source-level construct for closed, case-based data with
per-case payloads. Discriminated unions give authors a concise way to model UI state, domain
events, and result values while preserving NX's existing record-like `$type` payload model.

## What Changes

- Add discriminated union declarations using `type Name =` followed by required leading-pipe case
  declarations.
- Keep simple scalar choices on the existing `enum` keyword; discriminated unions are for
  record-like cases, including fieldless cases mixed with payload cases.
- Add union case construction using scoped element-style constructors such as
  `<LoadState.failed message={"Network unavailable"} />`, with scoped value syntax for fieldless
  cases such as `LoadState.idle`.
- Extend `if value is { ... }` matching so union case patterns narrow the matched identifier inside
  each arm without requiring an `as` binding in the first version.
- Support optional shared fields by allowing a union declaration to extend an abstract record.
- Preserve closed-union semantics across parsing, lowering, name resolution, type checking,
  interpretation, runtime value conversion, generated TypeScript, generated C#, the managed .NET
  binding, VS Code grammar support, examples, and documentation.
- Add diagnostics for invalid union declarations, missing or duplicate cases, invalid case
  construction, inaccessible case fields, and non-exhaustive union matches without an `else` arm.
- Update docs and fixtures to describe when to use `enum` versus discriminated unions.

## Capabilities

### New Capabilities

- `discriminated-unions`: Syntax, semantics, narrowing, construction, runtime behavior, and tooling
  support for closed record-like union case declarations.

### Modified Capabilities

- `runtime-output-format`: Canonical raw output for discriminated union case values uses the same
  `$type`-discriminated map shape as polymorphic records.
- `cli-code-generation`: Generated TypeScript and C# output includes exported discriminated unions
  and their cases.
- `dotnet-binding`: Managed raw-value and typed-model workflows support discriminated union values
  using the canonical `$type` payload shape.

## Impact

- Grammar and parser artifacts: `nx-grammar.md`, `nx-grammar-spec.md`, tree-sitter grammar,
  generated parser files, AST wrappers, validation, fixtures, snapshots, and parser tests.
- HIR and analysis: discriminated union declarations and cases in lowering, prepared bindings,
  symbol resolution, inheritance/shared-field validation, type inference, type checking, and
  diagnostics.
- Runtime/value model: interpreter construction and matching of union case values, case narrowing
  behavior, canonical `NxValue` conversion, JSON and MessagePack serialization.
- Code generation: exported type model, TypeScript generation, C# generation, cross-module imports,
  generated tests, and generated documentation examples.
- .NET host code: runtime serialization helpers and tests under `bindings/dotnet`.
- Tooling and docs: VS Code TextMate grammars, snippets, samples, language tour, reference syntax,
  examples, and OpenSpec coverage.
