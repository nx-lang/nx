## Context

See proposal.md for motivation. What the design builds on:

- `export_update` in `crates/nx-cli/src/typegen/model.rs` copies the target's effective fields
  from `nx_hir::effective_record_shape`, whose `EffectiveField.module_identity` names the module
  that declared each field. Today only the field *names* of foreign-module fields are kept
  (`ExportedUpdate.inherited_from_other_modules`), and `warn_about_unresolvable_inherited_fields`
  uses them to warn.
- A module identity is the module's source file name: an absolute path for a library module. A
  `LibraryArtifact` keys `namespaces` (each module's resolved `ModuleNamespace`) and `sources` by
  that identity, and every `LibraryInterfaceItem` carries its `module_identity` and
  `definition_id`. `ModuleNamespace::entry(PreparedNamespace::Type, name)` answers "what does this
  type name mean in this module" with a `DeclaringOrigin` (module identity + definition id),
  following the module's own imports and aliases.
- Both emitters resolve a type name in two steps: `graph.declaration(name)` for something this
  output declares, then `imported_types_by_visible_name` for an `ImportedType` (visible name,
  exported name, library name, kind). C# renders an imported type as
  `global::<assumed dependency namespace>.<ExportedName>`; TypeScript adds
  `import type { ExportedName as <sanitized visible name> } from "<assumed package>"`. Neither
  emitter looks at anything else, so a synthesized `ImportedType` plus a matching visible name in
  the field's `TypeRef` is enough for both.
- `ImportedTypeCollector` owns the `LibraryRegistry` that loads every imported library, caches
  each dependency by canonical root as a `CachedImportedLibrary` (library name, export kinds,
  alias targets), and is alive in both `from_artifact_with_warnings` and
  `from_library_with_warnings` until the graph is built.
- The three test gaps in the proposal are covered by existing scenarios in `update-records`,
  `nx-ir-format`, and `dotnet-binding`; no spec text changes for them.

## Goals / Non-Goals

**Goals:**
- Generated companions compile when an inherited field's type comes from a dependency, for both
  target languages, from a single model-level resolution pass.
- The resolution mirrors what an explicit `import` of the type in the generating module would
  produce, so generated output looks the same whichever way the author reached the type.
- Retire the interim warning path rather than keeping two mechanisms.

**Non-Goals:**
- Publishing real package or namespace metadata for dependencies; the assumed sibling namespace
  and assumed package name stay as they are.
- Cross-library references to a dependency's `<T>_update` companion (a base field typed
  `Other.Update` where `Other` lives in the dependency). `build_cached_imported_library` skips
  update records today, and `rewrite_update_type_references` already warns for them; this change
  leaves that as is.
- Composing TypeScript companions from the base's companion (follow-up item 3), rejected because
  C# has no equivalent and the emitters should agree on where inherited fields come from.

## Decisions

### D1. Fields carry their declaring module; the update-only name list goes away

`ExportedRecordField` gains `declaring_module: Option<String>`, `None` for a field declared in the
module that owns the exported declaration and `Some(identity)` otherwise. `export_update` fills it
from `EffectiveField.module_identity`; every other constructor leaves it `None`.
`ExportedUpdate.inherited_from_other_modules` is removed, since the field now says it directly.

Alternative: keep the name list and look the field up by name. Rejected: the type resolution needs
the identity per field anyway, and two representations of one fact drift.

### D2. Resolve in the model by synthesizing imported types, not in the emitters

A new pass, run on the `Vec<ExportedModule>` after declarations are collected and before
`build_from_exported_modules`, walks every `ExportedType::Update` field with a declaring module
and, for each non-primitive type name in its `TypeRef`:

1. Looks the name up in the declaring module's `ModuleNamespace` under `PreparedNamespace::Type`,
   falling back to `PreparedNamespace::Element` for a field typed by a component, which only the
   element namespace reaches.
2. Maps the resulting `DeclaringOrigin` to the library that owns that module identity and to that
   library's interface item with the same module identity and definition id, which gives the
   exported name, visibility, and kind.
