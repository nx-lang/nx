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

## 3. The `visualizer` declaration

- [x] 3.1 Add `visualizer_definition` to `crates/nx-syntax/grammar.js` as a module member, with `visualizer` a keyword only in that position; regenerate the parser and update `ast.rs`, `syntax_kind.rs` and the grammar tests
- [x] 3.2 Lower visualizer declarations in HIR as a separate list on the module, keeping the target path and value; keep them out of the module's items, exports, IR and declaration schema
- [x] 3.3 Resolve the target and report targets that cannot have a visualizer (functions, values, payload unions, non-primitive aliases, generic and derived declarations)
- [x] 3.4 Type-check the value against the visualizer record for the target's kind, binding `Target` for `@nx/visualizer` constructions that leave it unset (the target, or `T.Props` for a component) and rejecting a different explicit `Target`
- [x] 3.5 Report non-constant visualizer declaration values (braces, references, conditionals, loops, interpolation, calls)
- [x] 3.6 Report `duplicate-visualizer` for two visualizer declarations of one target in one module
- [x] 3.7 Make sure the interpreter, codegen and both runtimes ignore visualizer declarations; add an IR test that an image is identical with and without them
- [x] 3.8 Add tests for every scenario in `visualizer-declarations` that analysis alone covers

## 4. The `@nx/visualizer` library

- [x] 4.1 Write `crates/nx-api/src/std/visualizer/visualizer.nx` with the eight types, a doc comment on every type, field and case, and the `unstable` header
- [x] 4.2 Register the library beside `@nx/agent` so it is importable, can be an implicit import, and generates with `typegen @nx/visualizer`
- [x] 4.3 Add the library tests from `visualizer-library` (exact exports, no diagnostics, every scenario)

## 5. Resolution and the source tree

- [x] 5.1 Gather visualizer declarations for a document's program (entry module, its imports, the standard libraries) and for host visualizer modules named in the request
- [x] 5.2 Resolve visualizer declarations along inheritance and across sources, merging attributes and `PropertyVisualizer`s by `property`, ordering properties, and resolving formats through aliases as written
- [x] 5.3 Report cross-module `duplicate-visualizer` on each document whose program loads both visualizer declarations
- [x] 5.4 Add the `visualizers` option and the `visualizer` values to `crates/nx-language-service/src/source_tree.rs`, list unions and aliases with visualizers that type listed properties, mark visualizer declarations, and report missing host modules
- [x] 5.5 Add the option and types to `@nx-lang/language-protocol`, carry them through the wasm snapshot and `@nx-lang/language-core`, and extend the Rust/TypeScript parity test with a document with visualizer declarations
- [x] 5.6 Add tests for every scenario in the `source-tree` delta

## 6. Viewer and playground

The viewer first read `visualizer` values on cards (kind names, subtitle, badge and image parts, labels,
boolean phrases, the details strip, the `row` and `inline` layouts, case labels, the other formats and
"Viewer for …"). The Read view replaced the cards within this change; those parts stay in the source
tree for any reader.

- [x] 6.1 Replace the cards in `packages/viewer` with a reading of the document's text: code as written in the colors the host gives, an element over several lines as a group with a row to each attribute, an element on one line as its name and properties, every node rendered once with its key
- [x] 6.2 Read `visualizer` values in the reading: the title alone on an element's line, the id as one quiet token where `idPosition` puts it, the icon the visualizer names or the kind's on an element that starts its line, and the `id` and `color` formats
- [x] 6.3 Select units of the reading by click and by the arrow keys, draw the selection and the hovered unit as rows, and announce what is selected
- [x] 6.4 Keep the coverage test passing over the repository's `.nx` files and every fixture, with fixtures for groups, lines, content, ids, icons, colors, text types and comments inside attributes
- [x] 6.5 Ask for visualizer declarations in the playground's Read view, color it with the editor's themes, set it in Geist and Geist Mono, draw Lucide icons, carry the editor's cursor into Read and the selection back, and add the playground scenarios to its tests
- [x] 6.6 Add a "Larger examples" topic to the playground (a full survey, a settings screen, an orders dashboard), longer programs for judging the Read view
- [x] 6.7 Fold the reading: groups and multi-line declarations fold, runs of supporting declarations fold to one summary line and start folded, with a hanging chevron, Alt+click, arrow keys, `folded`, `collapseAll`, `expandAll` and `nx-fold`, and Collapse all and Expand all in the playground
- [x] 6.8 Cut `@nx/visualizer` to what the reading uses: `ElementVisualizer` with `title`, `id`, `idPosition: IdPosition = start` (`start | end`), `icon` and `properties`; `PropertyVisualizer` with `property` and `format`; `ValueVisualizer`; `ValueFormat = id | color`. Unions take no visualizer for now. Update the resolution, the source tree protocol, the examples, the docs and the tests

## 7. Editor, docs and examples

- [x] 7.1 Scope `visualizer` declarations in `src/vscode/syntaxes/nx.tmLanguage.json`, with grammar tests
- [x] 7.2 Add `visualizer` to `nx-grammar.md` and the website syntax reference, and write a website page on visualizer declarations with the survey example
- [x] 7.3 Add the `@nx/visualizer` reference page beside the agent library's
- [x] 7.4 Document `T.Props` beside `T.State` on the website's component pages
- [x] 7.5 Add a playground example, `visualizer-declarations.nx`, with a small record and union with visualizers, and its expected output

## 8. Verification

- [x] 8.1 Run the Rust workspace tests, clippy, the TypeScript package tests, the viewer coverage test, and `openspec validate add-visualizer-declarations --strict`
