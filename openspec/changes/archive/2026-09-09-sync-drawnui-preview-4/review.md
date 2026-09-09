# Review: sync-drawnui-preview-4

## Scope
**Reviewed artifacts:** proposal.md, design.md (D1–D10), tasks.md, specs/drawnui-nx-catalog/spec.md, specs/playground/spec.md  
**Reviewed code:** the uncommitted working tree under `sites/playground/`: `scripts/generate-catalog.mjs`, `scripts/check-examples.mjs`, `scripts/sync-drawnui.mjs`, `src/drawnui-runtime.ts`, `src/render/values.ts`, `src/examples/types.ts`, `src/examples/index.ts`, `src/examples/examples.json`, `src/gallery/Gallery.tsx`, `src/editor/NxEditor.tsx`, `tsconfig.json`, the eight new and eight rewritten files under `src/examples/nx/`, `catalog/*`, `README.md`, `docs/CATALOG.md`, `docs/FINDINGS.md`, `src/drawnui/UPSTREAM.md`. The vendored tree (`src/drawnui/**`, `reference/demo-pages/**`, `public/{fonts,images,lottie,anims,shaders}`) was diffed against the upstream checkout rather than read.

**Checks run during the review**

| Check | Result |
|---|---|
| `pnpm run typecheck` | passes |
| `pnpm test` (server/route/proxy/watchdog tests + `check-examples`) | 37 tests pass, 20 examples pass |
| `pnpm run generate-catalog` then diff against the committed `catalog/` | byte-identical (restored afterwards) |
| `diff -rq` of every copied tree against `/home/bret/src/DrawnUi.React` at `f617e07` | no differences; the vendored copy is verbatim |
| Example order in `examples.json` vs upstream `catalog.ts` | matches, root last |
| Reorder language list vs upstream `ReorderPage.tsx` | 50 entries in both |
| FINDINGS F22 claim (`+` lowers to `Concat` only when both operands are string-typed) | confirmed at `crates/nx-hir/src/lower.rs:985-990` |
| FINDINGS F24 claim (surface created in the `CanvasView` constructor, before the wrapper assigns `RenderingMode`) | confirmed: `core/Canvas.ts:52-57` calls `OnResized()` in the constructor; `react/index.tsx:139-140` assigns the prop after `new` |
| `grep -n twelve README.md` | nothing stale |

## Findings

### 🟡 Fixed - RF1 A drawn editor's typed text likely falls back to Skia's built-in face after the first keystroke, so the Editor example does not hold the font default the playground spec requires
- **Severity:** Low
- **Evidence:** `SkiaEditor` extends `SkiaShape`, not `SkiaLabel` (`src/drawnui/controls/SkiaEditor.ts:23`), so the `SkiaLabel` style from design D3 never reaches the editor itself. Its inner text and placeholder labels are built in code, so they are styled lazily on first measure (`src/drawnui/core/SkiaControl.ts:295`), which does set `FontText` because their `FontFamily` still equals the class default. But every `UpdateLabel()` call then overwrites both labels with the editor's own `fontFamily`, which defaults to `""` (`SkiaEditor.ts:39`, `:219`, `:223`), and `UpdateLabel()` runs on every `Text`, `FontSize`, `PlaceholderText` and similar change. With upstream now resolving `""` to the built-in face, the placeholder draws in OpenSans and the text typed into it does not. The demo has the same configuration, so this is an engine gap rather than a regression of this change, but `editor.nx` is where a visitor sees it and the `playground` spec's new requirement is stated for "drawn text". Reasoned from the code; not reproduced in a browser during this review.
- **Recommendation:** Reproduce in the browser (type into the first editor and compare the face with the placeholder). If it reproduces, set `FontFamily="FontText"` on each `SkiaEditor` in `editor.nx` with a one-line comment that this is a deviation from the original for the reason above, and record the engine behavior in `docs/FINDINGS.md` beside F23 as another item worth reporting upstream. If it does not reproduce, mark this resolved.
- **Fix:** Reproduced in the browser: the placeholder drew in OpenSans and text typed over it in a monospace built-in face. Every `SkiaEditor` in `editor.nx` (11) now names `FontFamily="FontText"`, with a header comment calling out the deviation; recorded as FINDINGS F25 beside F23. Re-screenshotted after the rebuild: typed text draws in OpenSans.

