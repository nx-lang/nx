## Context

`add-visual-value-editing` made Visual edit what a document writes. The viewer opens an editor in
the value's own box: a text field, a number field with steps and scrubbing, a toggle, a case popover
or a color popover. It fires `nx-edit` with the node's key and the edit, and the host asks the
language service's `sourceEdit` for the text edits and applies them, so undo is the editor's. A
property with a default, or an optional one, can be left out with Delete. Values the viewer cannot
edit, references to named values among them, fire `nx-open`, and the playground opens Source there.

The source tree already gives each declaration's properties, in order, with their types, defaults,
docs and flags, and each union's cases with their docs. With `visualizers`, it gives each property's
resolved `format`. The visualizer library has `ElementVisualizer`, `PropertyVisualizer` and
`ValueVisualizer`, trimmed on 2026-10-16 to what Visual used. Visualizer declarations merge along
inheritance and across modules, attribute by attribute, and an alias's `ValueVisualizer` formats
every property its type names.

The design behind this change is "Editing in Visual: design and plan"
(https://claude.ai/artifact/66bGJxr9DjiS9GY5jk86Xu), phases 2 and 3: discovery (more properties and
named values) and editor hints. Bret asked to combine them (2026-10-22).

## Goals / Non-Goals

**Goals:**
- A reader sees, on any element, what else it could set, and sets it where it reads.
- Where a document or its libraries name values of a property's type, the reader picks one and the
  source refers to it, which is how a design system steers people to its tokens.
- A library says once how a type or a property edits (a slider from 0 to 64 snapping to 4, 8, 16, a
  segmented control, a unit, a multi-line field) and every place that uses it gets that editor.
- Nothing moves when an editor opens, as in phase 1, and undo stays the host's.

**Non-Goals:**
- Setting a property whose type is a record, an element, a sequence of them, or a function. Those
  list under more properties but open Source; phase 5's insert menu sets them.
- Named values outside the document's scope (an index of every library), and palettes.
- A spacing editor, property groups, `readonly`: later hints, when something uses them.
- Text content over several lines and expressions edited as code (phases 4 and 7).
- Enforcing `min` and `max` in the language: they are hints to an editor, not type constraints.

## Decisions

### More properties is a row on the selected element, not a panel

The selected element's sheet, or its line, ends with a quiet `+ N more` row, N being the properties
it does not write that Visual can list. It shows only on the selected element, so the reading stays
as compact as Source until the reader asks. Opening it (click, or Enter on the row) draws the unwritten
properties under it as dimmed rows, in the visualizer's order, else declaration order, each reading its
default (`size  medium  default`) or nothing for an optional property, with its doc in the hover card.
Past 12 a filter field leads the list. A one-line element opens its list under its line, as a group
would. Editing a dimmed row's value commits `addProperty`; the row then reads as written, in place,
so nothing jumps. Collapsing the list, or selecting elsewhere, hides it again.

Alternatives: always showing the rows (too tall: Source shows only what is written, and Visual
promised to be no taller than Source); an inspector beside the reading (rejected in phase 1).

### `addProperty` is aimed at the element, and the service decides where the attribute goes

`{ kind: "addProperty", name, value }`, keyed by the element's node. The service writes the attribute
after the element's last attribute, in the same layout: on a one-line element, a space and `name=value`
on the tag's line; on an element written a property per line, a new line at its attributes'
indentation. An element with no attributes takes it on the tag's line. A property the element already
writes, or one it does not declare, is refused (`invalidValue`), as is a content property. Only the
service knows the layout and the spelling, so the viewer stays out of text.

### Named values come from a query, and are written as references

`namedValues(uri, key, property)` answers, for the property `property` of the element at `key`, the
values in the document's scope that the property's type accepts: parameterless `let`s, typed or
inferred, in the document or visible through its imports, and the members of such a `let` whose value
is a record, one level deep (`theme.accent`). Each item carries its reference as it would be written,
its declared type, its doc, and its value's source text when it is a literal (so a color can show its
swatch). Assignability is the type checker's, so a `Color` alias, a subtype or a union case all count.

Picking one writes `setValue` (or `addProperty`) with `{ kind: "reference", path: "theme.accent" }`,
spelled in braces where an attribute's value needs them (`color={theme.accent}`) and bare in a braced
sequence or a `let`'s value. A value already written as such a reference opens its type's editor with
the reference marked in the list, and typing a literal replaces it. A reference whose type the viewer
has no editor for still opens Source.

The viewer asks through a host hook, `namedValues(key, property) => Promise<NamedValue[]>`, when an
editor opens, and shows the list when it arrives; with no hook, editors read as in phase 1. A hook,
rather than more data in the source tree, keeps the tree small and asks only for the property in hand.

