# Review: add-visual-value-editing

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/source-edits/spec.md, specs/viewer/spec.md,
specs/playground/spec.md, specs/language-protocol/spec.md  
**Reviewed code:** `git diff origin/main...HEAD`: crates/nx-language-service/src/source_edit.rs and
source_edit_tests.rs; bindings/wasm (native lib.rs, abi.ts, api.ts, host.ts, language-service test);
bindings/node (native lib.rs, index.ts, native.ts); packages/language-protocol, language-core,
language-http, language-client (src and tests); packages/viewer/src/edit.ts, editors.ts, index.ts,
source.ts, styles.ts and test/editing.test.ts; sites/playground/src/App.tsx, editor/NxEditor.tsx,
editor/positions.ts, language/worker.ts, worker/session.ts, reading/ReadingView.tsx, reading/edits.ts,
reading/edits.test.mjs, reading/useSourceTree.ts; READMEs and src/vscode/CHANGELOG.md.  
**Checks run:** viewer tests (104 pass), playground tests (104 pass), `cargo test -p nx-language-service
source_edit` (23 pass), `openspec validate --strict` (valid). Edge cases were probed through the wasm
SDK's `sourceEdit` and `sourceTree` (scratch script, not committed).

## Findings

### ✅ Verified - RF1 `setValue` spells a negative number (and a case) without regard to its place, so it can break the document or make the value uneditable
- **Severity:** Medium (confidence: high, reproduced)
- **Evidence:** `spell_value` (crates/nx-language-service/src/source_edit.rs:222) returns the same text
  wherever the target is, but only `rhs_expression` (an unbraced attribute value or a `let` value)
  accepts `signed_numeric_literal`. `value_list_item_expression` does not (crates/nx-syntax/grammar.js:540-553).
  Results probed through the wasm SDK:
  - item of a sequence: `let xs:int+ = { 2 3 5 }`, `xs.value[1]` set to `-4`, answers edits with no
    refusal, and the result `{ 2 -4 5 }` is a parse error ("Expected ? here", "expects int+, found object+");
  - braced value: `p={8}` set to `-3` gives `p={-3}`, which parses as a prefix operator, so the key now
    names an `operator` node and the value is no longer editable in Visual after the next tree;
  - a case written into a braced place: `let x:string = {"a"}` set to case `large` gives `{large}`,
    which is "Undefined identifier 'large'". Bare case names only resolve unbraced.
  The viewer makes the first two easy to hit: ArrowDown and scrubbing step below zero freely
  (packages/viewer/src/editors.ts:87). The spec allows "signed or not" and says the edit touches nothing
  beyond the target, but leaves out that the answer must still parse. The round-trip test
  (source_edit_tests.rs:358) only ever writes `42` and compares keys, never parse errors, so it misses
  this. The wasm parity corpus (`let xs = { 1 2 }` with `-2.5`) produces the broken text too, but never
  applies it.
- **Recommendation:** Make the spelling depend on the place. For a signed number in a sequence item or
  a braced value, either refuse with `invalidValue` (simplest), or let the grammar take
  `signed_numeric_literal` as a list item. Refuse a `case` value at a braced place that is not already
  a qualified case. In `round_trips_over_the_repository`, also write a negative number, and assert
  that the edited document has no more parse errors than the original. Add a sequence-item scenario
  with a negative number to the spec.
