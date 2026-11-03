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

### 🟡 Fixed - RF1 Keyboard users cannot reach a row's pin, and a focused pin or other row button swallows the arrow keys
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

### 🟡 Fixed - RF2 An invalid entry now traps the whole page: blur never cancels, and every press anywhere is swallowed
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

### 🟡 Fixed - RF3 Alt-scrubbing an integer property writes a fractional literal
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

### 🟡 Fixed - RF4 A host-set selection refused over an invalid entry leaves the host out of sync
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

### 🟡 Fixed - RF5 A summary item whose value is an element reads only the element's kind
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

### 🟡 Fixed - RF6 Scrub distance is counted from the end of the 4 px slop, not from the press, unlike the spec's scenarios
- **Severity:** Low (confidence: high)
- **Evidence:** `#onScrubMove` resets `scrub.x` to the pointer once the slop is passed
  (`index.ts:3603-3610`), so N px gives `trunc((N−4)/3)` steps. The spec's scenarios count from the
  press: "drags … 30 pixels to the right → reads `46`" for `outerRadius=36`, and "drags the caption
  of `size=16` nine pixels → `17`, `18`, `19`". The code gives `44`, and `17`, `18` for those. The
  test encodes a 13 px drag for three steps (`editing.test.ts:1253`).
- **Recommendation:** Either count steps from the press point once the slop is passed, or reword
  the scenarios to say the count starts after the 4 px threshold.
- **Fix:** Decided for the spec: steps now count from the press, so 30 px is ten steps (46 in the scenario). Scrub tests updated.

### 🟡 Fixed - RF7 The viewer README still documents steppers and "Show N unset properties"
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

### 🟡 Fixed - RF8 Several spec scenarios and task items have no test
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

### 🟡 Fixed - RF9 Dead code and stale comments from the removed steppers and unset list
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
- **Fix:** Removed `Live.settle`, the `.more-toggle`, `.more-filter` and `.more-block` CSS, and the `.more-block` key handling; reworded the stepper comments; moved `derivedStep`'s doc comment back onto the function.
