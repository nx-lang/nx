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
Its properties SHALL be `tree`, `text`, `colors`, `icons`, `markdown`, `folded`, `selection`,
`stale` and `editable`, and a property set before the element is defined SHALL take effect once it is.
Where the reading shows a string without its quotes, or with quiet quotes of its own, as a property's
value, UI text or an id, it SHALL read the string's `\"` and `\\` escapes as the quote and backslash
they stand for; a string drawn as code, between its own quotes, and a string holding any other
escape SHALL read as written. Where there
is no DOM, importing the package SHALL do nothing, and the package SHALL export a function that
defines the element in a registry the host chooses.

#### Scenario: A plain page
- **WHEN** a page with no framework imports the package, adds `<nx-viewer>` and sets its `tree` and
  `text`
- **THEN** the element SHALL show the document's reading

#### Scenario: Host styles don't reach in
- **WHEN** the host page styles `div`, `span` or `button` elements globally
- **THEN** the element's rendering SHALL not change

#### Scenario: A quote in a string
- **WHEN** a property's value, shown in a property sheet, is written `"Say \"hi\""`
- **THEN** the reading SHALL read `Say "hi"`, as its field does

#### Scenario: A stale tree
- **WHEN** the host sets the element's `stale` property while it computes a newer tree
- **THEN** the element SHALL mute its reading and mark it out of date, and selection SHALL still work

#### Scenario: Stale for a moment
- **WHEN** the host sets `stale` and clears it again within half a second, as while an edit's tree is
  on its way
- **THEN** the reading SHALL NOT be muted or badged, and showing or hiding the badge SHALL NOT move
  the reading

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

### Requirement: An element reads as a group where its properties do not fit on a line
An element SHALL read as a group when it holds an element, or when it shows more than one property and
they do not fit after its name on one line in the pane, however the document breaks its lines. The
element SHALL measure which elements those are as it draws, and again once a change in the pane's
width settles or its fonts load, keeping the node at the top of what scrolls it where it was, so a
divider dragged across it does not draw it on every step; a reading drawn hidden SHALL be measured
once it is shown. A group SHALL show a header holding its icon, its name and its id;
when its properties fit on that line, they SHALL follow on it as `name=value`, the id after them when
its visualizer puts it at the end, and when they do not, each property SHALL have a row, with the
property's name in one column and its value in another. A property whose value needs lines of its
own, such as a list, an element that holds elements, or an element too long for its row, SHALL have a
row either way. Under them SHALL
come what the element holds, indented. The group SHALL show no angle brackets and no closing tag. A
bracket SHALL run down the group from under its name and turn right at its end. A value of several
lines SHALL read under its name rather than in the value column, and in a pane narrower than about 64
characters every value SHALL read under its name. A property's element value SHALL read in its row or
on its line as a line when it fits there, and otherwise, or when it holds elements, under the
property's name as a group, measured where the group holding it shows it, on its header or its row. What the
element holds SHALL read an item to a line: an element in its own form, and anything else, such as an
`if` or a `for` block, or an element written as its tags because a group cannot show it, as written,
its later lines keeping their indentation relative to its own start, set in once. A blank line the
document leaves between two held items SHALL become a little room, and so SHALL the space between two
siblings where either is a group. A line too long for the pane SHALL wrap a step in from where its
unit starts in a row's value, a line that shows one property, and an item a group holds as written;
code read as written elsewhere SHALL keep its lines.

#### Scenario: Attributes a line each
- **WHEN** a document holds `<Step` with `id="welcome"`, `title="Welcome"` and `note="First"` each on
  a line of its own, the visualizer of `Step` names `id` as its id, and the pane is too narrow for
  `Step welcome title=Welcome note=First` on one line
- **THEN** the reading SHALL show a group headed `Step welcome` with a row `title` and `Welcome` and a
  row `note` and `First`, and no `<`, `/>` or quotes

