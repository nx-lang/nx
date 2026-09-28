# value-view Specification

## Purpose

The `<nx-value>` custom element, published in `@nx-lang/value-view`, that shows an annotated NX
value (the `NxValueText` that `evaluateNx()` returns) as NX text: highlighted with the published NX
grammar, folding long values, explaining nodes on hover and reporting where they are declared. It
depends on no UI framework, so any host, the playground among them, can reuse it.

## Requirements

### Requirement: One element shows an NX value in any host
The repository SHALL publish `@nx-lang/value-view`, which defines an `<nx-value>` custom element
that shows an `NxValueText`, the annotated NX text `evaluateNx()` returns. The element SHALL depend
on no UI framework and SHALL keep its markup and styles in a shadow root, so a host needs only to
import the package, create the element and set its `value`.

#### Scenario: A plain page
- **WHEN** a page with no framework imports the package, adds `<nx-value>` and sets its `value`
- **THEN** the element SHALL show the value's text

#### Scenario: A React page
- **WHEN** a React 19 component renders `<nx-value>` and passes the value as a property
- **THEN** the element SHALL show the value's text, with no wrapper component needed

#### Scenario: Host styles don't reach in
- **WHEN** the host page styles `pre`, `span` or `button` elements globally
- **THEN** the element's rendering SHALL not change

### Requirement: The value reads as NX
The element SHALL show the value's `text` exactly, character for character, in a monospaced face
with its line breaks and indentation kept. It SHALL highlight the text with the NX grammar the
repository publishes for editors, through Shiki, so the value colors the way the same text colors in
an editor, unless the text is longer than 20,000 characters, when it SHALL show the text uncolored
so the page stays responsive. It SHALL follow the page's light or dark scheme.

#### Scenario: The text is the CLI's text
- **WHEN** the element is given the `NxValueText` for `<User id="1" name="Ada" />`
- **THEN** its rendered text content SHALL be `<User id="1" name="Ada" />`

#### Scenario: Highlighting matches the editor
- **WHEN** the element shows `<User id="1" name="Ada" />`
- **THEN** `User`, `id`, `name` and each string SHALL carry the colors the published grammar's
  scopes give them in the element's theme

#### Scenario: A long value is shown uncolored
- **WHEN** the element is given a value whose text is longer than 20,000 characters
- **THEN** it SHALL show the text without coloring it, and SHALL still fold and hover it

#### Scenario: A shared highlighter
- **WHEN** the host sets the element's `highlighter` to a Shiki highlighter that has the NX grammar
  loaded
- **THEN** the element SHALL highlight with it and load no grammar or highlighter of its own

#### Scenario: Dark pages
- **WHEN** the page's color scheme is dark
- **THEN** the element SHALL use its dark colors

### Requirement: Long values fold
The element SHALL offer a fold toggle, in the gutter, for every record, property and sequence node
inside the value whose text spans more than one line and that starts its line; the value as a whole
does not fold, and a node that starts partway along a line folds with the node that starts it. A
folded node SHALL show its first line followed by an ellipsis, and choosing either the toggle or the
ellipsis SHALL unfold it. Every node SHALL start unfolded.

#### Scenario: Folding a record in a sequence
- **WHEN** the element shows a sequence of three records that each span several lines, and the
  visitor folds the second
- **THEN** the second record SHALL show only its first line and an ellipsis, and the first and third
  SHALL be unchanged

#### Scenario: A node partway along a line has no toggle of its own
- **WHEN** the element shows markup in which a record starts after `content={` on a line
- **THEN** that record SHALL have no toggle, no line SHALL have more than one, and no toggle SHALL
  sit over the text

#### Scenario: One-line values have no toggle
- **WHEN** the element shows `<User id="1" name="Ada" />`
- **THEN** it SHALL offer no fold toggle

#### Scenario: Folding does not change what is copied
- **WHEN** some nodes are folded and the visitor copies the value
- **THEN** the clipboard SHALL receive the whole text

### Requirement: Hover explains a value
Hovering a node SHALL show a description of the innermost node under the pointer. The host MAY
supply the description through a `describe` hook, which receives the node and returns markdown or
nothing. When it returns nothing, or no hook is set, the element SHALL show a default description
built from the node alone, spelled as `nx-language-service` spells hovers: a fenced `nx` fragment
such as `Task`, `(property) done: boolean` or `(case) Status.active`. The element SHALL render
fenced `nx` blocks highlighted, and other text as paragraphs with inline code.

#### Scenario: The default hover on a property
- **WHEN** no `describe` hook is set and the visitor hovers `done=false` in a `Task`
- **THEN** the element SHALL show `(property) done: boolean`

#### Scenario: The default hover on a sequence
- **WHEN** no `describe` hook is set and the visitor hovers the space between two items of a
  sequence of three `Task` records
- **THEN** the element SHALL show `Task*` and a count of 3 items

#### Scenario: A host's description wins
- **WHEN** the host's `describe` hook returns markdown for a node
- **THEN** the element SHALL show that markdown in place of the default

#### Scenario: Hover is reachable from the keyboard
- **WHEN** a visitor moves focus onto a node with the keyboard
- **THEN** the element SHALL show the same description it shows on hover

### Requirement: The element reports where a value is declared
When the visitor clicks a node that has a declaration, the element SHALL dispatch an
`nx-value-navigate` event carrying that node and its declaration span, so the host can show the
declaration.

#### Scenario: Clicking a type name
- **WHEN** the visitor clicks `User` in `<User id="1" name="Ada" />`, whose node's declaration is
  `type User` in the entry module
- **THEN** the element SHALL dispatch `nx-value-navigate` with that declaration span

#### Scenario: Nothing to navigate to
- **WHEN** the visitor clicks a node with no declaration, such as a string
- **THEN** the element SHALL dispatch no event

### Requirement: The element marks cut, stale and copied values
The element SHALL show a notice after the text when its `truncated` property is set. When its
`stale` property is set, it SHALL mute the text and show an "out of date" badge. It SHALL offer a
control that copies the text to the clipboard and confirms that it did.

#### Scenario: A cut value
- **WHEN** the host sets `truncated`
- **THEN** a notice SHALL follow the text saying that the rest of the value was cut

#### Scenario: A stale value
- **WHEN** the host sets `stale`
- **THEN** the text SHALL be muted, a badge SHALL say it is out of date, and hover and folding SHALL
  still work

#### Scenario: Copying
- **WHEN** the visitor presses the copy control
- **THEN** the clipboard SHALL hold the value's text, and the element SHALL confirm the copy
