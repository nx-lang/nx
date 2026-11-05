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
comment, rendered from Markdown, in its prose face. An element's card SHALL instead be titled by
the element's name in the prose face, bold, after a quiet chip naming its declaration's kind
(`component`, `function`, `type`, `action` or `case`) and followed quietly by the base it extends,
with no other keyword or element syntax. It SHALL show:

- over an element's icon, or its name where it has no icon, and over the line of doc its
  inspector opens with: its name and the base its declaration extends, as a title, and the
  declaration's doc; over a row's name beside its icon, and over the value that names a row,
  nothing, so that resting there to drag or select it opens nothing;
- over a field's label, or a property's name where the reading writes one: `(property) name: Type`, with `?` for an
  optional property and ` = default` for one with a default, and the property's doc;
- over a value the reading shows without its property's name in a field, as an element value's
  title: that property's card;
- over a union case written as a value, bare or qualified: `(case) Union.case` and the case's doc;
- over a reference or a call written as a value: its name and type, and the doc of the declaration
  it refers to when that has one;
- nothing over a literal, a comment, punctuation, or anything for which the tree gives neither a
  type nor a doc.

The first card SHALL open after the pointer has rested for about a second; while a card is open, or
for about a second and a half after one closed, the next SHALL open at once, as a design tool's
tooltips do. No card SHALL open while a button is held, so none opens during a drag, nor for what
the pointer was over when it pressed, until it moves onto something else, nor for what the reading
scrolls under a still pointer, until the pointer moves. A card SHALL stay open while the
pointer moves from what it describes onto the card, so its text can be selected and its links
followed, and SHALL close when the pointer leaves both, on a press, when the reading scrolls, when the reader
presses Escape, or when the text or the tree changes. It SHALL NOT change the selection, the folds
or the scroll position. With the reading focused, the `i` key SHALL show the card for the selected
unit (the declaration of an element, the property of a row) and a screen reader SHALL hear its text.
The card SHALL stay inside the element's box, below what it describes or above it where there is no
room, never over it. It SHALL start a few pixels past the right end of what it describes, or past
the pointer when that is wide, such as a doc line, so the labels below stay readable while the
reader scans, and SHALL move left only as far as the box needs. It SHALL follow the page's light or dark scheme. What a card says SHALL NOT
also show as the browser's tooltip.

A doc's Markdown SHALL be rendered by the element's `markdown` function when the host sets one, and
otherwise by a built-in renderer that SHALL create text and elements only, never parse HTML: it
SHALL render paragraphs, inline and fenced code, emphasis, lists and links, a link only when its
target is `http:` or `https:`, opening outside the page, and anything else, raw HTML included, as
text.

#### Scenario: An element's declaration
- **WHEN** the pointer rests on the icon of `<Shape Type=Circle />`, whose component `Shape` extends
  `Node` and is documented "A filled shape."
- **THEN** a card SHALL open titled `Shape`, after a `component` chip and before `extends Node`, and showing `A filled shape.`

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

#### Scenario: Cards wait, then follow the pointer, and keep out of a press
- **WHEN** the pointer rests on the `Type` label for half a second, then presses there
- **THEN** no card SHALL open, and none SHALL open for `Type` while the pointer stays on it
- **WHEN** the pointer moves onto `Circle` and rests there for a second, then onto a label
- **THEN** `Circle`'s card SHALL open, and the label's SHALL open at once after it

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
  type is declared: a number field whose up and down arrows step by 1, by 10 with Shift, and that
  changes as its label is dragged left or right;
- a boolean: a toggle, with no field;
- a bare case of a union: a popover listing the union's cases that hold no properties, each with its
  doc, the current one marked, filtered as the reader types;
- a string a visualizer formats as `color`: a popover with a hex field and the browser's color picker;
- a color literal in an inspector: its parts in its value's place, as a design tool shows a color: its
  swatch, its six-digit hex and its opacity in whole percent. A click on the swatch SHALL open the
  browser's color picker, which keeps the opacity; a click on the hex SHALL edit it in place, keeping the
  opacity; a click on the opacity SHALL edit its percent in place, keeping the hex; and dragging the `%`
  sideways SHALL scrub the opacity by 1 percent for each 3 pixels, by 10 with Shift, between 0 and 100,
  committed once on release. A part left as it opened SHALL write nothing. Elsewhere a color literal
  SHALL read as written and open its popover;
- a run of text: a text field that takes one line.

