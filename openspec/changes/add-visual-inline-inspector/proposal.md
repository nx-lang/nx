## Why

Visual reads close to the source: raw property names, `name=value` lines, and sheets that wrap by
pane width. That suits people who know NX, but most people who use Visual to adjust a design, a
screen or a survey won't know the property names, and design tools have taught them a different
convention: labels or icons, grouped properties, numbers you drag to change. PR #71 started a
compact, selection-driven outline. This change keeps its structure and state handling and replaces
how elements and their properties read and edit, so Visual feels like an approachable design tool
while keeping NX's integrated, inline layout, which we want to put forward as its distinctive idea.

The agreed mockup is https://claude.ai/artifact/1tRgw7QraUeS485JmPuMDA.

## What Changes

- **Inline inspector.** Each element in the composition (an element outside another element's
  property value) reads as one row: icon, kind, title and a curated summary. Selecting it opens an
  inspector under the row with its properties as compact fields in groups, in a two-column grid that
  indents at most one step. Clicking the selected row again closes it. A pin keeps an inspector open
  for comparison. The row the reader acts on stays put on screen while inspectors open and close.
  This replaces the width-dependent line/sheet reading for those elements; the `compact` opt-in goes
  away, since the playground is the viewer's only host.
- **Friendly names.** A property shows its visualizer's `label`, else one derived from its name:
  words split at case changes, `_`, `-` and letter–digit edges; known acronyms upper case; sentence
  case; `is`/`has` dropped from a boolean. The raw name stays in the tooltip and the hover card.
- **Icons and letters.** A property visualizer can name a Lucide `icon` or a `short` label of one or
  two characters (`X`, `W`), drawn at the start of the field. A property with neither gets a caption.
- **Scrubbing replaces steppers.** Dragging a number's icon, letter or caption left and right changes
  it (Shift ×10, Alt ×0.1), live, as one edit and one undo step. Up and Down step a focused field.
  The stepper buttons are removed.
- **Groups and order.** A property visualizer can name a `group`. Groups are collapsible, and unset
  properties show as quiet empty fields in their group, replacing "Show N unset properties".
- **Curated summaries.** `ElementVisualizer.summary` names the properties a row shows, drawn with the
  same icons and letters. Without one, a row shows up to three short written values.
- **References and nested values.** `{blue}` reads as a token chip with the swatch of the `let` it
  names when that `let` is written as a literal; a nested element value reads as its summary and
  opens into a nested group.
- **Visualizers for element functions' properties.** Element-style functions get a derived
  `F.Props` record, as components have, so `FunctionVisualizer` can carry property visualizers and a
  `summary` checked against `F.Props.Property`. `@nx/graphics` helpers such as `Star` use this.
- `@nx/graphics` and the playground examples ship visualizers with labels, icons, groups and
  summaries.
- **BREAKING (unstable API):** the viewer's `compact` property and PR #71's Details button,
  "kept open" state and status line are removed; `detailsState` keeps `pinned` and `suppressed`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `viewer`: element rows with curated summaries, the inline inspector, labels, icons, groups,
  scrubbing instead of steppers, pins and restorable state.
- `visualizer-library`: `label`, `icon`, `short` and `group` on `PropertyVisualizer`; `summary` on
  `ElementVisualizer`; `Target`, property visualizers and `summary` on `FunctionVisualizer`.
- `visualizer-declarations`: `Target` binding for function visualizers; checks for `short` and
  `summary`.
- `component-props-type`: element-style functions derive `F.Props` as components do.
- `source-tree`: visualizer values carry labels, icons, short labels, groups and summaries, and
  function entries carry their property visualizers.
- `graphics-library`: visualizers with labels, icons, groups and summaries.
- `fiddle-nx-visual-view`: the inline inspector, its state across switches, scrubbing.

## Impact

- `packages/viewer`: element rows, the inspector and its fields, scrubbing, state; PR #71's
  `compact.ts` state model is kept and simplified.
- `crates/nx-program/src/std/visualizer` and `.../graphics`: library types and visualizers; the
  compiler's visualizer checks and `F.Props` derivation; the source tree's visualizer data.
- `sites/playground`: wiring, examples' visualizers, tests and browser checks.
- No change to NX evaluation or the source-edit protocol.
