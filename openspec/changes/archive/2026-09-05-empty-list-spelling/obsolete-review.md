# Review: empty-list-spelling

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, and all three delta specs  
**Reviewed code:** grammar/parser regeneration, lowering, type inference/checking, formatter, type generators/codegen, language-service completions, documentation, and associated tests  

## Findings

### ✅ Verified - RF1 Nested empty lists are formatted into syntax that NX rejects
- **Severity:** Medium
- **Evidence:** `format_property_value` recursively wraps every `Value::Array` in braces at `crates/nx-cli/src/format.rs:115-123`. A runtime `string[][]` value containing an empty list therefore renders as `items={{}}`. But `values_braced_expression` is not a `value_expression` or a `value_list_item_expression` (`crates/nx-syntax/grammar.js:443-460`), and `<Box items={{}} />` reports a parse error. Before this change the inner empty list caused the formatter to fail; it now silently emits source that cannot round-trip, contrary to the `unbraced-literal-forms` requirement.
- **Recommendation:** Preserve an explicit formatting failure when an array is nested in another array until nested braced values are grammatical, or extend the grammar/lowering/type-checking and add an end-to-end nested-list round-trip test.
- **Fix:** Took the first option. `format_property_value` now returns `unspellable_nested_list()` when an array element is itself an array, on its own terms rather than as a side effect of an emptiness check. Confirmed emptiness is not the discriminator: `items={{"a" "b"}}` is a syntax error too, so the pre-existing non-empty case is now reported rather than silently emitted, which is stricter than the pre-change behavior the finding asks to restore. Three tests added (`test_format_empty_list_nested_in_a_list_is_unspellable`, `..._non_empty_list_...`, and `test_format_list_of_records_holding_lists_still_renders`, which pins that a record between the two braces still renders). The `unbraced-literal-forms` requirement gained a sentence and three scenarios, and `format_value`'s doc comment no longer claims the action handler is the only unspellable value.
- **Verification:** Confirmed the direct-array guard at `crates/nx-cli/src/format.rs:120-127` rejects both empty and non-empty nested arrays, while an intervening record remains renderable. The three formatter tests and the workspace suite pass.

### ✅ Verified - RF2 Task 2.4's argument and list-element sites are not implemented or tested
- **Severity:** Medium
- **Evidence:** Task 2.4 requires a test for `{}` at arguments and list elements (`tasks.md:34-37`), but `empty_lists.rs` covers neither (`crates/nx-types/tests/empty_lists.rs:39-120`). More importantly, neither grammar branch accepts a values-braced expression: call arguments use `value_expression`, and list items use `value_list_item_expression`, while both omit `values_braced_expression` (`crates/nx-syntax/grammar.js:443-460`). For example, `consume({})` is a parse error before the argument checker can provide its `string[]` parameter type.
- **Recommendation:** Either admit `values_braced_expression` in these expression positions and thread the expected type through them, with the required per-site tests, or revise task 2.4/design scope and mark it incomplete rather than completed.
- **Status (superseded):** First resolved as a scope correction, taking the second of the two options the finding offered. The finding's grammar analysis was confirmed — `f({})` and `f({"a"})` produced the identical syntax error, so the position rejected a braced value at every arity rather than rejecting the empty one.
- **Fix:** On direction, the *first* option instead: the argument half is implemented, and the list-element half stays out on its own merits. New tasks section 7 and design D8 carry it.
  - **Grammar.** `call_expression` now draws each argument from a hidden `_call_argument` rule, `choice(value_expression, values_braced_expression)`. Scoped to the argument list rather than added to `value_expression`, which would also have reached parenthesized expressions, binary operands, member-access targets and conditional arms. `tree-sitter generate` produces a conflict set identical to the one before the edit, compared order-insensitively, so the rule adds no ambiguity.
  - **Type checking needed no new inference.** `infer_call` already checks every argument through `check_typed_binding_for` with the parameter type — the same seam a property binding uses — so `f({})` takes its element type from the parameter and `f({"a"})` binds by the existing scalar-to-list coercion. Verified end to end: `echo({})`, `echo({"only"})`, `echo({"a" "b"})` evaluate to lists of 0, 1, and 2 elements.
  - **One diagnostic fix was needed.** `infer_call` returns early on an error callee, an argument-count mismatch, and a non-function callee. An empty argument would still be pending at those exits and drew a second diagnostic telling the author to annotate a binding — the same spurious-follow-up shape RF3 fixed at annotated bindings. All three exits now call a new `discharge_pending_empty_lists`, so a broken call reports once.
  - **List elements stay rejected, as a decision rather than a deferral.** At arity one the brace is a scalar, so a nested brace collapses: `{{"a" "b"}}` would mean a scalar holding a `string[]` and `{{}}` would mean what `{}` means, making a one-row `string[][]` unwritable while a two-row one is writable — silently, since the collapsed value still binds at a `string[]` site. Recorded in design D8 and proposal's "Not in this change".
  - **Tests.** Five parser tests (braced argument at each arity, every argument position, and four still-rejected nestings including `count({{"a"} "b"})`), eight type tests (element type from a parameter, singleton coercion, every position, nullable parameter, record constructor, non-list parameter reports only the mismatch, uncheckable calls report once), and one interpreter test pinning the runtime arity behavior. `braced-value-sequences` gained four parse scenarios and four typing scenarios; both grammar documents now spell the argument list `ValueOrValuesBracedExpression`.
