## Why

In Source, hovering a name shows what it is and its doc comment; in the Visual reading nothing does,
so a reader who switches to Visual to read a document loses the very documentation that explains it:
what a `SkiaShape` is, what `Type` takes, what `Circle` means. The reading already holds all of it:
the source tree it draws carries each declaration's doc, kind and bases, each property's type, default
and doc, each union case's doc, and each node's type.

## What Changes

- `<nx-viewer>` shows a hover card, in the editor's hover's style, for what is under the pointer:
  - an element's name or icon: its declaration's signature (kind, name, base) and doc;
  - a property's name, on a row or a line: `(property) Type: ShapeType = Rectangle` and its doc;
  - a union case written as a value: the case, qualified by its union, and its doc;
  - a reference or call written as a value: its type, and its declaration's doc when it has one;
  - nothing where there is nothing to say: a literal, a comment, punctuation.
- It behaves as an editor's hover: it opens after a short pause, stays while the pointer moves onto
  it, so its text can be selected and its links followed, and closes when the pointer leaves, the
  reading scrolls, or the reader presses Escape. It never moves the reading or the selection.
- From the keyboard, `i` shows the card for the selected unit, and a screen reader hears it.
- The card is set in the reading's code and prose faces and its light or dark colors. A doc comment's
  Markdown is rendered by a small built-in renderer that builds text nodes only (paragraphs, code,
  emphasis, lists, links to `http` and `https`), or by a host's renderer, a new `markdown` property.
- The icons' plain `title` tooltips give way to the card.
- No host wiring is needed: the playground and the DrawnUI fiddle get it with the package.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `viewer`: a new requirement for hover cards and their keyboard and screen-reader use; the element
  gains a `markdown` property.

## Impact

- `packages/viewer`: `src/index.ts` (pointer and keyboard handling, the card's placement), a new
  module for the card's content and one for the Markdown subset, `src/styles.ts`, tests, README.
- `src/vscode/CHANGELOG.md`.
- No change to the language service, the protocol or the SDKs: the source tree already carries what
  the cards show.