### 🟡 Fixed - RF2 The new component expansion in `check-examples` has no test, so the F22 class of failure it exists to catch is not pinned
- **Severity:** Low
- **Evidence:** `scripts/check-examples.mjs:29-64` adds `expandComponents`, and FINDINGS F22 explains that the previous check passed a runtime failure inside a component body. Nothing in `src/*.test.mjs` or `server/*.test.mjs` exercises it; the only coverage is that the twenty shipped examples happen to pass. A future change to the IR runtime's descriptor shape (`$type`, `componentEntrypoints`, `initializeComponent(...).rendered`) would silently turn the walk into a no-op and the check would go back to passing bodies it never ran.
- **Recommendation:** Move `expandComponents` into its own module (for example `scripts/expand-components.mjs`) so it can be imported without running the script's top-level loop, and add one test that compiles a small source whose component body does `"a" + Item.Field` on a record field and asserts the walk throws. That is the F22 regression, and the test documents it.
- **Fix:** `expandComponents` moved to `scripts/expand-components.mjs`; `scripts/expand-components.test.mjs` compiles a body doing `"Reorder " + Item.Title`, asserts plain `root` evaluation leaves the descriptor unexpanded, asserts the walk throws `Operator 'add'`, and asserts a valid nested body expands silently. `scripts/*.test.mjs` joined the `pnpm test` glob; the suite is 40 tests.

### 🟡 Fixed - RF3 The gallery blurbs for Images and Layouts no longer match the upstream card text that the other eighteen blurbs and `root.nx` use
- **Severity:** Low
- **Evidence:** `src/examples/examples.json` changes the Images blurb to "SkiaImage — every TransformAspect, effects, custom filters, SkiaImageTiles, alignment, clipping" and the Layouts blurb to add "ZIndex, ImageComposite, DecoratedGrid", while upstream `reference/demo-pages/catalog.ts:9,13` keeps the shorter text and `root.nx` (task 4.2, "the card list matches upstream's order and text") reproduces the shorter text for the same pages. Every other blurb is the upstream text verbatim. The gallery card and the drawn root-menu card for the same page now say different things, and there is no stated rule for which wins.
- **Recommendation:** Pick one rule and apply it: either restore the two blurbs to upstream's text (simplest, consistent with the other eighteen and with `root.nx`), or state in `index.ts`'s doc comment that a blurb describes what the port covers and may extend upstream's text.
- **Fix:** Both blurbs restored to upstream's `catalog.ts` text, so all twenty match upstream and `root.nx`.

### 🟡 Fixed - RF4 `sprites.nx` carries an empty full-size layer as a placeholder for the missing player
- **Severity:** Low
- **Evidence:** `src/examples/nx/sprites.nx:98-100` adds `<SkiaLayer WidthRequest=576 HeightRequest=256 />` inside the board purely to have somewhere to hang the "NX has no code-behind yet" comment. The layer draws nothing, measures and arranges every frame, and every other example places such notes as a comment or a label without an empty control.
- **Recommendation:** Delete the empty layer and keep the comment; the card title and the trailing label already say the player is absent.
- **Fix:** The empty layer is gone; the comment stays at the spot where the original adds the player.

## Questions
- RF1: was the Editor example checked for the typeface of typed text, as opposed to the placeholder, during task 5.3?
  - **Answer:** No. Task 5.3 checked typing, caret, selection, bullets and the Monaco coexistence, and the font pass in 6.4 checked labels only. The review's reasoning was right; see the RF1 fix.

## Summary
- The change does what the proposal and design say. The vendored tree is a verbatim copy of upstream `f617e07`, the committed catalog is exactly what the generator emits, the type check and the full test suite pass, and the example set matches upstream's order and content, including the full 50-language reorder list.
- The generator's record rule (D1), the style defaults (D3), the bevel constructor (D8), the Monaco focus release (D9) and the near-viewport canvas mounting (D10) all read correctly against the engine code they depend on; both FINDINGS claims that could be checked from source (F22, F24) hold.
- Four low-severity findings: a probable engine gap in how `SkiaEditor` handles the font default that the Editor example would show (RF1), no test pinning the new component expansion in `check-examples` (RF2), two gallery blurbs that drifted from upstream's text without a stated rule (RF3), and an empty placeholder layer in the Sprites board (RF4). None blocks archiving.
