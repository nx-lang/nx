## Context

NX currently has the pieces for file-preserving caching, but the ownership boundaries are still off.
`LibraryArtifact` exists, yet the last proposal still centered caching on `ProgramBuildContext`,
which is too narrow for real server workflows. Hosts often need to load and analyze shared
libraries at startup, or on demand later, before there is any tenant-specific program to execute.

There are three architectural problems underneath the current design:

1. Library snapshots do not have a durable host-owned registry that exists independently of a
   particular program build.
2. Build-time import resolution still performs ad hoc filesystem loading instead of resolving
   against an explicit library snapshot graph.
3. Analysis still relies on temporary copied imported HIR during import resolution, which creates
   duplicated item/expr IDs that are thrown away after diagnostics and type checking.

The intended workflow is:

- The host creates a long-lived `LibraryRegistry`.
- The host loads and analyzes one or more local NX libraries into that registry, potentially before
  any program exists.
- The host creates a cheap `ProgramBuildContext` for one tenant or build scope.
- The host builds transient `ProgramArtifact`s from fresh source against the libraries visible
  through that build context.
- Runtime evaluation and component lifecycle calls continue to execute against those transient
  `ProgramArtifact`s.

This is a breaking API change across Rust, FFI, and .NET. The repository does not need backward
compatibility yet, so the design should move directly to the simpler steady-state model rather than
carrying standalone library handles or a context-owned cache in parallel.

This change also folds in a smaller language-surface update that already exists in unstaged code:
enum members are now written in `snake_case` by convention in first-party NX source, docs, tests,
formatting, and value-conversion examples. That convention is part of this change, but its work is
verification-oriented rather than a fresh implementation task.

## Goals / Non-Goals

**Goals:**
- Introduce a reusable `LibraryRegistry` that owns analyzed `LibraryArtifact` snapshots and their
  dependency graph outside the lifetime of any single program.
- Make `ProgramBuildContext` a cheap per-build or per-tenant view over that registry rather than
  the owner of loaded libraries.
- Allow hosts to load and analyze libraries even when no `ProgramArtifact` exists yet.
- Make program construction resolve imports through registry-backed snapshot metadata instead of
  reloading libraries from disk for each build.
- Replace copied imported HIR with a registry-backed semantic resolution model.
- Preserve `ProgramArtifact` as the runtime execution boundary for eval and component lifecycle
  operations.
- Ensure multiple transient `ProgramArtifact`s can reuse the same selected library snapshots without
  duplicating library artifact state.
- Update Rust, FFI, and .NET APIs directly to the new context-based model.
- Capture the enum-member `snake_case` convention in the change artifacts and verify the already
  present code/docs/test updates that implement it.

**Non-Goals:**
- Add filesystem watching, automatic invalidation, or hot reload for loaded libraries.
- Add cross-process or on-disk persistent caches.
- Change NX import syntax or remote-library semantics.
- Enforce enum-member casing as a parser-only restriction; this change documents the convention and
  preserves source spelling, but does not add a separate syntax gate.
- Redesign interpreter execution away from `ProgramArtifact` / `ResolvedProgram`.
- Preserve the standalone `NxLibraryArtifact` host API for compatibility.
- Introduce hierarchical or parent-linked registries in the first implementation; a single
  reusable registry plus per-build visibility controls is enough for this phase.

## Decisions

### 1. Expose `LibraryRegistry` as the public owner of analyzed library snapshots

The public host-facing abstraction for cached libraries will be `LibraryRegistry`. It owns the
process- or host-lifetime store of analyzed `LibraryArtifact` snapshots and the dependency graph
between those snapshots.

`LibraryRegistry` will:

- own cached library snapshots keyed by canonical library root path
- allow hosts to load and analyze one or more root libraries even when no program exists yet
- track the dependency closure between loaded snapshots
- assign internal snapshot identity where needed for dependency graph bookkeeping

Alternative considered:
- Keep `ProgramBuildContext` as the owner of loaded libraries.

Why rejected:
- Real hosts need to preload and analyze shared libraries before any program build exists. Making
  build contexts own the cache ties library lifetime to the wrong abstraction.

### 2. Keep `ProgramBuildContext` as a registry-backed build scope, not as the cache owner

`ProgramBuildContext` remains part of the design, but as a cheap per-build or per-tenant view over
one `LibraryRegistry`. It defines which loaded libraries are visible when constructing a
`ProgramArtifact`; it does not own those snapshots.

This allows:

- one shared registry for server-wide libraries
- many short-lived build contexts for different tenants or requests
- per-build visibility control without duplicating the library cache

Alternative considered:
- Build programs directly from the registry with no intermediate build context.

Why rejected:
- The build context remains useful as the boundary for tenant visibility, build-specific selection,
  and future policy knobs. It should exist, but it should not own the libraries.

