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

A first version had a slider snapping to `stops` and named values: every `let` in scope whose type
fit the property, offered as references, through a `namedValues` query. After trying it and comparing
how other products edit numbers (Figma, Framer, Paper, Penpot, Webflow; 2026-10-22), Bret chose
instead: choices that a library declares, shown Paper-style; Framer-style steppers; and no slider.
Listing every `let` of a fitting type guessed at intent from types alone (an `int` count is offered
for a spacing), and tools that do offer tokens scope them explicitly (Figma's variable scopes,
Penpot's typed tokens), so named values wait for design-system support.

## Goals / Non-Goals

**Goals:**
- A reader sees, on any element, what else it could set, and sets it where it reads.
- A library says once how a type or a property edits (a range and a unit, the choices to pick from,
  a segmented control, a multi-line field) and every place that uses it gets that editor.
- Nothing moves when an editor opens, as in phase 1, and undo stays the host's.

**Non-Goals:**
- Setting a property whose type is a record, an element, a sequence of them, or a function. Those
  list under more properties but open Source; phase 5's insert menu sets them.
- Named values and references: choices from a design system's tokens, and `let`s offered as
  references, wait until design-system support is fleshed out. A value written as a reference opens
  Source, as in phase 1.
- Sliders: a slider earns its place with a live preview while dragging, which this change does not
  have (it commits on release); they may return with one, for opacity-like values.
- Palettes.
- A spacing editor, property groups, `readonly`: later hints, when something uses them.
- Text content over several lines and expressions edited as code (phases 4 and 7).
- Enforcing `min` and `max` in the language: they are hints to an editor, not type constraints.

## Decisions

### More properties is a row on the selected element, not a panel

The selected element's sheet, or its line, ends with a quiet `Show N unset properties` row (Bret,
2026-10-23: `+ N more` read as more written properties), `Hide unset properties` while open, N being the properties
it does not write that Visual can list. It shows only on the selected element, so the reading stays
as compact as Source until the reader asks. Opening it (click, or Enter on the row) draws the unwritten
properties under it as dimmed rows, in the visualizer's order, else declaration order, each reading its
default (`size  medium  default`) or, for an optional property, its type, quieter (`note  string`),
in a place as wide as a short entry (24 characters, or the row's width when narrower) that the hover
mark and the field it opens fill, with its doc in the hover card. Under a one-line element that starts
partway along its line, as `let <Special …/> = <Drink …/>`, the rows start under the element, inside
its highlight, no further in than half the pane.
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

### Editor hints are visualizer attributes, resolved like formats

On both `PropertyVisualizer` and `ValueVisualizer`: `editor: ValueEditor?` (`field | dropdown |
segmented`), `min`, `max`, `step: float64?`, `unit: string?`, and the content `choices: ValueChoice+?`.
On `PropertyVisualizer` only: `multiline: boolean = false`, `placeholder: string?`.
`ValueVisualizer.format` becomes optional, so a type can be described by its hints alone:

```nx
visualizer FontSize = <ValueVisualizer min=8 max=128 unit="px">
  <ValueChoice value=12 label="sm" />
  <ValueChoice value=16 label="base" />
  <ValueChoice value=20 label="lg" />
</ValueVisualizer>
```

Resolution follows `format`: a property's own `PropertyVisualizer` wins attribute by attribute, then
the `ValueVisualizer` of the alias its type names. Choices are one hint: a visualizer that gives any
replaces all of an earlier or an alias's, in its order, since merging two lists would mix two
scales. The source tree carries the result as an `editor` object on each `SourcePropertyVisualizer`,
and on an alias's `SourceVisualizer`; each choice's value is typed as a `setValue` value, so the
viewer passes it on as it is.

`ValueChoice` is `{ value: object, label: string? }`, content rather than an attribute so a scale
reads one choice a line, and named `ValueChoice` so it cannot clash with a document's own `Choice`
type (the survey example has one). `value` is `object` because a `PropertyVisualizer`'s target is the
record, so the library cannot name the property's type; analysis checks each choice instead.

A hint that does not fit its property's type is a warning (`editor-hint-mismatch`), not an error, and
the viewer ignores it: `field`, `min`, `max`, `step` and `unit` on a non-number; `dropdown` and
`segmented` on anything but a union of bare cases; `multiline` and `placeholder` on a non-string; a
choice whose value is not a number literal for a number, or not a string literal for a string, and
any choice on anything else. `min` above `max` is a warning too. Warnings, because a hint never
changes what a program means.

### The hinted editors

- **Choices**, Paper-style (Bret, 2026-10-22): a list under a number's or a string's field, each
  choice's label with its value after it, quieter, in a second column; one column when no choice has a
  label. The property's default, when it has one, reads on a row of its own above them (`Default 16`),
  set off by a rule, and picking it resets the property as the row menu's Reset does. The current
  value's choice is marked. The list opens from a chevron past the open field, from Alt+Down, or as
  the reader types, filtering by label and value. While it is closed, Up and Down step a number;
  while it is open, they move through it. Picking writes the choice's value, never its label: these
  are a library's suggestions, not references, and the field stays typeable, so they never restrict.
  A fixed set is a union, which has its own editors.
- **Steppers**, Framer-style (Bret, 2026-10-22): an up and a down button over the gap just past a
  number, as the case chevron is drawn, so nothing moves. They show while the number's field is open
  and on hover of a value in the selected row, not everywhere, so the reading does not flicker under
  a passing pointer. A click steps once (Shift ten steps), into the open field or committed at once;
  pressing one and dragging up or down scrubs, into the open field on release, else committed once.
  Their tooltip names the keys. Dragging a number's name sideways stays, as before. On a one-line
  element the steppers, and the chevron while a field with choices is open, are wider than the one
  space past the value, so they cover the start of the next property while they show. That is the
  trade for nothing moving: they show only on the selected row and while the field is open, and
  drawing them above or below the line would cover a neighboring line instead.
- **Segmented** (`editor=segmented`): the case popover shows the cases as one row of buttons. With more
  than five cases, or names too long for the reading's width, it falls back to the list.
- **Dropdown** (`editor=dropdown`): the phase 1 case popover, named so a library can say it.
- **Field** (`editor=field`): the number field every number has.
- **Range**: arrows, steppers and scrubbing stay within `min` and `max`, and step by `step` (Shift
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

- **The unset-properties row adds a line to the selected element.** It is one quiet line, only on the
  selection. If it reads as clutter, it can move into the row menu.
- **Choices write literals.** A document that picks `lg` holds `20`, not a name; if the library
  later changes its scale, the document keeps `20`. That is the point of choices as suggestions;
  tokens that a document refers to come with design-system support.
- **Steppers are small targets.** They are drawn only where they help (an open field, the selected
  row), with a tooltip, and the arrow keys and the name drag remain.

## Open Questions

1. The unset-properties row: on the selected element only (proposed), or on every multi-line element?
2. Named values: answered (Bret, 2026-10-22): not yet; choices a library declares instead, tokens
   with design-system support later.
3. Sliders: answered (Bret, 2026-10-22): none for now; choices and steppers instead.
4. The row menu: build it now (proposed), or leave removal to Delete for another round?
