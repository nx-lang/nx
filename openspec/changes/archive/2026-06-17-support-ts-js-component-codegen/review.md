# Review: support-ts-js-component-codegen

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/executable-code-generation/spec.md

**Reviewed code (working tree vs HEAD):**
- `crates/nx-codegen/src/model.rs` — new component/descriptor model types
- `crates/nx-codegen/src/builder.rs` — component build, descriptor vs function-call routing, action-handler rejection
- `crates/nx-codegen/src/emit.rs` — component class / state companion / initialize / evaluate / try* emission, descriptor emission, runtime-helper selection
- `crates/nx-codegen/src/runtime.rs` — `NxRuntimeError`, `NxResult`, normalization helpers
- `crates/nx-codegen/src/lib.rs` — exports
- `crates/nx-codegen/src/tests.rs`, `crates/nx-cli/src/main.rs` — component codegen tests
- `README.md` — documentation

**Verification performed:** `cargo test -p nx-codegen` (29 passed) and `cargo test -p nx-cli codegen` (4 passed) both green on this working tree.

Note: the working tree also contains unrelated edits to `crates/nx-cli/src/typegen.rs`, `typegen/languages/csharp.rs`, and `typegen/model.rs` that belong to the separate `preserve-csharp-literal-defaults` change; those were treated as out of scope.

## Findings

### ✅ Verified - RF1 Enum/record/union/component-typed props and state bypass value validation
- **Severity:** Medium
- **Evidence:** `emit_type_schema` ([emit.rs](../../../crates/nx-codegen/src/emit.rs)) maps any non-primitive `TypeRef::Name` to `js_string("any")`, and `nxNormalizeValue` returns `"any"` values unchanged ([runtime.rs](../../../crates/nx-codegen/src/runtime.rs)). So an `enum`-typed prop/state field accepts any string (e.g. `mode: "bogus"`), and record/union-typed fields accept any object, without the membership/shape validation the interpreter applies. Generated `evaluate`/`initialize` therefore diverge from interpreter semantics on invalid host input, producing descriptors with illegal enum members instead of failing. The design explicitly listed enum parity as a tracked risk ("[Generated JSON validation becomes inconsistent with interpreter coercion] -> ... add parity tests for ... enums"), and `generated_component_entry_handles_enums_nullable_fields_and_lists` only exercises *valid* enum values.
- **Recommendation:** Either emit an enum-membership check (the enum cases are already in the codegen model) so invalid members throw `NxRuntimeError`, or, if pass-through is intentional for this phase, add a negative parity test documenting the trust boundary and note the limitation in the spec/README.
- **Status:** Previously left open per request, now addressed.
- **Fix:** Extended generated runtime schemas and `nxNormalizeValue` so generated JSON-facing prop/state normalization validates enum membership, typed record `$type` and fields, union case `$type` and fields, and component descriptor `$type` and props. Added negative generated-JavaScript tests for invalid enum prop/state input plus invalid record/union/component-shaped host values.
- **Verification:** Confirmed. `emit_type_schema_inner`/`emit_named_type_schema` now emit `{ enum }`, `{ record, fields }`, `{ union }`, and component schemas with a `seen` recursion guard (cyclic refs degrade to `"any"`), resolving named types via imports then local declarations. `nxNormalizeValue` validates each in both TS and JS runtimes (`invalid-enum`, `invalid-record-type`, `invalid-union`, `unknown-field`/`invalid-field`), with `$type` requirement via `nxRequireRecordType`. Both `nxNormalizeRecordValue`/`nxRequireRecordType` are present in the TS and JS runtime strings (parity). `generated_component_entry_rejects_invalid_enum_host_input` and `generated_component_entry_rejects_invalid_named_host_input_shapes` execute generated JS and assert the specific diagnostic codes for prop and state paths. Full `cargo test -p nx-codegen` (29) green.

