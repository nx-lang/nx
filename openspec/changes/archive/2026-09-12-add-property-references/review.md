# Review: add-property-references

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, and the seven spec deltas
(`property-references`, `update-records`, `component-syntax`, `nx-ir-format`,
`typescript-ir-runtime`, `executable-code-generation`, `cli-code-generation`).

**Reviewed code:** the whole working tree diff (37 files, ~3.6k lines) —
`crates/nx-hir/{lib,lower,unions,records,scope,prepared}.rs`,
`crates/nx-types/{infer,check}.rs`, `crates/nx-interpreter/{interpreter.rs,eval/logical.rs}`,
`crates/nx-api/{artifacts,component,value}.rs`,
`crates/nx-codegen/{builder,ir,model,emit,runtime,tests}.rs`,
`crates/nx-cli/src/typegen{.rs,/model.rs}`, `crates/nx-language-service/src/lib.rs`,
`runtime/typescript/src/index.ts`, the .NET generated fixtures, docs, `nx-grammar.md`, and
`examples/nx/component.nx`.

**Verification performed:** `cargo test --workspace` (1749 passed, 0 failed), `pnpm -r test` (pass),
`dotnet test bindings/dotnet` (113 passed), `openspec validate add-property-references --strict`
(valid). Findings below were additionally reproduced by hand through `nxlang run` / `nxlang codegen`
and by executing generated JavaScript with `node`.

## Findings

