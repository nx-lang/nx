## 1. Adding a property in the language service

- [x] 1.1 `addProperty` in `crates/nx-language-service/src/source_edit.rs`: keyed by an element, the attribute written after the last one in the element's layout (on the tag's line, or on its own line at the attributes' indentation), the value spelled as `setValue` spells it at an attribute; refuse `notEditable` for a non-element and `invalidValue` for a property already written, one the declaration lacks, or a content property; verify with tests for each scenario in `source-edits`
- [x] 1.2 Extend the round-trip test over the repository's `.nx` files: for every element, adding each unwritten property with its default yields a document that parses with the same keys plus the new attribute's

## 2. Editor hints

- [x] 2.1 `@nx/visualizer`: `ValueEditor` and its doc comments; `editor`, `min`, `max`, `step`, `unit` on `PropertyVisualizer` and `ValueVisualizer`; `multiline` and `placeholder` on `PropertyVisualizer`; `ValueVisualizer.format` optional; the library still builds with no diagnostics
- [x] 2.2 Hint resolution beside `format`'s: a `PropertyVisualizer`'s hints win one by one, then the alias's `ValueVisualizer`; the `editor-hint-mismatch` and `editor-hint-range` warnings; verify with tests for each scenario in `visualizer-declarations`
- [x] 2.3 The source tree: an `editor` value on `SourcePropertyVisualizer` and on an alias's `SourceVisualizer`, mismatched hints left out; the protocol types in `packages/language-protocol`; verify with source-tree tests and the TypeScript parity tests

## 3. Discovery and the row menu in the viewer

- [x] 3.1 The `+ N more` row on the selected element (sheet, group or one-line element): N counts the unwritten properties Visual can list; opening it draws them dimmed in visualizer order, else declaration order, with defaults, docs in the hover card, and a filter field past 12; verify with unit tests over fixtures
- [x] 3.2 Editing a dimmed row fires `nx-edit` with `addProperty`; the row reads as written in place on the next tree, the selection kept; properties whose type has no editor fire `nx-open`; verify "Nothing moves" for an added property
- [x] 3.3 The row menu: `⋯` on a selected or hovered written row, Reset to default or Remove, and Show in Source; keyboard reachable; no menu on dimmed rows; remove the quiet button from phase 1
- [x] 3.4 Styles for the dimmed rows, the `+ N more` row and the menu, light and dark

## 4. Hinted editors in the viewer

- [x] 4.1 Read hints from the tree
- [x] 4.2 Arrows and scrubbing kept in range and stepped by `step`; a typed number out of range kept with its note
- [x] 4.3 Units read after a number and shown after the field; typing the unit accepted and dropped
- [x] 4.4 Segmented cases, falling back to the list past five cases or when too wide; `editor=dropdown` and `editor=field`
- [x] 4.5 Multi-line string fields (Enter a line break, Ctrl/Cmd+Enter commits), Shift+Enter as a line break in any string field, strings written over several lines now editable, strings with escapes other than `\"` and `\\` still opening Source; placeholders
- [x] 4.6 Verify each `viewer` scenario for hints with tests

## 5. Choices and steppers instead of the slider and named values (Bret, 2026-10-22)

- [x] 5.1 Remove the slider, `editor=slider` and `stops` from the library, analysis, the source tree, the protocol, the viewer, tests and docs
- [x] 5.2 Remove named values: the `namedValues` query (language service, wasm and Node snapshots, `language-protocol`, `language-core`, `language-http`, `language-client`), the `reference` value of `setValue` and `addProperty`, the viewer's `namedValues` host function and lists, and the playground's; a value written as a reference opens Source again, as in phase 1
- [x] 5.3 `ValueChoice` in `@nx/visualizer` and `choices` content on `PropertyVisualizer` and `ValueVisualizer`; analysis warns `editor-hint-mismatch` at a choice that does not fit; resolution treats the choices as one hint; the source tree carries them typed as `setValue` values; verify with tests for each scenario in `visualizer-declarations`, `visualizer-library` and `source-tree`
- [x] 5.4 The viewer's choices list: two columns (one with no labels), the default's row on top, the current choice marked, a chevron, Alt+Down and typing to open, Up and Down stepping while closed and moving while open, picking commits the value, the default's row resets; verify each `viewer` scenario
- [x] 5.5 Steppers on numbers: an overlay past the value, shown while its field is open and on hover in the selected row; click, Shift+click, and a vertical drag committed once on release; a tooltip; verify each `viewer` scenario
- [x] 5.6 The "Visual test cases" example: choices for a number and a string, steppers, a reference opening Source; screenshots light and dark

## 6. The playground and docs

- [x] 6.1 Extend the "Visual test cases" example with the cases in the `playground` scenario, each with a comment saying what to try
- [x] 6.2 Verify the playground scenarios by tests and screenshots, in light and dark and at 375 pixels wide
- [x] 6.3 Document hints, `+ N more` and the row menu in the viewer, visualizer and language packages' READMEs and changelogs (redone for 5.1 to 5.5)
- [x] 6.4 Run `cargo test`, `pnpm -r build` and `pnpm -r test`, and `openspec validate add-visual-property-editing --strict`, all passing