The element SHALL find a value's type through its element's declaration and the aliases it names, in
the tree's declaration table. An editor SHALL take the value's own place in the reading, and a popover
SHALL open over the reading, so that opening an editor moves nothing else. A popover SHALL be placed
by its value before anything in it is brought into view, and SHALL scroll only its own list, never the
reading. A click on an editable value SHALL edit it and SHALL select its property: a field SHALL open with a
collapsed caret where the click fell, a click on a check or cross SHALL toggle the boolean, and a
case or a color SHALL open its popover; a click anywhere else in a unit, as on a property's name,
SHALL select the unit and open nothing. The second click of a double-click SHALL NOT toggle a boolean
again, and a click on a value whose popover is open SHALL close it, as a dropdown's does. Enter or F2 with a unit selected SHALL open the editor of its value, or of its first value
when it holds several, with the field's text selected; Space SHALL toggle a boolean. Under the
pointer, an editable value SHALL show a faint mark and a text cursor, or a hand for one that toggles
or opens a popover. An open field SHALL read as an input,
with a ring of its own apart from the selection's color. Enter or leaving the field SHALL commit, Escape
SHALL cancel and keep the selection, and Tab and Shift+Tab SHALL commit and open the next and previous
editable value, selecting its property. A text field SHALL grow with its text and wrap within the reading's width. A number
SHALL be committed once when a drag ends, while the drag's values go live as the next requirement
says, and an entry an editor cannot take, such as a number field
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
  between and no text selected, and its property SHALL be selected

#### Scenario: A click on a name selects
- **WHEN** the reader clicks the name of the row `name`
- **THEN** the row SHALL be selected and no editor SHALL open

#### Scenario: Toggling
- **WHEN** a row `iced` reading a cross is selected and the reader presses Space, or clicks the cross
- **THEN** the element SHALL fire `nx-edit` setting the boolean `true`

#### Scenario: Cancelling
- **WHEN** the reader opens a text field, types, and presses Escape before typing pauses
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

### Requirement: A selected row has a menu
When the host has set `editable`, a written row SHALL show a quiet `⋯` just past its value, or past the
value's tools when they show, while the pointer is over it and while it is selected. Opening it SHALL list Reset to default for a property with a
default, followed, quieter, by the default as the reading draws it when it is a short literal, Remove
for an optional one, and Show in Source for every row; Reset and Remove SHALL fire
`nx-edit` removing the attribute, as Delete does, and Show in Source SHALL fire `nx-open` with the
row's value. The menu SHALL open over the reading, move nothing, and close on Escape, a choice or a
click elsewhere. Dimmed rows SHALL have no menu. A property written on a line SHALL have the same
menu: a click on its name or its `=` SHALL select the property, drawn just around it, and while it
is selected its `⋯` SHALL show right after it, taking room in the line, and Delete SHALL remove it as it removes a row.

#### Scenario: Resetting from the menu
- **WHEN** the reader opens the menu of the row `size=large` of a `Drink` whose `size` defaults to
  `medium` and chooses Reset to default
- **THEN** the item SHALL have read `Reset to default` and `medium`, and the element SHALL fire
  `nx-edit` removing that attribute

#### Scenario: Removing a property written on a line
- **WHEN** a `Drink` reads on one line with the optional `color="#6f4e37"`, and the reader clicks
  `color` and opens the `⋯` shown after it
- **THEN** the menu SHALL list Remove and Show in Source, and Remove SHALL fire `nx-edit` removing
  that attribute

#### Scenario: Clicking a label selects
- **WHEN** the reader clicks the label of `size=16` without dragging
- **THEN** the property SHALL be selected and its field SHALL take focus, and Up SHALL make it `17`

### Requirement: Editor hints choose and shape a value's editor
The element SHALL edit a value as its property's resolved editor hints, from the tree's visualizer
values, say:

