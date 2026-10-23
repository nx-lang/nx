## 1. The `move` edit and item removal in the language service

- [ ] 1.1 `SourceEdit::Move { to }` in `crates/nx-language-service/src/source_edit.rs`: find the item's list (a sequence or element content, not a branch or loop body, no text), refuse as the `source-edits` spec says, and answer the edits that remove the item, with the comments directly above it, and insert it at `to`, in the list's layout (whole lines, or within one line); verify with tests for each scenario in `source-edits`
- [ ] 1.2 `remove` of an item: the same items and refusals as `move`, its lines and the comments directly above it, no doubled blank line, or within one line; verify with tests for each scenario in `source-edits`
- [ ] 1.3 Extend the round-trip test over the repository's `.nx` files: for every item that moves, moving it to each position yields a document that parses, with the list's items in the new order and every key outside the list unchanged; removing each yields one that parses without it
- [ ] 1.4 The protocol: `{ kind: "move"; to: number }` in `packages/language-protocol`; its validation in `packages/language-http`; the TypeScript parity tests over a corpus with moves

## 2. Moving and deleting in the viewer

- [ ] 2.1 Which items move: elements that are items of a sequence or element content, outside branches and loop bodies, in a list of two or more items holding no text, with `editable` set; verify with unit tests over fixtures
- [ ] 2.2 The handle: the item's icon, and its name where shown (the name alone without `icons`), only where the reading draws the list an item to a line; a grab cursor, `Drag to reorder` at the end of its hover card, a click still selecting
- [ ] 2.3 The drag: pointer events captured on the handle, closing any hover card, a 4 px start, the item muted in place, a one-line copy following the pointer, the landing line between items of the same list only, nothing else moving, `nx-edit` `move` on a drop that changes the position, cancel on Escape or a drop elsewhere, scrolling near the scrolling area's edges; verify each `viewer` scenario
- [ ] 2.4 Alt+Up and Alt+Down on the selected item; Delete and Cmd+Backspace on a selected item that can be deleted (not the only item of a `+` list); the item's `⋯` menu with Move up, Move down, Delete and Show in Source, opened from the context-menu key and Shift+F10 too; verify each `viewer` scenario
- [ ] 2.5 After a move, select the item at its new position, after a delete the next item, else the previous, else what holds the list, and keep the list's folds on their items in the host's newer tree, firing `nx-fold`; the live region's `Moved to N of M` and `Deleted`; nothing changes when the tree comes back unchanged
- [ ] 2.6 Styles for the grab cursor, the muted item, the copy and the landing line, light and dark

## 3. The playground and docs

- [ ] 3.1 Extend the "Visual test cases" example with the lists in the `playground` scenario, each with a comment saying what to try
- [ ] 3.2 Verify in the browser: drag down and up, Escape, scrolling while dragging, Alt+Up and Alt+Down, Delete, the menu, undo and redo, a folded step, in light and dark and at 375 pixels wide (a touch drag on the icon)
- [ ] 3.3 Document moving and deleting in the viewer's README, the `sourceEdit` `move` edit and `remove` of an item in the language packages' READMEs, and the changelogs
- [ ] 3.4 Run `cargo test`, `pnpm -r build` and `pnpm -r test`, and `openspec validate add-visual-reorder-and-delete --strict`, all passing