- **Fix:** `set_value` refuses a signed number, and a case, as `invalidValue` unless the target is an
  unbraced attribute or `let` value (a qualified case still keeps its qualifier). Tests:
  `a_negative_number_only_where_the_grammar_takes_one`, `a_bare_case_only_where_it_resolves`;
  `round_trips_over_the_repository` now also writes `-42` and asserts no new parse errors. The viewer's
  `EditTarget.signed` is false in a sequence or inside braces: the field marks a negative entry
  invalid, and the arrows and scrubbing stop at zero there (test "takes a sign only where the grammar
  takes one"). Spec: a sentence in "A value is set in the spelling its place takes" and the scenario
  "A negative item of a sequence".
- **Verification:** Verified. `set_value` (source_edit.rs) now refuses a signed number, and a case, as `invalidValue` outside an unbraced attribute or `let` value. A qualified case is still handled first and keeps its qualifier. The new tests `a_negative_number_only_where_the_grammar_takes_one` and `a_bare_case_only_where_it_resolves` cover the three cases I reproduced (`{ 2 -4 5 }`, `{-3}`, `{large}`). The round-trip test now also writes `-42` and asserts there are no new parse errors. On the viewer side, `EditTarget.signed` (edit.ts) stops the arrows and scrubbing at zero and marks a negative entry invalid in a sequence or inside braces. The spec has the sentence and the scenario. `cargo test -p nx-language-service source_edit`: 28 pass.

### ✅ Verified - RF2 The viewer decodes every string escape as its bare character, so changing part of a string with `\n` or `\t` rewrites those escapes as letters
- **Severity:** Medium (confidence: high)
- **Evidence:** `decodeString` (packages/viewer/src/edit.ts:46-49) replaces every `\X` with `X`, so
  `"a\nb"` opens as `anb`. If the reader changes anything in the field, `#finishField`
  (index.ts:1318) commits the whole field content, and `quote_string` (source_edit.rs:245) writes it
  back as `"…anb…"`, silently dropping the escape. NX authors do write `\n` in strings: several .nx
  files under docs/ say "`\n` requests explicit line breaks". The literal is kept as written in the
  tree (`value: "\"a\\nb\""`), so the viewer has what it needs to avoid this. The test only covers `\"`
  (editing.test.ts:104, 199).
- **Recommendation:** Decode only the escapes `quote_string` writes (`\"` and `\\`). For a string holding
  any other escape, offer no text editor (`nx-open` instead) until NX settles what string escapes mean
  (the HIR keeps them as written today). Add a test with `\n`.
- **Fix:** `decodeString` reads only `\"` and `\\`, and answers nothing for a string holding any
  other escape, so such a string gets no text editor; Enter or a double-click opens it in Source
  (`nx-open`). Test "offers no text field for a string with an escape other than a quote or a
  backslash" (`title="a\nb"`).
- **Verification:** Verified. `decodeString` decodes only `\"` and `\\`, and returns `undefined` for a string holding any other escape, so `targetOf` offers no editor and Enter or a double-click falls through to `nx-open`. This matches what `quote_string` writes back. It is covered by the viewer test with `title="a\nb"`.

### ✅ Verified - RF3 A tree that arrives while a field is open reopens the field with all its text selected, so the reader's next keystroke replaces their draft
- **Severity:** Medium (confidence: high on the mechanism, medium on how often it happens)
- **Evidence:** `#render` keeps only the open field's text (index.ts:637) and reopens it through
  `#openEditor` → `#openField`, which always ends with `#selectAll(field)` (index.ts:704, 1211). The
  caret position and any selection the reader made are lost. The common flow hits this: Tab commits
  value A and opens B (index.ts `#onFieldKey`). The reader starts typing in B, A's tree arrives tens of
  milliseconds later, B is rebuilt with "what was typed so far" selected, and the next key replaces it.
  The playground makes this worse: ReadingView's effect sets `text`, `icons`, `colors` and `tree`
  (ReadingView.tsx:147-152), and each setter calls `#render` (index.ts:430, 444, 458), so one update
  rebuilds the field and moves the focus up to four times. An IME composition in progress is also cut
  off. The test "keeps an editor open across a newer tree" (editing.test.ts:240) checks only the text,
  not the caret.
- **Recommendation:** When reopening, put back the caret and selection offsets, and select all only on
  a fresh open. Better still, keep the field element across a render when its key is still editable.
  Add a test that a reopened field keeps a collapsed caret at the end of what was typed.
- **Fix:** `#render` keeps the open editor in `#reopen` (key, draft, caret and selection offsets), which
  survives a render with no tree, as the playground clears the tree before it sets the next. A field
  opened again puts its caret and selection back; only a fresh open selects all (popover inputs too).
  Test "keeps the caret of an editor open across a newer tree, and through a tree cleared first". The
  playground's other setters run while the tree is cleared, so the reading is rebuilt, and the field
  reopened, once per tree.
- **Verification:** Verified, with a caveat on the fallback (see RF9). `#render` saves the open editor's key, draft and caret in `#reopen`, which survives a render with no tree. Fields and popover inputs reopened this way put the caret back, and only a fresh open selects all. The playground's effect now clears the tree first, so the field is rebuilt once per tree, not once per setter. The new viewer test checks for a collapsed caret at offset 4 after a cleared-then-set tree. Viewer suite: 110 pass.

### ✅ Verified - RF4 In the playground, a refused or empty edit leaves Visual showing the reader's value, muted, indefinitely; the viewer's run check also disagrees with the service
- **Severity:** Medium (confidence: high, reproduced)
- **Evidence:** The viewer drops a value it is showing only when the host sets a tree
  (index.ts `set tree`). The viewer spec's "The host refuses" scenario assumes the host sets the tree
  again. The playground's queue returns without doing anything when the answer is refused, has no
  edits, or arrives after the text changed (sites/playground/src/reading/edits.ts:38). No new text means
  no new tree, so the muted value stays, though the source never changed. The viewer's pre-check makes
  refusals reachable, because `runProblem` (edit.ts:56) does not follow the run token's grammar
  (`_mixed_text_run`, grammar.js:767):
  - the viewer accepts `fo`, `e`, `i`, `el` and `els`, which the service refuses as `invalidValue`
    (probed);
  - the viewer rejects `a > b`, `a } b` and `if-then`, which the service accepts. The spec's own
    "Fish & chips > all" cannot be typed in Visual.
  No test covers a refusal reaching the reading in the playground.
- **Recommendation:** When the queue gets a refusal, an empty answer or a dropped answer, the playground
  should give the viewer the current tree again (for example a callback from `createEditQueue`), or the
  viewer should offer a way to drop a pending value. Make `runProblem` match the grammar: refuse
  `<` and `{` only, and refuse the `if`/`for`/`else` prefixes as the token regex does, including their
  short prefixes. Otherwise leave that check to the service and show its refusal. Add a playground test
  that a refused edit gives the reading the tree back.
- **Fix:** `createEditQueue` takes `onUnchanged`, called for a refused answer, an empty one, one for
  text that changed meanwhile, or a failure. The playground raises a `revision` prop on `ReadingView`,
  which sets the tree again, so the muted value goes. `runProblem` now follows `_mixed_text_run`: it
  refuses `<` and `{`, a leading space, and `if`/`for`/`else` or the start of one as a word (`fo`,
  `e`), and allows `>` and `}`. Tests in edits.test.mjs (refusals and no-change edits call
  `onUnchanged`; a stale answer too) and the viewer's run test.
- **Verification:** Verified. `createEditQueue` calls `onUnchanged` when the answer is refused or empty, when it arrives after the text changed, and when the request fails. The playground raises `revision`, which makes ReadingView set the tree again, so the muted value goes. `runProblem` now uses the `_mixed_text_run` token regex, anchored: it accepts `>` and `}`, and refuses `fo`, `e`, `el` and `els`, matching what I probed from the service. edits.test.mjs: 4 pass. One minor caveat, not reopened: setting the tree again also drops the muted values of later edits still in the queue until their own tree arrives, which is a brief flicker in a rare path.

### ✅ Verified - RF5 A text run's trailing whitespace is part of the field, so replacing a run before a braced value or an element merges words
- **Severity:** Medium (confidence: high, reproduced)
- **Evidence:** In `<L>Hello {1} world</L>`, the text node `text[0]` is `"Hello "`, trailing space
  included (range 11..17), while `text[2]` is `"world"` with no leading space. The run field opens with
  `"Hello "` selected (edit.ts:145, index.ts:1211). If the reader types `Hi`, the service writes
  `<L>Hi{1} world</L>` (probed), which reads "Hi1 world". The same happens before a child element
  (`Hello <b>x</b>`), which is common in mixed content. The tests only cover a run that is the whole
  body (`<Label>Hello</Label>`).
- **Recommendation:** Edit the run without its trailing (and leading) whitespace: the viewer shows the
  trimmed text, and `set_text` keeps the run's edge whitespace (replace only the trimmed span).
  Alternatively, have the source tree's text node leave out the trailing separator. Add a
  mixed-content test in Rust and in the viewer.
- **Fix:** `set_text` keeps the run's trailing whitespace and replaces only the words before it (the
  read-back check runs on the new words plus that whitespace). The viewer's run target is the value
  trimmed at the end. Tests: Rust `a_run_keeps_the_space_before_what_follows` (`Hello {1} world` →
  `Hi {1} world`) and viewer "edits a run of text without the space that separates it from what
  follows". Spec scenario "A run before a braced value".