- **Verification:** Fully reviewed the added grammar, generated node metadata, lowering path, type checker, parser/type/interpreter tests, and grammar documentation. `_call_argument` limits the new syntax to call arguments; it lowers directly into `Expr::Call`, and `infer_call` supplies the parameter type and discharges uncheckable calls. `echo({})`, `echo({"only"})`, and `echo({"a" "b"})` run successfully; nested braced list items remain rejected as deliberately specified. The targeted suites and `cargo test --workspace` pass.

### ✅ Verified - RF3 An already annotated non-list binding gets a spurious "annotate it" diagnostic
- **Severity:** Low
- **Evidence:** For `let value:string = {}`, `check_typed_binding_for` emits the correct type mismatch but leaves the expression in `pending_empty_lists` when the expected type is not an array (`crates/nx-types/src/infer.rs:2524-2539`). The final sweep then emits `empty-list-element-type-unknown` (`crates/nx-types/src/infer.rs:2698-2721`), instructing the user to annotate a binding that is already annotated. The element-type diagnostic is specified for sites with no expected type, not for a binding with an incompatible expected type.
- **Recommendation:** Consume/suppress the pending empty-list entry once a typed non-list binding has reported its mismatch, and add a regression test asserting this source reports only the actionable type mismatch.
- **Fix:** Done as recommended. `check_typed_binding_for` drops the pending entry on its failure path, just before it reports. That path is the right seam because every failure return from that function reports a diagnostic, so the entry is never dropped silently. Regression test `an_annotated_non_list_binding_reports_only_the_mismatch` asserts exactly one diagnostic and that it does not say "annotate".
- **Fix (adjacent, same line):** The surviving message read `expects string, found list T0[]`, leaking the element inference variable at an author who wrote `{}`. It now renders as `{}`, mirroring how `is_null_literal_type` is already special-cased two lines above, and the "found list" phrasing is suppressed for the empty list so it reads `expects string, found {}`. Test `a_mismatch_on_an_empty_list_does_not_name_the_element_variable`.
- **Verification:** Confirmed the failure path removes the pending expression before reporting, so `let value:string = {}` produces one actionable mismatch. Both added diagnostic regression tests pass.

## Questions
- None.

## Summary
- All three original findings are verified. RF2 was first resolved as a scope correction and then, on direction, implemented instead — a call argument now takes a braced value, while a list item still does not.
- `cargo test --workspace` and `openspec validate empty-list-spelling` pass. No new findings were identified.

## New Findings Discovered During 2026-09-04 21:24 Review

Independent second pass over the same artifacts and the full working tree. Scope of this pass:
grammar and regenerated parser, lowering, `nx-types` inference/checking, formatter, interpreter,
codegen and typegen, language service, documentation, and the added tests — plus behavioural probes
run against a freshly built `nxlang` (`cargo test --workspace` passes; `openspec validate
empty-list-spelling` passes).

### ✅ Verified - RF4 An empty braced value in markup child position type checks but is lost at runtime
- **Severity:** High
- **Evidence:** Design D4 states `<List>{}</List>` becomes legal and means "no children", the
  `braced-value-sequences` delta has a scenario for it, and task 2.4 lists element body content as a
  site to verify. The type checker accepts it — `crates/nx-types/tests/empty_lists.rs:63-68` asserts
  exactly that and nothing more — but the interpreter drops the value. `eval_content_expressions`
  splices a `Value::Array` into the parent content list
  (`crates/nx-interpreter/src/interpreter.rs:2668-2676`), so an empty list contributes zero values;
  `normalize_content_values` then maps a zero-length content list to `None`
  (`crates/nx-interpreter/src/interpreter.rs:2679-2685`), and `inject_element_content_field` returns
  without binding the content field (`crates/nx-interpreter/src/interpreter.rs:2628-2632`). The
  field falls through to `Value::Null` and fails coercion. Reproduced:
  - `type Box = { content items: string[] }` + `<Box>{}</Box>` → no type diagnostics, then
    `Runtime error: Type mismatch in record field 'items': expected string, got object?`
  - `content items: string[]?` + `<Box>{}</Box>` → runs, and silently yields `<Box items=null />`
    rather than an empty list, contradicting the delta's own "a nullable list site is a non-null
    empty list" rule (which is tested only in property position).
  - The same source generates correct TypeScript — `renderMain` returns `{ $type: "Box", items: [] }`
    — so the interpreter and the code generator now disagree about what the program means.
- **Recommendation:** Distinguish "no body content" from "body content that evaluated to an empty
  list" in the interpreter — e.g. have `eval_content_expressions`/`normalize_content_values` carry a
  present-but-empty result through to `inject_element_content_field` so the content property is bound
  to `Value::Array(vec![])`. Add an interpreter round-trip test for `<Box>{}</Box>` at both a
  `T[]` and a `T[]?` content field, and strengthen
  `empty_list_is_accepted_as_element_body_content` to evaluate rather than only type check.

- **Fix:** Done as recommended. `normalize_content_values` now takes the content expressions
  alongside their values, so it can tell a body that produced nothing from no body at all: the
  first binds `Value::Array(vec![])`, the second still returns `None` and leaves the content
  property to its default. Confirmed at both fields the finding names — `<Box>{}</Box>` at
  `content items: string[]` now evaluates to `<Box items={} />` instead of failing coercion, and at
  `string[]?` it binds the empty list rather than null, which is the delta's own "a nullable list
  site is a non-null empty list" rule that property position already followed. The interpreter and
  the code generator now agree: the same source generates `{ $type: "Box", items: [] }`. Three
  interpreter tests (`test_empty_body_content_binds_the_empty_list`,
  `..._at_a_nullable_list_field_is_not_null`, and `test_absent_body_content_leaves_the_content_
  property_to_its_default`, which pins the distinction from the other side). The typing requirement
  gained an evaluation scenario, since type checking alone was what let this through.

