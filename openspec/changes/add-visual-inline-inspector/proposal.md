## Why

Visual reads close to the source: raw property names, `name=value` lines, and sheets that wrap by
pane width. That suits people who know NX, but most people who use Visual to adjust a design, a
screen or a survey won't know the property names, and design tools have taught them a different
convention: labels, icons, numbers you drag to change. PR #71 started a
compact, selection-driven outline. This change keeps its structure and state handling and replaces
how elements and their properties read and edit, so Visual feels like an approachable design tool
while keeping NX's integrated, inline layout, which we want to put forward as its distinctive idea.

The agreed mockup is https://claude.ai/artifact/1tRgw7QraUeS485JmPuMDA.

## What Changes

- **Inline inspector.** Each element in the composition (an element outside another element's
  property value) reads as one row: icon, kind and one value that names it, with no label, as a
  design tool's layer list does: its title, else its text, else its id. Clicking that value edits it
  in the inspector. Selecting the row opens an
  inspector under the row with its properties as fields, one to a line, each label beside its value,
  indented at most one step. Clicking the selected row again closes it. A pin keeps an inspector open
  for comparison. The row the reader acts on stays put on screen while inspectors open and close.
  This replaces the width-dependent line/sheet reading for those elements; the `compact` opt-in goes
  away, since the playground is the viewer's only host.
- **Friendly names.** A property shows its visualizer's `label`, else one derived from its name:
  words split at case changes, `_`, `-` and letter–digit edges; known acronyms upper case; sentence
  case; `is`/`has` dropped from a boolean. The raw name stays in the label's hover card.
- **No property icons.** Fields show labels only; element rows keep their icons.
- **Scrubbing replaces steppers.** Dragging a number's label left and right changes
  it (Shift ×10, Alt ×0.1), live, as one edit and one undo step. Up and Down step a focused field.
  The stepper buttons are removed.
- **Order and unset properties.** Fields follow the visualizer's order, then declaration order, with
  no groups. An inspector shows what its element sets and its title; a last `Show N unset` line
  reveals the rest as quiet empty fields.
- **Colors in parts.** A color reads as swatch, six-digit hex and opacity percent, as in Figma: the
  swatch opens the picker, the hex and the percent edit in place, and dragging `%` scrubs opacity.
- **References, expressions and nested values.** A braced value keeps its quiet braces, so it reads
  as computed; `{blue}` leads with the swatch of the color the `let` it names holds, and its menu
  offers Go to. A short element value (up to four simple properties) reads its properties on its
  field's line as editable, scrubbable fields, without the kind its property's type implies;
  `ElementVisualizer inline=true|false` overrides that. Any other element value reads as its kind and
  the value that names it, and unfolds into a nested group.
- **Quieter hover cards.** Cards wait a second, then open at once while warm; none open during a
  press, a drag or a scroll; a row's card is on its icon; they sit just right of what they describe
  so the labels below stay readable. An element's card is titled by its name after a kind chip. The
  type's doc reads as one quiet line at the top of its inspector. No browser tooltips repeat a card.
- **Visualizers for element functions' properties.** Element-style functions get a derived
  `F.Props` record, as components have, so `FunctionVisualizer` can carry property visualizers
  checked against `F.Props.Property`. `@nx/graphics` helpers such as `Star` use this.
- `@nx/graphics` and the playground examples ship visualizers with labels.
- **BREAKING (unstable API):** the viewer's `compact` property and PR #71's Details button,
  "kept open" state and status line are removed; `detailsState` keeps `pinned` and `suppressed`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `viewer`: element rows named by one value, the inline inspector, labels,
  scrubbing instead of steppers, colors in parts, inline element values, pins and restorable state,
  and quieter hover cards.
- `visualizer-library`: `label` on `PropertyVisualizer`; `inline` on `ElementVisualizer`; `Target`
  and property visualizers on `FunctionVisualizer`.
- `visualizer-declarations`: `Target` binding for function visualizers; labels resolve like other
  hints.
- `component-props-type`: element-style functions derive `F.Props` as components do.
- `source-tree`: visualizer values carry labels and `inline`, and
  function entries carry their property visualizers.
- `graphics-library`: visualizers with labels.
- `fiddle-nx-visual-view`: the inline inspector, its state across switches, scrubbing.

## Impact

- `packages/viewer`: element rows, the inspector and its fields, scrubbing, state; PR #71's
  `compact.ts` state model is kept and simplified.
- `crates/nx-program/src/std/visualizer` and `.../graphics`: library types and visualizers; the
  compiler's visualizer checks and `F.Props` derivation; the source tree's visualizer data.
- `sites/playground`: wiring, examples' visualizers, tests and browser checks.
- No change to NX evaluation or the source-edit protocol.