- **Verification:** Verified. `set_text` keeps the run's trailing whitespace: it replaces only the trimmed words, after checking that the new text plus that whitespace reads back as one run. The no-change check compares against the same span. The viewer's run target is trimmed at the end. Covered by `a_run_keeps_the_space_before_what_follows` (`<L>Hi {1} world</L>`), the viewer test and the new spec scenario.

### ✅ Verified - RF6 A re-render during a number scrub drops the drag and leaves the reading with `scrubbing` set
- **Severity:** Medium (confidence: high on the mechanism)
- **Evidence:** `#render` sets `this.#scrub = undefined` (index.ts:639) but does not remove the
  `scrubbing` class that `#onPointerMove` put on the persistent body (index.ts:1680). Only
  `#onPointerUp` removes it (index.ts:1701), and that handler now returns early because `#scrub` is
  undefined. After that, the whole reading has `cursor: ew-resize !important; user-select: none`
  (styles.ts) until another scrub finishes. The drag in progress is silently dropped, with no commit
  and no visual reset. Scrubbing twice in quick succession is enough: the first release's tree arrives
  during the second drag.
- **Recommendation:** In `#render`, remove `scrubbing` and either carry the scrub over to the new name
  element (same key) or end it cleanly. Add a test that sets a tree mid-drag.
