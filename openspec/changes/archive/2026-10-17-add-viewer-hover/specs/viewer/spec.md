## ADDED Requirements

### Requirement: The reading shows hover cards
The element SHALL show a card for what the pointer rests on, built from the source tree alone, in the
form the editor's hover uses: a signature in NX, set in the reading's code face, then the doc comment,
rendered from Markdown, in its prose face. It SHALL show:

- over an element's name or icon: its declaration's signature (its kind, its name and the base it
  extends) and the declaration's doc;
- over a property's name, on a group's row or on a line: `(property) name: Type`, with `?` for an
  optional property and ` = default` for one with a default, and the property's doc;
- over a value the reading shows without its property's name, as an id or a title: that property's
  card;
- over a union case written as a value, bare or qualified: `(case) Union.case` and the case's doc;
- over a reference or a call written as a value: its name and type, and the doc of the declaration it
  refers to when that has one;
- nothing over a literal, a comment, punctuation, or anything for which the tree gives neither a type
  nor a doc.

The card SHALL open after the pointer has rested for about half a second, SHALL stay open while the
pointer moves from what it describes onto the card, so its text can be selected and its links
followed, and SHALL close when the pointer leaves both, when the reading scrolls, when the reader
presses Escape, or when the text or the tree changes. It SHALL NOT change the selection, the folds or
the scroll position. With the reading focused, the `i` key SHALL show the card for the selected unit
(the declaration of an element, the property of a row) and a screen reader SHALL hear its text. The
card SHALL stay inside the element's box, below what it describes or above it where there is no room,
and SHALL follow the page's light or dark scheme.

A doc's Markdown SHALL be rendered by the element's `markdown` function when the host sets one, and
otherwise by a built-in renderer that SHALL create text and elements only, never parse HTML: it SHALL
render paragraphs, inline and fenced code, emphasis, lists and links, a link only when its target is
`http:` or `https:`, opening outside the page, and anything else, raw HTML included, as text.

#### Scenario: An element's declaration
- **WHEN** the pointer rests on the name of `<Shape Type=Circle />`, whose component `Shape` extends
  `Node` and is documented "A filled shape."
- **THEN** a card SHALL open showing `component Shape extends Node` and `A filled shape.`

#### Scenario: A property's type, default and doc
- **WHEN** the pointer rests on `Type` in that element, and `Shape` declares `Type:ShapeType = Rectangle`
  documented "What the shape draws."
- **THEN** the card SHALL show `(property) Type: ShapeType = Rectangle` and `What the shape draws.`

#### Scenario: A union case
- **WHEN** the pointer rests on `Circle`, and the case `Circle` of `ShapeType` is documented "Round."
- **THEN** the card SHALL show `(case) ShapeType.Circle` and `Round.`

#### Scenario: Nothing to say
- **WHEN** the pointer rests on a string literal or on a comment
- **THEN** no card SHALL open

#### Scenario: Moving onto the card
- **WHEN** a card is open and the pointer moves from the name onto the card, then off both
- **THEN** the card SHALL stay open on the card, and close once the pointer has left both

#### Scenario: From the keyboard
- **WHEN** the reading is focused with a row `Type` selected and the reader presses `i`, then Escape
- **THEN** the card for the property `Type` SHALL open and its text SHALL be announced, then it SHALL
  close, the row still selected

#### Scenario: A doc cannot inject markup
- **WHEN** a declaration's doc holds `<img src=x onerror="alert(1)">` and a link to `javascript:alert(1)`
- **THEN** the card SHALL show the tag as text and the link's text without a link, and no script SHALL run

#### Scenario: A host's Markdown
- **WHEN** the host sets `markdown` to a function and the pointer rests on a documented element
- **THEN** the card SHALL show what that function returns for the doc

## MODIFIED Requirements

### Requirement: One element shows a source tree in any host
The repository SHALL publish `@nx-lang/viewer`, which defines an `<nx-viewer>` custom element that
shows a `SourceTree`, as `@nx-lang/language-protocol` types it, together with the text of the
document the tree describes, as a reading of that text. The element SHALL depend on no UI framework,
SHALL keep its markup and styles in a shadow root, and SHALL follow the page's light or dark scheme.
Its properties SHALL be `tree`, `text`, `colors`, `icons`, `markdown`, `folded`, `selection` and `stale`, and a
property set before the element is defined SHALL take effect once it is. Where there is no DOM,
importing the package SHALL do nothing, and the package SHALL export a function that defines the
element in a registry the host chooses.

#### Scenario: A plain page
- **WHEN** a page with no framework imports the package, adds `<nx-viewer>` and sets its `tree` and
  `text`
- **THEN** the element SHALL show the document's reading

#### Scenario: Host styles don't reach in
- **WHEN** the host page styles `div`, `span` or `button` elements globally
- **THEN** the element's rendering SHALL not change

#### Scenario: A stale tree
- **WHEN** the host sets the element's `stale` property while it computes a newer tree
- **THEN** the element SHALL mute its reading and mark it out of date, and selection SHALL still work
