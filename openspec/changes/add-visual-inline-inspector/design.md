## Context

Visual today reads composition elements as `name=value` lines or, when they don't fit the pane,
as sheets of rows. PR #71 (`add-compact-visual-element-details`, draft, not merged) added a compact
outline with Details and Pin buttons and a disclosure state model (`compact.ts`), host state in the
playground, reconciliation of state across tree updates and list moves, and browser checks. Bret
reviewed it against the existing view and a mockup
(https://claude.ai/artifact/1tRgw7QraUeS485JmPuMDA, source in
`/mnt/project-files/reviews/pr-71-ux/visual-inspector-mockup.html`) and chose a design-tool
direction: friendly names, scrubbing, quiet layer-list rows and an inline (not docked) inspector,
which is the view's distinctive idea. This change replaces #71's change and
builds on its code.

## Goals / Non-Goals

**Goals:**
- One row per composition element, never taller than a source line, readable without opening it.
- One inspector design for every element, driven by visualizers where they exist and sensible
  without them.
- Scrubbing as the single way to drag a number; keyboard stepping kept.
- Keep #71's pins, host-restorable state, reconciliation and anchoring.

**Non-Goals:**
- A docked inspector, multi-select, or editing several elements at once.
- Evaluating expressions or showing runtime values; selection mapping from preview instances beyond
  what exists today.
- Localizing labels (NX has no built-in localizable text type yet; labels are strings).
- Showing more than one value on a row, or a per-kind summary of what an element sets.
- New editors for compound values (gradient editors); nested values edit field by field.

## Decisions

### 1. Rows replace lines and sheets for composition elements; `compact` goes away
The playground is the viewer's only host, and keeping both readings doubles the rendering and
test surface. Composition elements always read as rows. Declarations keep today's reading, and
anything a row can't show losslessly keeps the as-written fallback, so source coverage holds.
Alternative: keep `compact` as an opt-in. Rejected: no host needs the old reading, and AGENTS.md
says no backward compatibility yet.

### 2. One open reason besides selection: the pin
#71 had three reasons an inspector is open (selected, explicitly opened, pinned) plus a status line.
The mockup showed two are enough: selection opens and closes it (click the selected row to close
it), and a pin keeps it. `detailsState` becomes `{ pinned, suppressed? }`; `opened` is removed with
the Details button. `changeDetails` keeps its pin and suppress logic; `mapDetails` and
`reconcileDetails` keep working on the smaller state.

### 3. Inspector layout
Fields read one to a line in the resolved property order (visualizer-listed properties first, then
declaration order), which is already what the source tree sends: the label on the left, in a column
as wide as the longest label up to 40% of the inspector, and the value on the right. Bret chose this
over labels above values and over groups (2026-11-03): it takes less height and is simpler to read.
A value that spans lines, such as a list of elements, and an unfolded element value go under their
label, a step in. Only set properties and the title show; a last `Show N unset` line reveals the unset ones (Bret,
2026-11-04), as dashed values with the default as placeholder. There are no
group headings and no `group` hint.

### 4. Field labels
A field always shows its label and no icon: good, recognizable icons exist for few properties, and
icons on some fields but not others look uneven (Bret, 2026-11-04). The label's hover card gives the
property's name as written; there are no browser tooltips. Booleans are checkboxes beside their
label. Dragging a number's label scrubs it.

### 5. Labels
Visualizer `label` (a string) wins. The derived label follows the common conversion (Rails
`humanize`, Django `verbose_name`, Unity `NicifyVariableName`) but in sentence case, which matches
Figma, Apple and Microsoft field labels. Abbreviations like `cx` can't be fixed mechanically, so
libraries supply labels; graphics does in this change.

### 6. Scrubbing
Pointer down on a lead, then 4 px of horizontal movement starts a scrub; less is a click that focuses
the field. One step per 3 px; Shift ×10, Alt ×0.1. Values go out as live edits through the existing
live-edit path and commit once on release, so undo is one step. Pointer lock is left for later.
Steppers and their layout rules ("room on a line") are deleted.

