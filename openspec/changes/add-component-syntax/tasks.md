## 1. Grammar and CST

- [x] 1.1 Add `component_definition`, `component_signature`, `component_body`, `emits_group`, `emit_definition`, and `state_group` rules to `crates/nx-syntax/grammar.js`, and include components in `module_definition`
- [x] 1.2 Add `component`, `emits`, and `state` keyword handling, then regenerate the tree-sitter outputs in `crates/nx-syntax/src/`
- [x] 1.3 Update `crates/nx-syntax/src/syntax_kind.rs` and related node metadata for the new component nodes and keywords

## 2. Parser Validation

- [x] 2.1 Add valid parser fixtures and unit tests for minimal components, components with `emits`, and components with `state`
- [x] 2.2 Add parser coverage for malformed component syntax where the new grammar should reject or recover from invalid `emits` or `state` sections
- [x] 2.3 Update tree-sitter highlight coverage so `component`, `emits`, and `state` are tokenized as keywords in the new syntax

## 3. Documentation and Examples

- [x] 3.1 Update parser-facing grammar/reference docs to describe component declarations, emitted action groups, and state groups
- [x] 3.2 Add or refresh `.nx` examples that demonstrate the new component syntax and clearly note that lowering/interpreter support is deferred
