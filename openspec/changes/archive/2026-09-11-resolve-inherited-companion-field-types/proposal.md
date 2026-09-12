## Why

A `<T>_update` companion copies its target's inherited fields, because a patch of `User` is not a
patch of its base and so the companion extends nothing in either target language. When the base
lives in another library and the importing module does not itself import a copied field's type,
typegen writes the bare name on the companion and neither emitter can resolve it. Today generation
only warns; the generated file still does not compile. This is item 1 of
`docs/add-update-actions-followsup.md`, and the change also closes the three test gaps recorded in
item 5 of that document, which need no new behavior.

## What Changes

- Typegen resolves a companion field's type in the namespace of the module that declared the field
  rather than the module that generates the companion, and emits a reference to the declaring
  library's generated declaration: a package import with a local alias in TypeScript,
  `global::<Dependency.Namespace>.<Name>` in C#. Both emitters already do this for types the module
  imports explicitly, so the work is in the typegen model.
- A library-local alias in the declaring module (`import { Tag as t.Tag }`) resolves to the
  origin declaration and generates the origin's exported name, exactly as an explicit import of
  that origin would.
- The warning stays only for a type the declaring library does not export, one typegen cannot
  generate a cross-library reference for, or a peer type the generating module cannot name
  unambiguously because an import of its own claims the same visible name; it no longer fires for
  a type that typegen can now resolve.
- Three test-only additions for scenarios the specs already state:
  - the checker accepts inherited fields on `User.Update` when the base is declared in another
    library, and rejects a field the base does not declare;
  - a `Counter.Update` constructed for a component target is emitted by the executable JavaScript
    with `$type` `Counter.Update` and only the `count` key;
  - a bare `User.Update` effect returned by a root-bound handler reaches a .NET host with exactly
    the keys `$type` and `name`.
- `docs/add-update-actions-followsup.md` drops item 1, the item 5 bullets this change lands, and
  item 3 (composing TypeScript companions), since choosing to resolve inherited fields in the model
  settles that alternative.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-code-generation`: the "Generated type surfaces include update records with distinct
  absence" requirement now states that an inherited companion field's type resolves in the module
  that declared the field, and gains scenarios for a field typed by a dependency export the
  generating module does not import, for a type reached through the declaring module's alias, for
  the warning that remains when the dependency does not export the type, and for the warning when
  an import in the generating module shadows the peer type an inherited field names.

## Impact

- `crates/nx-cli/src/typegen/model.rs`: `ExportedRecordField` carries its declaring module;
  `ImportedTypeCollector` maps module identities back to loaded libraries and resolves names
  through a declaring module's `ModuleNamespace`; `warn_about_unresolvable_inherited_fields` and
  `ExportedUpdate.inherited_from_other_modules` are replaced by the resolution pass.
- `crates/nx-cli/src/typegen/languages/{csharp,typescript}.rs`: no emitter change expected; the
  model hands them a synthesized `ImportedType` and a rewritten field type name.
- `crates/nx-cli/src/typegen.rs`: new cross-library generation test for both languages.
- `crates/nx-api/src/artifacts.rs` tests: cross-library `User.Update` checker test.
- `crates/nx-codegen/src/tests.rs`: component-target update execution test.
- `bindings/dotnet/tests/NxLang.Sdk.Tests/NxUpdateRecordTests.cs`: bare update-record effect test.
- `docs/add-update-actions-followsup.md`: items removed as described above.
