# Review: add-visual-reorder-and-delete

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/source-edits/spec.md, specs/viewer/spec.md,
specs/playground/spec.md, tasks.md (spec commits 414aea5, 3f40cef, a862e3f, and the spec edits made in
1e3dff0)  
**Reviewed code:** `git diff a862e3f 1e3dff0`: crates/nx-language-service/src/source_edit.rs,
source_edit_tests.rs; packages/language-protocol (src, README, test); packages/language-http (src,
test); bindings/wasm/test/language-service.test.ts; packages/viewer/src/move.ts, src/index.ts,
src/styles.ts, test/moving.test.ts, README.md; sites/playground/src/examples/nx/visual-test-cases.nx;
src/vscode/CHANGELOG.md. For context: sites/playground/src/App.tsx, reading/edits.ts,
reading/ReadingView.tsx, editor/NxEditor.tsx.  
**Checks run:** `cargo test -p nx-language-service source_edit` (51 passed, round trip included);
`pnpm test` in packages/viewer (180 passed) and packages/language-http (23 passed); vitest
`bindings/wasm/test/language-service.test.ts` (8 passed); `openspec validate
add-visual-reorder-and-delete --strict` (valid). I also ran throwaway probes against the built
wasm SDK and the built viewer from the scratchpad, without changing the repository. The probes
confirmed RF1, RF2 and RF5.

## Findings

### ✅ Verified - RF1 A move between identical items returns edits that leave the text unchanged, and Visual then waits for a tree that never comes
- **Severity:** Medium
- **Evidence:** `edit_tree` drops each replacement that rewrites its own bytes unchanged
  (crates/nx-language-service/src/source_edit.rs:156). It never checks whether the edits taken
  together change the document. `move_item` (source_edit.rs:647) always returns a removal and an
  insertion. Moving the first of `<F>\n  <D />\n  <D />\n</F>` to 1 returns 2 edits, and applying
  them gives the same text (probe). The source-edits requirement says "An edit that would leave the
  text as it is SHALL answer no edits". The playground then has two problems:
  - It applies the edits, which pushes an empty undo step (NxEditor.tsx `applyEdits`). It calls
    `onApplied`, not `onUnchanged`, so it does not bump the revision. `setSource` gets the same
    string, so no new tree is computed.
  - The viewer's `#following` is cleared only by a tree being set (index.ts:708-716), so it stays
    set. Until some later tree arrives, `#startDrag` refuses every drag (index.ts:2285), and
    `#itemAction` returns nothing for items of any other list (index.ts:2208). Alt+Up/Down, Delete
    and the item menu's Move/Delete therefore do nothing on other lists. A probe on the built viewer
    confirmed this: after an unanswered Alt+Down on one list, Alt+Up and Delete on another list fire
    nothing. Repeated identical items in a list (for example `<Divider />`, `<Spacer />`, or two
    `<Step />` without ids) are realistic.
- **Recommendation:** In `edit_tree`, apply the sorted replacements to `source`. If the result
  equals `source`, return no edits and no refusal. This also covers any future multi-edit kind. Add
  a Rust test with two identical items, and add the case to the wasm parity corpus. As defence in
  depth, the viewer could also drop `#following` when the host re-sets a tree of the same text
  (it already does). Documenting in the viewer README that a host must give a tree back after an
  edit that changed nothing would also help.
- **Confidence:** High. Both the Rust behaviour and the stuck viewer state were reproduced. That
  the playground gives no new tree is read from the code (`setSource` with an identical string).
- **Fix:** `edit_tree` now makes the sorted replacements on the text (`applied`), and answers no
  edits when the result is the text as sent (source_edit.rs). New Rust test
  `moving_past_an_item_just_like_it_changes_nothing` (lines and one line, both directions); the wasm
  parity corpus gained `let same = { <Panel …/> <Panel …/> }` with identical items. With no edits,
  the playground's `onUnchanged` bumps the revision and re-sets the tree, which ends the viewer's
  wait. The viewer README now says a host sets the tree again after an edit it does not make.

