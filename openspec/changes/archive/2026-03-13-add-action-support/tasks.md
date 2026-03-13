## 1. Grammar and CST

- [x] 1.1 Add `action_definition` to `crates/nx-syntax/grammar.js`, include it in `module_definition`, and extend `emits_group` so it can parse both inline action definitions and emitted action references
- Note: `emits_group` intentionally renamed its tree-sitter field from `"definitions"` to `"entries"` because the group now mixes emitted action definitions and references. Downstream consumers using `child_by_field("definitions")` should switch to `"entries"`.
- [x] 1.2 Regenerate the tree-sitter outputs in `crates/nx-syntax/src/` and update `crates/nx-syntax/src/syntax_kind.rs` plus related node metadata for `action_definition`, emitted action references, and the `action` keyword
- [x] 1.3 Update parser validation and syntax highlighting so fallback hints and keyword/type classification cover top-level `action` declarations and mixed component `emits` entries

## 2. HIR and Record Compatibility

- [x] 2.1 Add an action/plain kind marker to HIR record definitions and lower `action_definition` nodes into `Item::Record` values that preserve action identity
- [x] 2.2 Update record-oriented consumers and helpers so action records work anywhere normal records already work, including item lookup, scope registration, element-shaped record literal lowering, and any interpreter record resolution paths that depend on `Item::Record`
- [x] 2.3 Add HIR and interpreter coverage that proves action declarations stay record-compatible while remaining distinguishable from plain records

## 3. Parser Coverage and Documentation

- [x] 3.1 Add valid and invalid parser fixtures plus unit tests for standalone `action` declarations, component `emits` references, and mixed `emits` definition/reference lists
- [x] 3.2 Update parser-facing documentation and `.nx` examples to describe `action` declarations, the action-versus-record distinction, and shared actions referenced from component `emits`
