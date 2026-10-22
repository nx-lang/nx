## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: A selected element offers the properties it does not write
When the host has set `editable` and an element is selected, the element SHALL end it with a quiet
row reading `+ N more`, N being how many of the element's declared properties it does not write,
content properties left out; for an element of a union's case, its declared properties are the
union's shared ones and the case's, and a property written in any branch of a property-list
condition or match counts as written; an element that writes all of them SHALL show no such row, and only the
selected element SHALL show one. Opening the row, by a click or Enter, SHALL draw each unwritten
property under it as a dimmed row, in the order its resolved visualizer lists properties, else in
declaration order: its name, and its default read as the reading reads values, followed by a quiet
`default`, or nothing for an optional property. A dimmed row's name SHALL show its property's hover
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
- **THEN** the element SHALL end with `+ 4 more`, and opening it SHALL show the rows `note`, `iced`
  reading `✗ default`, `shots` reading `1 default` and `color`

#### Scenario: Setting one
- **WHEN** the reader clicks the cross of the dimmed row `iced`
- **THEN** the element SHALL fire `nx-edit` with the drink's key and `addProperty` for `iced` with the
  boolean `true`

### Requirement: A selected row has a menu
When the host has set `editable`, a written row SHALL show a quiet `⋯` at its end while the pointer is
over it and while it is selected. Opening it SHALL list Reset to default for a property with a
default, Remove for an optional one, and Show in Source for every row; Reset and Remove SHALL fire
`nx-edit` removing the attribute, as Delete does, and Show in Source SHALL fire `nx-open` with the
row's value. The menu SHALL open over the reading, move nothing, and close on Escape, a choice or a
click elsewhere. Dimmed rows SHALL have no menu.

#### Scenario: Resetting from the menu
- **WHEN** the reader opens the menu of the row `size=large` of a `Drink` whose `size` defaults to
  `medium` and chooses Reset to default
- **THEN** the element SHALL fire `nx-edit` removing that attribute

### Requirement: Editor hints choose and shape a value's editor
The element SHALL edit a value as its property's resolved editor hints, from the tree's visualizer
values, say:

- a number's field, arrows, steppers and drag SHALL stay within `min` and `max` and step by `step`,
  Shift by ten steps, and a typed number outside them SHALL stay in the field with a note saying the
  bound, such as `At most 64`, and SHALL NOT be committed; `editor=field` asks for the number field,
  which every number has;
- a number with `unit` SHALL read the unit after it, quieter, and show it after the field; the unit
  SHALL NOT be written, and an entry ending in it, such as `16px`, SHALL be taken as the number;
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
that the list is there; a click on it, or Alt+Down, SHALL open the list, and so SHALL typing, the list
then holding the choices whose label or value holds what was typed. While the list is closed, Up and
Down SHALL step a number; while it is open, they SHALL move through it, Enter SHALL choose the active
row, and Escape SHALL close the list before the field. Choosing a choice SHALL commit its value as
the field would: a value the field would refuse SHALL stay in the field, the list closed, with the
field's note. Choosing the default's row SHALL leave the attribute out, as the row's Reset to
default does, or, on a dimmed row, add nothing. The field SHALL stay typeable: the choices suggest
and never restrict, and any entry the field takes SHALL commit.

A number SHALL show steppers, an up and a down button drawn over the space just past the value so
that nothing moves, while its field is open and while the pointer is over its value on the selected
row, when the host has set `editable`. A click SHALL step the number once by `step`, or by one, and
Shift+click by ten steps, within `min` and `max`: into the field while it is open, else committed at
once. Pressing a stepper and dragging up or down SHALL change the number by a step for every few
pixels and show it in place: on release, into the field while it is open, else committed once. The steppers' tooltip SHALL name the arrow
keys, Shift and dragging. Dragging the property's name sideways SHALL still change the number.

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
- **WHEN** the row `padding=8` is selected and the reader clicks the up stepper past its value, and
  then Shift-clicks it
- **THEN** the element SHALL fire `nx-edit` setting the number `9`, and then the number `19`

#### Scenario: A stepper dragged
- **WHEN** the reader presses the up stepper of `gap=4` on its selected row, drags up, and lets go
  at `14`
- **THEN** the number SHALL read `14` in place while dragging, and the element SHALL fire `nx-edit`
  setting the number `14` once, on release

#### Scenario: A refused choice
- **WHEN** the reader opens a number hinted `max=10` with the choices `5` and `99`, and chooses `99`
- **THEN** no `nx-edit` SHALL fire, and the field SHALL stay open holding `99` with the note
  `At most 10`

#### Scenario: Out of range
- **WHEN** the reader types `80` in the field of a number hinted `max=64` and presses Enter
- **THEN** no `nx-edit` SHALL fire and a note beside the field SHALL read `At most 64`

#### Scenario: A unit
- **WHEN** a number `padding=8` is hinted `unit="px"`
- **THEN** it SHALL read `8 px`, and typing `12px` and Enter SHALL fire `nx-edit` setting the number `12`

#### Scenario: A line break in a string
- **WHEN** the reader edits `note="Ask for oat milk"`, with no hints, and presses Shift+Enter after
  `Ask` and then Enter
- **THEN** the element SHALL fire `nx-edit` setting a string holding a line break after `Ask`
