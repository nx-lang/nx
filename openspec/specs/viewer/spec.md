# viewer Specification

## Purpose
Defines the viewer: the `@nx-lang/viewer` package and its `<nx-viewer>` element, which show an NX
document's source tree as a reading view for people who read NX rather than write it. Elements read
as cards headed by their kind in words, values lose their syntax but keep a style, logic reads as
sentences and handlers as events, and declarations read as headed sections. The reading is lossless:
every node of the tree is rendered once, carries its key and opens to the exact source behind it.
Hover and reference expansion come from the tree's declaration table, and a shared selection by key
lets a host such as the playground move between the reading and the editor.
## Requirements
### Requirement: One element shows a source tree in any host
The repository SHALL publish `@nx-lang/viewer`, which defines an `<nx-viewer>` custom element that
shows a `SourceTree`, as `@nx-lang/language-protocol` types it, together with the text of the
document the tree describes, as a reading of that text. The element SHALL depend on no UI framework,
SHALL keep its markup and styles in a shadow root, and SHALL follow the page's light or dark scheme.
Its properties SHALL be `tree`, `text`, `colors`, `icons`, `markdown`, `folded`, `selection` and
`stale`, and a property set before the element is defined SHALL take effect once it is. Where there
is no DOM, importing the package SHALL do nothing, and the package SHALL export a function that
defines the element in a registry the host chooses.

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

### Requirement: The reading keeps every node
With nothing folded, the element SHALL render every node of the tree once, and the rendering of each
node SHALL carry the node's key. What is folded SHALL NOT be rendered; selecting a node folded away
reaches it. Offsets into the text SHALL be taken from each range's UTF-8 byte offsets, so text after a
character outside the basic multilingual plane is cut where it begins.

#### Scenario: Coverage over the repository
- **WHEN** the viewer's coverage test renders the source tree of every `.nx` file in the repository,
  with nothing folded
- **THEN** the key of every node SHALL appear in the rendering exactly once

#### Scenario: Text after an emoji
- **WHEN** a document holds `let wave = <Label text="😀 hello" title="after" />`
- **THEN** the reading of the element SHALL show `after` as the value of `title`

### Requirement: Code reads as written, in the editor's colors
The element SHALL read text as the document writes it, keeping its line breaks and indentation,
wherever the text is not part of an element read as a group or a line, such as a declaration, a
function's body or a comment. The
element SHALL color the text by the ranges its host gives in `colors`, each a start and end offset in
UTF-16 code units with a light color, a dark color and a font style, so the reading takes the colors
of the host's editor. The element SHALL mark comments and text shown to people as prose and each
line's indentation as indentation, so a host can set prose in a proportional font through
`--nx-viewer-prose-font-family` while indentation keeps a fixed width through
`--nx-viewer-indent-font-family`, and code is set through `--nx-viewer-source-font-family`.

#### Scenario: A type declaration
- **WHEN** a document declares `type Task = { title:string priority:int = 2 }` over several lines
- **THEN** the reading SHALL show the declaration as written

#### Scenario: Colors from the host
- **WHEN** the host gives a range over `Task` with a light color, a dark color and bold
- **THEN** the reading SHALL show `Task` in those colors, bold, and its text SHALL not change

### Requirement: An element over several lines reads as a group
An element the document writes over several lines SHALL read as a group: a header holding its icon,
its name, its id, and the attributes written on the line its tag starts on as `name=value`, the id
after those attributes when its visualizer puts it at the end; a row for
each attribute written on a line of its own, with the attribute's name in one column and its value in
another; and what the element holds under them, indented. The group SHALL show no angle brackets and
no closing tag. A bracket SHALL run down the group from under its name and turn right at its end. A
value of several lines SHALL read under its name rather than in the value column, and in a pane
narrower than about 64 characters every value SHALL read under its name. What the element holds SHALL
read an item to a line: an element in its own form, and anything else, such as an `if` or a `for`
block, as written, its later lines keeping their indentation relative to its own start. A blank line
the document leaves between two held items SHALL become a little room, and so SHALL the space between
two siblings where either is a group. A line too long for the pane SHALL wrap a step in from where its
unit starts in a group's header, a row's value, an element on one line alone on its line, and an item a
group holds as written; code read as written elsewhere SHALL keep its lines.

