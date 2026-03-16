## Why

`nx-grammar.md` changes braced expressions from a single-value interpolation form into a first-class
value-sequence construct and renames the relevant nonterminals. NX currently models those braces as
single-expression parser/runtime nodes, so the parser, lowering, type inference, interpreter, and
VS Code grammar would diverge from the language definition unless they are updated together.

## What Changes

- Replace the current single-value braced/interpolation model with `ValuesBracedExpression`, a
  brace-delimited sequence of one or more space-delimited values.
- Allow `ValuesBracedExpression` to contain any value kind, including elements, while keeping
  `ElementsBracedExpression` as the element-only form used in element child and control-flow
  positions.
- Define type-inference and compatibility rules so a braced expression with one source value of type
  `X` is treated as `X`, while a braced expression with multiple source values is treated as
  `list<X>`, with conversions applied where valid and semantic errors reported when narrowing from a
  list to a single value is not possible.
- Update parser node kinds, CST/HIR lowering, type checking, interpreter evaluation, and VS Code
  syntax support so the new braced-expression distinction is implemented consistently across NX.
- **BREAKING** Rename exposed parser node kinds and AST terminology that currently refer to
  `InterpolationExpression`, and update consumers that assume a braced expression always contains
  exactly one value.
- Add parser, lowering, interpreter, and editor tests that cover single-value braces, multi-value
  braces, element-valued braces, semantic compatibility failures, and invalid or ambiguous brace
  forms.

## Capabilities

### New Capabilities

- `braced-value-sequences`: Define syntax and semantics for `ValuesBracedExpression` as one or more
  values inside braces, including element-valued entries and the rule that source arity drives
  single-value versus list-valued inference.
- `typed-braced-expression-kinds`: Define the behavioral distinction between
  `ValuesBracedExpression` and `ElementsBracedExpression` across parsing, lowering, type
  compatibility, runtime evaluation, diagnostics, and editor tooling.

### Modified Capabilities

- None.

## Impact

- `crates/nx-syntax` grammar, generated parser artifacts, AST/node kinds, validation, and parser
  fixtures/snapshots.
- `crates/nx-hir` lowering logic and tests for braced expressions, control-flow bodies, and embedded
  expressions.
- `crates/nx-types` inference and compatibility rules for single-value versus list-valued braced
  expressions, including diagnostics for invalid coercions.
- `crates/nx-interpreter` runtime semantics for braced-expression results and sequence handling.
- `src/vscode` TextMate grammar, samples, and grammar tests.
- Downstream consumers of generated tree-sitter node kinds and syntax queries that reference
  `interpolation_expression`.
