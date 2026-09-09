## Why

The playground's vendored DrawnUI copy is from `0e4a32d`, taken on 2026-09-02. Upstream has since
moved sixty-six commits to `f617e07` (npm `0.1.0-preview.4`): eleven new controls, eight new demo
pages, substantial additions to seven of the pages already ported, and a startup change that alters
how every unstyled label draws. The gallery's claim that every DrawnUI demo page is ported is no
longer true, the catalog cannot spell the new control set, and a fresh sync would break both the
catalog generator and the type check. This change brings the playground back in step with the demo
site it mirrors.

## What Changes

- **Re-vendor DrawnUI at `f617e07`.** The sync script also copies the three asset trees the new
  demos need (`lottie/`, `anims/`, `shaders/`); `UPSTREAM.md` records the new commit. The vendored
  runtime now loads CanvasKit's "full" build (Skottie for Lottie), roughly 0.9 MB more of WASM.
- **Fix the catalog generator's record rule.** The new `VisualEffects` property is typed as an
  engine class, and the generator's "any class in the vendored tree is a record" rule followed it
  through `Parent` into the whole object graph, emitting `SkiaControl`, `Canvas` and eighteen other
  engine classes as record types. Only DrawnUI's value types are records now; a property typed as an
  engine class is omitted and listed, like a callback. The regenerated catalog gains eleven
  components (`SkiaLottie`, `SkiaGif`, `SkiaSprite`, `SkiaSpriteSet`, `SkiaEditor`, `SkiaBackdrop`,
  `SkiaImageTiles`, `SkiaDecoratedGrid`, `SkiaScrollBar`, `RefreshIndicator`, `SkiaShaderCarousel`),
  the `SkiaBevel` record, a fuller `SkiaGradient`, and the unions those controls use.
- **Keep the renderer and startup in step with the engine.** The renderer learns to construct
  `SkiaBevel`; startup registers the same style defaults as the demo, because upstream now resolves
  an empty `FontFamily` to Skia's built-in face rather than the first registered font, and without
  the style every label in every example would change typeface. The vendored Vite plugin, which
  imports an optional peer the playground does not have, is excluded from the type check.
- **Port the eight new demo pages** — Lottie & GIF, Shell, Editor, Keyboard Input, SkiaScroll,
  Shaders, Sprites, Drag to reorder — each declaring its coverage, so no DrawnUI demo page is
  omitted.
- **Update the seven pages whose originals grew** — Images, Layouts, Shapes, Text, Carousel &
  Drawer, Accessibility and the root menu — and rename Platform Looks to Common Controls, as the
  demo did. Three of these drop from complete to static or reduced, because the originals gained
  interaction or code-driven mechanisms.
- **Extend the coverage vocabulary with `code-behind`** for what several new pages depend on and no
  existing tag names: engine objects built or driven from code — shader effects, custom filters,
  sprite sets, drag logic in cell classes.
- **Refresh the docs.** `CATALOG.md` records the new divergences and omissions; the README's example
  count, vocabulary and sync notes follow.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `drawnui-nx-catalog`: the exclusion requirement gains the rule that a property typed as an engine
  object rather than a value type is omitted from the catalog and recorded, so a sync cannot leak
  engine classes into the catalog as record types.
- `playground`: a new requirement that drawn text uses the demo's font configuration, so a label
  that names no font family draws in the demo's text font rather than Skia's built-in face.

## Impact

- **Vendored tree**: `sites/playground/src/drawnui/**` (37 files changed, eleven added),
  `reference/demo-pages/**`, `public/images/` (six new images), and three new asset trees under
  `public/`.
- **Scripts**: `scripts/sync-drawnui.mjs` (three more copies), `scripts/generate-catalog.mjs` (the
  record rule), `scripts/check-examples.mjs` (the vocabulary).
- **Generated**: `catalog/skia.nx`, `catalog/catalog-meta.json`, `catalog/omitted.json`,
  `catalog/overrides.json`.
- **Client**: `src/drawnui-runtime.ts` (styles), `src/render/values.ts` (the bevel constructor),
  `src/examples/types.ts`, `src/examples/examples.json`, `src/examples/index.ts`, eight new and
  seven rewritten files under `src/examples/nx/`, `tsconfig.json`. Two files the implementation
  found it had to touch: `src/editor/NxEditor.tsx` (Monaco taking focus releases a drawn editor,
  design D9) and `src/gallery/Gallery.tsx` (a preview's canvas is mounted only near the viewport,
  design D10).
- **Docs**: `README.md`, `docs/CATALOG.md`, `docs/FINDINGS.md` for anything the ports uncover.
- **Not touched**: the compile server, the language route, the deployment, and every package the
  site depends on. No NX language change is needed; the dry run of the regenerated catalog
  compiled all twelve existing examples unchanged.