### 3. Keep `LibraryArtifact` as a local snapshot plus interface metadata, not as a recursive graph

`LibraryArtifact` should remain a snapshot of one library root only. It should preserve:

- local `ModuleArtifact`s for that library
- export metadata
- dependency roots
- diagnostics
- fingerprint
- interface metadata needed by dependent analysis, such as function signatures, value types,
  component prop/state/emits shapes, record shapes, enum members, and other exported type data

It should not own recursive `Arc<LibraryArtifact>` dependencies or context-local dependency IDs.
Those belong in `LibraryRegistry`, which owns the snapshot graph.

Alternative considered:
- Store dependent `LibraryArtifact` references directly inside each `LibraryArtifact`.

Why rejected:
- Recursive ownership blurs snapshot versus graph responsibilities, complicates deduplication, and
  leaks registry-specific identities into the artifact model.

### 4. Replace copied imported HIR with registry-backed semantic resolution

The current `resolve_local_library_imports` implementation copies visible imported items into a
temporary working `LoweredModule` for analysis. That is acceptable as a transition mechanism, but
it is not the right long-term model. The new design will resolve imports through registry-backed
library interfaces and transient resolved-import tables instead of by deep-copying foreign-library
HIR into the importing module.

Analysis of one module or library will therefore operate over:

- the module's local `LoweredModule`
- local library interfaces from the same `LibraryArtifact`
- imported library interfaces from `LibraryRegistry`

The stored `ModuleArtifact` and `LibraryArtifact` remain file- and library-local; only the
transient analysis context sees imported dependency interfaces.

This also fixes the crate boundary: `nx-hir` remains responsible for lowering only, `nx-api`
owns registry/build-context import preparation, and `nx-types` analyzes a caller-prepared
`LoweredModule`. The old public disk-loading `resolve_local_library_imports` helper is removed
rather than carried forward as a legacy API.

Alternative considered:
- Keep the current `ModuleCopier`-based imported-item copying approach.

Why rejected:
- It duplicates HIR, allocates fresh local expr/element IDs for imported items, and then throws
  those copied structures away once diagnostics and type inference complete.

### 5. Distinguish library snapshot IDs from runtime module IDs

`LibraryRegistry` may assign internal snapshot IDs for dependency-graph bookkeeping, but those IDs
are build-time graph identities only. The interpreter's runtime IDs remain program-specific and are
assigned only when constructing a `ResolvedProgram` for one `ProgramArtifact`.

This preserves the correct boundary:

- registry snapshot IDs identify analyzed libraries in a reusable cache
- runtime module IDs identify executable modules inside one specific program revision

Alternative considered:
- Assign runtime module IDs at library load time and reuse them across programs.

Why rejected:
- The interpreter executes a specific resolved program, not an arbitrary cached library. Runtime
  IDs must remain scoped to that program's executable closure.

### 6. `ProgramArtifact` preserves the selected registry snapshots and remains the runtime boundary

`ProgramArtifact` remains the artifact consumed by runtime evaluation and component lifecycle
entrypoints. It should preserve shared references to the exact library snapshots selected from the
registry through the active `ProgramBuildContext`, along with the `ResolvedProgram` built from
those snapshots.

This keeps runtime execution artifact-based while allowing repeated program builds to reuse
registry-owned library snapshots.

Alternative considered:
- Make runtime entry points consume `LibraryRegistry` or `ProgramBuildContext` directly.

Why rejected:
- Runtime execution already has the correct abstraction in `ProgramArtifact`. Registry and build
  context are build-time concerns, not interpreter-facing runtime dependencies.

### 7. Replace standalone library-handle FFI/.NET APIs with registry and build-context handles

The FFI and .NET layers should expose:

- create/free library registry
- load/analyze library into registry
- create/free build context from a registry
- build program artifact from source using a build context

Managed code will mirror that surface with disposable `NxLibraryRegistry` and
`NxProgramBuildContext` APIs.

Alternative considered:
- Keep the existing library-artifact handle and add a second set of context APIs beside it.

Why rejected:
- The current library-artifact host surface is incomplete and redundant once a build context exists.
  Keeping both would preserve the confusion this change is meant to remove.

### 8. Route source analysis and component convenience APIs through the same registry-backed resolver

Today, source analysis and program building still perform their own filesystem-based library
loading paths. This change will centralize import resolution behind the registry plus build-context
resolver so that shared analysis, `ProgramArtifact` construction, and source-based runtime
convenience APIs all consult the same library snapshot set.

Alternative considered:
- Keep the existing filesystem-based import loader for analysis and add registry lookup only at the
  final `ProgramArtifact` assembly layer.

Why rejected:
- That would let analysis and program construction disagree about the imported library universe and
  would preserve duplicated loading logic.

