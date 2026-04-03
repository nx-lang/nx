## Context

NX currently uses two different structural models at once:

- In `nx-hir`, a `Module` already means one lowered `.nx` source file.
- In `nx-api::prepared`, `PreparedLibrary` flattens many library files into one aggregate lowered
  module and then merges those items into source-driven runtime calls.

That split has three costs:

1. The terminology is inconsistent. `Module` is both the natural single-file HIR term and, in
   practice, part of a flattened multi-file runtime world.
2. Caching is too coarse at the library/runtime layer. Merging destroys per-file boundaries and
   forces re-analysis of merged results rather than preserving independently cacheable units.
3. The interpreter and snapshot model assume one flat lowered namespace. That blocks a cleaner
   program model where separate lowered source files remain separate after parsing and analysis.

The desired direction is:

- `SyntaxTree` remains the parse/CST artifact for one source file.
- `LoweredModule` becomes the explicit name for the lowered HIR of one source file.
- `ModuleArtifact` becomes the cached artifact bundle for one source file.
- `LibraryArtifact` becomes the cached artifact bundle for one library.
- `ProgramArtifact` becomes the cached artifact bundle for the fully resolved executable program,
  including standalone source files plus all dependent libraries.

This change is cross-cutting. It affects compiler data structures, library resolution, runtime
execution, snapshots, tests, FFI, .NET bindings, and documentation. It also intentionally changes
public terminology, so it needs an architectural design before implementation.

## Goals / Non-Goals

**Goals:**
- Rename the single-file lowered HIR type from `Module` to `LoweredModule`.
- Preserve file boundaries after lowering, analysis, and caching instead of flattening libraries
  into one merged lowered module.
- Introduce consistent cache/container terminology: `ModuleArtifact`, `LibraryArtifact`, and
  `ProgramArtifact`.
- Define a runtime-facing program model that lets the interpreter execute code across multiple
  lowered modules without relying on merged copies.
- Preserve or improve cache invalidation granularity by making per-file artifacts first-class.
- Keep `TypeEnvironment` as the analysis term for lexical name/type bindings and expression types.

**Non-Goals:**
- Introduce bytecode, transpiled output, or any compiled runtime format beyond the interpreted
  runtime structures needed to execute separate lowered modules.
- Redesign the NX language semantics for imports, visibility, or component behavior beyond what is
  required to support the artifact/runtime model.
- Solve remote library fetching, package versioning, or cross-process persistent cache storage.
- Preserve backward compatibility for the old `PreparedLibrary` terminology, merged-library
  internal architecture, or legacy prepared/runtime APIs.
- Rename `TypeEnvironment` to `TypeMap` or otherwise reframe the type-checker vocabulary in this
  change.

## Decisions

### 1. Rename the single-file lowered HIR type to `LoweredModule`

The existing `nx-hir::Module` type already represents one lowered source file, not a multi-file
program. The implementation and docs should make that phase and scope explicit by renaming it to
`LoweredModule`.

This keeps the important boundary visible:

- `SyntaxTree` is the parse result for one file.
- `LoweredModule` is the lowered HIR for one file.

Alternative considered:
- Keep `Module` as-is and use `LoweredModule` only informally in design/docs.

Why rejected:
- The current proposal is partly motivated by terminology drift. Keeping the old name in code would
  continue to blur the distinction between single-file lowered HIR and larger runtime structures.

### 2. Use `Artifact` terminology for cached containers, not for the core HIR type

The design introduces three explicit cache/container layers:

- `ModuleArtifact`: cached products for one source file
- `LibraryArtifact`: cached products for one library
- `ProgramArtifact`: cached products for one resolved executable program

The `Artifact` suffix means "cacheable derived products plus metadata," not "core syntax or HIR
node type." This keeps `LoweredModule` focused on code structure and keeps `*Artifact` focused on
analysis, fingerprints, diagnostics, dependencies, and invalidation.

Expected contents:

- `ModuleArtifact`
  - source identity / fingerprint
  - `SyntaxTree` or parse result metadata
  - `LoweredModule`
  - `TypeEnvironment`
  - diagnostics
  - import/dependency metadata

- `LibraryArtifact`
  - library root identity / fingerprint
  - `ModuleArtifact`s for all files in the library
  - export tables
  - dependency declarations
  - library-level diagnostics

- `ProgramArtifact`
  - root source set / entrypoints
  - resolved library dependencies
  - whole-program diagnostics and fingerprint
  - the runtime-facing resolved program structure used by the interpreter

Alternative considered:
- Keep `CompilationUnit` and `PreparedLibrary` rather than introducing `ModuleArtifact` and
  `LibraryArtifact`.

Why rejected:
- `CompilationUnit` is defensible but less aligned with the chosen NX vocabulary. The `Artifact`
  family makes the hierarchy easier to reason about once `LoweredModule` is the single-file HIR
  type.

### 3. Preserve separate lowered source files inside library and program artifacts

Libraries and programs should not be represented by copying many lowered modules into one aggregate
lowered module. Instead, file boundaries remain first-class all the way through caching and
resolution.

This is consistent with the existing `LibraryLoader` in `nx-hir::import_resolution`, which already loads a
library as separate per-file lowered modules before flattening visibility for resolution. The new
artifact model makes that file-preserving structure the primary architecture instead of an
implementation detail.

Alternative considered:
- Keep the current merged `PreparedLibrary` model and just rename types around it.

