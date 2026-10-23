## Why

Visual edits values and adds properties, but cannot change the order of what a list holds. In a
survey, the order of steps and of a question's choices is most of the design, and reordering is
hard to say in an AI prompt ("move the email question above the role question") and easy to say
with a pointer. This change is the first slice of phase 5, structure editing, of the design "Editing
in Visual: design and plan" (https://claude.ai/artifact/66bGJxr9DjiS9GY5jk86Xu, section "Structure
editing (phase 5): reordering first"). Bret narrowed phase 5 to reordering on 2026-10-23: adding,
inserting, duplicating and deleting elements are most likely done by prompt, and leaving them out
avoids the insert menu and its questions. Bret then asked for deleting too, since it is small: the
language service already removes properties, and Delete already removes a selected row. Dragging is the most intuitive way to reorder, so Visual
offers it where it works best, a list read an item to a line, with Move up and Move down as the
second way.

## What Changes

- **A `move` source edit.** `sourceEdit` gains `{ kind: "move", to }`: the target is an element that
  is an item of a sequence or of element content, and `to` is its position in the list once moved.
  The language service moves the item's text, with the comments right above it, and keeps the
  list's layout: whole lines in a list written an item to a line, in place in a list written on one
  line. It refuses an item inside an `if` or a `for`, a list that holds text, and a position outside
  the list.
- **Dragging in Visual.** With `editable` set, an item that can move shows a grip in the gutter left
  of its first line, on hover and while selected. Only the grip starts a drag, so values, steppers,
  the name scrub and text selection keep working. While dragging, the item dims in place, a
  one-line copy follows the pointer, and a line marks the gap where it will land, only between
  items of its own list. Nothing else moves until the drop, which fires one `nx-edit`. Escape or a
  drop outside the list cancels, and the reading scrolls near its edges.
- **Move up and Move down.** Alt+Up and Alt+Down move the selected item. These work in a list the
  reading draws on one line too, where there is no grip.
- **Delete.** `remove` takes an item too, with the comments right above it. Delete or Cmd+Backspace on
  a selected item fires it, with no confirmation since undo takes it back; the only item of a list
  typed `+` cannot be deleted.
- **An item's menu.** The selected item gets the quiet `⋯` menu a written row has: Move up, Move down,
  Delete and Show in Source.
- **The selection and folds follow** a moved item, or move to the next item after a delete, across
  the host's newer tree, and a live region says what happened.
- **The playground**'s "Visual test cases" example gains lists to reorder.
- Not in this change: adding, inserting and duplicating elements, cut, copy and paste, the
  insert menu, moving between lists or levels, and dragging several items.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `source-edits`: the `move` edit, `remove` for an item, and their refusals.
- `viewer`: dragging an item by its grip, Move up and Move down and Delete from the keyboard and an
  item's menu, and the selection and folds following a moved or deleted item.
- `playground`: the test-cases example covers reordering and deleting.

## Impact

- `crates/nx-language-service`: `source_edit` (`move`, `remove` of an item) and tests, including a round trip over the
  repository's `.nx` files.
- `packages/language-protocol`: the `move` edit; `packages/language-http`: its validation.
- `packages/viewer`: the grip, the drag, the keys, the item menu, styles, tests and README.
- `sites/playground`: the test-cases example.
- `src/vscode/CHANGELOG.md` and the package changelogs.
