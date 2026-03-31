## Why

NX record types currently support only flat declarations, which makes it hard to model shared structured data
without duplicating fields across related record definitions. Adding abstract record types and single inheritance
provides a simple, explicit reuse model that also enables substitutability in type-checked code.

## What Changes

- Add `abstract type` declarations for record types that define shared fields but cannot be instantiated directly.
- Add `abstract type Child extends Parent = { ... }` syntax for abstract record types that derive from other
  abstract record types.
- Add `type Child extends Parent = { ... }` syntax for non-abstract record types that derive from abstract record
  types.
- Enforce the language rules that only abstract record types may be extended, abstract record types cannot be
  instantiated, and non-abstract record types cannot be used as base types even though abstract inheritance chains
  are allowed.
- Add record subtyping semantics so values of a derived record type can be used where the parent abstract record
  type is expected.
- Update grammar docs, parser fixtures, HIR/lowering, type checking, interpreter behavior, VS Code support, and
  language documentation for the new syntax and semantics.

## Capabilities

### New Capabilities
- `record-type-inheritance`: Support abstract record declarations and single inheritance for record types,
  including parsing, lowering, type substitution, instantiation rules, editor support, and documentation.

### Modified Capabilities
- None.

## Impact

- Affects `nx-grammar.md`, `nx-grammar-spec.md`, and user-facing language documentation/examples.
- Affects `crates/nx-syntax` parsing, grammar fixtures, and syntax-tree support for `abstract` and `extends`.
- Affects `crates/nx-hir` item lowering and record metadata so inherited fields and abstract/base relationships are
  represented in HIR.
- Affects `crates/nx-types` and `crates/nx-interpreter` so abstract records are not directly constructed and
  derived records are accepted where parent record types are expected.
- Affects the VS Code extension syntax support, tests, and examples that demonstrate record type declarations.
