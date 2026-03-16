## 1. Grammar and Parser Surface

- [x] 1.1 Update `nx-grammar-spec.md` and any in-repo grammar references to match `nx-grammar.md` for `ValuesBracedExpression`, `ValueListItemExpression`, `ElementsBracedExpression`, and the text/embed brace forms
- [x] 1.2 Update `crates/nx-syntax/grammar.js` to rename interpolation nodes, add `value_list_item_expression`, and enforce the parenthesization rule for non-list-safe brace items
- [x] 1.3 Regenerate tree-sitter outputs and update `SyntaxKind`, node-type mappings, and queries for the renamed braced-expression kinds
- [x] 1.4 Add parser fixtures and parser tests for singleton braces, space-delimited lists, text/embed brace lists, legal singleton binary expressions, accepted parenthesized list items, and rejected non-parenthesized binary list items

## 2. HIR and Lowering

- [x] 2.1 Change HIR element children to preserve child expressions rather than only `ElementId`s and update dependent helpers/APIs accordingly
- [x] 2.2 Update lowering so singleton `values_braced_expression` nodes become the inner expression, multi-item braces become `Expr::Array`, and embed/text brace forms follow the same arity-sensitive behavior
- [x] 2.3 Update lowering of element child content, `if`, and `for` bodies to preserve element-producing braced/control expressions and add lowering tests for dynamic child content

## 3. Type Inference and Runtime

- [x] 3.1 Add coercion-aware type helpers for scalar-to-list wrapping, list-to-scalar errors, common-item-type resolution, and `object` fallback for heterogeneous multi-value braces
- [x] 3.2 Update type inference and type checking for braced value sequences, typed binding sites, and semantic diagnostics for parsed-but-incompatible list/scalar uses
- [x] 3.3 Update interpreter evaluation to produce correct braced results, preserve child expression results, normalize `children`, and enforce the same coercion/error rules at runtime

## 4. Tooling, Documentation, and Coverage

- [x] 4.1 Update the VS Code TextMate grammar, samples, and grammar tests for multi-value braces, embed braces, renamed parser terminology, and preserved interpolation scopes
- [x] 4.2 Update remaining NX docs/examples that still describe `InterpolationExpression` or single-value-only braced expressions
- [x] 4.3 Add cross-layer coverage across syntax, HIR, types, and interpreter for `object[]` fallback, scalar/list coercion, element-valued brace lists, and renamed node kinds