#### Scenario: Attributes a line each
- **WHEN** a document holds `<Step` with `id="welcome"` and `title="Welcome"` each on a line of its
  own, and the visualizer of `Step` names `id` as its id
- **THEN** the reading SHALL show a group headed `Step welcome` with one row, `title` and `Welcome`,
  and no `<`, `/>` or quotes

#### Scenario: Content under a header
- **WHEN** a document holds `<Row gap="wide">` on one line, two `<Item />` elements on the lines after
  it with a blank line between them, and `</Row>`
- **THEN** the reading SHALL show the header `Row  gap=wide`, the two items under it with a little
  room between them, and no closing tag

### Requirement: An element on one line reads as a line
An element the document writes on one line SHALL read as its icon and name, its id where its visualizer
puts it, the value of the attribute its visualizer names as its title alone, each other attribute as
`name=value`, and then what it holds as written, with no tags of its own. When the element shares
its line with other text, such as a property's name in a group or a `let` before it, the line SHALL
keep the `<` before its name and a `/>` after it, so its extent stays clear; alone on its line, it
SHALL show neither. An element alone on its line whose type is exactly the type its place declares,
through any list, SHALL leave out its name when it has an icon, which then names it.

#### Scenario: A list of choices
- **WHEN** a property typed `Choice+` holds `<Choice value="yes-one" label="Yes, one" />` on a line
  of its own, and the visualizer of `Choice` names `label` as its title, `value` as its id at the end and
  an icon
- **THEN** the reading SHALL show the icon, `Yes, one` and the id `yes-one`, without the name
  `Choice`

#### Scenario: An element sharing its line
- **WHEN** a group's row `status` holds `<OrderStatus.shipped carrier="Royal Mail" />`
- **THEN** the row's value SHALL read `<OrderStatus.shipped  carrier=Royal Mail />`

#### Scenario: Content on one line
- **WHEN** a document holds `<Label tone="quiet">Hello {who}</Label>` alone on its line
- **THEN** the reading SHALL show `Label  tone=quiet  Hello {who}` and no closing tag

### Requirement: Values lose their quotes and keep a style
In a group or a line, a string SHALL read without its quotes, and a boolean SHALL read as a check for
`true` and a cross for `false`. A string that a property, a field, a parameter or a declaration typed
`uitext`, or an alias that leads to it, holds SHALL read as text shown to people: in the prose font, in
a color of its own, with any quotes it keeps quiet. A string an element's visualizer names as its `id`, or
that a property's visualizer formats as `id`, SHALL read as one quiet token, the element's id moved to where
`idPosition` puts it. A string a visualizer formats as `color` SHALL read with a small square of that color
before it when it names a color.

#### Scenario: Text shown to people
- **WHEN** a type declares `label:uitext` and an element sets `label="First choice"` on one line
- **THEN** the reading SHALL show `First choice` as text shown to people, without quotes

#### Scenario: A color
- **WHEN** a property typed by an alias with `visualizer Color = <ValueVisualizer format=color />` holds
  `"#b6695d"`
- **THEN** the reading SHALL show a square of that color before `#b6695d`

### Requirement: Elements start with icons the host draws
The element SHALL take, through its `icons` property, a function that draws an icon by its Lucide name.
With it, every element that starts a line of its own SHALL start with an icon: the one its visualizer's
`icon` names, or else one for its kind of declaration, `component` for a component, `box` for a record,
`zap` for an action, `tag` for a union case and `square-function` for a function. When the function
draws nothing for the visualizer's icon, the element SHALL start with its kind's icon. The icon SHALL carry
the element's name as its label and add no text. Without the function, no element SHALL have an icon.

#### Scenario: A visualizer's icon and a kind's
- **WHEN** the host draws icons, the visualizer of `Choice` sets `icon="circle-dot"`, and a list holds a
  `Choice` and a record `Note` with no visualizer, each on a line of its own
- **THEN** the `Choice` SHALL start with the icon `circle-dot` and the `Note` with `box`

#### Scenario: An icon the host cannot draw, and a union case
- **WHEN** the host draws nothing for `no-such-icon`, the visualizer of a record `Tip` sets
  `icon="no-such-icon"`, and a list holds a `Tip` and `<Status.closed reason="done" />`, each on a line
  of its own
- **THEN** the `Tip` SHALL start with `box` and the case with `tag`

