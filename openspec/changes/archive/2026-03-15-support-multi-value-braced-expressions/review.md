# Review: support-multi-value-braced-expressions

All tests pass. The implementation is well-aligned with the design and specs.
Issues below are roughly ordered by severity within each category.

---

## Correctness

- [x] **R01 — `normalized_children.clone()` is redundant after `.is_some()` guard**
  In [interpreter.rs:626](crates/nx-interpreter/src/interpreter.rs#L626) and
  [interpreter.rs:700](crates/nx-interpreter/src/interpreter.rs#L700), the
  pattern `if normalized_children.is_some() { if let Some(v) = normalized_children.clone() { … } }`
  double-checks `Some` and needlessly clones `Value`. The outer guard already
  confirms `Some`, so the inner `if let` + `clone()` is dead logic. Use a single
  `if let Some(ref v) = normalized_children` or restructure to consume the value.

  Status: Fixed. The function and record child-injection paths now read
  `normalized_children` through `as_ref()` and only clone for the map insert.

- [x] **R02 — `child_values.clone()` is unnecessary**
  At [interpreter.rs:599](crates/nx-interpreter/src/interpreter.rs#L599),
  `normalize_child_values(child_values.clone())` clones the whole vec, but
  `child_values` is never used afterward. Pass by value instead.

  Status: Fixed. `child_values` is now moved directly into
  `normalize_child_values`.

- [x] **R03 — `eval_child_expressions` unconditionally flattens `Value::Array` results**
  In [interpreter.rs:720-728](crates/nx-interpreter/src/interpreter.rs#L720-L728),
  any child expression that evaluates to `Value::Array` has its items spliced
  into the parent list. The design doc says to "evaluate each child expression,
  normalize the resulting value(s), and populate `children`", which is consistent
  with flattening multi-value braces (which lower to `Expr::Array`). However,
  this also means a user-defined function that *returns* an array as a single
  child (e.g., `<Parent>{getItems()}</Parent>`) would have its result silently
  spliced rather than passed as one array-typed child. This may be fine for now
  given that the design focuses on braced-expression arity, but it's a semantic
  subtlety worth a comment or a targeted test confirming the intended behavior.

  Status: Fixed. Added an explicit interpreter comment documenting that child
  arrays are intentionally spliced because multi-item braces and element
  control-flow represent sibling child results, and added interpreter coverage
  for conditional and `for` child expressions.

- [x] **R04 — `lower_element_children` silently drops `TEXT_RUN` / `EMBED_TEXT_RUN` nodes**
  The catch-all `_ => {}` arm in [lower.rs:1099](crates/nx-hir/src/lower.rs#L1099)
  drops plain text content. This pre-dates this change and is outside the stated
  scope ("non-goal" territory), but now that `Element.children` is `Vec<ExprId>`
  there's an opportunity to represent text runs as string literal expressions.
  At minimum a `// TODO` or `// Intentionally skipped` comment would clarify.

  Status: Fixed. The skip arm now explicitly says plain text runs are
  intentionally ignored until HIR has a text child model.

- [ ] **R05 — `infer_element_expression` clones the full element to satisfy the borrow checker**
  At [infer.rs:199](crates/nx-types/src/infer.rs#L199), `element_ref` is
  `.clone()`'d because `&mut self` is needed for `infer_element_expression` while
  `self.module.element()` borrows `self`. This works but clones the entire
  `Element` (tag, properties, children) on every element expression. Consider
  passing `ElementId` and doing the lookup inside, or restructuring to avoid the
  borrow conflict.

  Status: Not fixed. Avoiding the clone cleanly requires reshaping the element
  inference helpers around `ElementId` or cloned field metadata, which is
  broader than the low-risk follow-up work done here.

## Duplication

- [x] **R06 — `check_element_bindings_against_function` and `check_element_bindings_against_record` are ~90% identical**
  [infer.rs:571-665](crates/nx-types/src/infer.rs#L571-L665). Both methods share
  the same children-conflict check, children-type check, and per-property type
  check. They differ only in how they resolve the "children" field and property
  types (function `Param` vs record `RecordField`). Extract a shared helper that
  takes a list of `(Name, TypeRef)` typed bindings and an optional children
  `TypeRef`.

  Status: Fixed. Both paths now build a shared `ElementBindingSpec` and route
  through one `check_element_bindings` helper in `nx-types::infer`.

- [x] **R07 — Interpreter duplicates type resolution and supertype logic from `nx-types`**
  The interpreter has its own parallel system: `runtime_type_from_type_ref`,
  `runtime_type_of_value`, `runtime_common_supertype`,
  `runtime_type_satisfies_expected`, `resolve_runtime_named_type`, and
  `is_object_type` in
  [interpreter.rs:780-960](crates/nx-interpreter/src/interpreter.rs#L780-L960).
  The type checker has near-identical logic (`common_supertype`,
  `type_satisfies_expected`, `is_object_type`, `type_from_type_ref`) in
  [infer.rs:705-780](crates/nx-types/src/infer.rs#L705-L780). The design
  (Decision 4) says both layers enforce the same coercion rules — having two
  independent implementations of those rules is a divergence risk. Consider
  extracting the shared pieces (supertype computation, type-ref resolution,
  "object" top-type check) into `nx-types` so both consumers share them.

  Status: Fixed. Extracted shared type-semantics helpers into `nx-types`
  (`common_supertype`, `type_satisfies_expected`,
  `type_satisfies_expected_with_coercion`, `is_object_type`,
  `resolve_type_ref_with`, and `resolve_type_ref_with_seen`) and switched the
  interpreter to use them.

- [x] **R08 — Interpreter children-injection for function vs. record paths is repeated**
  The function path ([interpreter.rs:602-670](crates/nx-interpreter/src/interpreter.rs#L602-L670))
  and record path ([interpreter.rs:673-706](crates/nx-interpreter/src/interpreter.rs#L673-L706))
  in `eval_element_expr` duplicate the same children-conflict / no-children-param /
  inject-children pattern (3 `.is_some()` checks + 2 `.clone()` calls each).
  Extract a helper.

  Status: Fixed. The function and record paths now share
  `inject_element_children_field`, which centralizes the conflict check,
  unsupported-body error, and `children` injection behavior.

## Grammar

- [x] **R09 — `_value_list_expression` 2-item minimum could use a clarifying comment**
  The hidden rule `_value_list_expression` uses
  `seq($.value_list_item_expression, repeat1(...))`, requiring 2+ items. This is
  correct for disambiguation with singleton `value_expression` (per spec), but
  the intent isn't immediately obvious in grammar.js. A one-line comment would
  help future readers.

  Status: Fixed. Added a grammar comment explaining that the 2-item minimum
  keeps `{value}` on the singleton path and avoids a one-item list ambiguity.

## Test coverage gaps

- [x] **R10 — No test for prefix unary expression rejected as a bare list item**
  The spec explicitly says "binary and prefix-unary expressions MUST be
  parenthesized" as list items. There's a test for rejected bare binary
  (`{a + b c}`) and for accepted parenthesized binary (`{(a - b) c}`), but no
  test confirming `{-x y}` is rejected or `{(-x) y}` is accepted. Add parser
  tests for both.

  Status: Fixed. Added parser tests for accepted `"{(-x) y}"` and rejected
  `"{-x y}"`.

- [x] **R11 — No interpreter test for `for` loop producing element children**
  The HIR lowering test `test_lower_dynamic_element_children_are_preserved`
  checks that `for item in items { <Row /> }` lowers correctly, but there's no
  corresponding interpreter test that evaluates a for loop producing element
  children through source parsing + execution.

  Status: Fixed. Added an interpreter test that evaluates
  `<collect>for item in items { <Row /> }</collect>` end to end.

- [x] **R12 — No interpreter test for conditional element children**
  Similarly, `if flag { <A /> } else { <B /> }` as element children is tested at
  the HIR level but not at the interpreter level through source parsing +
  execution.

  Status: Fixed. Added an interpreter test that executes conditional element
  children and checks both branches.

- [x] **R13 — No test for nested braced value sequences**
  No test covers `{ {a b} {c d} }` (nested braces) — either to confirm it
  parses correctly as a list containing two inner brace expressions, or to
  confirm it's rejected. This is an edge case users may attempt.

  Status: Fixed. Added a parser test confirming nested braced value sequences
  are currently rejected.

- [x] **R14 — No type checker test for `property-type-mismatch` on elements**
  The new `check_element_bindings_against_function` and
  `check_element_bindings_against_record` methods check property types, but
  there's no test in [check.rs](crates/nx-types/src/check.rs) that passes a
  wrong-typed property (e.g., `<Comp count="hello" />` where `count: int`) and
  verifies the `property-type-mismatch` diagnostic fires.

  Status: Fixed. Added a type-checker test that passes a string into an `int`
  element property and asserts `property-type-mismatch`.

- [x] **R15 — No type checker test for `children-binding-conflict`**
  The code emits a `children-binding-conflict` diagnostic when both a `children`
  property and body content are provided, but there's no test verifying this
  diagnostic fires from the type checker (the interpreter has its own test for
  the runtime error, but the static check is untested).

  Status: Fixed. Added a type-checker test that supplies both a `children`
  property and body content and asserts `children-binding-conflict`.

- [x] **R16 — No parser or HIR test for `@{<A/> <B/>}` (elements in embed braces)**
  The grammar and TextMate tests cover `@{user title}` (identifiers) and the
  TextMate test covers `@{user title <Badge/>}`, but there's no parser-level
  or HIR-level test for embed braces containing only elements.

  Status: Fixed. Added both a parser test and a lowering test for
  `@{<A/> <B/>}`.

- [x] **R17 — No test for empty braced expression `{}`**
  `lower_sequence_expr_from_children` handles the 0-children case by returning
  an error expression, but there's no test confirming `{}` produces the expected
  result (parse error or lowered error expr).

  Status: Fixed. Added a parser test confirming `{}` is rejected.

- [x] **R18 — Spec scenario "Multi-item braced value infers a list type" (`{1 2 3}`) is not directly tested**
  The braced-value-sequences spec says `{1 2 3}` should infer as `int[]`. The
  existing check.rs tests cover `{ 1 }` (scalar) and `{ <A /> <B /> }`
  (`object[]`), but there's no test for a homogeneous multi-item literal list
  like `{ 1 2 3 }` confirming it infers as `int[]` rather than `object[]`.

  Status: Fixed. Added a type-checker test verifying an unannotated `{ 1 2 3 }`
  body infers as `int[]`.

## Minor / style

- [x] **R19 — `find_kind` / `collect_kinds` helpers duplicated across test modules**
  [lower.rs:1254-1275](crates/nx-hir/src/lower.rs#L1254-L1275) and
  [parser_tests.rs:31-49](crates/nx-syntax/tests/parser_tests.rs#L31-L49) both
  define `find_kind` / `find_first_kind` and `count_kind` / `collect_kinds`
  helpers with identical logic. If useful across multiple test modules, consider
  a shared test utility.

  Status: Fixed. Extracted the shared syntax-tree traversal helpers into
  `crates/nx-syntax/tests/tree_helpers.rs` and reused them from both parser and
  lowering tests.

---

## Re-review findings

All 18 fixed items verified against the current code. All tests pass (0 failures).

- [x] **R20 — `semantics.rs` has no unit tests**
  The new [semantics.rs](crates/nx-types/src/semantics.rs) module contains the
  shared `common_supertype`, `type_satisfies_expected_with_coercion`,
  `resolve_type_ref_with`, and `builtin_type` functions. These are exercised
  indirectly through the type checker and interpreter tests, but there are no
  direct unit tests for edge cases like:
  - `common_supertype(int, float)` returning `float` (numeric promotion)
  - `common_supertype(int[], float[])` returning `float[]` (nested promotion)
  - `type_satisfies_expected_with_coercion(int, int[])` returning true (scalar-to-list)
  - `type_satisfies_expected_with_coercion(int[], int)` returning false (list-to-scalar rejection)
  - `builtin_type` case-insensitivity for names like "String", "INT", "Bool"

  Since this module is now the single source of truth for both the type checker
  and interpreter, direct tests would catch regressions that integration tests
  might miss.

  Status: Fixed. Added direct `semantics.rs` unit tests for numeric-width
  promotion, nested array promotion, scalar-to-list acceptance, list-to-scalar
  rejection, builtin-name case-insensitivity, and `resolve_type_ref_with`
  builtin-plus-callback resolution.

- [x] **R21 — `inject_element_children_field` still clones for `children` insert**
  At [interpreter.rs:718-719](crates/nx-interpreter/src/interpreter.rs#L718-L719),
  `children_value.clone()` is needed because the method takes
  `&Option<Value>`. This is correct given the method signature (both function
  and record paths may need the same value, but only one path runs per call).
  However, the caller at line 609 already owns the `Option<Value>` and
  only passes it to this method. If the method took `Option<Value>` by value
  instead of by reference, the clone could be eliminated entirely. Low priority
  but worth noting since `Value::Array` children can be large.

  Status: Fixed. `inject_element_children_field` now takes `Option<Value>` by
  value, computes conflict checks up front, and inserts the owned `children`
  value directly without cloning.

---

## VS Code extension review

The VS Code extension changes consistently rename `interpolation` →
`values-braced-expression` / `embed-braced-expression` across the TextMate
grammar, snippets, sample file, README, TODO, and tests. All 62 extension
tests pass.

- [x] **R22 — Snippet prefix `nxint` still suggests "interpolation"**
  The snippet ([snippets/nx.json:89](src/vscode/snippets/nx.json#L89)) was
  renamed from "Interpolation" to "Braced Value" and its description updated,
  but the trigger prefix is still `nxint`. Users typing "nxint" expect
  "interpolation", not "braced value expression". Consider renaming the prefix
  to `nxbv` or `nxbrace` to match the new terminology, or at minimum adding
  `nxbrace` as a second prefix so the snippet is discoverable under the new
  name.

  Status: Fixed. The snippet trigger now uses `nxbrace`, matching the updated
  braced-expression terminology.

- [ ] **R23 — `basic.nx` sample uses member access in embed braces without test coverage**
  [samples/basic.nx:16](src/vscode/samples/basic.nx#L16) uses
  `@{user.firstName user.lastName}` — member access expressions inside an
  embed brace list. The TextMate grammar test
  `tokenizes multi-value @{ } regions with element items` covers identifiers
  and elements inside `@{ }`, but there's no test confirming that
  `user.firstName` receives the expected scope (e.g., the `.` gets
  `punctuation.accessor` or similar). If member access isn't intended to work
  in the sample, the sample should be simplified; if it is, a test should
  confirm highlighting.

- [ ] **R24 — `basic.nx` sample attribute `class={"card" className}` is a multi-value brace in string attribute position**
  [samples/basic.nx:5](src/vscode/samples/basic.nx#L5) uses
  `class={"card" className}`. This parses correctly through `rhs_expression` →
  `values_braced_expression` in the tree-sitter grammar, but semantically it
  produces an `object[]` (`[string, string]`) value for a `class` attribute
  that presumably expects a string. This is fine as a showcase of the grammar
  feature, but it might confuse readers who expect `class` to be a single
  string. Consider adding a brief inline comment to the sample explaining the
  intent (e.g., "multi-value braces produce an array; the runtime joins class
  segments").
