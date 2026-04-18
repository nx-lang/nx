## 1. Grammar And Validation

- [x] 1.1 Update `nx-grammar.md`, `crates/nx-syntax/grammar.js`, and regenerated parser artifacts so type references accept repeated postfix `[]` and `?` suffixes in source order.
- [x] 1.2 Update `crates/nx-syntax/src/validation.rs` and `crates/nx-syntax` fixtures/tests so `T[][]`, `T?[]`, `T[]?`, and `T?[]?` parse successfully anywhere type annotations are allowed, while redundant same-layer `?` suffixes are rejected.

## 2. Lowering And Type Semantics

- [x] 2.1 Update `crates/nx-hir` type lowering and any related helpers that still assume a single suffix so composed suffixes preserve `Array`/`Nullable` wrapper order.
- [x] 2.2 Update `crates/nx-types` and related display/analysis tests to lock in the distinction between nested lists, lists of nullable elements, and nullable lists, including unambiguous rendered type strings for wrapped edge cases.

## 3. Code Generation

- [x] 3.1 Add or update `crates/nx-cli/src/codegen/languages/typescript.rs` and `crates/nx-cli/src/codegen/languages/csharp.rs` coverage so generated aliases and fields preserve `T[][]`, `T?[]`, and `T[]?` correctly.
- [x] 3.2 Add or update `crates/nx-cli` generator and CLI integration tests for exported aliases and record fields that use nested-list and nullable-list annotations.

## 4. Documentation And Verification

- [x] 4.1 Update reference docs and examples so nested lists and nullable lists are documented consistently across grammar, type-system, and sequence reference material.
- [x] 4.2 Run `openspec validate support-nested-and-nullable-list-types` and targeted Rust test suites for `nx-syntax`, `nx-hir`, `nx-types`, `nx-interpreter`, `nx-api`, and `nx-cli`, then fix any regressions.
