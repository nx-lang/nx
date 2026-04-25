## Context

NX already has the pieces needed for multi-module execution: `ModuleArtifact`, `LibraryArtifact`,
`ProgramArtifact`, `ProgramBuildContext`, and `ResolvedProgram` preserve source-file boundaries and
runtime module IDs. The missing piece is a shared logical module graph that can be populated from
different source providers. In-memory workspaces are the primary new public use case, but
filesystem-backed NX code should use the same internal analysis, import, diagnostics, artifact, and
entrypoint pipeline rather than remaining a separate concept.

The current `build_program_artifact_from_source` path accepts one source string plus a `file_name`.
It only resolves imports through `ProgramBuildContext` when that file name exists on disk, and the
diagnostic conversion layer may re-read label files when a label looks like a real path. That is
correct for file-backed flows but wrong for Nexara draft configuration, where the authoritative
source lives in database rows and the stable identifier is a logical NX identity.

There is no backward compatibility requirement for this change. Public APIs, native ABI shapes, and
existing file-backed behavior can change when doing so produces a simpler and more coherent
workspace/module-graph design.

## Goals / Non-Goals

**Goals:**
- Add a public `NxWorkspace` abstraction with source-backed `NxWorkspaceModule` entries.
- Keep workspace modules byte-oriented at API boundaries and validate UTF-8 before parsing.
- Generalize the internal pipeline around a logical module graph/source-provider abstraction used by
  both in-memory workspaces and filesystem-backed NX code.
- Normalize logical module identities without using filesystem APIs.
- Build and validate a whole workspace in memory with aggregated diagnostics.
- Resolve relative imports between workspace modules before falling back to libraries exposed by
  `ProgramBuildContext`.
- Preserve normalized workspace identities in diagnostics and use submitted source text for
  line/column mapping.
- Build a `ProgramArtifact` for an explicit entry identity and execute `root()` from that entry
  module.
- Rework or replace existing single-source and filesystem-backed APIs as adapters over the shared
  module-graph pipeline when that reduces complexity.

**Non-Goals:**
- Adding prepared, lowered, or serialized IR workspace module payloads.
- Replacing the existing directory-backed `LibraryRegistry` in the initial implementation.
- Introducing database-specific concepts into NX.
- Changing NX syntax.
- Defining cross-request workspace caching policy.
- Preserving old public API names, native ABI compatibility, or legacy file-backed edge behavior for
  its own sake.

## Decisions

### 1. `NxWorkspace` is the public abstraction, not `SourceWorkspace`

The Rust public model should be:

- `NxWorkspace { modules: Vec<NxWorkspaceModule> }` with validated, private module storage
- `NxWorkspaceModule { identity: String, source: Arc<str> }`

Rust constructors validate UTF-8 at the workspace boundary and then store decoded source text as
`Arc<str>` so module graphs and program artifacts can share submitted source maps without repeated
copies. The FFI accepts borrowed descriptors for the duration of one native call and copies valid
source directly into `Arc<str>` storage. The managed binding exposes `ReadOnlyMemory<byte>` so
Nexara can pass UTF-8 content from storage without pretending it is a file.

Alternative considered:
- Name the top-level concept `SourceWorkspace`.

Why rejected:
- Source bytes are the first payload format, not the long-term identity of the feature. The
  workspace should be able to grow future payload kinds without a public rename.

### 2. Workspace identities are normalized logical names

Workspace indexing should normalize identities with a small logical path normalizer:

- treat `/` as the separator
- reject empty identities and empty path segments after trimming
- reject absolute identities
- fold `.` and `..`
- reject normalized identities that escape the workspace root
- reject duplicate normalized identities
- preserve the normalized identity in all diagnostics and artifacts

The normalizer must not call `Path::exists`, `fs::canonicalize`, `is_dir`, or platform-specific path
cleanup. This keeps workspace behavior stable across operating systems and avoids accidental
coupling to temporary directories.

Alternative considered:
- Reuse `PathBuf` canonicalization for path-like identities.

Why rejected:
- Workspace identities are caller-owned logical keys. Canonicalization would both require files to
  exist and change behavior by host platform.

### 3. Add a shared module graph, resolver, and source-map layer in `nx-api`

The first implementation should add a focused internal abstraction instead of threading ad hoc maps
through every call. The core should be source-provider agnostic:

- an indexed module source map keyed by normalized logical identity
- a resolver that can resolve relative imports against an importing module identity
- helpers that convert diagnostics with an `identity -> source text` map before any file fallback
- an in-memory source provider for `NxWorkspace`
- a filesystem source provider for existing source-file and directory-backed workflows

For in-memory workspace builds, import resolution order should be:

1. normalize the import string against the importing module identity's parent
2. if the normalized identity exists in the workspace, bind to that workspace module
3. otherwise resolve supported library imports through the supplied `ProgramBuildContext`
4. otherwise produce a structured missing-import diagnostic

The initial workspace lookup should require exact normalized identity matches. That keeps the API
predictable: callers that use identities such as `shared/questions.nx` should import that logical
module using the same logical identity shape.

For filesystem-backed builds, the file provider should be responsible for discovering and reading
source files and mapping them into the same logical module graph. File IO belongs in that provider,
not in the shared resolver. Once source content and identities are loaded, parsing, preparation,
diagnostics, artifact construction, fingerprinting, and entrypoint selection should follow the same
code path as in-memory workspaces.

Alternative considered:
- Keep workspace and filesystem program construction as separate pipelines.

Why rejected:
- Separate pipelines would duplicate import and diagnostic behavior and recreate the mismatch this
  change is meant to remove. Source-provider adapters keep IO-specific behavior contained while the
  compiler operates on one module graph model.

### 4. Workspace validation analyzes the effective module set as a whole

`validate_workspace` should parse, lower, prepare, and type-check every submitted module once and
return one aggregated diagnostic list. It should not require the caller to loop over modules or
build a separate program artifact for each candidate entry.

Validation should still construct peer visibility and import visibility in the same order the
program builder uses. This prevents a workspace from passing validation and then failing
construction because the two paths disagree about relative imports or visible declarations.

The same analysis core should be callable by a filesystem source provider after it has loaded the
requested source set. That lets file-backed validation and workspace validation differ only in how
source modules are discovered and read.

Alternative considered:
- Implement validation as repeated single-module builds.

Why rejected:
- Repeated builds would duplicate work, make duplicate identities and cross-module diagnostics
  harder to report cleanly, and risk inconsistent import decisions across modules.

### 5. Workspace program builds select an explicit entry identity

`build_workspace_program_artifact` should require an `entry_identity` string, normalize it with the
same identity normalizer, and fail with diagnostics if the normalized entry is not in the workspace
or cannot produce a lowered module.

`ProgramArtifact` should record the normalized entry identity and, when the selected entry module
lowers successfully, its `RuntimeModuleId`. Runtime execution should use that module id to dispatch
`root()` instead of relying on the current global `entry_functions["root"]` map or on string lookup
through mixed source identities. `ResolvedModule` should keep runtime ids separate from typed module
provenance: source-provider modules carry normalized logical identities, while library modules
carry library root/module paths. The workspace builder may still include every analyzed workspace
module needed for imports and static analysis, but `eval_program_artifact` must execute the entry
module's `root()`.

File-backed program builds should also select an explicit normalized entry module identity once the
file provider has loaded the source set. Single-source helpers can provide a default entry identity,
but the resulting artifact should still use the same entry-module mechanism as workspace builds.

Alternative considered:
- Put the selected entry module first in `root_modules` and rely on first-wins global entry maps.

Why rejected:
- Ordering would be a hidden correctness rule and would still leave ambiguous behavior when
  multiple workspace modules define the same entrypoint name.

### 6. FFI descriptors are borrowed and validation-focused

The native ABI should add:

- `NxWorkspaceModule { identity_ptr, identity_len, source_utf8_ptr, source_utf8_len }`
- `nx_validate_workspace(build_context, modules, module_count, out_buffer)`
- `nx_build_workspace_program_artifact(build_context, modules, module_count, entry_identity,
  out_handle, out_buffer)`

FFI input validation should return `InvalidArgument` for malformed native inputs:

- null build context handles
- null module arrays when `module_count > 0`
- null identity/source pointers when their lengths are non-zero
- invalid UTF-8 in identities, sources, or entry identity
- malformed logical identities or duplicate normalized identities
- null output buffers or output handles

User-authored NX errors should serialize diagnostics into `out_buffer`. Validation should use
`Ok` with a serialized diagnostics array, where an empty array means the workspace is valid. Build
should follow the existing artifact-build convention: `Ok` plus a handle on success, `Error` plus
serialized diagnostics when static errors prevent an artifact, and `InvalidArgument` for malformed
FFI inputs.