#### Scenario: Content under a header
- **WHEN** a document holds `<Row gap="wide">` on one line, two `<Item />` elements on the lines after
  it with a blank line between them, and `</Row>`
- **THEN** the reading SHALL show the header `Row  gap=wide`, the two items under it with a little
  room between them, and no closing tag

#### Scenario: Several attributes on a wrapped line
- **WHEN** a document holds `<Shape kind="circle" width=52` on one line, `color="#6652AD" stroke=1>`
  on the next, an `<Item name="a" />` and `</Shape>`, in a wide pane
- **THEN** the reading SHALL show the header `Shape  kind=circle  width=52  color=#6652AD  stroke=1`,
  no rows, the item under it, and no closing tag

#### Scenario: A pane narrowed past a header
- **WHEN** the pane showing that `Shape` narrows until its header no longer fits on one line
- **THEN** the reading SHALL show the header `Shape`, a row each for `kind`, `width`, `color` and
  `stroke`, and the item still as the line `Item  name=a`

#### Scenario: An element value too long for its row
- **WHEN** a `Card` too long for the pane has `padding=<Thickness left=1 top=2 />` and
  `style=<Style color="red" font="Inter" size=12 />`, and the `Style` line does not fit in its row
- **THEN** the reading SHALL show the row `padding` with the line `<Thickness  left=1  top=2/>`, and
  under the row `style` a group headed `Style` with a row each for `color`, `font` and `size`

#### Scenario: An item written as its tags over several lines
- **WHEN** a group holds an element with a comment inside an attribute, written over two lines with
  its second line indented two spaces past its `<`
- **THEN** the reading SHALL show the element as written, its second line two spaces in from its `<`

### Requirement: An element reads as a line where its properties fit
An element that holds no element and no more than a line of text, and whose properties fit after its
name on one line in the pane, SHALL read as a line however the document breaks its lines, and so
SHALL one that shows one property at most, which a row would not shorten, wrapping where it must. A
line SHALL show its icon and name, its id where its visualizer puts it, the value of the attribute
its visualizer names as its title alone, each other attribute as `name=value`, and then what it
holds as written, with no tags of its own. When the element shares its line with other text, such as
a property's name in a group or a `let` before it, the line SHALL keep the `<` before its name and a
`/>` after it, so its extent stays clear; alone on its line, it SHALL show neither. An element alone
on its line whose type is exactly the type its place declares, through any list, SHALL leave out its
name when it has an icon, which then names it.

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

#### Scenario: Attributes on lines of their own that fit on one
- **WHEN** a document holds `<Step` with `id="welcome"` and `title="Welcome"` each on a line of its
  own, the visualizer of `Step` names `id` as its id and `title` as its title, and the pane is wide
- **THEN** the reading SHALL show the line `Step welcome Welcome`, with no `<`, `/>` or quotes

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
A click SHALL select the unit it falls in: an element read as a group, by its header or the room
between its rows; a row of a group; an element read as a line; an element written as its tags that
starts a line; or a block written as it is, such as an `if`. When the host has set `editable`, a
click on an editable value SHALL instead open its editor and leave the selection as it was. Outside every unit, a click SHALL select the
smallest node under it, and a drag that selects text SHALL select nothing. Selecting SHALL fire
`nx-select` with the node's key, role and range, and tell a screen reader what was selected. The host
SHALL be able to select a node by key through `selection` without the event; a key the tree does not
hold SHALL clear the selection. The up and down arrow keys SHALL select the previous and next unit, the
left arrow the unit that holds the selected one, and the right arrow the first one it holds. A
selection that is not itself a unit, such as a node the host selects inside a row, SHALL move from the
unit that holds it, or, outside every unit, to the nearest unit after or before it. The
selected unit and the one under the pointer SHALL be drawn as rows behind the text, from just before
the unit's start to near the reading's right edge, or, for an element inside a line, as a property's
value, to just past its end, a selected group's first line stronger than the rest of it, in colors a host sets through `--nx-viewer-selection`, `--nx-viewer-selection-inactive` and
`--nx-viewer-hover`.

