# Review: add-viewer-hover

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/viewer/spec.md  
**Reviewed code:** packages/viewer/src/card.ts, src/markdown.ts, src/source.ts, src/index.ts, src/styles.ts,
test/card.test.ts, test/markdown.test.ts, test/hover.test.ts, test/fixtures/sources.json (`hover`),
packages/viewer/README.md, src/vscode/CHANGELOG.md; compared against crates/nx-language-service/src/hover.rs and
lib.rs (the editor's hover) and packages/value-view (its existing hover)  

`pnpm --filter @nx-lang/viewer test` passes (73 tests) and `openspec validate add-viewer-hover --strict` is valid.
RF1 and RF2 were confirmed with throwaway jsdom probes (mock timers) that were deleted afterwards.

## Findings

### ✅ Verified - RF1 Once a card is open, moving to another target never opens that target's card
- **Severity:** High
- **Evidence:** `#point` (packages/viewer/src/index.ts:927) starts `#closeCardSoon()` (300 ms) and an open timer
  (500 ms) for the new target. The grace timer fires first and calls `#closeCard()`, which begins with
  `clearTimeout(this.#cardOpenTimer)` (index.ts:1023-1024), so the new target's pending open is cancelled. A probe
  opening the card on `Type`, then resting on `Circle` for 5 s, left the card hidden. The same happens moving from
  the card back onto a different target (the card's `pointerleave` arms the same close, index.ts:216) and crossing
  a gap between two targets. In practice a reader gets one card, then nothing until the pointer leaves every
  target and comes back.
- **Recommendation:** Keep the grace close from cancelling a pending open, for example by splitting a "hide the
  card" step (used by the grace timer) from `#closeCard()` (used by Escape, scroll, render and pointerdown), or by
  having the open timer clear the close timer and replace the content. Add a test: open on `Type`, move to
  `Circle`, tick 500 ms, and expect `(case) ShapeType.Circle`.
- **Fix:** `#closeCardSoon()` now calls a new `#hideCard()`, which hides the card and clears only the close timer; `#closeCard()` (Escape, scroll, click, a new reading) still cancels a pending open too. New test "opens the next target's card once the pointer rests there, after the first" covers target to target, card to target and across a gap.
- **Verification:** `#closeCardSoon()` now runs `#hideCard()`, which leaves `#cardOpenTimer` alone, and only `#closeCard()` (Escape, scroll, pointerdown, render) cancels a pending open. The new test covers target to target, card to target and across a gap. My original probe scenario (Type, then Circle) now opens `(case) ShapeType.Circle`.

### ✅ Verified - RF2 Two marks of the same card count as different targets
- **Severity:** Medium
- **Evidence:** `#point` compares elements (`target === this.#pointed`, `target === this.#cardAnchor`, index.ts:928,
  933), but one node is marked on several spans: a line's `name` and `=` (source.ts:973, 978), and an element's
  icon and name. Moving from `Type` to `=` restarts the 500 ms delay before a card opens, and once one is open it
  arms the close. Combined with RF1 the card closes and does not come back; with RF1 fixed it would close and
  reopen (flicker) for the same content.
- **Recommendation:** Treat a target with the same `data-hover` key as the open card's as the same target: keep the
  card and clear the close timer, and don't restart the open timer when the key under the pointer has not changed.
  Cover it in hover.test.ts (name → `=` keeps the card open with no gap).
- **Fix:** `#point` compares `data-hover` keys (`#pointedKey`, `#cardKey`) instead of elements, so a node's spans are one target. New test "takes the spans of one node for one target: a line's name and its =".
- **Verification:** `#point` compares `data-hover` keys (`#pointedKey`, `#cardKey`), and `#render` resets `#pointedKey`. Name → `=` keeps the timer and keeps the open card. Tested.

### ✅ Verified - RF3 The artifacts promise the editor's hover, but the signatures differ from it
- **Severity:** Medium
- **Evidence:** design.md:19 ("The card says what the editor's hover says for the same name") and D3 (design.md:47),
  the README's "as the editor's hover does" and CHANGELOG.md:56 all claim parity. The editor's hover spells a
  component as its tag with properties and no base (`component <Shape Type:ShapeType label?:string />`,
  crates/nx-language-service/src/hover.rs `component_signature`), an element function with its parameters
  (`let <Glow color:string />`), a record or union with its fields or cases (`record_signature`, `union_signature`),
  and an attribute qualified by its element and without a default (`(property) Shape.Type: ShapeType`, lib.rs:547).
  The viewer gives `component Shape extends Node`, `let <Glow />`, `type Task` and `(property) Type: ShapeType =
  Rectangle` (card.ts:97-135). The spec pins the viewer's shapes, so the code matches the spec; what doesn't hold
  up is the stated goal.
- **Recommendation:** Decide which one wins. Either restate the goal, D3, the README and the changelog as "in the
  editor's style", with the deliberate differences named (the base instead of the property list, an unqualified
  property with its default), or align card.ts with hover.rs and update the spec scenarios. See Questions.
- **Fix:** Kept the spec's shorter shapes and dropped the parity claim: design.md's goal and D3 (renamed "Signatures in the editor's style, shorter where a reader needs less", with the deliberate differences and why), the proposal, the README and the changelog now say "in the editor's hover's style".
- **Verification:** The goal (design.md:19-20), D3 (renamed, with the deliberate differences and why), the proposal, the README and the changelog now say "in the editor's hover's style". The spec's shapes are kept, as the Questions answer says. Consistent.

### ✅ Verified - RF4 A function declaration's own card leaves out its return type
- **Severity:** Low
- **Evidence:** `declarationSignature` adds `: returns` only when a caller passes it (card.ts:116-119), and only
  `referenceCard` does, so the declaration `twice` reads `let twice(n:int)` (pinned by card.test.ts:41), while a
  reference or call to it reads `let twice(n:int): int`. The editor's hover shows the written return type, and the
  tree carries it as the declaration node's `twice.type` typeReference child. The declaration card is reached
  with `i` on a selected declaration.
- **Recommendation:** For a declaration node, read the return type from its `type` child (when written) and pass
  it in. Update the test to expect `let twice(n:int): int`.
- **Fix:** A declaration node's card reads its return type from its `type` typeReference child and passes it in; card.test.ts expects `let twice(n:int): int`.
- **Verification:** A declaration node's card takes its `type` typeReference child's value. `twice` reads `let twice(n:int): int`, and a probe on `childCounts` gives `let kids(): Item+` and leaves `root` with no return type. Value and alias declarations ignore it, which is correct. card.test.ts updated.

### ✅ Verified - RF5 The pointer closes a card asked for from the keyboard by leaving the card
- **Severity:** Low
- **Evidence:** `#cardByKeyboard` is documented as "which the pointer does not close", and `#point` respects it
  (index.ts:937), but the card's own `pointerleave` calls `#closeCardSoon()` unconditionally (index.ts:216). A
  pointer crossing a keyboard-opened card closes it.
- **Recommendation:** Guard the card's `pointerleave` with `!this.#cardByKeyboard`, and add a test.
- **Fix:** The card's `pointerleave` arms the close only when `!this.#cardByKeyboard`. Covered by the new keyboard test.
- **Verification:** The card's `pointerleave` arms the close only when `!this.#cardByKeyboard`. The new test checks that a keyboard card survives the pointer crossing it and leaving, and closes on ArrowDown.

### ✅ Verified - RF6 `i` on a unit with no card says nothing and leaves an earlier card open
- **Severity:** Low
- **Evidence:** The `i` branch calls `preventDefault()` and `#openCard`, and updates the announcer only when a card
  opens (index.ts:900-913). On a unit the tree says nothing about (an intrinsic `<div />`, a literal picked by a
  click in a function body) a screen-reader user hears nothing, and a pointer card for some other node stays on
  screen as if it answered.
- **Recommendation:** When there is no card, close any open card and announce a short "No information" (or leave
  the key unhandled). Add the case to the keyboard test.
- **Fix:** `i` with no card closes any open card and announces "No information". New test "says there is nothing on i where the tree says nothing, closing any other card"; README and D6 say so.
- **Verification:** With no card, `i` closes any open card and announces "No information". Tested, and documented in the README and D6.

### ✅ Verified - RF7 Closing on scroll relies on a document capture listener that a host's shadow root hides
- **Severity:** Low
- **Evidence:** The card closes from `ownerDocument.addEventListener("scroll", …, true)` (index.ts:164, 267).
  `scroll` is not composed, so if the reading's scroller (`#scrollPort()`) sits inside another component's shadow
  root, its scroll never reaches the document, and the card stays where it was while the content scrolls away
  under it. The test only fires `scroll` at the document (hover.test.ts:258).
- **Recommendation:** Also listen on the scroller that `#scrollPort()` / `#scroller()` finds while a card is open
  (added on open, removed on close), or note the limitation in the README.
- **Fix:** While a card is open, the element also listens for `scroll` on `#scrollPort()`'s element, removed when the card hides. New test with the reading's scroller inside a host's shadow root; D6 notes it.
- **Verification:** While a card is open, `#openCard` listens for `scroll` on `#scrollPort().element`, and `#hideCard` (also run from disconnect) removes it. The test puts the scroller in a host's shadow root. Two levels of shadow roots above the element would still escape, since `#scrollPort` walks `parentElement`, but that limit predates this change and also affects placement. Acceptable.

### ✅ Verified - RF8 Risky timing paths have no tests
- **Severity:** Low
- **Evidence:** hover.test.ts covers open, the grace onto the card, nothing-to-say, Escape, scroll and a new text.
  It doesn't cover moving between targets (RF1, RF2), a new `tree` (only `text`) closing the card, a keyboard
  card surviving the pointer and closing on a selection change (index.ts:819-822), or a host `markdown` that
  throws or returns a non-`Node` falling back to the built-in renderer (index.ts:977-988).
- **Recommendation:** Add those cases with mock timers alongside the RF1, RF2 and RF5 fixes.
- **Fix:** Added tests for moving between targets (RF1), one node's spans (RF2), a new `tree` closing the card, a keyboard card surviving the pointer and closing on a selection change (RF5), `i` with nothing to say (RF6), a scroller in a shadow root (RF7), and a host `markdown` that throws or returns a non-Node. Viewer tests: 80 pass.
- **Verification:** Seven new tests cover each path listed, and `pnpm --filter @nx-lang/viewer test` passes 80/80.

### ✅ Verified - RF9 A second hover card and Markdown renderer join the one value-view already has
- **Severity:** Low
- **Evidence:** packages/value-view already shows a hover card with its own Markdown subset
  (value-view/src/markdown.ts, fenced code and inline code) and a 250 ms delay (value-view/src/index.ts:97), and
  the playground shows both panes side by side. The viewer adds a richer renderer with emphasis, lists and links
  and a 500 ms delay. Design D4 compares only against a third-party library, and the two cards now differ in
  timing and in what a doc renders as.
- **Recommendation:** At least name value-view's renderer in D4 and say why the two stay separate (no dependency
  between the packages). Consider aligning the delays, and later sharing the safer renderer.