- **Fix:** `#render` removes `scrubbing` and carries the drag on through the new tree on the same key
  (`#resumeScrub`: new name element, pointer captured again, the dragged value shown); when the number
  is gone, the drag ends cleanly. A render with no tree keeps the drag for the next one. Tests
  "carries a drag on through a newer tree" and "ends a drag cleanly when the number is gone from the
  newer tree".
- **Verification:** Verified. `#render` no longer drops the drag on a render without a tree. With a tree it removes `scrubbing` and either resumes the drag through `#resumeScrub` (new name element, pointer captured again, value shown) or ends it cleanly when the number is gone. Both paths have viewer tests.

### ✅ Verified - RF7 `remove` leaves a trailing line comment behind, and the comment cases that task 1.4 says were verified have no tests
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** In `remove` (source_edit.rs:350-386), only blanks count as separating space, so:
  - an attribute alone on its line with a trailing comment, `  a=1 // note`, becomes `  // note`, an
    orphaned comment line;
  - `<T a=1 // note\n  b=2 />` becomes `<T // note\n  b=2 />`.
  A comment line directly above an attribute that stands alone also stays. Task 1.4 is checked as
  "verify … with comments beside it", but source_edit_tests.rs has no case with a comment.
- **Recommendation:** Treat a trailing `//` comment on the same line as part of the attribute's line
  when deciding "alone on its line", and remove it with the attribute. Decide and document what
  happens to a comment line above it. Add the tests task 1.4 names.
- **Fix:** `remove` takes a `//` comment that follows the attribute on its line with it, so
  `  a=1 // note` goes as a whole line and `<T a=1 // note\n  b=2 />` becomes `<T\n  b=2 />`. A
  comment on the line above stays (it may be about more than the attribute); documented on `remove` and
  in the spec. Tests `removing_takes_a_trailing_comment_with_it` and
  `removing_keeps_a_comment_line_above`.
- **Verification:** Verified. A `//` comment after the attribute on its line now goes with it: a line holding only the attribute and its comment is removed whole, and `<T a=1 // note` becomes `<T`. Keeping a comment line above the attribute is now documented and tested. The spec has the scenario.

### ✅ Verified - RF8 The contract for applying several `TextEdit`s is inconsistent between the service, its tests and the host
- **Severity:** Low (confidence: high; latent while every answer has at most one edit)
- **Evidence:**
  - The Rust and protocol docs give each range "in the document as the request carried it"
    (source_edit.rs:66, language-protocol `TextEdit`), and the Rust test helper applies edits in
    reverse against the original (source_edit_tests.rs:47-55).
  - The protocol's `SourceEditAnswer` doc and the spec say "to apply one after another".
  - `NxEditor.applyEdits` (NxEditor.tsx:129-135) and edits.test.mjs apply each edit against the text
    the previous one left.
  These agree only for one edit. Later phases (adding properties, inserting elements) will return
  several.