- **Verification:** In 07637d3, `edit_tree` builds the edited text (`applied`) and returns no edits
  when it equals the source (source_edit.rs). The new test
  `moving_past_an_item_just_like_it_changes_nothing` covers lines and one line, in both
  directions, and the wasm parity corpus has `let same`. With no edits, the playground's edit
  queue calls `onUnchanged`, which re-sets the tree and clears the viewer's `#following`. The
  viewer README now states the host's duty. `cargo test -p nx-language-service source_edit`
  passes (55), and so does the wasm parity test (8).

### ✅ Verified - RF2 A second Delete on a two-item sequence, before the new tree arrives, sends a key that no longer exists
- **Severity:** Low
- **Evidence:** In a sequence of two (`choices={ <C value="a" /> <C value="b" /> }`, typed `C*`),
  the first Delete sets `following.map.count` to 1. A second Delete before the new tree arrives
  passes `#removeItem`'s guard (index.ts:2266), because `count === 1` and the list does not need
  one item. It fires `remove` again with key `s.value.choices.value[0]` (probe output:
  two identical `remove` events). After the first edit, the remaining item is `{x}` braces. The
  source tree keys it as the sequence (`s.value.choices.value`), as `mappedKey` itself notes
  (move.ts:247). It is no longer an item of a list at all. The service therefore answers
  `unknownNode`, as the probe confirmed. What the reader sees depends on timing:
  - If the first edit's tree has already arrived, Visual selects the row that held the list and
    says "Deleted", although nothing was deleted.
  - Otherwise, the playground's revision bump for the refusal re-sets the old tree. That clears
    `#following`, so the selection and folds do not follow the first delete either.
- **Recommendation:** In `#itemAction` (index.ts:2194), return `undefined` when
  `following.sequence && following.map.count < 2`, since a sequence left with one item is no longer
  a list. Add a viewer test that deletes twice from a two-item sequence before the tree arrives.
- **Confidence:** High for the event sequence and the refusal. Medium for the exact visible effect,
  which depends on timing.
- **Fix:** `#itemAction` answers nothing once a sequence's following map has fewer than two items
  (index.ts), so the second Delete fires nothing. New viewer test "deletes once from a sequence of
  two before the newer tree comes, which leaves no list" (fixture `pair`).

- **Verification:** `#itemAction` returns nothing once `following.sequence && following.map.count
  < 2`, so a second Delete fires nothing. The new test "deletes once from a sequence of two before
  the newer tree comes, which leaves no list" asserts that only one `remove` is fired. Viewer
  tests pass (185).

### 🟡 Fixed - RF3 In a one-line list, an item's hidden `⋯` overlaps the next item and is still clickable
- **Severity:** Low
- **Evidence:** `#markItems` (index.ts:1528-1557) appends the `.item-menu` anchor inside the
  `.line-element` itself. For an element in a list drawn on one line (`options={ <Option …/> <Option
  …/> }`), that is right after its `/>`, in the middle of the line. The mark is absolutely
  positioned at `left: 0` from that point with `margin-left: 0.5ch` and padding (styles.ts:581-598),
  so it covers about 0.5ch to 2.3ch past the item: the separating space and the next item's `<` and
  first letter. Its `opacity: 0` hides it, but it still takes pointer events. A click on the start
  of the next item therefore opens the previous item's menu. Hovering there also triggers
  `.line-element:hover > .item-menu > .row-menu` (styles.ts:602) on the previous item, so its `⋯`
  appears on top of the next item. The viewer spec places the mark "at the end of its first line,
  as a written row does". A written row's mark sits past the row's end (`left: 100%`), where
  nothing is drawn. This was not reproduced in a browser: jsdom does no layout.
