# Review: fix-directory-loaded-library-ir-semantics

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/library-registry/spec.md, specs/nx-ir-format/spec.md, specs/sdk-node/spec.md

**Reviewed code:** working-tree diff —
- `crates/nx-types/src/check.rs` (prepared-binding capture)
- `crates/nx-api/src/artifacts.rs` (artifact field init)
- `crates/nx-codegen/src/builder.rs` (codegen type-ref resolution)
- `crates/nx-codegen/src/tests.rs` (Rust regression)
- `bindings/node/native/src/lib.rs`, `bindings/node/src/types.ts` (string fingerprint)
- `bindings/node/test/sdk-node.test.ts`, `bindings/node/README.md`

## Findings

### ✅ Verified - RF1 Undocumented `Element` → `object` special-case shadows real declarations
- **Severity:** Medium
- **Evidence:** [builder.rs:1583-1587](../../../crates/nx-codegen/src/builder.rs#L1583-L1587) hardcodes
  `if name.as_str() == "Element" { return Primitive { name: "object" } }` ahead of binding
  resolution. This is not mentioned anywhere in `proposal.md`, `design.md`, or `tasks.md`, and is not
  exercised by any test added in this change (the fixture only uses `FlowStep`/`QuestionFlow`/
  `ChatLinkConfig` records). Any user- or library-declared type literally named `Element` will be
  silently emitted as the `object` primitive instead of a module-qualified nominal reference, losing
  its type identity in the IR. This directly contradicts the change's own goal of preserving nominal
  references for loaded-library declarations.
- **Recommendation:** Resolve `Element` through the normal binding path (a real builtin should be
  modeled in `is_builtin_type_name`/the type system, not name-matched in codegen). If `Element` is a
  compiler builtin with no source declaration, document the decision in `design.md`, give it a
  dedicated `CodegenTypeRef` variant rather than collapsing to `object`, and add a test. At minimum,
  scope-gate the match so it cannot shadow a declared `Element`.
- **Fix:** Moved the compiler `Element` fallback behind normal type/element binding lookup, added a
  Rust regression proving an imported `Element` record remains nominal, and documented the unbound
  `Element` fallback in `design.md`.
- **Verification:** Confirmed. The `object` fallback now only triggers when no `Type`/`Element`
  binding resolves ([builder.rs:1723-1735](../../../crates/nx-codegen/src/builder.rs#L1723-L1735)),
  so a declared `Element` resolves first. `design.md:98-109` records the decision and the rejected
  alternative. `nx_ir_declared_element_type_is_not_shadowed_by_builtin_element_supertype` proves an
  imported `Element` record stays nominal (module-qualified), and the test passes in
  `cargo test -p nx-codegen --lib` (57 passed).

### ✅ Verified - RF2 Type-namespace lookup silently falls back to the Element namespace, untested
- **Severity:** Low
- **Evidence:** [builder.rs:1589-1591](../../../crates/nx-codegen/src/builder.rs#L1589-L1591) changes
  type resolution to `resolve_binding(Type, name).or_else(|| resolve_binding(Element, name))`, and
  `collect_prepared_bindings` now also harvests `PreparedNamespace::Element` bindings
  ([check.rs:204-249](../../../crates/nx-types/src/check.rs#L204-L249)). This broadens how a type
  name resolves, but no added test covers an element-namespace type reference, so the behavior is
  unverified. Combined with RF1, the `Element`-related code is the least-tested part of the change.
- **Recommendation:** Add a regression that exercises an element/component type reference through the
  loaded-library path, or document why the fallback is safe and remove it if it is not needed for the
  shipped fixture.
- **Fix:** Extended the Rust and Node directory-loaded fixtures with a loaded-library `TextInput`
  component referenced as a field type, asserting that IR emits a module-qualified component nominal
  reference.
- **Verification:** Confirmed. Both `nx_ir_preserves_directory_loaded_cross_library_type_refs`
  (Rust) and the Node "directory-loaded cross-library type graphs" test now use `input:TextInput`
  where `TextInput` is an `external component` in a transitively loaded `ui` library, and assert the
  field type resolves to `kind: "nominal"`, `reference.kind: "component"`, name `TextInput`,
  module-qualified to the `ui` module — which exercises the `Element`-namespace fallback path. Both
  suites pass (Rust 57/57; Node 10/10).

### ✅ Verified - RF3 Reconstructed peer bindings are inserted after authoritative analyzed bindings
- **Severity:** Low
- **Evidence:** In [builder.rs:1716-1768](../../../crates/nx-codegen/src/builder.rs#L1716-L1768),
  `prepared_module_for` first inserts the authoritative `module_artifact.prepared_bindings`
  (lines 1733-1737) and then runs the visible-import reconstruction loop (lines 1743-1768).
  `PreparedModule::insert_binding` is last-write-wins per `(namespace, visible_name)`
  ([prepared.rs:372-376](../../../crates/nx-hir/src/prepared.rs#L372-L376)), so reconstructed peer
  bindings override the preserved semantic targets for any overlapping name. For the shipped fixture
  the two agree, so this is currently harmless, but it inverts the design's stated priority
  ("obtain type bindings from the analyzed semantic state, not from a partial reconstruction"). If a
  reconstructed binding ever diverges, the less-trustworthy value wins.
- **Recommendation:** Insert the authoritative analyzed bindings last (or skip reconstruction for
  names already covered by `prepared_bindings`) so preserved semantic targets take precedence.
- **Fix:** Changed prepared-module construction to insert reconstructed visible-import bindings first
  and preserved analyzed bindings last, so authoritative semantic targets take precedence.
- **Verification:** Confirmed. In `build_prepared_module_for` the visible-import reconstruction loop
  now runs first ([builder.rs:1868-1894](../../../crates/nx-codegen/src/builder.rs#L1868-L1894)) and
  the authoritative `module_artifact.prepared_bindings` are inserted last
  ([builder.rs:1896-1900](../../../crates/nx-codegen/src/builder.rs#L1896-L1900)); given
  `insert_binding` is last-write-wins, analyzed targets now override any overlapping reconstructed
  binding, matching the design's stated priority.

### ✅ Verified - RF4 `prepared_module_for` rebuilt per type reference, now O(modules × type-refs)
- **Severity:** Low
- **Evidence:** This change adds a loop over *every* resolved module plus a binding-clone pass into
  `prepared_module_for` ([builder.rs:1722-1737](../../../crates/nx-codegen/src/builder.rs#L1722-L1737)).
  `build_type_ref` recomputes a fresh `PreparedModule` on every call
  ([builder.rs:1559-1566](../../../crates/nx-codegen/src/builder.rs#L1559-L1566)) and is invoked once
  per parameter/field/return type (e.g. lines 392, 434, 472, 556, 593, 1379). A record with N fields
  therefore rebuilds the whole-program peer map N times, making IR generation scale with
  `modules × type-references`. The `_with_prepared` variant already exists to thread a single prepared
  module through; the field/param loops do not use it.
- **Recommendation:** Build the `PreparedModule` once per module (or memoize by module id) and pass it
  to `build_type_ref_with_prepared` for the field/parameter loops.
- **Fix:** Added `PreparedModuleCache` and threaded it through codegen so each resolved module's
  prepared view is built once per `build_codegen_program` call and reused by type-reference and
  descriptor paths.
- **Verification:** Confirmed. `PreparedModuleCache` memoizes by `module.id.as_u32()`
  ([builder.rs:205-225](../../../crates/nx-codegen/src/builder.rs#L205-L225)); `build_type_ref` and
  the component-contract paths now call `prepared_cache.get(...)` instead of rebuilding, and the
  cache is threaded through every `build_*` call site. The whole-program peer map is therefore built
  at most once per module per codegen run rather than once per type reference. All codegen tests
  still pass, so the refactor is behavior-preserving.

### ✅ Verified - RF5 No test for the "missing semantic data remains diagnostic" scenario
- **Severity:** Low
- **Evidence:** Task 3.3 and the `sdk-node` spec scenario "Missing library semantic data remains
  diagnostic" require that genuinely incomplete artifacts still produce a structured
  `codegen-missing-semantic-data` diagnostic and never emit partial IR. No Rust or Node test asserts
  this negative path (grep for `missing_semantic_data`/`codegen-missing` in the new tests returns
  nothing). The diagnostic code path is preserved
  ([builder.rs:1593-1599](../../../crates/nx-codegen/src/builder.rs#L1593-L1599)), but the requirement
  is unverified, and RF1/RF2 widen the set of names that now resolve instead of diagnosing.
- **Recommendation:** Add a focused test that triggers `codegen-missing-semantic-data` and asserts no
  IR JSON is returned.
- **Fix:** Added `nx_ir_missing_semantic_data_fails_without_partial_document`, which corrupts a
  program artifact's lowered module and asserts `emit_nx_ir` returns `codegen-missing-semantic-data`
  without producing IR.
- **Verification:** Confirmed. The test nulls `root_modules[0].lowered_module`, which hits the
  `missing_semantic_data_diagnostic(module, "lowered module", ...)` branch in `build_module`
  ([builder.rs:112-119](../../../crates/nx-codegen/src/builder.rs#L112-L119)); `build_codegen_program`
  returns `Err` whenever diagnostics are non-empty before constructing any document
  ([builder.rs:76-78](../../../crates/nx-codegen/src/builder.rs#L76-L78)), and the diagnostic code is
  `codegen-missing-semantic-data` ([builder.rs:2071](../../../crates/nx-codegen/src/builder.rs#L2071)).
  The test asserts `emit_nx_ir` returns that code and yields no IR; it passes. This is a coarse proxy
  (it removes the whole lowered module rather than a single unresolvable binding) but faithfully
  covers the "no partial IR on missing semantic data" requirement.

## Questions
- Resolved during fix: `Element` remains an object-shaped compiler boundary fallback only when no
  declared/imported binding resolves first, and `design.md` now records that behavior.

## Summary
The core fix is sound: `analyze_prepared_module` now captures prepared bindings
(value/type/element namespaces) deterministically, library module artifacts carry them through
`finalize_module_artifact`, and `prepared_module_for` seeds codegen with the preserved semantic
targets — which is exactly what lets the cross-library `QuestionFlow`/`FlowStep` references resolve to
module-qualified nominal records. The Rust and Node regressions assert that path well, and the
`programFingerprint` string conversion is correctly and consistently applied through the single
serialization path. The review-fix pass resolved the `Element` shadowing risk, added coverage for
element-namespace type references and missing semantic data diagnostics, restored analyzed-binding
precedence, and memoized prepared-module construction for the codegen hot path.