- **Recommendation:** Choose LSP semantics: every range refers to the document as sent, and edits do not
  overlap. State that in the spec and the protocol docs, and apply the edits with a single
  `executeEdits` call over all of them (Monaco applies such a batch correctly).
- **Fix:** LSP semantics throughout: every range is in the document as sent and no two overlap, so a
  host applies them together. Updated the Rust doc, the protocol's `TextEdit` and `SourceEditAnswer`
  docs, its README, the spec and `EditedDocument.apply`. `NxEditor.applyEdits` makes one
  `executeEdits` call with every edit; edits.test.mjs applies an answer's edits against the text as
  it was.
- **Verification:** Verified. The Rust and protocol docs, the README and the spec now say LSP semantics: every range is in the document as sent, and no two overlap. `NxEditor.applyEdits` makes one `executeEdits` call with all the edits inside a single pair of undo stops, which Monaco applies against the original model. The edits.test.mjs helper and the Rust `apply` both apply against the original text.

## New Findings Discovered During 2026-10-21 22:10 Verification

### ✅ Verified - RF9 An editor reopened with no caret it can read still selects everything, as a fresh open does
- **Severity:** Low (confidence: high on the code path; medium on which browsers reach it)
- **Evidence:** `#openField`, `#openColor` and `#openCases` take `caret === undefined` to mean a fresh
  open, and `#openField` then calls `#selectAll`. But `#caretOf` also returns `undefined` on a reopen
  when the DOM selection cannot be read inside the field. That happens when the root has no
  `ShadowRoot.getSelection` and `document.getSelection()` retargets the range to the shadow host, as
  WebKit does, so `field.contains(range.startContainer)` is false. On those browsers RF3 is still
  there: a tree arriving after Tab selects the reader's draft, and the next key replaces it.
- **Recommendation:** Tell a reopen apart from a fresh open: pass a flag, or a draft without a caret.
  When the caret is unknown on a reopen, collapse it at the end of the draft instead of selecting all.
  Where available, consider `Selection.getComposedRanges({ shadowRoots: [root] })` to read the caret
  in WebKit.
- **Fix:** A reopen whose caret cannot be read collapses the caret at the end of the draft
  (`#render` passes `{ start: end, end }`), so a reopened editor never selects the draft. `#caretOf`
  reads `Selection.getComposedRanges({ shadowRoots: [root] })` first where the browser has it, then
  the selection's own range. Test "puts the caret after the draft when where it was cannot be read".
- **Verification:** Verified (commit 0d6018a). A reopen whose caret can't be read now gets a collapsed caret at the end of the draft (`reopen.caret ?? { start: end, end }`), so a reopened field or popover input never selects what was typed. Only a fresh open (with no caret passed) selects all. `#caretOf` reads `getComposedRanges` first, then the selection's own range. The new test "puts the caret after the draft when where it was cannot be read" covers the fallback. Viewer suite: 111/111 pass. See RF10 for one hardening point on the `getComposedRanges` call.

## New Findings Discovered During 2026-10-21 22:20 Verification

### ✅ Verified - RF10 `#caretOf` calls `getComposedRanges` with the dictionary argument and no guard, so an older engine that throws on it would break the reading's render
- **Severity:** Low (confidence: medium; depends on the engine version)
- **Evidence:** `#caretOf` (packages/viewer/src/index.ts) calls
  `selection.getComposedRanges?.({ shadowRoots: [...] })`. `#render` calls it with no try/catch,
  whenever an editor is open. The first engines to ship `getComposedRanges`, WebKit's (Safari 17 and
  later), took the shadow roots as separate arguments (`getComposedRanges(...shadowRoots)`) before the
  spec moved to the dictionary. An engine with that signature rejects a plain object with a
  `TypeError`, and the optional call `?.` does not catch it. `#render` would then stop after clearing
  the body, leaving an empty reading the first time a tree arrives with a field open. That is the
  WebKit case RF9 set out to fix. The fallback added for RF9 never runs, because the exception comes
  first.
- **Recommendation:** Wrap the `getComposedRanges` call, or all of `#caretOf`, in try/catch and treat a
  throw as "caret unknown", which RF9's fallback already handles. Alternatively, detect the signature
  (try the dictionary, then the variadic form).
