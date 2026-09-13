# Review: add-component-type-parameters

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md` (19/19 marked done), and all eight
delta specs (`component-type-parameters`, `component-syntax`, `component-contract-inheritance`,
`unbraced-literal-forms`, `primitive-type-names`, `nx-ir-format`, `external-components`,
`cli-code-generation`).

**Reviewed code:** the whole working tree for this change —
`crates/nx-syntax` (`grammar.js`, `ast.rs`, `validation.rs`, `tests/parser_tests.rs`; generated
`parser.c`/`grammar.json`/`node-types.json` skimmed only),
`crates/nx-hir` (`lib.rs`, `lower.rs`, `components.rs`, `prepared.rs`, `scope.rs`),
`crates/nx-types` (`ty.rs`, `infer.rs`, `check.rs`, `semantics.rs`, `tests/`),
`crates/nx-interpreter`, `crates/nx-api/src/artifacts.rs`,
`crates/nx-codegen` (`builder.rs`, `emit.rs`, `ir.rs`, `model.rs`, `tests.rs`),
`crates/nx-cli/src/typegen` (`model.rs`, `languages/csharp.rs`, `languages/typescript.rs`),
`crates/nx-language-service`, and `docs/src/content/docs/reference/syntax/functions.md`.

**Verification run:** `cargo test --workspace` is green (60 suites, 0 failures) and
`cargo fmt --all --check` is clean. `cargo clippy --workspace --all-targets` is *not* clean, but
every error reproduces on a stashed tree (`approx_constant` at
[expr.rs:387](crates/nx-hir/src/ast/expr.rs#L387), the `nx-ffi` raw-pointer lints, and a missing
`criterion` dev-dependency for the `nx-syntax` bench) — none are caused by this change. Task 19's
"all green" claim is overstated only in that pre-existing respect.

Every finding below was reproduced with a throwaway test against the working tree; the scratch
tests were removed afterwards and the tree is byte-identical to how the review found it.

## Findings

### ✅ Verified - RF1 `TItem=object` is rejected as "not a visible type", and the diagnostic suggests `object`
- **Severity:** Medium
- **Evidence:** [infer.rs:2486-2493](crates/nx-types/src/infer.rs#L2486-L2493) decides visibility
  with `builtin_type`, which deliberately omits `object` because `object` is a `Type::Named`, not a
  `Primitive` ([semantics.rs:116-129](crates/nx-types/src/semantics.rs#L116-L129)). Its sibling
  `visible_type_names` at [infer.rs:2496-2508](crates/nx-types/src/infer.rs#L2496-L2508) hardcodes
  a separate list that *does* include `object`, so the two disagree. Checking
  `external component <List TItem:type items:TItem[]? />` with `let v = <List TItem=object items={ 1 2 } />`
  reports:
  `Type parameter 'TItem' on 'List' expects a type name, and 'object' is not a visible type; did you mean `object`?`
  followed by a cascading `Property 'items' on 'List' is typed by 'TItem', which was not specified`.
  The `component-type-parameters` spec requires resolution against "primitive type names, and every
  record, union, alias, and type parameter in scope", and `primitive-type-names` counts `object`
  among the eight; the new docs section also advertises "a primitive" as a valid argument. The
  self-contradicting suggestion makes this read as a defect rather than a restriction.
- **Recommendation:** Make `is_visible_type_name` answer for `object` (and derive
  `visible_type_names`'s primitive list from the same source so the two cannot drift again — e.g. one
  `PRIMITIVE_TYPE_NAMES` slice consulted by both). Add a scenario to the
  `component-type-parameters` "primitive, alias, and union" case covering `TItem=object`.
- **Fix:** Added `PRIMITIVE_TYPE_NAMES` (the eight names, `object` included) in
  [semantics.rs](crates/nx-types/src/semantics.rs) beside `builtin_type`, and made both
  `is_visible_type_name` and `visible_type_names` in `infer.rs` read it, so the two cannot drift.
  The "primitive, alias, and union" scenario and its test now include
  `<List TItem=object items={ 1 "two" } />`.
- **Verification:** Confirmed. `PRIMITIVE_TYPE_NAMES` is a single `pub(crate) const` in
  [semantics.rs](crates/nx-types/src/semantics.rs) and both `is_visible_type_name` and
  `visible_type_names` in [infer.rs:2486-2506](crates/nx-types/src/infer.rs#L2486-L2506) read it, so
  the two lists can no longer disagree. `<List TItem=object items={ 1 "two" } />` now checks with
  zero diagnostics (it previously produced the contradictory message plus a cascading
  "not specified" error), and the near-match path is unregressed: `TItem=Contatc` still reports
  "'Contatc' is not a visible type; did you mean `Contact`?". The scenario is in the spec and the
  test covers it.

### ✅ Verified - RF2 A state field typed by a type parameter emits an undeclared `TItem` in the executable TypeScript
- **Severity:** High
- **Evidence:** [emit.rs:1606-1630](crates/nx-codegen/src/emit.rs#L1606-L1630)
  (`emit_component_state_type`) renders each state field with the unerased declared `TypeRef` via
  `emit_type_ref`, while `<Name>State` declares no generic parameters — unlike `Props`/
  `ResolvedProps`, which got `component_generics(...)`. For
  `component <List TItem:type items:TItem[]? /> = { state { sel:TItem? = null } <Label /> }` the
  emitter produces `export type ListState = { readonly sel: TItem | null; };` and
  `assert_generated_typescript_artifact_type_checks` fails with
  `m0_main.ts(44,17): error TS2304: Cannot find name 'TItem'.` This is precisely the program the
  `component-type-parameters` scenario "Type parameter is usable in the component body" and the
  `component-contract-inheritance` scenario "Derived component body sees the inherited type
  parameter" both sanction, so a spec-blessed source produces non-compiling output.
- **Recommendation:** Decide the rule for state — D6 names three erasing surfaces and two generic
  ones but is silent on state — and apply it in `emit_component_state_type` and the
  `initial<Name>State` / `render<Name>` signatures that consume `<Name>State`. Erasing to `unknown`
  is the smaller change and matches `Element` (state is host-owned snapshot data, not something a
  TypeScript caller names). Add an `external-components`/`nx-ir-format` scenario pinning it.
- **Fix:** Decided the rule: state and everything derived from it erases in every target (D6 now says so).
  `emit_component_state_type` renders each field through the erasing helper, renamed
  `emit_erased_field_type` since it now serves both `Element` and `State`; the `initial<Name>State`
  and `render<Name>` signatures needed no change because `<Name>State` stays non-generic. New
  `external-components` scenario "Generic component erases the parameter on State" and codegen test
  `generated_typescript_erases_a_type_parameter_on_the_state_type`, which runs `tsc --strict`.
- **Verification:** Confirmed. `emit_erased_field_type`
  ([emit.rs](crates/nx-codegen/src/emit.rs)) is shared by the `Element` and `State` emissions, and
  `ListState` now renders `readonly sel: unknown | null`. The generated module passes
  `tsc --strict`, where it previously failed with `TS2304: Cannot find name 'TItem'`. Also verified
  the inherited case the finding did not ask for — `abstract component <Base TItem:type ... />` with
  `component <List extends Base /> = { state { sel:TItem? = null } ... }` — which works because
  `build_component` erases against the *effective* contract's parameters rather than the declared
  ones.

### ✅ Verified - RF3 An emitted-action payload typed by a type parameter aborts code generation with an internal-shaped error
- **Severity:** Medium
- **Evidence:** `build_component` erases parameters only in the prop and state field lists
  ([builder.rs:521-558](crates/nx-codegen/src/builder.rs#L521-L558)); the inline records that
  `emits { ... }` lowers to are ordinary `Item::Record`s and are never told about the owning
  component's parameters. `component <List TItem:type items:TItem[]? emits { pick { item:TItem } } /> = { <Label /> }`
  type-checks with zero diagnostics, then `emit_program` fails the whole program with
  `codegen-missing-semantic-data`: *"Cannot build codegen program for 'main.nx' because type binding
  'TItem' is unavailable"* — an internal-sounding error with a zero-width span at offset 0, for a
  program the checker accepted. No spec says whether a type parameter is legal in an emit payload.
- **Recommendation:** Pick one and write a scenario for it. Either erase the parameter in the emit
  payload record the way props are erased, or reject a type-parameter reference in an
  `emits` payload annotation at the checker with a targeted diagnostic. Leaving the checker
  permissive and codegen fatal is the one outcome to avoid.
- **Fix:** Chose rejection. Lowering (`predeclare_component` in `lower.rs`) now reports a payload field of
  an inline emit whose annotation names one of the component's type parameters: "Emitted action
  'pick' on component 'List' cannot type its payload field 'item' by type parameter 'TItem'; a type
  parameter is a type only in the component's props, state, and body". Rejection rather than erasure
  because the checker resolves the payload record outside the component's type scope, so erasing at
  codegen would still leave the payload unusable in the body; relaxing this later is additive. The
  `component-type-parameters` declaration requirement, a new scenario, the proposal, D6, and the docs
  section record it; test `a_type_parameter_in_an_emitted_action_payload_is_rejected`. The same
  root cause also hit a *referenced* `T.Update` record of a generic stateful component
  (`let u = <List.Update sel=null />` failed codegen identically); the builder now erases an update
  record's fields against its target component's effective parameters (`update_record_type_params`
  in `builder.rs`, applied in both `build_declaration` and `record_literal_shape`), pinned by
  `an_update_record_of_a_generic_component_erases_the_parameter` (tsc-checked, plus an
  `nx-ir-format` scenario "Update record carries the erased type").
- **Verification:** Reopened — the fix is correct but incomplete. Both halves work for a
  component's **own** type parameters: `emits { pick { item:TItem } }` on
  `component <List TItem:type ... />` is now rejected at lowering with the intended message, a
  payload field typed by anything else is unaffected, and the referenced-`T.Update` half is solid
  (verified with `let u = <List.Update sel=null />` under `tsc --strict`, for both an own and an
  inherited parameter). But the lowering check compares against `lower_type_parameters(signature)` —
  the component's *declared* list — and inherited parameters do not exist at lowering. So:

  ```nx
  abstract component <Base TItem:type items:TItem[]? />
  component <List extends Base emits { pick { item:TItem } } /> = { <Label /> }
  ```

  type-checks with **zero diagnostics** and then fails code generation with the same original error,
  `codegen-missing-semantic-data`: *"Cannot build codegen program for 'main.nx' because type binding
  'TItem' is unavailable"*, zero-width span at offset 0. The requirement this fix added says a
  payload field typed by a type parameter "SHALL be rejected" without qualifying it to declared
  parameters, and `component-contract-inheritance` makes an inherited parameter a type in the derived
  signature, so this is the change's own spec going unenforced.
- **Recommendation:** Move the check to where the effective contract is known — `components.rs`
  resolves the base chain and already raises `ComponentResolutionError` variants for exactly this
  class of problem — or run a second pass over inline emit payloads once the effective contract
  resolves. Add an inheritance scenario beside "A type parameter in an emitted action payload is
  rejected".
- **Fix (second pass):** Moved the check into contract resolution. `components.rs` gains
  `ComponentResolutionError::TypeParameterInEmitPayload` (code
  `component-type-parameter-in-emit-payload`, same message as before) raised by
  `check_emit_payloads_for_type_parameters`, which runs in both branches of
  `resolve_component_contract_inner` against the *effective* parameter list — inherited first, then
  declared — and reads each inline emit's action record from the component's own module. The lowering
  check was removed so the own-parameter case reports once. The reopened program
  (`abstract component <Base TItem:type ... /> component <List extends Base emits { pick { item:TItem } } />`)
  now fails type checking with the intended diagnostic instead of reaching codegen; the test
  `a_type_parameter_in_an_emitted_action_payload_is_rejected` covers the declared case (exactly one
  error), the inherited case, and the unaffected `index:int` payload. New scenario "An inherited type
  parameter in an emitted action payload is rejected"; D6 now says contract resolution, not lowering,
  reports it.
- **Verification (second pass):** Confirmed; the reopened gap is closed and the move was the right
  one. `check_emit_payloads_for_type_parameters`
  ([components.rs:1459](crates/nx-hir/src/components.rs#L1459)) runs from both branches of
  `resolve_component_contract_inner` (lines 1375 and 1433) against the effective `type_params`, and
  the lowering check is gone, so nothing reports twice. Behavior checked case by case:
  - The reopened program — `abstract component <Base TItem:type ... />` with
    `component <List extends Base emits { pick { item:TItem } } />` — now fails type checking with
    exactly one diagnostic naming `pick`, `item`, and `TItem`, and never reaches code generation.
  - The declared-parameter case still reports exactly one error, and a grandparent chain
    (`C extends B extends A`) with a suffix-wrapped payload (`item:TItem[]?`) is caught too, so the
    check reads through the whole effective list and through `[]`/`?` layers.
  - A base declared in **another module** is caught as well: a two-file workspace whose `app.nx`
    derives from an imported `ItemsBase` reports the diagnostic once, against the module that wrote
    the offending emit. The `module_identity` guard correctly scopes it there rather than
    re-reporting at every importer.
  - No over-reach: a payload typed `index:int` on a generic component, a component with no type
    parameters at all, and a base whose own emit offends while two derived components resolve
    against it (reported once, against the base) all behave correctly.

  Both spec scenarios are present, the test covers the declared case with an exact count, the
  inherited case, and the unaffected payload. The referenced-`T.Update` half verified in the first
  pass is unaffected.

### ✅ Verified - RF4 The generated update-record contract carries a dangling `TItem` in both C# and TypeScript
- **Severity:** High
- **Evidence:** `export_record` hardcodes `type_params: Vec::new()`
  ([model.rs:1563](crates/nx-cli/src/typegen/model.rs#L1563)), so the `T.Update` record derived from
  a generic component's state never learns the owning component's parameters, and neither emitter
  erases or declares them. For
  `export component <List TItem:type items:TItem[]? /> = { state { sel:TItem? = null } <Label /> }`
  typegen emits, in C#, `new NxField("sel", typeof(TItem))` and `public NxOptional<TItem?> Sel`;
  in TypeScript, `export interface List_update { $type: "List.Update"; sel?: TItem | null; }`.
  Neither compiles. `cli-code-generation`'s new requirement covers only the external-component
  contract, so this surface was never considered. Same root cause as RF2.
- **Recommendation:** Carry the owning component's effective type parameters onto the exported
  update record (and the property-union target it already tracks), then erase them in C# with the
  existing `erase_type_parameters` call in
  [csharp.rs:658-681](crates/nx-cli/src/typegen/languages/csharp.rs#L658-L681) and decide the
  TypeScript form alongside RF2's decision. Extend the `cli-code-generation` requirement to name the
  update record.
- **Fix:** `ExportedUpdate` and `ExportedExternalState` now carry `type_params` (the target component's
  effective list, via the shared `effective_component_type_params`), and a new
  `erase_field_type_parameters` helper in `model.rs` is applied by the C# `emit_record`,
  `emit_update`, and `emit_external_state` and by the TypeScript `emit_update` and
  `emit_external_state`. Both languages erase (C# `object`, TypeScript `unknown`) with no generic
  parameter, per the RF2 decision. The `cli-code-generation` requirement names the update companion
  and state record, with scenario "State and update companion erase the parameter" and test
  `a_generic_component_state_and_update_companion_erase_the_parameter`.
- **Verification:** Confirmed. For
  `export component <List TItem:type items:TItem[]? /> = { state { sel:TItem? = null } <Label /> }`
  neither the C# nor the TypeScript output contains `TItem` anywhere; the C# update companion that
  previously emitted `new NxField("sel", typeof(TItem))` and `NxOptional<TItem?> Sel`, and the
  TypeScript `interface List_update { sel?: TItem | null }`, both now carry the erased type. The
  `cli-code-generation` requirement names the update companion and state record.

### ✅ Verified - RF5 A derived contract whose abstract base lives in another module loses the inherited type parameters
- **Severity:** High
- **Evidence:** `effective_component_type_params`
  ([model.rs:1649-1673](crates/nx-cli/src/typegen/model.rs#L1649-L1673)) walks the base chain with
  `module.find_item(base)`, i.e. within one `LoweredModule`; its own doc comment records that "a base
  declared elsewhere contributes nothing here". But the TypeScript emitter's base reference *does*
  resolve across modules through `graph.resolved_record_base`
  ([typescript.rs:380-390](crates/nx-cli/src/typegen/languages/typescript.rs#L380-L390)), so the two
  halves disagree. A two-file library — `base.nx`:
  `export abstract external component <ItemsBase TItem:type items:TItem[]? />`, `derived.nx`:
  `export external component <ContactList extends ItemsBase extra:TItem[]? />` — generates:
  - TypeScript: `export interface ContactList extends ItemsBaseBase<TItem>, NxRecord<"ContactList"> { extra: TItem[] | null; }`
    — `TItem` is undeclared on the interface *and* passed as a generic argument that does not exist.
  - C#: `public TItem[]? Extra { get; set; }` — unerased, while the base's own `items` correctly
    erased to `object[]?` in `base.g.cs`.

  Neither output compiles. `component-contract-inheritance` requires the effective contract to
  include the whole base chain's parameters, and `nx_hir::effective_component_contract` already
  computes that across modules — the typegen model just re-derives it with a weaker walk. The
  change's own typegen test only covers a same-file base, which is why this passed.
- **Recommendation:** Source the parameter list from the resolved effective contract (or from the
  already-built `ExportedTypeGraph`, which resolves bases across modules) instead of re-walking one
  lowered module. Add a cross-module library typegen test alongside
  `a_derived_component_contract_carries_inherited_type_parameters`.
- **Fix:** `effective_component_type_params` now takes the prepared module and reads
  `nx_hir::effective_component_contract(prepared, component).type_params` when it is available,
  which resolves the base chain across modules the way the checker and the emitted base reference
  do; the single-module walk remains only as the fallback when there is no prepared module or the
  chain does not resolve. The same list now feeds the contract, the update companion, and the state
  record. Scenario "Derived contract carries a parameter inherited across modules" and test
  `a_derived_contract_carries_type_parameters_inherited_across_modules` (two-file library, both
  languages, no warnings).
- **Verification:** Confirmed against the exact two-file library from the finding. TypeScript now
  emits `export interface ContactList<TItem = unknown> extends ItemsBaseBase<TItem>, NxRecord<"ContactList">`
  — the generic argument passed to the base finally has a declaration to bind to — and C# emits
  `public object[]? Extra { get; set; }` instead of the unerased `TItem[]?`. Both outputs compile.
  Reading the effective contract from the prepared module is the right source: it is what the
  checker and the emitted base reference already agree on.

### ✅ Verified - RF6 The Migration Plan's "no generated output changes for a program without type parameters" is no longer true
- **Severity:** Low
- **Evidence:** [emit.rs:2223-2237](crates/nx-codegen/src/emit.rs#L2223-L2237) now emits
  `props.x ?? null` instead of `props.x` for *every* optional nullable prop with no default, whether
  or not the component is generic. D6 documents and justifies this as a latent-defect fix, but
  `design.md`'s Migration Plan still asserts the opposite, and the proposal's "What Changes" does not
  mention it. The observable effect for a non-generic component: a caller passing `{ x: undefined }`
  now gets `x: null` on the element rather than `undefined`, which `JSON.stringify` keeps rather than
  drops — a wire-shape change for that input. No test pins the new behavior for a *non*-generic
  component.
- **Recommendation:** Correct the Migration Plan sentence (and add the resolver fix to the proposal's
  "What Changes"), and add one codegen test asserting the resolved value for a key present with
  `undefined` on an ordinary nullable prop, so the fix is pinned where it actually applies.
- **Fix:** Rewrote the Migration Plan paragraph to state the resolver change and its observable effect,
  added the resolver fix to the proposal's "What Changes", and added codegen test
  `a_nullable_prop_present_with_no_value_resolves_to_null` on a non-generic
  `external component <Box label:string? />`, asserting `Box({ label: undefined })` and `Box({})`
  both serialize `label` as `null`.
- **Verification:** Confirmed. `design.md`'s Migration Plan now states the resolver change and its
  observable effect (a key present with `undefined` resolves to `null`, so `JSON.stringify` keeps it),
  and the proposal's "What Changes" carries it as its own bullet. Test
  `a_nullable_prop_present_with_no_value_resolves_to_null`
  ([tests.rs:3457](crates/nx-codegen/src/tests.rs#L3457)) executes the generated JavaScript for a
  non-generic `external component <Box label:string? />` and asserts both `Box({ label: undefined })`
  and `Box({})` serialize `label` as `null`.

## Questions

- Is a type parameter intended to be legal in a `state` annotation and in an `emits { ... }` payload?
  The `component-type-parameters` and `component-contract-inheritance` specs both use a state field
  in a scenario, so state appears intended — but no capability states the erasure rule for it, and
  emit payloads were never considered. RF2, RF3, and RF4 all turn on this answer.
  - **Decision taken by the fix:** state yes, erased everywhere; emit payloads no, rejected at
    lowering. Recorded in D6, the proposal, and the specs. If the author would rather allow emit
    payloads, the checker must first resolve the payload record inside the component's type scope;
    the lowering diagnostic is the only thing to remove afterwards.
- For a surface that *can* carry a parameter but is not caller-named (the `<Name>State` type, the
  `T.Update` contract), does D6's line — "generics where the host names the instantiation, erasure
  where it receives one" — put state on the erasure side? The review assumes yes, but this is the
  change author's call.
  - **Decision taken by the fix:** yes, erasure, in C# and TypeScript alike. The executable
    `T.Update` record renders the erased `object` as TypeScript `object` because that is how the
    executable emitter already renders an NX `object` annotation; only the typegen contracts and the
    executable `Element`/`State` types spell it `unknown`.
- Should a type parameter be allowed to shadow a primitive name outright? `external component <List string:type items:string[]? />`
  parses, checks, and accepts `<List string=int items={1} />`. It follows from "shadows any other
  type of the same name", but it is not covered by a scenario and the naming convention in the docs
  quietly assumes `T`-prefixed names.
  - **Status:** not addressed by the fix pass; it is a language-design call with no defect behind
    it. If shadowing a primitive should be rejected, validation is the place (beside the
    leading-position rule), with a scenario under the declaration requirement.
  - **Decided (author): rejected.** `validation.rs` now refuses a type parameter whose name is in
    `nx_syntax::PRIMITIVE_TYPE_NAMES` — the one list `nx-types` now re-exports — with "Type
    parameter 'string' cannot take the name of a primitive type" and the suggestion to rename it.
  - **Verified (pass 3).** See RF7/RF8 below: the rule itself works, with two low-severity
    follow-ups on its edges.
    Scenario "Type parameter named after a primitive is rejected" added under the declaration
    requirement; parser test covers all eight names and a declared-type name that stays allowed;
    checker test confirms the diagnostic reaches `check_str`. Docs and the design risks record it.

## Summary

The core of this change is in good shape. The separation of `type_params` from `props` (D1) does what
D1 promised — every value-level consumer excludes them by construction — the checker's use-site path
is genuinely shaped for later inference, `remove_property_entries` lands beside the existing
checker-only rewrites, and the parser/validation split (D2) is well covered, including the
prop-named-`type` regression the design flagged as a risk. Scenario coverage in
`crates/nx-types/tests/component_type_parameters.rs` is one test per scenario and reads cleanly;
the language-service smoke test, the interpreter test, and the tsc-checked codegen test all do real
work.

The gap is consistent and has one shape: **erasure was specified and implemented for props, but a
type parameter is also legal in `state` and in `emits`, and those flow into surfaces nobody erased.**
RF2, RF3, and RF4 are three instances of that; RF5 is the same omission reached a different way
(an inherited parameter the typegen model cannot see across a module boundary). Four of the six
findings produce output that does not compile in the target language, from source the specs
explicitly bless, so this should not archive as-is. RF1 is a small, self-contained resolver bug with
a visibly contradictory diagnostic. RF6 is documentation only.

Six findings opened, five of them `Open` defects and one a documentation correction; none are
design-level objections to the approach.

## Fix pass

All six findings were fixed in one pass; none were skipped. `cargo test --workspace` is green
after the fixes (three new codegen tests, two new checker assertions, two new typegen tests), and
`cargo fmt --all --check` is clean. `cargo clippy` still fails only on the pre-existing
`approx_constant` and bench errors the review already attributed to the base tree; the new code
adds no warnings. The docs change is one bullet in the existing "Type parameters" section and was
not rebuilt.

## Verification pass — 2026-09-13

Five of six findings verified fixed; **RF3 reopened**.

Each fix was checked behaviorally against the exact reproduction from the original finding, with
throwaway tests that were removed afterwards — the tree is byte-identical to how the fix pass left
it (32 files, 9613 insertions). `cargo test --workspace` is green (60 suites, 0 failures) and
`cargo fmt --all --check` is clean.

| Finding | Result |
| --- | --- |
| RF1 | ✅ Verified — `TItem=object` accepted; the two name lists now share one constant |
| RF2 | ✅ Verified — `ListState` erases to `unknown`; passes `tsc --strict`, inherited case too |
| RF3 | 🔴 **Reopened** — inherited type parameters escape the new lowering check |
| RF4 | ✅ Verified — no `TItem` anywhere in the C# or TypeScript update companion |
| RF5 | ✅ Verified — `ContactList<TItem = unknown>` in TS, `object[]?` in C#; both compile |
| RF6 | ✅ Verified — Migration Plan and proposal corrected; test pins the non-generic case |

The three erasure findings (RF2, RF4, RF5) and the resolver documentation (RF6) are genuinely
closed, and the decisions the fix pass recorded in D6 and the specs — state erases everywhere, emit
payloads are rejected — are coherent and well documented. RF1's fix removes the drift at its source
rather than patching the symptom.

RF3 is the one to finish. Its chosen approach (reject rather than erase) is sound and the message
is good, but the check sits at lowering, which only knows a component's *declared* type parameters.
A derived component whose emit payload names an **inherited** parameter still type-checks clean and
still aborts code generation with the original `codegen-missing-semantic-data` error — the exact
"checker permissive, codegen fatal" outcome the finding asked to avoid, and a gap against the
requirement the fix itself added. The referenced-`T.Update` half of RF3, added during the fix pass,
is solid and was verified for both own and inherited parameters.

## Fix pass 2 — 2026-09-13

RF3 reopened item addressed: the emit-payload check now runs at contract resolution over the
effective type parameters, so an inherited parameter is caught exactly like a declared one.
`cargo test --workspace` is green and `cargo fmt --all --check` is clean after the change.


## Verification pass 2 — 2026-09-13

**All six findings are now verified fixed. No findings remain open, and no new findings were
raised in this pass.**

RF3 was the only item outstanding. Moving the check from lowering to contract resolution is the
right fix rather than a patch: lowering structurally cannot know an inherited parameter, while
`resolve_component_contract_inner` already has the effective list and already raises this class of
error. The reopened reproduction now fails type checking with one clear diagnostic instead of
aborting code generation, and the check holds up on the cases the finding did not name — a
grandparent chain, a suffix-wrapped payload type, a base imported from another module, and a base
whose own emit offends while several derived components resolve against it (reported once, against
the base). Nothing over-reports and nothing over-reaches.

Verification was behavioral, one probe per case, with throwaway tests removed afterwards; the tree
is byte-identical to how fix pass 2 left it (32 files, 9672 insertions). `cargo test --workspace`
is green (60 suites, 0 failures) and `cargo fmt --all --check` is clean. `cargo clippy` still
carries only the pre-existing base-tree errors this review identified at the outset.

| Finding | Result |
| --- | --- |
| RF1 | ✅ Verified (pass 1) |
| RF2 | ✅ Verified (pass 1) |
| RF3 | ✅ Verified (pass 2) — inherited, cross-module, and deep-chain cases all caught |
| RF4 | ✅ Verified (pass 1) |
| RF5 | ✅ Verified (pass 1) |
| RF6 | ✅ Verified (pass 1) |

From the reviewer's side this change is ready to archive. The one item left on the table is not a
defect: whether a type parameter should be allowed to shadow a primitive name
(`<List string:type ... />`) remains an open language-design question recorded under Questions.

## New Findings Discovered During 2026-09-13 20:30 Verification

Pass 3 reviewed the unstaged layer on top of the staged change: the decision to reject a type
parameter named after a primitive, and the move of `PRIMITIVE_TYPE_NAMES` from `nx-types` into
`nx-syntax` so validation and the checker share it.

**The rule itself is correct and well covered.** All eight primitive names are refused with one
diagnostic each; a declared type's name (`Contact:type`) is still accepted; a prop literally named
`type` (`component <X type:string />`) is unaffected; and a primitive-named definition outside a
component signature still reports only the position message ("not supported in a record"), because
the new check sits after that early return. The requirement text, a new scenario, both tests, and
the docs bullet all match the implemented behavior. Two low-severity follow-ups on its edges:

### ✅ Verified - RF7 `Element` can still be taken as a type parameter name, though it is a built-in the author cannot declare
- **Severity:** Low
- **Evidence:** The new check tests only `PRIMITIVE_TYPE_NAMES`
  ([validation.rs:463-471](crates/nx-syntax/src/validation.rs#L463-L471)), so
  `component <List Element:type slot:Element? /> = { <Label /> }` is accepted, and `slot:Element?`
  resolves to the parameter rather than the element type — `<List slot={<Label />} />` reports
  "Property 'slot' on 'List' is typed by 'Element', which was not specified; add Element=<type>".
  The behavior is coherent, so this is a consistency gap rather than a defect. But the rationale the
  fix records — "a parameter shadows a same-named declared type inside its component, which is
  harmless for a record or alias but would make a primitive mean two things in one file" — applies
  to `Element` exactly as it does to `object`: the language service classifies it as a built-in
  "valid in type position but not a primitive"
  ([lib.rs:50-51](crates/nx-language-service/src/lib.rs#L50-L51)), and an author who takes the name
  loses the only way to name the element type in that component's signature, with no declaration to
  point at.
- **Recommendation:** Either add `Element` to the refused set (a `RESERVED_TYPE_NAMES` beside
  `PRIMITIVE_TYPE_NAMES`, since the two are used for different things elsewhere), or state in the
  requirement that the restriction is deliberately limited to the eight primitives so the omission
  reads as a decision. Whichever way, a line in the docs bullet keeps it honest.
- **Fix:** Added `nx_syntax::BUILTIN_TYPE_NAMES` (`["Element"]`) beside `PRIMITIVE_TYPE_NAMES`;
  validation now refuses a type parameter named after either, with "Type parameter 'Element'
  cannot take the name of the built-in type 'Element'". The language service's
  `BUILTIN_TYPE_COMPLETIONS` reads the same constant. Requirement text, a new scenario, the parser
  test, the docs bullet, and the design risk all name `Element`.

### ✅ Verified - RF8 Two crates still keep verbatim copies of the primitive-name list the new constant claims to own
- **Severity:** Low
- **Evidence:** The new constant's doc comment
  ([lib.rs:18-26](crates/nx-syntax/src/lib.rs#L18-L26)) states: "Every crate that decides whether a
  bare name is a primitive reads this list, so the set has one home." Two crates that already depend
  on `nx-syntax` do not:
  - `PRIMITIVE_TYPE_COMPLETIONS` at
    [lib.rs:46-48](crates/nx-language-service/src/lib.rs#L46-L48) — a character-for-character
    duplicate of the eight names, and the list the `primitive-type-names` capability constrains
    ("`never` SHALL NOT be offered as a completion").
  - `is_primitive_type_name` at
    [model.rs:1724-1729](crates/nx-cli/src/typegen/model.rs#L1724-L1729) — the same eight as a
    `matches!` pattern.

  Both predate this change, so nothing is broken today. What makes it worth raising now is that
  RF1 *was* this exact drift — two copies of this list disagreeing about `object` — and the stated
  point of hoisting the constant was to make that impossible. Leaving two more copies keeps the
  failure mode alive at two sites while the doc comment says it is gone.
- **Recommendation:** Point both at `nx_syntax::PRIMITIVE_TYPE_NAMES` (`PRIMITIVE_TYPE_COMPLETIONS`
  becomes `&PRIMITIVE_TYPE_NAMES`; `is_primitive_type_name` becomes a `contains`), or narrow the doc
  comment to the crates that actually read it. The first is a few lines and closes RF1's root cause
  for good. Note `is_primitive_type_name` at
  [model.rs:1440](crates/nx-cli/src/typegen/model.rs#L1440) is a *different* set (`void`, no
  `string`/`object`) and should be left alone.
- **Fix:** `PRIMITIVE_TYPE_COMPLETIONS` is now `&nx_syntax::PRIMITIVE_TYPE_NAMES` and typegen's
  `is_primitive_type_name` (the eight-name one) is a `contains` on it; the `void` set at the other
  site was left alone as recommended. The language service's own test still pins the literal set,
  which is now a guard on the shared constant. The constant's doc comment lists the four readers,
  and the declaration moved below the import block per the cosmetic note.
- **Verification:** Confirmed. A workspace-wide grep finds no remaining production copy of either
  list: the only surviving literal is the language-service *test* at
  [lib.rs:2513](crates/nx-language-service/src/lib.rs#L2513), which is the intended guard on the
  shared constant, and `&["Element"]` is gone entirely. All four readers the doc comment names are
  real — `validation.rs:466`, `nx-types` via the `semantics.rs` re-export, the language service's
  `PRIMITIVE_TYPE_COMPLETIONS`/`BUILTIN_TYPE_COMPLETIONS`, and typegen's `is_primitive_type_name` —
  so the claim now holds. The `void` set at [model.rs:1440](crates/nx-cli/src/typegen/model.rs#L1440)
  was correctly left alone. No behavior moved: the language service suite (110 tests) and the typegen
  suite (129 tests) both pass unchanged. RF1's root cause is closed at every site.

## Verification pass 3 — 2026-09-13

Scope: the unstaged layer only (`nx-syntax/src/lib.rs`, `validation.rs`, `parser_tests.rs`,
`nx-types/src/semantics.rs`, `tests/component_type_parameters.rs`, and the docs bullet), plus a
regression check on the six findings already closed.

- **RF1–RF6 remain ✅ Verified.** `semantics.rs` changing from an owned const to a re-export does
  not disturb RF1: `<List TItem=object items={ 1 "two" } />` still checks clean, and the RF3
  inherited emit-payload rejection still fires. `cargo test --workspace` is green (60 suites, 0
  failures) and `cargo fmt --all --check` is clean.
- **Two new findings, both Low: RF7 and RF8.** Neither blocks archiving; both are small and
  self-contained. RF8 is the one I would take, because it closes RF1's root cause rather than
  leaving the doc comment writing a cheque the code does not cash.

One cosmetic note, not a finding: the new `PRIMITIVE_TYPE_NAMES` declaration is placed between the
`pub use` block and the `use nx_diagnostics::…` line in `nx-syntax/src/lib.rs`, splitting the import
block in two. Moving it below the imports would keep the file's import section contiguous.

Verification was behavioral, one probe per case, with throwaway tests removed afterwards; the tree
is byte-identical to how the author left it (staged: 33 files; unstaged: 7 files, 140 insertions).

## Fix pass 3 — 2026-09-13

RF7 and RF8 fixed: `Element` is refused as a type parameter name through a new
`BUILTIN_TYPE_NAMES` constant, and the language service and typegen now read the shared
primitive list. `cargo test --workspace` green, `cargo fmt --all --check` clean.


## New Findings Discovered During 2026-09-13 21:15 Verification

### ✅ Verified - RF9 The reason recorded for refusing `Element` (and, implicitly, the primitives) is not true
- **Severity:** Low
- **Evidence:** `BUILTIN_TYPE_NAMES`'s doc comment
  ([lib.rs:41-44](crates/nx-syntax/src/lib.rs#L41-L44)) justifies the rule with: "An author cannot
  declare one, so a type parameter cannot take the name either: there would be no other way to name
  the built-in inside that component." The first clause is false, and the second is true of a
  module-level declaration too. All three of these parse and validate with **zero** diagnostics:

  ```nx
  type Element = { id:int }
  type string  = { id:int }
  type object  = { id:int }
  ```

  and the declaration wins at the use site — `type Element = { id:int }` followed by
  `let x:Element = <Element id=1 />` type-checks clean, so the author has shadowed the built-in and
  has no way left to name it in that module. That is exactly the situation the rule calls
  unacceptable for a type parameter. The same reasoning appears in the validation comment
  ([validation.rs:463-465](crates/nx-syntax/src/validation.rs#L463-L465)) — "would make a primitive
  or a built-in mean two things in one file, with no declaration left to name the original by" — and
  in the parser test's inline comment ("`Element` is a built-in the author cannot declare"). It is
  also the same latitude `primitive-type-names` grants explicitly for `never`: "A user declaration
  MAY take the name `never`."

  Nothing is broken — the rejection is defensible on its own terms, and this pass verified it works.
  The problem is that the recorded reason is the thing a future reader will use to decide whether to
  extend the rule (to more built-ins) or relax it, and it will not survive a check.
- **Recommendation:** Keep the behavior, fix the justification. The honest version is roughly: a type
  parameter has no declaration site a reader can consult and its scope silently covers the whole
  component, so it is held to a stricter rule than a module-level `type` declaration, which NX does
  permit for these names. Update the three comments and drop the "cannot declare" clause. If instead
  the asymmetry is unintended, the question is the opposite one — whether module-level declarations
  of primitive and built-in names should be refused too — and that is a larger change than this one.
- **Fix:** Behavior unchanged; the justification was rewritten in all three comments
  (`BUILTIN_TYPE_NAMES` doc, the validation comment, the parser-test comment) and in the design
  risk entry, which no longer says `Element` cannot be declared. The recorded reason is now the
  asymmetry itself: a module-level `type` declaration may take these names because it is a site a
  reader can find, while a type parameter has no such site and its scope silently covers the whole
  component, so it is held to the stricter rule.
- **Verification:** Confirmed, and the new reason checks out. The false clause is gone from all four
  places (`BUILTIN_TYPE_NAMES` doc, the validation comment, the parser-test comment, the design
  risk); a workspace grep for "cannot declare"/"author cannot" finds only unrelated pre-existing
  text. The replacement claim is true as written: `type Element = {...}`, `type string = {...}`,
  `type object = {...}` and `type never = {...}` all validate with zero diagnostics, so NX does let
  a module-level declaration take these names, while a type parameter may not. The design risk now
  cites the `never` precedent from `primitive-type-names` for the same point. Behavior is unchanged:
  the primitive and `Element` rejections still fire with their own messages, and RF1 and RF3 still
  hold.

## Verification pass 4 — 2026-09-13

Scope: the RF7/RF8 fix layer (`nx-syntax/src/lib.rs`, `validation.rs`, `parser_tests.rs`,
`nx-language-service/src/lib.rs`, `nx-cli/src/typegen/model.rs`, the spec, design, and docs), plus a
regression check on everything already closed.

- **RF7 ✅ Verified.** `Element` is refused with its own message; primitives keep the primitive
  message; ordinary and near-miss names (`element`, `ELEMENT`, `Elements`) are unaffected; the
  outside-a-component path still reports only the position message.
- **RF8 ✅ Verified.** No production duplicate of either list survives anywhere in the workspace, all
  four readers named in the doc comment are real, and the language service (110 tests) and typegen
  (129 tests) suites pass unchanged.
- **RF1–RF6 remain ✅ Verified.** Spot-checked the two most exposed to this layer: `TItem=object`
  still checks clean (RF1) and the inherited emit-payload rejection still fires (RF3).
- **One new finding, RF9, Low.** Comment accuracy only; no behavior change requested.

`cargo test --workspace` is green (60 suites, 0 failures) and `cargo fmt --all --check` is clean.

**Eight of nine findings are closed. RF9 is the only one open, it is a three-comment edit, and it
does not block archiving.** Every defect this review found in the implementation is fixed and
verified; what remains is a sentence that explains the last rule incorrectly.

## Fix pass 4 — 2026-09-13

RF9 fixed: comment-only. The three comments and the design risk now give the true reason for
refusing primitive and built-in names on a type parameter. `cargo test --workspace` green,
`cargo fmt --all --check` clean.


## Verification pass 5 — 2026-09-13

Scope: the RF9 comment-only fix (`nx-syntax/src/lib.rs`, `validation.rs`, the parser-test comment,
and the design risk entry), plus a regression check on everything already closed.

- **RF9 ✅ Verified.** The false clause is gone from all four places, and the replacement reason is
  true as written.
- **RF1–RF8 remain ✅ Verified.** Spot-checked the three most exposed to this layer: `TItem=object`
  still checks clean, the inherited emit-payload rejection still fires, and a primitive-named type
  parameter is still refused.

`cargo test --workspace` is green (60 suites, 0 failures) and `cargo fmt --all --check` is clean.

**All nine findings are closed. Nothing is open. From the reviewer's side this change is ready to
archive.**

### Out-of-scope observation, not a finding

While confirming RF9's new claim, one pre-existing defect surfaced that this change neither caused
nor touches. A module-level declaration named after a *true* primitive is accepted but inert, and
using it produces a diagnostic that names the same type twice:

```nx
type string = { id:int }
let x:string = <string id=1 />
// Initializer for value 'x' expects string, found string
```

The annotation resolves to the primitive while the element constructs the record, and both render
as `string`. `object`, `Element`, and `never` do not have this problem — a declaration under those
names shadows normally — because `object` is a `Named` type and only the other seven go through
`builtin_type`. That is the same `Primitive`-vs-`Named` split behind RF1, showing up on a different
surface.

It is unrelated to component type parameters and out of scope here. Written up in
`specs/future.md` under "A Declaration Named After A Primitive Is Constructible But Unnameable",
which records the full characterization — the declaration is constructible through inference but can
never be named, and `primitive-type-names`' "a user declaration MAY take the name `never`, resolved
by the same rules that govern any non-primitive name" holds for `never`, `object`, and `Element` but
not for the seven true primitives.
