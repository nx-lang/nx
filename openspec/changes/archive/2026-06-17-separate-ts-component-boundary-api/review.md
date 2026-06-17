# Review: separate-ts-component-boundary-api

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/executable-code-generation/spec.md, specs/external-components/spec.md

**Reviewed code:**
- crates/nx-codegen/src/model.rs
- crates/nx-codegen/src/builder.rs
- crates/nx-codegen/src/emit.rs
- crates/nx-codegen/src/runtime.rs
- crates/nx-codegen/src/lib.rs
- crates/nx-codegen/src/tests.rs
- crates/nx-cli/src/main.rs
- README.md

Verification: `cargo test -p nx-codegen` (35 passed, includes `tsc`/`node` round-trips). Also reproduced
the RF1 defect by generating output for a small program and loading it under Node.

## Findings

### ✅ Verified - RF1 Non-constant record/union field defaults are emitted as bare identifiers in generated schemas, producing code that crashes at module load
- **Severity:** High
- **Evidence:** `emit_schema_field` ([emit.rs](../../../crates/nx-codegen/src/emit.rs)) emits a record/union
  field default as `defaultValue: <emit_expression(default)>`. `emit_expression` produces whatever code the
  default lowers to, including references to *sibling fields of the same record*, which have no binding in the
  schema object-literal scope. Reproduced with:
  ```nx
  type Pair = { a:int b:int = { a } }
  component <Box p:Pair /> = { p }
  ```
  Generated JS contains `... { name: "b", schema: nxNumberSchema, required: false, hasDefault: true, defaultValue: a } ...`
  and importing the module throws `ReferenceError: a is not defined`. The TypeScript target emits the same
  `defaultValue: a`, which fails to type-check. Codegen reports success, so the breakage is silent until load.
  This contradicts design decision 6 ("defaults derived from props or other expressions" should remain in
  generated typed code, with schema metadata describing only structural type) and the `NxFieldSchema.defaultValue?: NxValue`
  contract, which expects a literal JSON value, not arbitrary expression code. Component prop/state defaults are
  unaffected because `emit_component_boundary_schema` deliberately omits `defaultValue` and resolves defaults in
  typed code; only the record/union schema path (`emit_record_schema` → `emit_schema_field`, also reached via
  `emit_union_schema`) embeds the expression.
- **Recommendation:** Only emit `defaultValue` into schema metadata when the default is a literal/constant
  value. For non-constant record/union field defaults, omit the schema default and materialize the value in
  generated typed code (mirroring the component prop/state approach), or generate a helper closure the schema
  can call. Add a regression test for a record/union field default that references a sibling field (and run it
  through both the JS `node` round-trip and the `tsc` type-check harness), since task 4.5's schema tests only
  cover literal defaults.
- **Fix:** Literal defaults still emit as schema `defaultValue`; dynamic record/union defaults now emit
  `defaultFactory` closures bound to prior fields and runtime normalization evaluates and validates those
  defaults. Added a regression covering record and union sibling defaults through generated JS and the
  generated TypeScript type-check harness.
