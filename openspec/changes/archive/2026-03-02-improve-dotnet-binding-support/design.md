## Context

NX already exposes a layered interop stack:

- `crates/nx-api` defines stable Rust-side values and diagnostics for consumers.
- `crates/nx-ffi` exposes a small C ABI used by foreign runtimes.
- `bindings/c/nx.h` mirrors the native ABI for C consumers.
- `bindings/csharp` contains the managed wrapper, tests, and documentation.

That structure works, but the .NET story is still incomplete. The binding is stored under a C#-specific path, the public managed model is not explicitly CLS-compliant, the native ABI/header contract is maintained manually, and the documented consumption flow depends on manually setting library search paths. The immediate target is not published distribution; it is source-based consumption by repositories that vendor NX via git submodule or subtree and build it locally.

Constraints for this change:

- Keep the binding implementation and examples C#-first.
- Document that other .NET languages are expected to work, but do not claim verified support beyond C# yet.
- Keep the managed package target framework at `.NET 10` only.
- Do not add non-C# smoke tests in this change.
- Do not require published NuGet distribution in this change.

## Goals / Non-Goals

**Goals:**

- Rename the repository layout from `bindings/csharp` to `bindings/dotnet` without changing the core Rust layering.
- Define a clearer .NET binding boundary between public managed API, internal native loading/PInvoke code, and Rust ABI exports.
- Make the public managed API explicitly CLS-compliant.
- Add a native ABI compatibility check so managed code can fail fast on mismatch.
- Support vendored-source consumption where another repository builds NX and references the resulting managed/native artifacts.
- Improve .NET-facing documentation and tests to describe the supported workflow precisely.

**Non-Goals:**

- Publishing a public NuGet package.
- Adding first-class F#, VB, or other non-C# examples or tests.
- Replacing the C ABI with a different interop technology such as UniFFI or C++/CLI.
- Broadening framework support beyond `.NET 10`.
- Expanding the native API to mirror all Rust internals.

## Decisions

### 1. Rename `bindings/csharp` to `bindings/dotnet`, but keep C# as the primary authoring language

The repository path will change from `bindings/csharp` to `bindings/dotnet`. The managed assembly, tests, and docs will still be authored in C#, and examples will remain C#-first.

Rationale:

- The directory name should describe the platform surface, not the implementation language alone.
- The binding is intended to be consumable from other .NET languages even if only C# is actively validated today.
- This naming leaves room for future multi-language .NET guidance without forcing a second rename later.

Alternatives considered:

- Keep `bindings/csharp`: rejected because it understates the intended platform scope.
- Rename only documentation references: rejected because repo layout would still communicate the narrower scope.

### 2. Keep the existing Rust layering and continue using a thin C ABI plus a thin managed wrapper

The design will keep:

- `crates/nx-api` for stable Rust-side DTOs and values.
- `crates/nx-ffi` for the native `cdylib`.
- `bindings/c/nx.h` as the exported native contract.
- `bindings/dotnet` for the managed wrapper, tests, and docs.

The managed layer will remain intentionally thin. It will marshal inputs, call the native ABI, validate compatibility, translate failures into .NET exceptions, and deserialize payloads into .NET types.

Rationale:

- The current architecture already isolates Rust internals effectively.
- A coarse-grained ABI is more stable and easier to package than exposing many fine-grained native calls.
- This preserves the existing `nx-api -> nx-ffi -> managed wrapper` contract instead of reopening the interop architecture.

Alternatives considered:

- Move `nx-ffi` under `bindings/`: rejected because it is still a Rust workspace crate and belongs with the other Rust crates.
- Switch to subprocess or CLI invocation: rejected because it would weaken the library integration story.
- Switch to UniFFI: rejected because the current C ABI is already viable and this change is focused on support quality, not interop technology replacement.

### 3. Make the public managed API CLS-compliant, not just the assembly declaration

The managed assembly will declare explicit CLS compliance and the public API surface will be adjusted to comply with it. In practice, that means public types exposed by the runtime must avoid non-CLS primitives such as `uint` in favor of CLS-compliant equivalents and should expose strongly typed enums where the Rust side already has stable enums.

Expected public-surface changes include:

- Add `[assembly: CLSCompliant(true)]`.
- Replace public `uint` diagnostic/span fields with CLS-compliant signed integer types.
- Replace stringly typed severity with a managed `NxSeverity` enum that maps to the stable Rust severity model.
- Keep pointer-sized/native details internal to the interop layer.

