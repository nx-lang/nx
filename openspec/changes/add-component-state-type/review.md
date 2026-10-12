# Review: add-component-state-type

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, delta specs (component-state-type, update-records,
property-references, component-action-handlers, component-syntax, component-type-parameters, nx-ir-format,
cli-code-generation, typescript-ir-runtime, plus the spelling-only deltas)  
**Reviewed code:** implementation commit 5a16a87 against c58a5b3: `crates/nx-hir` (lower.rs, lib.rs, records.rs,
unions.rs, components.rs, scope.rs, declarations.rs), `crates/nx-types/src/infer.rs` and tests,
`crates/nx-codegen` (builder.rs, ir.rs, model.rs, tests), `crates/nx-ir`, `crates/nx-ir-runtime`,
`runtime/typescript/src/index.ts` and tests, `crates/nx-api/src/artifacts.rs`, `crates/nx-cli/src/typegen*`,
`crates/nx-interpreter`, `crates/nx-language-service/src/source_tree.rs`, .NET fixtures/tests (read only; .NET is
not installed), docs, website pages, examples, IR conformance programs.  
**Checks run:** `cargo test --workspace` (all pass), `pnpm test` in `runtime/typescript` (all pass, `dist`
unchanged after rebuild), `cargo fmt --check` (clean), `openspec validate add-component-state-type --strict`
(valid), plus CLI probes (`nxlang run`/`codegen`/`typegen`) and IR probes through the TypeScript runtime.

## Findings

### ✅ Verified - RF1 A state patch produced by `diff` inside a handler is returned as an effect instead of patching state
- **Severity:** High
- **Evidence:** `referenced_derived_declarations` (`crates/nx-codegen/src/builder.rs:122-209`) emits a derived
  declaration only when it is constructed, named in an annotation, reached by a member access, or appears in the
  inferred type of a top-level function/value. A handler body such as
  `<Button onTapped={diff(<State count={count} />, <State count={count + 1} />)} />` type checks (its result is
  `Counter.State.Update`, which `check_handler_result_item` accepts), and `Counter.State` is emitted (it is
  constructed), but `Counter.State.Update` is never spelled, so it is not emitted. Both runtimes recognize a state
  patch by looking up the result's `$type` declaration and comparing its update target with the component's state
  record (`runtime/typescript/src/index.ts:4384`, `crates/nx-ir-runtime/src/component.rs:520`); with no
  declaration the lookup fails and the patch falls through to `effects`. Reproduced with the TS runtime: dispatching
  the tap returns `state {"count":0}` and `effects [{"$type":"Counter.State.Update","count":1}]`. This violates
  `component-action-handlers` ("A value whose type is ... `<Component>.State.Update` SHALL patch that component's
  state"). `diff` over state records is new in this change (before it a component's patch target could not be
  diffed), so this is a regression the new type introduces.
- **Recommendation:** Close the reference set over state records: when a `<C>.State` record is referenced, also mark
  its `<C>.State.Update` (it lives in the same module, so the lookup mirrors `derived_state_target`). Alternatively,
  emit a stateful component's `State.Update` whenever a handler owned by it exists. Add an IR runtime/conformance
  test that dispatches a handler returning `diff(...)` of two state records and asserts the state changes in both
  runtimes.
- **Fix:** `referenced_derived_declarations` (`crates/nx-codegen/src/builder.rs`) now brings a referenced state record's update record along (new `state_update_record`, same module). The `handlers` IR conformance program gains a `Diffed` component whose handler returns `diff(<State …/>, <State …/>)`; its lifecycle shows the state patched and no effects in both runtimes.
- **Verification:** Verified. `state_update_record` brings a referenced state record's `.State.Update` into the IR. I re-ran the original probe (handler returning `diff` of two state records) through the TS runtime: dispatch now gives `state {"count":1}` and no effects. The new `Diffed` lifecycle in the `handlers` conformance program pins this in both runtimes: the tap gives `count` 1, `note` "tapped" and empty effects. Other conformance images are unchanged, and `cargo test --workspace` passes.

