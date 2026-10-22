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

### Requirement: Editors offer the named values the host gives
The element SHALL accept from the host a `namedValues` function, given an element's key and a
property's name and answering a promise of the named values the property can take, each with its
reference path, type, doc and literal value as the `source-edits` capability defines them. When an
editor opens on a property's value, written or dimmed, and the host has given the function, the
element SHALL ask it and, when the answer holds items, SHALL list them above the editor's own entry,
in a popover for a field, each reading its path, and its color as a swatch when its value is a color
literal, with its doc in a hover card; a case popover or a color popover SHALL list them above its
own choices. The list SHALL filter as the reader types. Choosing one SHALL commit the value as the
`reference` with its path. An editor SHALL open at once and show the list when the answer arrives;
with no function, or an empty answer, editors SHALL read as they do without this requirement.

#### Scenario: Picking a token
- **WHEN** the host's `namedValues` answers `accent` and `theme.muted` for `color`, and the reader opens
  the color popover of `color="#5a3a22"` and chooses `accent`
- **THEN** the element SHALL fire `nx-edit` setting the reference `accent`

### Requirement: A reference to a named value edits with its type's editor
A value written as a reference to a named value SHALL be editable when the host has set `editable`
and its property's type has an editor, the reference being a name or a path of names, in braces or
bare: it SHALL open that
editor with the reference marked as the current item of the named values list, and an entry the
reader types SHALL replace the reference with a literal. A named value is a `let` without
parameters: the reference's name, or a path's first name, SHALL refer to one, as the tree's
declaration for it says. A reference whose type has no editor, and a reference to anything else,
such as a component's parameter or a loop variable, SHALL fire `nx-open` as an expression does.

#### Scenario: From a token back to a literal
- **WHEN** the reader opens the editor of `color={accent}` and types `#ffffff` in the hex field
- **THEN** the element SHALL fire `nx-edit` setting the string `#ffffff`

#### Scenario: A parameter is not a named value
- **WHEN** the reader double-clicks `{name}` in `let <Special name:string /> = <Drink name={name} />`
- **THEN** no editor SHALL open, and the element SHALL fire `nx-open` with its key

### Requirement: Editor hints choose and shape a value's editor
The element SHALL edit a value as its property's resolved editor hints, from the tree's visualizer
values, say:

- a number with `editor=slider`, or with both `min` and `max` and no `editor`, SHALL open a popover
  under the value holding a slider and the number field beside it; the slider SHALL draw `stops` as
  ticks and snap to them unless Alt is held, show the value in place while dragged, and commit once on
  release;
- a number with `editor=field` SHALL edit as a number field even with `min` and `max`;
- a number's arrows, drag and slider SHALL stay within `min` and `max` and step by `step`, Shift by ten
  steps, and a typed number outside them SHALL stay in the field with a note saying the bound, such as
  `At most 64`, and SHALL NOT be committed;
- a number with `unit` SHALL read the unit after it, quieter, and show it after the field; the unit
  SHALL NOT be written, and an entry ending in it, such as `16px`, SHALL be taken as the number;
- a case with `editor=segmented` SHALL show the union's cases as one row of buttons in its popover,
  unless there are more than five or they do not fit the reading's width, when it SHALL list them;
  `editor=dropdown` SHALL list them;
- a string with `multiline` SHALL edit in a field that holds line breaks, where Enter makes a line
  break and Ctrl or Cmd+Enter commits;
- a field whose property has a `placeholder` SHALL show it, quieter, while the field is empty, and
  SHALL NOT write it.

In any string field, Shift+Enter SHALL make a line break. A string literal written over several lines
SHALL edit in a field that holds line breaks; one written with an escape other than `\"` and `\\`
SHALL still fire `nx-open`.

#### Scenario: A spacing slider
- **WHEN** the reader opens `padding=8`, hinted `editor=slider min=0 max=64 stops={ 0 4 8 16 24 }`, and
  drags the slider's thumb from 8 to near 15
- **THEN** the value SHALL read `16` while dragged and, on release, the element SHALL fire one
  `nx-edit` setting the number `16`

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
