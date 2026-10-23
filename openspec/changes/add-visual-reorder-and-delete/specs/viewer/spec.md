## ADDED Requirements

### Requirement: An item of a list is dragged by its icon
When the host has set `editable`, the element SHALL let the reader move an item that moves: an
element that is an item of a sequence or of element content, not inside a condition's branch or a
loop's body, in a list of two or more items that holds no text. Where the reading draws such a list
an item to a line, the item's handle SHALL be its icon, with the element's name beside it where the
reading shows one, or its name alone where the host draws no icons; the element SHALL add no mark of
its own for it. Over the handle the pointer SHALL show a grab cursor, and the hover card the icon or
name shows SHALL end with a quiet `Drag to reorder`. A click on the handle SHALL select the item, as
today; pressing it and moving the pointer 4 pixels or more SHALL instead start a drag and close any
hover card. While dragging, the item SHALL be drawn muted where it stands, a copy of one line holding
its icon and its title, or its name, SHALL follow the pointer, and a line in the selection colour SHALL
mark the gap between items of the item's own list nearest the pointer; nothing else in the reading
SHALL move. Releasing over the list at a gap that does not leave the item where it is SHALL fire
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
- **WHEN** the pointer is over the icon of the only choice of a question, or of an element inside an
  `if` block
- **THEN** the cursor SHALL not show a grab hand, and pressing and moving SHALL start no drag

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
deleted: an element that is an item of a sequence or of element content, not inside a condition's
branch or a loop's body, in a list that holds no text, unless it is the only item of a list whose type
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
