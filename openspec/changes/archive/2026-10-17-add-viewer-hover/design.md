## Context

`<nx-viewer>` draws a `SourceTree` and the document's text (`packages/viewer/src/source.ts`), and
handles the pointer and the keyboard in `src/index.ts`: hovering highlights a unit, a click selects
one, the arrow keys walk them, and an announcer (`role="status"`) tells a screen reader what was
selected. Every node the reading draws carries its key (`data-key`); names are drawn as text inside
their unit, and an element's icon carries a `title` with its name.

The tree already holds everything a hover card needs (`@nx-lang/language-protocol`): each node's
`type` and `declaration`; each declaration's `kind`, `bases`, `doc`, `properties` and `cases`; each
property's `type`, `default`, `doc` and flags; each case's `doc`. The docs are Markdown, rendered as
the editor's hover renders them, doc links resolved. The editor's hover (`crates/nx-language-service/
src/hover.rs`) shows a fenced NX signature, such as `(property) Type: ShapeType = Rectangle`, and then
the doc.

## Goals / Non-Goals

**Goals:**
- The card says what a name is in the editor's hover's style (an NX signature, then the doc), from the
  tree alone, in any host. It is shorter than the editor's where a reader needs less (decision 3).
- Hovering never moves anything: the selection, the folds and the scroll stay as they were.
- A doc from a shared document cannot run script or inject markup in the host page.

**Non-Goals:**
- Asking the host's language service for hover text. A host-supplied hover can come later as an
  optional property, if a host wants exact parity.
- Cards for an `if` or `for`, an operator or a type written in a declaration's signature.
- Go-to-definition or any action in the card; its links are the doc's own.
- Touch: a long press is left for later, as no host needs it yet.

## Decisions

### 1. Content from the tree, not a host query

The card is built from the tree the element already has, so every host gets it with no wiring, no
latency and no second source of truth. The alternative, a `hover(key)` callback that a host answers
with the language service's hover, would match Source exactly but require each host to wire it, and
wait on a query per card; the tree's fields already give the same signature and doc.

### 2. Hover targets are marked as the reading draws them

What the pointer must rest on to open a card is a name or a value, not the whole unit: a row's name,
a line's `name=`, an element's name and icon, a value. The reading marks each such span with the
node it describes and what kind of card it takes (declaration, property, case, value). An id or a
title, drawn without its property's name, is marked with its property. The card logic then needs no
knowledge of the layout, and a unit's empty room opens nothing.

### 3. Signatures in the editor's style, shorter where a reader needs less

The card's signature is built in the editor's style from the tree, and differs from the editor's on
purpose where the reading already shows the rest. A component reads as its kind, name and base,
`component Shape extends Node`, not as its tag with every property, which the reading's rows and
lines show beside it; a property reads unqualified, since it sits on its element, and with its
default, which says what an omitted one reads as. The shapes: `component Shape extends Node`,
`type Choice`, `action Submitted`, `let <Glow />` for an element function; `(property) name?: Type =
default`; `(case) Union.case`; `name: Type` for a reference or a member; and for a call, or a
reference to a function, the function's `let twice(n:int): int`, its return type read from the
callee's type, since `twice: <function n:int />: int` reads worse than the signature it spells. A
qualified case is one target: its union's name opens the case's card. The doc follows unchanged. It
is set as code in the reading's code face, without syntax colors: the host's colors cover the
document's text, not text the card makes.

### 4. A small Markdown renderer that builds DOM, and a host override

The package has no dependencies, and a Markdown library with a sanitizer would add weight and still
parse HTML. `@nx-lang/value-view` has a hover card of its own, with a smaller renderer (code only) and
a 250 ms delay; the viewer does not depend on it, and the two packages stay apart for now. Sharing one
renderer, the viewer's, and one delay is left for when a package both can depend on exists. The built-in renderer handles the subset doc comments use (paragraphs, inline and fenced
code, emphasis, lists, links) and builds every node with `createElement` and text, so raw HTML stays
text and nothing is parsed as markup; a link is made only for `http:` and `https:` and opens with
`rel="noopener noreferrer"` outside the page. A host that wants its own rendering sets `markdown`,
a function from the doc's text to a `Node`, and is responsible for what it returns.

### 5. The card lives in the element and is placed in its frame

The card is one element in the shadow root, outside the scrolling body, placed against the frame
below what it describes, or above it when there is no room, and clamped to the element's box; it
closes on scroll rather than following. It takes the reading's tokens for its colors, so it follows
the light and dark scheme with the rest. Alternative: the Popover API's top layer, which would let the
card leave the element's box. Kept for later: the element's box is the pane in both hosts, and the
top layer brings focus and stacking questions a hover does not need.

### 6. Timing and the keyboard

The card opens after 500 ms at rest on a target, as editors do, and gives the pointer 300 ms to reach
it before closing. With the reading focused, `i` opens the card of the selected unit, through the
same content, and its text goes to the existing announcer, or "No information" where the tree says
nothing; Escape closes it. A card asked for from the keyboard stays while the pointer moves, until
Escape or the selection moves. The spans that mark one node, as a line's name and its `=`, are one
target, so moving between them neither restarts the rest nor closes the card. A scroll closes the
card wherever it happens: the document hears of most, and the reading's own scroller, which may sit
in a host's shadow root, is listened to while a card is open. A plain letter is free
in the reading, whose keys are the arrows, and needs no chord. A text or tree change closes the card,
since what it described may be gone. The icon's `title` is dropped, since the card names the element
and says more; its `aria-label` stays. An id's chip drops its `title` too, for the same reason: the
property's card names it.

## Risks / Trade-offs

- [A card over a dense line opens while the reader only moves across it] → The 500 ms rest, and
  targets limited to names and values, keep it from flickering; moving off closes it.
- [The built-in renderer misses some Markdown] → Doc comments use a small subset; anything else reads
  as its text, and a host can supply `markdown`.
- [The tree's types are spelled as the checker spells them, which may differ in detail from the
  editor's hover] → Both come from the same language service; tests pin the card for each target.
- [The card covers what is under it] → It sits below or above its target, never on it, and closes on
  leave, scroll and Escape.
