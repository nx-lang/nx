## 1. Audit and baseline

- [x] 1.1 Confirm the merged runtime and `refactor-tooling-api` prerequisites; recheck source/artifact evaluation callers in NX, ReachMe and FiddleEngine, inventory exact removals/retained APIs, and record any new caller before deleting its API
- [x] 1.2 Audit all emitter rejection paths and unsupported constructs against source/interpreter behavior; record coverage and resolve source-reachable CLI/semantic gaps; defer the two approved construction gaps to complete-ir-construction-lowering with ignored success-expectation tests
- [x] 1.3 Record semantic test inventory, small/large/multi-module CLI timings, WASM size and baseline checks; and compare them later with the explanation threshold in the design

## 2. Preserve behavior through test-only IR composition

- [x] 2.1 Add integration-test-only build/emit/prepare/link helpers above the compiler/runtime boundary; verify explicit expected values for single- and multi-module functions without adding production exports
- [x] 2.2 Inventory and port interpreter language-behavior integration/unit cases, including known agent-library/import failures; classify every original case as ported, intentionally changed, redundant or interpreter-internal with a reason
- [x] 2.3 Port component lifecycle, explicit-state, nested host-input, token/effect and atomic-dispatch coverage to existing runtime APIs; retain DTO serialization/update-helper tests without interpreter execution
- [x] 2.4 Run temporary differential checks on supported cases and handwritten IR expectations on all ported cases; record declared int32/error/limit differences separately and resolve unexpected discrepancies before deletion except the two explicitly approved follow-up gaps

## 3. Formatting and CLI

- [x] 3.1 Replace the formatter's interpreter Value with a formatter-owned value model in nx-api, retaining format_program_artifact_value/formatValue annotations, type/state options and existing canonical ambiguities; verify playground/value-view fixtures and formatting error recovery
- [x] 3.2 Add a read-only Rust runtime result view that keeps constant cases and function references and feeds the same formatter; verify enum-at-object, nested values, function names, declared float32, float32 at an object site and round trips, without a float32 value kind, new lifecycle variants or compiler dependencies in the runtime
- [x] 3.3 Implement CLI source execution through all-module IR emission/prepare/link; test single- and cross-module programs, entry selection, missing root and static-error gating, with unchanged invocation and output; record direct IR input in specs/future.md
- [x] 3.4 Share CLI runtime options/output/error handling; preserve NX/JSON formats, map debug locations and unavailable-source errors, enforce a per-invocation 1,000,000-operation default, and test exhaustion/reset without changing runtime library defaults or playground limits

## 4. Remove legacy API and binding execution surfaces

- [x] 4.1 Remove the listed Rust source/artifact evaluation, component wrappers, ResourceLimits and interpreter-value conversion exports/types; retain construction helpers, formatting models, diagnostics and the existing ir_runtime namespace
- [x] 4.2 Remove C FFI execution/lifecycle exports and .NET Evaluate/InitializeComponent/EvaluateComponent/DispatchComponentActions overloads and execution-only models; retain codegen/build/schema/DTO/serialization/update helpers and shared errors still used by tooling
- [x] 4.3 Remove Node artifact/source JSON/byte evaluation and WASM evaluateNx/native export; retain tooling APIs and WASM formatValue; verify playground, ReachMe and FiddleEngine source paths require no removed method
- [x] 4.4 Refresh generated C header and native/WASM ABI versions and wrapper expectations together; replace retained API smoke tests and remove deleted API/opaque-snapshot envelope tests with coverage accounting, without adding replacement snapshot transport

## 5. Corpus and parity

- [x] 5.1 Replace corpus expected-result generation and checks with direct Rust IR runtime evaluation; regenerate and review every result difference, requiring unchanged goldens except explicitly justified changes
- [x] 5.2 Replace generated-JavaScript/TypeScript parity and facade tests that call the interpreter with test-only IR helpers plus handwritten expectations; cover int32 crossing 2^31 and document/test the existing distinct 2^53/i64 backend limits
- [x] 5.3 Complete the source-test coverage and final differential inventory; verify semantic expectations, diagnostics/limits, components and formatting pass before removing the comparison engine

## 6. Delete, document and validate

- [x] 6.1 Delete nx-interpreter and its workspace/dependency entries, temporary differential helpers and remaining interpreter imports in a separate final implementation commit; remove nx-interpreter from scripts/check-crate-layering.mjs and its test; verify crate-layering guards and runtime-only packaging/builds
- [x] 6.2 Verify the already-synced main specs, then archive with `--skip-specs` to preserve the retired component-runtime-bindings capability deletion; edit the graphics-library, graphics-preview and resolved-program-runtime Purpose paragraphs by hand; grep openspec/specs for remaining interpreter references; remove or rewrite the interpreter-only specs/future.md entries (32-bit values, discarded range types, aliased imports); validate all specs strictly
- [x] 6.3 Update API/binding/CLI READMEs, generated declarations, examples, value-view docs, IR docs and corpus instructions with compile-then-run migration examples and exact breaking removals; document that .NET/C execution has no replacement binding in this change
- [x] 6.4 Run workspace build/test/clippy/fmt, crate-layering and package checks, Node/WASM/.NET suites, TypeScript runtime/parity, CLI, playground, value-view and website tests, native graphics/preview and supported runtime WASM checks; record results and justified removals
- [x] 6.5 Re-measure timings/WASM size against agreed criteria, document breaking API/snapshot/int32/diagnostic changes and run openspec validate retire-hir-interpreter --strict; record implementation task completion