#### Scenario: Selecting a line
- **WHEN** a reader clicks a word of an element on one line that is not an editable value
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
by how many properties and children it hides, cut to one line; a declaration to its first line. The
children SHALL be counted as the group shows them open: each element and each expression that gives
elements, such as `{Children}`, once, and inside an `if`, a match or a `for`, what every branch or its
body writes, while text and comments SHALL NOT count. A run of
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

#### Scenario: A folded group counts its children
- **WHEN** the reader folds a group with no title that holds an element, a comment, an `if` writing
  two elements and an `else` writing one, a `for` writing one, and a match with two arms writing one
  each
- **THEN** the group SHALL show its header and `7 children`

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

### Requirement: A host can carry a selection and a place to its editor
The package SHALL export `keyAtRange(tree, text, start, end?)`, the key of the smallest node holding
the text between two UTF-16 offsets, whitespace at either end left out, or a cursor's node without
`end`; and `rangeOfKey(tree, text, key)`, the UTF-16 offsets where that node starts and ends. The
element SHALL offer `keyAtTop()`, the first node drawn at or below the top of what scrolls the
reading and how many pixels below it is; `topOf(key)`, how many pixels below that top the node, or the
nearest drawn node holding it, is drawn; and `scrollToKey(key, top)`, which scrolls the reading so
that node is drawn `top` pixels below the top. What scrolls the reading SHALL be its nearest
scrolling ancestor, or the page.

#### Scenario: A selection of whole lines
- **WHEN** a host passes a selection that runs from the start of an element's line through the
  newline after it
- **THEN** `keyAtRange` SHALL give the element's key

#### Scenario: The same place in another view
- **WHEN** a host reads `keyAtTop()`, shows its editor scrolled so the line where that node starts is
  as far below the top, and later calls `scrollToKey` with a node and its line's distance from the
  editor's top
- **THEN** the reading SHALL be scrolled so that node is drawn that far below its top

#### Scenario: A node folded away
- **WHEN** the node a host asks `topOf` for is inside a folded group
- **THEN** `topOf` SHALL give where the group is drawn

