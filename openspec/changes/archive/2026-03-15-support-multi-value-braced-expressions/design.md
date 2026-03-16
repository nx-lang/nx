## Context

`nx-grammar.md` replaces the old single-value interpolation concept with
`ValuesBracedExpression` and distinguishes it from `ElementsBracedExpression`. The current
implementation is not aligned with that model:

- `crates/nx-syntax/grammar.js` still exposes `interpolation_expression` and parses exactly one
  `value_expression` between braces.
- `crates/nx-hir/src/lower.rs` unwraps braced expressions directly to the single inner expression,
  so source arity is lost immediately.
- `crates/nx-types` understands arrays, but it has no concept of scalar-to-list coercion at binding
  sites and no diagnostics for list-to-scalar narrowing.
- `crates/nx-hir::Element` stores `children: Vec<ElementId>`, and lowering currently drops
  non-literal child content such as `if`, `for`, and braced child expressions instead of preserving
  them.
- `src/vscode` treats interpolation lexically and is insensitive to the internal AST shape, but the
  grammar tests assume the existing single-value brace behavior.
- `nx-grammar.md` now consistently applies the multi-value brace model in text forms as well:
  `TextContent` uses `ValuesBracedExpression`, and `EmbedBracedExpression` allows one or more
  `ValueExpression`s within `@{...}`.
- `nx-grammar.md` now also narrows which expressions can appear bare in a space-delimited list via
  `ValueListItemExpression`; more ambiguity-prone forms such as binary expressions must be
  parenthesized when used as list items.
- `nx-grammar-spec.md` is still a parser-facing grammar reference in the repository, so it must be
  updated alongside `nx-grammar.md` to avoid two incompatible descriptions of the language.

## Goals / Non-Goals

**Goals:**

- Parse `{ ... }` as one-or-more source values and rename CST node kinds to match the updated
  grammar terminology.
- Preserve the source distinction between a singleton braced value and a multi-value braced list so
  type inference can yield `X` for one source value and `X[]` for multiple source values.
- Apply scalar-to-list coercions only at semantic binding sites where a list is expected, and report
  diagnostics when a list value is used where only a single item is valid.
- Preserve dynamic element children in HIR/interpreter so braced values, `if`, and `for` bodies in
  element content survive lowering instead of being dropped.
- Update VS Code grammar/tests and language fixtures to cover single-value braces, multi-value
  braces, element-valued braces, and semantic failure cases.

**Non-Goals:**

- Introducing a second general-purpose collection literal syntax beyond the existing array/value
  constructs.
- Providing backwards-compatible tree-sitter node aliases for `interpolation_expression`; this
  repository is still free to make source-breaking parser API changes.
- Solving unrelated type-system gaps outside the coercion paths needed for braced value sequences.

## Decisions

### 1. Rename the CST surface and parse braced values as repeated expressions

`nx-syntax` will replace `interpolation_expression` with `values_braced_expression` and parse it as
either a single `value_expression` or a space-delimited sequence of `value_list_item_expression`
entries, matching the new `ValueListItemExpression` grammar. The element-only rule remains a separate
`elements_braced_expression` node used where the grammar requires element bodies rather than general
values. `embed_braced_expression` will keep its `@{...}` delimiter but adopt the same repeated-value
shape. Generated artifacts (`grammar.json`, `node-types.json`, `parser.c`, `syntax_kind.rs`,
queries, fixtures) will be regenerated in the same change.

Why this approach:

- It matches the updated grammar names directly instead of keeping translation layers in the parser.
- It keeps the distinction between "general values in braces" and "elements-only in braces" visible
  to downstream tooling.
- It keeps plain `{...}` and embedded `@{...}` aligned semantically while preserving their distinct
  lexical forms for text parsing and editor tooling.
- It preserves the long-term space-delimited list syntax while reducing parser ambiguity by
  requiring parentheses for list items that are not in the `ValueListItemExpression` subset.

Alternatives considered:

- Keep the old `interpolation_expression` name internally and only update documentation. Rejected
  because it would leave the parser API and the spec permanently out of sync.
- Collapse both brace forms into one CST node. Rejected because `ElementsBracedExpression` has a
  narrower contract and different downstream validation needs.
- Abandon space-delimited lists in favor of a separator-based syntax. Rejected because
  space-delimited sequencing remains the intended long-term source form.

### 2. Lower singleton braced values to the inner expression and multi-value braces to `Expr::Array`

HIR will not add a new dedicated "braced values" node. During lowering:

- a `values_braced_expression` with one child lowers to that child expression,
- a `values_braced_expression` with multiple children lowers to `Expr::Array`,
- an `elements_braced_expression` lowers to a list-producing expression using the same child
  machinery described below.

Why this approach:

- The language rule is defined by source arity, and existing `Expr::Array` already models "multiple
  runtime values" well.
- Singleton braces should behave like the contained value unless a surrounding typed context needs a
  list, so preserving a dedicated wrapper in HIR adds complexity without a new semantic category.

Alternatives considered:

- Add `Expr::BracedValues { values: Vec<ExprId> }`. Rejected for now because the needed behavior can
  be expressed with existing HIR plus context-sensitive coercion, and a new variant would expand the
  type checker and interpreter surface area without clear benefit.

### 3. Change element children from `Vec<ElementId>` to expression children

The current `Element.children: Vec<ElementId>` model is already too narrow for the grammar that NX
accepts. To support `ValuesBracedExpression` containing elements, plus existing `if`/`for` child
forms, HIR will change element children to `Vec<ExprId>` and lowering will preserve child
expressions instead of extracting only literal nested elements.