- **Fix:** D4 now names value-view's card (code-only renderer, 250 ms delay), says the packages stay apart because neither depends on the other, and leaves one shared renderer and delay for when a shared package exists. The delays stay as the spec pins them (about half a second).
- **Verification:** D4 names value-view's card (code-only renderer, 250 ms), says why the packages stay apart, and defers a shared renderer and delay. Acceptable.

## Questions
- RF3: should the viewer's signatures match the editor's hover exactly (tag with properties, qualified property,
  no default), or keep the shorter shapes the spec pins and drop the parity claim?
  - **Answer (fixer):** keep the spec's shapes, which the user approved with the proposal, and state the
    differences as deliberate; the claim is reworded (see RF3).

## Verification Summary
- All nine findings were verified fixed on 2026-10-17. None were reopened and no new findings came up. Viewer tests pass 80/80 and `openspec validate add-viewer-hover --strict` is valid.

## Summary
- The card content, the Markdown renderer's safety (DOM only, `http:`/`https:` links, raw HTML as text) and the
  marks in the reading are sound and well tested. The spec's scenarios pass, and the docs and changelog are updated.
- One real behavioral bug (RF1): after the first card, hovering another name usually shows nothing. That should be
  fixed before archiving, together with the same-key handling (RF2). RF3 needs a decision about parity with the
  editor. The rest are small robustness issues and missing tests.