### ✅ Resolved - RF2 Element content is silently dropped for components without a content prop
- **Severity:** Low
- **Evidence:** In `emit_component_descriptor` ([emit.rs](../../../crates/nx-codegen/src/emit.rs)), content is only attached when `descriptor.content_field` is `Some`. If a descriptor supplies children but the target component declares no content prop (`content_field == None`), `descriptor.content` is discarded with no diagnostic. This contradicts the change's stated principle of failing rather than silently dropping data (mirrors the explicit action-handler rejection).
- **Recommendation:** Confirm the type checker rejects content on a no-content-prop component before codegen; if it does, this is moot — otherwise emit a codegen diagnostic instead of dropping the content.
- **Status:** Confirmed this is moot: the type checker rejects `<Panel><span /></Panel>` for `external component <Panel />` before codegen with an error that the component does not declare a content property.
- **Verification:** Independently reproduced via the `nxlang` CLI: `codegen` on `external component <Panel /> ... <Panel><span /></Panel>` fails before emission with `Element 'Panel' passes body content, but 'Panel' does not declare a content property` and writes no output. The silent-drop path in `emit_component_descriptor` is therefore unreachable for this case. Resolved as not-a-bug.

### ✅ Verified - RF3 Abstract "contract-only" behavior has no negative test
- **Severity:** Low
- **Evidence:** The implementation correctly gates state companions and `initialize`/`evaluate`/`render` behind `!component.is_abstract` ([emit.rs](../../../crates/nx-codegen/src/emit.rs) `emit_component_declaration`). But spec scenarios "Abstract component is contract-only" and "Abstract component emits no state companion" have no asserting test. The cross-module test uses an abstract external `Question` yet never asserts the absence of `initialize`/`State` on the abstract class, so a regression that started emitting an instantiable abstract API would not be caught.
- **Recommendation:** Add a test that declares an abstract component and asserts the generated output contains the base contract but no `<Name>State`, `static initialize(`, or `static evaluate(`.
- **Fix:** Added `abstract_components_emit_contract_only_surface`, which asserts an abstract base emits its interface/abstract class and no state companion or initialize/evaluate/try* API surface.
- **Verification:** Confirmed. The test declares `abstract component <SearchBase>` with a concrete `SearchBox extends SearchBase`, slices the abstract base class body (between `export abstract class SearchBase` and the following `export interface SearchBox`), and asserts it contains `static normalizeProps(` but none of `static initialize(`/`evaluate(`/`tryInitialize(`/`tryEvaluate(`, plus no `SearchBaseState`. This guards both the "contract-only" and "no state companion" spec scenarios against regression. Test passes.

### ✅ Verified - RF4 Component "snapshot" tests are substring assertions, not snapshots
- **Severity:** Low
- **Evidence:** Task 5.1 calls for snapshot tests of generated component classes, state companions, descriptor construction, and helper imports. `emits_component_classes_state_companions_and_short_api_names` and the CLI tests use `module.contains(...)` checks rather than golden-file snapshots, so structural/formatting regressions in the emitted classes (ordering, signatures, whitespace) are largely unguarded.
- **Recommendation:** Add at least one golden/inline snapshot of a representative component module (matching the existing `emits_supported_subset_snapshots_in_both_target_modes` style) covering a stateful component + descriptor.
- **Fix:** Added `emits_component_module_inline_snapshot`, an inline golden assertion over the complete generated TypeScript module body for a stateful component and root descriptor construction.
- **Verification:** Confirmed. The test asserts `generated_module_body(...).trim_end()` equals a full inline golden string covering the runtime-helper import line, the `TextInput`/`SearchBox` interfaces, the `SearchBoxState` companion, and the `normalizeProps`/`initialize`/`evaluate`/`render` bodies plus the root descriptor's `SearchBox.normalizeProps({ placeholder: "Manual" })` call — exact ordering, signatures, and whitespace, so structural/formatting regressions now fail. Test passes.

