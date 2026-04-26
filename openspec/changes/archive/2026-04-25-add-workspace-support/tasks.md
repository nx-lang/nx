## 1. Shared Module Graph And Identity Normalization

- [x] 1.1 Add public Rust `NxWorkspace` and `NxWorkspaceModule` types in `nx-api` with source UTF-8 byte payloads and export them from the crate root.
- [x] 1.2 Implement a logical workspace identity normalizer that uses forward slashes, folds `.` and `..`, rejects empty/absolute/escaping identities, and never calls filesystem APIs.
- [x] 1.3 Add unit tests for identity normalization, duplicate normalized identities, root-escaping identities, and path-like identities that do not exist on disk.
- [x] 1.4 Introduce an internal logical module graph/source-provider abstraction that can be populated from in-memory workspace modules or filesystem-backed source files.
- [x] 1.5 Add a source-map aware diagnostic conversion path that resolves spans from provider-supplied source text before any file-backed fallback.
- [x] 1.6 Add tests proving in-memory and filesystem-backed providers can feed equivalent module identities and source text into the shared graph.

## 2. Shared Analysis And Import Resolution

- [x] 2.1 Implement workspace indexing that validates identity/source UTF-8, owns decoded source text for the call, and reports duplicate or malformed workspace identities.
- [x] 2.2 Implement a filesystem source provider that loads NX source files into the same logical module graph used by in-memory workspaces.
- [x] 2.3 Parse and lower all loaded modules with normalized identities while preserving parse-failure module records and diagnostics.
- [x] 2.4 Add a shared resolver that resolves relative imports against the importing module identity, prefers loaded graph module matches, and then falls back to `ProgramBuildContext` library lookup.
- [x] 2.5 Prepare loaded modules with peer visibility, graph import visibility, and existing build-context library interfaces.
- [x] 2.6 Implement `validate_workspace` so it analyzes the effective workspace once and returns aggregated diagnostics across all submitted modules.
- [x] 2.7 Move existing single-source and file-backed analysis/build callers onto the shared module-graph path, changing public API shapes where that keeps the design simpler.
- [x] 2.8 Add Rust tests for valid multi-module workspaces, duplicate identities, invalid source bytes, missing imports, root-escaping imports, relative imports, multi-module diagnostics, and equivalent file-backed import behavior.

## 3. Workspace Program Artifacts And Runtime Entrypoints

- [x] 3.1 Implement `build_workspace_program_artifact` with entry identity normalization, missing-entry diagnostics, static-error gating, selected libraries, and workspace-derived fingerprinting.
- [x] 3.2 Extend `ProgramArtifact` and/or `ResolvedProgram` to record the selected entry identity or entry module ID explicitly for every source provider.
- [x] 3.3 Update `eval_program_artifact` and interpreter entrypoint lookup so artifacts execute `root()` from the selected entry module.
- [x] 3.4 Rework single-source and file-backed program build behavior to use explicit entry identity and the shared module graph, without adding compatibility shims solely for old API behavior.
- [x] 3.5 Add Rust tests for two workspace modules defining `root()`, entry module without `root()`, execution across workspace imports, equivalent file-backed entry identity behavior, and artifacts remaining executable after the build context is released.

## 4. Native FFI And C Header

- [x] 4.1 Add a C-compatible `NxWorkspaceModule` descriptor to `nx-ffi` and include it in the generated C header.
- [x] 4.2 Add `nx_validate_workspace` with descriptor parsing, pointer validation, UTF-8 validation, serialized diagnostics output, and `Ok` status for successful validation requests.
- [x] 4.3 Add `nx_build_workspace_program_artifact` with descriptor parsing, explicit entry identity parsing, artifact-handle ownership, and existing build-error diagnostic conventions.
- [x] 4.4 Ensure FFI build calls copy workspace data into NX-owned structures before returning and never retain caller-owned pointers.
- [x] 4.5 Bump the native ABI version, regenerate `bindings/c/nx.h`, and update FFI smoke tests for valid workspaces, invalid pointers, invalid UTF-8, static diagnostics, and post-call artifact evaluation.

## 5. Managed .NET Binding

- [x] 5.1 Add public `NxWorkspace` and `NxWorkspaceModule` managed types with `IReadOnlyList<NxWorkspaceModule>` and `ReadOnlyMemory<byte>` source payloads.
- [x] 5.2 Add managed interop declarations and descriptor packing for workspace validation and workspace program artifact builds.
- [x] 5.3 Add `NxRuntime.ValidateWorkspace(NxWorkspace, NxProgramBuildContext)` returning `IReadOnlyList<NxDiagnostic>`.
- [x] 5.4 Add `NxProgramArtifact.BuildWorkspace(NxWorkspace, string entryIdentity, NxProgramBuildContext)` and convenience string-to-UTF-8 module creation helpers.
- [x] 5.5 Validate nulls and empty identities in managed code before invoking FFI, and pin descriptors and byte buffers only for the native call duration.
- [x] 5.6 Add .NET tests for byte-backed modules, validation diagnostics, duplicate identities, missing entries, entry-scoped `root()`, and artifact evaluation after managed buffers are released.

## 6. Documentation And Verification

- [x] 6.1 Update Rust and .NET API documentation to describe logical workspace identities, source providers, exact workspace import identity matching, diagnostics mapping, validation status conventions, and any breaking API changes.
- [x] 6.2 Add binding documentation or examples showing how database/editor rows map to `NxWorkspaceModule` identities and source bytes without temporary files.
- [x] 6.3 Update repository callers and tests to the new API shapes instead of preserving backward-compatible wrappers.
- [x] 6.4 Run the relevant Rust test suites for `nx-api`, `nx-ffi`, `nx-interpreter`, and any affected lower crates.
- [x] 6.5 Run the managed .NET runtime tests under `bindings/dotnet`.
- [x] 6.6 Run OpenSpec validation/status for `add-workspace-support` and fix any artifact or requirement format issues.