- **Fix:** The `getComposedRanges` call is in a try/catch; a throw reads as no composed range, and the
  selection's own range is read, or the caret goes after the draft. The test "puts the caret after the
  draft when where it was cannot be read" now has `getComposedRanges` throw.
- **Verification:** Verified (commit ccb1715). The `getComposedRanges` call is now inside a try/catch, and a throw counts as no composed range. `#caretOf` then reads the selection's own range or returns nothing, and `#render` puts the caret after the draft as RF9's fallback does. The render can no longer abort. The RF9 test now makes `getComposedRanges` throw a `TypeError` and still checks that the caret is collapsed after the draft. Viewer suite: 111/111 pass.

## New Findings Discovered During 2026-10-21 23:44 Review

Scope of this pass: commit f90ad06 only (click to edit, `offsetAtPoint`, popover placement, the Reset/Clear button and
Backspace removed, hover cues and the field's input look, the playground's Undo/Redo buttons and narrow-screen
icons, and the spec, design and tasks section 6). Checks:
- viewer tests 112/112, playground `pnpm test` 122/122, `openspec validate --strict` valid;
- in Chromium against the dev server (`/play/survey`, 1200 px and 375 px wide):
  - a click puts a collapsed caret at the right character;
  - the popover is placed by its value and the reading does not scroll;
  - the Undo and Redo buttons enable and disable correctly;
  - switching examples resets the history;
  - a double-click toggles a boolean only once;
  - a click on a name only selects;
  - the title bar fits at 320 px.

### 🟡 Fixed - RF11 The title bar's Undo, pressed while a Visual field is open, undoes the step before and throws away what was typed in the field
- **Severity:** Medium (confidence: high, reproduced in Chromium)
- **Evidence:** I set the title (edit A, committed), then clicked the description, typed `BB` and clicked the
  title bar's Undo:
  1. The button's pointerdown blurs the field, which commits B: `nx-edit` fires and the queue asks the
     worker.
  2. The button's click then runs `visualHistory("undo")` (App.tsx) at once, which undoes A.
  3. B's answer arrives for text that has since changed, so `createEditQueue` drops it (edits.ts,
     `document.text() !== text`).

  The source ends with neither A nor B. Redo brings back A, but B is lost. Redo has the same race. The
  spec scenario "Undo from the title bar" covers only an edit already made.
- **Recommendation:** Make the buttons act on what the reader sees:
  - with a Visual editor open, Undo should cancel the field (the reader's typing is the latest change)
    rather than undo a step;
  - or Undo should wait until the commit's edit has been applied (for example, go through the same
    queue, `createEditQueue`) before undoing.

  `preventDefault` on the buttons' pointerdown keeps the field from blurring first. Add a playground test
  for an undo with a field open, or with an edit still queued.
- **Fix:** `createEditQueue` gains `after(step)`, which runs a step once every edit fired before it
  has been made or refused. The playground's undo and redo (title bar and shortcuts) go through it,
  so an Undo pressed with a field open takes back the field's edit, which the press committed, and
  the edit before it stays. Test in edits.test.mjs: a step waits for the edit fired before it. Spec
  sentence in the playground's requirement.

### 🟡 Fixed - RF12 With Backspace and the row button gone, Reset and Clear are out of reach on Mac laptops and touch screens
- **Severity:** Medium (confidence: high on the key mapping; how much it matters depends on who uses Visual)
- **Evidence:** Removal now listens only for `event.key === "Delete"` (index.ts `#onKey`). On a Mac laptop the
  key labelled "delete" reports `Backspace`; a forward `Delete` needs Fn+Delete, which few readers know. Phones
  and tablets have neither key, and the "Reset to default" and "Clear" button was their only way.
  The viewer spec still requires that "A property with a default, or an optional one, can be left out"
  (specs/viewer/spec.md). The playground is mostly read on Macs and phones, and there this has become a
  keyboard-only feature with no way to discover it. The design's reason for dropping Backspace, that "a
  reader who meant to change the text would press it", applies only while a field is open, and then the
  field already handles its own keys.
- **Recommendation:**
  - Accept Cmd+Backspace on macOS, the Finder and Mail convention for delete, or Backspace with a row
    (not a field) selected.
  - Keep one pointer path to it: a small menu or a context-menu item on the row, which phase 2 needs
    anyway.
  - Or say in the spec and README that removal is keyboard-only for now, so the gap is a deliberate
    choice.
