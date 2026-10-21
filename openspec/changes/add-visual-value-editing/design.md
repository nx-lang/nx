## Context

`<nx-viewer>` (`packages/viewer`) draws a `SourceTree` and the document's text as a reading. Every
node it draws carries its key; a click selects a unit (a group, a row, a line), the arrow keys walk
units, and hover cards show a name's declaration and doc. The tree's declaration table gives each
element's properties with their types, defaults, docs and flags, each union's cases, and, when
visualizers are asked for, each property's format (`id`, `color`). The viewer already reads
`uitext` values as prose and `format=color` values with a swatch.

Keys are paths such as `roleQuestion.value.choices[2].label`, stable under edits elsewhere in the
document (`source-tree`). A literal's node holds its text as written (`value`, quotes and escapes
included); a bare union case is a `case` node whose `declaration` is its union; a run of text content
is a `text` node.

The playground keeps its Monaco editor mounted while Visual shows, so the editor's model and undo
stack outlive switching views. Its worker answers language queries through `@nx-lang/language-core`
over the wasm SDK's language snapshot.

The design behind this change, with the survey of Figma, Framer, Storybook, Xcode, Android Studio and
others, is the doc "Editing in Visual: design and plan"
(https://claude.ai/artifact/66bGJxr9DjiS9GY5jk86Xu). Bret confirmed editing in place over a side
panel on 2026-10-21.

## Goals / Non-Goals

**Goals:**
- A reader changes a value in Visual where it reads, with an editor that fits its type, and the
  source changes by the smallest text edit that says the same thing.
- Nothing moves when an editor opens: the editor takes the value's box, and pickers open as popovers.
- Undo and redo are the host editor's, the same in Source and Visual.
- Discovery starts here: a union's dropdown lists every constant case with its doc.
- The viewer stays host-agnostic and read-only unless a host opts in.

**Non-Goals:**
- Writing a property that is not yet written, and named values or tokens (phase 2).
- Editor hints in `@nx/visualizer`: sliders, stops, units, segmented controls (phase 3).
- Typed text bodies such as `<Note:markdown>`, and text written over several lines (phase 4).
- Inserting, deleting, duplicating or reordering elements, and templates (phases 5 and 6).
- Diagnostics in Visual. An edit that type-checks badly shows its diagnostic in Source, as any edit
  does; the editors keep most such edits from being made.
- Editing expressions, `if`, `for` and `is` blocks, declarations, types or imports.
- Live preview while scrubbing a number: the edit is sent once, on release.
- The DrawnUI fiddle: its Visual stays read-only.

## Decisions

### 1. The language service writes the text, from a typed value

The viewer says what the reader meant, by key, and the language service answers the text edits.
The request carries a typed value, `{ kind: "string", value }`, `{ kind: "number", text }`,
`{ kind: "boolean", value }` or `{ kind: "case", name }`, not NX source text, so quoting, escaping,
entities, braces and whitespace are decided in one place, the one that has the syntax tree. The
alternative, a viewer that edits the text from node ranges, starts faster but would duplicate the
lexer's rules in TypeScript and drift from them; the next phases (adding properties on one-line and
multi-line elements, inserting elements with indentation) need the syntax tree even more.

The query is named `sourceEdit`, a noun like `sourceTree` and `hover`; the design doc called it
`editSource`. It is stateless like the others: the request carries the documents, and the key is
resolved against them, so a key from a slightly older tree still finds its node when nothing around
it changed, and an unknown key is refused rather than guessed.

### 2. Three edits, aimed at nodes

- `setValue` on a literal or a bare case that is a whole value: an attribute's value, an item of a
  braced sequence, or the value of a `let` without parameters. Its braces, when written, stay: `{true}`
  becomes `{false}`. A value inside an expression (an operand, an argument, a pattern) is not a whole
  value and is refused.
- `setText` on a `text` node written on one line, outside a typed or raw body. Element content has
  no escapes (entities are kept as written, and `\{` is only an escape in typed text), so the new
  text is written as it is, and refused unless the document reads it back as one run of exactly that
  text: `<` and `{` start a tag or a braced value, and a run cannot start with a space or with `if`,
  `for` or `else` as a word.
- `remove` on an attribute: the attribute goes, with the space that separates it from its neighbour,
  or its whole line when it has one. This is Reset for a property with a default and Clear for an
  optional one. The service does not check that the property may be left out; the viewer offers it
  only when it may, and the checker reports it otherwise.

A value of the wrong kind for its place (a number where a string was) is spelled anyway, and
type-checks badly like a hand edit would; refusing it would need the checker in the edit path for
little gain. What cannot be spelled is refused: a number that is not a numeric literal, a case name
that is not an identifier, text that is empty, holds a line break or would not read back as itself.

### 3. The answer is edits or a refusal, never an error

