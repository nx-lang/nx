## 1. Component props record

- [x] 1.1 Add `PROPS_RECORD_SUFFIX` (`Props`) beside the state suffix in `crates/nx-hir/src/lib.rs`, with `props_record_name` and `is_props_record_name`, and a `RecordKind::Props { component }` marker
- [x] 1.2 In `lower.rs`, synthesize `T.Props` for every component (inherited props first, types, optional marks and `content` kept, defaults dropped, the component's type parameters carried), and run the update-record and property-union synthesis on it
- [x] 1.3 Reserve `Props` as an inline emit name and a shared emitted action; reject `T.Props.Props`, `T.State.Props`, `T.Update.Props` and `R.Props`
- [x] 1.4 Carry derived `Props` declarations through imports and `DERIVED_DECLARATION_SUFFIXES` in `crates/nx-api/src/artifacts.rs`
- [x] 1.5 Change the component diagnostics for `T.Update`/`T.Property` in `crates/nx-types/src/infer.rs` to suggest `T.Props.*` when the component has no state
- [x] 1.6 Make `typegen` report an exported declaration that refers to `T.Props` and emit nothing for props records; leave NX IR lowering to treat `T.Props` as any record
- [x] 1.7 Add tests for every scenario in `component-props-type` and the changed scenarios in `property-references`, `update-records` and `component-syntax`

## 2. Type parameter property unions

- [x] 2.1 Accept `P.Property` for a record's own type parameter in its field types, as a rigid type; reject it for a component's type parameter and outside field types, and reject `P.Update`/`P.State`
- [x] 2.2 Substitute `P.Property` with the argument's property union on application, so contextual bare names resolve against it; report an argument without one
- [x] 2.3 Map `P.Property` in `typegen` (TypeScript `Extract<keyof P, string>`, C# `string`)
- [x] 2.4 Add tests for every scenario in the `record-type-parameters` delta

## 3. The `viewer` declaration

- [x] 3.1 Add `viewer_definition` to `crates/nx-syntax/grammar.js` as a module member, with `viewer` a keyword only in that position; regenerate the parser and update `ast.rs`, `syntax_kind.rs` and the grammar tests
- [x] 3.2 Lower viewer declarations in HIR as a separate list on the module, keeping the target path and value; keep them out of the module's items, exports, IR and declaration schema
- [x] 3.3 Resolve the target and report targets that cannot have a viewer (functions, values, payload unions, non-primitive aliases, generic and derived declarations)
- [x] 3.4 Type-check the value against the viewer record for the target's kind, binding `Target` for `@nx/viewer` constructions that leave it unset (the target, or `T.Props` for a component) and rejecting a different explicit `Target`
- [x] 3.5 Report non-constant viewer declaration values (braces, references, conditionals, loops, interpolation, calls)
- [x] 3.6 Report `duplicate-viewer` for two viewer declarations of one target in one module
- [x] 3.7 Make sure the interpreter, codegen and both runtimes ignore viewer declarations; add an IR test that an image is identical with and without them
- [x] 3.8 Add tests for every scenario in `viewer-declarations` that analysis alone covers

## 4. The `@nx/viewer` library

- [x] 4.1 Write `crates/nx-api/src/std/viewer/viewer.nx` with the eight types, a doc comment on every type, field and case, and the `unstable` header
- [x] 4.2 Register the library beside `@nx/agent` so it is importable, can be an implicit import, and generates with `typegen @nx/viewer`
- [x] 4.3 Add the library tests from `viewer-library` (exact exports, no diagnostics, every scenario)

## 5. Resolution and the source tree

- [x] 5.1 Gather viewer declarations for a document's program (entry module, its imports, the standard libraries) and for host viewer modules named in the request
- [x] 5.2 Resolve viewer declarations along inheritance and across sources, merging attributes, `PropertyViewer`s and `CaseViewer`s by `of`, ordering properties, not inheriting `name`, and resolving formats through aliases as written
- [x] 5.3 Report cross-module `duplicate-viewer` on each document whose program loads both viewer declarations
- [x] 5.4 Add the `viewers` option and the `viewer` values to `crates/nx-language-service/src/source_tree.rs`, list unions and aliases with viewers that type listed properties, mark viewer declarations, and report missing host modules
- [x] 5.5 Add the option and types to `@nx-lang/language-protocol`, carry them through the wasm snapshot and `@nx-lang/language-core`, and extend the Rust/TypeScript parity test with a document with viewer declarations
- [x] 5.6 Add tests for every scenario in the `source-tree` delta

## 6. Viewer and playground

- [x] 6.1 Read `viewer` values in `packages/viewer`: kind names, title, subtitle, badge and image parts, labels, boolean phrases, the property order, and the details strip
- [x] 6.2 Add the `row` and `inline` layouts
- [x] 6.3 Show case labels and examples, and value formats (`url`, `email`, `phone`, `image`, `color`, `markdown`, `text`), with the `images` property
- [x] 6.4 Read a `viewer` declaration as "Viewer for …"
- [x] 6.5 Keep the coverage test passing over the repository, and add a fixture with viewer declarations (the survey question library with the proposal's viewer declarations) to the viewer tests
- [x] 6.6 Ask for viewer declarations in the playground's Read view and add the playground scenarios to its tests

## 7. Editor, docs and examples

- [x] 7.1 Scope `viewer` declarations in `src/vscode/syntaxes/nx.tmLanguage.json`, with grammar tests
- [x] 7.2 Add `viewer` to `nx-grammar.md` and the website syntax reference, and write a website page on viewer declarations with the survey example
- [x] 7.3 Add the `@nx/viewer` reference page beside the agent library's
- [x] 7.4 Document `T.Props` beside `T.State` on the website's component pages
- [x] 7.5 Add a playground example, `viewer-declarations.nx`, with a small record and union with viewers, and its expected output

## 8. Verification

- [x] 8.1 Run the Rust workspace tests, clippy, the TypeScript package tests, the viewer coverage test, and `openspec validate add-viewer-declarations --strict`
