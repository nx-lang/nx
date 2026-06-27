# Review: add-node-native-host-binding

## Scope

**Reviewed artifacts:** proposal.md, design.md, tasks.md (35/35 complete), specs/dotnet-binding/spec.md, specs/sdk-node/spec.md

**Reviewed code:**
- `bindings/node/native/src/lib.rs` (napi-rs native layer)
- `bindings/node/src/index.ts`, `errors.ts`, `types.ts`, `native.ts` (TypeScript public API)
- `bindings/node/test/sdk-node.test.ts`, `package.json`, `tsconfig.json`, `.gitignore`
- `bindings/node/native/Cargo.toml`, root `Cargo.toml` workspace registration
- `bindings/node/README.md`
- Shared Rust references: `crates/nx-api/src/diagnostics.rs`, `crates/nx-api/src/artifacts.rs`, `crates/nx-value/src/lib.rs`, `crates/nx-codegen/src/ir.rs`
- `.NET` rename surface (`bindings/dotnet/src/NxLang.Sdk/**`, `bindings/dotnet/tests/NxLang.Sdk.Tests/**`)

**Verification performed:**
- `npx vitest run` in `bindings/node` → 8/8 tests pass against the prebuilt native binary.
- `npx tsc -p tsconfig.json --noEmit` → clean.
- `grep` for residual `NxLang.Runtime` references across `*.cs/*.csproj/*.sln/*.md/*.ps1/*.targets/*.yml/*.toml` → none (rename complete).
- Confirmed `native/*.node` is gitignored and the committed-looking `.node` artifact is untracked.

**Review-fix verification performed:**
- `cargo test -p nx-api` → 94/94 tests pass.
- `cargo test -p nx-sdk-node-native` → native crate builds and its 0 tests pass.
- `npm test` in `bindings/node` → native rebuild, TypeScript build, and 9/9 Vitest tests pass.
- `npm install` and `npm test` in `bindings/node` under Node 24.15.0 → refreshed Node 22+ tooling lockfile, native rebuild, TypeScript 6.0 build, and 9/9 Vitest 4.1 tests pass.
- `pwsh -NoProfile -File tools/packaging/Stage-NxSdkNativeArtifact.ps1 -Configuration Release` → stages the local `linux-x64` native SDK asset under `artifacts/nx-sdk`.
- `dotnet test bindings/dotnet/NxLang.sln` → 93/93 tests pass with the renamed `NxSdk*` build properties.
- `dotnet pack bindings/dotnet/src/NxLang.Sdk/NxLang.Sdk.csproj -c Release -p:Version=0.0.0 -p:PackageVersion=0.0.0 -p:NxSdkNativePackageAssetsRoot="$PWD/artifacts/nx-sdk/" -p:NxSdkSupportedRids=linux-x64` → creates a local single-RID package.
- `pwsh -NoProfile -File tools/packaging/Test-NxSdkPackage.ps1 -PackagePath bin/Packages/Release/NxLang.Sdk.0.0.0.nupkg -RuntimeIdentifiers linux-x64` → verifies package metadata and native SDK asset contents.
- `pwsh -NoProfile -File tools/packaging/SmokeTest-NxSdkPackage.ps1 -PackagePath bin/Packages/Release/NxLang.Sdk.0.0.0.nupkg -RuntimeIdentifier linux-x64` → verifies package consumption and evaluates `42`.

## Findings