Alternative considered:
- Return `Error` from validation whenever diagnostics are non-empty.

Why rejected:
- Validation is a diagnostic-producing query, not artifact construction. Returning `Ok` with the
  diagnostic array lets callers distinguish a successful validation request from interop failure.

### 7. Managed APIs own validation and pinning ergonomics

The .NET binding should add:

- `NxWorkspace`
- `NxWorkspaceModule`
- `NxRuntime.ValidateWorkspace(NxWorkspace, NxProgramBuildContext)`
- `NxProgramArtifact.BuildWorkspace(NxWorkspace, string entryIdentity, NxProgramBuildContext)`
- convenience string-based module factories that encode UTF-8 in managed code

The managed layer should reject null workspaces, null modules, null build contexts, and empty
identities before calling native code. It should not perform filesystem normalization. Module
descriptors and backing byte arrays are pinned only for the native call duration; the returned
artifact handle owns any data it needs afterward.

Alternative considered:
- Expose only string-based managed modules.

Why rejected:
- Nexara's primary input is already byte-oriented. String convenience overloads are useful, but the
  public core shape should avoid unnecessary conversions and ownership ambiguity.

### 8. Do not preserve backward compatibility as a design constraint

The implementation should prefer the unified module-graph model over preserving existing API shapes
or legacy behavior. Existing Rust, FFI, C header, and .NET APIs can be renamed, removed, or changed
when the old shape would force duplicate workspace/file-backed logic.

Alternative considered:
- Keep existing public APIs stable and add workspace support beside them.

Why rejected:
- That would likely preserve two program-building models indefinitely. This repository does not
  need backward compatibility yet, so the simpler long-term architecture should drive the change.

## Risks / Trade-offs

- [Resolver refactoring touches import analysis, diagnostics, and program assembly] -> Mitigation:
  introduce the shared module graph with explicit in-memory and filesystem source providers, then
  move both paths onto it with tests covering each provider.
- [Filesystem behavior can leak into in-memory semantics] -> Mitigation: keep all file IO and
  canonicalization inside the filesystem source provider and keep the shared resolver logical.
- [Workspace module imports may be confused with existing library imports] -> Mitigation: make
  workspace exact-identity resolution run first and document that unresolved imports continue to use
  `ProgramBuildContext` library semantics.
- [Breaking public APIs can disrupt local callers] -> Mitigation: document the new APIs and update
  repository consumers/tests in the same change rather than maintaining compatibility shims.
- [Multiple root modules expose existing global-entry ambiguity] -> Mitigation: add an explicit
  entry identity/module reference and tests where two modules define `root()`.
- [Diagnostic span conversion can accidentally re-read files] -> Mitigation: add source-map aware
  diagnostic conversion and tests using path-like identities that do not exist on disk.
- [FFI pinning bugs can retain caller-owned memory] -> Mitigation: construct owned Rust
  `NxWorkspaceModule` values before returning from build calls and test that artifacts remain
  executable after the managed buffers are released.

## Migration Plan

1. Add logical identity normalization and diagnostic source-map helpers in `nx-api`.
2. Add a shared module graph/resolver abstraction with in-memory and filesystem source-provider
   adapters.
3. Add `NxWorkspace`/`NxWorkspaceModule` Rust types and workspace validation/build functions over
   the shared module graph.
4. Move existing single-source and filesystem-backed program construction onto the shared module
   graph, changing public APIs where a cleaner unified shape requires it.
5. Extend `ProgramArtifact`/`ResolvedProgram` with explicit entry identity or entry module
   resolution for both in-memory and filesystem-backed builds.
6. Add FFI descriptors and native workspace validation/build exports.
7. Add managed workspace models, P/Invoke declarations, wrapper APIs, and tests.
8. Update generated C headers and binding documentation.
9. Migrate Nexara to build `NxWorkspace` values from effective config-file rows and remove the
   draft temp-directory materialization path.

## Open Questions

- Should a later change add extension fallback such as `./shared/questions` ->
  `shared/questions.nx`, or should workspace imports remain exact forever?
- Should component initialization from a workspace artifact also be scoped by entry identity for
  component names, or is root evaluation the only entrypoint that needs explicit scoping in this
  phase?
