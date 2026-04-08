## Context

NX already has the rough shape of a multi-phase analysis pipeline:

- raw lowering in `nx-hir`
- same-library peer visibility in `nx-api`
- build-context import visibility from `LibraryRegistry` snapshots
- scope and type analysis over a prepared view
- whole-program assembly into `ProgramArtifact` and `ResolvedProgram`

The immediate record-inheritance fix solves one regression, but the deeper architectural problem is
that NX currently has no single binding model. Different phases answer “what does this name mean?”
through different mechanisms:

- lowering keeps a temporary local `Name -> TypeTag` map
- top-level prepared lookup uses `LoweredModule::find_item()`
- lexical scopes use a partial `ScopeManager`
- type checking uses `TypeEnvironment` plus separate alias and enum maps
- runtime uses `ResolvedProgram` entry and import tables

Prepared analysis currently works by cloning or synthesizing foreign HIR items into transient
`LoweredModule`s so later phases can continue using `find_item()`. That preserves momentum, but it
means the semantic model is encoded indirectly in copied HIR instead of in explicit bindings with
stable origins.

This change touches `nx-hir`, `nx-types`, and `nx-api`, and it affects both shared source analysis
and library/program artifact construction. Because the target architecture also changes how runtime
lookup is assembled, it reaches into `nx-interpreter` as well.

## Goals / Non-Goals

**Goals:**
- Make raw lowering explicitly file-local and free of validations that depend on peer visibility,
  imports, or prepared visible-name lookup.
- Introduce an explicit `PreparedModule` boundary for the analysis pipeline instead of passing an
  ad hoc second `LoweredModule` or encoding visibility through copied HIR.
- Introduce one shared top-level binding model that distinguishes definitions from visible symbol
  bindings and records the origin of each visible name.
- Centralize name-resolution-dependent semantic validation, lexical scope resolution, and type
  analysis on top of that shared binding model.
- Replace string-based top-level rescans in runtime assembly with module-qualified definition
  references derived from the same symbol model.
- Preserve the existing file-preserving artifact model for `ModuleArtifact`, `LibraryArtifact`, and
  `ProgramArtifact`, even while the binding model becomes explicit.
- Keep standalone analysis helpers usable by having them internally prepare a trivial prepared
  module and binding table when no library context exists.

**Non-Goals:**
- Flattening a library into one merged lowered module.
- Changing library-root, export, or build-context semantics.
- Solving every future semantic rule immediately; this change establishes the binding architecture
  and migrates the existing name-resolution-dependent consumers onto it.
- Redesigning `LibraryRegistry` or `ProgramBuildContext`.

## Decisions

### 1. Separate definitions from visible bindings

Raw lowering will continue to produce one file-local `LoweredModule`. That module owns definitions
only: items, expressions, elements, imports, and local source spans. It does not own the prepared
visible namespace.

Prepared analysis will introduce an explicit binding layer:

- stable local definition identities for top-level declarations in one raw module
- visible symbol bindings for prepared names
- origin metadata describing whether a binding targets a local definition, a same-library peer
  definition, or an imported library interface definition

Alternative considered:
- Keep using copied/synthesized HIR items as the prepared visible namespace.

Why rejected:
- Copied HIR encodes visibility indirectly, duplicates data, and gives each phase a different
  mechanism for rediscovering names rather than a stable binding identity.

### 2. `PreparedModule` becomes the shared analysis surface

Prepared-module analysis already exists behaviorally; this change makes it explicit. The prepared
analysis surface should carry:

- preserved raw module
- local definition table for that module
- prepared visible namespace and binding tables, organized by the namespaces NX cares about
  (`type`, `value`, and any element-like lookup NX keeps distinct)
- enough origin metadata for diagnostics and runtime assembly to recover the owning module and
  local definition identity of a binding target

`PreparedModule` should replace the current practice of passing two unrelated `LoweredModule`
values around and calling whichever one seems appropriate.

This boundary should be used by:

- standalone source-analysis helpers
- library artifact construction
- program root analysis with `ProgramBuildContext`

Alternative considered:
- Add a wrapper type but keep prepared visibility represented as a cloned `analysis_module`.

Why rejected:
- A renamed copy-based wrapper is still a copy-based wrapper. The point of the change is to make
  binding identity explicit, not just to hide the clone behind a struct.

### 3. Raw lowering becomes strictly file-local again

`lower()` will own syntax-to-HIR normalization and declaration checks that can be answered from one
source file only. Checks that depend on peer files, imported library interfaces, alias chains, or
prepared visible-name lookup will no longer run during lowering.

This means the current `lower_without_record_validation()` split is transitional and should be
removed. The stable contract becomes:

- lowering produces raw file-local definitions
- preparation constructs bindings for the visible namespace
- prepared-module validation, scope building, and type checking consume those bindings

Alternative considered:
- Keep the current default lowering behavior and preserve specialized opt-out helpers for binding-
  aware callers.

Why rejected:
- That would keep the wrong default in place and let new name-resolution-dependent logic regress
  back into raw lowering.

### 4. Prepared bindings replace `find_item()` as the top-level resolver

Top-level semantic consumers should stop discovering declarations by rescanning HIR items by name.
Instead, prepared-module APIs should resolve visible names to stable bindings. This affects:

- record inheritance and record-shape resolution
- alias and enum resolution during type checking
- function/component/record lookup for element typing
- library and program import/export assembly

`LoweredModule::find_item()` can remain as a raw-module convenience for file-local introspection,
tests, or migration shims, but it should no longer be the authoritative prepared lookup path.

Alternative considered:
- Leave `find_item()` as the main resolver and build richer ad hoc side maps in each consumer.

