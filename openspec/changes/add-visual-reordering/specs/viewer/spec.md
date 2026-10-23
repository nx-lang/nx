## ADDED Requirements

### Requirement: An item of a list moves by its grip
When the host has set `editable`, the element SHALL let the reader move an item that moves: an
element that is an item of a sequence or of element content, not inside a condition's branch or a
loop's body, in a list of two or more items that holds no text. Where the reading draws such a list an item to a line, the element SHALL show a
grip in the gutter left of the item's first line, left of its fold chevron when it folds, while the
pointer is over the item and while it is selected, as quiet as the fold chevron. Pressing the grip
and moving the pointer 4 pixels or more SHALL start a drag; a click on the grip SHALL select the
item. While dragging, the item SHALL be drawn muted where it stands, a copy of one line holding its
icon and its title, or its name, SHALL follow the pointer, and a line in the selection colour SHALL
mark the gap between items of the item's own list nearest the pointer; nothing else in the reading
SHALL move. Releasing over the list at a gap that does not leave the item where it is SHALL fire `nx-edit` with the
item's key and `move` to the position that gap gives it. Releasing anywhere else, or Escape, SHALL
cancel the drag and fire nothing. While the pointer is near the top or bottom edge of the reading's
scrolling area, the reading SHALL scroll toward it. The drag SHALL work with a mouse, a pen and a
touch on the grip.

#### Scenario: Dragging a step down
- **WHEN** the reader drags the grip of the first of four steps and releases it with the line marking
  the gap between the third and the fourth
- **THEN** the element SHALL fire `nx-edit` with the first step's key and `move` to `2`, and nothing
  else in the reading SHALL have moved during the drag

#### Scenario: Cancelling
- **WHEN** the reader presses Escape during a drag
- **THEN** the drag SHALL end, the item SHALL read as before, and no event SHALL fire

#### Scenario: No grip
- **WHEN** the pointer is over the only choice of a question, or over an element inside an `if`
  block
- **THEN** no grip SHALL show

### Requirement: An item of a list moves up and down
With the reading focused, no editor open and an item that moves selected, Alt+Up and Alt+Down SHALL
fire `nx-edit` with the item's key and `move` to the position before or after its own, and SHALL do
nothing at the list's start or end. The selected item SHALL show a quiet `⋯` at the end of its first
line, as a written row does, whose menu lists Move up when an item stands before it, Move down when
one stands after it, and Show in Source, which fires `nx-open` with the item's key; the menu SHALL
open from the context-menu key and Shift+F10 too, and look and close as a row's menu does. These
SHALL work where the reading draws the list on one line as well.

#### Scenario: Alt+Down
- **WHEN** the reader selects the second of three choices and presses Alt+Down
- **THEN** the element SHALL fire `nx-edit` with that choice's key and `move` to `2`

#### Scenario: The first item's menu
- **WHEN** the reader opens the `⋯` menu of the first of three steps
- **THEN** it SHALL list Move down and Show in Source, and no Move up

### Requirement: A moved item keeps its selection and folds
After the element fires a `move`, it SHALL read the host's newer tree with the item at its new
position selected, and with each fold on an item of that list kept on the item it was on, firing
`nx-fold` with the folded keys as the newer tree gives them. A polite live region SHALL say where the
item went, as `Moved to 3 of 8`. When the host's newer tree holds the list unchanged, as after a
refusal, the selection and folds SHALL stay as they were.

#### Scenario: The selection follows
- **WHEN** the reader moves the selected first step to `2` and the host sets the newer tree
- **THEN** the step now third SHALL be selected, and the live region SHALL say `Moved to 3 of 4`

#### Scenario: A fold follows
- **WHEN** the second step is folded and the reader moves the first step to `2`
- **THEN** in the newer tree the step now first, which was second, SHALL be folded, and the element
  SHALL fire `nx-fold` with its key
