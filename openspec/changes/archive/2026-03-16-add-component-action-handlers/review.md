# Review: add-component-action-handlers

All workspace tests pass. The implementation covers the spec scenarios well.

---

## Re-review pass (latest)

All previously checked items (R01–R14, R16, R18, R19) are verified fixed. R15 and R17 remain
open.

Latest fix: ordinary record-typed function parameters now keep their prior pass-through semantics,
while handler-input normalization remains scoped to `validate_handler_input`; regression tests cover
both the restored ordinary-parameter behavior and the handler-input path.

- [x] **R18** — `normalize_external_record_value` (interpreter.rs:1246) creates a fresh
  `ExecutionContext::new()` to evaluate default expressions. This is correct for handler inputs
  (which have no ambient execution scope), but the same function is now also called from
  `coerce_value_to_resolved_type` (line 905) for **all** record-typed function parameters. If a
  record definition has a default expression that references a variable (e.g.
  `type Config = { timeout:int = defaultTimeout }`), the fresh context will fail with an undefined
  variable error during parameter coercion. The fix should either pass the caller's
  `ExecutionContext` through to `normalize_external_record_value` when coercing function arguments,
  or limit record normalization to the handler-input path only.

- [x] **R19** — The `coerce_value_to_resolved_type` refactoring (lines 898–922) now intercepts
  **every** `Value::Record` whose expected type is a named record definition, routing it through
  `normalize_external_record_value` → `build_record_value_from_definition` with
  `missing_operation: Some(…)`. This means any record value passed as a function parameter that is
  missing a non-nullable, non-default field will now produce a `MissingRequiredRecordField` error,
  whereas previously it would have passed through silently (the field would just be absent). This
  is arguably better behavior, but it is a semantic change beyond the scope of the action-handler
  feature. If intentional, add a test confirming the new stricter validation for ordinary record
  parameters. If unintentional, scope the record normalization to handler input paths only.

---

## Code duplication

- [x] **R01** — `to_nx_value` is duplicated verbatim between `nx-api/src/value.rs` and
  `nx-cli/src/json.rs`, including the identical `ActionHandler` arm and `fields_to_properties`
  helper. The `nx-cli` copy is private so it could import the `nx-api` version, or the conversion
  could live in `nx-value` itself. This change made the duplication worse by adding the same
  `ActionHandler` mapping in both files.

- [x] **R02** — The fallback `if let Some(value_node) … else error_expr` pattern is repeated four
  times inside `lower_element` property lowering (lines ~1409, ~1424, ~1430, ~1437 of `lower.rs`).
  Extract a small helper like `fn lower_value_or_error(&mut self, value_node: Option<SyntaxNode>,
  span: TextSpan) -> ExprId` and use it for all non-handler branches.

## Correctness / robustness

- [x] **R03** — `scope.rs:build_scopes` silently ignores `Item::Component(_)` and never defines
  a symbol for the component name. The lowering prepass makes component names available for handler
  binding, but the scope manager is unaware of component names. If scope-based diagnostics (e.g.
  unused-symbol warnings) or future resolution passes rely on `ScopeManager`, this gap will surface
  as a false positive. Add a `SymbolKind::Component` (or reuse `SymbolKind::Type`) and define the
  component in the root scope.

- [x] **R04** — `InferenceContext::infer_expr` returns `Type::Error` for `Expr::ActionHandler`
  (infer.rs). This is a reasonable stub for now, but should at minimum be documented with a TODO
  comment explaining that handler type inference is deferred. Without it, any expression-level type
  check that encounters a handler will silently report `Error`, which could mask real type errors in
  surrounding code.

- [x] **R05** — `Interpreter::infer_type` maps `Value::ActionHandler` to
  `Type::named("object")`. This is inconsistent with `value.type_name()` which returns
  `"action_handler"`. Pick one canonical name and use it in both places so runtime error messages
  are consistent.

- [x] **R06** — `validate_handler_input` resolves the record definition to confirm it is an action,
  but does **not** validate that the input record's fields match the expected action schema. A
  handler for `SearchSubmitted { searchString:string }` will happily accept a
  `<SearchSubmitted />` with no fields, and the body will crash with an unrelated
  "undefined variable" error when it tries `action.searchString`. Consider validating required
  fields up front and producing a clear "missing field" error.

