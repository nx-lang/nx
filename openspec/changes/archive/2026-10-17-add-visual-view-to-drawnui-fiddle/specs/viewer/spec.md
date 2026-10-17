## RENAMED Requirements

- FROM: `### Requirement: An element over several lines reads as a group`
- TO: `### Requirement: An element reads as a group where its properties do not fit on a line`
- FROM: `### Requirement: An element on one line reads as a line`
- TO: `### Requirement: An element reads as a line where its properties fit`

## MODIFIED Requirements

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

### Requirement: A selection is a unit of the reading
A click SHALL select the unit it falls in: an element read as a group, by its header or the room
between its rows; a row of a group; an element read as a line; an element written as its tags that
starts a line; or a block written as it is, such as an `if`. Outside every unit, a click SHALL select the
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

## ADDED Requirements

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