- a number's field, arrows and scrubbing SHALL stay within `min` and `max` and step by `step`,
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
that the list is there, and so SHALL one past the value while its property is selected; a click on it, or Alt+Down, SHALL open the list (a
click on the selected property's chevron opening the field with it), and so SHALL typing, the list
then holding the choices whose label or value holds what was typed. While the list is closed, Up and
Down SHALL step a number; while it is open, they SHALL move through it, from the current value's row
when no row is active yet, Enter SHALL choose the active
row, and Escape SHALL close the list before the field. Choosing a choice SHALL commit its value as
the field would: a value the field would refuse SHALL stay in the field, the list closed, with the
field's note. Choosing the default's row SHALL leave the attribute out, as the row's Reset to
default does, or, on a dimmed row, add nothing. The field SHALL stay typeable: the choices suggest
and never restrict, and any entry the field takes SHALL commit.

A
case's chevron SHALL show inside its field, as the choices chevron does, and a click on it SHALL open the case's popover, as a click on
the value does, or close it when open.

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

#### Scenario: Stepping with the keyboard
- **WHEN** the reader focuses the field of `padding=8` and presses Up, and then Shift+Up
- **THEN** the element SHALL fire `nx-edit` setting the number `9`, and then the number `19`

#### Scenario: A case's chevron
- **WHEN** the reader opens the inspector of `<Drink code="SPC" size=large />`, selects the field of
  `size=large` and clicks the chevron past its value
- **THEN** the chevron SHALL sit inside the field, covering nothing, SHALL stay shown, and the popover
  of `Size`'s cases SHALL open

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

### Requirement: An item of a list is dragged by its icon
When the host has set `editable`, the element SHALL let the reader move an item that moves: an
element that is an item of a sequence or of element content, not an item of a condition's branch or a loop's body, in a list of two or more items that holds no text. Where the reading draws such a list
an item to a line, the item's handle SHALL be its icon, with the element's name beside it where the
reading shows one, or its name alone where the host draws no icons; the element SHALL add no mark of
its own for it. Over the handle the pointer SHALL keep its usual cursor, showing a grabbing hand only
while a drag is under way, and the hover card the icon or name shows SHALL end with a quiet `Drag to
reorder`. A click on the handle SHALL select the item, as
today; pressing it and moving the pointer 4 pixels or more SHALL instead start a drag and close any
hover card. While dragging, the item SHALL be drawn muted where it stands, a copy of one line holding
its icon and its title, or its name, SHALL follow the pointer, and a line in the selection colour SHALL
mark the gap between items of the item's own list nearest the pointer, except the two gaps beside
the item, where it SHALL show no line; nothing else in the reading SHALL move. Releasing over the list at a gap that does not leave the item where it is SHALL fire
`nx-edit` with the item's key and `move` to the position that gap gives it. Releasing anywhere else,
or Escape, SHALL cancel the drag and fire nothing. While the pointer is near the top or bottom edge of
the reading's scrolling area, the reading SHALL scroll toward it. The drag SHALL work with a mouse, a
pen and a touch on the handle. An item that shows neither an icon nor a name moves only as the next
requirement says.

#### Scenario: Dragging a step down
- **WHEN** the reader drags the icon of the first of four steps and releases it with the line marking
  the gap between the third and the fourth
- **THEN** the element SHALL fire `nx-edit` with the first step's key and `move` to `2`, and nothing
  else in the reading SHALL have moved during the drag

#### Scenario: Cancelling
- **WHEN** the reader presses Escape during a drag
- **THEN** the drag SHALL end, the item SHALL read as before, and no event SHALL fire

#### Scenario: A click on the icon
- **WHEN** the reader clicks the icon of a step without moving the pointer
- **THEN** the step SHALL be selected and no drag SHALL start

#### Scenario: Nothing to drag
- **WHEN** the reader presses and moves the icon of the only choice of a question, or of an element inside an
  `if` block
- **THEN** no drag SHALL start

### Requirement: An item of a list moves up and down
With the reading focused, no editor open and an item that moves selected, Alt+Up and Alt+Down SHALL
fire `nx-edit` with the item's key and `move` to the position before or after its own, and SHALL do
nothing at the list's start or end. These SHALL work where the reading draws the list on one line as
well, where there is no handle to drag.

#### Scenario: Alt+Down
- **WHEN** the reader selects the second of three choices and presses Alt+Down
- **THEN** the element SHALL fire `nx-edit` with that choice's key and `move` to `2`

### Requirement: An item of a list can be deleted
When the host has set `editable`, the element SHALL let the reader delete an item that can be
deleted: an element that is an item of a sequence or of element content, not an item of a condition's branch or a loop's body, in a list that holds no text, unless it is the only item of a list whose type
needs one or more (`FlowStep+`). With such an item selected, the reading focused and no editor open,
Delete, or Cmd+Backspace, SHALL fire `nx-edit` with the item's key and `remove`; Backspace alone
SHALL NOT. The element SHALL ask for no confirmation, since the host's undo takes the edit back.

#### Scenario: Deleting a choice
- **WHEN** the reader selects the second of three choices and presses Delete
- **THEN** the element SHALL fire `nx-edit` with that choice's key and `remove`

#### Scenario: The only step
- **WHEN** the only step of a `QuestionFlow`, whose `steps` is `FlowStep+`, is selected and the reader
  presses Delete
- **THEN** no `nx-edit` SHALL fire

### Requirement: A selected item has a menu
A selected item that moves or can be deleted SHALL show a quiet `⋯` at the end of its first line, as
a written row does, whose menu lists Move up when the item moves and an item stands before it, Move
down when it moves and one stands after it, Delete when it can be deleted, and Show in Source, which
fires `nx-open` with the item's key; the first three SHALL fire the same `nx-edit` as Alt+Up, Alt+Down
and Delete. The menu SHALL open from the context-menu key and Shift+F10 too, and look and close as a
row's menu does.

#### Scenario: The first item's menu
- **WHEN** the reader opens the `⋯` menu of the first of three steps
- **THEN** it SHALL list Move down, Delete and Show in Source, and no Move up

### Requirement: The selection and folds follow a moved or deleted item
After the element fires a `move`, it SHALL read the host's newer tree with the item at its new
position selected, and after it fires an item's `remove`, with the item that took its place selected,
else the one before it, else the element or row that holds the list. Each fold on an item of that list
SHALL stay on the item it was on, the element firing `nx-fold` with the folded keys as the newer tree
gives them. A polite live region SHALL say what happened, as `Moved to 3 of 8` or `Deleted`. When the
host's newer tree holds the list unchanged, as after a refusal, the selection and folds SHALL stay as
they were.

#### Scenario: The selection follows
- **WHEN** the reader moves the selected first step to `2` and the host sets the newer tree
- **THEN** the step now third SHALL be selected, and the live region SHALL say `Moved to 3 of 4`

#### Scenario: A fold follows
- **WHEN** the second step is folded and the reader moves the first step to `2`
- **THEN** in the newer tree the step now first, which was second, SHALL be folded, and the element
  SHALL fire `nx-fold` with its key

#### Scenario: Deleting the last item
- **WHEN** the reader deletes the last of three choices and the host sets the newer tree
- **THEN** the choice now last, which was second, SHALL be selected

### Requirement: Edits go live while the reader changes a value
While the host has set `editable`, the element SHALL fire `nx-edit` with `live: true` for each value
the reader gives a value before committing it, so the host can show it at once: an open field's entry
once typing pauses for 300 ms, at once when an arrow key or scrubbing changes a number, and at
once when the color picker or a color's hex field changes a color. A live edit SHALL fire only for an
entry the editor can take and that differs from the last value fired, and an entry the editor cannot
take SHALL fire nothing until it can. A live edit SHALL carry the same key and edit as the commit of
that value would, and the reader's next value of the same value SHALL replace it. The value's commit
SHALL end its live edits with `nx-edit` without `live`: on commit, with the value committed, even when
the host's tree already reads it; on cancel, as Escape or leaving a field holding an entry it cannot
take does, with the value the editor opened on, when a live edit has fired. Scrubbing SHALL fire a live edit for each new value and commit once on release. Closing a color's popover by a press
elsewhere SHALL commit the entry it holds when the entry can be taken and differs from the value, as
leaving a field does, and Escape SHALL cancel it. An editor SHALL stay open, with its entry and caret,
across the host's newer tree.

#### Scenario: Arrow keys in an open field
- **WHEN** the reader opens the field of `size=16` and presses Up twice
- **THEN** the element SHALL fire `nx-edit` with `live: true` setting `17`, then `18`, and on Enter
  `nx-edit` without `live` setting `18`

#### Scenario: Editing a color in its parts
- **WHEN** an inspector shows `tint=#33669980` and the reader clicks its opacity, types `25` and
  presses Enter
- **THEN** the value reads `#336699` and `25%`, and the element fires one edit setting `#33669940`
- **WHEN** the reader then drags the `%` 30 pixels to the right and releases
- **THEN** the element fires a live edit for each new opacity and one edit setting `#33669999`

#### Scenario: Dragging in the color picker
- **WHEN** the reader opens a color's popover and the picker reports `#112233`, then `#445566`, then
  the reader presses Escape
- **THEN** the element SHALL fire live edits setting `#112233` and `#445566`, then `nx-edit` without
  `live` setting the color the popover opened on

#### Scenario: Typing
- **WHEN** the reader types `Mocha` into a string field and pauses
- **THEN** the element SHALL fire one live edit setting `Mocha`, and none for the letters before it

#### Scenario: Scrubbing
- **WHEN** the reader drags the label of `size=16` nine pixels to the right and releases
- **THEN** the element SHALL fire live edits setting `17`, `18` and `19`, then one `nx-edit` without
  `live` setting `19`

### Requirement: The package gives a host an edit queue
The package SHALL export `createEditQueue(service, uri, document, onApplied, onUnchanged?)`, which
makes the element's edits in the order they are fired: for each, it SHALL ask `service.sourceEdit` for
the document's text as it is when the edit's turn comes, and apply the answer's text edits to the
document as one step unless the text changed while it was asked. It SHALL call `onApplied` with the new
text after an edit that changed it, and `onUnchanged` after one that was refused, answered with no
edits, answered for text since changed, or failed; a failed edit SHALL NOT hold up the ones after it.
The queue's `after(step)` SHALL run `step` once every edit fired before it has been made or refused.
When the document can apply text edits outside its undo history (`preview`), the queue SHALL apply a
live edit (`live: true`) that way, rebased on the text before the first live edit of that value and
replacing the previous one, and SHALL skip a live edit a newer live edit of the same value supersedes
before its turn. The next edit without `live` of that value SHALL be asked for that text, with the
preview taken back first, and applied as one step, so a gesture is one step of the document's history;
an answer that changes nothing leaves the text as it was before the gesture. An edit of another value,
or `after`, SHALL first make a gesture still open one step of the history. A document without
`preview` SHALL ignore live edits.

#### Scenario: Edits in order
- **WHEN** a host fires two `move` edits on the same item before the first is answered
- **THEN** the second SHALL be asked for the text the first left, and both SHALL be applied

#### Scenario: Text changed meanwhile
- **WHEN** the document's text changes while an edit is being asked
- **THEN** the answer SHALL NOT be applied and `onUnchanged` SHALL be called

#### Scenario: Undo after an edit in flight
- **WHEN** a host calls `after(undo)` while an edit is being asked
- **THEN** `undo` SHALL run after that edit is applied

#### Scenario: A gesture is one step
- **WHEN** a host fires live edits setting a number to `17`, then `18`, then the commit setting `18`
- **THEN** the document SHALL read `18` after each live edit's preview, and its history SHALL hold one
  step, from the number before the gesture to `18`

#### Scenario: Cancelled
- **WHEN** a host fires live edits setting a color, then the edit without `live` setting the color it
  started from
- **THEN** the document SHALL read as it did before the gesture, with no step added to its history

### Requirement: A number's editor keeps within its constraint
The viewer SHALL bound a number's editor by its property's `bounds`, with or without editor hints,
and SHALL edit a value as a number when its declared type is a constrained numeric type, whose
spelling now carries a `where` clause, as it does for the base type. These rules add to "Editor hints
choose and shape a value's editor" for a property with `bounds`, and, but for the step below, change
nothing for one without:
the field, arrows and scrubbing SHALL stay within the bounds, and a typed number outside them
SHALL stay in the field with a note and SHALL NOT be committed. The note SHALL read `At least N` or
`At most N` for an inclusive end and `Above N` or `Below N` for an exclusive end. When the property
also has `min` or `max` hints, the editor SHALL keep within the intersection of the hints and the
bounds, so a hint can narrow the range the editor offers but not widen it; a hint range that does not
overlap the bounds SHALL be ignored and the bounds used alone. Arrows and scrubbing SHALL
NOT step onto or past an exclusive end: a step that would SHALL leave the value where it is. Bounds
arrive with an integer's ends already inclusive. A choice outside the bounds SHALL be refused as a choice outside the hints is. When a
floating-point property has no `step` hint and the range the editor offers (the intersection, the
bounds alone, or with no bounds the hints alone) has both ends, its step SHALL be the largest power of ten not above a tenth of
that range's span, so a `0.0..=1.0` number steps by `0.1`, as does a `0.0..=1000.0` number hinted
`max=1`; an integer's step SHALL stay `1`.

#### Scenario: A constraint alone bounds the arrows
- **WHEN** the reader steps up `opacity=0.9` twice, where `opacity` is `float64 where in 0.0..=1.0` with no visualizer
- **THEN** the first step SHALL commit `1.0` and the second SHALL leave the value at `1.0`

#### Scenario: Out of range
- **WHEN** the reader types `1.5` in that field and presses Enter
- **THEN** no `nx-edit` SHALL fire and a note beside the field SHALL read `At most 1`

#### Scenario: An exclusive end
- **WHEN** the reader types `0` in the field of a number with `bounds` `min` `0` and `minExclusive` `true`
- **THEN** no `nx-edit` SHALL fire and the note SHALL read `Above 0`

#### Scenario: A hint narrows the constraint
- **WHEN** `columns` is `int where in 1..=1000`, hinted `max=12`, and the reader types `20`
- **THEN** the note SHALL read `At most 12`
- **AND** with the hint `max=2000` instead, typing `1500` SHALL show `At most 1000`

#### Scenario: The step follows the narrowed range
- **WHEN** `dim` is `float64 where in 0.0..=1000.0`, hinted `max=1` with no `step`
- **THEN** the editor SHALL offer `0..=1` and step by `0.1`
- **AND** a plain `float64`, or a `float64 where >= 0.0`, hinted `min=0 max=1` SHALL step by `0.1` too

### Requirement: An element in the composition reads as one row
The viewer SHALL read each element in the composition as one row whatever the pane's width: an
element outside the value of another element's non-content property, content children included
(inside `if`, `for` and braced lists too). The row SHALL show, on one line, the child fold chevron
when the element holds content, its icon, its kind and the one value it is named by, and SHALL be
exactly one source line tall, its icon centered on the line and starting at the element's column in
the source, its chevron hanging to the left. Rows, and the lines of an `if` or a `for`
among them, SHALL follow one another a source line apart with no extra room between them, as a design
tool's layers do; an open inspector SHALL bring its own margins. As the line fills, that value SHALL be cut and faded. An element inside a property's
value SHALL read inside its owner's inspector, not as a row. Comments, `if` and `for` SHALL read as
written between rows, and an element whose syntax a row cannot show losslessly SHALL keep its
as-written reading. Declarations other than `let` values (types, functions' and components'
signatures, visualizers) SHALL keep their existing reading. The viewer SHALL NOT evaluate anything
to draw a row.

#### Scenario: A root element starts under the lets before it
- **WHEN** a module's root `<Drawing>` follows `let blue: Paint = …`
- **THEN** Drawing's icon SHALL start where `let` does, its chevron to the left of it

#### Scenario: Rows keep a steady rhythm
- **WHEN** a Row holds two Text rows and then `if customer.returning { <Badge /> }`
- **THEN** the Row, the two Texts, the `if` line, the Badge and the closing brace SHALL each start one
  source line below the one before

#### Scenario: A narrow pane keeps the structure
- **WHEN** the pane holding `<Text x=250 y=165 fontSize=18>NX graphics</Text>` is narrowed until
  its text does not fit
- **THEN** Text SHALL still read as one row, its text cut and faded, and nothing SHALL wrap

#### Scenario: A long title stays on its line
- **WHEN** a `SelectQuestion` row's title is a question longer than the room left on its line
- **THEN** the row SHALL read its kind and the question's first words, faded where cut, on one line

#### Scenario: A paint is not a row
- **WHEN** a `Circle` has `fill=<Paint.radial …/>`
- **THEN** Circle SHALL have a row and the paint SHALL read in Circle's inspector, not as a row

### Requirement: An `if` or a `for` among rows frames rows level with their siblings
Among an element's rows, an `if` or a `for` SHALL read as written, with every one of its lines, its
first, its `else`, its closing brace and those of the blocks inside it, starting where the icons of
its sibling rows do. The rows it holds SHALL read level with those siblings, as children of the same
element, framed by the block's lines. Its braces SHALL read in the quiet color, and its condition and
keywords in their code colors.

#### Scenario: A condition among rows
- **WHEN** a Section holds a Toggle and then `if requireTwoFactor { <Banner …/> }`
- **THEN** `if`, the Banner's icon, the closing brace and the Toggle's icon SHALL start at the same
  place, and the braces SHALL be gray

#### Scenario: A condition inside a loop
- **WHEN** a Stack holds `for member in members { <MemberRow …/> if !member.twoFactor { <Text …/> } }`
- **THEN** `for`, MemberRow, the inner `if`, the Text and both closing braces SHALL start at the same
  place

### Requirement: A row names its element by one value
A row SHALL show, after its kind, at most one value and no label, as a design tool's layer list
shows a layer's name: the element's title when its visualizer names one and the element writes it on
one line; else the text content it holds, with no element or block in it, however many lines it
runs; else its id when its visualizer names one. An element with none of these SHALL show its icon
and kind alone. The value SHALL read on one line, as its inspector field would, faded where it is
cut, and SHALL stay while the inspector is open. No other property SHALL read on the row. A row SHALL
have no fold for its text content; open, the inspector SHALL show that text as its first field.
Content that holds elements SHALL read as the row's children. A folded element value in an
inspector SHALL be named by the same rule, or else, when it sets just one thing on one line, by
that value, as a paint by its color.

A click on that value SHALL select its property, open the element's inspector if it is closed, and
open the property's field for editing, as a click on the field's value does; a value that cannot be
edited in place SHALL select its property.

#### Scenario: A title names the row
- **WHEN** a `Button` whose visualizer has `title=label` writes `<Button label="Save changes" kind=primary />`
- **THEN** its row SHALL read its icon, `Button` and `Save changes`, and not `primary`

#### Scenario: Text content names the row
- **WHEN** a document writes `<Text x=250 y=165 fontSize=18>` and `NX graphics` on its own line before `</Text>`
- **THEN** Text's row SHALL read `NX graphics` after its name, with no fold, no closing tag and no font
  size, and its inspector SHALL show `NX graphics` as its first field

#### Scenario: An id names the row only without a title
- **WHEN** `Workspace`'s visualizer has `title=name id=slug` and a document writes
  `<Workspace name="Hive5 Tools" slug="hive5-tools" … />`
- **THEN** its row SHALL read `Hive5 Tools` and not `hive5-tools`, which SHALL read in its inspector

#### Scenario: Nothing names the row
- **WHEN** a `Star` with no title, text or id writes `pointCount=5 fill={blue}`
- **THEN** its row SHALL read its icon and `Star` alone

#### Scenario: Editing the row's value
- **WHEN** a closed row reads `Hive5 Tools` and the reader clicks it
- **THEN** Workspace SHALL be selected with its inspector open and its Name field open for editing

### Requirement: Selecting an element opens its inspector under its row
Selecting a row, or any node inside its element's property values, SHALL open that element's
inspector directly under its row; selecting another element SHALL close it unless it is pinned.
Clicking the selected row again, or pressing Enter on it, SHALL close and reopen its inspector
without changing the selection. When the element's declaration has a doc, the inspector SHALL open
with its first paragraph as one quiet line, as plain words, cut where it runs out. Its labels SHALL start under its row's kind,
however deep the element is and whether or not it sits inside an `if` or a `for`. Opening or closing any inspector SHALL keep the row the
reader acted on where it was on screen, where the scroll range allows; a selection made from
outside the viewer SHALL scroll only when its row is off screen, just far enough to show it.

#### Scenario: Selecting a sibling below
- **WHEN** Star's inspector is open and the reader clicks Circle's row lower down
- **THEN** Star's inspector SHALL close, Circle's SHALL open, and Circle's row SHALL stay where it was
  on screen

#### Scenario: Closing the selected element's inspector
- **WHEN** Star is selected with its inspector open and the reader clicks Star's row
- **THEN** Star SHALL stay selected and its inspector SHALL close; a second click SHALL reopen it

#### Scenario: A deep element
- **WHEN** an element five levels deep is selected
- **THEN** its inspector's labels SHALL start under the element's kind and the inspector SHALL use
  the rest of the pane's width

#### Scenario: An element inside a condition
- **WHEN** a Text inside `if order.note? { … }` and a Button after the block are each selected
- **THEN** each inspector's labels SHALL start under its own row's kind

### Requirement: A pin keeps an inspector open
Each row SHALL offer a pin, shown while the row is hovered or selected and always while pinned.
Pinning SHALL keep the element's inspector open when the selection moves elsewhere; unpinning SHALL
close it unless the element is selected. Pins SHALL not change the selection or folds. Several
elements MAY be pinned at once. Read-only documents SHALL allow pinning.

#### Scenario: Comparing two elements
- **WHEN** the reader selects Star, pins it, and selects Circle
- **THEN** both inspectors SHALL be open, Star's pin shown as on

### Requirement: Folding children is separate from the inspector
A row whose element holds content SHALL fold and unfold its children with its chevron, Left and
Right, and Alt+click for the subtree, as groups fold today; a row without content SHALL show no
chevron. Folding SHALL NOT open, close or unpin an inspector, and opening an inspector SHALL NOT
fold or unfold children. Folding over the selection SHALL select the folded element without
unfolding it, keeping a selection in that element's own properties. `collapseAll` and `expandAll`
SHALL change folds only.

#### Scenario: Inspecting a folded container
- **WHEN** the reader folds Drawing's children and selects Drawing
- **THEN** Drawing's inspector SHALL open with its children still hidden

### Requirement: The inspector shows properties as labeled fields
An inspector SHALL show, as a field, each property its element writes and its title property
whether written or not, one to a line, in the resolved property order. When its element's type
declares other properties, its last line SHALL read `Show`, the number of them and `unset`, as `Show 3 unset`;
pressing it SHALL show them as fields in their places and turn the line into `Hide unset`, which
hides them again, and the inspector SHALL forget the choice when it closes. Tab from the
inspector's last field SHALL reach that line. A field SHALL show the property's resolved label on the
left and its value on the right; a field SHALL show no icon; the labels SHALL share a
column as wide as the longest, up to about 40% of the inspector, and a longer label SHALL wrap. A
value over several lines, as a list of elements, or an unfolded element value SHALL read under its
label, a step in; an unfolded element value's own fields SHALL start under its kind, as a row's
inspector starts under the row's kind, and its menu SHALL stay on its label's line. A boolean SHALL read as a checkbox beside its label. A property the element does
not write SHALL show as an empty value with a dashed outline and its default, if any, as a quiet
placeholder, else, for text, "Add" and its label, as `Add description`; setting it SHALL write it,
and a menu on a written property SHALL offer Reset to default or Remove as the row menu does today.
The inspector SHALL have no group headings. No label or field SHALL carry a browser tooltip; a
label's hover card SHALL give the property's name as written in source. The `title` and `id`
properties SHALL read in the inspector as fields too.

#### Scenario: Star's inspector
- **WHEN** Star's visualizer lists `cx` and `cy` with the labels `Center X` and `Center Y`, then
  `outerRadius`, `innerRadius` and `pointCount`, and Star writes all five but no `stroke`
- **THEN** the inspector SHALL show `Center X` beside `150` and `Center Y` beside `55` first, then the
  three labels with no icons, then a line counting the properties Star does not write
- **AND WHEN** the reader presses that line
- **THEN** `Stroke` SHALL read in its place beside an empty dashed value

#### Scenario: Unset properties hide again
- **WHEN** the reader shows a Card's unset properties, selects another row and selects the Card again
- **THEN** its inspector SHALL show only what the Card writes and its title, and the `Show N unset` line

#### Scenario: A list of elements under its label
- **WHEN** a Page writes `actions={ <Button label="Export" /> <Button label="Invite" /> }` and the
  reader opens Page's inspector
- **THEN** `Actions` SHALL read on its own line, with the two Buttons under it, a step in

### Requirement: A property without a label is named from its name
Where a property has no resolved label, the viewer SHALL derive one from its name: split it into
words at a lowercase or digit followed by an uppercase letter, at an uppercase letter followed by an
uppercase and a lowercase letter, and at `_`, `-` and spaces; write a word that is a known acronym
(`id`, `url`, `uri`, `ui`, `api`, `html`, `css`, `json`) in upper case; write the first word
capitalized and the rest in lower case; and drop a leading `is` or `has` word from a boolean
property's name when other words follow.

#### Scenario: Derived labels
- **WHEN** properties are named `outerRadius`, `imageURL`, `max_length`, `isRequired` and `x1`
- **THEN** their labels SHALL be `Outer radius`, `Image URL`, `Max length`, `Required` and `X1`

### Requirement: Numbers change by dragging their label
Pressing a number field's label and dragging left or right SHALL change the
number by one `step` (or one) for every three pixels, ten steps with Shift and a tenth with Alt,
within `min` and `max`, showing it in the field and going live as it changes, and SHALL commit once
on release as one undo step. The pointer SHALL read as a horizontal resize over it. A press that
moves less than four pixels SHALL focus the field instead. In a focused number field, Up and Down
SHALL step it, Shift by ten steps. The viewer SHALL NOT show stepper buttons. On touch, a drag on the
label SHALL scrub and a tap SHALL focus the field. The label's hover card SHALL end with a quiet
`Drag sideways to change; Shift for 10 steps, Alt for a tenth`.

#### Scenario: Scrubbing a radius
- **WHEN** the reader drags Star's `Outer radius` label 30 pixels to the right
- **THEN** the field SHALL read `46` during the drag, the preview SHALL follow, and one `nx-edit`
  SHALL commit `outerRadius=46` on release

### Requirement: References and nested values read by what they name
A value source writes in braces SHALL keep them, in the quiet color of an `if`'s braces and with the
spaces inside them dropped, so that it reads as computed; a literal SHALL read without them. A
property whose value is a reference to a top-level `let` SHALL read as a token with that name in its
braces; when
the `let` is written as a literal color, or as an element whose visualizer formats one property as a
color, the token SHALL show that color's swatch before its braces. Opening the token SHALL offer Go to `blue` and
Show in Source. A property whose value is an element SHALL read as a field with the element's icon,
kind and the value it is named by, and SHALL unfold into a nested group of that element's fields, edited the same
way. A short element value SHALL instead read its set properties on its field's line, each its label
and its value, edited and scrubbed there as an inspector's fields are, wrapping under the first where
the line runs out: one that holds only properties, one to four of them, each one simple value on one
line (a number, a boolean, a color, a bare case, a reference, or a string of at most 24 characters).
Its visualizer's `inline` SHALL decide instead when it is set: `true` reads any such value of one-line
properties on its line, `false` never does. A sole property, or the title, SHALL read without its
label. On its line its kind SHALL read only what the property's type leaves open: nothing for the
record the type names, and the case alone for a case of the union it names. Its chevron SHALL still
unfold it into the nested group, its kind in full, where the properties it does not set are. An action value, a component's `State.Update` included, SHALL show the action icon. Any other expression SHALL read as written, code-styled, and open in Source.

#### Scenario: A shared paint
- **WHEN** Star writes `fill={blue}` and `let blue: Paint = <Paint.color color=#2563eb/>`
- **THEN** Star's `Fill` field SHALL read a `#2563eb` swatch and the token `{blue}`, and its `⋯`
  menu's Go to `blue` SHALL select `blue`'s value

#### Scenario: An expression keeps its braces
- **WHEN** a Box writes `w={1 + 2}` and `label="plain"`
- **THEN** its `W` field SHALL read `{1 + 2}`, its braces quiet and its tokens in their code colors,
  and its `Label` field SHALL read `plain`

#### Scenario: A short element value on its field's line
- **WHEN** Drawing writes `viewBox=<Rect x=0 y=0 width=400 height=240/>` and
  `background=<Paint.color color=#ffffff/>`
- **THEN** View box SHALL read `X 0  Y 0  Width 400  Height 240`, with no `Rect`, and Background
  SHALL read `color` and the color, and dragging the `Width` label SHALL scrub `width` in that Rect

#### Scenario: A nested element value
- **WHEN** Drawing writes `viewBox=<Rect x=0 y=0 width=400 height=240/>` and the reader unfolds it
- **THEN** a nested group SHALL show the Rect's fields, and changing its width SHALL edit `width` in
  that Rect

### Requirement: A host can restore pins and closed inspectors
The viewer SHALL expose `detailsState` with `pinned`, the pinned elements' keys, and `suppressed`,
the key of a selected element whose inspector the reader closed. Assigning undefined SHALL clear it.
A reader's change SHALL fire `nx-details` with the complete state and the selection; assigning state
SHALL NOT fire it, and assigning equal state SHALL do nothing. While the tree is briefly absent the
state SHALL be kept. A new tree SHALL drop keys it no longer has, keep unchanged ones, and follow
items the viewer moved; where source edits make an item's identity uncertain, the pins inside that
list SHALL be dropped rather than moved to another item, and a dropped pin SHALL NOT come back on
undo. A change the viewer makes this way SHALL fire `nx-details`.

#### Scenario: Restoring without an echo
- **WHEN** the host assigns the state an earlier `nx-details` reported
- **THEN** the same inspectors SHALL be open and no `nx-details` SHALL fire

#### Scenario: A pinned item moves
- **WHEN** the reader moves a pinned item down with Alt+Down
- **THEN** the pin SHALL follow the item and SHALL NOT pass to the item now in its old place

### Requirement: The inspector keeps editing whole
Fields SHALL edit through the existing in-place editing, `nx-edit` and undo, including choices,
colors, multiline strings and invalid-entry notes. Closing an inspector, or a selection that would
close it, while one of its fields holds an invalid entry SHALL keep the inspector and the field open
and show the field's note. `finishEditing()` SHALL settle an open field, returning false and keeping
it open when its entry is invalid. Opening, closing and pinning SHALL NOT enter undo history or
re-run the document, and a new tree SHALL keep an open field, its caret and the selection.

#### Scenario: Closing over an invalid entry
- **WHEN** the reader types `abc` into Star's `X` and clicks Circle's row
- **THEN** Star's inspector SHALL stay open with the field and its note, and Circle SHALL NOT be
  selected

### Requirement: Rows and inspectors are keyboard accessible
Rows SHALL be tree items with their selection and expansion exposed; Up and Down SHALL move between
rows, Left and Right fold as today, and Enter SHALL close and reopen the selected row's inspector.
Tab SHALL move from a row into its open inspector's fields and pin. The pin SHALL be a button with
an element-specific name and a pressed state, reachable without hover and with a visible focus ring.
Arrow keys in a field SHALL NOT move between rows. Closing an inspector that holds focus SHALL move
focus to its row. Opening, closing and pinning SHALL be announced briefly.

#### Scenario: Keyboard comparison
- **WHEN** a keyboard user selects Star, tabs to its pin, presses Space and moves down to Circle
- **THEN** both inspectors SHALL be open and the pin SHALL report pressed