Why rejected:
- It would preserve the same cache invalidation and runtime-resolution problems while only changing
  labels.

### 4. Add a runtime-facing `ResolvedProgram` inside `ProgramArtifact`

The interpreter should not consume raw `ModuleArtifact`s directly and perform import/export
resolution ad hoc during evaluation. It should consume a narrower runtime structure,
`ResolvedProgram`, inside `ProgramArtifact` that precomputes the executable view of the program.

`ResolvedProgram` should provide:

- symbol lookup tables for functions, components, records, and enums
- resolved references that identify both the owning lowered module and the local item/expression
- the entrypoint mapping for evaluation/component lifecycle calls

`ProgramArtifact` remains the cache/container abstraction. `ResolvedProgram` is the interpreter-
facing resolved executable view nested inside it.

The interpreter then executes within one `LoweredModule` at a time and crosses module boundaries
through resolved references such as:

- function refs: `(module_id, item_ref)`
- expression refs: `(module_id, expr_id)`
- element refs: `(module_id, element_id)`

Alternative considered:
- Make the interpreter consume `ProgramArtifact` directly with no narrower runtime structure.

Why rejected:
- `ProgramArtifact` is the cache container. The interpreter needs a tighter executable abstraction
  rather than depending on the full cache layout.

### 5. Replace legacy prepared/runtime APIs directly instead of maintaining compatibility shims

This change should perform a direct API cutover. The old merged-library `PreparedLibrary` model and
its associated runtime entrypoints should be updated to the new artifact/program vocabulary and
runtime model rather than preserved behind adapters or compatibility layers.

The design goal is a simpler steady-state architecture, not a prolonged migration period where both
the old and new models coexist. If a caller-facing API needs to change to reflect `LoweredModule`,
`ModuleArtifact`, `LibraryArtifact`, or `ProgramArtifact`, this change should update that API
directly.

Alternative considered:
- Preserve old prepared/runtime APIs temporarily and route them through compatibility shims.

Why rejected:
- The repository does not need backward compatibility yet, and carrying two architectural models at
  once would add complexity right where this change is trying to simplify the system.

### 6. Make snapshots and captured handlers module-aware

The current runtime snapshot/handler model assumes a single lowered module namespace. Once
execution spans multiple `LoweredModule`s, any runtime object that points back into code must also
carry module identity.

This applies to:

- serialized component snapshots
- serialized/lazy action handlers
- any internal runtime references to expressions or elements

The design therefore requires replacing bare local expression references with module-qualified
references in runtime-owned serialized state.

Alternative considered:
- Keep snapshots opaque but continue encoding only local expr IDs and rely on merged program order.

Why rejected:
- That would silently reintroduce the merged-module assumption and make per-file execution models
  unsafe.

### 7. Keep `TypeEnvironment` terminology

`TypeEnvironment` should remain the name for the type-analysis structure. It is not just a flat map
of names to types; it models lexical scopes and expression-type bindings. Renaming it to
`TypeMap` would make the compiler terminology less accurate and does not materially improve the new
artifact model.

Alternative considered:
- Rename `TypeEnvironment` to `TypeMap` as part of the terminology cleanup.

Why rejected:
- The change goal is to clarify module/library/program artifact boundaries, not to rename stable
  type-checker concepts to less precise terms.

## Risks / Trade-offs

- Breaking rename across many crates and bindings → Mitigation: stage the rename through aliases or
  coordinated crate-by-crate updates in one change set, with tests updated in the same sequence.
- Interpreter migration from one flat lowered module to resolved multi-module execution is
  architecturally significant → Mitigation: introduce the resolved runtime structure first, then
  port entrypoints incrementally while preserving behavior tests.
- Snapshot format changes can invalidate stored host-owned component snapshots → Mitigation: bump
  snapshot version and treat old snapshots as incompatible rather than attempting silent migration.
- Separate-file artifacts can increase the amount of resolution metadata that must be maintained →
  Mitigation: centralize symbol/export indexes in `LibraryArtifact` and `ProgramArtifact` rather
  than scattering lookup logic through the interpreter.
- Direct API cutover increases the amount of caller-facing churn in one release →
  Mitigation: make the rename and runtime-model changes cohesive, update all first-party call sites
  together, and treat the change as intentionally breaking rather than partially compatible.

## Migration Plan

1. Rename the HIR type from `Module` to `LoweredModule` within `nx-hir` and propagate that rename
   through dependent crates.
2. Introduce `ModuleArtifact` in the analysis layer and teach the source-analysis pipeline to
   produce it from one source file.
3. Introduce `LibraryArtifact` using file-preserving library loading/indexing rather than merged
   aggregate lowered modules.
4. Introduce `ProgramArtifact` and its embedded `ResolvedProgram` for whole-program execution.
5. Update interpreter internals to resolve and execute across multiple `LoweredModule`s through the
   new runtime-facing program structure.
6. Update snapshot and action-handler serialization to use module-aware runtime references.
7. Replace `PreparedLibrary`-based merged-library flows in `nx-api`, `nx-ffi`, and .NET bindings
   with program/library artifact terminology and runtime plumbing in the same cutover.
8. Remove the legacy merged prepared-library infrastructure as part of the implementation rather
   than preserving it behind compatibility shims.

Rollback strategy:
- Revert the change set and restore the prior merged-library implementation. The design does not
  rely on a dual-path compatibility period.