- **Verification:** Confirmed. `normalize_content_values` now takes the content expressions so a
  written body that produced nothing is told apart from no body at all
  (`crates/nx-interpreter/src/interpreter.rs:2679-2696`). Both sources the finding names now
  evaluate: `type Box = { content items: string[] }` + `<Box>{}</Box>` renders `<Box items={} />`
  instead of failing coercion, and at `string[]?` it binds the empty list rather than null. The
  three interpreter tests pin all three states, including the absent-body case from the other side.
  Note that the fix reaches further than `{}` — see RF12.

### ✅ Verified - RF5 A braced value beside other body content reports a spurious element-type diagnostic
- **Severity:** Medium
- **Evidence:** `check_content_binding` threads the expression id through only when the body has
  exactly one item (`crates/nx-types/src/infer.rs:2465-2473`); the multi-item path computes
  `normalized_sequence_type` and calls `check_typed_binding` with `None`
  (`crates/nx-types/src/infer.rs:2476-2490`), which cannot discharge the pending entry because
  `check_typed_binding_for` only removes it under `if let Some(expr)`
  (`crates/nx-types/src/infer.rs:2544-2553`). Reproduced: with
  `type Box = { content items: object[] }`, the source `<Box>{}<Foo/></Box>` reports
  `Cannot determine the element type of this empty list…` even though the content property is
  list-typed and would have supplied `object`.
- **Recommendation:** Discharge (and resolve) the pending empty lists among `content` on the
  multi-item path — the element type is available from `expected`'s array element there — and add a
  test for a braced value among sibling body content.

- **Fix:** Done as recommended, in two parts, because discharging alone would have traded the
  spurious diagnostic for a spurious mismatch. `check_content_binding`'s multi-item path now
  resolves the pending empty lists among `content` against the content property's element type. And
  `normalized_sequence_type` no longer joins an empty list's element variable into the sequence's
  item type: content is spliced, so `{}` beside a sibling is zero values beside it, and joining its
  variable dragged the item type to `object` — which passes at the `object[]` site the finding used
  but reports a mismatch at a `Badge[]` one. A sequence of nothing but empty lists is itself an
  empty list and still owes an element type. Two tests, one per site type.

- **Verification:** Confirmed, both halves. `check_content_binding`'s multi-item path resolves the
  pending empty lists against the content property's element type
  (`crates/nx-types/src/infer.rs:2527-2536`), and `normalized_sequence_type` filters empty lists out
  of the join (`crates/nx-types/src/infer.rs:2251-2269`). Probed at three sites: `<Box>{}<Foo/></Box>`
  at `object[]` is clean and evaluates to the sibling alone, the same body at a concrete `Badge[]`
  content field is clean (the case the fix note says a discharge-only fix would have broken), and
  `<Box>{}{}</Box>` at `Badge[]` binds the empty list.

### ✅ Verified - RF6 An empty list accepted at a non-list site is still reported by the end-of-analysis sweep
- **Severity:** Medium
- **Evidence:** `check_typed_binding_for` removes the pending entry on exactly two paths: the
  array-expected resolution (`crates/nx-types/src/infer.rs:2544-2553`) and the failure path before
  reporting a mismatch (`crates/nx-types/src/infer.rs:2569-2576`). A site that *accepts* the empty
  list without being array-typed hits neither. Reproduced: `type Box = { thing: object }` with
  `<Box thing={} />` — the binding is accepted (no mismatch), and the sweep then reports
  `Cannot determine the element type of this empty list; write it where a list type is expected, or
  annotate the binding it belongs to`. The advice does not apply: the author did write it at a site
  with an expected type. It also makes `{}` unwritable at an `object`-typed property while
  `{"a" "b"}` is accepted there, which is a surface asymmetry the delta does not claim.
- **Recommendation:** Decide the rule and state it in `braced-value-sequences`: either accept `{}` at
  a non-list site that admits a list (discharging the entry and resolving the element to `object`),
  or reject it there with a message naming the site's declared type instead of the generic
  annotate-the-binding advice. Add a test either way.

- **Fix:** Took the first of the two options. `check_typed_binding_for` now discharges on its
  accepting path too, resolving the element to `object`. The reasoning is the finding's own
  asymmetry argument: `{"a" "b"}` is accepted at an `object`-typed property, so refusing `{}` there
  would make emptiness the thing the site judges, which it is not — and a list with no elements has
  no element type such a site can observe. The rule is stated in `braced-value-sequences` rather
  than left implicit, and `empty_list_at_a_site_that_accepts_a_list_without_being_one` asserts both
  forms are clean at `type Box = { thing:object }`.

- **Verification:** Confirmed. The accepting path in `check_typed_binding_for` now discharges and
  resolves the element to `object` (`crates/nx-types/src/infer.rs:2615-2627`). `type Box = { thing:
  object }` with `<Box thing={} />` is clean and evaluates to `<Box thing={} />`. The rule is stated
  in `braced-value-sequences` rather than left implicit, so the asymmetry the finding raised is now
  a claim the spec makes.

