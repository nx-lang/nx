## ADDED Requirements

### Requirement: An element in the composition reads as one row
The viewer SHALL read each element in the composition as one row whatever the pane's width: an
element outside the value of another element's non-content property, content children included
(inside `if`, `for` and braced lists too). The row SHALL show, on one line, the child fold chevron
when the element holds content, its icon, its kind and the one value it is named by, and SHALL be
exactly one source line tall, its icon centered on the line. Rows, and the lines of an `if` or a `for`
among them, SHALL follow one another a source line apart with no extra room between them, as a design
tool's layers do; an open inspector SHALL bring its own margins. As the line fills, that value SHALL be cut and faded. An element inside a property's
value SHALL read inside its owner's inspector, not as a row. Comments, `if` and `for` SHALL read as
written between rows, and an element whose syntax a row cannot show losslessly SHALL keep its
as-written reading. Declarations other than `let` values (types, functions' and components'
signatures, visualizers) SHALL keep their existing reading. The viewer SHALL NOT evaluate anything
to draw a row.

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
without changing the selection. The inspector SHALL lay its fields out in a grid of two columns
when the pane is wide enough and one when not, and its labels SHALL start under its row's kind,
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
An inspector SHALL show every property its element's type declares, written or not, as a field, one
to a line, in the resolved property order. A field SHALL show the property's resolved label on the
left and its value on the right; a field SHALL show no icon; the labels SHALL share a
column as wide as the longest, up to about 40% of the inspector, and a longer label SHALL wrap. A
value over several lines, as a list of elements, or an unfolded element value SHALL read under its
label, a step in. A boolean SHALL read as a checkbox beside its label. A property the element does
not write SHALL show as an empty value with a dashed outline and its default, if any, as a quiet
placeholder, else, for text, "Add" and its label, as `Add description`; setting it SHALL write it,
and a menu on a written property SHALL offer Reset to default or Remove as the row menu does today.
The inspector SHALL have no group headings. Every field and label SHALL carry a tooltip with the
property's label and its name as written in source. The `title` and `id` properties SHALL read in
the inspector as fields too.

#### Scenario: Star's inspector
- **WHEN** Star's visualizer lists `cx` and `cy` with the labels `Center X` and `Center Y`, then
  `outerRadius`, `innerRadius` and `pointCount`, and Star writes no `stroke`
- **THEN** the inspector SHALL show `Center X` beside `150` and `Center Y` beside `55` first, then the
  three labels with no icons, and `Stroke` beside an empty dashed value

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
Pressing a number field's label, with its icon, and dragging left or right SHALL change the
number by one `step` (or one) for every three pixels, ten steps with Shift and a tenth with Alt,
within `min` and `max`, showing it in the field and going live as it changes, and SHALL commit once
on release as one undo step. The pointer SHALL read as a horizontal resize over it. A press that
moves less than four pixels SHALL focus the field instead. In a focused number field, Up and Down
SHALL step it, Shift by ten steps. The viewer SHALL NOT show stepper buttons. On touch, a drag on the
label SHALL scrub and a tap SHALL focus the field.

#### Scenario: Scrubbing a radius
- **WHEN** the reader drags Star's `Outer radius` label 30 pixels to the right
- **THEN** the field SHALL read `46` during the drag, the preview SHALL follow, and one `nx-edit`
  SHALL commit `outerRadius=46` on release

### Requirement: References and nested values read by what they name
A property whose value is a reference to a top-level `let` SHALL read as a token with that name; when
the `let` is written as a literal color, or as an element whose visualizer formats one property as a
color, the token SHALL show that color's swatch. Opening the token SHALL offer Go to `blue` and
Show in Source. A property whose value is an element SHALL read as a field with the element's icon,
kind and the value it is named by, and SHALL unfold into a nested group of that element's fields, edited the same
way. An action value, a component's `State.Update` included, SHALL show the action icon. Any other expression SHALL read as written, code-styled, and open in Source.

#### Scenario: A shared paint
- **WHEN** Star writes `fill={blue}` and `let blue: Paint = <Paint.color color=#2563eb/>`
- **THEN** Star's `Fill` field SHALL read a `#2563eb` swatch and the token `blue`

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

## MODIFIED Requirements

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

#### Scenario: Clicking a caption selects
- **WHEN** the reader clicks the caption of `size=16` without dragging
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

#### Scenario: Dragging in the color picker
- **WHEN** the reader opens a color's popover and the picker reports `#112233`, then `#445566`, then
  the reader presses Escape
- **THEN** the element SHALL fire live edits setting `#112233` and `#445566`, then `nx-edit` without
  `live` setting the color the popover opened on

#### Scenario: Typing
- **WHEN** the reader types `Mocha` into a string field and pauses
- **THEN** the element SHALL fire one live edit setting `Mocha`, and none for the letters before it

#### Scenario: Scrubbing
- **WHEN** the reader drags the caption of `size=16` nine pixels to the right and releases
- **THEN** the element SHALL fire live edits setting `17`, `18` and `19`, then one `nx-edit` without
  `live` setting `19`

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

## REMOVED Requirements

### Requirement: An element reads as a group where its properties do not fit on a line
**Reason**: Elements in the composition now read as one row with an inspector.
**Migration**: None; declarations and as-written fallbacks keep their reading.

### Requirement: An element reads as a line where its properties fit
**Reason**: Elements in the composition now read as one row with an inspector.
**Migration**: None.

### Requirement: A selected element offers the properties it does not write
**Reason**: The inspector shows unwritten properties as empty fields.
**Migration**: None.
