## Why

Visual is the playground's default view, and today it is read-only: to change a title, a size or a
flag, a reader switches to Source, finds the attribute and edits its text. Visual was always meant to
gain editing. Most changes to an NX document are expected to come from an AI prompt, so editing in
Visual is for fine-tuning and for discovery: a union's dropdown shows every case a property can take,
which a text box never does. This change is the first of six planned in the design "Editing in
Visual: design and plan" (https://claude.ai/artifact/66bGJxr9DjiS9GY5jk86Xu): values, edited in
place. Adding properties, editor hints, Markdown, structure and templates follow as their own changes.

## What Changes

- A new language query, `sourceEdit`, turns what a reader meant into text edits: set a value (a
  string, a number, a boolean or a union case), set a run of text, or remove a property. The language
  service owns the spelling (quotes, escapes, braces, a property's whole line), so the source stays
  the truth and a viewer never writes NX text itself.
- `<nx-viewer>` gains an `editable` property. When a host sets it, the reading edits values in place:
  a value's editor takes the value's own box, so nothing moves and nothing grows taller. Strings, UI
  text and ids edit as text; numbers as a field with arrow-key steps and scrubbing by dragging the
  property's name; booleans toggle; a constant union case picks from a dropdown of every case, with
  docs; a color edits as a hex field with the browser's color picker. A one-line run of plain text
  content edits as text. A property with a default or an optional one can be reset or cleared.
- The viewer fires `nx-edit` with the node's key and the edit, and leaves applying it to the host. It
  fires `nx-open` when the reader asks to see a node in its source, for a value Visual cannot edit,
  such as an expression.
- The playground handles both: it asks its worker for the edit, applies it to the editor's model so
  undo and redo are the editor's in both views, refreshes the reading at once, and opens Source at the
  node for `nx-open`.
- Not in this change: adding a property not yet written, named values and tokens, editor hints in
  `@nx/visualizer` (sliders, segmented controls), Markdown and other typed text bodies, text written
  over several lines, inserting, deleting or reordering elements, templates, and showing diagnostics in
  Visual. The DrawnUI fiddle's Visual view stays read-only: it does not set `editable`.

## Capabilities

### New Capabilities

- `source-edits`: the `sourceEdit` query: the edits it makes, how it spells values and text, what it
  refuses, and its exposure to TypeScript hosts.

### Modified Capabilities

- `language-protocol`: the protocol defines six queries, `sourceEdit` among them.
- `viewer`: the `editable` property, editing values in place, and the `nx-edit` and `nx-open`
  events.
- `playground`: Visual edits the visitor's source through the editor, with the editor's undo.

## Impact

- `crates/nx-language-service`: a new `source_edit` module and tests.
- `bindings/wasm` (native ABI and `host.ts`) and `bindings/node` (native and `index.ts`): the snapshot
  answers `sourceEdit`. The wasm ABI version goes up by one.
- `packages/language-protocol`, `language-core`, `language-client`, `language-http`: the query's
  types, answer, client method and request validation.
- `packages/viewer`: editing in `src/index.ts` and `src/source.ts`, a new module for the editors,
  `src/styles.ts`, tests and README.
- `sites/playground`: the worker answers the query; `ReadingView.tsx` and `App.tsx` apply edits and
  handle `nx-open`; the editor handle gains a way to apply edits.
- `src/vscode/CHANGELOG.md` and the package changelogs.
