# Review: add-visual-inline-inspector

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/viewer, specs/fiddle-nx-visual-view,
specs/component-props-type, specs/visualizer-library, specs/visualizer-declarations, specs/source-tree,
specs/graphics-library  
**Reviewed code:** net diff `d5b621e..HEAD`, which is the change's own commits. The prototype commits
1b8a994/2a6616c/3d37013 cancel out. `git merge-base HEAD origin/main` resolves to an older commit
(5dd8e67) because the local `origin/main` is stale, so that range also pulls in unrelated commits.
Files: packages/viewer/src/{source,index,labels,compact,styles,edit}.ts, packages/viewer/test/*,
packages/viewer/README.md, packages/language-protocol/src/index.ts, crates/nx-hir/src/{lower,visualizers}.rs,
crates/nx-types/src/infer.rs, crates/nx-program/src/{program_visualizers.rs,std/visualizer/visualizer.nx,std/graphics/graphics.nx,visualizer_declarations_tests.rs},
crates/nx-language-service/src/{source_tree,source_tree_tests,visualizers}.rs, sites/playground/src/{App.tsx,reading/ReadingView.tsx,examples/nx/*},
sites/playground/scripts/check-inspector-browser.mjs, website visualizer docs  
**Verification run:** `pnpm test` in packages/viewer (220/220 pass); `cargo test -p nx-program -p
nx-language-service -p nx-hir -p nx-types -p nx-api` (all pass); `pnpm run check-examples` (26/26);
`check-inspector-browser.mjs` with `CHROME_PATH=/usr/bin/google-chrome` (PASS); `openspec validate
--strict` (valid). Extra jsdom probes and a Playwright/Chrome probe (scratchpad, not committed) back
the findings below.

## Findings

### ✅ Verified - RF1 Keyboard users cannot reach a row's pin, and a focused pin or other row button swallows the arrow keys
- **Severity:** High (confidence: high, verified in jsdom and in real Chrome)
- **Evidence:** The spec ("Rows and inspectors are keyboard accessible") says Tab SHALL move from a
  row into its open inspector's fields *and pin*, and the pin SHALL be "reachable without hover".
  Its scenario reads: "selects Star, tabs to its pin, presses Space and moves down to Circle → both
  inspectors open". Three things stop that:
  1. `index.ts:4314-4332` intercepts Tab on the body whenever the selected row's inspector has a
     field. It opens the first editable value, or selects the first field when the document is
     read-only, and calls `preventDefault`, so focus never reaches the pin. Selection opens the
     inspector, so it almost always has fields.
  2. Once the selection moves into a field, `#selectNode` (`index.ts:1640-1666`) resets every
     pin's `tabIndex` to -1. Only a selected *row* gets 0, so no pin is a Tab stop any more.
  3. `#onKeyDown` returns early for any key pressed on a button (`index.ts:4258`). With focus on
     the pin, which `#toggleDetails` puts there after every pin click (`index.ts:793`), Up and Down
     do nothing. The same happens on a group heading or a value-fold chevron after they are toggled
     (`index.ts:826`, `index.ts:841`).
  
  Probe in Chrome (graphics preset, Star selected, body focused, Tab ×6): focus went to cx, cy,
  outerRadius, innerRadius, pointCount, rotation fields and never to the pin. With the pin focused,
  ArrowDown left the selection on Star. The tests miss this. `inspector.test.ts:380` only checks the
  pin's `tabIndex`, and `check-inspector-browser.mjs:100` calls `.focus()` on the pin directly
  instead of tabbing to it. Task 2.7 is marked done.
- **Recommendation:** Let Tab from the selected row reach the pin, for example pin first and then
  the fields. Keep the pin a Tab stop while the selection is inside its inspector, or make Shift+Tab
  from the first field go back to it. In `#onKeyDown`, let row-navigation keys (Up, Down, Left,
  Right, Enter on a row) through when the focused button is a pin, chevron or group heading, or move
  focus back to the body after the click. Add a test that presses Tab until the pin has focus, then
  Space, then ArrowDown.
- **Fix:** Tab from a selected row now focuses its pin, and Tab from the pin goes into the open inspector (`#enterInspector`). The pin of the row the selection is in (the row or one of its fields) stays a Tab stop. Up and Down on a focused button (pin, group heading, value chevron) move focus back to the reading and walk the rows. New tests: Tab to the pin, then into the inspector; arrows from a focused pin.
- **Verification:** Verified. In Chrome (graphics preset, Star selected, Tab ×6 from the body), focus went to `Pin Star` first, then into cx, cy, outerRadius and the other fields. ArrowDown on the focused pin moved the selection to the next row. In jsdom the selected row's pin stays at tabIndex 0 while the selection is in its row. The new tests pass.

### ✅ Verified - RF2 An invalid entry now traps the whole page: blur never cancels, and every press anywhere is swallowed
- **Severity:** Medium (confidence: high, verified)
- **Evidence:** `#finishField(…, "blur")` was the only path that dropped an invalid entry when the
  reader left the field. The field's blur listener now passes `"key"` (`index.ts:2116-2119`), and no
  caller passes `"blur"` any more, so the branch at `index.ts:2328` is dead. The document-level
  *capture* `pointerdown` handler also calls `finishEditing()` for a press anywhere outside the
  field. When that fails it calls `preventDefault()` and `stopImmediatePropagation()`
  (`index.ts:491-498`). With an invalid draft, no pointerdown on the page reaches its target: the
  preset `<select>`, Run, Share, the preview, or another app widget. Because `preventDefault()` on
  pointerdown also suppresses the compatibility mousedown, focus doesn't move either.
  
  Probe: after typing `abc` into Star's X and clicking the preset combobox in Chrome, focus stayed
  in `nx-viewer` and the field stayed open. In jsdom, a listener on a button outside the viewer
  never saw the pointerdown.

  The new requirement only asks that "closing an inspector, or a selection that would close it"
  keeps an invalid field open. The unchanged requirement "Edits go live…" still says "on cancel, as
  Escape or **leaving a field holding an entry it cannot take** does…". The host already guards
  its own Visual→Source switch through `finishEditing()` (`App.tsx:360`).
- **Recommendation:** Restore `"blur"` for real focus loss, or at least for presses outside the
  viewer, and drop the invalid entry there as before. Keep the guard only for presses and
  selections inside the reading that would close or replace the inspector holding the field. Never
  call `stopImmediatePropagation` on presses outside the element. Add a test that a press outside
  the viewer reaches its target, and that blur with an invalid entry cancels.
- **Fix:** Decided: a press or focus outside the viewer drops the invalid entry, as the unchanged live-edit requirement says; inside the viewer it still keeps the inspector and field open. The document pointerdown handler only blocks presses inside the viewer, and a field's blur cancels an invalid entry when focus moves to something outside the viewer (a blur with no related target, such as switching windows, keeps it). A new test covers the press outside.
- **Verification:** Verified against the decided behavior. In jsdom, an invalid entry plus a press on a button outside the viewer: the press reaches its target, the field closes, the value reads as before, and no edit fires. A valid entry plus a press outside commits once. A blur whose relatedTarget is outside the viewer drops the entry; a blur with no relatedTarget keeps it, as stated. In Chrome, clicking the preset `<select>` with `abc` in Star's X focused the SELECT and closed the field. Presses inside the viewer are still blocked, and `check-inspector-browser.mjs` passes.

### ✅ Verified - RF3 Alt-scrubbing an integer property writes a fractional literal
- **Severity:** Medium (confidence: high, verified)
- **Evidence:** `#onScrubMove` (`index.ts:3616-3619`) divides the step by 10 for Alt whatever the
  type. `stepNumber` then keeps one decimal, so an `int` gets a real literal. Probe with
  `type Box = { count:int = 3 }` and `<Box count=3 />`: an Alt-drag fired live edits `3.0`, `3.1`
  and `3.2`, then committed `count=3.2`, which is a type error in the document. The first live
  edit `3.0` also happens with no movement (zero steps), because the text differs from `3` only in
  formatting. The only Alt test (`editing.test.ts:1260`) uses a `float64`.
- **Recommendation:** Ignore Alt, or keep the step at 1, for integral targets. The integrality is
  already known for `derivedStep`/`FLOAT_TYPES`, so carry it on the `EditTarget`. Don't fire a
  live edit when the step count is 0. Add an `int` Alt-scrub test.
- **Fix:** `boundedHints` marks whole-number types with `integer: true`, and Alt scrubbing steps a whole number by at least one. Zero steps fire nothing. New test: Alt-scrubbing an `int` writes whole numbers.
- **Verification:** Verified. Probe `count:int = 3` with an Alt-drag of 10 px: live edits fire `4`, `5`, `6`, then commit `6`. No fractional text, and no zero-step live edit.

### ✅ Verified - RF4 A host-set selection refused over an invalid entry leaves the host out of sync
- **Severity:** Low (confidence: medium)
- **Evidence:** `#select` restores `priorState` and returns when `finishEditing()` fails
  (`index.ts:1584-1586`). This is the intended behavior for a reader's click, but it also runs for
  `element.selection = …` set by the host. The viewer keeps the old selection and fires no
  `nx-select`, so the playground's `selection` state names an element the viewer doesn't show as
  selected. The test at `inspector.test.ts:374-377` asserts this ("which does not happen"). The
  ReadingView effect (`ReadingView.tsx` selection layout effect) only re-applies on change, so the
  two stay apart until the next selection. One case: the host's carry-over selection on the next
  switch uses the refused key.
- **Recommendation:** When a host-set selection is refused, fire `nx-select` with the kept
  selection so the host can correct itself. Alternatively, apply host selections without the
  invalid-entry guard.
- **Fix:** A refused host selection now fires `nx-select` with the selection the viewer keeps (`#announceSelection`). Covered by the new outside-press test.
- **Verification:** Verified. In jsdom, the host assigns `selection = "b.value"` while `a.value.count` holds an invalid entry: the selection stays `a.value.count`, and `nx-select` fires once with `a.value.count`.

### ✅ Verified - RF5 A summary item whose value is an element reads only the element's kind
- **Severity:** Low (confidence: high)
- **Evidence:** `summaryItem` (`source.ts:1013-1016`) writes only `tree.nodes[at].name` for an
  element value. The spec says each item "SHALL read as its inspector field would, without the
  field … or a swatch and the value for a color", and the field shows the element's icon, kind and
  summary. As shipped, Drawing's curated `summary={viewBox background}` reads `View box Rect` and
  `Background Paint.color`. An inline `fill=<Paint.color color=#ffffff/>` reads `Paint.color` with
  no swatch, so siblings can't be told apart, which is the summary's purpose. This is separate from
  the accepted gap for `let` references.
- **Recommendation:** Render the nested element's own summary items, or its color swatch when its
  visualizer formats a property as `color`, inside the item. Otherwise drop element-valued
  properties from graphics' curated summaries (Drawing).
- **Fix:** An element value that writes exactly one one-line, non-element property reads as that value in a summary (a `Paint.color` reads as its swatch and hex). Other element values still read by kind, such as `Rect`; checked in the playground.
- **Verification:** Verified. In Chrome, Drawing's summary reads `Rect | #ffffff` (the background `Paint.color` shows its swatch and hex), and Circle reads `R30, blue`. Element values that set more than one property still read by kind, which is acceptable.

### ✅ Verified - RF6 Scrub distance is counted from the end of the 4 px slop, not from the press, unlike the spec's scenarios
- **Severity:** Low (confidence: high)
- **Evidence:** `#onScrubMove` resets `scrub.x` to the pointer once the slop is passed
  (`index.ts:3603-3610`), so N px gives `trunc((N−4)/3)` steps. The spec's scenarios count from the
  press: "drags … 30 pixels to the right → reads `46`" for `outerRadius=36`, and "drags the caption
  of `size=16` nine pixels → `17`, `18`, `19`". The code gives `44`, and `17`, `18` for those. The
  test encodes a 13 px drag for three steps (`editing.test.ts:1253`).
- **Recommendation:** Either count steps from the press point once the slop is passed, or reword
  the scenarios to say the count starts after the 4 px threshold.
- **Fix:** Decided for the spec: steps now count from the press, so 30 px is ten steps (46 in the scenario). Scrub tests updated.
- **Verification:** Verified. In Chrome, a 30 px drag on Star's outer-radius lead took `outerRadius` from 36 to 46, matching the spec scenario.

### ✅ Verified - RF7 The viewer README still documents steppers and "Show N unset properties"
- **Severity:** Medium (confidence: high)
- **Evidence:** `packages/viewer/README.md:308-366` describes steppers ("A number shows steppers, an
  up and a down button…", "Pressing a stepper and dragging further up…", stepper tooltips, room on
  a line). Lines 318 and 329 list steppers in the editor tables, line 383 has "past its steppers or
  chevron", and line 424 says "A selected element ends with a quiet row, `Show N unset properties`".
  All of that UI is removed. Task 3.4 is marked done, and AGENTS.md asks for related documentation
  to be updated.
- **Recommendation:** Rewrite those sections for scrubbing on leads, Up/Down stepping and dashed
  unset fields in the inspector.
- **Fix:** README editing sections rewritten for scrubbing, chevrons inside fields and unset fields in the inspector; no stepper or `Show N unset properties` text remains.
- **Verification:** Verified. The README has no stepper or `Show N unset properties` text left. The remaining mention of unset properties (line 100) describes the dashed fields correctly.

### ✅ Verified - RF8 Several spec scenarios and task items have no test
- **Severity:** Low (confidence: high)
- **Evidence:**
  - Task 1.1 promises "tests for props order, defaults and rejection for paren-style functions".
    `visualizer_declarations_tests.rs::an_element_function_has_a_props_record` checks that
    `Badge.Props` exists and that `double.Props` is rejected. It doesn't check the spec's case
    order (`label`, `tone`) or that `F.Props` drops defaults; a probe shows `<Badge.Props
    label="x"/>` is correctly rejected for missing `tone`, but no test pins it.
  - Nothing tests Tab reaching the pin, blur with an invalid entry, a press outside the viewer, or
    Alt on an integer (RF1–RF3).
  - Nothing tests scrubbing carried through a new or a cleared tree. The old stepper tests for this
    were deleted, but `#render` still carries `#scrub` across trees (`index.ts:1243-1249`).
- **Recommendation:** Add these tests.
- **Fix:** Added `an_element_functions_props_record_keeps_parameter_order_and_no_defaults` (nx-types) and tests for RF1 to RF4. A scrub carried through a new or empty tree is still untested; the scrub keeps its target, and its live edits go through the existing live-edit path, which has tree tests.
- **Verification:** Verified. `an_element_functions_props_record_keeps_parameter_order_and_no_defaults` passes (`cargo test -p nx-types --test component_props_type`: 13 passed) and checks the order `label`, `tone`, `count`, the optional mark, and the rejection of a missing `tone`. The viewer tests for RF1–RF4 pass (223/223). A scrub carried across a new or empty tree is still untested; that gap is accepted as deferred and is low risk.

### ✅ Verified - RF9 Dead code and stale comments from the removed steppers and unset list
- **Severity:** Low (confidence: high)
- **Evidence:**
  - `Live.settle` (`index.ts:281-282`) is only ever initialized to `undefined`
    (`index.ts:1905`).
  - Comments still describe steppers: `index.ts:92`, `index.ts:501` ("Stepper clicks made with the
    field closed…"), `index.ts:3472` and `index.ts:4259`.
  - `styles.ts:677-740` keeps `.more-toggle`, `.more-filter-row` and `.more-block` rules for
    elements that are no longer created, and `index.ts:4263` still looks for `.more-block`.
  - In `edit.ts:136-142`, inserting `LARGEST_FLOAT32` between `derivedStep`'s doc comment and the
    function leaves two JSDoc blocks in a row. The function loses its doc, and the constant gets
    the wrong one.
- **Recommendation:** Remove the dead field, rules and comments, and move the constant above the
  doc comment.
- **Fix:** Removed `Live.settle`, the `.more-toggle`, `.more-filter` and `.more-block` CSS, and the `.more-block` key handling; reworded the stepper comments; moved `derivedStep`'s doc comment back onto the function.
- **Verification:** Verified. No `Live.settle`, `.more-toggle`, `.more-filter`, `.more-block` or stepper comments remain in `packages/viewer/src`, and `derivedStep`'s doc comment sits on the function again. The fix left one mis-indented line; see RF10.

## Questions
- None. Two things are left to the author. In RF2, should a press outside the viewer cancel an
  invalid entry, as the unchanged live-edit requirement says, or keep it? In RF6, should the spec or
  the code move?

## Summary
- The language side works and is well tested: `F.Props` for element functions, `FunctionVisualizer`
  `Target`, `summary` resolution, `visualizer-hint` warnings, and the source tree carrying
  label/icon/short/group/summary. `@nx/graphics` labels and groups every property, with a test.
  The viewer's rows, inspectors, groups, pins, host state, reconciliation and anchoring work, and
  the real-browser check passes.
- The main gaps are in input handling. Keyboard access to the pin is broken (RF1), an invalid entry
  blocks presses anywhere on the page (RF2), and Alt-scrub corrupts integers (RF3). The rest are
  smaller spec mismatches, stale docs and missing tests.

## New Findings Discovered During 2026-11-03 03:00 Verification

### ✅ Resolved - RF10 The fix left one line indented two spaces too far
- **Severity:** Low (confidence: high)
- **Evidence:** `packages/viewer/src/index.ts:2444`, `        this.#pending.set(target.key, value);`,
  is indented 8 spaces in a 6-space block. It was introduced when `clearTimeout(live.settle)` was
  removed. It changes no behavior.
- **Recommendation:** Re-indent it to match the surrounding lines.
- **Fix:** Re-indented; whitespace only, so no separate verification pass.

## New Findings Discovered During 2026-11-04 22:36 Review

**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/*/spec.md (as updated for the final state)  
**Reviewed code:** `git diff origin/main...HEAD`, focused on `packages/viewer/src` (source.ts, index.ts,
editors.ts, card.ts, styles.ts, labels.ts, compact.ts), `crates/nx-language-service`, `crates/nx-hir`,
`crates/nx-types`, `crates/nx-program/src/std/{visualizer,graphics}`, `packages/language-protocol`,
`sites/playground`, the viewer README and the website's visualizer pages.  
**Checks run:** viewer build and tests (234/234 pass), `pnpm run test:inspector-browser` (PASS),
`cargo test -p nx-language-service` (331 pass), `openspec validate add-visual-inline-inspector --strict`
(valid). Two throwaway probes against the real language host, removed afterwards, back RF11 and RF13.