### ✅ Verified - RF2 TypeScript typegen emits invalid code for every exported component without props
- **Severity:** High
- **Evidence:** Every component now has a `T.Property`, so `typegen` emits `<Name>_property` for prop-less
  components too. `ExportedUnion::is_constant` was changed to treat an empty property companion as constant
  (`crates/nx-cli/src/typegen/model.rs:124-128`), and the constant-union path in
  `crates/nx-cli/src/typegen/languages/typescript.rs:552-566` joins zero cases into an empty right-hand side.
  `nxlang typegen` on `export component <Form /> = { <Panel /> }` produces `export type Form_property = ;`, which does
  not parse. Prop-less exported components (`<App />`, `<Spacer />`, external ones) are common, so most libraries'
  generated TypeScript breaks. (C# emits an empty `enum`, which compiles; the executable JS emitter already writes
  `never`.)
- **Recommendation:** In the constant-union branch, emit `never` when `cases` is empty (as the non-constant branch
  at `typescript.rs:586` already does). Add a typegen test for a prop-less exported component, ideally to the test
  that type-checks generated TypeScript with `tsc` (`typegen.rs` around line 5401).
- **Fix:** The TypeScript constant-union path emits `never` when there are no cases. `documented_typescript_type_checks` adds a prop-less `Spacer` and checks `Spacer_property = never` and that host code using `Spacer_update`/`Spacer_property` passes `tsc`.
- **Verification:** Verified. The constant-union branch emits `never` when there are no cases. `nxlang typegen` on `export component <Form />` now writes `export type Form_property = never;`. The `tsc`-backed test now includes a prop-less `Spacer` and its `_update`/`_property` companions. The only other ` | ` join in the emitter (`typescript.rs:606`) already handles the empty case.

### ✅ Verified - RF3 Doc links in state-field docs are checked twice, and links to props are falsely reported unresolved
- **Severity:** Medium
- **Evidence:** `predeclare_state_record` copies each state field, docs included, into the derived `T.State`
  record (`crates/nx-hir/src/lower.rs:897-902`, `..field.clone()`). `resolve_doc_links` skips
  only `RecordKind::Update` (`crates/nx-types/src/doc_links.rs:184`), so the `T.State` record's field docs are
  visited again with only the record in scope. Probe: a state field documented `/// Grows by [step]; see [Nope].`
  in `component <Counter step:int = 1 />` yields three `unresolved-doc-link` warnings: `[Nope]` twice and a spurious
  `[step]` (which resolves fine in the component scope).
- **Recommendation:** Skip every derived record there: `Item::Record(record) if record.is_derived() => {}`, and
  update the comment. Add a doc-links test with a state field doc that links a prop and an unknown name.
- **Fix:** `resolve_doc_links` skips every derived record (`record.is_derived()`). New test `a_state_field_doc_is_checked_once_in_the_component_scope` in `doc_links.rs`: a state field linking a prop and an unknown name yields exactly one warning, for the unknown name.
- **Verification:** Verified. `for_each_doc` skips every derived record (`record.is_derived()`). Re-running my `nx-types` probe on the original source now gives exactly one `unresolved-doc-link`, for `[Nope]`, with no duplicate and no false `[step]`. The new `a_state_field_doc_is_checked_once_in_the_component_scope` test pins this.

### ✅ Verified - RF4 A bare `State` in a component's signature is rewritten, hijacking a top-level `State` for prop types
- **Severity:** Medium
- **Evidence:** In the predeclaration pass `current_component` is set before props, emits and state are lowered
  (`crates/nx-hir/src/lower.rs:1101`), so `resolve_bare_state_type` rewrites `State` in prop and inline-emit field
  types too. With `type State = idle | busy` and `component <C mode?:State /> = { state { count:int = 0 } ... }`,
  `<C mode={State.idle} />` is rejected: "Property 'mode' on 'C' expects C.State?". The spec scopes the bare name to
  "state defaults, its rendered body expression, and any handler body", and says `State` is ordinary outside the
  body. The final lowering pass sets `current_component` only after props (`lower.rs:1373`), so the two passes also
  disagree about how a prop typed `State` lowers.
- **Recommendation:** Rewrite only in state fields, the body and handlers. In the predeclaration pass, set
  `current_component` after the props and emits have been lowered and before the state fields are. Or, if signatures
  are meant to be covered, update the `component-state-type` spec and design to say so. Either way, add a test that
  pins the behavior for a prop typed `State` when a top-level `State` exists.
