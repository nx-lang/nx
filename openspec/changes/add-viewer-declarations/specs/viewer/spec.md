## MODIFIED Requirements

### Requirement: One element shows a source tree in any host
The repository SHALL publish `@nx-lang/viewer`, which defines an `<nx-viewer>` custom element that
shows a `SourceTree`, as `@nx-lang/language-protocol` types it, together with the text of the
document the tree describes, as a reading of that text. The element SHALL depend on no UI framework,
SHALL keep its markup and styles in a shadow root, and SHALL follow the page's light or dark scheme.
Its properties SHALL be `tree`, `text`, `colors`, `icons`, `selection` and `stale`, and a property set
before the element is defined SHALL take effect once it is. Where there is no DOM, importing the
package SHALL do nothing, and the package SHALL export a function that defines the element in a
registry the host chooses.

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

## REMOVED Requirements

### Requirement: The reading is lossless
**Reason**: The reading no longer offers each node's source text in a panel, or folds imports into a
details strip; it is the document's text itself, read as groups and lines.
**Migration**: See "The reading keeps every node".

### Requirement: Elements read as cards
**Reason**: Elements read as groups and lines in the document's own layout, not as cards.
**Migration**: See "An element over several lines reads as a group" and "An element on one line reads as
a line".

### Requirement: Values have a style and lose their quotes
**Reason**: Replaced by the reading's own treatment of values in groups and lines.
**Migration**: See "Values lose their quotes and keep a style".

### Requirement: Omitted defaults show as ghosts
**Reason**: The reading shows what the source writes; an omitted default is not shown.
**Migration**: None.

### Requirement: Logic reads as sentences
**Reason**: Code that is not an element reads as written, in the editor's colors.
**Migration**: See "Code reads as written, in the editor's colors".

### Requirement: Handlers read as events
**Reason**: A handler is an attribute and reads as one, by name and value.
**Migration**: See "An element on one line reads as a line" and "An element over several lines reads as
a group".

### Requirement: Declarations, references and comments read in place
**Reason**: Declarations and comments read as written; there are no declaration cards or reference
links.
**Migration**: See "Code reads as written, in the editor's colors".

### Requirement: Hover and peek explain what a node refers to
**Reason**: The reading has no hover panel or reference expansion.
**Migration**: None.

### Requirement: A selection is shared by key
**Reason**: A click now selects a unit of the reading rather than the smallest node under the pointer.
**Migration**: See "A selection is a unit of the reading".

## ADDED Requirements

### Requirement: The reading keeps every node
The element SHALL render every node of the tree once, and the rendering of each node SHALL carry the
node's key. Offsets into the text SHALL be taken from each range's UTF-8 byte offsets, so text after a
character outside the basic multilingual plane is cut where it begins.

#### Scenario: Coverage over the repository
- **WHEN** the viewer's coverage test renders the source tree of every `.nx` file in the repository
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
after those attributes when its viewer puts it at the end; a row for
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
  own, and the viewer of `Step` names `id` as its id
- **THEN** the reading SHALL show a group headed `Step welcome` with one row, `title` and `Welcome`,
  and no `<`, `/>` or quotes

#### Scenario: Content under a header
- **WHEN** a document holds `<Row gap="wide">` on one line, two `<Item />` elements on the lines after
  it with a blank line between them, and `</Row>`
- **THEN** the reading SHALL show the header `Row  gap=wide`, the two items under it with a little
  room between them, and no closing tag

### Requirement: An element on one line reads as a line
An element the document writes on one line SHALL read as its icon and name, its id where its viewer
puts it, the value of the attribute its viewer names as its title alone, each other attribute as
`name=value`, and then what it holds as written, with no tags of its own. When the element shares
its line with other text, such as a property's name in a group or a `let` before it, the line SHALL
keep the `<` before its name and a `/>` after it, so its extent stays clear; alone on its line, it
SHALL show neither. An element alone on its line whose type is exactly the type its place declares,
through any list, SHALL leave out its name when it has an icon, which then names it.

#### Scenario: A list of choices
- **WHEN** a property typed `Choice+` holds `<Choice value="yes-one" label="Yes, one" />` on a line
  of its own, and the viewer of `Choice` names `label` as its title, `value` as its id at the end and
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
a color of its own, with any quotes it keeps quiet. A string an element's viewer names as its `id`, or
that a property's viewer formats as `id`, SHALL read as one quiet token, the element's id moved to where
`idPosition` puts it. A string a viewer formats as `color` SHALL read with a small square of that color
before it when it names a color.

#### Scenario: Text shown to people
- **WHEN** a type declares `label:uitext` and an element sets `label="First choice"` on one line
- **THEN** the reading SHALL show `First choice` as text shown to people, without quotes

#### Scenario: A color
- **WHEN** a property typed by an alias with `viewer Color = <ValueViewer format=color />` holds
  `"#b6695d"`
- **THEN** the reading SHALL show a square of that color before `#b6695d`

### Requirement: Elements start with icons the host draws
The element SHALL take, through its `icons` property, a function that draws an icon by its Lucide name.
With it, every element that starts a line of its own SHALL start with an icon: the one its viewer's
`icon` names, or else one for its kind of declaration, `component` for a component, `box` for a record,
`zap` for an action, `tag` for a union case and `square-function` for a function. When the function
draws nothing for the viewer's icon, the element SHALL start with its kind's icon. The icon SHALL carry
the element's name as its label and add no text. Without the function, no element SHALL have an icon.

#### Scenario: A viewer's icon and a kind's
- **WHEN** the host draws icons, the viewer of `Choice` sets `icon="circle-dot"`, and a list holds a
  `Choice` and a record `Note` with no viewer, each on a line of its own
- **THEN** the `Choice` SHALL start with the icon `circle-dot` and the `Note` with `box`

#### Scenario: An icon the host cannot draw, and a union case
- **WHEN** the host draws nothing for `no-such-icon`, the viewer of a record `Tip` sets
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