Alternative: listing every in-scope value in the source tree (one query, but the tree grows with the
scope, and assignability would be the viewer's to compute).

### Editor hints are visualizer attributes, resolved like formats

On both `PropertyVisualizer` and `ValueVisualizer`: `editor: ValueEditor?` (`field | slider |
dropdown | segmented`), `min`, `max`, `step: float64?`, `stops: float64*`, `unit: string?`. On
`PropertyVisualizer` only: `multiline: boolean = false`, `placeholder: string?`. `ValueVisualizer.format`
becomes optional, so `visualizer Spacing = <ValueVisualizer editor=slider min=0 max=64 stops={0 4 8 16
24 32 48 64} unit="px" />` needs no format. Resolution follows `format`: a property's own
`PropertyVisualizer` wins attribute by attribute, then the `ValueVisualizer` of the alias its type
names. The source tree carries the result as an `editor` object on each `SourcePropertyVisualizer`,
and on an alias's `SourceVisualizer`.

A hint that does not fit its property's type is a warning (`editor-hint-mismatch`), not an error, and
the viewer ignores it: `slider`, `min`, `max`, `step`, `stops` and `unit` on a non-number;
`dropdown` and `segmented` on anything but a union of bare cases; `multiline` and `placeholder` on a
non-string. `min` above `max` is a warning too. Warnings, because a hint never changes what a
program means.

### The hinted editors

- **Slider** (`editor=slider`, or a number with both `min` and `max` and no `editor`): a popover
  under the value with a slider and the number field beside it. Stops draw as ticks and the thumb snaps
  to them unless Alt is held; the field takes any number in range. Dragging shows the value in place
  and commits once on release, like scrubbing. After Bret compared other products' sliders
  (2026-10-22): stops are labeled with their numbers (only the ends past seven stops, and never
  overlapping); named values whose literal is in range tick on the track and are labeled by path, so
  the list and the scale read as one, and clicking a label chooses it; the arrows go stop to stop
  (Alt+arrow by `step`, Home and End to the ends); the property's default is a notch over the track,
  and a double-click resets to it as the row's Reset to default does. Dragging onto a named value's
  mark still writes the number; only choosing it writes a reference.
- **Segmented** (`editor=segmented`): the case popover shows the cases as one row of buttons. With more
  than five cases, or names too long for the reading's width, it falls back to the list.
- **Dropdown** (`editor=dropdown`): the phase 1 case popover, named so a library can say it.
- **Field** (`editor=field`): a plain number field, even with `min` and `max`.
- **Range**: arrows, scrubbing and the slider stay within `min` and `max`, and step by `step` (Shift
  by ten steps). A typed number outside the range stays in the field with a note (`At most 64`),
  as `12px` does today.
- **Unit**: read after the number, quieter (`16 px`), and shown after the field while editing; never
  written, and typing it (`16px`) is accepted and dropped.
- **Multi-line** (`multiline`): the text field wraps, Enter makes a new line and Ctrl or Cmd+Enter
  commits. Any string field takes Shift+Enter as a line break, so a long string can gain one without
  the hint (Bret, 2026-10-22). Line breaks are written as line breaks, as `sourceEdit` already spells
  them, so a string written over several lines, like the survey's intro, now edits in Visual. A string
  written with escapes other than `\"` and `\\`, such as `\n`, still opens in Source, since editing
  it would rewrite its escapes.
- **Placeholder**: shown in an empty field, quieter, never written.

### The row menu

A selected row shows `⋯` at its end on hover and when selected. Its menu offers Reset to default or
Remove (as Delete), and Show in Source (as `nx-open`). It takes two clicks, which answers the phase 1
worry that a button beside each row makes destructive changes too easy. Dimmed rows have no menu.

## Risks / Trade-offs

- **The `+ N more` row adds a line to the selected element.** It is one quiet line, only on the
  selection. If it reads as clutter, it can move into the row menu.
- **Named values can be many** in a library-heavy document. The list filters as the reader types and
  shows the closest first: the document's own, then its imports'.
- **References change meaning, not just text.** Picking `accent` ties the value to the token; the
  marked reference and the swatch from its literal make that visible.
- **Size.** This is about phase 1's size. If named values grow (the query and its tests across Rust
  and TypeScript), they are the piece to split off, and the tasks put them last for that reason.

## Open Questions

1. The `+ N more` row: on the selected element only (proposed), or on every multi-line element?
2. Named values: offer `let`s of the document and its imports, and one level of record members
   (proposed), or only `let`s?
3. A number with `min` and `max` and no `editor`: slider by default (proposed), or field until asked?
4. The row menu: build it now (proposed), or leave removal to Delete for another round?