## Missing test coverage

- [x] **R07** — No test for **variable shadowing** inside a handler body. The design doc calls out
  "add focused tests for shadowing and call-site parameters." Add a test where the surrounding scope
  defines a variable named `action` (or another name that the handler body also uses) to verify that
  the handler's implicit `action` binding correctly shadows the outer one, and that captured
  variables are snapshotted, not live-referenced.

- [x] **R08** — No test for a handler whose body **references a captured variable that was later
  mutated** in the surrounding scope. Since the implementation snapshots variables at handler
  creation time, a test should confirm the snapshot semantics explicitly.

- [x] **R09** — No test for **invoking a handler with a wrong action type name**. There is
  validation code (`validate_handler_input`) that checks `type_name != expected_action_name`, but
  no test exercises this path (e.g. invoking a `SearchSubmitted` handler with a `ValueChanged`
  action record).

- [x] **R10** — No test for **non-action record returned from handler body** (e.g. a plain `type`
  record, not an `action` record). The `ensure_action_result_value` checks `RecordKind::Action`,
  but the existing test (`renderWrong`) only tests a string return. A test returning a plain
  record would cover the `is_action` false branch.

- [x] **R11** — No **lowering test for shared emit references**. The lowering tests cover inline
  emits (`ValueChanged { value:string }`) but not the shared reference path
  (`emits { SearchSubmitted }` where `SearchSubmitted` is a standalone `action` declaration).
  Specifically, test that the `ComponentEmit.kind` is `Shared` and that the `action_name` equals
  the referenced action name rather than a qualified `Component.Action` form.

- [x] **R12** — No test for **duplicate emit names mapping to the same handler prop**. The
  `predeclare_component` code detects when two emits produce the same `on<Name>` handler prop and
  emits a diagnostic, but no test exercises this path.

- [x] **R13** — No test for `format_value` (format.rs) or `format_value_json_pretty` (json.rs)
  with an `ActionHandler` value. These are new display arms that are untested.

## Design / style

- [x] **R14** — `Component.handler_name_collisions` stores collision state from the prepass and
  is then consulted during element lowering. This mixes diagnostic concern into the data model. An
  alternative is to compute collisions inline during element lowering (the component signature is
  already available). If it stays, consider making it a `FxHashSet<Name>` instead of `Vec<Name>`
  since it is used for membership checks.

- [ ] **R15** — The `lower_element` property lowering block (lines ~1366-1442) is deeply nested
  (5+ levels). Consider extracting the handler-recognition logic into a dedicated method like
  `fn try_lower_handler_property(…) -> Option<ExprId>` that returns `Some(expr)` when a handler
  is recognized and `None` to fall through to normal property lowering.

- [x] **R16** — `Component` struct in `lib.rs` does not store the component body expression. The
  design doc says body semantics are deferred, but the component's render body is currently
  discarded entirely during lowering — `lower_component_definition` just returns the prepass
  signature. When body lowering is added later, this will need a second pass. A brief comment on
  `lower_component_definition` noting this is intentional would help future readers.

## Documentation

- [ ] **R17** — The reference doc (`reference/syntax/functions.md`) removed the "Components with
  Children" section that documented child slots and named fragments. This content is not specific
  to the old model and is still valid for `let`-based element functions. Either restore it under
  the `let` heading or move it to its own section.

## Notes On Deferred Findings

- **R15** — Partially improved by R02: the repeated fallback branch is now centralized in
  `lower_value_or_error`, but the handler-recognition logic is still nested inside
  `lower_element`. I left the larger extraction alone to avoid mixing structural refactors with the
  correctness and coverage fixes above. Recommendation: revisit if more handler lowering logic is
  added here.

- **R17** — Confirmed in spirit, but I did not restore the old section verbatim because the
  historical example used older child-slot syntax that does not match the current documentation
  surface. Recommendation: add a modern reference section for `children`-style slots under the
  `let` heading, and only reintroduce named-fragment documentation if that syntax is still intended
  to be user-facing.