### ✅ Verified - RF1 Generated JavaScript/TypeScript that calls an update intrinsic inside an element property never imports the runtime helper, so the module throws `ReferenceError` when it runs
- **Severity:** High
- **Evidence:** [emit.rs:3977-3979](crates/nx-codegen/src/emit.rs#L3977-L3979) —
  `CodegenExpressionKind::Element(_) => { output.insert("nxElement"); }` inserts the element helper
  and does **not** recurse into the element's properties or content, so an `IntrinsicCall` nested in
  a property is never seen by `collect_expression_runtime_helpers`. The new arm at
  [emit.rs:3996-4001](crates/nx-codegen/src/emit.rs#L3996-L4001) is therefore unreachable for the
  most common spelling. Reproduced with
  `type User = { name:string email:string? age:int? }` /
  `let root() = <Box keys={changed(<User.Update age={null} name="Ada" />)} />`: the emitted
  `m0_probe2.js` contains `import { nxElement } from "./nx-runtime.js";` only, and
  `node -e "import('./m0_probe2.js').then(m => m.root())"` fails with
  `nxChangedFields is not defined`. The same source with the call as the direct function body
  (`let root() = {changed(...)}`) emits `import { nxChangedFields } ...` and runs. This breaks the
  `executable-code-generation` requirement that a call be "emitted as a call into the separately
  supplied NX runtime helpers".
- **Recommendation:** Make the `Element(_)` arm of `collect_expression_runtime_helpers` recurse into
  `element.properties[*].value` and `element.content` after inserting `"nxElement"` (the sibling
  collectors `collect_expression_value_references` and `collect_expression_runtime_helpers`'s
  `ComponentDescriptor` arm already do this). Add a codegen test whose intrinsic call sits inside an
  element property and executes under node.
- **Fix:** The `Element` arm of `collect_expression_runtime_helpers` now recurses into the element's properties and content, as the value-reference collector already did. New codegen test `generated_javascript_calls_an_intrinsic_inside_an_element_property` asserts the `nxChangedFields` import and executes the module under `node`.
- **Verification:** Verified. `collect_expression_runtime_helpers`'s `Element` arm now recurses into properties and content ([emit.rs](crates/nx-codegen/src/emit.rs)). Re-ran the original repro: `m0_probe2.js` emits `import { nxChangedFields, nxElement } from "./nx-runtime.js";` and `node` returns `{"$type":"Box","keys":["name","age"]}`. `generated_javascript_calls_an_intrinsic_inside_an_element_property` passes and compares against the interpreter.

### ✅ Verified - RF2 A property union reached through a workspace import loses its inherited cases, rejecting valid source
- **Severity:** High
- **Evidence:** `complete_property_unions` runs only in
  [check.rs:142](crates/nx-types/src/check.rs#L142), on the module being analyzed. A workspace
  import binds the *target* module's item directly from its lowered module —
  [artifacts.rs:1316-1336](crates/nx-api/src/artifacts.rs#L1316-L1336) →
  `add_workspace_item_bindings` — so the importing module sees the union as lowering left it, with
  only the fields the target itself declares. Reproduced with `data.nx`
  (`export abstract type Named = { name:string }` / `export type User extends Named = { email:string? }`)
  and `main.nx` (`import { User } from "./data.nx"` /
  `let root() = <Box a={User.Property.name} b={User.Property.email} />`):
  `error: Union 'User.Property' has no case named 'name' Cases: email`. The library-package import
  path is covered by a test and works; the workspace path is not covered and does not. This breaks
  the `property-references` requirement that `T.Property` have one case per effective field
  "including inherited fields" and be "visible and importable wherever `T` is".
- **Recommendation:** Complete the target module's property unions before its items are bound into a
  dependent module (run `complete_property_unions` as part of preparing every workspace module, not
  only the one being analyzed), or resolve a property union's cases from the target's effective
  shape at use time rather than from the stored case list. Add a workspace-import scenario to
  `crates/nx-types/tests/property_references.rs` alongside the existing library-import one.
- **Fix:** `complete_graph_property_unions` in `nx-api/src/artifacts.rs` completes every prepared module's property unions once all modules are prepared and peer namespaces are shared (both the workspace-graph and the multi-file library paths), then replaces every module's peers with the completed modules, so an importer reads the full case list. Fixing the codegen half exposed the shared root cause: the resolved program's selective workspace and library imports never carried `X.Update` / `X.Property` alongside `X` (the prepared bindings did), so cross-module `<User.Update />` emitted as `nxElement("User.Update", ...)` and `User.Property.name` as `m0_User["Property"]["name"]`, a `ReferenceError` at run time. Both import paths now bring the derived declarations along. Tests: `property_union_of_a_workspace_import_includes_inherited_fields` in `nx-api` (which owns workspace building, so it lives there rather than in `nx-types`) and `generated_javascript_reaches_property_unions_and_intrinsics_across_workspace_modules` in `nx-codegen`.
- **Verification:** Verified. `share_graph_namespaces` + `complete_graph_property_unions` complete every prepared module and republish the completed clones as peers before any importer reads them; `DERIVED_DECLARATION_SUFFIXES` now carries the derived declarations along all three import paths (library interface, workspace item, resolved-program selective). Re-ran the original repro: the workspace program builds clean and `node` returns `{"$type":"Box","a":"name","b":"email"}` — the inherited `name` case included. Checked the ordering concern in the new pass: completion only rewrites union cases and never adds or removes items, so the namespace indices computed beforehand stay valid and a base in a peer module still resolves. `property_union_of_a_workspace_import_includes_inherited_fields` and `generated_javascript_reaches_property_unions_and_intrinsics_across_workspace_modules` pass.

### ✅ Verified - RF3 `changed` falls back to insertion order in generated code and in the exported host helper, diverging from the interpreter's declaration order
- **Severity:** Medium
- **Evidence:** [emit.rs:2076-2098](crates/nx-codegen/src/emit.rs#L2076-L2098) passes a field-order
  array only when `update_field_order` finds the update record's declaration in the emitted program;
  otherwise `nxChangedFields` sorts by `order.indexOf(key)` against an empty array
  ([runtime.rs:434-441](crates/nx-codegen/src/runtime.rs#L434-L441)), which is a stable no-op and
  leaves the value's own key order. Reproduced across a workspace import: the emitted call is
  `nxChangedFields(nxElement("User.Update", { age: null, name: "Ada" }, []))` with no order array,
  while the interpreter on equivalent single-module source yields `name age`. The TypeScript runtime
  has the same hole — `changedFields` at
  [index.ts:997-1006](runtime/typescript/src/index.ts#L997-L1006) uses declaration order only when
  `program.nominalShapesByDiscriminator` has exactly one shape for the `$type`, and the exported
  host helper takes `program` as optional, so a host calling `changedFields(update)` gets insertion
  order. The `update-records` spec requires the result be "ordered as the cases of `T.Property` are
  declared rather than as the update was written".
- **Recommendation:** Carry the field order in the IR `intrinsicCall` node (or reference the
  property union declaration) so neither runtime has to rediscover it, and make the exported
  `changedFields` helper require the order (or the program) rather than silently degrading. Add a
  cross-module codegen test that compares against the interpreter.
- **Fix:** The builder computes the target's declared field order for `changed` from the argument's static type (`effective_record_shape_at` on the `Named` origin) and stores it on the codegen `IntrinsicCall`; a `changed` whose order cannot be determined is a build diagnostic rather than a silent omission. Generated JavaScript always passes the array (`nxChangedFields` now requires it), NX IR carries it as `fieldOrder` (nx-ir-format delta updated), and the TypeScript runtime uses it. The exported `changedFields(update, program)` helper requires the prepared program and fails with a diagnostic when it does not declare the update record. `update_field_order` in `emit.rs` is deleted. The cross-module codegen test compares against the interpreter.
- **Verification:** Verified for the substance of the finding. The order now travels with the call: `update_field_order` moved into `builder.rs` and resolves through `effective_record_shape_at`, an undeterminable order is a build diagnostic rather than a silent omission, `emit.rs`'s copy is deleted, IR carries `field_order`, and the TypeScript runtime prefers `op.fieldOrder`. `changedFields(update, program)` now requires the program and fails with `Cannot order the fields of '<type>'` when it does not declare the update record. Re-ran the cross-module repro: the emitted call carries `["name", "email", "age"]` and `node` returns `["name","age"]`, matching the interpreter. One residual, tracked as RF14: the emitted *JavaScript* runtime's `nxChangedFields(update, order = [])` kept its insertion-order default, where the TypeScript copy requires the argument.

### ✅ Verified - RF4 Ordering comparisons on property-union cases type-check and then fail at runtime
- **Severity:** Medium
- **Evidence:** [infer.rs:1148-1160](crates/nx-types/src/infer.rs#L1148-L1160) computes
  `sibling_cases` inside the combined `Eq | Ne | Lt | Le | Gt | Ge` arm, so it admits `<`, `<=`,
  `>`, and `>=` between two cases of one union as well as `==` and `!=`. The comment above it
  ("Two cases of one union are comparable with each other") only justifies equality, and
  `eval_lt`/`eval_le`/`eval_gt`/`eval_ge`
  ([logical.rs:145-175](crates/nx-interpreter/src/eval/logical.rs#L145-L175)) have no `UnionCase`
  arm. Reproduced: `<Box lt={User.Property.name < User.Property.email} />` passes analysis and dies
  with `Runtime error: Type mismatch in less than: expected comparable types within same category,
  got union_case and union_case`.
- **Recommendation:** Split the arm so `sibling_cases` applies to `Eq | Ne` only, leaving relational
  operators to report the existing type error. Add a negative test.
- **Fix:** `sibling_cases` applies to `Eq | Ne` only; `<`, `<=`, `>`, `>=` between two cases report the existing `Cannot compare types` error. Test: `property_union_values_cannot_be_ordered`.
- **Verification:** Verified. `sibling_cases` is gated on `matches!(op, Eq | Ne)` ([infer.rs](crates/nx-types/src/infer.rs)). The original repro now reports `Cannot compare types User.Property.name and User.Property.email` at analysis time instead of reaching the interpreter, and `==` between two cases still checks. `property_union_values_cannot_be_ordered` passes.

### ✅ Verified - RF5 The change alters `==` on records and arrays and the IR encoding of every constant union case, for programs that never touch property references, with no spec delta and against design.md's stated goal
- **Severity:** Medium
- **Evidence:** Two behavior changes ride along outside any requirement in the deltas:
  (a) `eval_eq` becomes `values_equal` with structural record and array comparison
  ([logical.rs:82-135](crates/nx-interpreter/src/eval/logical.rs#L82-L135)); previously `a == b` on
  two records or two lists was always `false`. The `update-records` delta mentions structural
  equality only as `diff`'s comparison rule, not as a change to `==`.
  (b) `is_constant` is added to the IR `unionCase` **expression** op
  ([ir.rs:507-513](crates/nx-codegen/src/ir.rs#L507-L513)). At `HEAD` that field existed only on
  union *declarations*, while `runtime/typescript`'s `evalUnionCase` already read `op.isConstant`
  — so before this change an authored constant case evaluated through IR produced
  `{ $type: "Mode.dark" }` instead of `"dark"`. Emitting `nx-ir` for
  `type Mode = light | dark` / `let root() = <Box m={Mode.dark} />` now yields three `isConstant`
  keys where it previously yielded two, so design.md's goal "Byte-identical IR, generated code, and
  typegen output for programs that never touch the feature" no longer holds.
  Both changes look correct and (b) is a necessary prerequisite; the issue is that they are
  undeclared.
- **Recommendation:** Either add the two behaviors to the spec deltas (a `MODIFIED` requirement for
  `==` in whichever capability owns equality, and one in `nx-ir-format` for the constant-case
  encoding fix) or split them into their own change. Correct the "byte-identical IR" claim in
  design.md either way, and note the constant-case fix in the proposal's **BREAKING** list, since it
  changes the value a pre-existing program produces through the TypeScript IR runtime.
- **Fix:** (a) New `value-equality` capability delta specifying `==` / `!=` as structural over records and lists, with an interpreter test `equality_compares_records_and_lists_structurally`; D4 in design.md notes the change. (b) The nx-ir-format `MODIFIED` requirement now states that the union-case construct marks a constant case in expression position and that a `changed` call carries the field order, with scenarios for both; D5 explains why. The "byte-identical IR" goal is corrected to name the one difference, and the proposal lists both behaviors as **BREAKING** and adds `value-equality` to the capabilities.
- **Verification:** Verified. (a) A new `value-equality` capability delta specifies `==`/`!=` as structural over records and lists with two scenarios, and `equality_compares_records_and_lists_structurally` passes. (b) The `nx-ir-format` `MODIFIED` requirement now states that a constant case is marked constant in expression position and that a `changed` call carries the field order, with a scenario for each. design.md's goal is restated as "Unchanged generated code and typegen output ... IR for such a program differs in exactly one way", and the proposal lists both behaviors as **BREAKING** and adds `value-equality` to its capability list. `openspec validate add-property-references --strict` reports valid.

### ✅ Resolved - RF6 Hover for a reference to a top-level value regressed from the declaration form to the bare type, and the two affected tests were rewritten to match
- **Severity:** Medium
- **Evidence:** [infer.rs:3574-3587](crates/nx-types/src/infer.rs#L3574-L3587) now sorts value
  bindings so locals are inferred in declaration order. The side effect is visible in
  [nx-language-service/src/lib.rs:3671-3680](crates/nx-language-service/src/lib.rs#L3671-L3680),
  where `let ab = 1` / `{ab 2}` changed from `let ab: int` to `int`, and at
  [lib.rs:4003-4009](crates/nx-language-service/src/lib.rs#L4003-L4009), where
  `let chosen: Role.admin` became `Role.admin` — the assertions were edited rather than the behavior
  preserved. The previous form was deliberate ("design D5" per the comment that was deleted).
  Nothing in this change's requirements needs it, and no spec delta covers hover.
- **Recommendation:** Confirm whether the reordering is actually required for property unions. If it
  is, restore the declaration-form hover explicitly (the hover provider can still prefer the
  declaration when the name resolves to a top-level `let`) rather than absorbing the loss into the
  tests; if it is not, drop the sort.
- **Status:** The sort is required: without it `apply_yields_the_record_type` and `top_level_values_are_inferred_in_declaration_order` fail, because the intrinsic rule needs the type of a value argument, which was previously known only when hash order happened to infer that value first. The hover provider is unchanged; its documented rule ("a name is an expression first ... the declaration is the answer only where the name reaches no expression") now applies deterministically. The previous `let ab: int` answer was produced only because the reference had no recorded type when inference happened to check `value` before `ab`, so the rewritten assertions describe the provider's intended behavior rather than a loss. Preferring the declaration for a top-level value reference would contradict the provider's shadowing rationale and is a language-service design change outside this change's scope; reopen if the declaration form is wanted as a deliberate hover change.
- **Verification:** Accepted as resolved. The claim checks out: `apply_yields_the_record_type` infers `let v = {apply(u, <User.Update ... />)}` from a preceding `let u = <User ... />`, so the intrinsic rule genuinely needs `u`'s type to be recorded before `v` is checked, and `top_level_values_are_inferred_in_declaration_order` pins the chain `a → b → c → d → e`. Both pass. The hover provider itself is untouched, so the rewritten assertions describe its existing documented rule under a now-deterministic order rather than a new behavior. Left resolved; reopen only if the declaration-form hover is wanted as a deliberate language-service change.

### ✅ Verified - RF7 A bare `Property` outside a component reports two diagnostics, one of them the generic "Undefined identifier"
- **Severity:** Medium
- **Evidence:** `check_undefined_identifiers` was taught to skip intrinsic callees
  ([scope.rs:448-457](crates/nx-hir/src/scope.rs#L448-L457)) but not bare `Property`. For
  `let k = {Property.name}` outside a component the output is
  `error ... Undefined identifier 'Property'` followed by the intended
  `bare-property-outside-component` message. Design D2 describes one diagnostic that "names the
  qualified spelling"; the extra generic error is the noise the reporting in
  `report_unresolved_property_reference` was written to avoid ("Reporting here ... is what keeps the
  access from cascading").
- **Recommendation:** Suppress the undefined-identifier diagnostic for an identifier spelled
  `Property` at the head of a member access or in a type reference, the same way the intrinsic
  callee is skipped, so only the actionable message is reported.
- **Fix:** The `Member` arm of `check_undefined_identifiers` skips the base check when the flattened access is headed by `Property`, so only `bare-property-outside-component` is reported. Test: `bare_property_outside_a_component_reports_exactly_one_diagnostic`. A bare `Property` in a type annotation reports nothing at all, but so does any unknown type name in a parameter annotation (`let t(p:Nope) = {p}`), which is a pre-existing gap outside this change.
- **Verification:** Verified. The original repro now reports exactly two diagnostics for the whole file — the `apply` reservation and `bare-property-outside-component` — with no `Undefined identifier 'Property'`. `bare_property_outside_a_component_reports_exactly_one_diagnostic` passes. The noted type-annotation gap is pre-existing and correctly left alone.

### ✅ Verified - RF8 `diff` in both JavaScript runtimes ignores fields absent from `after`, where the interpreter walks the target's effective shape
- **Severity:** Low
- **Evidence:** `nxDiffRecords` ([runtime.rs:417-431](crates/nx-codegen/src/runtime.rs#L417-L431))
  and `diffRecordValues` ([index.ts:1024-1038](runtime/typescript/src/index.ts#L1024-L1038)) both
  iterate `Object.keys(after)` only, so a key present in `before` and missing from `after` is never
  compared. `Interpreter::diff_records`
  ([interpreter.rs:2686-2708](crates/nx-interpreter/src/interpreter.rs#L2686-L2708)) iterates the
  shape and treats a missing field as `Value::Null`. Generated record literals always emit every
  field (verified: `<User name="Ada" />` for a nullable `email` emits `email: null`), so the gap is
  not reachable from generated source — but it is reachable through the exported `diffRecords` host
  helper, whose doc comment does not require complete records.
- **Recommendation:** Iterate the union of `before` and `after` keys in both runtimes, or document
  on the exported helper that both arguments must be complete normalized records.
- **Fix:** Both `nxDiffRecords` and `diffRecordValues` iterate the union of `before` and `after` keys, so a field only one record carries is compared against `null`.
- **Verification:** Reopened — the fix landed in only one of the two emitted runtime copies. `crates/nx-codegen/src/runtime.rs` emits two hand-maintained runtime modules, one per target. The TypeScript copy iterates the union of keys ([runtime.rs:428](crates/nx-codegen/src/runtime.rs#L428)), and so does `runtime/typescript`'s `diffRecordValues`, but the **JavaScript** copy still reads `for (const key of Object.keys(after))` ([runtime.rs:789](crates/nx-codegen/src/runtime.rs#L789)). Demonstrated against the emitted `nx-runtime.js`: `nxDiffRecords({$type:"User",name:"Ada",email:"x@y"}, {$type:"User",name:"Ada"})` returns `{"$type":"User.Update"}`, dropping the `email` change that the interpreter and the TypeScript runtime both report as `email: null`. The same program compiled to `--target javascript` and `--target typescript` therefore answers differently. Apply the identical `new Set([...Object.keys(before), ...Object.keys(after)])` change to the JavaScript copy.
- **Fix (second pass):** The JavaScript copy of `nxDiffRecords` now iterates the same key union as the TypeScript copy. The cross-target test added for RF14 runs `nxDiffRecords(user, partial)` and `nxDiffRecords(partial, user)` against both emitted runtimes and asserts `{ "$type": "User.Update", "email": null }` and `{ "$type": "User.Update", "email": "x@y" }`, so this exact repro is now executed on every run.
- **Verification (second pass):** Verified. Both emitted copies of `nxDiffRecords` now iterate `new Set([...Object.keys(before), ...Object.keys(after)])` — compared the two raw-string runtime modules in `runtime.rs` side by side and they agree. Ran the original repro against the freshly emitted `nx-runtime.js`: `nxDiffRecords(user, partial)` returns `{"$type":"User.Update","email":null}` and the reverse returns `{"$type":"User.Update","email":"x@y"}`, both matching the interpreter on equivalent source. `emitted_runtime_intrinsic_helpers_agree_across_targets` executes this repro in both directions on every run.

### ✅ Verified - RF9 A doc comment was displaced, leaving `collect_type_ref_names` undocumented and `item_type_refs` with two summary lines
- **Severity:** Low
- **Evidence:** [typegen/model.rs:1639-1641](crates/nx-cli/src/typegen/model.rs#L1639-L1641) —
  `item_type_refs` was inserted between `/// Collects every type name `ty` mentions.` and the
  `collect_type_ref_names` it described, so the new function now carries that line plus its own
  `/// Every type annotation one declaration writes.`.
- **Recommendation:** Move `/// Collects every type name `ty` mentions.` back onto
  `collect_type_ref_names`.
- **Fix:** The doc line is back on `collect_type_ref_names`'s replacement; see RF10.
- **Verification:** Verified. `nx_hir::item_type_refs` carries "Every type annotation one declaration writes." and `nx_hir::type_ref_names` carries "Every type name `ty` mentions, in the order written." ([lib.rs:457-487](crates/nx-hir/src/lib.rs#L457-L487)); neither function borrows the other's summary.

### ✅ Verified - RF10 `item_type_refs` and `collect_type_ref_names` are now duplicated across crates
- **Severity:** Low
- **Evidence:** `item_type_refs` exists identically at
  [builder.rs:219](crates/nx-codegen/src/builder.rs#L219) and
  [typegen/model.rs:1641](crates/nx-cli/src/typegen/model.rs#L1641); `collect_type_ref_names` now
  has three copies ([builder.rs:244](crates/nx-codegen/src/builder.rs#L244),
  [typegen/model.rs:1666](crates/nx-cli/src/typegen/model.rs#L1666),
  [typegen/languages/typescript.rs:664](crates/nx-cli/src/typegen/languages/typescript.rs#L664)).
  The two `item_type_refs` bodies must stay in step: each is the definition of "which annotations
  count as a reference to a derived declaration", and a new `Item` variant or annotation site
  silently missed in one of them drops a needed declaration from IR or from typegen.
- **Recommendation:** Move `item_type_refs` (and a shared `collect_type_ref_names` over
  `ast::TypeRef`) into `nx-hir` beside the other item helpers and call it from both crates.
- **Fix:** `nx_hir::item_type_refs` and `nx_hir::type_ref_names` (in `nx-hir/src/lib.rs` beside `is_update_intrinsic`) replace the two `item_type_refs` copies and all three `collect_type_ref_names` copies; typegen keeps a one-line `type_ref_name_strings` adapter for its string sets.
- **Verification:** Verified. `grep` for `fn item_type_refs|fn collect_type_ref_names|fn type_ref_names` across `crates/` now returns only the two `nx-hir` definitions plus typegen's one-line `type_ref_name_strings` adapter. Both duplicate `item_type_refs` bodies and all three `collect_type_ref_names` copies — including the one in `typegen/languages/typescript.rs` — are gone.

### ✅ Verified - RF11 The generated-JavaScript intrinsic tests all place the call as the direct function body, which is why RF1 shipped
- **Severity:** Low
- **Evidence:** Every case in `generated_javascript_applies_an_update_through_the_runtime`,
  `generated_javascript_lists_changed_fields_in_declaration_order`, and
  `generated_javascript_merges_and_diffs_like_the_interpreter`
  ([tests.rs](crates/nx-codegen/src/tests.rs)) is shaped `let root() = {intrinsic(...)}` — never
  inside an element property, never across modules. Two smaller issues in the same tests: the
  `let Some(output) = ... else { return; };` inside the `for` loop of
  `generated_javascript_merges_and_diffs_like_the_interpreter` abandons the remaining cases instead
  of `continue`-ing, and the node-absent skip is silent, so a run without `node` reports success
  with nothing executed (task 4.5 asked for that to be noted; `node` was available in this review's
  run and the generated modules were executed by hand).
- **Recommendation:** Add an element-property case and a cross-module case to the executing tests,
  change the loop's skip to `continue`, and make the node-absent path emit a visible skip notice.
- **Fix:** Added the element-property and workspace cross-module executing cases (see RF1, RF2). The loop in `generated_javascript_merges_and_diffs_like_the_interpreter` uses `continue`, and both execution helpers go through `node_is_available()`, which prints a skip notice on stderr when `node` is missing. `node` was available in this run and every executing test ran.
- **Verification:** Verified. `generated_javascript_calls_an_intrinsic_inside_an_element_property` and `generated_javascript_reaches_property_unions_and_intrinsics_across_workspace_modules` both execute under `node` and compare against the interpreter; the loop in `generated_javascript_merges_and_diffs_like_the_interpreter` uses `continue`; and `node_is_available()` prints "skipping: `node` is not available ..." to stderr. `node` was available in this run and both new tests passed.

### ✅ Verified - RF12 The `nx-grammar.md` addition was appended without rewrapping the paragraph
- **Severity:** Low
- **Evidence:** [nx-grammar.md:129-133](nx-grammar.md#L129-L133) — the new sentence "A derived
  `<Name>.Property` union is a constant union whose cases are the effective field names of
  `<Name>`." was appended to an existing wrapped line, producing one over-long line where the rest
  of the file wraps consistently.
- **Recommendation:** Rewrap the paragraph.
- **Fix:** Paragraph rewrapped.
- **Verification:** Verified. The paragraph is rewrapped — the two affected lines are 98 and 69 characters, in line with the rest of the file.

### ✅ Verified - RF13 The interpreter's `merge` and `diff` do not check that both arguments target the same declaration, while both JavaScript runtimes do
- **Severity:** Low
- **Evidence:** `merge_update_records`
  ([interpreter.rs:2674-2681](crates/nx-interpreter/src/interpreter.rs#L2674-L2681)) discards the
  second argument's `type_name` without comparing it, and `diff_records` likewise uses only
  `before`'s type; the TypeScript runtime's `mergeUpdateRecords` and `diffRecordValues` fail with a
  named diagnostic on a mismatch. The checker prevents the mismatch in typed source, so this is
  defense in depth rather than a live bug, but the three runtimes disagree on what an ill-formed
  value does.
- **Recommendation:** Add the same `type_name` equality checks to the interpreter's `merge` and
  `diff`, reporting through `RuntimeErrorKind::TypeMismatch` as `apply`'s neighbours already do.
- **Fix:** `require_same_record_type` reports a `TypeMismatch` naming both records from `merge_update_records` and `diff_records`. Unit test `merge_and_diff_reject_records_of_different_types` in `interpreter.rs`.
- **Verification:** Verified. `require_same_record_type` reports a `TypeMismatch` naming both records and is called from both `merge_update_records` and `diff_records` ([interpreter.rs:2712-2750](crates/nx-interpreter/src/interpreter.rs#L2712-L2750)). `merge_and_diff_reject_records_of_different_types` passes.

## Questions

- Was the `register_value_bindings` declaration-order sort (RF6) required by property references, or
  is it an independent fix that happened to land here? The answer decides whether RF6 is a
  regression to repair or a change to document.
  - **Answer:** Required. The intrinsic typing rule reads the type of a value argument, which is
    only known once that value's binding has been inferred; two tests fail without the sort. See
    RF6.
- Is the constant-case IR encoding fix (RF5b) meant to ship inside this change? It changes the value
  existing programs produce through the TypeScript IR runtime, which reads like its own
  **BREAKING** note.
  - **Answer:** Yes; it is a prerequisite for property union cases evaluating as bare strings. It
    is now declared in the nx-ir-format delta and listed as **BREAKING** in the proposal. See RF5.

## Summary

The language-facing work is solid and closely follows design.md: the derived union really does reuse
the constant-union machinery end to end, the four intrinsics are isolated behind one check-side and
one eval-side branch as D4 promised, the diagnostics for nested derived names, unextendable unions,
stateless components, and reserved emit names all behave exactly as the `property-references` and
`component-syntax` scenarios specify, and typegen's companion handling mirrors `_update` cleanly.
All four verification suites pass.

The problems are concentrated at the code-generation boundary, which the tests exercise in only one
expression shape. Two are blocking: generated JavaScript is broken whenever an intrinsic call sits
inside an element property (RF1), and a property union reached through a workspace import silently
loses its inherited cases (RF2) — both reproduced by hand and both invisible to the current suites.
RF3 and RF4 are real divergences between the interpreter and the generated runtimes. RF5 and RF6
are scope questions rather than defects: two behavior changes to `==`, to IR, and to hover ride
along without a requirement covering them, and design.md's byte-identical-IR claim is no longer
true. 13 findings opened: 2 High, 5 Medium, 6 Low.

## New Findings Discovered During 2026-09-12 10:30 Verification

### ✅ Verified - RF14 The two emitted runtime modules in `runtime.rs` are maintained by hand and have now drifted twice on the same helpers
- **Severity:** Medium
- **Evidence:** `crates/nx-codegen/src/runtime.rs` holds two parallel raw-string runtime modules —
  one emitted for `--target typescript`, one for `--target javascript` — that must implement the
  same semantics. The intrinsic helpers added by this change diverged in two places:
  `nxDiffRecords` iterates the union of keys in the TypeScript copy and only `Object.keys(after)` in
  the JavaScript copy (RF8), and `nxChangedFields` is `(update, order: readonly string[])` in the
  TypeScript copy but `(update, order = [])` in the JavaScript one, keeping exactly the silent
  insertion-order fallback RF3 was filed to remove. Nothing fails when they drift: no test executes
  the emitted JavaScript runtime's helpers directly, and the emitted TypeScript runtime is only
  type-checked, so both divergences survived a full green run of `cargo test --workspace`,
  `pnpm -r test`, and `dotnet test`.
- **Recommendation:** Make the drift detectable rather than relying on review. Either generate the
  JavaScript module from the TypeScript one (strip the annotations) so there is a single source, or
  add a test that imports the emitted `nx-runtime.js` and exercises `nxApplyUpdate`,
  `nxMergeUpdates`, `nxDiffRecords`, and `nxChangedFields` against the same cases the interpreter
  tests use — including a record missing an optional field, and `nxChangedFields` called without an
  order.
- **Fix:** Drift is now executed rather than reviewed. `emitted_runtime_intrinsic_helpers_agree_across_targets` in `nx-codegen/src/tests.rs` runs one script — `nxApplyUpdate`, `nxMergeUpdates`, `nxDiffRecords` with a record missing an optional field in each position, `nxChangedFields` with an order, and `nxChangedFields` without one — against the emitted JavaScript runtime and against the emitted TypeScript runtime compiled by `tsc`, asserts both against the interpreter's answers, and asserts the two outputs are identical. `nxChangedFields` in both copies now fails through `nxRuntimeError` when the order is missing, closing the silent fallback the JavaScript copy kept. To make the TypeScript half run, `tsc_command()` finds `tsc` on `PATH` or in the repository's own `runtime/typescript/node_modules/.bin`, and the three existing type-check tests use it too; they had been skipping silently wherever `tsc` was not on `PATH`, which is why two pre-existing strict-mode errors in the emitted TypeScript runtime had survived: `NxSchema` typed a union's cases as `NxRecordSchema[]` while the emitter also places enum schemas there for constant cases (the type is now `NxSchema[]` and the union validator narrows to record cases in both copies), and this change's `nxValuesEqual` indexed a narrowed `NxValue` by string (now cast to a record as `runtime/typescript`'s `valuesEqual` does). Generating one module from the other was not attempted: the two copies differ in more than annotations (`unknown` casts, `as` narrowing), so the executing test is the smaller change.
- **Verification (second pass):** Verified. The two runtime copies now agree on all four helpers: `nxDiffRecords` iterates the same key union, and `nxChangedFields(update, order)` in both copies raises `nxRuntimeError("nxChangedFields needs the update record's declared field order")` when the order is missing — confirmed by hand against the emitted `nx-runtime.js`. `emitted_runtime_intrinsic_helpers_agree_across_targets` passed with no skip notice, so both halves ran: `node` executed the JavaScript runtime and `tsc --strict` compiled the TypeScript one, and the test asserts the two outputs are byte-equal as well as correct. Spot-checked its expectations against the interpreter on equivalent NX source (`diff` in both directions, `merge`, `apply`) and all three match; the fix note describes these as compared against the interpreter when they are in fact correct hardcoded literals, which is a wording nit, not a gap.

  The claim about the silently-skipping `tsc` tests checks out, and the counterfactual is worth recording: type-checking the runtime module as it was emitted **before** this pass reports two `--strict` errors — `TS2322` assigning `readonly NxSchema[]` to `readonly NxRecordSchema[]`, and `TS7053` indexing a narrowed `NxValue` by string in `nxValuesEqual` — while the currently emitted module type-checks clean (exit 0, no output). So the three type-check tests were genuinely inert wherever `tsc` was off `PATH`, and `tsc_command()`'s fallback to `runtime/typescript/node_modules/.bin` makes them real. Reviewed the `NxSchema` widening itself: the `find` predicate's `typeof candidate === "object" && "record" in candidate` narrowing is applied symmetrically to both copies and changes no behavior, since an enum schema's absent `record` already failed the old comparison.

## Fix verification

Re-run after the fixes (2026-09-12): `cargo test --workspace` (1756 passed, 0 failed),
`pnpm -r test` (pass), `dotnet test bindings/dotnet` (113 passed),
`openspec validate add-property-references --strict` (valid). Every finding's original repro was
re-run by hand through `nxlang codegen --target javascript` / `--target typescript` and executed
with `node` where applicable.

**11 of 13 verified** (RF1-RF5, RF7, RF9-RF13); RF6 accepted as resolved; **RF8 reopened** because
the fix reached only the TypeScript copy of the emitted runtime. One new finding, **RF14**, records
the structural cause: the two emitted runtime modules are maintained by hand, nothing executes the
JavaScript one's helpers, and they have now drifted twice on the same four functions.

The two blocking findings are genuinely closed. RF2's fix turned out to be broader than the
diagnosis — completing every prepared module's property unions before any importer reads them, and
carrying `X.Update` / `X.Property` along all three import paths — and the workspace program now
builds, runs, and agrees with the interpreter end to end.

## Fix verification (second pass, 2026-09-12)

`cargo test --workspace` (1757 passed, 0 failed), `pnpm -r test` (pass),
`dotnet test bindings/dotnet` (113 passed). Both remaining findings verified by hand against the
freshly emitted runtimes as well as by the new test.

**RF8 and RF14 verified; all 14 findings are now closed** — 12 verified, RF6 resolved, and RF8
verified after one reopen. The two hand-maintained runtime copies agree on every intrinsic helper,
the drift is now executed rather than reviewed, and the `tsc` type-check tests that had been
silently inert now run and pass with `--strict`.