### 7. One value names a row
Design tools keep their layer lists quiet: Figma shows one name per layer, with no label, which is a
text layer's text, an image's file name or the layer's type until someone renames it. Bret chose the
same for rows (2026-11-03): a row shows its icon, its kind and one value: the visualizer's `title`
when the element writes it on one line (a string, or an expression as written), else its text
content, else its `id`. Nothing else it sets reads on the row. A row of several values was busy, and
it invited clicks on values that could not be edited there; now a click on the one value opens the
inspector with that field ready to edit. An image names itself by its file name when its visualizer
makes its source the title.

### 8. References and nested elements
A reference to a top-level `let` is a token. The swatch comes from the `let`'s written value when it
is a color literal or an element whose visualizer formats one property as a color, which is a lookup
in the source tree, not evaluation. A nested element value is a field showing its kind and the value that
names it, or else the one thing it sets (a paint's color), that unfolds into a nested group; its fields edit the nested element through the same source edits.

### 9. `F.Props` for element-style functions
Graphics helpers (`Star`, `Arrow`, `Arc`, `RingSector`, `RegularPolygon`) are element-style
functions with `FunctionVisualizer`, which has no `Target`, so property visualizers can't name their
properties. Deriving `F.Props` for element-style functions, exactly as components derive `T.Props`,
gives `FunctionVisualizer` a `Target` and reuses all of the `PropertyVisualizer` checking. This is
the "functions may get `T.Props` eventually" step, done now because this change needs it.
Alternative: string property names on `FunctionVisualizer`. Rejected: unchecked and a second way to
say the same thing.

### 10. Reuse from #71
Keep `compact.ts` (trimmed), the `nx-details` event, `finishEditing()`, anchoring, the playground's
host state and reset rules, and the browser-check harness. Replace #71's row rendering in
`source.ts` with the row and inspector renderers, and delete the Details button, status line and the
prototype's one-line rows.

### 11. Colors read in their parts
Design tools show a color as a swatch, a six-digit hex and an opacity percent, each editable on its
own (Bret, 2026-11-04). An inspector does the same for a color literal: the swatch opens the
browser's picker (which keeps the opacity), the hex and the percent edit in place, and dragging the
`%` scrubs the opacity as a label scrubs a number. Writing back keeps the literal's form: a color at
full opacity stays six digits.

### 12. Short element values read on their field's line
`viewBox=<Rect x=0 y=0 width=400 height=240/>` is four numbers; folding it behind a chevron hides
what a designer edits most (Bret, 2026-11-04). A value that holds only properties, one to four, each
one simple value on one line, reads them on its field's line as labeled, scrubbable fields, Figma's
X/Y/W/H. The kind is dropped where the property's type implies it (a `Rect` for a `Rect`); a union
keeps its case. The rule is static, not width-based, so a value never switches form as the pane
resizes; it wraps instead. `ElementVisualizer inline=true|false` overrides it for a type. The chevron
still unfolds the full nested group, which holds the unset properties.

### 13. Braced values keep their braces
A value written in braces is computed; showing `{1 + 2}` and `{blue}` with their braces, quiet, tells
a reader it isn't a literal they can scrub (Bret, 2026-11-04). Literals read without them. A
reference to a color `let` leads with that color's swatch.

### 14. Quieter hover cards
Cards were distracting while scanning (Bret, 2026-11-04). Following design tools' tooltips: the first
waits a second, later ones open at once for a second and a half; none opens under a held button or
after a press or scroll until the pointer moves on. A row's card is on its icon, not its name or
value, which a reader rests on to select or drag. The type's doc reads as a quiet line atop its
inspector instead. A card sits below what it describes, starting just past its right end (or the
pointer, on a wide anchor) so labels below stay readable. An element's card is titled by its name
after a kind chip (`component`, `function`, `type`, `action`, `case`) instead of an NX signature
such as `let <Star />`; property and value cards keep their signatures.

## Risks / Trade-offs

- [Rows hide values that lines showed, such as a shape's size and fill] → the preview shows them, the
  inspector opens on selection, and Source is one click away; design tools make the same trade.
- [Layout shift when inspectors open] → anchoring the acted-on row; scroll only for off-screen
  selections; one short line per field.
- [Big elements bury the tree] → one line per field keeps inspectors short; the pin and closing the
  selected row put them away.
- [Remaining editing requirements still speak of lines] → they apply to fields in the inspector;
  wording can be tightened while syncing the specs.

## Migration Plan

No stored data. Close #71 when this change's PR opens; its branch stays for reference.
