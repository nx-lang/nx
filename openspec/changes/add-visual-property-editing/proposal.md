## Why

Visual edits the values a document already writes (`add-visual-value-editing`). What it cannot do yet
is the part of editing that makes Visual worth having beside a text box: show the choices. A reader
can't see which properties an element could also set, can't pick from the values a library offers,
and every number is the same plain field, whether it is a font size on a scale of sizes or a
count. This change combines phases 2 and 3 of the
design "Editing in Visual: design and plan" (https://claude.ai/artifact/66bGJxr9DjiS9GY5jk86Xu):
discovering properties, and the editor hints that pick a property's editor. Bret asked for the two
together so the property editing can be tried as one experience.

After trying a first version (a slider snapping to `stops`, and named values: every `let` whose type
fits, offered as references), Bret compared how other products edit numbers (2026-10-22) and chose
Paper-style choices that a library declares, Framer-style steppers, and no slider. Named values
from `let`s guessed at intent from types alone, so they wait for design-system support.

## What Changes

- **More properties.** A selected element ends with a quiet `Show N unset properties` row. Opening it lists the
  properties the element does not write, dimmed, each with its default and doc. Editing one writes it.
  A new `sourceEdit` edit, `addProperty`, writes an attribute into an element: on its line for a
  one-line element, on a line of its own for one written a property per line.
- **A row menu.** A selected row gets a quiet `⋯` menu: Reset to default or Remove (what Delete does
  today), and Show in Source. Delete stays as the keyboard path.
- **Editor hints in `@nx/visualizer`.** `PropertyVisualizer` and `ValueVisualizer` gain `editor`
  (`field | dropdown | segmented`), `min`, `max`, `step`, and `choices`: `ValueChoice`
  children, each a `value` and an optional `label`, the values a viewer offers to pick, in order.
  `PropertyVisualizer` also gains `multiline` and `placeholder`. `ValueVisualizer.format` becomes
  optional. Analysis warns when a hint or a choice does not fit the property's type. The source tree
  carries the resolved hints.
- **New editors.** Choices in a list under a number or string field, Paper-style: each choice's label
  beside its value (one column when no choice has a label), the property's default on a row of its
  own above them, the current value marked; picking writes the value, and the default's row resets.
  The field stays typeable, so choices suggest and never restrict. Steppers on numbers, Framer-style:
  up and down buttons just past the value, while its field is open or on a selected row; a click
  steps once, Shift ten steps, and a vertical drag scrubs. A segmented control for a union's cases;
  numbers kept in range, with a note when a typed
  number is outside it; a multi-line text field for `multiline` strings; a placeholder in an empty
  field. Any string field takes Shift+Enter as a line break.
- **The playground**'s "Visual test cases" example gains the new cases.
- Not in this change: element-typed and record-typed properties set from Visual (phase 5's insert
  menu), a spacing editor, palettes, text content over several lines (phase 4 or 7), and expressions
  edited as code (phase 7). Deferred until design-system support is fleshed out: choices from a
  design system's tokens, and named values or references from `let`s; a value written as a
  reference still opens Source. Sliders are deferred too, perhaps returning with a live preview while
  dragging for opacity-like values.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `source-edits`: `addProperty`.
- `visualizer-library`: the editor hints on `PropertyVisualizer` and `ValueVisualizer`, `ValueChoice`
  and the `ValueEditor` union; `ValueVisualizer.format` optional.
- `visualizer-declarations`: hints merge like formats, an alias's hints reach the properties it types,
  and a hint that does not fit its property's type is a warning.
- `source-tree`: a resolved visualizer carries each property's editor hints, and an alias's.
- `viewer`: the more-properties row, the row menu, and the hinted editors: choices and steppers.
- `playground`: the test-cases example covers the new cases.

## Impact

- `crates/nx-api/src/std/visualizer/visualizer.nx` and the visualizer resolution and diagnostics in
  the analyzer.
- `crates/nx-language-service`: `source_edit` (`addProperty`), the source tree's visualizer values,
  and tests.
- `packages/language-protocol`: the `addProperty` edit and the visualizer hint fields.
- `packages/viewer`: rows, menu, editors, styles, tests and README.
- `sites/playground`: the test-cases example.
- `sites/website`: the visualizer library reference page.
- `src/vscode/CHANGELOG.md` and the package changelogs.