3. If the origin library is the one being generated (library mode, a peer module), rewrites the
   name to the origin's exported name; the graph already declares it and the emitters' existing
   cross-module handling (TypeScript relative import, C# same namespace) covers the rest. An
   import in the generating module whose visible name is that exported name would win over the
   graph in both emitters, so that case warns instead of writing the bare name.
4. If the origin is an exported, cross-library-referenceable item of a dependency, chooses a
   visible name (D3), adds an `ImportedType` for it to the generating module's `imported_types` if
   none exists, and rewrites the name in the field's `TypeRef` to that visible name.
5. Otherwise, warns as the spec describes and leaves the name as written.

Why here: `ImportedType` is exactly the shape both emitters already consume, so the pass adds no
emitter code and both languages agree by construction. Alternative: give each emitter a
"resolve in declaring module" branch. Rejected: duplicates the origin mapping in two places and
adds a third lookup step to code that is already the most intricate part of each emitter.

### D3. Visible-name choice

Reuse the module's existing `ImportedType` when one already points at the same library and
exported name, whatever its visible name. Otherwise use the origin's exported name if nothing in
the graph declares it and no existing import claims it as a visible name; if it is taken, use
`<library name>.<exported name>`, which the emitters sanitize the same way they sanitize a
qualified selective import (`Flow_QuestionFlow`). C# output does not depend on the visible name at
all; TypeScript gets a bare `Tag` in the common case and an aliased import only under a genuine
collision.

### D4. Origin lookup lives in `ImportedTypeCollector`

`CachedImportedLibrary` keeps its `Arc<LibraryArtifact>` (or the two maps the pass needs:
`namespaces` and an index from `(module_identity, definition_id)` to the interface item), and the
collector keeps an index from module identity to the cached library. Transitive dependencies are
loaded through the same `load_dependency` cache by walking `LibraryArtifact.dependency_roots`, so
a base whose field type comes from *its* dependency (the alias scenario) resolves too. In library
mode the generating library's own `namespaces` and interface items are indexed the same way so
peer identities resolve through the same code path.

Membership in a library's `namespaces` map, not a path-prefix test, decides which library owns a
module identity; that keeps the mapping independent of how canonical paths are spelled.

### D5. The warning survives only for what typegen cannot generate

`warn_about_unresolvable_inherited_fields` is deleted. The pass in D2 emits one warning per
unresolvable name, in the same form as today but saying why: the name did not resolve in the
declaring module, the origin is not exported, or its kind has no cross-library representation.
The `.Update`-typed case keeps the existing warning from `rewrite_update_type_references`, which
runs later on the already-rewritten names.

### D6. Where the three deferred tests live

- Cross-library `User.Update` type checking: `crates/nx-api/src/artifacts.rs` tests, next to
  `library_artifact_record_inheritance_resolves_across_library_files`, using a `TempDir` library
  for the base and `build_program_artifact_from_source` with a registry-backed
  `ProgramBuildContext` for the importing module. `nx-types` tests are single-file by
  construction (`check_str`), so this is the lowest layer that can express the scenario.
- Component-target `Counter.Update` execution: `crates/nx-codegen/src/tests.rs`, beside
  `generated_javascript_keeps_absent_update_fields_absent`, through
  `execute_generated_javascript_root`, with the existing node-absent early return.
- Bare update-record effect in .NET: `NxUpdateRecordTests.cs`, using the root-bound shape from
  `a_root_bound_handler_in_the_output_runs_and_its_results_are_effects`: a root `let` binds
  `onTapped=<User.Update name="Ada" />` on an external `Button` and a stateless component renders
  that `let`'s result, so the handler is reachable by token from the component's rendered output
  and every result it returns is an effect.

## Risks / Trade-offs

- [The assumed dependency namespace and package are derived from the directory name, so a
  transitive origin two libraries away gets a namespace derived from *its* directory] → This is
  the existing rule for direct imports; the pass produces exactly what an explicit import would.
  The existing "assumed package" warning already covers the TypeScript side.
- [Rewriting a field's `TypeRef` name changes what `ExportedUpdate.fields` reports to anything
  else that reads it] → Only the emitters and the model's own tests read it; the model tests that
  assert field names are unaffected because only type names change.
- [A peer origin in library mode whose exported name collides with another module's declaration]
  → The graph's owner map already has first-wins semantics for duplicate names across modules;
  this change does not make that worse and does not try to fix it.
- [`node` absent in CI makes the new JavaScript execution test pass vacuously] → Pre-existing for
  the whole family; noted in the follow-up doc, not addressed here.