- **Fix:** Cmd+Backspace removes as Delete does (a Mac laptop's delete key). Touch has no removal for
  now: the spec and design say removal is from the keyboard until a later change gives rows a menu
  (phase 2 needs one for adding properties). Test: the removal test presses Cmd+Backspace.

### 🟡 Fixed - RF13 A click on a case (or a color) whose popover is open opens it again instead of closing it
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:**
  1. The document pointerdown handler `#onDocumentPointerDown` cancels the popover, because the value
     is outside it.
  2. The click that follows reaches `#onClick`, finds no open session and opens the popover again.

  So the popover cannot be closed by clicking its value. The new chevron and pointer cursor make the
  value look like a dropdown, where a second click on the trigger closes it.
- **Recommendation:** In `#onDocumentPointerDown`, when the press is on the session's own `span`,
  remember that, and let the following click close the popover rather than reopen it. Add a viewer test.
- **Fix:** A press on the value whose popover is open closes it and marks that key, and the click that
  follows leaves it closed, as a dropdown does. Test "closes a case's popover on a click on its value,
  as a dropdown does"; spec sentence added.

### 🟡 Fixed - RF14 `offsetAtPoint`, which places the caret where a click falls, has no test that exercises it
- **Severity:** Low (confidence: high)
- **Evidence:** jsdom gives ranges no layout, so `offsetAtPoint` (editors.ts) returns the end of the text in every
  viewer test. The test "opens a value on a click, with a caret and nothing selected…" only checks that the
  caret is collapsed. The spec scenario "A click edits where it falls", with the caret between the
  characters the click fell between, is checked only by hand. It works in Chromium. The function picks the
  nearest line first, then the nearest character, which deserves a test, especially for wrapped UI text.
- **Recommendation:** Unit-test `offsetAtPoint` with a stubbed `Range.prototype.getBoundingClientRect`
  that lays out characters on one line and on two wrapped lines. Check a point before the first
  character, between two characters, and after the end of a line.
- **Fix:** A test of `offsetAtPoint` with laid-out characters (Range boxes stubbed: 8 px a
  character, 4 a line) checks before/after a character's middle, a second line's start and past a
  line's end.

### 🟡 Fixed - RF15 `Removal`'s "reset" and "clear" have nothing left to tell apart
- **Severity:** Low (confidence: high)
- **Evidence:** The button that showed "Reset to default" or "Clear" is gone. `removalOf` (edit.ts) still answers
  `"reset" | "clear"`, but its one caller, `#removeSelected`, checks only whether the answer is undefined.
  The same holds for the viewer test "knows which properties may be left out".
- **Recommendation:** Make it `mayLeaveOut(index, at): boolean`, or keep the two cases only if the planned
  row menu will label them, and say so in a comment.
- **Fix:** `removalOf` is now `isRemovable`, a boolean; the `Removal` type is gone.

## Questions
- Should Enter and F2 toggle a boolean when it is the first editable value of the selected unit? They do
  now, because `#openEditor` toggles. Spec and design say Enter "opens" the value, and Tab skips booleans.
  - **Answer:** Kept by design: a boolean's editor is its toggle, so opening it toggles, as Space does.
- Task 5.2 (full `cargo test`, `pnpm -r build/test`) is still unchecked. Is it waiting on the
  background workspace run?
  - **Answer:** It was. The run passed apart from the wasm `recursion-stack` test, which fails in cloud
    containers on main too; 5.2 is now checked, and the suites were run again after these fixes.

## Summary
- The language-service query, its exposure through every transport, and the viewer's editors are
  well structured and well tested on the main paths. All suites run here pass.
- The main risks are at the edges of spelling and of the viewer's live state:
  - a negative number in a sequence item breaks the parse (RF1);
  - string escapes are lost on edit (RF2);
  - a field reopened on a new tree loses the reader's caret (RF3);
  - refused edits stay shown in the playground (RF4);
  - a text run's trailing separator is eaten (RF5);
  - an interrupted scrub leaves the reading's cursor and text selection stuck (RF6).
- RF7 and RF8 are smaller correctness and contract issues worth settling before later phases build on
  `sourceEdit`.
