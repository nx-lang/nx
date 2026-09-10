## 1. Sync and toolchain

- [x] 1.1 Add `samples/public/lottie`, `samples/public/anims` and `samples/public/shaders` to the copy list in `scripts/sync-drawnui.mjs`, run `pnpm run sync-drawnui`, and verify `src/drawnui/UPSTREAM.md` records `f617e070e26502fc55b565986bf2547f83aaf224`, `public/lottie`, `public/anims` and `public/shaders/transitions` exist, and `git status` shows the six new images under `public/images`
- [x] 1.2 Add `src/drawnui/vite` to `exclude` in `tsconfig.json` and verify `pnpm run typecheck` passes on the synced tree
- [x] 1.3 Add the demo's `ConfigureStyles` block to `src/drawnui-runtime.ts` (per design D3: `SkiaLabel` and `SkiaButton`, derived types included, `FontFamily: "FontText"`), and verify in the browser that a label with no `FontFamily` — the Shapes example's heading is one — draws in OpenSans rather than Skia's built-in face

## 2. Catalog

- [x] 2.1 Change `mapType` in `scripts/generate-catalog.mjs` so a class-typed property is a record only when the class is declared in `core/Types` (design D1), run `pnpm run generate-catalog`, and verify `catalog/skia.nx` declares exactly the value-type records (`CornerRadius`, `SkiaBevel`, `SkiaGradient`, `SkiaPoint`, `SkiaShadow`, `Thickness`), no `SkiaControl`, `Canvas`, `SkiaEffect` or `__type` record, and `catalog/omitted.json` lists `SkiaControl.VisualEffects`
- [x] 2.2 Verify the regenerated catalog has the eleven new components from the proposal and `SkiaGradient` carries `Angle`, `ColorPositions`, `TileMode`, `Light`, `Opacity`, and that `catalog-meta.json` marks `SkiaBevel` as constructed
- [x] 2.3 Add the `SkiaBevel` constructor to the table in `src/render/values.ts` (design D8) and verify a scratch example with `BevelType=Bevel Bevel=<SkiaBevel Depth=4 />` draws a beveled shape without the "no constructor" error
- [x] 2.4 Run `pnpm run check-examples` against the regenerated catalog and verify all twelve existing examples still compile, evaluate and report their coverage unchanged (the check also gained component-body expansion while implementing 5.8; see FINDINGS F22)
- [x] 2.5 Update `docs/CATALOG.md`: the component, union and record counts from the generator's summary line; the omitted-property list, now including engine-object references as a class beside callbacks; `SkiaBevel` and the fuller `SkiaGradient` in the types table; and verify the counts match the generator output and every omitted property in the doc appears in `catalog/omitted.json`

## 3. Coverage vocabulary

- [x] 3.1 Add `code-behind` to `Capability` and `CAPABILITY_WORDING` in `src/examples/types.ts` and to the set in `scripts/check-examples.mjs`, with the definition from design D6 in the type's doc comment, and verify `check-examples` accepts an example naming it and still rejects a tag outside the vocabulary

## 4. Existing examples

Each task ends with `pnpm run check-examples` passing for that example and a look at it in the
gallery and editor view. Every dropped behavior gets a note at the point it would have appeared,
naming the same capability the metadata names.

