## Why

NX is written mostly by language models and read mostly by people checking that it says what they
meant. Today the only way to read a program is its source, which asks a reviewer to see past quotes,
braces, `={}` and operators to find the survey question, the condition or the agent prompt inside.
The NX Viewer and Previewer design proposes a reading view: a typeset rendering of the source that
hides syntax and never meaning, so each element reads as a card, each condition as a sentence, and
each piece still maps back to the text behind it.

The source tree (`add-source-tree`) now gives every construct of a document a role, a range, a
stable key, a resolved type and the declaration it refers to, with defaults, doc comments and union
cases in a declaration table. That is everything a first reading view needs, with no further
compiler work.

## What Changes

- A new framework-free package, `@nx-lang/viewer`, defines an `<nx-viewer>` custom element that
  shows a source tree as a reading view, as `@nx-lang/value-view` shows a value. A host sets the
  tree and the document's text, and the element renders it in a shadow root.
- The reading is lossless. Every node of the tree is shown, or folded into a details strip one click
  away, and every rendered piece carries the node's key and opens to the exact source text behind
  it. Elements read as cards headed by their kind in words, with their properties in declaration
  order and omitted defaults shown as muted ghost rows. Strings, numbers, booleans and union cases
  each have a value style. Conditions, matches and loops read as labelled sentences, operators read
  by the agreed word list, handlers read as "When … →" with one line per state change, and doc
  comments and markdown text read as prose.
- Hovering a property's name explains it from the declaration table: its type, doc comment and
  default, and for a union every case with the current one marked. A reference can be peeked and
  expanded in place when the document declares its target.
- The element has a selection: a click selects a node and reports its key and range, and a host can
  select a node by key. A helper finds the node a value origin (`add-value-origin`) points at, for
  the previewer to use later.
- The playground's source pane gains an Edit and Read switch. Read shows the visitor's source in
  `<nx-viewer>`, kept current as they type, and the selection carries between the two.

Viewer hints (`viewer` declarations) and the titles, icons and type wording they will supply are
not part of this change, so the reading here is the generic one the type checker alone allows. Nor
are the change view, the outline zoom, lenses such as the flow map, direct edits, or linking the
viewer to a preview.

## Capabilities

### New Capabilities

- `viewer`: the `<nx-viewer>` element, how it reads each role of the source tree, its hover,
  reference peek, source toggle and selection, and how it stays lossless.

### Modified Capabilities

- `playground`: the source pane reads as well as edits.

## Impact

- `packages/viewer`: the new package, its tests and a coverage test over every `.nx` file in the
  repository.
- `sites/playground`: the Edit and Read switch in the source pane, using the `sourceTree` query the
  playground's worker already answers.
- No change to the compiler, the language service, the source tree or the wasm ABI.