### ✅ Verified - RF7 Newly-parsing brace positions beyond markup child and function body are unspecified and unusable
- **Severity:** Medium
- **Evidence:** Admitting zero items on `values_braced_expression` reaches every rule that
  references it, not just the two design D4 enumerates. `value_if_simple_expression`'s `then`/`else`
  and `value_for_expression`'s `body` are `values_braced_expression`
  (`crates/nx-syntax/grammar.js:567-575`, `:628-636`), and `value_if_match_arm`/
  `value_if_condition_arm` bodies list it explicitly (`crates/nx-syntax/grammar.js:595-624`). All of
  those now parse `{}` where they previously did not, and none supplies an expected type, so the
  form is accepted by the parser and can never type check. Reproduced:
  - `let pick(c:boolean): string[] = {if { c => {} else => {"a" "b"} }}` → `Cannot determine the
    element type of this empty list…`, even though the enclosing function declares `string[]`. The
  empty arm also drags the join to `object[]`, the fallback D3 says must not happen.
  - `let xs:string[] = {for y in ys {}}` → parses (it was a parse error before this change) and
    reports two diagnostics.
  Coverage stops short of these: `test_parse_empty_for_body_is_still_rejected`
  (`crates/nx-syntax/tests/parser_tests.rs`) pins only the *element-position* `for`, and no test
  covers the value-position `for`, the value `if` branches, or arm bodies.
- **Recommendation:** Either enumerate these positions in `braced-value-sequences` and D4's
  "consequence to state" and add parse tests pinning what they do, or thread the expected type into
  arm and branch bodies so `{}` is usable where the enclosing site declares a list type. At minimum
  the change should not leave positions that parse a form that can never check.

- **Fix:** Both options, split by what each position can actually support — the answer to the
  question this review raised. **Arms and branches are now usable.** An empty list absorbs into the
  list it is joined with (`common_supertype`), and the value `if` and `infer_match_expr` resolve
  their branch and arm bodies against the join, which is the nearest thing to an expected type
  those positions have. The finding's own example now type checks *and evaluates*:
  `let pick(c:boolean): string[] = {if { c => {} else => {"a" "b"} }}` returns `{}` and
  `{"a" "b"}`. The `object[]` fallback D3 forbids is gone with it. **The `for` body is discharged
  at the `for`**, so the enclosing site judges `{}[]` whole: `let xs:string[][] = {for y in ys {}}`
  now checks clean — that position is usable, at a nested list site — and
  `let xs:string[] = {for y in ys {}}` reports exactly one diagnostic instead of two. An
  unannotated binding is still named, so the demand is not traded for a silent unresolved type.
  **Documented as well**, with parse fixtures for all three positions
  (`test_parse_empty_braced_value_in_value_position_control_flow`), the positions enumerated in the
  parse requirement, and the settling rule stated as design **D9** rather than as four separate
  fixes — the root cause this review named in its summary.
- **Status of the residue:** Two shapes still parse and cannot check, and probing shows both are
  pre-existing rather than introduced. `{if c {} else {"c"}}` joins a list with a scalar and lands
  on `object` — but so does `{if c {"z"} else {"a" "b"}}`, with no empty list anywhere: it is the
  arity rule crossing the join, which is the `common_supertype` item already in Open Questions. And
  a `for` body of `{}` at a *flat* list site is a nested list at a flat site, exactly as
  `{for y in ys {y y}}` is. Both now report once, naming what they found. Recorded in D9's
  "Consequence to state"; fixing them means fixing the join, which changes inferred types for
  programs that compile today and needs its own change.

- **Verification:** Confirmed for what the finding stated. All three positions are now documented
  (design D9, the parse requirement's enumeration, and a parse fixture for value-position control
  flow) and the two that can be made usable are: the finding's own example,
  `let pick(c:boolean): string[] = {if { c => {} else => {"a" "b"} }}`, type checks and evaluates —
  `pick(true)` returns the empty list and `pick(false)` returns the two-item one — and
  `let xs:string[][] = {for y in ys {}}` is clean. The residue the fix note records is accurately
  characterized: `{if c {"z"} else {"a" "b"}}` lands on `object` with no empty list anywhere, so
  that half is the pre-existing join problem.
  **Caveat:** the fix note's claim that "an unannotated binding is still named, so the demand is not
  traded for a silent unresolved type" holds for `let`, not for a function. See RF11, filed against
  the mechanism this fix introduced.

### ✅ Verified - RF8 An inference variable still leaks into a diagnostic when the empty list is nested one level deep
- **Severity:** Low
- **Evidence:** RF3's fix renders an unresolved empty list as `{}`, but `is_empty_list_type` matches
  only `Array(Variable)` at depth one (`crates/nx-types/src/infer.rs:3421-3424`). A list *of* empty
  lists is `Array(Array(Variable))` and falls through. Reproduced:
  `let xs:string[] = {for y in ys {}}` reports `Initializer for value 'xs' expects string[], found
  T1[][]` — the same `T0[]`-style leak RF3 was opened for, at a source the author wrote as `{}`.
  Only reachable because this change made a value-position `for` body of `{}` parse (see RF7).
- **Recommendation:** Render any type whose only unresolved part is an empty list's element variable
  without naming the variable (e.g. `{}[]`), or resolve RF7 so the shape is unreachable. Add the
  case to `a_mismatch_on_an_empty_list_does_not_name_the_element_variable`.

- **Fix:** Took the first option, and RF7's discharge removed the second diagnostic that
  accompanied it. `empty_list_display` replaces the depth-one special case and walks arrays and
  nullables to any depth, so the finding's source now reads `expects string[], found {}[]`. The
  unannotated-binding check was generalized the same way, so `let a = {for y in ys {}}` still names
  `a` rather than silently taking a type with a live variable in it. Asserted in
  `empty_list_as_a_for_body_at_a_flat_list_site_reports_only_the_mismatch` (exactly one message, no
  `T0`/`T1`) and `an_unannotated_binding_of_a_list_of_empty_lists_is_still_named`.