- **Fix:** Followed the spec: `predeclare_component` restores the enclosing component right after lowering the state fields, so props and emit fields see `State` as an ordinary name, matching the final lowering pass. New test `bare_state_in_the_signature_is_an_ordinary_name`.
- **Verification:** Verified. The predeclaration pass now restores `current_component` right after lowering the state fields, so props and emit fields see `State` as an ordinary name, matching the final lowering pass. The original probe (`type State = idle | busy` with `component <C mode?:State />` and `<C mode={State.idle} />`) now runs. `bare_state_in_the_signature_is_an_ordinary_name` pins it, and the existing bare-`State` tests for state fields, defaults, body and handlers still pass.

### ✅ Verified - RF5 Stale comments and docs still describe a component's own `Update` as its state patch
- **Severity:** Low
- **Evidence:**
  - `crates/nx-ir-runtime/README.md:137`: "Applies a partial state or a `<Component>.Update` record".
  - `crates/nx-hir/src/ast/expr.rs:454`: "its own update record patches the owner's state".
  - `crates/nx-ir-runtime/src/component.rs:964` and `runtime/typescript/src/index.ts:2255`: "patches it with the
    component's update records".
  - `sites/website/src/content/docs/reference/syntax/functions.md:8`: "applies the component's own update records to
    its state".
  - `crates/nx-cli/src/typegen/languages/typescript.rs:654`: "Emits an external component's state record", which
    now covers every component.
  - `crates/nx-cli/src/typegen/model.rs:1109`: the doc line for `companion_target_name` was left above the new
    `companion_stem`, so `companion_stem`'s doc starts with an unrelated sentence and `companion_target_name` has none.
- **Recommendation:** Reword each to `<Component>.State.Update` / "the update record of its state record", and move
  the stray doc line back onto `companion_target_name`.
- **Fix:** Reworded each comment and doc to the `State.Update` spelling (nx-ir-runtime README and `component.rs`, `ast/expr.rs`, TS runtime `index.ts`, website `functions.md`, typegen `typescript.rs`), and moved the stray doc line back onto `companion_target_name`.
- **Verification:** Verified. Every location listed is reworded: nx-ir-runtime README, `component.rs`, `ast/expr.rs`, TS `index.ts` and its rebuilt `dist`, website `functions.md:8`, and typegen `typescript.rs`. The stray doc line is back on `companion_target_name`. A grep finds no remaining 'own update records' or `<Component>.Update` record wording for state patches.

### ✅ Verified - RF6 Some spec scenarios and risky paths have no direct test
- **Severity:** Low
- **Evidence:** No test encodes a handler's `<State.Update label={} />` and checks the JSON is exactly
  `{"$type":"Counter.State.Update","label":null}` (`component-state-type`, "A state patch is encoded under the state
  record's name"). There is no type-checker test for a bare `{Property.count}` value inside a stateful component
  (only the type-position form is tested). No test covers the RF1 and RF2 paths. Task 3.4 (JS emitter, `emit.rs`) is
  checked, but `emit.rs` is unchanged. Its output is correct by name (probe: `apply`/`diff` on `Counter.State`
  generate `Counter_State`/`Counter_State_Update`), but no executable-codegen test covers a component's props patch
  alongside its state patch beyond `crates/nx-codegen/src/tests.rs:2331-2357`.
- **Recommendation:** Add the wire-form test (IR runtime or codegen JSON), the value-position bare `Property` test,
  and the RF1/RF2 regression tests. Note in tasks.md that 3.4 needed no emitter change.
- **Fix:** Added the wire-form check (`clearNote` entrypoint in the `handlers` corpus: `{"$type":"Diffed.State.Update","note":null}`), `a_bare_property_value_inside_a_component_suggests_state_property`, `bare_state_property_case_is_a_value_inside_a_component`, and the RF1/RF2 regression coverage above. tasks.md 3.4 now says no emitter change was needed.
- **Verification:** Verified. The `handlers` corpus pins the wire form (`clearNote` gives `{"$type":"Diffed.State.Update","note":null}`), and the Clear button exercises a state-clearing handler. The new type-checker tests cover a bare `Property` in value position, which suggests `State.Property`, and a `State.Property` case used as a value. RF1 and RF2 have regression tests, and tasks.md 3.4 now says no emitter change was needed.

## Questions
- RF4 (answered by the fix): the spec scopes the bare name to state, the body and handlers, so the signature is left alone.
- RF4 (original): should the bare `State` reach prop and emit-field types in the component signature? The spec says only state
  defaults, the body and handlers, but the design talks about the "component body" loosely.
- For an external component, typegen now emits `<Name>_update` as an untyped `NxUpdateRecord` in C#, although
  typegen also emits a props contract record under the component's name. Is `NxUpdate<Ticker>` (applying to the
  props contract) wanted for external components? That would be a follow-up, not a defect.

