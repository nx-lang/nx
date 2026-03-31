## 1. Grammar and Editor Surface

- [x] 1.1 Update `nx-grammar.md`, `nx-grammar-spec.md`, and `crates/nx-syntax/grammar.js` for `abstract type` and `extends` record declarations, including abstract-derived record syntax
- [x] 1.2 Regenerate the tree-sitter outputs in `crates/nx-syntax/src/` and update syntax kinds, AST helpers, validation hints, and highlight queries for the `abstract` and `extends` keywords plus inherited record metadata
- [x] 1.3 Update the VS Code grammar, samples, and grammar tests in `src/vscode/` to highlight and validate abstract and derived record declarations

## 2. HIR and Record Resolution

- [x] 2.1 Extend `nx_hir::RecordDef` and lowering so record definitions preserve `is_abstract` and `base` metadata for abstract roots, abstract derived records, and concrete derived records
- [x] 2.2 Add shared record-resolution helpers that resolve base records, reject non-abstract bases, detect inheritance cycles, merge effective fields/defaults, and reject duplicate inherited field names
- [x] 2.3 Add lowering and HIR coverage for abstract roots, abstract inheritance chains, concrete derived records, and invalid base declarations

## 3. Type Checking and Runtime Semantics

- [x] 3.1 Update `crates/nx-types` to use effective inherited fields when checking record literals and element-style record construction
- [x] 3.2 Add module-aware record subtype checks in `crates/nx-types` so concrete derived records are accepted where abstract parent or ancestor record types are expected, and reject abstract record instantiation during analysis
- [x] 3.3 Update `crates/nx-interpreter` to apply inherited defaults, honor effective record shapes, reject abstract instantiation at runtime, and validate host-facing typed record coercion against record ancestry
- [x] 3.4 Add type-checker and interpreter tests for valid substitution, inherited defaults, duplicate inherited fields, invalid base usage, and abstract instantiation failures

## 4. Documentation, Examples, and Regression Coverage

- [x] 4.1 Update language documentation and examples, including `examples/nx/types.nx` and other record-oriented samples, to demonstrate abstract roots, abstract derived records, and concrete derived records
- [x] 4.2 Add parser fixtures and end-to-end regression coverage across syntax, HIR, type checking, and interpreter layers for valid and invalid record inheritance cases