### Requirement: A selection is a unit of the reading
A click SHALL select the unit it falls in: an element over several lines, by its header or the room
between its rows; a row of a group; an element on one line; an element written as its tags that starts
a line; or a block written as it is, such as an `if`. Outside every unit, a click SHALL select the
smallest node under it, and a drag that selects text SHALL select nothing. Selecting SHALL fire
`nx-select` with the node's key, role and range, and tell a screen reader what was selected. The host
SHALL be able to select a node by key through `selection` without the event; a key the tree does not
hold SHALL clear the selection. The up and down arrow keys SHALL select the previous and next unit, the
left arrow the unit that holds the selected one, and the right arrow the first one it holds. A
selection that is not itself a unit, such as a node the host selects inside a row, SHALL move from the
unit that holds it, or, outside every unit, to the nearest unit after or before it. The
selected unit and the one under the pointer SHALL be drawn as rows behind the text, from just before
the unit's start to near the reading's right edge, a selected group's first line stronger than the rest
of it, in colors a host sets through `--nx-viewer-selection`, `--nx-viewer-selection-inactive` and
`--nx-viewer-hover`.

#### Scenario: Selecting a line
- **WHEN** a reader clicks a word of an element on one line
- **THEN** the element SHALL be selected and `nx-select` SHALL carry its key

#### Scenario: Selecting from the host
- **WHEN** the host sets `selection` to a node's key
- **THEN** that node SHALL be selected and no `nx-select` SHALL fire

#### Scenario: Walking the tree
- **WHEN** a group is selected and the reader presses the right arrow, the down arrow and the left
  arrow
- **THEN** the selection SHALL move to the group's first item, to its second, and back to the group

#### Scenario: Arrows from inside a row
- **WHEN** the host selects the string `"z"` in a group's row `note` and the reader presses the left
  arrow
- **THEN** the selection SHALL move to the row `note`, and from `"z"` the down arrow SHALL select the
  unit after that row

#### Scenario: From an origin to a node
- **WHEN** a value's origin names a byte range
- **THEN** `nodeAtSpan` SHALL give the node with exactly that range, an element where several share it

### Requirement: The reading folds
An element read as a group and a top-level declaration written over several lines SHALL each fold:
a group to its header, followed by its title's value when its visualizer names a title and otherwise
by how many properties and items it hides, cut to one line; a declaration to its first line. A run of
top-level declarations that support the document (imports, types, actions, components with no body
of their own, external or abstract, visualizer declarations, and functions other than `root` and
element functions, whatever modifiers they are written with), with the
comments written over them, SHALL fold to one line saying what it holds, such as `6 types · 2
visualizers`. A fold control SHALL hang to the left of each such unit, so folding moves nothing; it
SHALL show while the pointer is over the reading and always on what is folded. Clicking it SHALL fold
or unfold the unit, and an Alt+click SHALL fold or unfold everything inside it as well. With a group
selected, the left arrow SHALL fold it and the right arrow unfold it before they move the selection.
Until the host or the reader folds anything, each run and each declaration in it SHALL be folded and
everything else open, so the document's content comes first; `folded` SHALL hold the keys of what is
folded, setting it SHALL fold exactly those, and setting `undefined` SHALL go back to the defaults.
`collapseAll` and `expandAll` SHALL fold and unfold everything. Each change the reader makes SHALL
fire `nx-fold` with the keys folded. Selecting a node folded away SHALL unfold what hides it; folding a unit
that holds the selection, by the reader, `collapseAll` or the host, SHALL keep it folded and select
the outermost folded unit still drawn that holds the selection, or clear the selection when that
unit is a run, which is not a node. `nx-fold` SHALL carry the selection that results. `if` and
`for` blocks SHALL NOT fold.

#### Scenario: Supporting declarations come folded
- **WHEN** a document imports a library, declares two types, a function and a visualizer, and then a
  value, and the host sets no `folded`
- **THEN** the reading SHALL show one line, `1 import · 2 types · 1 function · 1 visualizer`, before
  the value, which SHALL be open

#### Scenario: Opening a run
- **WHEN** the reader clicks that line
- **THEN** the run SHALL open, each declaration in it SHALL show its first line, and `nx-fold` SHALL fire

#### Scenario: A folded group
- **WHEN** the reader folds a group whose visualizer names `title` as its title and which sets
  `title="Ship the site"`
