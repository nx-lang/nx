## Context

Visual reads a document's source tree and edits it through `sourceEdit`: the viewer fires `nx-edit`
with a node's key and an edit, the host asks the language service for the text edits, applies them as
one undoable step, and sets the newer tree (`add-visual-value-editing`, `add-visual-property-editing`).
Items of a sequence, of element content and of a branch are keyed by their position in their list
(`steps[2]`), so a key names a place, not an item.

Phase 5 of the design doc was structure editing: add, delete, duplicate and move elements, with an
insert menu. Bret narrowed it to reordering (2026-10-23), with dragging where it works best and Move
up and Move down as the second way, and asked that the UI stay simple. He then added deleting
(2026-10-23), which shares the item rules and the line handling with moving.

## Goals / Non-Goals

**Goals:**
- Reorder the items of one list, by drag and by keyboard or menu, each move one edit and one undo
  step.
- Keep everything a row already does (click to edit, steppers, the name scrub, text selection,
  folding) working as it does.
- Keep the source's layout and comments.

**Non-Goals:**
- Adding, inserting, duplicating, cut, copy and paste.
- Moving between lists or levels, or in or out of an `if` or a `for`.
- Dragging several items, a touch long-press, drag between documents.
- Reordering literal items, such as the numbers of `{ 4 2 5 }`.

## Decisions

### 1. The icon is the handle (Bret, 2026-10-23)

A row is full of click targets: a value opens its editor, steppers step, a property's name scrubs,
and text can be selected. Dragging from anywhere on the row would compete with each of them. Every
element on a line of its own already starts with its icon, and, where the reading shows it, its name;
neither opens a field, a stepper or a scrub, and a click there selects the item. So they are the
handle: a grab cursor over them, `Drag to reorder` at the end of the hover card they already show,
and a press that moves 4 px starts the drag, while a click still selects. Nothing new is drawn, and
no selection is needed first. Without the host's `icons`, the name alone is the handle.

Alternatives considered: a grip (`⠿`) in the gutter on hover, as in Notion and Linear, the more
familiar cue but a new mark beside the fold chevron; and dragging a selected item from anywhere,
which loses text selection there and shows no cue. Bret chose the icon. The icon is a small target
(about 14 px), which the name beside it widens where it shows.

### 2. A line marks the landing, and nothing else moves

During a drag the item dims in place, a one-line copy (its icon and title, or name) follows the
pointer, and a 2 px line in the selection colour marks the gap nearest the pointer, only between
items of the item's own list. Items do not slide apart to show the gap: Visual's rule is that nothing
moves under the reader, and with tall open groups a sliding list jumps. The reading changes once, on
the host's newer tree. The line is not drawn at the two gaps beside the item, where a drop
would not move it; a drop there, outside the list, or Escape cancels with no event. Near the top or bottom of the reading's scrolling area, the reading scrolls,
faster the nearer the edge.

The drag uses pointer events with the pointer captured on the handle, which also makes a touch drag
on it work; it starts once the pointer has moved 4 px, so a click on the handle selects the item. On touch, the browser can aim a press near the icon at the fold chevron beside it, so a
press there that lands within the handle starts the drag too, and the handle and that chevron take
no touch panning.

### 3. Where items move

An item moves when it is an element that is an item of a sequence (`choices={ … }`, a `let`'s
`{ … }`) or of element content (`steps`, a menu's drinks), its list has two or more items, and the
list holds no text. Items an `if` or a `for` makes are code: they do not drag, and nothing drops
among them, but an item can move past an `if` or `for` block, which is an item of the list. The
items of an element inside a branch or a loop body are that element's own list, and move within it. Literal
items (`{ 4 2 5 }`) are not selectable units in Visual, so they do not move in this change.

Dragging works where the reading draws the list an item to a line, which is how a group draws what
it holds and how a list property's row draws its elements. Move up and Move down work wherever the
item moves.

### 4. `move` names a position, not a sibling

`{ kind: "move", to }` takes the item's position once moved, counted as the list's keys count. The
viewer knows the positions from the keys, both a drag and Alt+Down map to one number, and the moved
item's newer key is the list's position `to`. A `before: key` form was considered; it needs an "at
the end" case and says nothing more.

### 5. The language service keeps the layout

- **A list written an item to a line** (each item, with its own lines, shares no line with another):
  the item's whole lines move, with the comments and doc comments directly above it (no blank line
  between), to stand before the item now at `to`, or after the last item. The blank lines between
  the item and the next one (or the one before, when it is last) go with it, so a list that spaces
  its items a blank line apart keeps that spacing; other blank lines stay where they are. The item
  keeps its indentation, which its siblings share.
- **A list written on one line**: the item moves within the line, with the space that separated it
  from its neighbour.
- **Any other layout** is refused `notEditable`; Visual then leaves the reading as it was.

The answer holds the edits that remove the item and insert it, which never overlap.

### 6. Delete reuses `remove` and the row's keys

`remove` already removes an attribute; it now takes an item too, the same items `move` takes, and
removes its lines and the comments directly above it, as a move does at its source. Delete and
Cmd+Backspace, which remove a selected row, remove a selected item, and the item's menu lists Delete.
There is no confirmation: undo is one keystroke and a title-bar button away. The viewer does not offer
to delete the only item of a list typed `+`, so the reading never offers an edit that breaks the type;
the language service, as for properties, does not check, so a host that sends it gets the type
error. Nothing checks references by id string, such as a survey condition naming a question's id: the
type checker cannot see them, so deleting a question others name is not stopped. Typed id references
would be a later change.

### 7. The selection, folds and announcements follow the item

Since keys are positions, a move or a delete changes the keys of the item and of every item between its old and
new places. The viewer remembers the move it fired, and on the host's newer tree selects the item
at `to` (after a delete, the item that took its place, else the one before it, else what holds
the list) and maps each fold on that list's items to its item's new key, firing `nx-fold` with the
keys as they are now. A polite live region says "Moved to 3 of 8" or "Deleted". If the host refuses, the tree
comes back unchanged and nothing moves.

### 8. The item menu

A selected item that moves or can be deleted gets the quiet `⋯` a written row has, at the end of its first line (a
group's header, or the line of a one-line element): Move up, Move down (each only where the item has
a neighbour that way), Delete and Show in Source. It is the row menu's look and keyboard (the context-menu
key, Shift+F10); deferred actions such as Duplicate would join it later.

## Risks / Trade-offs

- [The handle is small and has no mark of its own] → the grab cursor and the hover card's `Drag to
  reorder` say it, and Move up and Move down are in the menu and on Alt+Up and Alt+Down.
- [Alt+Up and Alt+Down on macOS scroll by a page in some browsers] → only while the reading has
  focus and a movable item is selected, where the viewer prevents the default.
- [Comments that belong to an item but stand apart from it with a blank line stay behind] → only
  the comments directly above move; the reader sees the result in Source and can undo.
- [A long drag over a long list scrolls a lot] → the speed grows with nearness to the edge, and
  Escape cancels.