The answer is `{ edits: TextEdit[] }`, each edit a protocol range and its new text, applied in
order against the document as sent. A request NX cannot honor answers `{ edits: [], refused }` with a
reason, `unknownNode`, `notEditable` or `invalidValue`, so a host can tell "not this" from a broken
service. An edit that would change nothing answers no edits.

### 4. The editor follows the type, found through the tree

For a value on an element, the viewer finds the property through the element's declaration and reads
its declared type; an alias is followed through the declaration table to its format. The editors:

| Value | Editor |
| --- | --- |
| string, `uitext`, `format=id` | text field in the value's box, the value without quotes or escapes |
| `int`, `float32`, `float64` (or an alias of one) | number field: arrows step by 1, Shift by 10; dragging the property's name sideways scrubs |
| boolean | toggles; no field opens |
| bare case of a union | dropdown popover of the union's constant cases, each with its doc; typing filters |
| `format=color` | popover with a hex field and the browser's color picker |
| text run | text field, one line |

For an item of a sequence or a `let`'s value, the literal's own kind picks the editor. Payload cases
and anything not in the table stay read-only. The viewer decides only what to offer: the service
checks again.

### 5. A click on a value edits it

A click on an editable value edits it, as a form's or a property grid's field does, and selects its
unit: a field opens with the caret where the click fell, a boolean toggles, and a case or a color
opens its popover. A click anywhere else in a unit, as on a property's name or an element's header,
selects it, as today. Enter or F2 on a selected unit opens its first value with its text selected,
to type over it, as in a spreadsheet; Tab moves to the next value. Space on a selected boolean row
toggles it. (The first version asked for a selection first, so a click on an unselected value only
selected it. In use, Bret found the extra click unexpected, and a field opened with all its text
selected looked like a selection, not an editor, so a click now edits directly.) Under the pointer, a
value shows a faint mark and a text cursor, or a hand for a toggle or a popover, and a case a chevron,
as a dropdown does. An open field reads as an input: an editor's background, a focus ring and its own
selection color, apart from the selected row's band. Enter or leaving the
field commits, Escape cancels and keeps the selection, Tab commits and moves to the next editable
value, Shift+Tab to the previous. A text field grows with its text and wraps within the reading's
width; it never pushes the rows below unless the value already wrapped.

### 6. The host applies edits

The viewer fires `nx-edit` with the target's key and the edit, and shows the new value at once,
muted as stale, until the host sets the next tree. The host asks its language service, applies the
edits to its own text (in the playground, Monaco's `executeEdits`, which joins the undo stack), and
sets the new text and tree. A refusal leaves the text alone; the viewer drops its shown value when the
host sets a tree in which the value did not change. The host serializes edits: a second `nx-edit`
waits for the first one's tree. With Visual focused, Ctrl or Cmd+Z and Ctrl+Y or Shift+Cmd+Z reach
the host's undo; inside an open field they are the field's. Since shortcuts are hard to discover and
absent on a phone, the playground's NX title bar also shows Undo and Redo buttons in both views, as
Docs, Sheets and Notion's mobile apps put them in their toolbars.

### 7. Reset and Clear

With a row selected, Delete removes its property when its declaration gives it a default or makes it
optional. Backspace does not: a reader who meant to change the text would press it. (The first version
also showed a quiet "Reset to default" or "Clear" button on the selected row; Bret found it made a
destructive change too easy for a rare action, so it was dropped. A row menu with Reset and Remove
fits phase 2, where adding properties needs a menu anyway.)

### 8. `nx-open` for what Visual cannot edit

Enter or a double-click on a value Visual cannot edit fires `nx-open` with the node's key, role and
range, and the playground switches to Source with the cursor at the node. The reading says nothing
else: no lock icons or disabled styling, so a read-only value reads exactly as it does today.

## Risks / Trade-offs

- [A key resolves to a different node after a nearby edit] → Keys are paths; a sibling inserted
  before the target in a list shifts indices. Phase 1 edits never insert or remove elements, and the
  host serializes edits against the newest text, so this cannot happen within Visual. An edit made in
  Source between the tree and the request could still move it, which is why the request carries the
  documents and an unknown key is refused.
- [Round trips are slower than typing] → Each commit is one query and one re-analysis, as one
  keystroke in Source already costs. Text fields commit once, not per character.
- [Scrubbing without live preview feels dead] → The reading shows the number as it changes; the
  preview catches up on release. Live preview can come with phase 3's sliders.
- [Wasm ABI change] → The ABI version goes up; the playground and SDKs are built together, as for the
  source tree.

## Migration Plan

None: new query, opt-in viewer property. Hosts that do not set `editable` see no change.

## Open Questions

None blocking. Whether a click on a boolean should toggle without a selection first is left to try
in the playground.
