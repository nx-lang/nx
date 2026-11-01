## 1. Audit and baseline

- [ ] 1.1 Confirm the merged runtime and `refactor-tooling-api` prerequisites; recheck source/artifact evaluation callers in NX, ReachMe and FiddleEngine, inventory exact removals/retained APIs, and record any new caller before deleting its API
- [ ] 1.2 Audit all emitter rejection paths and unsupported constructs against source/interpreter behavior; record coverage and fix or resolve source-reachable CLI/semantic gaps before cutover
- [ ] 1.3 Record semantic test inventory, small/large/multi-module CLI timings, WASM size and baseline checks; confirm proposed CLI flags and settle performance acceptance criteria before CLI implementation

## 2. Preserve behavior through test-only IR composition

- [ ] 2.1 Add integration-test-only build/emit/prepare/link helpers above the compiler/runtime boundary; verify explicit expected values for single- and multi-module functions without adding production exports
- [ ] 2.2 Inventory and port interpreter language-behavior integration/unit cases, including known agent-library/import failures; classify every original case as ported, intentionally changed, redundant or interpreter-internal with a reason
- [ ] 2.3 Port component lifecycle, explicit-state, nested host-input, token/effect and atomic-dispatch coverage to existing runtime APIs; retain DTO serialization/update-helper tests without interpreter execution
- [ ] 2.4 Run temporary differential checks on supported cases and handwritten IR expectations on all ported cases; record declared int32/error/limit differences separately and resolve unexpected discrepancies before deletion

## 3. Formatting and CLI

- [ ] 3.1 Refactor canonical-value formatting to remove interpreter Value conversions, retaining format_program_artifact_value/formatValue annotations, type/state options and existing canonical ambiguities; verify playground/value-view fixtures and formatting error recovery
- [ ] 3.2 Implement minimal read-only runtime-result access for CLI NX spelling as needed, including float32 provenance; verify enum-at-object, nested values, function names, float32 and round trips without new lifecycle variants or compiler dependencies in the runtime
- [ ] 3.3 Implement CLI source compilation through all-module IR emission/prepare/link and direct IR input with explicit format/module catalog selection; test equivalent source/IR results, cross-module entry selection, missing modules and missing root, with unchanged existing source invocation behavior
- [ ] 3.4 Share CLI runtime options/output/error handling; preserve NX/JSON formats, map debug locations and unavailable-source errors, enforce a per-invocation 1,000,000-operation default, and test exhaustion/reset without changing runtime library defaults or playground limits

## 4. Remove legacy API and binding execution surfaces

- [ ] 4.1 Remove the listed Rust source/artifact evaluation, component wrappers, ResourceLimits and interpreter-value conversion exports/types; retain construction helpers, formatting models, diagnostics and the existing ir_runtime namespace
- [ ] 4.2 Remove C FFI execution/lifecycle exports and .NET Evaluate/InitializeComponent/EvaluateComponent/DispatchComponentActions overloads and execution-only models; retain codegen/build/schema/DTO/serialization/update helpers and shared errors still used by tooling
- [ ] 4.3 Remove Node artifact/source JSON/byte evaluation and WASM evaluateNx/native export; retain tooling APIs and WASM formatValue; verify playground, ReachMe and FiddleEngine source paths require no removed method
- [ ] 4.4 Refresh generated C header and native/WASM ABI versions and wrapper expectations together; replace retained API smoke tests and remove deleted API/opaque-snapshot envelope tests with coverage accounting, without adding replacement snapshot transport

## 5. Corpus and parity

- [ ] 5.1 Replace corpus expected-result generation and checks with direct Rust IR runtime evaluation; regenerate and review every result difference, requiring unchanged goldens except explicitly justified changes
- [ ] 5.2 Replace generated-JavaScript/TypeScript parity and facade tests that call the interpreter with test-only IR helpers plus handwritten expectations; cover int32 crossing 2^31 and document/test the existing distinct 2^53/i64 backend limits
- [ ] 5.3 Complete the source-test coverage and final differential inventory; verify semantic expectations, diagnostics/limits, components and formatting pass before removing the comparison engine

## 6. Delete, document and validate

- [ ] 6.1 Delete nx-interpreter and its workspace/dependency entries, temporary differential helpers and remaining interpreter imports in a separate final implementation commit; verify crate-layering guards and runtime-only packaging/builds
- [ ] 6.2 Sync the explicit API-removal/CLI/formatting deltas and sweep remaining interpreter wording without inventing replacement APIs; reconcile pending support-stateful-component-value-evaluation scope and validate all specs strictly
- [ ] 6.3 Update API/binding/CLI READMEs, generated declarations, examples, value-view docs, IR docs and corpus instructions with compile-then-run migration examples and exact breaking removals; document that .NET/C execution has no replacement binding in this change
- [ ] 6.4 Run workspace build/test/clippy/fmt, crate-layering and package checks, Node/WASM/.NET suites, TypeScript runtime/parity, CLI and playground tests, native graphics/preview and supported runtime WASM checks; record results and justified removals
- [ ] 6.5 Re-measure timings/WASM size against agreed criteria, document breaking API/snapshot/int32/diagnostic changes and run openspec validate retire-hir-interpreter --strict; leave implementation tasks unchecked in the proposal PR