## Summary
- The change is broadly complete and coherent. HIR synthesis, bare-`State` resolution, handler routing, the IR v6
  layout, both runtimes, typegen companions and the docs and samples all move together, and the Rust and TypeScript
  suites pass.
- Two high-severity defects should be fixed before archiving:
  - A `diff`-built state patch in a handler is silently treated as an effect (RF1).
  - TypeScript typegen produces uncompilable output for prop-less exported components (RF2).
- The doc-link duplication (RF3) and the signature-scope question (RF4) are smaller but user-visible.

## Verification Summary
- All six findings (RF1–RF6) are verified against commit 9c37d4c. `cargo test --workspace`, the TypeScript runtime
  tests and `cargo fmt --check` pass, and the original probes for RF1–RF4 now behave as the specs require. No new
  findings.

## New Findings Discovered During 2026-10-12 05:25 Review

**Scope of this pass:** only commit fcb6f95 "Drop a component's T.Update and T.Property" (`git diff e35769b fcb6f95`),
read against the revised proposal, design, tasks and delta specs, plus a repo-wide search (crates/,
runtime/typescript, bindings/, packages/, sites/, docs/, nx-grammar*.md, openspec/specs and this change) for anything
that still assumes a component has `T.Update`/`T.Property` or `<Name>_update`/`<Name>_property`.  
**Checks run:** `cargo test -p nx-types --test component_state_type --test property_references --test update_records
--test unresolved_type_names` (all pass); `cargo test -p nx-cli fixture` (the .NET `UpdateRecords.g.cs` and prelude
fixtures match typegen output); `nxlang run`/`nxlang codegen --target nx-ir` probes of the new diagnostics: a
stateful, stateless, abstract and external component's `.Update`/`.Property` in tag, type, value, handler and
intrinsic positions, through a workspace import, and the `.State` cascade cases. Every probe gave exactly one
diagnostic with the expected suggestion, so the `unresolved_derived_name_message` branch and the intrinsic cascade
suppression behave as the specs require. I found no behavioral bug in the commit; the findings are spec gaps, dead
code and coverage.