### 9. Make prepared-module analysis the shared seam between `nx-api` and `nx-types`

The shared analysis seam should be "analyze this prepared lowered module," not "analyze this
source string by loading imports from disk." `nx-api` already owns build-context visibility,
library registry state, and import normalization policy, so it should also own the transient
module preparation step that synthesizes imported interfaces for analysis.

`nx-types` should therefore expose a prepared-module analysis entry point and remain free of
filesystem or registry import-loading policy. The convenience source/file entry points in
`nx-types` may still parse and lower source, but they should not perform disk-backed library
resolution themselves.

Alternative considered:
- Teach `nx-types` about `ProgramBuildContext` directly.

Why rejected:
- That would invert the dependency direction, duplicate import policy in a lower-level crate, and
  make the core type checker responsible for build-time registry concerns.

### 10. Document enum-member naming as a `snake_case` convention and preserve exact spelling

First-party NX examples, fixtures, grammar tests, runtime formatting, and host value conversion
should use `snake_case` enum members by convention rather than `PascalCase`. The underlying
language model should preserve the exact member spelling written in source through lowering, type
analysis, runtime values, formatting, and host-facing value conversion rather than rewriting the
member name to another case.

This part of the change is intentionally verification-oriented. The repository already has
corresponding unstaged code, test, and doc edits; the OpenSpec artifacts need to capture that
language-facing behavior and require a correctness pass over those existing changes.

Alternative considered:
- Leave the enum-member casing changes out of this change because the code was updated separately.
- Turn the convention into a new parser-only casing restriction.

Why rejected:
- The code already changes user-facing behavior, examples, and tests, so the change artifacts
  should document it.
- The intent is to standardize first-party usage and preserve source spelling, not to invent a new
  parser-only rule.

## Risks / Trade-offs

- [Hosts must preload imported libraries explicitly] → Mitigation: make registry-load and
  missing-library diagnostics name the normalized root and document the `LibraryRegistry` plus
  `ProgramBuildContext` workflow clearly.
- [Loaded snapshots can become stale if files change on disk] → Mitigation: keep snapshots
  immutable and require explicit reload into the registry rather than hidden invalidation.
- [Library analysis still needs same-library peer visibility during transient checking] →
  Mitigation: keep peer-library visibility as a transient prepared-module concern in `nx-api`
  while limiting imported dependency libraries to synthesized interface items only.
- [Registry ownership and build-scope visibility can be confused] → Mitigation: keep the API
  naming explicit: registry owns snapshots, build context selects visible snapshots, program
  artifact executes them.
- [The change is breaking across Rust, FFI, and .NET] → Mitigation: cut over the first-party code,
  tests, and docs in one coordinated change rather than carrying shims.
- [Program artifacts need to remain executable after build context release] → Mitigation: keep
  shared snapshot refs or equivalent resolved-program data on `ProgramArtifact` itself.
- [Enum-member convention could be mistaken for a hard syntax ban] → Mitigation: document clearly
  that `snake_case` is the first-party convention and that runtime/tooling preserve exact source
  spellings rather than normalizing them.

## Migration Plan

1. Introduce `LibraryRegistry` in `nx-api` as the owner of analyzed library snapshots and their
   dependency graph.
2. Extend `LibraryArtifact` to publish the interface metadata needed for dependent analysis while
   remaining a local snapshot with dependency metadata only.
3. Refactor import resolution and analysis to resolve imports through registry-backed interface
   metadata and transient resolved-import tables instead of copied imported HIR, and remove the
   old public disk-loading import-resolution surface from `nx-hir`.
4. Introduce `ProgramBuildContext` as a registry-backed build scope that selects visible libraries
   for one program build or tenant.
5. Add registry- and build-context-backed program-building entry points and update
   `ProgramArtifact` to preserve the selected library snapshots and program-specific runtime IDs.
6. Update `nx-types` to analyze caller-prepared modules and move source-import preparation policy
   up into `nx-api`.
7. Update source-based evaluation and component lifecycle convenience APIs to use the shared
   registry-backed resolver.
8. Update FFI to expose registry and build-context handles and remove standalone library-artifact
   host handles.
9. Update the .NET binding to expose `NxLibraryRegistry` and `NxProgramBuildContext`.
10. Refresh tests and docs to use the explicit preload/analyze/build/execute workflow.
11. Verify the already-present enum-member `snake_case` changes across parser/lowering, typing,
    runtime formatting, host value conversion, examples, grammar tests, and docs, and adjust the
    captured design notes if verification finds gaps.

Rollback strategy:
- Revert the change set and restore per-build filesystem library loading. No mixed compatibility
  layer is planned in this design.

## Open Questions

- None. This design intentionally chooses explicit registry ownership and registry-backed analysis
  over standalone library handles or copied imported HIR.
