## MODIFIED Requirements

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
  between and no text selected, and nothing SHALL be selected

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
Shift+click by ten steps, within `min` and `max`: into the field while it is open, else applied at once as a live edit and
committed once the reader pauses, so a run of clicks is one commit. Pressing a stepper and dragging
up or down SHALL change the number by a step for every few pixels and show it in place, each step
going live: on release, into the field while it is open, else committed once. The steppers' tooltip SHALL name the arrow
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

### Requirement: A selected row has a menu
When the host has set `editable`, a written row SHALL show a quiet `⋯` at its end while the pointer is
over it and while it is selected. Opening it SHALL list Reset to default for a property with a
default, followed, quieter, by the default as the reading draws it when it is a short literal, Remove
for an optional one, and Show in Source for every row; Reset and Remove SHALL fire
`nx-edit` removing the attribute, as Delete does, and Show in Source SHALL fire `nx-open` with the
row's value. The menu SHALL open over the reading, move nothing, and close on Escape, a choice or a
click elsewhere. Dimmed rows SHALL have no menu. A property written on a line SHALL have the same
menu: a click on its name or its `=` SHALL select the property, drawn just around it, and while it
is selected its `⋯` SHALL show past the end of its line, as a list's item sharing its line has, and
Delete SHALL remove it as it removes a row.

#### Scenario: Resetting from the menu
- **WHEN** the reader opens the menu of the row `size=large` of a `Drink` whose `size` defaults to
  `medium` and chooses Reset to default
- **THEN** the item SHALL have read `Reset to default` and `medium`, and the element SHALL fire
  `nx-edit` removing that attribute

#### Scenario: Removing a property written on a line
- **WHEN** a `Drink` reads on one line with the optional `color="#6f4e37"`, and the reader clicks
  `color` and opens the `⋯` shown at the end of the line
- **THEN** the menu SHALL list Remove and Show in Source, and Remove SHALL fire `nx-edit` removing
  that attribute

## ADDED Requirements

### Requirement: Edits go live while the reader changes a value
While the host has set `editable`, the element SHALL fire `nx-edit` with `live: true` for each value
the reader gives a value before committing it, so the host can show it at once: an open field's entry
once typing pauses for 300 ms, at once when a stepper, an arrow key or a drag changes a number, and at
once when the color picker or a color's hex field changes a color. A live edit SHALL fire only for an
entry the editor can take and that differs from the last value fired, and an entry the editor cannot
take SHALL fire nothing until it can. A live edit SHALL carry the same key and edit as the commit of
that value would, and the reader's next value of the same value SHALL replace it. The value's commit
SHALL end its live edits with `nx-edit` without `live`: on commit, with the value committed, even when
the host's tree already reads it; on cancel, as Escape or leaving a field holding an entry it cannot
take does, with the value the editor opened on, when a live edit has fired. A click on a stepper with
the field closed SHALL fire a live edit and commit 500 ms after the last such click on that value, or
at once when the reader edits, selects or opens anything else. Closing a color's popover by a press
elsewhere SHALL commit the entry it holds when the entry can be taken and differs from the value, as
leaving a field does, and Escape SHALL cancel it. An editor SHALL stay open, with its entry and caret,
across the host's newer tree.

#### Scenario: Steppers in an open field
- **WHEN** the reader opens the field of `size=16` and clicks its up stepper twice
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

#### Scenario: Clicks with the field closed
- **WHEN** the reader clicks the up stepper of `size=16` three times, a moment apart, and waits
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
