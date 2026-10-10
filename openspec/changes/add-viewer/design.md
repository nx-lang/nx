## Context

See proposal.md for why. What shapes the approach:

- The NX Viewer and Previewer design (lane 2, stages 1 and 2) asks for a reading view as a custom
  element beside `<nx-value>`, framework-free, so the playground and ReachMe embed the same code.
- `@nx-lang/value-view` sets the pattern: a shadow root, properties rather than attributes for data,
  light and dark from the page's scheme, Shiki for any NX it colors, jsdom tests against fixtures the
  real compiler produced.
- The source tree is a flat list of nodes in source order with parent indices, roles, flags, keys,
  types and declaration indices, plus a declaration table. It is marked unstable, so the viewer is
  its first consumer and may ask for changes.
- Agreed naming: a Viewer is a declaration-level presentation (a later `viewer` declaration); a View
  is one Viewer applied to one element node, internal to this package. Agreed operator wording:
  school-math symbols where one means exactly the same thing, words for programmer conventions.

## Goals / Non-Goals

**Goals:**

- A reviewer who does not read code can read a survey, a component or an agent and see everything
  it says.
- Nothing is lost: every node is shown or one click away, and every piece opens to its source text.
- The same element in the playground and in any other host, with no framework.
- A selection API that the previewer and the change view can build on.

**Non-Goals:**

- Viewer hints: card titles, icons, type wording such as "whole number", hidden or reordered
  properties. They come with the `viewer` declaration change, and this change's generic rules are
  their fallback.
- The change view (a structural diff of two trees), the outline zoom, lenses, direct edits and
  diagnostics on cards.
- Linking to a preview. Only the helper that finds a node by span is here.

## Decisions

### The element takes the tree and the text

`<nx-viewer>` has a `tree` property (`SourceTree` from `@nx-lang/language-protocol`) and a `text`
property, the document's source. The tree carries every name, value, token and comment, so the
reading needs only the tree; the text is for the source toggle, which shows the exact characters of
a node's range. The element slices the text by the range's UTF-8 byte offsets, so an emoji or a
non-Latin name cannot shift it. The host fetches the tree itself, from whatever service it has, so
the package depends only on the protocol's types.

### One renderer for each role of the source tree

Every node of the source tree has one role, such as `element`, `attribute`, `literal`, `condition`
or `comment`, from the closed list the `source-tree` capability defines. The viewer has one
rendering function for each role, which turns a node of that role and its children into DOM: the
`condition` renderer draws the "Only when" label, the `literal` renderer draws a value without its
quotes, and so on. Only the `element` renderer consults the declaration table, for property order
and defaults. A view (one element node read through its Viewer) is internal to the element
renderer: today every element uses the generic Viewer, and the hint change adds others without
changing the other roles' renderers. A new role in the source tree needs one new renderer, and the
coverage test below fails until it has one.

### Lossless by construction, checked over the repository

Every node's rendered DOM carries `data-key` with the node's key. Imports and plain comments between
declarations fold into a details strip at the top of the document and under each declaration, and
everything else renders inline. A coverage test computes the source tree of every `.nx` file in the
repository through `@nx-lang/sdk-wasm`, renders it in jsdom with the details strips open, and checks
that every node's key appears exactly once. A role the renderer forgets fails the test with the
file and the key, the way the source tree's own token test fails.

### Kinds in words, types as NX spells them

An element card's header is its kind's name split at its capitals into sentence case
(`SingleChoice` becomes "Single choice", `URLField` becomes "URL field"), and an unresolved kind is
shown as written with a muted marker. Types are shown as NX spells them (`int`, `string?`,
`Choice*`): wording such as "whole number" for `int` needs a decision per type that a hint can
override, so it waits for the hint change rather than becoming a second fixed list now.

### Properties in declaration order, ghosts from defaults

An element's attributes are listed in the order its declaration's properties are, inherited first,
then any attribute the declaration does not list, in source order. A property with a default that
the element does not set is a ghost row, muted and marked "default", read from the declaration
table's default text. The element shows ghosts by default and hides them when `ghosts` is false, as
a technical reader may prefer.

### Operators by the agreed list, grouping as written

| Source | Reads as |
| --- | --- |
| `==` `!=` `<` `>` `<=` `>=` | = ≠ < > ≤ ≥ |
| `+` `-` `*` `/`, prefix `-` | + − × ÷, − |
| `%` | remainder of a ÷ b |
| `&&` `\|\|` `!` | and, or, not |
| `a ?? b` | a, otherwise b |
| `x?` | x is given |
| `a..b` `a..=b` | a up to b, a through b |
| `"a" + x` with a string type | a sentence with a slot for `x` |

Each reading stands for one operator, so the reading stays one-to-one with the source. An operator
token the list does not name is shown as written. Parentheses the source writes are kept, and none
are added, so the reading groups exactly as the source does.

### Handlers read as events

An attribute flagged `handler` reads as "When" and the handler name without its `on` prefix, split
into words (`onIntegerAnswered` becomes "When integer answered"). When its value is a `stateUpdate`
element, each of that element's attributes is a line "set `name` to …". Any other value is read
after an arrow as an ordinary value.

### Selection by key

A click on a rendered node selects the smallest node under it, outlines it and dispatches
`nx-select` with the node's key, role and range. Setting the `selection` property to a key selects
that node, opening any fold around it, and scrolls it into view. `nodeAtSpan(tree, start, end)`
returns the index of the node whose byte range is exactly that span, preferring an element node, so
a previewer can turn a value origin into a selection.

### The playground switches the source pane between Edit and Read

The source pane's title gets an Edit and Read switch. Read replaces the Monaco editor with
`<nx-viewer>`, fed by the `sourceTree` query the playground's worker already answers, refreshed on
the same pause that refreshes evaluation, and marked stale while a newer tree is computed. Switching
from Edit selects the node at the cursor; switching back puts the cursor at the start of the
selected node. Hover in Read uses the declaration table, not the language service, so it works from
the tree alone.

## Risks / Trade-offs

- [The generic reading looks plain for library types, with no titles] → Hints are the next change;
  this one shows what the tree alone gives, so the hint design can be judged against it.
- [The source tree's shape changes while unstable] → The viewer is in the same repository and its
  coverage test runs on every tree change, so a shape change breaks the build, not a user.
- [Large documents render slowly] → Measured on the question-flow conformance program in the tests;
  folding declarations by default is the fallback.
- [Reading "and" and "or" without added grouping can mislead] → The source's parentheses are kept and
  the source toggle shows the text; adding parentheses the source lacks would break the one-to-one
  reading.

## Open Questions

- Whether the details strip should also hold `key`-like identifier properties (`id`) once hints can
  mark them, as the design's "Imports, ids, keys, handler tokens" row suggests.