- **Verification:** Confirmed. `empty_list_display` walks arrays and nullables to any depth
  (`crates/nx-types/src/infer.rs:3503-3520`) and replaced the depth-one check at both the mismatch
  message and the unannotated-binding branch. `let xs:string[] = {for y in ys {}}` now reports one
  diagnostic reading `expects string[], found {}[]`, with no `T1`. Probed the other direction too:
  a function whose return type is an unresolved empty list reports `expects string, found {}` rather
  than naming a variable.

### ✅ Verified - RF9 `void` still appears as a primitive in the README and the VS Code syntax highlighting
- **Severity:** Low
- **Evidence:** Task 4.3 removed `void` from `PRIMITIVE_TYPE_COMPLETIONS` and tasks 5.1/5.2 updated
  the two grammar documents, but two other source-surface listings were missed:
  - `src/vscode/syntaxes/nx.tmLanguage.json:252` and `:2315` both still match
    `string|int|int32|int64|float32|float64|boolean|void|object` as primitive types, so `void` in
    type position is still coloured as a primitive in the editor — including a reference to a user
    type now legitimately named `void`.
  - `README.md:201` still advertises the primitive set as including `void`.
  (`nx-planning.md:47` and `nx-rust-plan.md:664` also list it; those read as historical planning
  documents, so they are noted rather than flagged.)
- **Recommendation:** Drop `void` from both `tmLanguage.json` alternations and from the README's
  primitive list, and add them to task 5's document sweep.

- **Fix:** Done as recommended. Both alternations in `src/vscode/syntaxes/nx.tmLanguage.json` and
  the `README.md` primitive list drop `void`; the README list was also missing `object`, so it now
  reads as the eight canonical names rather than seven. Covered by task 8.10, and the completion
  requirement in `primitive-type-names` was widened to every first-party listing of the names, with
  scenarios for highlighting and documentation — the requirement said "completions" and so did not
  reach the two listings that were missed. `nx-planning.md` and `nx-rust-plan.md` left alone as
  historical planning documents, per the finding.

- **Verification:** Confirmed. Both alternations in `src/vscode/syntaxes/nx.tmLanguage.json`
  (`:252`, `:2315`) and the `README.md:201` primitive list drop `void`; the README list also gained
  the missing `object`, so it reads as the eight canonical names. No `void` remains in any
  first-party listing outside the two historical planning documents the finding excluded.

### ✅ Verified - RF10 A top-level empty list still renders as the empty string
- **Severity:** Low
- **Evidence:** `format_value_inner`'s `Value::Array` arm joins elements with newlines and has no
  empty case (`crates/nx-cli/src/format.rs:45-52`), so `format_value(&Value::Array(vec![]))` returns
  `Ok("")`. `unbraced-literal-forms` requires that first-party output re-parse and type check, or
  else report a failure; an empty string does neither. Property position was given the `{}` spelling
  and this sibling path was not. It is also the one place the RF1 nested-list guard does not run, so
  a top-level list of lists is silently flattened.
- **Recommendation:** Either emit `{}` for an empty top-level list and reuse the nested-list guard on
  that path, or report a failure there, and say which in `unbraced-literal-forms`.

- **Fix:** Took the first option, both halves. `format_value_inner` emits `{}` for an empty list
  and applies the same direct-array guard property position got from RF1 — a run of values one per
  line cannot say where an inner list ends, so a list of lists would have come back as the
  flattened run of its elements. Two tests, and `unbraced-literal-forms` states the own-value path
  for both rules rather than leaving it read across from property position.

- **Verification:** Confirmed. `format_value_inner` emits `{}` for an empty top-level list and
  carries the same direct-array guard as property position (`crates/nx-cli/src/format.rs:46-63`).
  Observed the guard firing end to end: a value shaped `{}[]` reaches the CLI's own output path and
  is reported rather than flattened.

## Questions (2026-09-04 21:24 review)
- RF7: is threading the expected type into `if`/`for`/arm bodies in scope for this change, or should
  the delta state that those positions do not supply one? The answer decides whether RF6 and RF7 are
  one fix or two.
  - **Answered: one fix, and neither option as posed.** Threading a declared type down is not needed
    and would not have been enough — a branch body has no declared type to thread even when the
    function does. What those positions *do* have is each other: the type the alternatives join to.
    Making an empty list absorb into that join is a smaller change than threading and settles RF6
    and RF7 under one rule with RF5, stated as design D9. See RF7's fix note for what that leaves.
- The working tree also carries `examples/nx/types.nx` edits about integer literals at float sites
  and an untracked `openspec/changes/resolve-editor-positions/`, which appear to belong to other
  work. Confirm those are meant to ride along before archiving.
  - **Status:** Left for the user; this is a question about what to commit, not a defect. The
    working tree also carries `docs/scratch-highlighting.nx`, `examples/nx/template-candidate.nx`,
    and `drawnup-startup-fixes-review.md`, none of which this change touches.

## Summary (2026-09-04 21:24 review)
- The `void` half is clean apart from RF9's two missed listings; the call-argument half (section 7)
  holds up under probing — grammar scoping, `infer_call` plumbing, uncheckable-call discharge, and
  runtime arity all behave as the tasks claim.