## Questions
- RF2: Answered. The NX type checker already rejects element content passed to a component that declares no content prop, so RF2 is resolved as not-a-bug.
- RF1: Answered by implementation. Validation now happens at generated JSON-facing prop/state normalization boundaries.

## Summary
Solid, well-structured implementation that matches the proposal and design: descriptors are atomic (`normalizeProps`, no body evaluation), function element calls stay eager, state lives in companion classes with `initial`/`fromJSON`/`toJSON`, `initialize`/`evaluate` plus `try*` variants are emitted, action-handler bindings are rejected with a diagnostic, and abstract components are correctly contract-only. All targeted `nx-codegen` (29) and `nx-cli` codegen (4) tests pass. RF1, RF3, and RF4 are now **✅ Verified** by the reviewer — non-primitive (enum/record/union/component) host input is validated in both the TS and JS runtimes with executed-JS negative tests, the abstract contract-only surface has a guarding test, and a full-module inline golden snapshot is in place. RF2 is **✅ Resolved** as not-a-bug, independently reproduced via the CLI: the type checker rejects content on a no-content component before codegen. No open findings remain.

## New Findings Discovered During 2026-06-17 01:13 Review

The implementation was substantially rewritten since the previous review (the `normalizeProps` /
`SearchBox.normalizeProps(...)` / class-method surface that RF1/RF4 verified no longer exists; the
runtime now uses a schema-driven `nxComponentSchema` / `nxExternalComponentSchema` design with
`initializeJson` / `evaluateJson`). This pass re-reviewed the code from scratch against
`proposal.md`, `design.md`, `tasks.md`, and `specs/executable-code-generation/spec.md`, plus the
interpreter (`crates/nx-interpreter/src/interpreter.rs`).

**Reviewed code (working tree vs HEAD):** `crates/nx-codegen/src/{model,builder,emit,runtime,lib}.rs`,
`crates/nx-codegen/src/tests.rs`, `crates/nx-cli/src/main.rs`, `README.md`. Cross-checked against
`crates/nx-interpreter/src/interpreter.rs`. `cargo test -p nx-codegen` (37 passed) is green.