- **Recommendation:** Hide the mark in a way that also stops hit-testing, such as `visibility:
  hidden` or `pointer-events: none` while it is not shown, for row menus and item menus alike.
  For an item that shares its line, either show the mark only while the item is selected, or hang
  it at the end of the row's line rather than after the item. Check this in the playground's
  one-line `size` options.
- **Confidence:** Medium. This is from reading the CSS and DOM, not from a browser.
- **Fix:** Hidden menu marks are now `visibility: hidden` rather than `opacity: 0`, so they take no
  clicks (row and item marks alike). An item that shares its line hangs its mark (class `shared`)
  past the end of that line: at the end of the written row whose value holds it, or else past the
  outermost one-line element around it (`#lineEnd`). It shows only while that item is selected
  (`#selectNode` marks it `selected`), and the row's own mark gives way to it then. Hover rules
  exclude shared marks. New viewer test "hangs the menu mark of an item sharing its line at the end
  of its row, shown while selected". Checked in Chromium on a new one-line case in the "Visual test
  cases" example (`let pair`): the mark sits past `} />`, a point at the next item's `<` hits that
  item, and the mark opens Move down, Delete, Show in Source.

- **Verification:** Fixed for a one-line list that is a written row's value or sits inside a
  one-line element. In Chromium, on `let pair` and on `let row = <P options={ … } />`, the marks
  are `shared`, all hang past the end of the line, and only the selected item's mark shows.
  Marks are now `visibility: hidden`, so a hidden mark takes no clicks.

  It is not fixed when `#lineEnd` (index.ts) finds neither a row nor an outer line element. A
  `let` declaration holding a one-line sequence is one such case, for example
  `let xs = { <O value="a" label="A" /> <O value="b" label="B" /> <O value="c" label="C" /> }`.
  There `end` is `undefined`, so the mark is not `shared` and goes back inside the item's own
  `.line-element`. In Chromium, `xs.value[0]` spans x=120–293, its mark x=297–311, and
  `xs.value[1]` starts at x=301. The `.line-element:hover` rule makes that mark visible while the
  pointer is over the item. After hovering `xs.value[0]` and moving to the start of
  `xs.value[1]`, `elementFromPoint` returns `xs.value[0]`'s visible `row-menu`. A click there
  opened `xs.value[0]`'s menu (Move down, Delete, Show in Source) instead of selecting
  `xs.value[1]`.

  Recommendation: treat every item that does not stand on its own line as shared. When
  `#lineEnd` finds no row or outer element, anchor the mark after the list's last item, or at the
  end of the line that holds the list, for example the declaration's line. Add a viewer test for
  a one-line sequence in a `let`.
- **Fix (round 2):** `#lineEnd` falls back to the list that holds the item (a `let`'s one-line
  sequence), so the mark hangs past its `}`; only where none of row, outer one-line element or list
  is drawn around the item does it stay after the item. Every item that shares its line now has a
  `shared` mark, shown only while selected, never on hover. New viewer test "hangs the menu mark of
  an item of a let's one-line sequence after the sequence". Checked in Chromium with a temporary
  `let xs = { <Option …/> <Option …/> <Option …/> }` (not committed): the mark sits past `}`, a
  point at `xs.value[1]`'s `<` hits that item, and the mark opens Move down, Delete, Show in Source.

