# Review: represent-nxvalue-enum-as-string

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, archived delta specs under `specs/`, and synced specs under `openspec/specs/`  
**Reviewed code:** `crates/nx-value/src/lib.rs`, `crates/nx-api/src/value.rs`, `crates/nx-api/src/eval.rs`, `crates/nx-api/src/component.rs`, `crates/nx-interpreter/src/interpreter.rs`, `crates/nx-ffi/tests/ffi_smoke.rs`, `bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeBasicTests.cs`, `bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeComponentTests.cs`, and `bindings/dotnet/README.md`

## Findings

### 🟡 Fixed - RF1 Managed runtime coverage still misses an enum-typed DTO round-trip
- **Severity:** Medium
- **Evidence:** `bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeComponentTests.cs:363` and `bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeComponentTests.cs:381` only exercise the new wire shape through `string`-typed props and rendered fields. That is enough to prove the runtime now accepts and emits bare strings, but it does not verify the managed enum-recovery scenario required by `openspec/specs/dotnet-binding/spec.md:203`. A regression where raw runtime payloads stay correct but `NxRuntime.InitializeComponent<TProps, TElement>` or `DispatchComponentActions<TActions, TEffect>` stops mapping those strings back into enum-typed CLR properties through the shared converters would still pass the new tests.
- **Recommendation:** Add an end-to-end .NET runtime test that uses an actual enum-typed DTO property decorated/generated with `NxEnumJsonConverter` / `NxEnumMessagePackFormatter`, and assert both successful round-trip and unknown-member failure through the runtime wrapper rather than only through direct converter unit tests.
- **Fix:** Added end-to-end .NET component tests that round-trip an enum-typed DTO through `NxRuntime.InitializeComponent<TProps, TElement>`, map the raw JSON result from `InitializeComponentJson` back into a typed DTO, and assert the runtime wrapper surfaces a wrapped serialization failure when the managed enum wire format does not recognize the returned member.

### 🟡 Fixed - RF2 The change is copied into `archive` but not actually archived yet
- **Severity:** Low
- **Evidence:** `openspec/changes/archive/2026-04-18-represent-nxvalue-enum-as-string/tasks.md:35` still leaves the archive step unchecked, while `openspec/changes/archive/2026-04-18-represent-nxvalue-enum-as-string/tasks.md:41` claims the repo-wide verification is clean. In the current tree, the original active change directory still exists at `openspec/changes/2026-04-18-represent-nxvalue-enum-as-string/`, and `openspec status --change "2026-04-18-represent-nxvalue-enum-as-string" --json` still resolves the change as active rather than archived. As staged, this leaves two copies of the change in the repo and keeps OpenSpec tooling pointed at the active copy.
- **Recommendation:** Finish the archive workflow by removing or migrating the active change directory, then rerun the verification step and update the task state so the archive status matches what is actually in the tree.
- **Fix:** Removed the duplicate active change directory from `openspec/changes/`, kept the archived copy as the canonical change record, marked the archived task list's archive step complete, and corrected the archived verification note so it matches the intentional `$enum` / `$member` references that remain in synced spec text.

## Questions
- None.

## Summary
- The core Rust implementation looks coherent: enum strings are emitted consistently and schema-aware coercion is wired through component props, actions, and evaluation output. Both review findings are now fixed pending independent verification.