## ADDED Requirements

### Requirement: Compact composition rows have a stable identity
The viewer SHALL offer an opt-in `compact` boolean, false by default. In compact presentation,
representable elements outside non-content property-value subtrees SHALL show one compact identity
row independent of pane width, with icon when available, kind, id, and literal title or a property
count. Property-value elements SHALL retain their existing nested reading. Unrepresentable syntax
SHALL retain its as-written fallback. Comments, logic, content, and exact source navigation SHALL be
preserved. Header title/id nodes SHALL NOT be duplicated in details. Summaries SHALL NOT evaluate
source expressions or require the preview runtime.

#### Scenario: Resizing the outline
- **WHEN** a compact composition element's pane is resized
- **THEN** it SHALL retain its identity row and disclosure state, with details wrapping independently

#### Scenario: A compound paint value
- **WHEN** Circle has a radial-gradient property and is shown compactly
- **THEN** Circle SHALL have a composition row and the gradient SHALL remain inside Circle's properties

#### Scenario: Ordinary reader remains available
- **WHEN** a host does not enable compact presentation
- **THEN** the viewer SHALL use its existing width-dependent group and line reading

### Requirement: Compact children fold independently of details
Compact composition rows with content children SHALL provide child disclosure separate from Details.
Leaf rows SHALL NOT offer an empty child chevron. Child disclosure SHALL NOT change property choices
or pins; property disclosure SHALL NOT change child folds. Folding over a selected content descendant
SHALL select the folded ancestor without reopening its children. A selection in the ancestor's own
properties SHALL remain selected. Hidden descendant property choices SHALL be retained.

#### Scenario: Inspecting a folded container
- **WHEN** a reader opens Drawing's properties and then folds its children
- **THEN** Drawing's properties SHALL remain visible and its content children SHALL be hidden

#### Scenario: Folding over a selected descendant
- **WHEN** Circle is selected and the reader folds its Drawing ancestor
- **THEN** Drawing SHALL become selected, remain child-folded, and retain Circle's disclosure choices

#### Scenario: Whole-outline commands
- **WHEN** a reader invokes collapseAll or expandAll in compact presentation
- **THEN** child and declaration folds SHALL change without clearing explicit property choices or pins

### Requirement: Inline details follow selection and deliberate disclosure
Selecting a compact composition element or a node in its property-value subtree SHALL open that
composition owner's temporary details. Moving to another owner SHALL close only temporary details.
Explicitly opening Details SHALL keep them open across selection changes without selecting the owner.
Closing Details SHALL clear that owner's explicit-open entry and pin and suppress its temporary
details until selection leaves that owner. Moving selection within the same owner SHALL NOT clear
suppression. Closing details containing the selected property SHALL move selection to the owner.
A host explicitly selecting a hidden property SHALL reveal its necessary details and ancestors;
reassigning an unchanged selection SHALL NOT override intentional closure.

#### Scenario: Temporary inspection
- **WHEN** the reader selects Star and then Circle without explicitly opening or pinning either
- **THEN** only Circle's temporary details SHALL remain open

#### Scenario: Explicitly opened details
- **WHEN** the reader opens Star's Details and then selects Circle
- **THEN** Star's details SHALL remain open and Circle's temporary details SHALL open

#### Scenario: Closing selected properties
- **WHEN** Star's radius is selected and the reader closes Star's Details
- **THEN** selection SHALL move to Star and its details SHALL stay closed until the owner changes

#### Scenario: Source navigation reveals a property
- **WHEN** the host changes selection to a property inside an undisclosed element
- **THEN** its ancestors and owning details SHALL open and the exact property SHALL be selected

### Requirement: Properties can be pinned for comparison
Each compact composition row SHALL provide Pin independently of selection and child folding. Pinning
SHALL keep its details open across selection changes without adding an explicit-open entry. Unpinning
SHALL remove only the pin; details SHALL remain open if selection or explicit disclosure requires it.
The UI SHALL distinguish Pinned, Kept open, and Following selection states. Read-only source SHALL
allow selection, folding, disclosure, and pinning while still refusing source edits.

#### Scenario: Pinning for comparison
- **WHEN** the reader selects Star, pins its properties, and then selects Circle
- **THEN** both details SHALL be visible with Star identified as pinned and Circle following selection

#### Scenario: Unpinning while still selected
- **WHEN** the reader unpins the selected Star without an explicit-open entry
- **THEN** its details SHALL remain visible until selection moves to another owner

#### Scenario: Unpinning explicitly opened details
- **WHEN** the reader unpins an explicitly opened Star while Circle is selected
- **THEN** Star SHALL remain open as Kept open

### Requirement: A host can restore compact disclosure state
The viewer SHALL expose `detailsState` containing opened and pinned element keys and an optional
suppressed owner key. Undefined SHALL restore empty choices. A reader change SHALL fire `nx-details`
with the complete state and resulting selection; programmatic restoration SHALL NOT echo an event.
Automatic reconciliation that changes choices SHALL notify the host. Equivalent assignments SHALL
be no-ops. Temporary absence of a tree SHALL retain state. Complete tree updates SHALL prune missing
owners, preserve unchanged owners, and follow known list-item moves without transferring choices to
a different item. Ambiguous arbitrary source edits SHALL clear affected list-subtree choices rather
than attach them to an uncertain owner. Deleted choices SHALL NOT resurrect on undo.

#### Scenario: Host restoration without a feedback loop
- **WHEN** the host assigns the state previously reported by nx-details
- **THEN** the same details SHALL show and no new nx-details event SHALL fire