Interpreter element evaluation will then evaluate each child expression, normalize the resulting
value(s), and populate `children` as an array when a function/record/component accepts child
content.

Ordinary braced child expressions in markup remain general value expressions, not element-only
wrappers. That means `{...}` in element-body positions may evaluate to scalars, elements, or lists.
Whether the resulting value is compatible with the surrounding markup is a later semantic question
handled by type checking and runtime validation, not a parse-time restriction.

Why this approach:

- It fixes an existing semantic gap instead of adding one more special-case path that only works for
  literal child elements.
- It gives a uniform representation for `<Child/>`, `{ child }`, `{ childA childB }`,
  `if cond { <A/> } else { <B/> }`, and `for item in items { <Row/> }`.

Alternatives considered:

- Keep `Vec<ElementId>` and flatten only element-valued braces during lowering. Rejected because it
  still cannot represent child `if`/`for` forms or scalar child values that the grammar currently
  permits.

### 4. Add explicit sequence coercion rules at typed binding sites, not to generic compatibility

The new semantic rule is source-arity-sensitive, so it should not be encoded as a blanket change to
`Type::is_compatible_with`. Instead, `nx-types` and the interpreter will add explicit coercion-aware
assignment helpers used at the places where NX already has an expected type:

- function argument binding,
- record field/default checking,
- element property/children binding,
- annotated return types and other typed assignment points added during this change.

Rules:

- If the actual value is scalar `X` and the expected type is `X[]`, coerce by wrapping the value in
  a one-element list.
- If the actual value is `X[]` and the expected type is scalar `X`, report an error.
- If a multi-value brace produces a list, infer the most specific common element type available; if
  no more specific common type exists, fall back to `object`. For example,
  `{ <A/> <B/> }` infers as `object[]`.

Why this approach:

- It keeps ordinary array compatibility rules stable.
- It makes the coercion sites explicit and diagnosable instead of hiding them inside a general
  "compatible" predicate.
- It gives heterogeneous element/value lists a defined result type without introducing a new
  top-level "node" or "children" supertype into the language.

Alternatives considered:

- Extend `Type::is_compatible_with` so `X` is always compatible with `X[]`. Rejected because it
  would make array compatibility globally asymmetric and would leak braced-expression semantics into
  unrelated type comparisons.
- Reject multi-value braces when their items do not share a more specific type. Rejected because the
  intended fallback for that case is `object`.

### 5. Rename VS Code braced-expression scopes to match the updated grammar terminology

The TextMate grammar should rename its repository rules and exported scopes to match the new
grammar terms, using names such as `meta.values-braced-expression.nx`,
`meta.embed-braced-expression.nx`, and the corresponding punctuation scopes. The editor-facing
change remains lexical, but its terminology should no longer preserve the old interpolation model.

Why this approach:

- It keeps the parser, spec, tests, and editor layer on the same vocabulary instead of maintaining
  an unnecessary translation layer.
- The repository is still allowing source-breaking terminology changes, and the VS Code extension is
  early enough that scope churn is cheaper now than after wider adoption.
- Separate `values` versus `embed` braced-expression scopes make the plain `{...}` and typed-text
  `@{...}` forms more explicit to tests and downstream scope selectors.

Alternatives considered:

- Keep the old interpolation-era scope names. Rejected because it would leave the editor layer as
  the lone place still presenting obsolete grammar terminology.

## Risks / Trade-offs

- [Space-delimited lists still have edge cases even with `ValueListItemExpression`] -> Encode the
  restricted item subset directly in the tree-sitter grammar, allow full `ValueExpression` parsing
  for singleton brace contents, require parentheses for non-list-safe expressions when they appear
  as list items, and add parser tests for forms such as `{a b}`, `{a - b}`, `{(a - b) c}`, and
  `{foo + bar baz}`.
- [Element body support is broader than the current runtime model] -> Move children to expression
  children and add coercion/normalization at evaluation time rather than trying to preserve the old
  `Vec<ElementId>` model.
- [Downstream parser consumers will break on renamed node kinds] -> Regenerate parser artifacts and
  update in-repo queries/tests in the same change; do not attempt a compatibility alias layer.
- [Falling back to `object` reduces type precision for heterogeneous lists] -> Accept the fallbackd
  for generality, and rely on explicit annotations or more homogeneous source values when stricter
  typing is required.

## Migration Plan

1. Update `nx-grammar-spec.md` to match the `nx-grammar.md` changes, including
   `values_braced_expression`, `value_list_item_expression`, the text/embed brace forms, and the
   parenthesization rule for non-list-safe expressions in space-delimited lists.
2. Update the tree-sitter grammar and generated parser artifacts to introduce
   `values_braced_expression`, `value_list_item_expression`, and the renamed related node kinds.
3. Encode the parenthesization rule for non-list-safe expressions in brace lists and add parser
   coverage for accepted versus rejected list forms.
4. Refactor HIR element children to store expressions, then update lowering to preserve child
   expressions and to lower singleton versus multi-value braces using source arity.
5. Extend `nx-types` with coercion-aware binding checks and diagnostics for list-to-scalar misuse.
6. Extend the interpreter to apply the same coercion rules at runtime when binding typed parameters,
   record fields, and `children`.
7. Update VS Code grammar/tests and NX fixtures/snapshots across syntax, lowering, type checking,
   and interpreter coverage.

Rollback is just a normal source revert. There is no migration state to preserve because NX is not
maintaining a backwards-compatible parser API yet.