### 🟡 Fixed - RF7 Main-spec requirements the change does not touch still say typegen emits a component's `<Name>_update` and `<Name>_property`
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** The `cli-code-generation` delta now says a component "SHALL NOT get a `<Name>_update` of its own" and
  "SHALL NOT get a `<Name>_property`", but the delta modifies only the two companion requirements. Several other
  requirements in the main specs still use the old names, and nothing in the change modifies them, so archiving would
  leave the specs contradicting themselves and the implementation:
  - `openspec/specs/cli-code-generation/spec.md:633-637` (requirement "C# generation emits typed property keys beside
    each property companion", scenario "External component state gets keys over its state record"):
    "`Ticker_update` SHALL apply to and diff `Ticker_state` values". Typegen now emits `Ticker_state_update` and no
    `Ticker_update` (`crates/nx-cli/src/typegen.rs`, the `Ticker` test asserts `!output.contains("Ticker_update")`).
  - Same requirement, `:645-648` (scenario "Component state gets a property companion but no keys"): "C# generation
    SHALL emit `Counter_property` with the single case `count`". It is now `Counter_state_property`, and
    `Counter_property` is asserted absent. The requirement text at `:612-616` ("a non-external component's state …
    generation SHALL emit the property companion alone") is also out of date: every exported component with state now
    gets `<Name>_state`.
  - `:650-662` (requirement "Generated contracts erase component type parameters …"): "A generic component's
    `<Name>_update` companion … SHALL erase". Scenario `:684-687` "State and update companion erase the parameter"
    checks `Picker_update`.
  - `:745-747` (requirement "Generated contracts carry a generic record as a generic type in both languages",
    scenario "A component's state companion still erases the component's parameters"): "the generated `Picker_update`
    SHALL declare no generic parameter". The implementation and test
    (`a_generic_component_state_and_its_update_companion_erase_the_parameter`) use `Picker_state_update`.
  - `openspec/specs/dotnet-binding/spec.md:708-710` (requirement "Managed update records support apply, merge, diff,
    and changed", scenario "Merge and changed work on an update with no plain record type"): "merges two
    `Counter_update` values for a component's state … `Counter_property` values". This commit moved the matching .NET
    test to the abstract record `Stepped` (`NxUpdateRecordTests.AbstractRecordUpdate_WithNoPlainType_…`), and the
    change has no `dotnet-binding` delta. The requirement text at `:682-685` ("component state, which [has] no plain
    record type") is also out of date, because `Counter_state_update` derives from `NxUpdate<Counter_state>`.
  - Most of this text was already out of date after the first implementation commit, which renamed the state
    companions to `_state_update`/`_state_property`. It falls in this review because this commit is the one meant to
    remove every component `_update`/`_property`.
- **Recommendation:** Add `MODIFIED` blocks to `specs/cli-code-generation/spec.md` for the three requirements above.
  Rename `Ticker_update` → `Ticker_state_update`, `Counter_property` → `Counter_state_property` and `Picker_update` →
  `Picker_state_update`, reword "a non-external component's state" (an abstract record is the remaining no-keys case),
  and say that a component's state companions erase. Add a `specs/dotnet-binding/spec.md` delta that moves the
  "no plain record type" scenario to an abstract record (`Stepped_update`/`Stepped_property`, matching the .NET
  test) and drops "component state" from the list of DTOs without a plain type. Re-run
  `openspec validate add-component-state-type --strict`.
- **Fix:** Added MODIFIED blocks to the `cli-code-generation` delta for "C# generation emits typed property keys beside each property companion" (every component with state gets keys over `<Name>_state`; `Ticker_state_update`, `Counter_state_property`, no `Ticker_update`/`Counter_property`), "Generated contracts erase component type parameters …" (`<Name>_state` and `<Name>_state_update` erase; `Picker_state_update`) and "Generated contracts carry a generic record …" (`Picker_state_update`). Added a `dotnet-binding` delta that moves the no-plain-type scenario to `Stepped_update`/`Stepped_property` and drops component state (and actions, which do have plain types) from the no-plain-type list. Listed both in the proposal. `openspec validate --strict` passes.

### 🟡 Fixed - RF8 The change's own artifacts still give a component a derived update record in two places
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - `openspec/changes/add-component-state-type/specs/nx-ir-format/spec.md:190-194` (MODIFIED requirement "NX IR
    erases component type parameters"): "… the component's derived update record SHALL describe each field the same
    way". A component no longer has one. The scenario under it (`List.State.Update`/`List.State`) is already correct,
    so the requirement text and its scenario disagree.
  - `design.md:40-41` (Non-Goals: "Typing the previewer's `setProps` … as `T.Update`. That can follow once the type
    exists.") and `proposal.md:72-73` ("typing it as `T.Update` is a possible follow-up"). These still assume a
    component `T.Update` will exist. Under the 2026-10-12 decision, that follow-up belongs to the future props record.
- **Recommendation:** In the nx-ir-format delta, write "its derived state record and that record's update record SHALL
  describe each field the same way". In the design and proposal, say that `setProps` could be typed by the future
  props record (for example `T.Properties.Update`), not by `T.Update`.
- **Fix:** The nx-ir-format delta now says the component's derived state record and that record's update record describe each field the same way. The design's non-goal and the proposal's impact note now say `setProps` could be typed by the future props record's update (perhaps `T.Properties.Update`), not `T.Update`.

### 🟡 Fixed - RF9 Dead code and stale comments still describe a component's own update record
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - `crates/nx-codegen/src/builder.rs:2874-2883`: `erased_record_type_params` keeps the arm
    `_ => target.clone()`, which treats an update target that is not a record as a component. No update record
    targets a component any more, so the arm is unreachable. Its comment ("the update of a component or of its state
    record copies props or state") and the function doc (`:2850-2857`: "the update record of such a component", "a
    component's prop or state annotations") are out of date.
  - `crates/nx-codegen/src/model.rs:157`: `property_target` is documented as "The record, action, or component a
    derived `<Target>.Property` union names". It should say "record, action, or component state record", as this
    commit already wrote in `nx-hir/src/lib.rs`.
  - `crates/nx-api/src/artifacts.rs:2834-2836`: the doc of `DERIVED_DECLARATION_SUFFIXES` says an import of a
    "record, action, or component" brings "its update record and property union". For a component it brings only
    `State`, `State.Update` and `State.Property`. The code is harmless (missing items are skipped), but the comment is
    wrong.
  - `runtime/typescript/test/runtime.test.ts:974` and `runtime/typescript/dist/test/runtime.test.js:775`: "The
    component's own update record patches its props, not its state." The assertion under it (`Card.Update` is
    rejected) is still right. The Rust counterpart in `ir_runtime_tests.rs:3243` was reworded in this commit, but this
    one was missed.
  - `docs/add-update-actions-followsup.md:138` still describes "a component's `Counter_update`, which has no plain
    type". This is a history note, so updating it is optional.
- **Recommendation:** Replace the `_ => target.clone()` arm with `_ => return Vec::new()` (or reduce the match to the
  state-record cases), and update the three Rust comments to say "component state record". Reword the TS test
  comment to match the Rust one ("A component has no update record of its own, so a `Card.Update` patches nothing"),
  then rebuild `dist`.
- **Fix:** `erased_record_type_params` returns no parameters for an update whose target is not a state record (the component arm is gone) and its doc and comments describe only state records. Reworded `model.rs` `property_target`, the `DERIVED_DECLARATION_SUFFIXES` doc and the TS runtime test comment (and rebuilt `dist`). Left `docs/add-update-actions-followsup.md:138` as is, since it records what an earlier archived change did.

### 🟡 Fixed - RF10 The new cascade suppression and the cross-module diagnostic are only partly tested
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `crates/nx-types/src/infer.rs` adds suppression in two places. One is `intrinsic_update_argument`
  (`:3693-3698`), covered by `an_unknown_derived_name_passed_to_an_intrinsic_is_reported_once` (`changed` only). The
  other is `intrinsic_record_argument` (`:3653-3658`, the record argument of `apply`/`diff`, such as
  `diff(<State />, <State />)` in a component without state, or `apply(<Form.State …/>, …)`); no test covers it.
  `merge` with two unknown operands is not covered either. The suppression keys on spelling
  (`is_derived_record_name`) plus a failed `record_definition_for`, so a regression could silently drop a needed
  diagnostic or bring the cascade back. Probes show it works today: `diff(<State />, <State />)` in a stateless `Form`
  gives exactly two `unknown-state-record` diagnostics and no `intrinsic-argument-type`. Separately, this commit
  deleted the only nx-api test of a component's derived names through a workspace import
  (`props_companions_of_a_workspace_import_include_inherited_props`), and no test pins the new suggestion for an
  imported component. A workspace probe gives the right message.
- **Recommendation:** In `component_state_type.rs`, add an "exactly these diagnostics" test for `apply`/`diff` with an
  unknown `X.State` record argument and for `merge` of two `Counter.Update` operands. In
  `crates/nx-api/src/artifacts.rs`, add a workspace test asserting that `<Counter.Update …/>` on an imported stateful
  component reports `unknown-update-record` with the `Counter.State.Update` suggestion.
- **Fix:** Added `unknown_derived_records_passed_to_record_and_update_intrinsics_are_reported_once_each` (`diff` of two unknown `Form.State` records, `merge` of two `Counter.Update`s: exactly one diagnostic per operand, no `intrinsic-argument-type`) and the nx-api workspace test `update_record_of_a_workspace_imported_component_suggests_its_state_update`.

### Questions (this pass)
- None.

### Summary (this pass)
- The commit does what the 2026-10-12 decision asks. HIR no longer synthesizes a component's `T.Update`/`T.Property`.
  The props-shape resolution paths (`effective_component_props_in`, `resolve_component_props_shape`) and the
  `handler-update-props` diagnostic are gone. Typegen drops `target_is_component` and emits no component
  `_update`/`_property`. The new unknown-name diagnostics and the cascade suppression behave correctly in every probe.
- The main gap is in the specs (RF7): the main `cli-code-generation` and `dotnet-binding` specs still name component
  `<Name>_update`/`<Name>_property` in requirements this change does not modify. Fix them before archiving. RF8–RF10
  are small cleanups and test additions.
