# Review: add-action-support

All tests pass. The change is well-structured — grammar, HIR, interpreter, tests, docs, and
openspec artifacts are consistent. Issues below are ranked roughly by importance.

## Issues

- [x] **R-01: `RecordDef` AST wrapper silently collapses action and record identity.**
  `crates/nx-syntax/src/ast.rs:156-160` — `RecordDef::can_cast` accepts both
  `RECORD_DEFINITION` and `ACTION_DEFINITION`, which means any code iterating CST children
  via `RecordDef::cast()` will see actions as records with no way to tell them apart.
  The HIR lowering avoids this because it matches on `SyntaxKind` directly, but any future
  CST-level consumer (e.g., a refactoring tool or linter) would lose the distinction.
  Consider either (a) adding an `ActionDef` AST wrapper that also exposes `name()`/`properties()`,
  or (b) adding a `RecordDef::is_action()` helper that checks `self.syntax.kind()`.

- [x] **R-02: `emits_group` field renamed from `"definitions"` to `"entries"` without migration note.**
  `crates/nx-syntax/grammar.js:258` — the tree-sitter field name changed. No existing Rust
  code uses `child_by_field("definitions")` today, so nothing breaks, but the rename is
  invisible in the diff description or commit message. External consumers (e.g., editor
  queries or downstream tree-sitter bindings) that were built against the prior grammar would
  silently get no results. Add a note in the openspec tasks or commit message that this is
  an intentional field rename.

- [x] **R-03: Validation test changed expected error kind without explanation.**
  `crates/nx-syntax/src/validation.rs:753-759` — the existing
  `test_component_emits_error_hint_uses_signature_context` test was modified to expect
  `"Invalid emits block"` instead of the previous `"Invalid component signature"`. This is
  presumably correct because the grammar change causes the parser to recover differently, but
  the test has no comment explaining why the expected diagnostic shifted. A one-line comment
  would make the intent clear for future readers.

- [x] **R-04: `lower_action_definition` is public but probably should be private.**
  `crates/nx-hir/src/lower.rs:915-917` — `lower_action_definition` is `pub` while the shared
  helper `lower_record_like_definition` it delegates to is `fn` (private). The only call site
  is `lower_module_items`. Consider making it `fn` (private) to match
  `lower_record_like_definition` and keep the API surface minimal. `lower_record_definition`
  is already `pub`, so this inconsistency stands out.

- [x] **R-05: Interpreter test mixes integration concerns into a unit-style test.**
  `crates/nx-interpreter/tests/simple_functions.rs:338-377` —
  `test_action_record_defaults_and_constructor_paths` calls `execute_function`, then re-parses
  the same source to call `lower` and `instantiate_record_defaults`. The re-parse duplicates
  work and tests two separate concerns (runtime execution vs. default instantiation) in one
  test. Consider splitting into two tests or extracting a shared parsed module.

- [x] **R-06: No test for qualified-name emit reference (`Namespace.ActionType`).**
  The grammar and design doc both mention that `emit_reference` uses `qualified_name` to
  support namespaced references like `Namespace.ActionType`, but no fixture or test exercises
  this path. A valid fixture with a dotted emit reference would confirm the grammar actually
  handles it.

- [ ] **R-07: Doc examples use inconsistent field separator style.**
  `docs/src/content/docs/language-tour/functions.md` — the action definition example uses
  `searchString:string` (no space around colon) while other examples in the same file use
  `query: string` (spaces around colon). Pick one style for consistency within the doc page.

- [ ] **R-08: `component.nx` example drops the `SearchRequested` action without replacement comment.**
  `examples/nx/component.nx` — the diff replaces the inline `SearchRequested` emit definition
  with the `SearchSubmitted` reference, but `SearchRequested` was used in the prior version's
  narrative. Readers comparing the old and new example may be confused by the silent rename.
  A brief comment in the example or commit message noting the intentional replacement would
  help.

## Verification (R-01 through R-06)

All six checked items verified against staged changes. Tests pass.

- **R-01:** Fixed via `RecordDef::is_action()` on the CST wrapper (`ast.rs:177`), with test
  assertions for both true and false cases.
- **R-02:** Fixed via migration note added to `tasks.md` under task 1.1 (currently unstaged).
- **R-03:** Fixed via explanatory comment added above the assertion in
  `test_component_emits_error_hint_uses_signature_context`.
- **R-04:** Fixed — `lower_action_definition` is now `fn` (private).
- **R-05:** Fixed — split into `test_action_record_literal_uses_defaults` (runtime) and
  `test_action_record_defaults_instantiation` (HIR instantiation).
- **R-06:** Fixed via new fixture `component-emits-qualified-reference.nx` using
  `SharedActions.SearchSubmitted` and corresponding parser test.

## Follow-up Notes

- **R-07:** I did not reproduce the specific inconsistency described here. The current
  `language-tour/functions.md` example block uses the compact `name:type` style consistently.
  The repo still has mixed annotation spacing elsewhere, though, including
  `examples/nx/component.nx`. Recommendation: handle this as a small docs-wide style cleanup
  once the preferred convention is explicit.

- **R-08:** I left `examples/nx/component.nx` unchanged. The existing leading comment already
  explains that the file demonstrates shared and inline actions, and adding a historical rename
  comment inside the sample would read more like changelog text than example code.
  Recommendation: if the `SearchRequested` to `SearchSubmitted` rename needs to be called out,
  capture it in the PR description or change summary rather than in the example file.
