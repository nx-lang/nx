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

### 🟡 Fixed - RF1 A state patch produced by `diff` inside a handler is returned as an effect instead of patching state
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

### 🟡 Fixed - RF2 TypeScript typegen emits invalid code for every exported component without props
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

### 🟡 Fixed - RF3 Doc links in state-field docs are checked twice, and links to props are falsely reported unresolved
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

### 🟡 Fixed - RF4 A bare `State` in a component's signature is rewritten, hijacking a top-level `State` for prop types
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

### 🟡 Fixed - RF5 Stale comments and docs still describe a component's own `Update` as its state patch
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

### 🟡 Fixed - RF6 Some spec scenarios and risky paths have no direct test
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