- **THEN** the group SHALL show its header and `Ship the site`, and none of its rows

#### Scenario: Selecting into a fold
- **WHEN** the host selects a property of a folded group
- **THEN** the group SHALL unfold and the property SHALL be selected

#### Scenario: Folding over the selection
- **WHEN** a row of a group is selected and the reader folds the group
- **THEN** the group SHALL stay folded and SHALL be selected

#### Scenario: Folding a run over the selection
- **WHEN** a declaration in a run is selected and the reader folds the run
- **THEN** the run SHALL stay folded, nothing SHALL be selected, and `nx-fold` SHALL carry no selection

### Requirement: The reading shows hover cards
The element SHALL show a card for what the pointer rests on, built from the source tree alone, in
the form the editor's hover uses: a signature in NX, set in the reading's code face, then the doc
comment, rendered from Markdown, in its prose face. It SHALL show:

- over an element's name or icon: its declaration's signature (its kind, its name and the base it
  extends) and the declaration's doc;
- over a property's name, on a group's row or on a line: `(property) name: Type`, with `?` for an
  optional property and ` = default` for one with a default, and the property's doc;
- over a value the reading shows without its property's name, as an id or a title: that property's
  card;
- over a union case written as a value, bare or qualified: `(case) Union.case` and the case's doc;
- over a reference or a call written as a value: its name and type, and the doc of the declaration
  it refers to when that has one;
- nothing over a literal, a comment, punctuation, or anything for which the tree gives neither a
  type nor a doc.

The card SHALL open after the pointer has rested for about half a second, SHALL stay open while the
pointer moves from what it describes onto the card, so its text can be selected and its links
followed, and SHALL close when the pointer leaves both, when the reading scrolls, when the reader
presses Escape, or when the text or the tree changes. It SHALL NOT change the selection, the folds
or the scroll position. With the reading focused, the `i` key SHALL show the card for the selected
unit (the declaration of an element, the property of a row) and a screen reader SHALL hear its text.
The card SHALL stay inside the element's box, below what it describes or above it where there is no
room, and SHALL follow the page's light or dark scheme.

A doc's Markdown SHALL be rendered by the element's `markdown` function when the host sets one, and
otherwise by a built-in renderer that SHALL create text and elements only, never parse HTML: it
SHALL render paragraphs, inline and fenced code, emphasis, lists and links, a link only when its
target is `http:` or `https:`, opening outside the page, and anything else, raw HTML included, as
text.

#### Scenario: An element's declaration
- **WHEN** the pointer rests on the name of `<Shape Type=Circle />`, whose component `Shape` extends
  `Node` and is documented "A filled shape."
- **THEN** a card SHALL open showing `component Shape extends Node` and `A filled shape.`

#### Scenario: A property's type, default and doc
- **WHEN** the pointer rests on `Type` in that element, and `Shape` declares
  `Type:ShapeType = Rectangle` documented "What the shape draws."
- **THEN** the card SHALL show `(property) Type: ShapeType = Rectangle` and `What the shape draws.`

#### Scenario: A union case
- **WHEN** the pointer rests on `Circle`, and the case `Circle` of `ShapeType` is documented
  "Round."
- **THEN** the card SHALL show `(case) ShapeType.Circle` and `Round.`

#### Scenario: Nothing to say
- **WHEN** the pointer rests on a string literal or on a comment
- **THEN** no card SHALL open

#### Scenario: Moving onto the card
- **WHEN** a card is open and the pointer moves from the name onto the card, then off both
- **THEN** the card SHALL stay open on the card, and close once the pointer has left both

#### Scenario: From the keyboard
- **WHEN** the reading is focused with a row `Type` selected and the reader presses `i`, then Escape
- **THEN** the card for the property `Type` SHALL open and its text SHALL be announced, then it
  SHALL close, the row still selected

#### Scenario: A doc cannot inject markup
- **WHEN** a declaration's doc holds `<img src=x onerror="alert(1)">` and a link to
  `javascript:alert(1)`
- **THEN** the card SHALL show the tag as text and the link's text without a link, and no script
  SHALL run

#### Scenario: A host's Markdown
- **WHEN** the host sets `markdown` to a function and the pointer rests on a documented element
- **THEN** the card SHALL show what that function returns for the doc
