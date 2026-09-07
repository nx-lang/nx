# Measured: do library modules reach the language service?

Task 1.1 of `add-web-editor-packages`. Test:
`crates/nx-language-service` → `tests::measure_library_modules_reaching_workspace_analysis`.

## Setup

- A library directory `ui/` holding one file, `button.nx`:
  `export let <Button label:string /> = <button />`, loaded into a `LibraryRegistry`, and a
  `ProgramBuildContext` built from that registry with `registry.build_context()`.
- A logical workspace holding one module, `tenant/form.nx`:
  `import { Button } from "../ui"` followed by `<Button label="go" />`.
- `analyze_workspace_modules(&workspace, &context)`.

## Observed

| Question | Answer |
| --- | --- |
| Does the library's `ModuleArtifact` appear in the returned module list? | **No.** The list holds only `tenant/form.nx`. `analyze_logical_module_graph` collects the libraries an import selected into a separate `libraries` field, which `analyze_workspace_modules` drops. |
| Does the document's import resolve against the context? | **Yes.** The form module carries no diagnostics and its `prepared_bindings` hold `Button` with an `Imported` target. |
| What origin does the imported binding carry? | Module identity is the library module's `file_name`, which is the **absolute path** of the source file (`/tmp/.../ui/button.nx`), and `LocalDefinitionId(0)`. |
| Does the library keep enough to describe the declaration? | The library's `ModuleArtifact` retains its `lowered_module` (`Some`), so the item can be read the way workspace items are read. The library artifact does not carry the module's source text on the `ModuleArtifact`. |

## What this decides for D1

Passing the build context to `analyze_workspace_modules` and `validate_workspace` is sufficient for
**resolution and diagnostics**: the import binds, and no false "missing module" errors are raised.

It is **not** sufficient for hover, completion detail, or symbols on library names. The language
service resolves a visible binding to a `Declaration` by looking its origin up in `by_origin`, which
`build_workspace_declarations` fills only from the returned workspace modules. A binding whose
origin is a library module therefore has no `Declaration` and is dropped from the document scope.

So D1's fallback branch is required: after analyzing the workspace, `build_workspace_declarations`
must also walk the libraries the build context makes visible and index each library module's
lowered items under that module's `file_name`, exactly as it indexes workspace modules. This needs:

1. A public accessor on `ProgramBuildContext` that yields the visible `LibraryArtifact`s
   (`visible_library` is private today; only import resolution walks it).
2. Source text for library modules, because `declaration_from_item` slices signature text from the
   module source. `LibraryArtifact` does not expose per-module source, so the library build must
   retain it (a `sources` map keyed by module identity, alongside the existing `namespaces` map).
