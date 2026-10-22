## Why

Visual edits the values a document already writes (`add-visual-value-editing`). What it cannot do yet
is the part of editing that makes Visual worth having beside a text box: show the choices. A reader
can't see which properties an element could also set, can't pick a design system's named color or
spacing instead of typing a literal, and every number is the same plain field, whether it is a
spacing that snaps to 4, 8, 16 or a font size with a unit. This change combines phases 2 and 3 of the
design "Editing in Visual: design and plan" (https://claude.ai/artifact/66bGJxr9DjiS9GY5jk86Xu):
discovering properties and named values, and the editor hints that pick a property's editor. Bret
asked for the two together so the property editing can be tried as one experience.

## What Changes

- **More properties.** A selected element ends with a quiet `+ N more` row. Opening it lists the
  properties the element does not write, dimmed, each with its default and doc. Editing one writes it.
  A new `sourceEdit` edit, `addProperty`, writes an attribute into an element: on its line for a
  one-line element, on a line of its own for one written a property per line.
- **A row menu.** A selected row gets a quiet `⋯` menu: Reset to default or Remove (what Delete does
  today), and Show in Source. Delete stays as the keyboard path.
- **Named values.** Where values of a property's type are in scope, such as `let accent:Color` or the
  members of a `let theme:Theme` record, an editor lists them above free entry, and picking one writes
  a reference (`color={accent}`) instead of a literal. A value that is already such a reference edits
  with its type's editor, the reference marked, instead of opening Source. A new language query,
  `namedValues`, answers the values a property can take; `sourceEdit` gains a `reference` value.
- **Editor hints in `@nx/visualizer`.** `PropertyVisualizer` and `ValueVisualizer` gain `editor`
  (`field | slider | dropdown | segmented`), `min`, `max`, `step`, `stops` and `unit`;
  `PropertyVisualizer` also gains `multiline` and `placeholder`. `ValueVisualizer.format` becomes
  optional. Analysis warns when a hint does not fit the property's type. The source tree carries the
  resolved hints.
- **New editors.** A slider in a popover with the number field beside it, snapping to `stops`; a
  segmented control for a union's cases; units shown after a number and never written; numbers kept
  in range, with a note when a typed number is outside it; a multi-line text field for `multiline`
  strings; a placeholder in an empty field. Any string field takes Shift+Enter as a line break.
- **The playground** answers `namedValues`, and the "Visual test cases" example gains the new cases.
- Not in this change: element-typed and record-typed properties set from Visual (phase 5's insert
  menu), a spacing editor, palettes, text content over several lines (phase 4 or 7), expressions edited
  as code (phase 7), and named values from anywhere but the document's scope.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `source-edits`: `addProperty`, the `reference` value, and the `namedValues` query.
- `language-protocol`: the protocol defines seven queries, `namedValues` among them.
- `visualizer-library`: the editor hints on `PropertyVisualizer` and `ValueVisualizer`, and the
  `ValueEditor` union; `ValueVisualizer.format` optional.
- `visualizer-declarations`: hints merge like formats, an alias's hints reach the properties it types,
  and a hint that does not fit its property's type is a warning.
- `source-tree`: a resolved visualizer carries each property's editor hints, and an alias's.
- `viewer`: the more-properties row, the row menu, named values in editors, a reference that edits,
  and the hinted editors.
- `playground`: Visual's named values come from the worker; the test-cases example covers the new
  cases.

## Impact

- `crates/nx-api/src/std/visualizer/visualizer.nx` and the visualizer resolution and diagnostics in
  the analyzer.
- `crates/nx-language-service`: `source_edit` (`addProperty`, `reference`), a `named_values` module,
  the source tree's visualizer values, and tests.
- `bindings/wasm` and `bindings/node`: the snapshot answers `namedValues`; the wasm ABI version goes
  up by one.
- `packages/language-protocol`, `language-core`, `language-client`, `language-http`: the new query,
  edit and value, and the visualizer hint fields.
- `packages/viewer`: rows, menu, editors, the `namedValues` host hook, styles, tests and README.
- `sites/playground`: the worker, `ReadingView.tsx`, and the test-cases example.
- `sites/website`: the visualizer library reference page.
- `src/vscode/CHANGELOG.md` and the package changelogs.