Why rejected:
- That preserves the current fragmentation, where each phase builds its own partial symbol model and
  the architecture remains difficult to reason about.

### 5. Lexical scopes layer over prepared top-level bindings

NX already has a `ScopeManager`, but it is partial and not the authoritative resolver. This change
should make lexical scope resolution a real layer that composes with prepared top-level bindings:

- params, `let` bindings, and loop bindings resolve lexically
- top-level names are resolved through prepared bindings when lexical lookup misses
- undefined-name diagnostics and shadowing behavior are produced from that unified model

`TypeEnvironment` should remain responsible for inferred types, not for declaration discovery.
Similarly, type-checker side maps for aliases and enums should shrink toward caches keyed by binding
or definition identity rather than becoming the source of truth for resolution.

Alternative considered:
- Keep `ScopeManager`, `TypeEnvironment`, and top-level `find_item()` as separate independent
  resolution systems.

Why rejected:
- The whole point of the change is to stop asking different questions to different tables depending
  on which phase happens to be running.

### 6. Runtime assembly consumes module-qualified definition references

`ResolvedProgram` already stores module-qualified runtime references, but item lookup still depends
on visible names and module rescans. This change should strengthen runtime references so they
identify the owning module plus the stable local definition identity of the target declaration.

That means:

- entry/import tables are built from prepared bindings and their target origins
- runtime item references stop depending on `item_name` string rescans
- the interpreter resolves top-level items by module-qualified definition reference

Expression and element references can keep their existing local arena identities as long as they are
already module-qualified and remain stable within the owning raw module.

Alternative considered:
- Keep runtime assembly string-based and limit the binding architecture to static analysis only.

Why rejected:
- That would preserve two different cross-module resolution models, one static and one runtime, and
  the runtime would continue paying the cost of rediscovering symbols that analysis already
  resolved.

### 7. `LibraryArtifact` persists raw snapshots plus library-owned indexes

Loaded libraries should persist only durable, reusable library facts. A `LibraryArtifact` should
continue to be file-preserving and should hold:

- one raw `ModuleArtifact` per source file
- stable local definition identities for the library's own top-level declarations
- library-owned export and library-visible binding indexes that map visible names to those stable
  definition identities or published interface origins
- interface metadata needed by dependent analysis
- dependency roots, diagnostics, and fingerprint metadata

It should not persist:

- per-caller `PreparedModule`s
- build-context-specific visible namespaces
- copied peer/imported HIR used only for one analysis request
- program-scoped runtime module IDs

This gives `LibraryRegistry` a reusable snapshot that can serve many prepared-analysis requests
without freezing any one caller's prepared view into the loaded artifact itself.

Alternative considered:
- Persist prepared modules for each library file inside `LibraryArtifact`.

Why rejected:
- Prepared modules are caller-relative and transient by design. Persisting them would blur the
  boundary between durable raw snapshots and one particular analysis context, and it would
  reintroduce copy-based HIR storage as part of the loaded library format.

### 8. Convenience helpers prepare trivial modules internally

Callers that analyze one source file without library context should not need to learn all of this
plumbing. Shared helpers such as `analyze_str`, `check_str`, and similar wrappers should perform:

1. parse
2. raw lowering
3. trivial prepared-module and binding construction with no peer/import augmentation
4. prepared-module validation
5. scope and type analysis

This keeps the public ergonomics small while preserving the correct architectural ordering.

Alternative considered:
- Require every caller to construct prepared modules explicitly.

Why rejected:
- It would push internal pipeline complexity onto simple callers that do not benefit from the extra
  ceremony.

## Risks / Trade-offs

- [A new binding model touches every analysis layer] -> Mitigation: keep the initial prepared
  binding API intentionally small, migrate the highest-value consumers first, and use compatibility
  shims only where needed to keep the refactor moving.
- [Changing `lower()` semantics can break tests or callers that expected record diagnostics during
  lowering] -> Mitigation: make the new contract explicit, update tests to assert those diagnostics
  through prepared-module analysis, and remove the transitional helper in the same change.
- [Binding identity design can sprawl if it tries to solve every use case at once] -> Mitigation:
  define stable top-level definition identities first and keep the first implementation focused on
  top-level declarations, lexical scopes, and runtime item lookup.
- [Migrating runtime lookup raises compatibility risk for serialized state] -> Mitigation: preserve
  the existing module-qualified guarantees while upgrading local item references in one coordinated
  change to `ResolvedProgram` and interpreter serialization.
- [Some consumers may continue calling `find_item()` accidentally] -> Mitigation: introduce shared
  prepared-resolution helpers early and migrate direct `find_item()` call sites explicitly.

## Migration Plan

1. Add stable local definition identities and an explicit prepared-module/binding abstraction.
2. Change raw lowering so it no longer performs name-resolution-dependent validation.
3. Populate prepared bindings from local declarations, same-library peers, and imported library
   interfaces without cloning foreign HIR into analysis modules.
4. Refactor `LibraryArtifact` so it persists raw modules plus stable definition/binding indexes and
   published interface metadata, but not prepared modules.
5. Migrate prepared semantic validation, lexical scope building, and type analysis to resolve
   through prepared bindings instead of `find_item()` and ad hoc side maps.
6. Update runtime assembly and interpreter lookup to consume module-qualified definition references.
7. Remove transitional record-specific lowering helpers and remaining copy-based prepared-resolution
   code once all consumers are migrated.

## Open Questions

No blocking design questions. The implementation will still need to choose exact type names for the
prepared binding and definition-reference structs, but the architectural direction is fixed: raw
definitions stay file-local, visible names become bindings, and all later phases consume those
bindings instead of cloned HIR or repeated string lookup.