### ✅ Verified - RF4 The implementation commit changed approved requirements, and one rewritten sentence is hard to parse
- **Severity:** Low
- **Evidence:** 1e3dff0 edits specs/source-edits/spec.md, specs/viewer/spec.md and design.md, as
  well as checking off tasks. The behaviour changes are:
  - A move now carries the blank lines after the item, where the approved text said "blank lines
    between other items SHALL stay where they are".
  - `remove` now removes those blank lines.
  - The one-line move keeps "the space that separated it" rather than "one space from its
    neighbours".
  - A new scenario, "Items a blank line apart".
  - The landing line is no longer drawn at the gaps beside the item.
  - A touch on the fold chevron next to the handle starts the drag (design only).

  These look like reasonable fixes found during implementation. However, the spec was approved
  before implementation, and these edits are not separated out or noted as needing approval. The
  new `remove` wording ("where more than blank lines stand there, it SHALL remove instead, of two
  blank lines that would then stand together, one", source-edits spec, "An item is removed with its
  lines") is very hard to read.
- **Recommendation:** Get Bret's sign-off on these spec changes, ideally as a separate spec commit
  or with a line in the proposal. Reword the sentence, for example: "…and the blank lines between it
  and the item after it, or the item before it when it is the last. When anything other than blank
  lines stands in that gap, it SHALL remove only the item's lines, and, of two blank lines that
  would then stand together, one."
- **Confidence:** High (this is visible in the diff).
- **Fix:** Reworded the `remove` sentence as recommended. These spec edits are small corrections
  found while implementing (blank-line spacing carried with the item, no landing line at the item's
  own gaps, the touch fallback). They are called out to Bret in the thread for sign-off with this
  review.

- **Verification:** The `remove` sentence in specs/source-edits/spec.md now reads clearly. The
  implementer has passed the 1e3dff0 spec edits to Bret for sign-off. That approval is a human
  step outside this review, and no further change is needed here. `openspec validate --strict`
  passes.

### ✅ Verified - RF5 Removing a middle item from a one-line list takes the space after it, where the spec says the space before it
- **Severity:** Low
- **Evidence:** The spec says "In a list written on one line, it SHALL remove the item and the
  space that separates it from the item before it, or after it when it is the first". `cut`
  (source_edit.rs:606-616) tries the gap to the next item first, for every position. With single
  spaces the result is the same text. With uneven spacing it differs:
  `{ <C value="a" />  <C value="b" /> <C value="c" /> }` minus `b` leaves `a  c` with two spaces,
  where the spec gives `a c` (probe). For lists written an item to a line, the spec does say "the
  item after it", so the code's order is right there.
- **Recommendation:** For `ListLayout::OneLine`, prefer the previous gap and fall back to the next
  one for the first item. Keep the current order for the lines layout. Alternatively, change the
  spec to match the code. Add a one-line removal test for a middle item.
- **Confidence:** High.
- **Fix:** `cut` now prefers the gap before the item on one line (after it for the first), and keeps
  the gap after the item first for lists written an item to a line. New Rust test
  `removing_a_middle_item_on_one_line_takes_the_space_before_it` (uneven spacing). A one-line move
  carries the space before it too, which gives the same order of items.

- **Verification:** `cut` now tries the gap before the item first in `ListLayout::OneLine`, and
  the gap after it first in `Lines`. The new test
  `removing_a_middle_item_on_one_line_takes_the_space_before_it` uses uneven spacing. I checked
  that a one-line move using the gap before still gives edits that don't overlap: the insert
  point is before `spans[from-1].1` when moving up, and after `end` when moving down. The
  existing one-line move tests and the repository round trip pass.

### ✅ Verified - RF6 Several spec scenarios and risky paths have no tests
- **Severity:** Low
- **Evidence:** packages/viewer/test/moving.test.ts has no test for:
  - the viewer scenario "A click on the icon" (a click on the handle selects the item, and no drag
    starts);
  - the hover card ending with `Drag to reorder`;
  - the name acting as the handle without `icons`;
  - a drag being refused while a move or delete is still waiting for the host's tree
    (index.ts:2285);
  - repeated Delete before the tree arrives (see RF2);
  - the touch redirect from the fold chevron.

  In Rust, the fallback branch of `cut` (source_edit.rs:620-633), which handles a gap holding more
  than blank lines and removes one of two blank lines that would then stand together, has no unit
  test. It is only reached by the repository round trip if some `.nx` file happens to have that
  layout. A probe shows it works today. There is also no test for a move that leaves the text
  unchanged (RF1).
- **Recommendation:** Add viewer tests for a pointerdown and pointerup without movement on the
  handle (expect it selected and no `nx-edit`), for the hint text in the card, for an
  icon-less handle, and for a drag attempt after an unanswered Alt+Down. Add Rust tests for
  removing and moving an item whose neighbouring gaps hold a comment set off by blank lines, and
  for swapping two identical items.
- **Confidence:** High.
- **Fix:** Added viewer tests for a click on the handle (selects it, no drag, no edit) and the hover
  card ending `Drag to reorder`; the name as the only handle without `icons`; no drag and no
  other list's moves or deletes while a move waits for the tree; repeated Delete (RF2). Added Rust
  tests for removing and moving beside a comment set apart by blank lines (the `cut` fallback) and
  for a move that changes nothing (RF1). The touch fallback from the fold chevron depends on layout
  and touch adjustment, which jsdom has neither of; it stays verified in Chromium only (touch
  drag at 375 px).

- **Verification:** The listed tests are present in packages/viewer/test/moving.test.ts:
  - a click on the handle selects without dragging, and the card ends `Drag to reorder`;
  - the name is the only handle without `icons`;
  - no drag, and no moves or deletes on another list, while a move is pending;
  - repeated Delete on a two-item sequence;
  - the shared mark.

  The Rust tests are present for the `cut` fallback beside a comment set apart by blank lines,
  and for a move that changes nothing. Leaving the touch redirect from the fold chevron to
  browser checks is reasonable, since jsdom has no layout. All pass (Rust 55, viewer 185). The
  RF3 gap above has no test yet.

## Questions
- Can items inside an element that is itself inside an `if` branch or a `for` body be moved? For
  example, the steps of `<Group>` in `if wide { <Group> <A /> <B /> </Group> }`, or cells in a
  `for` template. Both the service (`item_list`, source_edit.rs:471) and the viewer (`itemListOf`)
  check only the direct parent, so today they can be moved and deleted. The spec says "not inside a
  condition's branch or a loop's body", which could be read as "at any depth". Allowing it seems
  useful, especially for a `for` template. If it is intended, the spec could say "not an item a
  condition's branch or a loop's body makes".
- Were the spec edits in 1e3dff0 (RF4) agreed with Bret?

- **Answers (implementer):** Items of an element that is itself inside a branch or a loop body move
  within that element by design: that element's list is its own. The specs now say "not an item of
  a condition's branch or a loop's body", and design decision 3 says so. The spec edits in 1e3dff0
  go to Bret for sign-off (RF4).

- **Reviewer:** The answers are accepted. The spec wording "not an item of a condition's branch or
  a loop's body" matches what the service and the viewer do.

## New Findings Discovered During 2026-10-23 17:56 Verification

- None. RF3 was reopened above.

## Summary
- The change is solid overall. The language service's layout handling (whole lines with the
  comments directly above, blank-line spacing carried with the item, one-line lists, CRLF) is
  well built and well tested, including a round trip over the repository's `.nx` files. Protocol,
  HTTP validation and wasm parity are in place. The viewer covers drag, keys, the item menu, and
  the selection and folds following a moved item, with tests for most scenarios.
- The main problem is RF1: a move that leaves the text unchanged returns edits, which breaks the
  spec and leaves Visual waiting for a tree that never comes, disabling moves and deletes on other
  lists. RF2 and RF3 are smaller interaction bugs. RF4 is a process and wording point about spec
  edits made after approval, RF5 is a small spec divergence, and RF6 lists tests to add.
- **Verification (2026-10-23 17:56), against 07637d3:** RF1, RF2, RF4, RF5 and RF6 are verified.
  RF3 is reopened: a one-line list that is neither a row's value nor inside a one-line element,
  such as a `let` holding a one-line sequence, still hangs each item's mark over the next item,
  where a click opens the wrong menu. No new findings.
