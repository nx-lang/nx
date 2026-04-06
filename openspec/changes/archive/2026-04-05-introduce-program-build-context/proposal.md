## Why

NX can now build a `LibraryArtifact`, but the current proposal still puts library ownership in the
wrong place. Real hosts often need to preload and analyze shared libraries before any program
exists, and current analysis still relies on temporary copied imported HIR instead of a durable
dependency graph over library snapshots. The same workstream also already includes user-visible
enum-member casing updates in code, docs, and tests, but those language-surface changes are not
yet reflected in this OpenSpec change.

## What Changes

- **BREAKING** Replace the standalone reusable-library-handle model with a two-layer architecture:
  a long-lived `LibraryRegistry` that owns analyzed library snapshots, and a `ProgramBuildContext`
  that selects visible libraries from that registry when building a program.
- Add API entry points that load and analyze library roots into a reusable `LibraryRegistry` even
  when no executable program exists yet.
- Update program-building APIs to construct transient `ProgramArtifact`s from source against a
  registry-backed `ProgramBuildContext` rather than by reloading imported libraries from disk.
- Replace analysis-time copied imported HIR with registry-backed dependency resolution over
  library export and interface metadata, with `nx-api` owning module preparation and `nx-types`
  analyzing already-prepared lowered modules.
- Keep runtime evaluation and component lifecycle execution centered on `ProgramArtifact`; runtime
  module IDs remain program-specific and are assigned only when assembling a `ResolvedProgram`.
- Update Rust, FFI, and .NET APIs to expose registry and build-context workflows instead of a
  standalone reusable `LibraryArtifact` host API.
- Record the concurrent enum-member convention shift so first-party NX examples, tests, runtime
  formatting, and host value conversion use `snake_case` enum members instead of `PascalCase`,
  while preserving the exact source spelling at runtime.

## Capabilities

### New Capabilities
- `enum-values`: Defines the documented `snake_case` convention for enum members and requires
  first-party tooling/runtime surfaces to preserve enum member spellings exactly as written.
- `library-registry`: Defines reusable registries that load and analyze `LibraryArtifact`
  snapshots outside the lifetime of any single executable program.
- `program-build-context`: Defines registry-backed build contexts that select visible library
  snapshots when constructing fresh `ProgramArtifact`s from source.

### Modified Capabilities
- `artifact-model`: The artifact model needs to define `LibraryArtifact` as a local snapshot plus
  interface metadata, `LibraryRegistry` as the owner of snapshot graphs, and `ProgramArtifact` as
  the executable closure over selected library snapshots.
- `module-imports`: Import resolution needs to resolve imported libraries through registry-backed
  snapshot metadata instead of copying imported declarations into the importing module's stored
  HIR.
- `source-analysis-pipeline`: Shared source analysis needs to resolve imports through a supplied
  registry-backed build context and report missing libraries instead of silently loading them from
  disk.
- `dotnet-binding`: The managed API contract needs to expose `NxLibraryRegistry` and
  `NxProgramBuildContext` instead of a standalone reusable library handle.
- `component-runtime-bindings`: Component lifecycle bindings need to build transient
  `ProgramArtifact`s against a registry-backed `ProgramBuildContext`.

## Impact

- Affected code spans `nx-hir`, `nx-types`, `nx-api`, `nx-ffi`, `bindings/dotnet`, and
  tests/docs for library analysis, program construction, and component build flows.
- Public Rust, native FFI, and .NET APIs will change in intentionally breaking ways.
- `nx-hir` will become a lowering-only layer for public callers; the old public disk-loading
  import-resolution helper is removed rather than preserved as a compatibility shim.
- FFI ABI versioning and managed interop entry points will need to be updated to reflect registry
  and build-context handles.
- Import resolution, type analysis, and program fingerprinting will need to reflect exact selected
  library snapshots without depending on temporary copied imported HIR.
- First-party enum fixtures, grammar tests, runtime formatting, host value conversion, examples,
  and docs need to consistently reflect the `snake_case` enum-member convention already present in
  the unstaged code changes.