### Requirement: Values edit in place when the host allows it
When the host sets `editable`, the element SHALL let the reader edit a value where it reads; otherwise
it SHALL offer no editing and read as it does without this property. A value SHALL be editable when it
is a literal or a bare union case that is a whole value (an attribute's value, an item of a braced
sequence, or a parameterless `let`'s value), or a run of plain text content written on one line, and
an editor fits it:

- a string, a `uitext` string or an id: a text field holding the value without quotes or escapes;
- a number, typed `int`, `float32` or `float64` or by an alias of one, or a numeric literal where no
  type is declared: a number field whose up and down arrows step by 1, by 10 with Shift, and whose
  property name, dragged sideways, changes the value as the pointer moves;
- a boolean: a toggle, with no field;
- a bare case of a union: a popover listing the union's cases that hold no properties, each with its
  doc, the current one marked, filtered as the reader types;
- a string a visualizer formats as `color`: a popover with a hex field and the browser's color picker;
- a run of text: a text field that takes one line.

The element SHALL find a value's type through its element's declaration and the aliases it names, in
the tree's declaration table. An editor SHALL take the value's own place in the reading, and a popover
SHALL open over the reading, so that opening an editor moves nothing else. A popover SHALL be placed
by its value before anything in it is brought into view, and SHALL scroll only its own list, never the
reading. A click on an editable value SHALL edit it and SHALL leave the selection as it was: a field SHALL open with a
collapsed caret where the click fell, a click on a check or cross SHALL toggle the boolean, and a
case or a color SHALL open its popover; a click anywhere else in a unit, as on a property's name,
SHALL select the unit and open nothing. The second click of a double-click SHALL NOT toggle a boolean
again, and a click on a value whose popover is open SHALL close it, as a dropdown's does. Enter or F2 with a unit selected SHALL open the editor of its value, or of its first value
when it holds several, with the field's text selected; Space SHALL toggle a boolean. Under the
pointer, an editable value SHALL show a faint mark and a text cursor, or a hand for one that toggles
or opens a popover, and a case SHALL show a chevron after it. An open field SHALL read as an input,
with a ring of its own apart from the selection's color. Enter or leaving the field SHALL commit, Escape
SHALL cancel and keep the selection, and Tab and Shift+Tab SHALL commit and open the next and previous
editable value, the selection again left as it was. A text field SHALL grow with its text and wrap within the reading's width. A number
SHALL be committed once when a drag ends, and an entry an editor cannot take, such as a number field
holding `12px`, SHALL stay in the field marked as invalid and SHALL NOT be committed; once typing
pauses, or at once when the reader tries to commit it, a note beside the field, or under it where the
reading has no room beside it, SHALL say why, and SHALL go when the entry can be taken or the editor
closes. Opening an editor SHALL close any hover card.

#### Scenario: Editing a string
- **WHEN** the host has set `editable`, the reader selects the row `name` of a `Drink` whose name is
  `"Latte"`, presses Enter, types `Flat white` over it and presses Enter
- **THEN** the element SHALL fire `nx-edit` setting the string value of the row's literal to
  `Flat white`, and SHALL read `Flat white` until the host sets a newer tree

#### Scenario: Picking a case
- **WHEN** the reader opens the editor of `size=medium`, typed `Size = small | medium | large`
- **THEN** a popover SHALL list `small`, `medium` and `large`, `medium` marked, and choosing `large`
  SHALL fire `nx-edit` setting the case `large`

#### Scenario: A click edits where it falls
- **WHEN** the host has set `editable` and the reader clicks in the middle of the value `"Latte"`,
  with nothing selected
- **THEN** a field SHALL open on `Latte` with the caret between the characters the click fell
  between and no text selected, and nothing SHALL be selected

#### Scenario: A click on a name selects
- **WHEN** the reader clicks the name of the row `name`
- **THEN** the row SHALL be selected and no editor SHALL open

#### Scenario: Toggling
- **WHEN** a row `iced` reading a cross is selected and the reader presses Space, or clicks the cross
- **THEN** the element SHALL fire `nx-edit` setting the boolean `true`

#### Scenario: Cancelling
- **WHEN** the reader opens a text field, types, and presses Escape
- **THEN** no `nx-edit` SHALL fire, the value SHALL read as before and the unit SHALL stay selected

#### Scenario: Not editable by default
- **WHEN** the host has not set `editable` and the reader double-clicks a value
- **THEN** no editor SHALL open and no `nx-edit` SHALL fire

#### Scenario: Saying why
- **WHEN** the reader types `12px` in a number field and presses Enter
- **THEN** no `nx-edit` SHALL fire and a note beside the field SHALL read `Not a number`

#### Scenario: Nothing moves
- **WHEN** the reader opens the editor of a value on a one-line element
- **THEN** every other part of the reading SHALL stay where it was drawn

### Requirement: A property with a default, or an optional one, can be left out
The element SHALL let the reader leave out a property that has a default or is optional: when the
host has set `editable` and such a property's row is selected, Delete, or Cmd+Backspace (a Mac
laptop's delete key is Backspace), SHALL remove it. Backspace alone SHALL NOT, since a reader who meant
to change the text would press it. The row's menu SHALL offer the same removal, as Reset to default
for a property with a default and as Remove for an optional one, and the element SHALL offer no
button for it that a single click would take. A row whose property is required SHALL NOT be removed. After the removal the element SHALL
select the element that held the row.

#### Scenario: Resetting
- **WHEN** the row `size` of a `Drink` whose `size` defaults to `medium` is selected and the reader
  presses Delete
- **THEN** the element SHALL fire `nx-edit` removing that attribute

#### Scenario: Backspace
- **WHEN** the row `color` of a `Drink`, whose `color` is optional, is selected and the reader presses
  Backspace
- **THEN** no `nx-edit` SHALL fire

#### Scenario: A required property
- **WHEN** the row `name` of a `Drink`, whose `name` is required, is selected and the reader presses
  Delete
- **THEN** no `nx-edit` SHALL fire

### Requirement: Edits and requests for the source reach the host as events
The element SHALL fire `nx-edit` for each committed edit, carrying the target node's key and the edit
as the `source-edits` capability types it, and SHALL NOT change the text or the tree itself: it SHALL
show the edited value, muted as stale, until the host sets a newer tree, and SHALL then read the tree
as it is. The element SHALL keep the selection by key across the newer tree. With the reading focused
and no editor open, the element SHALL leave Ctrl or Cmd with Z or Y to the host. When the host has
set `editable`, Enter or a double-click on a value the element cannot edit, such as an expression,
SHALL fire `nx-open` with the node's key, role and range. The element SHALL fire `nx-draft` with
`changed` true when an open editor comes to hold an entry that differs from its value, and with
`changed` false when it no longer does, as when it commits, cancels or closes.

#### Scenario: The host refuses
- **WHEN** the element fires `nx-edit` and the host sets the same tree again without changing the text
- **THEN** the element SHALL read the value as the tree gives it

#### Scenario: A change not yet committed
- **WHEN** the reader opens the field of `name="Latte"`, types `Mocha` over it and presses Enter
- **THEN** the element SHALL fire `nx-draft` with `changed` true, then with `changed` false, then
  `nx-edit`

#### Scenario: An expression
- **WHEN** the host has set `editable` and the reader double-clicks the value of `count={total + 1}`
- **THEN** the element SHALL fire `nx-open` with the key of that value

### Requirement: A selected element offers the properties it does not write
When the host has set `editable` and an element is selected, the element SHALL end it with a quiet
row reading `Show N unset properties` (`property` for one), and `Hide unset properties` while open,
N being how many of the element's declared properties it does not write,
content properties left out; for an element of a union's case, its declared properties are the
union's shared ones and the case's, and a property written in any branch of a property-list
condition or match counts as written; an element that writes all of them SHALL show no such row, and only the
selected element SHALL show one. Opening the row, by a click or Enter, SHALL draw each unwritten
property under it as a dimmed row, in the order its resolved visualizer lists properties, else in
declaration order: its name, and its default read as the reading reads values, followed by a quiet
`default`, or, for an optional property, its type, quieter, in a place for the value wide enough to
click and type in, which a field opened there SHALL keep, its type shown until something is typed.
The rows SHALL be drawn under the element, where it starts on its line, inside its highlight. A dimmed row's name SHALL show its property's hover
card. With more than 12 such rows a filter field SHALL lead them, narrowing them as the reader types.
A dimmed row whose type has an editor SHALL edit as a written value does, opening on its default; a
commit SHALL fire `nx-edit` with the element's key and `addProperty`, the property's name and the
value, and the row SHALL then read as written, where it stands, until the host's newer tree places
it. A dimmed row whose type has no editor, such as a record or an element, SHALL fire `nx-open` with
the element's key on Enter or a double-click. Selecting another unit, or Escape on the open list,
SHALL close it.

#### Scenario: Seeing what else a drink can set
- **WHEN** the reader selects `<Drink code="LAT" name="Latte" size=small />`, whose type declares
  `code`, `name`, `note?`, `size`, `iced` (default `false`), `shots` (default `1`) and `color?`
- **THEN** the element SHALL end with `Show 4 unset properties`, and opening it SHALL show the rows
  `note` reading `string`, quieter, `iced` reading `✗ default`, `shots` reading `1 default` and
  `color` reading its type

#### Scenario: Setting one
- **WHEN** the reader clicks the cross of the dimmed row `iced`
- **THEN** the element SHALL fire `nx-edit` with the drink's key and `addProperty` for `iced` with the
  boolean `true`

### Requirement: A selected row has a menu
When the host has set `editable`, a written row SHALL show a quiet `⋯` at its end while the pointer is
over it and while it is selected. Opening it SHALL list Reset to default for a property with a
default, followed, quieter, by the default as the reading draws it when it is a short literal, Remove
for an optional one, and Show in Source for every row; Reset and Remove SHALL fire
`nx-edit` removing the attribute, as Delete does, and Show in Source SHALL fire `nx-open` with the
row's value. The menu SHALL open over the reading, move nothing, and close on Escape, a choice or a
click elsewhere. Dimmed rows SHALL have no menu.

#### Scenario: Resetting from the menu
- **WHEN** the reader opens the menu of the row `size=large` of a `Drink` whose `size` defaults to
  `medium` and chooses Reset to default
- **THEN** the item SHALL have read `Reset to default` and `medium`, and the element SHALL fire
  `nx-edit` removing that attribute

### Requirement: Editor hints choose and shape a value's editor
The element SHALL edit a value as its property's resolved editor hints, from the tree's visualizer
values, say:

- a number's field, arrows, steppers and drag SHALL stay within `min` and `max` and step by `step`,
  Shift by ten steps, and a typed number outside them SHALL stay in the field with a note saying the
  bound, such as `At most 64`, and SHALL NOT be committed; `editor=field` asks for the number field,
  which every number has;
- a case with `editor=segmented` SHALL show the union's cases as one row of buttons in its popover,
  unless there are more than five or they do not fit the reading's width, when it SHALL list them;
  `editor=dropdown` SHALL list them;
- a string with `multiline` SHALL edit in a field that holds line breaks, where Enter makes a line
  break and Ctrl or Cmd+Enter commits;
- a field whose property has a `placeholder` SHALL show it, quieter, while the field is empty, and
  SHALL NOT write it.

A number or a string whose hints give `choices` SHALL offer them in a list under its field, Paper-style:
each choice reading its label, and its value after it, quieter, in a second column; a choice with no
label reads its value alone, in one column when none has a label. When the property has a default, a
row of its own above the choices, set off by a rule, SHALL read `Default` and the default. The choice
equal to the current value SHALL be marked. While the field is open, a chevron past it SHALL show
that the list is there, and so SHALL one past the value while the pointer is over it, as the steppers
below do; a click on it, or Alt+Down, SHALL open the list (a
click on the hover chevron opening the field with it), and so SHALL typing, the list
then holding the choices whose label or value holds what was typed. While the list is closed, Up and
Down SHALL step a number; while it is open, they SHALL move through it, from the current value's row
when no row is active yet, Enter SHALL choose the active
row, and Escape SHALL close the list before the field. Choosing a choice SHALL commit its value as
the field would: a value the field would refuse SHALL stay in the field, the list closed, with the
field's note. Choosing the default's row SHALL leave the attribute out, as the row's Reset to
default does, or, on a dimmed row, add nothing. The field SHALL stay typeable: the choices suggest
and never restrict, and any entry the field takes SHALL commit.

A number SHALL show steppers, an up and a down button drawn over the space just past the value so
that nothing moves, while its field is open and while the pointer is over its value, selected or
not, when the host has set `editable`. On a one-line element, or a group's header, where more of the
line follows the value, its closing `/>` included, the steppers and the chevron SHALL NOT cover it:
they SHALL take room in the line, moving what follows along, as a field does when it grows, and
SHALL give it back when they go. So SHALL an earlier item of a braced list, and an unset property's
value, whose quiet `default` moves aside. Where taking room would wrap a line that only just fits,
the tools SHALL cover the gap instead, so nothing below moves. Outside the selection, where they take
room, they SHALL wait until the pointer has rested on the value for a moment, so a line does not jump
as the pointer passes over it; an unset property's row counts as its element's selection. A click SHALL step the number once by `step`, or by one, and
Shift+click by ten steps, within `min` and `max`: into the field while it is open, else committed at
once. Pressing a stepper and dragging up or down SHALL change the number by a step for every few
pixels and show it in place: on release, into the field while it is open, else committed once. The steppers' tooltip SHALL name the arrow
keys, Shift and dragging. Dragging the property's name sideways SHALL still change the number. A
case's chevron SHALL show past the value as the choices chevron does, under the same rules for
hover and for room on a line, SHALL stay while the pointer moves onto it, and a click on it SHALL
open the case's popover, as a click on the value does, or close it when open.

In any string field, Shift+Enter SHALL make a line break. A string literal written over several lines
SHALL edit in a field that holds line breaks; one written with an escape other than `\"` and `\\`
SHALL still fire `nx-open`.

#### Scenario: Picking a labeled choice
- **WHEN** the reader opens `size=16`, hinted with the choices `12` labeled `sm`, `16` labeled `base`
  and `20` labeled `lg`, where `size` defaults to `16`, opens the list and chooses `lg`
- **THEN** the list SHALL have shown a `Default 16` row above `sm 12`, `base 16` and `lg 20`, `base`
  marked, and the element SHALL fire `nx-edit` setting the number `20`

#### Scenario: Back to the default from the list
- **WHEN** the reader opens the list of the written `size=20` above and chooses its `Default 16` row
- **THEN** the element SHALL fire `nx-edit` removing the attribute

#### Scenario: Typing past the choices
- **WHEN** the reader types `17` in that field and presses Enter
- **THEN** the element SHALL fire `nx-edit` setting the number `17`

#### Scenario: A stepper
- **WHEN** the reader points at `padding=8`, selected or not, and clicks the up stepper past its
  value, and then Shift-clicks it
- **THEN** the element SHALL fire `nx-edit` setting the number `9`, and then the number `19`

#### Scenario: A stepper dragged
- **WHEN** the reader presses the up stepper of `gap=4` on its selected row, drags up, and lets go
  at `14`
- **THEN** the number SHALL read `14` in place while dragging, and the element SHALL fire `nx-edit`
  setting the number `14` once, on release

#### Scenario: Room on a line
- **WHEN** the reader selects `<Banner title="Fresh" titleSize=20 align=start badge="New" />`, on one
  line, with `titleSize` and `badge` hinted with choices, and points at `titleSize`, then at `badge`
- **THEN** the steppers and chevron past `titleSize` SHALL sit in the line, before `align`, and the
  chevron past `badge` SHALL sit before the closing `/>`, covering neither

#### Scenario: A case's chevron
- **WHEN** the reader points at `size=large` in `<Drink code="SPC" size=large />`, on one line, moves
  the pointer onto the chevron past it, and clicks it
- **THEN** the chevron SHALL sit before the closing `/>`, covering nothing, SHALL stay shown, and the
  popover of `Size`'s cases SHALL open

#### Scenario: Resting before making room
- **WHEN** the reader, with nothing selected, moves the pointer across `titleSize` on that line
  without stopping, and then rests on it
- **THEN** nothing in the line SHALL move as the pointer passes, and once it rests the steppers and
  chevron SHALL take room before `align`

#### Scenario: A refused choice
- **WHEN** the reader opens a number hinted `max=10` with the choices `5` and `99`, and chooses `99`
- **THEN** no `nx-edit` SHALL fire, and the field SHALL stay open holding `99` with the note
  `At most 10`

#### Scenario: Out of range
- **WHEN** the reader types `80` in the field of a number hinted `max=64` and presses Enter
- **THEN** no `nx-edit` SHALL fire and a note beside the field SHALL read `At most 64`

#### Scenario: A line break in a string
- **WHEN** the reader edits `note="Ask for oat milk"`, with no hints, and presses Shift+Enter after
  `Ask` and then Enter
- **THEN** the element SHALL fire `nx-edit` setting a string holding a line break after `Ask`