### ✅ Verified - RF1 Codegen (IR generation) diagnostics drop labels and spans
- **Severity:** Low
- **Evidence:** [lib.rs:307-321](bindings/node/native/src/lib.rs#L307-L321) — `codegen_error` reconstructs each `NxDiagnostic` by hand and hard-codes `labels: Vec::new()`, so source spans/labels are discarded for `generateNxIr()` failures. The validation and evaluation paths instead serialize the full `nx_api::NxDiagnostic` (including labels with resolved spans) via `diagnostics_json` / `evaluation_error`. The `sdk-node` "Node diagnostics and errors are ergonomic and stable" requirement says diagnostics SHALL include `labels`/`span` "when the Rust diagnostic provides them," and `nx_codegen::CodegenError` carries internal `Diagnostic` values that may have labels. This is low impact because codegen runs on an already-built artifact and rarely emits spanned diagnostics, but it is an inconsistency with the other two diagnostic paths.
- **Recommendation:** Route codegen diagnostics through the shared `nx_api::diagnostics_to_api` conversion (passing the artifact's source) rather than manually rebuilding `NxDiagnostic` with empty labels, so IR-generation failures preserve the same structured span data as validation/evaluation.
- **Fix:** Added a shared `diagnostics_to_api_with_source_entries` conversion path and changed the Node native codegen error handler to pass the program artifact's preserved source entries, so `generateNxIr()` diagnostics retain labels and spans.
- **Verification:** Confirmed `codegen_error` ([lib.rs:312-323](bindings/node/native/src/lib.rs#L312-L323)) now routes `CodegenError.diagnostics` (internal `nx_diagnostics::Diagnostic` values with labels, per `options.rs:139-140`) through `diagnostics_to_api_with_source_entries`, supplying `program.source_entries()` ([artifacts.rs:95-106](crates/nx-api/src/artifacts.rs#L95)) and a fallback source. The shared conversion ([diagnostics.rs:107-179](crates/nx-api/src/diagnostics.rs#L107)) resolves each label's span/line/column from the matching source entry — identical to the validation/evaluation paths. A new Node test (`sdk-node.test.ts:189-208`) asserts an IR-generation diagnostic preserves `label.file === "input.nx"`, `primary === true`, and a non-empty span; it passes (9/9 Node tests, 94/94 `nx-api` tests). Fix is correct and complete.

### ✅ Verified - RF2 Tests assert error type but not the identity named in the diagnostic
- **Severity:** Low
- **Evidence:** Two `sdk-node` spec scenarios require the diagnostic to *name the offending identity*: "Duplicate normalized identities are rejected" (error "SHALL name the normalized identity involved") and "Missing workspace entry reports diagnostics" (error "SHALL contain structured diagnostics naming `missing.nx`"). The corresponding tests only assert the thrown type: [sdk-node.test.ts:71-95](bindings/node/test/sdk-node.test.ts#L71-L95) checks `toThrowError(NxEvaluationError)` for both the duplicate-identity and `entryIdentity: "missing.nx"` cases without inspecting the diagnostic message/identity. (The invalid-workspace test at line 60 does correctly assert `message.includes("shared/missing.nx")`.)
- **Recommendation:** Strengthen both tests to assert the thrown `NxEvaluationError.diagnostics` (or message) actually contains the normalized duplicate identity (`lib/config.nx`) and the missing entry identity (`missing.nx`), so the tests exercise the spec's identity-naming guarantee rather than only the failure type.
- **Fix:** Updated the Node tests to capture `NxEvaluationError` and assert duplicate normalized identity diagnostics mention `lib/config.nx` and missing workspace entry diagnostics mention `missing.nx`.
- **Verification:** Confirmed [sdk-node.test.ts:83-108](bindings/node/test/sdk-node.test.ts#L83) now captures the `NxEvaluationError` via `captureEvaluationError` and asserts `duplicateError.diagnostics.some(... message.includes("lib/config.nx"))` (line 90) and `missingEntryError.diagnostics.some(... message.includes("missing.nx"))` (line 102), exercising the spec's identity-naming guarantee rather than only the failure type. Tests pass (9/9). Fix is correct and complete.

### ✅ Verified - RF3 Byte serialization bypasses the shared `NxValue::to_msgpack_vec` helper
- **Severity:** Low
- **Evidence:** [lib.rs:253-260](bindings/node/native/src/lib.rs#L253-L260) — `value_bytes` calls `rmp_serde::to_vec(value)` directly. `nx_value` exposes the canonical `NxValue::to_msgpack_vec()` ([nx-value/src/lib.rs:96](crates/nx-value/src/lib.rs#L96)), which today is literally `rmp_serde::to_vec(self)`, so output is currently byte-identical and there is no correctness bug. However, the design states the binding should "treat the Rust/codegen output as the source of truth" for canonical bytes; calling the shared helper guards against future drift if the canonical MessagePack encoding changes.
- **Recommendation:** Use `value.to_msgpack_vec()` for the MessagePack path so the Node SDK inherits any future change to the canonical encoding automatically. Purely a maintainability/parity-hardening nit.
- **Fix:** Changed the Node native MessagePack evaluation path to use `NxValue::to_msgpack_vec()`.
- **Verification:** Confirmed `value_bytes` ([lib.rs:258-265](bindings/node/native/src/lib.rs#L258)) now calls `value.to_msgpack_vec()` for the MessagePack path instead of `rmp_serde::to_vec`, so the Node SDK inherits any future change to the canonical encoding. Byte-evaluation test (`sdk-node.test.ts:126`) confirms a valid `Buffer` result; passes. Fix is correct and complete.

### ✅ Verified - RF4 Packaging tool filenames still use "NxRuntime"
- **Severity:** Low
- **Evidence:** `tools/packaging/` retains `SmokeTest-NxRuntimePackage.ps1`, `Test-NxRuntimePackage.ps1`, `NxRuntimeRids.ps1`, and `Stage-NxRuntimeNativeArtifact.ps1`. Their *contents* were updated (no `NxLang.Runtime` string remains anywhere in the tree), and these names arguably refer to the native runtime artifact rather than the renamed managed package — so this is ambiguous rather than clearly wrong. Given the change's intent to standardize on SDK naming and "not retain the previous package identity," the `*Package*` script names reading "NxRuntimePackage" are a residual naming inconsistency.
- **Recommendation:** Optional — if these scripts package/verify the renamed `NxLang.Sdk` NuGet package, rename them to `*NxSdkPackage*` for consistency; if they genuinely refer to the native runtime asset, leave as-is. No functional impact either way.
- **Fix:** Renamed the packaging scripts, RID helper, CI artifact names, package-validation properties, and related docs/spec wording from `NxRuntime*` / native runtime asset terminology to `NxSdk*` / native SDK asset terminology for the `nx_ffi` package surface.
- **Verification:** Confirmed `tools/packaging/` now contains `NxSdkRids.ps1`, `SmokeTest-NxSdkPackage.ps1`, `Stage-NxSdkNativeArtifact.ps1`, and `Test-NxSdkPackage.ps1` (no `*NxRuntime*` filenames remain). The remaining in-tree `NxRuntime` token references (e.g. `SmokeTest-NxSdkPackage.ps1:43`, `bindings/dotnet/README.md`, active `dotnet-binding` spec) are the public managed static class `NxRuntime` ([NxLang.Sdk/NxRuntime.cs:18](bindings/dotnet/src/NxLang.Sdk/NxRuntime.cs#L18)), which is correctly out of this finding's scope (the rename targeted the package/native-asset surface, with `PackageId` = `NxLang.Sdk`). No residual `NxLang.Runtime` references exist anywhere outside archived changes. Fix is correct and complete.

### ✅ Verified - RF5 Design open question unresolved: supported Node major versions
- **Severity:** Low
- **Evidence:** [design.md:176-181](openspec/changes/add-node-native-host-binding/design.md#L176-L181) left open "Which Node major versions should be supported for the initial source-built package and future prebuilds?" The implementation originally used `engines.node >= 20`, but Node 20 is now past end-of-life and the README's "Future Distribution" section is expected to describe supported Node/platform targets per the `sdk-node` packaging requirement.
- **Recommendation:** Resolve the question explicitly with the current supported Node lines and ensure the package metadata plus README "Future Distribution" section state the supported Node majors and platform triples.
- **Fix:** Set the Node package engine floor to Node 22+, updated Node package tooling to current compatible versions (`@napi-rs/cli` 3.7, `@types/node` 22.20, TypeScript 6.0, Vitest 4.1), and updated the design/README to state Node 22+ source builds with active LTS validation starting on Node 22 and Node 24.
- **Verification:** Confirmed `package.json` declares `engines.node` `>=22.0.0` with `@napi-rs/cli ^3.7.2`, `@types/node ^22.20.0`, `typescript ^6.0.3`, `vitest ^4.1.9`. The README states Node 22+ source builds (lines 15-16) and a Node 22+ prebuild path under "Future Distribution" (lines 153-156), and `design.md` now records the resolution under "Resolved Questions" (lines 178-182). Build + tests pass under this tooling. Fix is correct and complete.

### ✅ Verified - RF6 Design open question unresolved: named-entrypoint host support sequencing
- **Severity:** Low
- **Evidence:** [design.md:176-181](openspec/changes/add-node-native-host-binding/design.md#L176-L181) leaves open whether named-entrypoint evaluation should be added to the Rust host API before the Node package exposes entrypoint selection beyond `root()`. Today the API surfaces an `entrypoint?: "root" | string` option ([types.ts:71-73](bindings/node/src/types.ts#L71-L73)) that throws for any non-`root` value ([index.ts:395-413](bindings/node/src/index.ts#L395-L413)) — correct against the current "unsupported named entrypoint does not fall back to global lookup" scenario, but it ships a typed option that always fails, which is a latent API-shape decision that should be deliberate, not left open.
- **Recommendation:** Resolve the question one way: either (a) keep `root`-only and document the `entrypoint` option as reserved-for-future with the throw behavior as intended, or (b) decide that named-entrypoint host support is a prerequisite and track it as a follow-up change. Update the design's Open Questions section to record the decision before archiving.
- **Fix:** Replaced the design open question with the explicit decision that the initial native Node SDK is `root()`-only and the `entrypoint` option is reserved for future host support, intentionally throwing `unsupported-entrypoint` for non-`root` values.
- **Verification:** Confirmed `design.md` "Resolved Questions" (lines 183-185) records the `root()`-only decision with the `entrypoint` option reserved for future host support. `assertSupportedRootEntrypoint` ([index.ts:561-579](bindings/node/src/index.ts#L561)) returns early for `"root"` (and the default) and otherwise throws `NxEvaluationError` with a structured `unsupported-entrypoint` diagnostic — no JavaScript-side global lookup fallback. The "does not fall back to JavaScript-side named entrypoint lookup" test (`sdk-node.test.ts:218-225`) asserts a non-`root` entrypoint throws `NxEvaluationError`; passes. Fix is correct and complete.

## Summary
Solid, well-scoped implementation that matches the proposal and specs closely. The review-fix pass addressed RF1, RF2, RF3, RF4, RF5, and RF6: codegen diagnostics now preserve labels/spans through shared conversion, Node tests assert the required diagnostic identities and IR diagnostic labels, MessagePack output uses the canonical `NxValue` helper, the .NET packaging/native `nx_ffi` asset tooling now uses SDK naming, and the design/README now resolve Node 22+ support plus the `root()`-only named-entrypoint sequencing decision.