#### Scenario: A refresh temporarily clears the tree
- **WHEN** the host clears the tree and then restores the same document with a property edit
- **THEN** the existing valid pins and explicit-open choices SHALL survive

#### Scenario: A pinned item moves
- **WHEN** a reader reorders a pinned list item through the viewer
- **THEN** its pin SHALL follow that item and SHALL NOT transfer to the item taking its former position

#### Scenario: A source edit makes item identity ambiguous
- **WHEN** arbitrary source edits change list membership without a reliable mapping
- **THEN** affected list-subtree choices SHALL clear and unaffected choices SHALL remain

#### Scenario: A pinned item is deleted and undone
- **WHEN** a pinned item is deleted and the deletion is undone
- **THEN** its pin SHALL remain cleared and any undo-selected property SHALL reveal temporary details

### Requirement: Compact disclosure preserves editing and place
Open compact details SHALL support existing editors, unwritten properties, menus, source opening,
live updates, and undo behavior. A host SHALL be able to settle editing before hiding Visual through
`finishEditing()`, which SHALL return false for an invalid draft and true after successfully settling.
Actions that hide an active field SHALL use its normal validation
and commit behavior first; invalid drafts SHALL remain visible with their problem reported. Escape
SHALL retain normal cancellation behavior. Disclosure SHALL NOT modify source history or run the
snippet. Refreshes SHALL preserve valid active editor state. Local disclosure and selection-following
transitions SHALL keep an already visible acted-on header at its prior viewport position where scroll
bounds permit; revealing an offscreen target SHALL scroll only as needed to bring it into view.

#### Scenario: Editing a pinned property
- **WHEN** a reader changes a pinned Star's radius while Circle is selected
- **THEN** the edit SHALL use the existing source-edit path and both details SHALL remain open

#### Scenario: Closing over an invalid draft
- **WHEN** a reader enters an invalid numeric draft and tries to close its details
- **THEN** the details and field SHALL remain visible and validation SHALL explain the problem

#### Scenario: Selecting a lower sibling
- **WHEN** selecting an already visible Circle closes temporary details above it
- **THEN** Circle's header SHALL remain at its prior viewport position where scroll bounds permit

### Requirement: Compact controls are accessible independently
Child disclosure and Details SHALL expose element-specific names, expanded state, and controlled
regions. Pin SHALL expose its pressed state and element-specific name. Controls SHALL be reachable
with Tab, activatable with Enter/Space, and visible without hover with a visible focus indicator.
Outline left/right arrows SHALL operate child folds before navigating; they SHALL NOT toggle details
or intercept text-field cursor keys. Disclosure SHALL restore focus to its control or owner header
when focused content disappears. Selection and pin/disclosure changes SHALL be announced concisely.

#### Scenario: Keyboard comparison
- **WHEN** a keyboard user selects Star, tabs to Pin, activates it, and selects Circle
- **THEN** both property sheets SHALL remain open with Pin's pressed state exposed

#### Scenario: Closing the focused details
- **WHEN** an action closes details whose input has focus after successful validation
- **THEN** focus SHALL return to the owner's Details control or header rather than the document body

## MODIFIED Requirements

### Requirement: One element shows a source tree in any host
The repository SHALL publish `@nx-lang/viewer`, which defines an `<nx-viewer>` custom element that
shows a `SourceTree`, as `@nx-lang/language-protocol` types it, together with the text of the
document the tree describes, as a reading of that text. The element SHALL depend on no UI framework,
SHALL keep its markup and styles in a shadow root, and SHALL follow the page's light or dark scheme.
Its properties SHALL be `tree`, `text`, `colors`, `icons`, `markdown`, `folded`, `selection`,
`stale`, `editable`, `compact` and `detailsState`, and a property set before the element is defined SHALL take effect once it is.
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
With nothing folded and, in compact presentation, every property detail disclosed, the element SHALL
render every node of the tree once, and the rendering of each
node SHALL carry the node's key. What is folded SHALL NOT be rendered; selecting a node folded away
reaches it. Offsets into the text SHALL be taken from each range's UTF-8 byte offsets, so text after a
character outside the basic multilingual plane is cut where it begins.

#### Scenario: Coverage over the repository
- **WHEN** the viewer's coverage test renders the source tree of every `.nx` file in the repository,
  with nothing folded and, in compact presentation, every property detail disclosed
- **THEN** the key of every node SHALL appear in the rendering exactly once

#### Scenario: Text after an emoji
- **WHEN** a document holds `let wave = <Label text="😀 hello" title="after" />`
- **THEN** the reading of the element SHALL show `after` as the value of `title`

### Requirement: An element reads as a group where its properties do not fit on a line
Outside compact composition rows, this requirement and its scenarios apply unchanged. An element SHALL
read as a group when it holds an element, or when it shows more than one property and
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
Outside compact composition rows, this requirement and its scenarios apply unchanged. An element SHALL
read as a line, however the document breaks its lines, when it holds no element and
no more than a line of text and its properties fit after its name on one line in the pane, and so
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

### Requirement: The reading folds
For compact composition rows, element folding SHALL hide content children independently of properties,
and collapseAll/expandAll SHALL leave property disclosure and pins unchanged. The group-folding
behavior and scenarios below apply to the regular presentation and nested property readers; run and
declaration folding applies in both presentations.

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

### Requirement: A selected element offers the properties it does not write
In compact presentation this affordance SHALL appear within the selected owner's open details and
SHALL remain hidden while those details are closed. The behavior below otherwise applies unchanged.
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