### ✅ Verified - RF11 A reference's swatch and Go to look the name up, so a parameter or local that shadows a top-level `let` shows the wrong color and goes to the wrong place
- **Severity:** Medium (confidence: high)
- **Evidence:** `referencedColor` (`packages/viewer/src/source.ts:869-871`) and `#referenceGoal`
  (`packages/viewer/src/index.ts:4260-4263`) both find "the top-level declaration node whose `name`
  equals the reference's name". They never check what the reference resolves to. The source tree
  already resolves it: `source_tree.rs:1072-1078` leaves `declaration` unset for a local and sets it
  through `visible_declaration` otherwise. A probe with
  `let blue = #2563eb` / `let <Badge blue:color /> = <Tag name="t" tint={blue} />` got the reference
  node `{"role":"reference","name":"blue"}` with no `declaration`, but `Badge.body.tint` still drew
  the `#2563eb` swatch. Its `⋯` menu would also offer "Go to blue" and select the unrelated
  top-level `let`. The same thing happens with a `for` item or a component parameter named like a
  `let`.
- **Recommendation:** Resolve the reference through the tree. Take the reference's `declaration`
  (none means it's a local, so no swatch and no Go to), and find the `let` node that declaration
  points to rather than matching by name. Add a test with a shadowing parameter.
- **Fix:** `referencedColor` and `#referenceGoal` now go through the new `topLevelLet(tree, reference)` in `source.ts`. It matches the top-level `let` by the reference's resolved `declaration` instead of by name. A reference the tree leaves unresolved (a local) gets no swatch and no Go to. New test: "finds a reference's swatch and Go to through what it resolves to, not its name" (`inspector.test.ts`).
- **Verification:** Verified. `topLevelLet` (`source.ts`) matches a top-level declaration node by the reference's resolved `declaration`. Both `referencedColor` and `#referenceGoal` use it, and no name-based lookup is left in `packages/viewer/src`. The new test passes: the top-level `{blue}` keeps its swatch, and the shadowing parameter has a `declaration` of undefined and gets no swatch. One weakness: the test's Go to check sits inside `if (menu !== null)`, so it would pass silently if the menu were never drawn. The code path is the same lookup, so this is not a reason to reopen.

### 🟡 Fixed - RF12 Fields still carry browser tooltips, against the spec and README
- **Severity:** Low (confidence: high)
- **Evidence:** The viewer spec's inspector requirement says "No label or field SHALL carry a browser
  tooltip" (`specs/viewer/spec.md`, inspector requirement), and the README says "The browser's own
  tooltips never repeat a card". Three places still set them:
  - `source.ts:783` sets `unit.title = "Drag to change the opacity"` on every color field's `%`.
    That repeats the scrub hint the label card gives (`index.ts:4818-4822`).
  - `source.ts:823` sets `box.title = written` (`"true"`/`"false"`) on the boolean mark. In an
    inspector that mark is the checkbox (`styles.ts:273-286`).
  - `editors.ts:75` sets `span.title = value` again whenever a toggled boolean shows its pending
    value.

  No test checks the no-tooltip rule.
- **Recommendation:** Drop these `title`s. `aria-label` already names the checkbox. Add a test that
  no element inside `.inspector` has a `title` attribute.
- **Fix:** Removed the `title` on the opacity `%` (`source.ts`), the boolean mark (`source.ts`) and `showValue`'s toggled boolean (`editors.ts`). `aria-label` stays. New test: no `[title]` inside `.inspector-fields`, before and after a toggle.
- **Verification:** Reopened: the fix is incomplete. Unset fields still get a tooltip. `defaultShown` (`editors.ts:321`, `mark.title = text`) draws an unset boolean's default for every field revealed by `Show N unset` (`index.ts:4020`). A probe (`type Fill = { tint:color on:boolean = false … }`, `let fill = <Fill tint=#336699 />`, then a click on `Show N unset`) found `.inspector [title]` = `sheet-boolean false=false` and `inspector-more=Hide the properties it does not set`. The new test only looks at written fields, so it misses this. Drop `mark.title` in `defaultShown`, and either drop the `inspector-more` title (its text already says the same) or exempt it in the spec. Extend the test to cover the fields after `Show N unset`.
- **Fix:** (second pass) Removed `defaultShown`'s `title` on an unset boolean (`editors.ts`). The `Show N unset` line now carries its longer wording as `aria-label` instead of `title`. The tooltip test now also reveals unset properties and checks that nothing in the inspector has a `title`.

### ✅ Verified - RF13 `@nx/graphics` property order contradicts the graphics-library spec
- **Severity:** Low (confidence: high)
- **Evidence:** The spec requires "position …, size …, shape …, transform (rotation), appearance
  (opacity), fill, stroke and text (font, size), then accessibility". A probe of the resolved
  visualizers got:
  - Rectangle: `x, y, width, height, rx, ry, fill, fillRule, stroke, transform, opacity, clipPath, clipRule, accessibility`
  - Star: `cx, cy, outerRadius, innerRadius, pointCount, rotation, fill, stroke, transform, opacity, …`
  - Text: `text, x, y, fontFamily, fontSize, fontWeight, fontStyle, textAnchor, fill, transform, opacity, …`

  So fill and stroke come before transform and opacity on every shape. Text's content comes before
  its position, and its font comes before its fill. The cause is that `Shape`'s visualizer
  (`graphics.nx:749-753`) is merged in ahead of `GraphicsElement`'s (`graphics.nx:741-747`). The
  "every property has a label" test (`source_tree_tests.rs`) doesn't check order.
- **Recommendation:** Either fix the order in `graphics.nx`, for example by having `Shape` list
  `transform` and `opacity` before `fill`, or reword the spec to describe the order that ships. Then
  pin one element's order in a test.
- **Fix:** The spec now describes the order that ships, which matches Figma's inspector: position, size and shape (rotation included), then fill and stroke, then transform, opacity, clipping, accessibility and content. Text lists its text first, then position, font and fill. `graphics.nx`'s comment says the same. The graphics order was not changed, so nothing Bret has reviewed moves.
- **Verification:** Verified. The graphics-library spec now describes the shipped order. I checked it against the resolved visualizers from my earlier probe. Rectangle and Star go position, size, shape (with `rotation`), then fill/stroke, then transform, opacity, clipping, accessibility. Text goes `text`, `x`, `y`, font, anchor, fill. The `graphics.nx` comment agrees.

### ✅ Verified - RF14 The website's copy of the `@nx/visualizer` source lacks `inline`
- **Severity:** Low (confidence: high)
- **Evidence:** Under "The library source", the code block in
  `sites/website/src/content/docs/reference/libraries/visualizer.md` differs from
  `crates/nx-program/src/std/visualizer/visualizer.nx` in one place only: the copy is missing the
  `inline?: boolean` field and its doc comment (`visualizer.nx:30-32`). The page's own table
  (line 66) does list `inline?: boolean`.
- **Recommendation:** Paste in the current source. Consider adding a check that the copy matches the
  file.
- **Fix:** Added `inline?: boolean` and its doc comment to the website's copy of the library source. The copy now matches `visualizer.nx` byte for byte (checked by script).
- **Verification:** Verified. The website's "library source" block now matches `visualizer.nx` exactly; my own `diff` shows no differences.

### ✅ Verified - RF15 Leftovers of the removed experiments in comments and docs
- **Severity:** Low (confidence: high)
- **Evidence:**
  - `crates/nx-program/src/std/graphics/graphics.nx:739-740`: "Labels, short labels and groups follow
    design-tool inspectors: Position, Size, Shape, …". `short` and `group` no longer exist, and
    neither do group headings.
  - `docs/prototypes/README.md:3-4` is added by this diff. It says "The production implementation is
    in `packages/viewer` (`compact` and `detailsState`)" and links to
    `packages/viewer/README.md#compact-authoring-view`, an anchor that doesn't exist. Its steps 3
    and 8 (lines 22-27) describe the removed Details button. `docs/visual-ux-improvements.md` also
    talks about "compact summary" rows (line 38). That is less of a problem if it is explicitly kept
    as #71's history.
  - `packages/viewer/README.md:372` still says a case's chevron is placed "room on a line included".
    That is the stepper-era layout rule that design §6 says was deleted.
- **Recommendation:** Reword the graphics comment. Point the prototype README at the inline
  inspector (or mark it as #71's historical prototype) and fix the link. Drop "room on a line
  included".
- **Fix:** Reworded the `graphics.nx` comment. `docs/prototypes/README.md` now calls itself PR #71's historical prototype and links to the README's "Rows and inspectors" section. Dropped "room on a line included" from the viewer README. `docs/visual-ux-improvements.md` is #71's own write-up and is left as history.
- **Verification:** Verified. `graphics.nx`, `docs/prototypes/README.md` and the README hold no `short labels`, `compact-authoring-view` or `room on a line` text. The prototype README now calls itself #71's history and links to `#rows-and-inspectors`, which exists. Its numbered steps still describe Details, which is acceptable for a page marked as history.

### ✅ Verified - RF16 `inline`'s doc comment and the proposal say more than the code does
- **Severity:** Low (confidence: high)
- **Evidence:**
  - `ElementVisualizer.inline` (`visualizer.nx:30-31`; protocol `index.ts` `SourceVisualizer.inline`)
    says "`true` always". The code doesn't do that. `inlineable` (`source.ts:1245-1270`) still
    refuses an element value under `inline=true` when any property is an element, a list, an
    expression or spans lines. Only the four-property and 24-character limits are lifted. The
    viewer spec has it right: "`true` reads any such value of one-line properties".
  - The proposal says derived labels split at "letter–digit edges". `derivedLabel` (`labels.ts:16-19`)
    doesn't, and the spec's scenario expects `x1` → `X1`.
- **Recommendation:** Make the doc comments say "`true` whenever each property is one simple value on
  a line", and drop "letter–digit edges" from the proposal.
- **Fix:** `inline`'s doc (`visualizer.nx`, the website copy, the protocol's `SourceVisualizer.inline`) now says "`true` whenever each property is one simple value on a line". Dropped "letter–digit edges" from the proposal.
- **Verification:** Verified. `visualizer.nx`, the website copy and `SourceVisualizer.inline` now say "`true` whenever each property is one simple value on a line", which matches `inlineable`. The proposal no longer mentions letter–digit splitting.

### ✅ Verified - RF17 Doc comments stacked onto the wrong functions in the color code
- **Severity:** Low (confidence: high)
- **Evidence:**
  - `packages/viewer/src/editors.ts:426-433` has three JSDoc blocks in a row before `colorFromPart`:
    `preserveHexAlpha`'s ("Replace RGB while retaining…"), `expandHex`'s ("A hex color written out in
    full…"), and `colorFromPart`'s own. As a result `expandHex` (line 459) and `preserveHexAlpha`
    (line 480) have no doc. This is the same pattern RF9 fixed in `edit.ts`.
  - `packages/viewer/src/index.ts:2175` sits above `#openColorPart`'s doc. It is `#openField`'s doc
    ("A field in the value's own place…"), and `#openField` at line 2238 now has none.
- **Recommendation:** Move each comment back onto its own function.
- **Fix:** Moved `expandHex`'s and `preserveHexAlpha`'s doc comments onto those functions, and `#openField`'s back onto `#openField`.
- **Verification:** Verified. Each function now has its own doc comment: `colorFromPart`, `expandHex` and `preserveHexAlpha` in `editors.ts`, and `#openColorPart` and `#openField` in `index.ts`. No stacked blocks remain.

### ✅ Verified - RF18 A shorthand color shows a three-digit hex where the spec says six digits
- **Severity:** Low (confidence: high)
- **Evidence:** The viewer spec says a color literal in an inspector reads as "its swatch, its
  six-digit hex and its opacity". The proposal and design §11 say the same. `hexParts`
  (`source.ts:2317-2320`) returns the literal as written for `#abc`, and only removes the alpha digit
  for `#abcd` (giving `#abc`). So `.color-hex` shows `#abc`, and the hex field opens on `#abc`
  (`index.ts:2181-2183`). The color tests only use `#33669980`
  (`editing.test.ts:1604-1672`).
- **Recommendation:** Either show `expandHex(rgb)` (writing back already expands when the hex
  changes), or say in the spec that a shorthand reads as written. Add a shorthand case to the
  tests.
- **Fix:** `hexParts` now gives the color in six digits (`#abc` → `#aabbcc`, `#abcd` → `#aabbcc` at 87%), so the hex reads and opens in six digits. Writing back already expanded. New test: "reads a shorthand color's hex in six digits".
- **Verification:** Verified. `hexParts` expands a three-digit color, so `#abc` reads `#aabbcc` at 100% and `#abcd` reads `#aabbcc` at 87%. That is the new test. `colorFromPart` compares the entry against the six-digit `rgb`, so leaving the hex field unchanged still writes nothing. A new hex keeps the shorthand alpha doubled.

### ✅ Verified - RF19 Risky new paths have no tests
- **Severity:** Low (confidence: high)
- **Evidence:** All 234 viewer tests pass, but nothing exercises the following:
  - `inline=true` lifting the four-property or 24-character limit. Only `inline=false` is tested
    (`inspector.test.ts:368`).
  - An inline union case, where only the case is drawn (`impliedKind` "case", `source.ts:1281-1292`).
  - An actual scrub on an inline label. `inspector.test.ts:332` only checks the `scrubbable` class.
  - Hover cards under a held button, during a drag, or after a wheel (`#point` and `#quietCards`,
    `index.ts:726`, `715`, `4758-4798`). `hover.test.ts:368` covers only a press.
    Browsers can send a `pointerover` for whatever scrolls under a still pointer, and `#point` then
    clears `#cardQuietKey` (`index.ts:4768-4770`). So after a wheel scroll a card can still open
    for what scrolled underneath. That is worth checking against the spec's "nor for what the
    pointer was over … when the reading scrolled". (Confidence on this point: medium.)
  - The no-tooltip rule (RF12), shorthand colors (RF18) and shadowed references (RF11).
- **Recommendation:** Add these unit tests. Add a wheel case to the browser check.
- **Fix:** Added tests for `inline=true` past the four-property limit, an inline union value reading only its case, and a scrub on an inline label (live edits, one commit). Shadowed references, the no-tooltip rule and shorthand colors are covered above. The wheel case was a bug: `pointerover` for what scrolled under a still pointer cleared the quiet key. A wheel or scroll now records where the pointer stood, and no card opens until a `pointermove` leaves that spot (`#cardStillAt`, `#pointerMoved`). New test: "opens none for what a wheel scrolls under a still pointer, until the pointer moves". The spec and README now say so. Held-button cards were already tested.
- **Verification:** Verified. The new tests pass: `inline=true` with five properties, an inline union showing only its case, a scrub on an inline label (live edits, then one commit of `5`), shadowed references, tooltips on written fields, shorthand colors, and the wheel case. The viewer suite passes 240/240. The wheel fix (`#cardStillAt`, `#pointerMoved`) works for a wheel. It does not cover other scrolls; see RF20.

### Questions (2026-11-04 pass)
- RF13: the spec was changed to match the shipped order (see its Fix).

### Summary (2026-11-04 pass)
- The language side is solid: `F.Props` for element functions, `FunctionVisualizer` property
  visualizers, `label` and `inline` reaching the tree, and the graphics labels. The viewer passes its
  unit and browser checks, and no `summary`, `short`, `group`, property `icon`, stepper or `compact`
  code is left.
- RF11 is the one behavioral bug: a shadowed reference gets another `let`'s swatch and Go to. The
  rest are spec/code mismatches (tooltips, graphics order, six-digit hex), stale docs and comments,
  and missing tests.

## New Findings Discovered During 2026-11-04 22:50 Verification

### 🟡 Fixed - RF20 Scrolls other than the wheel still let a card open for what scrolls under a still pointer
- **Severity:** Low (confidence: medium)
- **Evidence:** `#cardStillAt` is set only by `#quietCards(true)`, which runs in two places: on a
  `wheel` (`index.ts:723`) and from `#onDocumentScroll`. `#onDocumentScroll` is attached to the
  scroller only while a card is open; `#openCard` adds it and `#hideCard` removes it. With no card
  open, other scrolls set nothing. That includes keyboard paging, arrow-key selection that
  `scrollIntoView`s the next row, a preview click revealing a row, and a scrollbar drag. The
  scrollbar case is a press, but its quiet key is cleared by the next `pointerover`. In each case,
  the `pointerover` that browsers send for what moved under the pointer reaches `#point`, which
  opens a card after a second. The spec now says no card opens "for what the reading scrolls under
  a still pointer, until the pointer moves". The confidence is medium because it depends on the
  browser sending boundary events after a scroll that doesn't come from the wheel. The new hover
  test covers only a wheel.
- **Recommendation:** Keep a `scroll` listener on the scroll port at all times, not only while a card
  is open, and have it call `#quietCards(true)`. Then add a test that scrolls the port with no card
  open.
- **Fix:** Instead of listening for every kind of scroll, the viewer now notices when the pointer stays still. In a browser, a pointer that moves gets `pointerover` before it moves. So a `pointerover` at the exact spot of the last `pointermove` means something arrived under a pointer that did not move: a scroll of any kind, or a redraw. That sets `#cardStillAt`, and no card opens until the pointer moves (`index.ts`, pointerover handler). The test helper `pointAt` now sends a `pointerover` and then a `pointermove`, as browsers do. New test: "opens none for what scrolls under a still pointer however the reading scrolled". Checked in Chromium that cards still open on a real hover.