- The empty-list half is sound at property, annotated-`let`, default, parameter, and function-body
  sites, and formatting round-trips. The gaps are concentrated where the brace reaches a position the
  design did not enumerate: element body content is broken at runtime (RF4) and spuriously diagnosed
  when it has siblings (RF5), non-list accepting sites are spuriously diagnosed (RF6), and control-
  flow bodies and match/condition arms parse a form that can never check (RF7).
- Root cause behind RF5/RF6: the pending-empty-list entry is discharged on only two of the paths
  that can conclude a binding, so any other accepting path leaves a diagnostic behind. Worth fixing
  as one change rather than site by site.

## Fix pass (RF4-RF10)

All seven findings are fixed and awaiting reviewer verification. RF5, RF6 and RF7 share one root
cause — the review's own summary named it — and are fixed as one rule rather than three: every path
that concludes a binding settles the empty list, so the end-of-analysis sweep reports only what
nothing concluded. That rule is design **D9**, and `braced-value-sequences` states each of the four
paths and what element type it supplies.

Two findings turned up artifact claims that were wrong rather than merely incomplete, and both are
corrected: the proposal and design said `T[][]` is "a type the system holds and source cannot
construct", but `let xs:string[][] = {for y in ys {y y}}` type checks and evaluates today, before
this change — a nested list has no *literal* spelling, which is a narrower claim and the one that
actually explains why formatting reports it.

Verification: `cargo test --workspace` passes with 1435 tests, 14 of them new (8 type, 3
interpreter, 2 formatter, 1 parser). `openspec validate empty-list-spelling --strict` passes. Every
`.nx` file in the repository — all 111 — produces byte-identical output with and without the
interpreter change in RF4, which is the one edit here that can reach source containing no `{}`.
`cargo fmt --check` reports the same pre-existing set as before this pass, and the clippy hits in
the three edited files are all in functions this pass did not touch. Tasks section 8 records the
work; nothing is committed.

## New Findings Discovered During 2026-09-05 00:59 Verification

Both come out of the RF4–RF10 fix pass itself; neither existed before it.

