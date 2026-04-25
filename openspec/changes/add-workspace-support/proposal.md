## Why

NX hosts currently need to materialize database- or editor-backed NX modules into temporary
directories before validation and evaluation. This adds avoidable IO, makes diagnostics harder to
map back to caller-owned records, and forces in-memory callers to pretend logical modules are real
files.

## What Changes

- Introduce `NxWorkspace` as the public logical workspace abstraction for a set of NX modules that
  are visible together for validation, import resolution, and program construction.
- Introduce source-backed `NxWorkspaceModule` values whose primary payload is UTF-8 bytes and whose
  stable identity is a logical module identity, not a filesystem path.
- Generalize the internal NX program-building pipeline around a logical module graph and resolver
  so both in-memory workspaces and filesystem-backed NX code use the same analysis, import,
  diagnostics, artifact, and entrypoint semantics.
- Add Rust APIs to validate an in-memory workspace and to build a `ProgramArtifact` for a selected
  workspace entry module while reusing `ProgramBuildContext` for loaded libraries and built-ins.
- Rework existing single-source and filesystem-backed build/evaluation paths as adapters over the
  shared module graph where doing so simplifies the implementation.
- Add native FFI APIs that accept borrowed workspace module descriptors for the duration of the
  call and never retain caller-owned pointers after returning.
- Add managed .NET APIs centered on `NxWorkspace`, `NxWorkspaceModule`,
  `ValidateWorkspace`, and `BuildWorkspaceProgramArtifact`.
- Update import resolution so workspace modules resolve by normalized logical identity and relative
  imports can bind to in-memory workspace modules without probing the filesystem.
- Update diagnostics so workspace labels use normalized module identities and spans are calculated
  from the submitted in-memory source bytes.
- Update program artifact and runtime entrypoint semantics so workspace builds execute `root()` from
  the selected entry module rather than from an arbitrary global symbol table entry.

## Capabilities

### New Capabilities
- `workspace-programs`: Defines logical NX workspaces, in-memory source modules, shared module-graph
  analysis for in-memory and filesystem-backed source providers, workspace validation,
  workspace-backed program artifact construction, FFI ownership rules, and managed workspace APIs.

### Modified Capabilities
- `source-analysis-pipeline`: Shared analysis must support aggregated diagnostics over multiple
  modules from a generalized source provider and must use caller-provided source maps for
  diagnostic spans before falling back to filesystem-backed sources.
- `module-imports`: Import resolution must define workspace identity normalization, relative
  workspace import lookup, duplicate identity rejection, missing import diagnostics without
  filesystem probing, and shared resolver semantics for filesystem-backed NX code.
- `artifact-model`: Program artifacts must preserve root modules, selected entry identity,
  diagnostics, and fingerprint metadata derived from the source provider's module identities and
  content.
- `program-build-context`: Program builds must continue to resolve reusable libraries through the
  supplied context while allowing workspace modules to satisfy module-local imports first.
- `resolved-program-runtime`: Runtime entrypoint lookup must be scoped to the selected workspace
  entry module so multiple submitted modules can each define `root()`.
- `dotnet-binding`: The managed binding must expose byte-oriented workspace models and wrappers for
  workspace validation and workspace program artifact builds.

## Impact

- Affected code spans `crates/nx-api`, `crates/nx-hir`, `crates/nx-types`,
  `crates/nx-interpreter`, `crates/nx-ffi`, `bindings/c`, and `bindings/dotnet`.
- Public Rust, native FFI, and .NET API surface area will change; this change does not need to
  preserve backward compatibility for existing APIs or behavior when a simpler unified design is
  available.
- Diagnostics, import resolution, and program construction need an internal resolver/source-map
  abstraction so in-memory flows avoid filesystem assumptions while filesystem-backed flows reuse
  the same module-graph pipeline.
- Nexara can replace temporary source-set materialization with direct construction of
  `NxWorkspace` modules using stable logical identities.