### ✅ Verified - RF5 Normal (non-external) component element expressions are deep-rendered instead of producing atomic descriptors, diverging from the interpreter and the spec
- **Severity:** High
- **Evidence:** For a normal component element such as `<Child label="Name" />`,
  `emit_component_descriptor` ([emit.rs:2208-2244](../../../crates/nx-codegen/src/emit.rs#L2208-L2244))
  emits `Child({ label: "Name" })`, which calls the exported normal-component function
  ([emit.rs:1014-1068](../../../crates/nx-codegen/src/emit.rs#L1014-L1068)) and therefore evaluates
  the component body via `renderChild`. The interpreter does the opposite: `eval_element_expr`
  returns an **atomic** `Value::Record { type_name: component.name, fields: normalized_props }` for
  *every* non-abstract component — external **and** normal — without evaluating the body
  ([interpreter.rs:2292-2329](../../../crates/nx-interpreter/src/interpreter.rs#L2292-L2329)).
  Reproduced via the CLI on the spec's own example (`component <Child label:string /> = { <Text value={label} /> }` / `let root() = { <Child label="Name" /> }`):
  - interpreter (`nxlang run`): `{ "$type": "Child", "label": "Name" }`
  - generated JS (`nxlang codegen` + node): `{ "$type": "Text", "value": "Name" }`

  This violates spec scenarios "Component expression returns descriptor without evaluating body" and
  "Component descriptor construction does not deep-render children", the "Component descriptor parity"
  requirement, and design Decision 1 ("It must not evaluate `SearchBox`'s component body as a side
  effect of constructing that descriptor"). Only an explicit entry-API call
  (`SearchBoxSchema.initializeJson`/`evaluateJson`) is supposed to evaluate a body. The README at
  [README.md:133-135](../../../README.md#L133-L135) documents this divergent behavior as intentional,
  but it contradicts the interpreter, which design Non-Goals say must not change.
- **Recommendation:** For `CodegenComponentTargetKind::Normal` descriptors, emit an atomic descriptor
  object — `{ $type: "Child", ...resolveChildProps({...}) }`, mirroring the external factory
  ([emit.rs:907-949](../../../crates/nx-codegen/src/emit.rs#L907-L949)) — instead of calling the
  render function. Restrict body evaluation to the entry `initialize`/`evaluate` APIs. Then reconcile
  README lines 133-135 and re-verify that evaluating a parent entry returns child descriptors rather
  than a deep-rendered tree.
- **Fix:** Normal component functions now construct atomic descriptors with generated `Element`
  types, while `render*` helpers and `*Schema.initializeJson` / `evaluateJson` are the only paths
  that evaluate normal component bodies. Normal component schemas also expose descriptor schemas for
  component-typed validation.
- **Verification:** Confirmed. `emit_component_declaration` now routes both external and normal
  concrete components through `emit_component_descriptor_factory`
  ([emit.rs:739-748](../../../crates/nx-codegen/src/emit.rs#L739-L748),
  [emit.rs:906-948](../../../crates/nx-codegen/src/emit.rs#L906-L948)), so the exported `Child(props)`
  emits `{ $type: "Child", ...resolvedProps }`; `renderChild` (body evaluation) is now reached only
  from the schema's `initialize`/`evaluate` ([emit.rs:1177-1227](../../../crates/nx-codegen/src/emit.rs#L1177-L1227)).
  Re-ran the spec's own example through the CLI: interpreter (`nxlang run`) and generated JS
  (`nxlang codegen` + node) both yield `{"$type":"Child","label":"Name"}`, and the generated module
  shows `export function root() { return Child({ label: "Name" }); }` →
  `return { $type: "Child", label: resolvedProps.label };`. The README at lines 133-140 is reconciled
  (normal and external elements both construct atomic descriptors; bodies evaluate only via the
  `*Json` entry APIs). Parity restored.

### ✅ Verified - RF6 The only normal-child component test encodes the buggy behavior and skips interpreter parity, masking RF5
- **Severity:** Medium
- **Evidence:** `generated_component_bodies_render_normal_child_components`
  ([tests.rs:1084-1101](../../../crates/nx-codegen/src/tests.rs#L1084-L1101)) asserts a hardcoded
  `"rendered child"` rather than comparing against the interpreter with
  `assert_json_values_eq(&output, &interpreter_json_root(...))` (the pattern every other parity test
  uses). Task 5.4 ("child component descriptors inside parent component bodies are not deep-rendered")
  therefore has no asserting test — the implemented behavior is the inverse of what 5.4 requires, and
  the green suite hides it. No parity test exercises a *normal* component used as a child descriptor.
- **Recommendation:** Once RF5 is fixed, replace the hardcoded assertion with an interpreter-parity
  check and add a dedicated 5.4 test proving a normal `<Child />` inside a parent body yields a
  `Child` descriptor (not its rendered body).
- **Fix:** Replaced the hardcoded normal-child render assertion with
  `generated_normal_component_descriptor_matches_interpreter`, and added
  `generated_component_bodies_return_normal_child_descriptors` to prove parent entry evaluation
  returns a `Child` descriptor rather than the child render result.
- **Verification:** Confirmed. The old masking test
  `generated_component_bodies_render_normal_child_components` is gone; no `"rendered child"` hardcoded
  assertion remains. `generated_normal_component_descriptor_matches_interpreter`
  ([tests.rs:1094-1105](../../../crates/nx-codegen/src/tests.rs#L1094-L1105)) now asserts
  `assert_json_values_eq(&output, &interpreter_json_root(source))` — true interpreter parity.
  `generated_component_bodies_return_normal_child_descriptors`
  ([tests.rs:1107-1124](../../../crates/nx-codegen/src/tests.rs#L1107-L1124)) evaluates
  `ParentSchema.evaluateJson({})` and asserts the result is `{ "$type": "Child", "label": "Name" }`
  rather than the child body `"rendered child"`, giving task 5.4 a real assertion. Both tests pass
  (node available); `cargo test -p nx-codegen` is 37 passed / 0 failed.

### ✅ Verified - RF7 design.md (Decisions 3–5) is out of sync with the implemented API surface
- **Severity:** Low
- **Evidence:** design.md describes generated component entry **classes** with `SearchBox.State` and
  the *shorter* method names `initialize` / `evaluate` / `tryInitialize` / `tryEvaluate`
  ([design.md:74-138](design.md)), and spec scenario "Concrete component emits a generated class"
  says "component entry class." The implementation emits **no classes** (tests assert
  `!module.contains("export class SearchBox")` and `!"static initialize("`,
  [tests.rs:725-728](../../../crates/nx-codegen/src/tests.rs#L725-L728)) and exposes
  `SearchBoxSchema.initializeJson` / `evaluateJson` / `tryInitializeJson` / `tryEvaluateJson` — the
  `Json`-suffixed names that Decision 3 explicitly argued *against*. This is a documentation/spec
  drift, not a correctness bug; the function + schema surface is internally consistent and tested.
- **Recommendation:** Update design.md and the spec scenario wording to describe the
  function-plus-`*Schema` approach and the `*Json` method names (or rename for consistency with the
  design's stated intent), so the artifacts match the shipped surface before archiving.
- **Fix:** Updated README and the change proposal, design, delta spec, and tasks to describe
  descriptor functions, `*Schema` JSON entry APIs, `*Json` method names, and state helpers/types
  instead of generated component classes or state companion classes.
- **Verification:** Confirmed. No "class" mentions remain in proposal.md, design.md, or the delta
  spec. The spec requirement is reworded to "Generated code emits component descriptor functions and
  schema entry objects" with scenario "Concrete component emits a descriptor function and schema
  entry" referencing `SearchBoxSchema`, and the state requirement/diagnostic scenarios now use
  `evaluateJson` / `tryEvaluateJson`
  ([specs/executable-code-generation/spec.md:3-16,78-125](specs/executable-code-generation/spec.md)).
  README lines 122-141 describe the `SearchBox(props)` descriptor function, `SearchBoxElement`,
  `SearchBoxSchema`, the `*Json` entry APIs, and `initialSearchBoxState` / `renderSearchBox` helpers —
  matching the shipped surface. Documentation/spec drift resolved.

## Updated Questions
- RF5: Answered by implementation. Deep-rendering normal component element expressions was treated
  as a bug; codegen now emits atomic normal-component descriptors and keeps body evaluation behind
  explicit schema entry APIs.

## Updated Summary
A from-scratch re-review of the rewritten implementation. RF1–RF4 from the prior pass concern code
that no longer exists, but the equivalent concerns (schema-driven enum/record/union/component
validation, abstract contract-only surface, inline snapshot) are still covered by current tests.
RF5 is fixed by making normal component element expressions atomic descriptors, RF6 is fixed by
adding interpreter parity and parent-entry descriptor tests, and RF7 is fixed by aligning README and
OpenSpec artifacts with the function-plus-schema API surface. Targeted `nx-codegen` and `nx-cli`
codegen suites pass after the fixes.

**Verification pass (2026-06-17 01:22):** RF5, RF6, and RF7 are all **✅ Verified**. Normal-component
element expressions now emit atomic descriptors (CLI re-run confirms interpreter/generated-JS parity
on the spec example); the masking test is replaced with interpreter-parity and parent-entry
descriptor tests (task 5.4 now has a real assertion); and the design/spec/proposal/tasks/README no
longer reference component classes. `cargo test -p nx-codegen` (37 passed) and
`cargo test -p nx-cli codegen` (4 passed) are green. No findings remain open; no new findings.
