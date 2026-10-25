# Review: update-drawnui-fiddle-to-nx-0-9

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/fiddle-nx-visual-view/spec.md, specs/fiddle-nx-catalog-visualizers/spec.md, specs/viewer/spec.md  
**Reviewed code:** NX `0204ed9e` (vs `main`): `packages/viewer/src/edits.ts`, `packages/viewer/src/index.ts`, `packages/viewer/test/edits.test.ts`, `packages/viewer/README.md`, `sites/playground/src/App.tsx`, `src/vscode/CHANGELOG.md`. Fiddle `5f962dc` (vs `main`): `nx/src/visual.ts`, `nx/src/runtime.ts`, `nx/src/main.ts`, `nx/visualizers.nx`, `nx/test/visual.test.ts`, `nx/test/visualizers.test.ts`, `nx/test/edits.test.ts`, `src/DrawnUi.Fiddle/Pages/FiddlePage.razor`, `wwwroot/fiddle-react.js`, `wwwroot/fiddle-nx.js`, `nx/CATALOG.md`, `README-NX.md`. Compared against the playground host (`sites/playground/src/App.tsx`, `reading/ReadingView.tsx`). Task 7 is intentionally not started and is not treated as incomplete; the path-pinned `nx/rust/Cargo.toml` is expected.

## Findings

### ✅ Verified - RF1 Visual stays read-only (or stays editable) when the editor's read-only state changes while Source shows
- **Severity:** Medium
- **Evidence:** `viewer.editable` is set only in `draw()` (`nx/src/visual.ts:442`) and in the `onDidChangeConfiguration` listener (`nx/src/visual.ts:661-663`). That listener is registered in `show()` and disposed by `stop()` on `hide()`, so a change made while Source shows is never heard. When Visual is shown again, `show()` calls `refresh()`, and with the same text and tree (`nx/src/visual.ts:587`, `viewer.tree === tree && viewer.text === text`) it does not call `draw()`, so `viewer.editable` keeps its old value. Example: a visitor opens a share (read-only, Visual drawn with `editable=false`), switches to Source, presses Edit (`EnterEdit` → `fiddleSetReadOnly(false)`), and switches back to Visual without typing. Visual still edits nothing until the text changes. The `visual.test.ts` case "the reading edits while the editor takes typing…" toggles read-only only while Visual shows, so it does not cover this.
- **Recommendation:** In `show()`, after `mount()`, set `viewer.editable = editable()` (or set it in the `refresh()` early-return branch as well). Add a `visual.test.ts` case that hides, toggles `setReadOnly`, shows again and checks `viewer().editable`.
- **Fix:** In `show()`, once the editor is set, `viewer.editable` is set from the editor's read-only option, so a reading shown again without a redraw follows a change made while Source showed. New `visual.test.ts` case: shown read-only, hidden, made editable, shown again with the text unchanged.
- **Verification:** Verified in fiddle `4c5a207`: `show()` sets `viewer.editable = editable()` after the early return and `stop()`, so a reading shown again without a redraw picks up a read-only change made while Source showed. The new `visual.test.ts` case (read-only → hide → editable → show, text unchanged) covers it and passes.