- **Verification:** Confirmed. `emit_schema_field_default` ([emit.rs:1844](../../../crates/nx-codegen/src/emit.rs#L1844))
  emits literal/enum/array defaults as `defaultValue` and routes non-constant defaults to a
  `defaultFactory` closure bound to prior fields (`__nx_record`), and runtime
  `nxNormalizeRecordInput` ([runtime.rs:364](../../../crates/nx-codegen/src/runtime.rs#L364)) evaluates
  and re-validates the factory result against the field schema, with prior fields already present in
  `output`. The regression `generated_record_schema_defaults_can_reference_previous_fields`
  ([tests.rs:956](../../../crates/nx-codegen/src/tests.rs#L956)) asserts no `defaultValue: a` leaks,
  exercises both record and union sibling defaults through the JS `node` round-trip, and type-checks
  the TS. No remaining bare-identifier path.

### ✅ Verified - RF2 `CodegenComponentDescriptor.props` is built per element expression but never consumed
- **Severity:** Medium
- **Evidence:** `build_component_descriptor_expression` ([builder.rs](../../../crates/nx-codegen/src/builder.rs))
  resolves the target component's effective contract and calls `build_effective_component_fields` to materialize
  every prop (including building each prop's default expression, which can recurse cross-module) to populate
  `CodegenComponentDescriptor.props`. Emission never reads that field: every `descriptor.*` access in
  [emit.rs](../../../crates/nx-codegen/src/emit.rs) uses `component`, `target_kind`, `properties`, `content`, or
  `content_field` (the content field name is derived separately from `contract.content_prop()`). The work is
  pure overhead and adds an extra contract-resolution failure path for every component element expression.
- **Recommendation:** Drop the `props` field from `CodegenComponentDescriptor` and the field-building call,
  keeping only the `content_field` lookup. If the props are intended for a future emission path, add a comment
  saying so, since it is otherwise dead.
- **Fix:** Removed `CodegenComponentDescriptor.props` and the per-element `build_effective_component_fields`
  call, keeping contract resolution only for content-field lookup.
- **Verification:** Confirmed. `CodegenComponentDescriptor` ([model.rs:271](../../../crates/nx-codegen/src/model.rs#L271))
  no longer carries a `props` field, and `build_component_descriptor_expression`
  ([builder.rs:1211](../../../crates/nx-codegen/src/builder.rs#L1211)) now resolves the contract only to
  derive `content_field`, with no `build_effective_component_fields` call. The remaining
  `build_effective_component_fields` call ([builder.rs:336](../../../crates/nx-codegen/src/builder.rs#L336))
  is the legitimate declaration-level `build_component` path. Builds and tests pass.

### ✅ Verified - RF3 Generated barrel index does not export the stateful component state helpers the README tells callers to use
- **Severity:** Low
- **Evidence:** `emit_index` ([emit.rs](../../../crates/nx-codegen/src/emit.rs)) re-exports only the component
  function, the `*Schema` value, and the `Props`/`Element`/`State` types. README/migration notes (task 5.3)
  direct typed callers to "use generated state helpers such as `initialSearchBoxState` / `renderSearchBox`",
  but those helpers are only exported from the per-module file, not the index barrel.
- **Recommendation:** Either re-export the state helpers (and the render helper) from the index for stateful
  components, or adjust the README to say callers import them from the component's module file.
- **Fix:** `emit_index` now re-exports generated initial-state and render helpers for stateful component
  entrypoints, with a regression assertion for `initialSearchBoxState` and `renderSearchBox`.
- **Verification:** Confirmed. `emits_typed_component_functions_state_helpers_and_schema_boundaries`
  ([tests.rs:705](../../../crates/nx-codegen/src/tests.rs#L705)) asserts the index emits
  `export { SearchBox, SearchBoxSchema, initialSearchBoxState, renderSearchBox }`, so the helpers the
  README/migration notes reference are now reachable from the barrel. Test passes.

### ✅ Verified - RF4 Redundant `target_kind` match in `emit_component_descriptor`
- **Severity:** Low
- **Evidence:** `emit_component_descriptor` ([emit.rs](../../../crates/nx-codegen/src/emit.rs)) matches
  `descriptor.target_kind` with `Normal | External` arms that produce identical output (`{name}({ ... })`). The
  match adds no behavior; both normal render functions and external factories are called the same way.
- **Recommendation:** Remove the match and emit the call directly, leaving a comment that the call form is
  intentionally identical for both kinds.
- **Fix:** Removed the redundant match and left a short comment explaining that normal component render
  functions and external factories intentionally share the same unsuffixed call shape.
- **Verification:** Confirmed. `emit_component_descriptor` ([emit.rs:2208](../../../crates/nx-codegen/src/emit.rs#L2208))
  now emits the call directly (`{}({{ {} }})`) with a comment noting the shared call shape and that
  `target_kind` is retained for return-type/import analysis. The remaining `match descriptor.target_kind`
  ([emit.rs:1385](../../../crates/nx-codegen/src/emit.rs#L1385)) is that separate analysis path, not the
  removed call-shape match. Tests pass.

### ✅ Verified - RF5 External component `element` schema reuses props field metadata, so element JSON validation is looser than the `Element` type
- **Severity:** Low
- **Evidence:** `nxExternalComponentSchema` ([runtime.rs](../../../crates/nx-codegen/src/runtime.rs)) sets
  `element: nxNamedRecordSchema(config.name, config.props.fields)`, reusing the *props* field list where
  defaulted fields are `required: false`. The generated `*Element` TypeScript type marks all fields `readonly`
  and required. So validating an element value (e.g. via a cross-module `{Schema}.element` reference produced by
  `emit_named_type_schema`) would accept an element missing a defaulted field even though a real element always
  has it.
- **Recommendation:** Either derive the element schema from normalized (all-required) field metadata, or
  document that `element` is intentionally lenient. Low impact today because nothing validates element JSON
  on the inbound path.
- **Fix:** External component `element` schemas now derive all-required normalized field metadata, and same-module
  inline external element schemas apply the same strictness. Added a regression proving an inbound element value
  missing a defaulted field is rejected.
- **Verification:** Confirmed. Runtime `nxElementRecordSchema` ([runtime.rs:164](../../../crates/nx-codegen/src/runtime.rs#L164))
  maps props fields to `required: true`, and the same-module inline path passes `require_all_fields`
  through `emit_named_type_schema` → `emit_component_schema` ([emit.rs:1689](../../../crates/nx-codegen/src/emit.rs#L1689))
  for concrete external components. The schema-boundary regression
  ([tests.rs:917](../../../crates/nx-codegen/src/tests.rs#L917)) feeds `ElementHostSchema` a `TextInput`
  element missing the defaulted `value` field and asserts `ok: false` with a `missing-field` diagnostic.
  Test passes.

### ✅ Verified - RF6 Component metadata retained but unused (`base`, `ancestors`, `emits`)
- **Severity:** Low
- **Evidence:** `CodegenComponent.base`, `.ancestors`, and `.emits` ([model.rs](../../../crates/nx-codegen/src/model.rs))
  are populated in `build_component` but never read by the emitter. `emits` in particular is described in tasks
  3.5 as retained "so codegen can reject handlers explicitly," but handler rejection actually happens in
  `build_expression` for `ast::Expr::ActionHandler`, independent of this field.
- **Recommendation:** Remove the unused fields (and the associated `CodegenComponentEmit` plumbing) or add a
  comment explaining the intended future use, to avoid implying behavior that does not exist.
- **Fix:** Removed the unused `base`, `ancestors`, and `emits` component metadata, along with the
  `CodegenComponentEmit` struct, builder population, and public re-export.
- **Verification:** Confirmed. `CodegenComponent` ([model.rs:140](../../../crates/nx-codegen/src/model.rs#L140))
  now holds only `is_abstract`, `is_external`, `props`, `state`, and `body`. `CodegenComponentEmit` is
  gone from the codebase, and lib.rs ([lib.rs:16](../../../crates/nx-codegen/src/lib.rs#L16)) no longer
  re-exports it. The `base:` references that remain are unrelated `CodegenExpression` variants. The crate
  builds and all tests pass, so no reader depended on the removed metadata.

## Questions
- For RF1, should record/union field defaults ever be expressible as non-literals in NX source? If the language
  guarantees they are constant-foldable, the fix can be a constant-folding assertion plus a literal emit;
  otherwise generated typed materialization is required.

## Summary
Implementation closely follows the design: typed component functions, `Props`/`Element`/`State`/`Schema`
naming with collision handling, render-vs-element-by-kind expression emission, and schema-backed JSON
boundaries are all present and well tested (including `tsc`/`node` round-trips). All 25 tasks are checked and
`cargo test -p nx-codegen` is green. The one substantive defect is RF1: record/union field defaults that are
not literals are baked into schema metadata as raw expressions and crash the generated module at load (or fail
`tsc`), with no test covering that path. RF2 is a worthwhile cleanup of dead per-expression work; the remaining
findings are low-severity polish and doc/consistency items.

## Fix Status
All six findings are addressed and marked fixed pending independent verification. Fix pass:
`cargo test -p nx-codegen` (36 passed, including generated JS and TypeScript round-trips available locally).

## Verification Status (2026-06-17)
All six findings independently verified as ✅ Verified. Each fix was confirmed at the code level
(emit/runtime/model/builder/lib) and backed by passing regressions. Verification run:
`cargo test -p nx-codegen` (36 passed; 0 failed), including the new
`generated_record_schema_defaults_can_reference_previous_fields`, the element-strictness assertion in
`generated_schema_boundaries_validate_missing_unknown_defaults_and_state_json`, and the index-export
assertion in `emits_typed_component_functions_state_helpers_and_schema_boundaries`. No new findings.