- [x] 4.1 Rename Platform Looks to Common Controls in `examples.json`, `looks.nx`'s heading and `root.nx`'s card, and verify the gallery card and the drawn heading both read "Common Controls"
- [x] 4.2 Rewrite `root.nx` per design D7: the subtitle, the nineteen cards from upstream's `catalog.ts` in a `SkiaWrap` at a fixed card width, gradient card titles cycling the six accents, the footer as `TextSpan`s; verify two columns at the page's maximum width and the card list matches upstream's order and text
- [x] 4.3 Extend `shapes.nx` with the context-menu card (inert, noted), the seven gradient cards and the six bevel/emboss cards; set coverage to static with `event-handlers`; verify every gradient type and both bevel types draw as in the demo
- [x] 4.4 Add the stroke, gradient-stroke and drop-shadow card to `text.nx` and verify the four labels draw with outline, gradient outline, shadow, and all three
- [x] 4.5 Extend `images.nx` with the HSL card, the custom-filter card drawn unfiltered with a `code-behind` note, `SkiaImageTiles` at a fixed offset, and the preload card with an inert button; set coverage to reduced with the `demonstrates` text and capabilities from design D7; verify the tiles draw and the HSL effect applies
- [x] 4.6 Extend `layouts.nx` with the ZIndex/FillRatio/Left/Top card, the composite card at rest, and the four templated cards rebuilt over static children including a `SkiaDecoratedGrid` with cells placed by `Column`/`Row`; set coverage to reduced per design D7; verify the decorated grid paints its separator lines and the Z-order card stacks as in the demo
- [x] 4.7 Rewrite `snapping.nx` per design D7: the playground card with four big slides and inert toggles, the peek card, the looped card over twelve static slides, the `DynamicSize` card over three heights, and the drawer with `CornerRadius` top-only (explicit zeros for the bottom corners, since unset corners mirror the first); add `list-virtualization` to its capabilities; verify the looped carousel wraps both ways and the dynamic carousel changes height when swiped
- [x] 4.8 Add the `AccessibilityTextSelectable` card to `accessibility.nx` and verify the paragraph can be selected and copied with the mouse in the output pane

## 5. New examples

Each task registers the example in `src/examples/examples.json` (in the gallery order from design
D7) and `src/examples/index.ts`, and ends with `pnpm run check-examples` passing for it and a look
at it in the gallery and editor view.

- [x] 5.1 Write `animations.nx` (Lottie & GIF) per design D7 and verify both Lotties and both GIFs play on their own, the tinted Lotties differ in color, and the `IsOn=false` Lottie rests on frame 0
- [x] 5.2 Write `scroll.nx` (SkiaScroll) per design D7 and verify the in-flow header scrolls away, the sticky header stays, the behind header moves at half speed under the content, the scroll bars appear and auto-hide, the refresh indicator follows a pull and springs back, and the snap carousel settles on a child
- [x] 5.3 Write `editor.nx` (Editor) per design D7 and verify typing, caret, selection and the password bullets work in each editor, then focus a drawn editor, click into the source pane and type, and verify the keystrokes reach Monaco and not the drawn editor
- [x] 5.4 Write `keyboard.nx` (Keyboard Input) per design D7 and verify the probe card draws at its initial "waiting" state with the cream and blue palette of the original
- [x] 5.5 Write `sprites.nx` (Sprites) per design D7 and verify the three sprite cards animate, the tile board draws with both trees animating and the red warrior mirrored, and the player's absence is noted in the source
- [x] 5.6 Write `shaders.nx` (Shaders) per design D7 with a `SkiaShaderCarousel` over four static `UseCache=Image` slides and `TransitionShader="shaders/transitions/cube.sksl"`; verify a swipe plays the cube transition — or, if static slides do not draw, replace the card with a note per the design's fallback — and verify the effect-host images draw plain with their notes
- [x] 5.7 Write `shell.nx` (Shell) per design D7 and verify the frosted-glass card blurs the baboon behind it as in the demo and the remaining cards draw with inert buttons
- [x] 5.8 Write `reorder.nx` (Drag to reorder) per design D7 over the full language list and verify the rows draw with grip bars, titles, tags and colored strokes, and the list scrolls

## 6. Docs and verification

- [x] 6.1 Update `README.md`: the example count and the sentence listing coverage states, the vocabulary with `code-behind`, the sync section naming the three added asset trees, and a note that the runtime loads CanvasKit's full build for Lottie; verify `grep -n twelve README.md` finds nothing stale
- [x] 6.2 Record in `docs/FINDINGS.md` anything the ports uncovered that is a toolchain or catalog gap rather than a known capability, with a stable number each, and verify each entry says where it was worked around
- [x] 6.3 Run `pnpm test` in `sites/playground` and verify the server, route, proxy and watchdog tests and all twenty examples pass
- [x] 6.4 Run `pnpm run build` and `pnpm start`, open the gallery, and verify every card draws, the font default holds on a label with no family, and the full CanvasKit binary and the Lottie, sprite and shader assets all load from under `/playground/`