### ✅ Verified - RF2 Undo on the bar after a field's change is dropped takes back an unrelated earlier step
- **Severity:** Medium
- **Evidence:** If a field in Visual holds an entry it cannot take (for example `abc` or a value outside a `min`/`max` hint), the press on Undo blurs the field and the field drops its change without firing `nx-edit`. `step('undo')` (`nx/src/visual.ts:341-353`) then runs `editor.trigger('nx-visual', 'undo')` against the model's history. That undoes the visitor's previous edit (a Visual edit or Source typing), which they did not ask for. The playground fixed exactly this case (commit `81143cbe`, "Fix review findings RF16-RF18"): `App.tsx` records `draftDroppedAt` on the draft→false transition and `undoPressedAt` on `pointerdown`, then skips the undo when the press itself dropped the draft. The fiddle has no equivalent, so the spec scenario "Undo with a field open" (text unchanged, old value reads) holds only when the draft commits, not when it is dropped.
- **Recommendation:** Copy the playground's guard. Record when `NX_DRAFT_EVENT` turns `changed` from true to false. Have the bar's Undo note its press time (a `pointerdown` handler in `FiddlePage.razor`/`fiddle-react.js`, or a `pressedAt` argument passed to `visual.undo`), and skip the editor undo when a draft was dropped at or after the press. Clear the marker on `nx-edit`. Add a `visual.test.ts` case, and a browser check if practical.
- **Fix:** Ported the playground's guard. `visual.ts` records when a field's draft turns from changed to unchanged (`draftDroppedAt`, cleared on `nx-edit`); `undo(pressedAt?)` skips the editor's undo when a draft was dropped at or after the press. `fiddle-react.js` records the bar's Undo press (`pointerdown` on `[data-fiddle-view-undo]`, its `timeStamp`) and passes it; `fiddle-nx.js` forwards it; documented in `ADDING-A-LANGUAGE.md`. New `visual.test.ts` case, and a `visual-check.mjs` check (FontSize field holding `12px`, Undo on the bar keeps the earlier color edit).
- **Verification:** Verified: `draftDroppedAt` is set on the draft's changed→unchanged transition, and `edit()` clears it on `nx-edit`. The viewer calls `#endSession` (which reports draft false) before `#commitValue`, so a committed entry clears the marker and its Undo still runs. Only a dropped entry is skipped. The capture-phase `pointerdown` in `fiddle-react.js` records the press before the field's blur, and both timestamps share the same time origin (`event.timeStamp` and `performance.now()`). `fiddleEditorViewCommand('undo')` hands it to `view.undo(pressedAt)` through `fiddle-nx.js`. Keyboard Ctrl+Z is unaffected. The unit test passes; the coordinator reports the new browser check passes.

### ✅ Verified - RF3 Show in Source can select the wrong text when the model has moved past the reading
- **Severity:** Low
- **Evidence:** `nx-open`'s `detail.range` is a range in the text the reading was drawn from (`text`). `leave()` applies `opening` (`nx/src/visual.ts:522-534`) without checking `model.getValue() === text`, while the normal path just below it does check. If the visitor commits a field and then quickly chooses Show in Source on another row, the edit lands first and the range now points into shifted text. The playground's `openInSource` places the cursor only when `reading.text === sourceRef.current`.
- **Recommendation:** Map the range only when `model.getValue() === text`. Otherwise either fall back to the key: once the new tree arrives, `rangeOfKey(tree, text, selection)`, since `selection` is already set to `detail.key`. Or just switch views without selecting.
- **Fix:** `leave()` places an opened value's range only while the editor holds the text the reading was drawn from; otherwise it leaves the editor's selection as it is. New `visual.test.ts` case.
- **Verification:** Verified: `leave()` now returns without selecting when `model.getValue() !== text`, and the new unit test passes.