### ✅ Verified - RF11 Discharging an empty list against an element type that is still a variable loses the diagnostic entirely
- **Severity:** Medium
- **Evidence:** `resolve_empty_lists_in` removes the pending entry whenever it is called
  (`crates/nx-types/src/infer.rs:1094-1100`), and the `for` arm removes its body's entry
  unconditionally (`crates/nx-types/src/infer.rs:480-487`). Neither checks that the element type it
  is settling on is actually known. When every alternative in a join is an empty list, the join is
  itself `Array(Variable)` — the arm bodies are discharged against a live inference variable, and
  because the enclosing `if`/`match` expression is never itself entered into `pending_empty_lists`,
  nothing is left for the end-of-analysis sweep to report. The `let` path still catches it via
  `empty_list_display` (`crates/nx-types/src/infer.rs:3091`), but a function has no equivalent
  check, so it falls through silently. Reproduced against the built CLI:
  - `let f(c:boolean) = {if { c => {} else => {} }}` — **no diagnostic at all**. Before this fix
    pass the two pending arms were reported by the sweep. `f`'s return type is now `T[]` with a live
    variable, and the new `type_satisfies_expected` rule makes it satisfy *every* list type:
    `<Box items={f(true)} ns={f(false)} />` with `items:string[] ns:int[]` type checks clean and
    evaluates.
  - `let f(ys:string[]) = {for y in ys {}}` — same, at `{}[]`: clean at both `string[][]` and
    `int[][]` in one program, with nothing reported.
  This is what D3 says must not happen ("types the binding as a list of anything, lets it flow
  anywhere, and reports the author's actual mistake nowhere at all") and what the delta requires
  ("Where the site supplies no expected type, the system SHALL report a diagnostic identifying the
  binding whose element type could not be determined"). It also falsifies D9's closing claim that
  "an unannotated binding is still named so the demand is not traded for a silently unresolved
  type" — true for `let`, not for a function. No runtime unsoundness: the values really are empty
  lists, so they do inhabit both types. The defect is the missing diagnostic and the variable
  escaping into an inferred signature.
- **Recommendation:** Do not discharge against an unknown element. Where the settled element type
  still contains an inference variable, re-register the *enclosing* expression (the `if`/`match`
  result, the `for` result) in `pending_empty_lists` under its own span instead of dropping the
  inner entries — that keeps the demand alive for the sweep while still reporting once, which is
  what RF8 asked for. Add tests for an unannotated function whose arms are all `{}` and whose `for`
  body is `{}`, asserting exactly one diagnostic.

- **Fix:** Done as recommended. `resolve_empty_lists_in`'s unconditional discharge at the arm join
  and the `for` body is replaced by `settle_empty_lists`, which settles the parts against the
  enclosing type only when that type says what the elements are, and otherwise re-registers the
  *enclosing* expression — the `if`, the `match`, the `for` — in `pending_empty_lists` under its own
  span. The demand survives, and stays one diagnostic rather than one per part, which is what RF8
  asked for. Both reproductions now report exactly once, pointing at the whole expression:
  `let f(c:boolean) = {if { c => {} else => {} }}` and `let f(ys:string[]) = {for y in ys {}}`.
  Moving the demand outward meant the two discharge conditions in `check_typed_binding_for` had to
  stop looking only at depth one — a `for` now carries the demand for a type shaped `{}[]` — so both
  use `empty_list_display`, and the `object[]` resolution stays confined to the depth-one case it
  was written for. Section 8's cases are all unchanged: the arm join still resolves, the `for` body
  at a nested list site is still clean, the flat site still reports once naming `{}[]`, and the
  unannotated `let` is still named. Four tests, including
  `an_unresolved_empty_list_does_not_escape_into_an_inferred_signature`, which pins the finding's
  sharpest consequence — one value inhabiting `string[]` and `int[]` in the same program — and
  `all_empty_alternatives_still_take_the_element_type_from_the_binding`, which pins that moving the
  demand costs nothing where the site does supply a type.

- **Verification:** Confirmed. `settle_empty_lists` (`crates/nx-types/src/infer.rs:1114-1139`)
  settles the parts only when the enclosing type is a list, and re-registers the enclosing
  expression when `empty_list_display` shows that type still holds an unresolved element; the value
  `if`, `infer_match_expr`, and the `for` all route through it
  (`:327`, `:761`, `:484`). Both reproductions now report exactly once, spanning the whole
  expression: `let f(c:boolean) = {if { c => {} else => {} }}` and
  `let f(ys:string[]) = {for y in ys {}}`. Re-probed everything the change of mechanism could have
  disturbed, and all of it still holds — RF4a/b, RF5, RF6, RF7 (arm usable in both directions, `for`
  clean at `string[][]`), RF8 (`found {}[]`, one diagnostic), the unannotated `let`, and the
  all-empty arms under a *declared* return type, which stays clean. Also probed a chain the fix
  makes possible, `let xs:string[][][] = {for y in ys {for z in ys {}}}`: the demand moves outward
  through both `for`s and the site accepts it. The two discharge sites in `check_typed_binding_for`
  were correctly widened to `empty_list_display` while the `object[]` resolution stayed at depth one
  (`:2650-2667`). One wording observation, not a defect: the sweep message says "this empty list"
  while pointing at a `for` or `if`, which is still accurate about what the expression evaluates to
  and still names an actionable fix.

### ✅ Verified - RF12 The RF4 interpreter fix changes what any zero-value element body means, not just `{}`
- **Severity:** Medium
- **Evidence:** `normalize_content_values` now keys on whether the body had *expressions*, not on
  whether it contained `{}` (`crates/nx-interpreter/src/interpreter.rs:2686-2696`), so every body
  that evaluates to no values binds the empty list where it previously left the content property to
  its declared default. `eval_content_expressions` splices arrays
  (`crates/nx-interpreter/src/interpreter.rs:2668-2676`), so a `for` that iterates zero times is
  exactly such a body. Reproduced with source containing no `{}` at all:
  ```
  type A = { n: int = 1 }
  type Box = { content items: object[] = {<A n=9 />} }
  let xs:string[] = {}
  <Box>for x in xs { <A n=2 /> }</Box>
  ```
  now renders `<Box items={} />`; before the fix it rendered the declared default `{<A n=9 />}`.
  The new behaviour is defensible — a body that ran and produced nothing is not the same as no body
  — but three things do not line up with it:
  - The proposal's Migration Plan still says the empty-list half is "purely additive... every
    program that compiles today still compiles and still means the same thing". That is now false.
  - `braced-value-sequences` states the runtime rule only for "an empty `ValuesBracedExpression` as
    element body content", which is narrower than what was implemented.
  - No test pins the `for`-over-empty case; the three added interpreter tests cover `{}`, `{}` at a
    nullable field, and an absent body.
  Separately, the function's own doc comment claims "an element-position `if` that takes no branch
  binds no children rather than leaving the property unbound". That is not what happens — probing
  `<Box>if false { <A n=2 /> }</Box>` binds `null`, because the untaken `if` evaluates to
  `Value::Null` rather than to an empty array, so it never reaches the zero-value case at all.
  (Null landing on a non-nullable `object[]` field without a diagnostic is pre-existing and outside
  this change.)
- **Recommendation:** Keep the behaviour and state it: widen the `braced-value-sequences` runtime
  requirement from `{}` to any body content that produces no values, correct the Migration Plan to
  name this as the one meaning change, add an interpreter test for a `for` over an empty list at a
  content field with a non-empty default, and fix the doc comment's `if` example.

- **Fix:** Kept the behaviour and stated it, as recommended, in all four places. The rule is worth
  keeping on its own terms: keying on `{}` instead would make two bodies that evaluate identically
  mean different things. `braced-value-sequences` now draws the distinction on whether a body was
  written rather than on how it was spelled, and says so at a content property with a non-empty
  default, with a scenario for the `for`-over-empty case. The Migration Plan no longer claims every
  program means the same thing; it names this shape as the one that changes, says why the
  alternative is worse, and records that no source among the repository's 111 `.nx` files hits it.
  `test_a_body_that_produced_nothing_binds_the_empty_list_over_a_default` pins it with source
  containing no `{}`. And the doc comment's `if` example is corrected: an untaken `if` evaluates to
  null and never reaches the zero-value case, so it named the one construct that is not an example.
  The pre-existing null-at-a-non-nullable-field gap it exposes is left alone, as the finding
  scopes it.

- **Verification:** Confirmed, in all four places the recommendation named. The behaviour is kept and
  the artifacts now describe it: `braced-value-sequences` draws the distinction on whether a body
  was written rather than on how it was spelled and states it holds at a non-empty default, with a
  scenario for the `for`-over-empty case that explicitly requires source containing no `{}`; the
  Migration Plan drops the "means the same thing" claim, names this shape as the one that changes,
  and says why keying on `{}` would be worse. The doc comment is corrected and now matches probed
  behaviour on both counts — a `for` that iterates zero times binds no children, an untaken `if`
  evaluates to null and never reaches the zero-value case. Re-probed all three states: `for` over an
  empty list at a content field with default `{<A n=9 />}` binds `{}`, `<Box />` still takes the
  default, and `<Box>if false { ... }</Box>` still binds null.
  `test_a_body_that_produced_nothing_binds_the_empty_list_over_a_default` pins the first with no
  `{}` in its source.

## Summary (2026-09-05 00:59 verification)
- **Verified fixed (7):** RF4, RF5, RF6, RF7, RF8, RF9, RF10. Each was re-probed against a freshly
  built `nxlang` rather than read; `cargo test --workspace` passes at 1435 tests.
- **Reopened (0).**
- **New findings (2):** RF11 and RF12, both introduced by the fix pass. RF11 is the one to fix
  before archiving — it removes a diagnostic the change's own D3 makes a headline decision. RF12 is
  mostly an artifact-and-test gap around a behaviour change that is probably correct.

## Fix pass (RF11-RF12)

Both fixed. RF11 was a real hole in D9 rather than a slip in applying it: "settle the empty list"
was implemented as "stop tracking it", which is the same thing only when the settling type is
known. The rule now distinguishes the two — met where the enclosing type supplies an element type,
moved outward where it does not — and D9's promise that an unannotated binding is still named holds
for a function as well as for a `let`.

RF12 needed no code change and four artifact changes; the behaviour it questions is kept, and the
Migration Plan now names the one program shape in this change that means something different than
it did.

Verification: `cargo test --workspace` passes with 1440 tests, 5 new (4 type, 1 interpreter).
`openspec validate empty-list-spelling --strict` passes. All 111 `.nx` files in the repository
produce output byte-identical to the previous pass. `cargo fmt --check` reports the same
pre-existing set, and the clippy hits in the edited files remain in functions this work does not
touch. Tasks section 9 records it; nothing is committed.

## Summary (2026-09-05 verification of RF11-RF12)
- **Verified fixed (2):** RF11, RF12. Both re-probed against a freshly built `nxlang`, along with a
  full regression sweep of RF4–RF10, since RF11's fix replaced the mechanism those verifications
  rested on.
- **Reopened (0).**
- **New findings (0).**
- `cargo test --workspace` passes at 1440 tests (5 more than the previous pass, matching the fix
  note's claim), and `openspec validate empty-list-spelling --strict` passes.
- All twelve findings are now verified. The remaining pre-archive item is not a defect: the working
  tree still carries `examples/nx/types.nx` edits, `openspec/changes/resolve-editor-positions/`,
  `docs/scratch-highlighting.nx`, `examples/nx/template-candidate.nx`, and
  `drawnup-startup-fixes-review.md`, none of which this change touches — decide what to commit.

## Design change: the empty list is given a real type (D10)

Raised in conversation rather than by review, but it settles findings this report spent most of its
length on, so it is recorded here.

Findings RF3, RF5, RF6, RF7, RF8 and RF11 — six of the twelve — were all the same defect at six
sites: the empty list was typed as a list whose element was an inference *variable*, which satisfies
nothing and joins to `object`, so every site that could conclude a binding had to resolve it by hand
before analysis ended. A site that concluded without doing so left either a spurious diagnostic or a
live variable. D9 stated that as a rule; RF11 then found a seventh site the rule itself had a hole
at.

The variable was standing in for the bottom type. Two rules this change had already written —
"an empty list satisfies any list type" (`infer.rs:3499`) and "an empty list joined with a list
yields that list" (`infer.rs:3586-3587`) — were bottom-type subtyping and bottom-as-join-identity,
special-cased to one type constructor and detected by a structural test on `Array(Variable)`.

`Primitive::Never` replaces the stand-in. `{}` is a `never[]`. The two mechanisms needed already
existed — arrays are covariant, and any array already satisfies `object` — so the two arms reach
every site an empty list can be written at, with no site-by-site plumbing.

**What it removed:** `pending_empty_lists`, the end-of-analysis sweep and its caller,
`discharge_pending_empty_lists`, `resolve_empty_lists_in`, `settle_empty_lists`, the empty-list
branches in `check_typed_binding_for`, the filter in `normalized_sequence_type`, and the resolution
in `check_content_binding`. 45 references in `infer.rs` down to 12, of which nine are rendering and
three are the one diagnostic that stays. D9 is deleted; D3 is rewritten; D10 is added, within the
bounds D6 set for a bottom type — inference-internal, no keyword, no rename of `Void`.

**What it kept:** D3's diagnostic, now as a legibility rule rather than a typing failure. `let a =
{}` and `let f(x) = {}` are still reported and now name the binding in both cases, where the
function form previously got the generic sweep message.

**One gap the tests caught:** NX carries two compatibility relations — the structural
`Type::is_compatible_with` and the richer `InferenceContext::type_satisfies_expected`, which does
not delegate to it. Both needed the bottom case; only the second had it until a direct test on the
first failed. `common_supertype` in `semantics.rs` inherits it through the former.

**Verification:** every test written against the variable-based machinery passes untouched — RF3's,
RF5's, RF6's, RF7's, RF8's and RF11's included. `cargo test --workspace` passes at 1446 tests, 6 new
(`never_is_not_a_primitive.rs`). All 111 `.nx` files produce byte-identical output. `cargo fmt
--check` reports the same pre-existing set; the clippy hits in the edited files are in functions this
work does not touch. `openspec validate --strict` passes. Tasks section 10 records it, and section 9
is marked superseded rather than deleted, since what it tried is the argument for what replaced it.

Findings RF1-RF12 are unaffected in substance; RF6's resolution changes only in that the element
type is now `never` rather than `object`, which is the precise answer rather than a stand-in whose
safety depended on the site pinning it.