Rationale:

- Declaring CLS compliance without aligning the public API would create misleading guarantees.
- The current public diagnostic model is the main blocker to real CLS compliance.
- Managed callers in other .NET languages benefit from stronger typing and fewer implementation leaks.

Alternatives considered:

- Add `[CLSCompliant(true)]` and mark individual public members `[CLSCompliant(false)]`: rejected because it would preserve the same cross-language friction while claiming compliance at the assembly level.
- Leave severity as a string: rejected because Rust already exposes a stable enum and the managed wrapper should not weaken that contract.

### 4. Add an explicit ABI version handshake and generate the native header from the Rust ABI

The native layer will export an ABI version function, and the managed layer will validate it before issuing runtime calls. The native C header will be generated from the Rust ABI definition so the exported contract is not maintained manually in two places.

Rationale:

- Path and packaging improvements reduce load failures, but they do not catch ABI drift.
- Generated headers reduce manual sync errors between Rust and C-facing representations.
- Fast failure on mismatch is safer than undefined behavior or confusing deserialization errors.

Alternatives considered:

- Rely on symbol presence only: rejected because it does not detect semantic ABI drift.
- Keep the header hand-maintained: rejected because the ABI surface is small enough that generation is cheap and less error-prone.

### 5. Standardize on source-based consumption via project reference and built outputs

This change will document and support a workflow where a consumer repository vendors NX as a git submodule or subtree, builds the native and managed components locally, and references the managed project or built assemblies directly. The supported path will not require creating or publishing a NuGet package.

The primary documented integration path will be a direct `ProjectReference` to the managed binding. Referencing built assemblies will be documented as a secondary fallback for consumers that cannot use a project reference.

The intended flow is:

1. Vendor NX source.
2. Build `crates/nx-ffi`.
3. Build the managed `bindings/dotnet` project.
4. Reference the managed project or built `NxLang.Runtime.dll` from the consuming application.
5. Copy or resolve the native library into the consuming app's output using documented build steps or helper targets.

Rationale:

- This is the narrowest supported workflow that matches the current adoption stage.
- Project reference and direct built-output reference are simpler than introducing packaging semantics that are not yet needed.
- It keeps all artifacts traceable to the vendored source tree.

Alternatives considered:

- Local-only NuGet packaging: rejected for now because it adds pack/restore complexity before a public packaging story exists.
- Manual PATH-based discovery as the primary workflow: rejected because it is too fragile for a documented integration path.

### 6. Keep C#-only validation for now, but state the support posture clearly

Tests, samples, and docs will remain C#-focused. Documentation will explicitly state that other .NET languages are expected to work because the binding is a normal managed assembly over a native dependency, but only C# usage is currently tested and documented.

Rationale:

- This matches actual validation without overcommitting support.
- It preserves focus on the highest-value consumer path.

Alternatives considered:

- Add immediate non-C# smoke tests: rejected by scope.
- Avoid mentioning other .NET languages entirely: rejected because it would understate the practical compatibility of the binding.

## Risks / Trade-offs

- [Directory rename breaks existing paths] -> Update all build scripts, docs, and references in one change and call out the rename as breaking.
- [CLS-compliance changes alter the managed API] -> Keep the semantic model the same, limit changes to typing and documentation, and document the managed-surface adjustments explicitly.
- [ABI handshake introduces another required native symbol] -> Keep the version export trivial and validate it centrally during managed initialization.
- [Source-based consumption may still be environment-sensitive across OSes] -> Provide a single documented workflow and helper targets/scripts that stage native binaries alongside managed outputs.
- [Generated header workflow adds tooling requirements] -> Use a lightweight, repo-local generation step and check in the generated output if needed for contributor convenience.

## Migration Plan

1. Rename `bindings/csharp` to `bindings/dotnet` and update all solution, docs, and script references.
2. Update the managed assembly surface for CLS compliance and new typed diagnostic models.
3. Add native ABI version export and managed compatibility validation.
4. Introduce header generation and update `bindings/c/nx.h`.
5. Add source-based build/reference documentation and helper build steps for vendored consumers.
6. Update C# tests to validate the new loading and API behavior.

Rollback strategy:

- The rename is the highest-impact step. If the broader change must be reverted, revert the directory and path updates together with the managed loader changes so the prior C# path remains internally consistent.

## Open Questions

- Whether helper build logic belongs in MSBuild targets, repository scripts, or both.