### ✅ Verified - RF4 `min` hints refuse values DrawnUI uses, contrary to the catalog's "a hint never restricts"
- **Severity:** Low
- **Evidence:** The viewer treats `min`/`max` as a validation problem and will not commit a value outside them (`packages/viewer/src/editors.ts:129-145` `rangeProblem`; `packages/viewer/src/index.ts:3246` commits only when `#problemOf` is undefined). Even so, `nx/visualizers.nx:6-11` and `nx/CATALOG.md:120` say "A hint suggests and never restricts: any value the catalog's type takes can still be typed". Some of the module's hints block values DrawnUI uses:
  - `Scale`, `ScaleX`, `ScaleY` `min=0` (`nx/visualizers.nx:30-32`). A negative scale mirrors a control in DrawnUI.
  - `WidthRequest`, `HeightRequest` and the `Minimum*`/`Maximum*Request` properties `min=0` (`nx/visualizers.nx:22-23, 42-45`). DrawnUI's default and "auto" for these is `-1` (`drawnui-react/dist/core/SkiaControl.js:58-64`).
  - `MaxLines` `min=0` (`nx/visualizers.nx:75`). DrawnUI's own default is `-1`; only `> 0` limits.
  - (Design's `Scale` row had no `min`; the implementation added it.)
- **Recommendation:** Remove `min=0` from `Scale*` and keep `step=0.1`. Either drop `min` from the size requests or accept that `-1` must be typed in Source. Fix the comment and the CATALOG.md sentence so they say a range is enforced on entry while choices only suggest.
- **Fix:** Removed the ranges that exclude values DrawnUI uses: `min=0` from `Scale`/`ScaleX`/`ScaleY`, the size requests (`WidthRequest`, `HeightRequest`, `Minimum*`/`Maximum*Request`), `MaxLines` and the spacing properties. `visualizers.nx`'s header comment, `CATALOG.md` and design.md now say a range bounds what Visual's field takes (Source still takes any value) and is given only where DrawnUI has no use for a value outside it, while choices only suggest. `visualizers.test.ts` asserts those four kinds carry no `min`. On the question: refusing entries is the viewer's specified behavior (viewer spec, editor hints), so the fiddle's table adapts rather than the viewer.
- **Verification:** Verified: the `min` hints on `Scale*`, the size requests, `MaxLines` and the spacing properties are gone, and the comment, `CATALOG.md` and design.md now describe ranges as enforced on entry and choices as suggestions. `visualizers.test.ts` asserts no `min` on the four kinds. The answer to the Question is accepted. The spec delta still says `MaxLines` steps "from `0`" (see RF7).

### ✅ Verified - RF5 A failed or aborted source edit surfaces as an unhandled promise rejection
- **Severity:** Low
- **Evidence:** `createEditQueue` returns `next`, which rejects when `sourceEdit` throws. Only the internal `last` chain catches it (`packages/viewer/src/edits.ts:61-66`). The fiddle calls `void edits(detail).finally(...)` (`nx/src/visual.ts:334`) and `void edits.after(...)` (`nx/src/visual.ts:342`) with no `catch`. A host crash in `runtime.sourceEdit` (which rethrows after `replaceHost()`) or any service error therefore logs an "Uncaught (in promise)" error, even though `onUnchanged` already handled it. The viewer README's example (`void queue(...)`) and the playground (`void applyVisualEdit(detail).finally(...)`) have the same pattern. The viewer's own test has to add `.catch(() => undefined)`.
- **Recommendation:** Add `.catch(() => undefined)` in the fiddle (and the playground/README example). Alternatively, make the queue's returned promise resolve after a failure, since the failure is already reported through `onUnchanged`, and document that. A viewer spec change before 0.9.0 is the cleaner fix.
- **Fix:** The fiddle catches the queue's rejection (`void edits(detail).catch(() => undefined).finally(...)`, and `edits.after(...).catch(...)` in `step`), as do the playground (`App.tsx`) and the viewer README's example, with a comment that a failure is already reported through `onUnchanged`.
- **Verification:** Verified: `visual.ts` catches both the edit and `after` promises, and so do the playground's `onEdit` and the README example. The playground's `visualHistory` still calls `void applyVisualEdit.after(...)` without a catch, but that step does not throw in practice, so this is not reopened.

### ✅ Verified - RF6 Hints the proposal and design name are missing from the module
- **Severity:** Low
- **Evidence:** The proposal lists `ScrollOrientation` among the short unions that get a segmented editor, and "a placeholder for `Source` and `Text`". Design decision 5 gives `Source` a placeholder on `SkiaImageBase`, `SkiaSvg` and `SkiaLottie`. In `nx/visualizers.nx`, `SkiaScroll` (`:239`) has no `Orientation` hint (`ScrollOrientation = Horizontal | Vertical | Both | Neither`), `SkiaLottie` (`:223`) has no `Source` placeholder, and no `Text` property has a placeholder. Task 5.3 allowed the table to be adjusted, but these omissions are not recorded in tasks.md or the design.
- **Recommendation:** Either add `<PropertyVisualizer property=Orientation editor=segmented />` to `SkiaScroll` (and `RefreshIndicator`), plus the `SkiaLottie` `Source` placeholder (and `SkiaGif`/`SkiaSprite` if wanted), or record in design.md/tasks.md 5.3 why they were dropped.
- **Fix:** Added `Orientation` `editor=segmented` to `SkiaScroll` and `RefreshIndicator`, and the `Source` placeholder to `SkiaLottie`. No `Text` placeholder: recorded in design.md (decision 5) why, and the proposal's line now matches.
- **Verification:** Verified: `Orientation editor=segmented` is on `SkiaScroll` and `RefreshIndicator`, the `Source` placeholder is on `SkiaLottie`, and design.md and the proposal record why there is no `Text` placeholder.

## New Findings Discovered During 2026-10-24 Verification

### ✅ Verified - RF7 The catalog spec still says `MaxLines` steps from `0`, which no longer matches the module
- **Severity:** Low
- **Evidence:** `specs/fiddle-nx-catalog-visualizers/spec.md:7` requires "whole-number steps from `0` for grid placement and `MaxLines`". After the RF4 fix, `nx/visualizers.nx` gives `MaxLines` only `step=1` with no `min`, and the proposal and design were updated to match, but the spec delta was not.
- **Recommendation:** Reword the requirement to "whole-number steps from `0` for grid placement, and whole-number steps for `MaxLines` and `ZIndex`". You could also add that no range excludes values DrawnUI uses (`-1` sizes, negative scales and spacing).
- **Fix:** The requirement now reads whole-number steps from `0` for grid placement, whole-number steps for `MaxLines` and `ZIndex`, and no range where DrawnUI uses values outside one (`-1` sizes, negative scales and spacing).
- **Verification:** Verified in NX `335096f9`: `specs/fiddle-nx-catalog-visualizers/spec.md` now requires whole-number steps from `0` for grid placement, whole-number steps for `MaxLines` and `ZIndex`, and no range where DrawnUI uses values outside one. This matches `nx/visualizers.nx` (`MaxLines step=1`, `ZIndex step=1`, `Row`/`Column` `min=0`) and design.md. `openspec validate` reports the change valid.

## Findings of the 2026-10-25 review of live edits (task 8)

Reviewed NX `4f2a571..3da76a2` plus the docs commit, and the fiddle `4c5a207..202a39f`.

### ✅ Fixed - RF8 Undo within 500 ms of a closed field's stepper click comes back undone
- **Severity:** High
- **Evidence:** `after()` recorded the preview as a step and the host undid it, but the viewer's settle timer then committed the stepped value, which the queue applied again as a new step.
- **Fix:** The queue remembers the value whose change `after` recorded, and the commit of it that follows changes nothing (`onUnchanged`). New `edits.test.ts` case.

### ✅ Fixed - RF9 Leaving Visual during a gesture leaves the preview outside the history
- **Severity:** Medium
- **Evidence:** Neither host ended the gesture on a switch to Source, and `disconnectedCallback` kept the live timers, so typing or an undo in Source met a preview the history never recorded.
- **Fix:** The fiddle's `hide()` and the playground's switch to Source (and Show in Source) call `after(() => undefined)` to record the gesture as a step; the viewer commits a gesture on disconnect. The design's risk note is corrected. New `editing.test.ts` case.

### ✅ Fixed - RF10 Opening the field of the number being stepped drops the clicks
- **Severity:** Medium
- **Evidence:** `#openEditor` seeded the field with `#currentOf`, the value the gesture began from.
- **Fix:** Opening a value's editor, other than a redrawn reading reopening one, first commits that value's stepping, so the field opens on the stepped value. New `editing.test.ts` case.

### ✅ Fixed - RF11 A preview's edit can split a surrogate pair
- **Severity:** Medium
- **Evidence:** `differenceOf` compared UTF-16 units, so one emoji replaced by another could start the range between the halves of a pair, which Monaco widens, corrupting the text.
- **Fix:** The common head and tail never end inside a pair. New `edits.test.ts` case.

### ✅ Fixed - RF12 A commit shows the old text while it is asked
- **Severity:** Low
- **Evidence:** `committed()` took the preview back before awaiting the language service, so the canvas jumped back and forth.
- **Fix:** The commit is asked against the gesture's start while the preview shows, and the preview is taken back and the answer applied in one turn. New `edits.test.ts` case.

**Verification (BretPC):** viewer 208/208, playground 125/125, `pnpm -r test` green, fiddle `npm run test:nx -- --nx ../nx` 140/140, and `nx/visual-check.mjs` all checks passed against a runtime rebuilt from this NX.

## Questions
- None (the RF4 question was answered: refusing entries is the viewer's specified behavior, and the table now adapts to it).

## Summary
- Verification (fiddle `4c5a207`, NX `4681fd67`): RF1–RF6 verified, with `npm run test:nx -- --nx ../nx` passing 137 tests (1 skipped). One new low-severity finding, RF7, a spec wording mismatch, was fixed in NX `335096f9` and verified. All findings are closed.
- The edit-queue move into `@nx-lang/viewer` is clean and matches the viewer spec. The fiddle's integration follows the playground closely, and its unit and browser checks cover the main paths. The two Medium issues are host-behavior gaps: the editable state is not re-read when Visual is shown again, and the dropped-draft Undo guard the playground already has is missing. The rest are small edge cases and table/doc mismatches.
