# Historical planning review (superseded scope)

This review assessed the earlier proposal that preserved every source/artifact execution API.
The proposal, design, tasks and deltas were rewritten on 2026-11-01 after the tooling refactor merged
and the decision to remove those APIs. The findings and effort estimate below are historical,
not approval or open findings against the revised plan. A fresh independent review is requested.

- RF1: revised integer delta limits its claim to the existing backend arithmetic contract.
- RF2: revised plan removes the execution cache entirely; nx-program remains execution-independent.
- RF3: revised CLI plan specifies a finite operation budget; removed APIs receive no replacement limits type.

---

# Review: retire-hir-interpreter

## Scope
**Reviewed artifacts:** proposal, design, tasks, and all seven delta specifications.
**Reviewed code:** current nx-api artifact, evaluation, component and value interfaces; nx-codegen dependencies and IR unsupported paths; nx-ir-runtime values, options, arithmetic, serialization and public exports; interpreter limits and test inventory; active support-stateful-component-value-evaluation proposal.

This is a planning and architecture review. All 24 tasks are unchecked; implementation correctness has not been verified. `openspec validate retire-hir-interpreter --strict` passes. No runtime suites were run.

## Findings

### 🔴 Open - RF1 Integer specification promises behavior the target runtime does not implement
- **Severity:** High
- **Evidence:** specs/primitive-type-names/spec.md:4-8 requires int64 arithmetic not to wrap at its declared width and promises identical integer results across all backends. The Rust value is i64 (crates/nx-ir-runtime/src/value.rs:31), and integer addition, subtraction and multiplication use wrapping operations (crates/nx-ir-runtime/src/eval.rs:1987-1995). JavaScript number precision also prevents general parity beyond its safe integer range. Removing int32 wrapping does not establish that broader contract.
- **Recommendation:** Limit this delta to removing int32-specific wrapping under the existing int contract, explicitly retain existing backend limits, and add boundary tests around 2^31, 2^53 and i64 overflow. If universal exact arithmetic is intended, make it a separate prerequisite with implementation tasks for every backend.

### 🔴 Open - RF2 Cache ownership and artifact revision rules need a concrete design
- **Severity:** Medium
- **Evidence:** design.md:44-59 moves ProgramArtifact into nx-program but places a linked execution cache “beside” it without specifying its owner or type. ProgramArtifact currently derives Clone and exposes mutable root_modules, entry identity, libraries, fingerprint and resolved_program (crates/nx-api/src/artifacts.rs:90-111). A OnceLock retained after mutation can execute stale code. A cache whose type depends on nx-codegen would recreate the dependency cycle the split is meant to remove.
- **Recommendation:** Specify the cache field/type and acyclic dependency graph before extraction; an nx-ir-runtime::Program cache can be held below nx-api without depending on nx-codegen. Define immutable artifact revisions or explicit invalidation, clone-sharing behavior, cached failure behavior, and concurrent initialization. Test repeated calls, cloned handles, distinct revisions and concurrent callers. Account for current pub(crate) helpers used across the proposed split.

### 🔴 Open - RF3 Resource-limit defaults remain unresolved at the native switch
- **Severity:** Medium
- **Evidence:** design.md's Open Questions leaves the operation-budget default finite versus unlimited unresolved. Current ResourceLimits defaults to 1,000,000 operations (crates/nx-interpreter/src/context.rs:40-43); RuntimeOptions defaults to None (crates/nx-ir-runtime/src/eval.rs:119). Task 5.2 only says to map the fields. This can silently remove an existing limit, while keeping the numeric limit changes its effective meaning because IR and HIR count different work.
- **Recommendation:** Explicitly retain a finite native API/CLI/playground default for this migration unless a separate behavior change is intended. Add source-level exhaustion tests through native and binding paths, confirm per-call/reset behavior, and document that the operation count unit changes. Treat any default-policy change as an explicit compatibility decision.

## Questions
- Which change lands first: this migration or support-stateful-component-value-evaluation? The latter currently proposes interpreter work and adds native/.NET workflows that this migration must preserve.
- What cold-evaluation latency and WASM size regression would block cutover? Recording numbers alone does not establish a release gate.

## Scope and effort

This is a substantial execution-path migration: crate extraction, roughly 553 existing interpreter tests to inventory and classify, rich runtime values/float32 preservation, snapshot encoding, diagnostic mapping, native API/CLI rewrites, binding validation, corpus changes, and deletion/documentation.

Planning estimate for one engineer familiar with the repository, assuming no significant language-coverage gaps:
- Audit, baseline and design resolution: 1–2 working days.
- Crate extraction and cache foundation: 2–3 days.
- Test migration, runtime/value/snapshot work and native cutover: 5–8 days.
- Binding/corpus checks, fixes, deletion and documentation: 2–4 days.
- Total: approximately 10–17 working days (2–4 weeks elapsed); allow longer if differential tests reveal missing semantics. This is a judgment estimate, not measured implementation throughput. The approximately 21,600 deleted lines include tests that must largely survive elsewhere, so they are not all maintenance savings.

## Preparation and sequencing

1. Resolve RF1–RF3 and coordinate the active component-value evaluation change.
2. Inventory source-reachable emitter gaps, actual interpreter-only behavior, tests, and diagnostic expectations; audit builder rejection paths as well as the literal unsupported diagnostic sites.
3. Capture a clean suite baseline and binding/toolchain readiness. Measure cold small-program evaluation, representative large/multi-module evaluation, warm dispatch and edit/rebuild cycles, plus WASM size. Agree acceptable regressions.
4. Land the program-building extraction independently, preserving current execution. Define cross-crate accessors and cache ownership explicitly.
5. Port handwritten semantic expectations and run both engines before switching. Keep declared differences separate from newly discovered regressions. Include the known agent-library failures as positive IR-path regression cases.
6. Switch native execution, then require all native, generated-JavaScript, TypeScript, .NET, Node, WASM and playground checks before the deletion commit. Inventory later runtime features (constraints, function references, origins and explicit-state component rendering) rather than relying only on the older test count.

## Architectural assessment

Retirement is a reasonable destination. Running the source path through the shipped IR runtime reduces duplicate semantic implementation and exercises code generation during ordinary use. The design already records interpreter failures that both IR runtimes handle.

The strongest reason to retain the HIR interpreter is independent compiler validation: two IR runtimes cannot detect the same emitter mistake. Other possible reasons are a materially faster cold path for interactive editing, HIR-level debugging, and a way to prototype semantics before implementing IR support. These are potential uses, not established current requirements. Keeping a production fallback would hide emitter gaps and perpetuate differing behavior; avoid that unless measurements or a concrete product need justify it.

Retain the interpreter temporarily as a test-only comparison engine during migration. Port semantic assertions before deletion, retain reviewed goldens, and consider bounded generated source programs with manually/metamorphically specified properties for additional compiler checks. The generated-JavaScript backend remains a useful comparison against IR-specific mistakes, though it shares compiler infrastructure and is not fully independent. Git history provides recovery without maintaining a permanently second runtime.

## Summary
Three findings opened. Support staged retirement after the design gaps and differential gates are addressed; do not treat this as a simple deletion. No implementation code was changed.
