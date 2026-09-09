## Context

See proposal.md — Why. What shapes the approach is what a dry run of the sync and the catalog
generator against `f617e07` found (run, inspected, and reverted while writing this change):

- **The catalog generator leaks engine classes.** The generator treats any class declared under the
  vendored tree as a record type. Until now the only class-typed properties on the authorable
  surface were the value types in DrawnUI's `Types` module. `f617e07` adds `VisualEffects:
  SkiaEffect[]` to `SkiaControl`; `SkiaEffect` has a `Parent: SkiaControl`, and from there the
  generator walked the entire engine — `SkiaControl`, `Canvas`, `SKRect`, `AccessibilityNode`,
  `CachedObject`, two anonymous `__type`s — emitting twenty-three records instead of five, private
  fields included, and a `SkiaControl` record beside the `SkiaControl` component.
- **The type check fails** on the vendored `src/drawnui/vite/index.ts`, upstream's build-time
  crawler plugin, which imports `playwright-core` — an optional peer the playground does not
  install and would never run.
- **Assets the sync does not copy.** The new pages read `lottie/*.json`, `anims/**/*.png` and
  `shaders/**/*.sksl`, none of which is in the sync's copy list. Six new photos under `images/`
  did arrive.
- **The default typeface changed.** Upstream now resolves an empty `FontFamily` to Skia's built-in
  face, as .NET does, and the demo compensates with `ConfigureStyles` defaulting every label and
  button to `FontText`. The playground's startup mirrors the demo's bootstrap but predates that
  line, so after a sync every label without a `FontFamily` — most of them — would change face.
- **Everything else held.** All twelve existing examples compiled, evaluated and passed
  `check-examples` against the leaked catalog, so the language side needs nothing; the reconciler's
  new `ApplyInitialStyles` and `Dispose` calls are internal; the runtime's switch to CanvasKit's
  full build is a `?url` import Vite already handles.

The constraints that carry over from the site's earlier changes still apply: examples are NX
compiled through the site's own pipeline, never native TSX; authored interaction is unsupported
(no event properties in the catalog); every non-complete example names its gap from the shared
vocabulary; and the vendored tree is copied verbatim, with `docs/CATALOG.md` the record of any
deliberate divergence.

## Goals / Non-Goals

**Goals:**

- A sync that a future maintainer can re-run and get a working playground from, not one that
  needs the repairs made here to be rediscovered.
- A catalog whose records are exactly DrawnUI's value types, so an engine class added upstream is
  reported as an omission rather than emitted as authorable.
- Every DrawnUI demo page at `f617e07` present in the gallery with an honest coverage state, and
  the existing ports brought up to their originals.
- Text drawn as the demo draws it.

**Non-Goals:**

- Authored interaction, animation from NX, or list virtualization. The new pages are ported under
  the same rules as the old ones: what the engine does on its own works, what needs authored code
  is drawn inert and named.
- Exposing engine objects (effects, filters, sprite sets, cell classes) to NX through the catalog.
  That is the `code-behind` gap this change names, not one it closes.
- Upstream's `SkiaShell` (a React component, not a catalog control), its browser-history routing,
  the crawler plugin, and the context-menu toast.
- Changes to the gallery or editor UI, the server, or the deployment.

## Decisions

### D1. Record types are DrawnUI's value types, nothing else

The generator maps a class-typed property to a record only when the class is declared in DrawnUI's
`core/Types` module — where `Thickness`, `CornerRadius`, `SkiaShadow`, `SkiaPoint`, `SkiaGradient`
and now `SkiaBevel` live. Any other class-typed property returns no mapping, so it is omitted and
listed in `omitted.json` exactly as a callback is.

*Alternatives.* Extending the name-based exclusion list (the one that already hides `Parent`,
`DrawingRect` and the like) with `VisualEffects` would fix this sync and leak again on the next
property upstream adds. Stopping the walk at `SkiaControl` would still emit `SkiaEffect` as a
record with fields no author can use. Tying records to the module DrawnUI itself uses for value
types is the rule that matches what a record means in the catalog.

### D2. The Vite plugin is excluded from the type check, not from the sync

`tsconfig.json` gains `src/drawnui/vite` in `exclude`. The sync keeps copying upstream's `src`
whole, so the vendored tree remains a faithful copy and `CATALOG.md`'s "no edits to the vendored
source" stays true.

*Alternatives.* Pruning the directory in the sync script hides upstream's shape and adds a special
case to a script whose point is to record what was copied. Installing `playwright-core` types adds
a dependency for code that never runs.

### D3. Startup registers the demo's style defaults

`drawnui-runtime.ts` adds the demo's `ConfigureStyles` block: `SkiaLabel` and `SkiaButton`, derived
types included, default `FontFamily` to `FontText`. Buttons need their own entry because a button
pushes its `FontFamily` onto its caption, so the label style never reaches it.

*Alternatives.* Setting `FontFamily` in every example restates a default in a hundred places and
diverges from the originals the examples are ported from. Reverting the vendored `Super.ts` is an
edit to the vendored tree that the next sync erases.

### D4. The sync copies the three new asset trees whole

`samples/public/lottie`, `samples/public/anims` and `samples/public/shaders` join the copy list,
each landing under `public/` by the same name. The `shaders/transitions` set is about sixty small
SkSL files of which the demo uses fourteen; copying the tree is simpler than a hand-picked list
that drifts, and the files are text.

### D5. The full CanvasKit build is accepted as-is

The vendored runtime imports `canvaskit-wasm/bin/full/canvaskit.js` for Skottie. Nothing in the
playground names the binary — the server caches build assets by their hashed name — so the only
effect is a larger download. The README notes the size and why.

### D6. The vocabulary gains `code-behind`

Several new pages are blocked by something none of the four tags names: an engine object built or
driven from code — a `SkiaShaderEffect` in `VisualEffects`, a CanvasKit filter in
`PaintColorFilter`, a `SkiaSpriteSet` subclass, a cell class carrying drag logic. Folding these
into `event-handlers` or `component-state` would misreport what is missing. `code-behind` is the
term DrawnUI's own docs use for it. The visitor wording becomes "NX has no code-behind yet",
which reads as the others do.

Used by: Images, Layouts, Shaders, Sprites, Drag to reorder.

### D7. Per-page porting plan

Coverage follows the existing definitions: *complete* when nothing the original does is missing;
*static* when the drawing is correct and only motion or interaction is absent; *reduced* when the
original's mechanism cannot be expressed and the example is scaled down. Anything the engine does
on its own — Lottie and GIF playback, sprite animation, scrolling, snapping, the drawn editor's
typing, the accessibility overlay — works in the port and does not count against it.

| Example | Coverage | Capabilities | Port | Dropped, with a note in place |
|---|---|---|---|---|
| Lottie & GIF (`animations`) | static | event-handlers, component-state | both Lotties and both GIFs with their props (`Repeat`, `SpeedRatio`, `ColorTint`, `Colors`, `AutoPlay`, `DefaultFrame`, `IsOn`); they play on their own | Start/Stop/Seek buttons, status readouts |
| Shell (`shell`) | reduced — *page navigation, popups, modals, toasts and a tabbed shell driven from code* | event-handlers, component-state | the status lines as text, the page/popup/modal/toast cards with inert buttons, and the `SkiaBackdrop` frosted-glass card in full | the nested tabbed shell (`SkiaShell` is a React component, not a control) |
| Editor (`editor`) | static | event-handlers, component-state | every `SkiaEditor` with its props; typing, caret, selection and the platform looks are the engine's | readouts, the buttons, the chat list |
| Keyboard Input (`keyboard`) | static | event-handlers, component-state | the probe card at its initial state | key events, modifier state, history |
| SkiaScroll (`scroll`) | static | event-handlers, component-state | every card: `Tag="Header"`/`"Footer"` children, sticky, behind with parallax, scroll bars, the refresh indicator, snap and track | `RefreshCommand` (the indicator follows the pull and springs back; without a command the scroll never enters refreshing), index readouts |
| Shaders (`shaders`) | reduced — *SkSL effects attached through VisualEffects, with uniforms, time and touch driven from code* | code-behind, animation, event-handlers | a `SkiaShaderCarousel` over four static photo slides (each `UseCache=Image`, as the control requires) with one transition shader, and the effect hosts drawn as plain images | every `VisualEffects` entry, the transition buttons |
| Sprites (`sprites`) | reduced — *a SkiaSpriteSet warrior moved across the board with the keyboard* | code-behind, event-handlers, component-state | the three `SkiaSprite` cards, the tile board with its trees and the idle red warrior | the player (a `SkiaSpriteSet` subclass built in code), movement, the buttons |
| Drag to reorder (`reorder`) | reduced — *reordering a templated list in place by dragging a row, the row lifted into a ghost that floats over the list* | code-behind, event-handlers, component-state, list-virtualization | the status lines and the whole language list as static rows (grip, title, tag, colored stroke) | the drag, the ghost, the buttons |
| Images | complete → reduced — *custom CanvasKit filters built in code, an animated tile offset and the image preload queue* | code-behind, animation, event-handlers | the HSL effect card, `SkiaImageTiles` at a fixed offset, the preload card with an inert button | `PaintColorFilter`/`PaintImageFilter` (hosts drawn unfiltered), the offset animation |
| Layouts | complete → reduced — *templated Wrap, Row, Grid and DecoratedGrid with Split, and an ImageComposite layer re-recording one spinning child* | list-virtualization, code-behind, animation, event-handlers | the ZIndex/FillRatio/Left/Top card in full; the composite card as its 24 shapes plus the spinner at rest; the templated cards as static children (a `SkiaWrap` of chips, a `SkiaRow` of five, a `SkiaDecoratedGrid` of cells placed by `Column`/`Row`, a `SkiaGrid` of chips) | `ItemsSource`/`Split`/`Invert`, the spin, the outline overlay, the buttons |
| Shapes | complete → static | event-handlers | the gradient cards (`FillGradient` with `Angle`, `ColorPositions`, `Light`, `Sweep`; `StrokeGradient`; label `FillGradient` and `GradientByLines`), the bevel/emboss cards | the context-menu card's handler (the shape is drawn) |
| Text | static | event-handlers | the stroke, gradient-stroke and drop-shadow card | — |
| Carousel & Drawer (`snapping`) | static | event-handlers, component-state, list-virtualization | the playground card with its four big slides and inert toggles, the peek card, the looped card over twelve static slides, the `DynamicSize` card over three slides of different heights, the drawer with top-only corners | toggles, buttons, readouts, templated cells |
| Accessibility | static | event-handlers, component-state | the `AccessibilityTextSelectable` card; selection works, it is the overlay's | — |
| Common Controls (`looks`) | static | event-handlers, component-state | rename only | — |
| Root menu (`root`) | static | event-handlers, component-state | the subtitle, cards in a `SkiaWrap` at a fixed width that gives two columns at the page's maximum width, gradient card titles, the footer as `TextSpan`s | navigation, the window-width card sizing |

Gallery order and names follow upstream's `catalog.ts`, which is now the demo's single source for
both: cells, uneven cells, images, SVG, shapes, text, layouts, common controls, carousel & drawer,
Lottie & GIF, shell, editor, keyboard, SkiaScroll, shaders, sprites, transforms, drag to reorder,
accessibility, and the root menu last as before. Transforms, Cells, Uneven cells and SVG are
unchanged upstream and untouched here.

### D8. The renderer constructs `SkiaBevel`

`SkiaBevel` is a class in `core/Types`, so the catalog marks it constructed and the renderer
throws on an unknown constructor by design. A `SkiaBevel: (fields) => new SkiaBevel(fields)`
entry joins the constructor table. `SkiaGradient` remains an interface, so it stays a plain object
with the new fields passing through.

### D9. Monaco taking focus releases the drawn editor

The risk below about `SkiaEditor` and Monaco materialized: the engine's input proxy releases a
drawn editor only to an `INPUT`, a `TEXTAREA` or a contenteditable, and Monaco 0.56 edits through
an EditContext host `div`, so characters typed into the source pane landed in the drawn editor.
`NxEditor.tsx` subscribes to Monaco's focus event and unfocuses `SkiaEditor.Focused`. The reverse
direction needs nothing: the proxy's hidden textarea takes DOM focus from Monaco by itself.

*Alternatives.* Dropping the Editor example hides an engine limitation the gallery exists to
report. Editing the proxy's check in the vendored tree is erased by the next sync. FINDINGS F23
records it for upstream.

### D10. The gallery mounts a preview's canvas only while its card is near the viewport

`RenderingMode="Default"` on the gallery's previews never took effect — the engine creates its
surface in the constructor, before the React wrapper assigns the prop — so every preview has
always been a WebGL canvas. Twelve fit under Chromium's limit of about sixteen live contexts;
twenty did not, and the oldest cards drew blank. `Gallery.tsx` observes each card and mounts its
`Canvas` only while the card is within 300 pt of the viewport; unmounting disposes the view, which
releases its context. The prop stays as written, since it documents the intent.

*Alternatives.* Fixing the assignment order in the vendored React wrapper is a one-line edit the
next sync erases, and the site would still have twenty software surfaces drawing at once. FINDINGS
F24 records it for upstream.

## Risks / Trade-offs

- [The style default is forgotten on a future sync and text silently changes face] → D3 puts the
  block beside the font registrations it belongs with, the playground spec now requires the
  behavior, and the visual pass in the tasks checks a label with no font family.
- [Only SVG remains a complete example] → That is the honest reading: the originals grew
  interaction and code-driven mechanisms. The coverage states exist so this reads as a report on
  NX rather than as broken examples.
- [A focused `SkiaEditor` captures keys at window level; Monaco in the source pane also needs
  them] → It did: Monaco edits through an EditContext host, which the engine's proxy does not
  recognize as a text field. Resolved by D9 and recorded as FINDINGS F23.
- [`SkiaShaderCarousel` may not accept static children] → It reads `Children[index]` when not
  templated, like its base, and only insists on `UseCache=Image` per slide. If it does not draw,
  the Shaders example keeps its reduced state and the carousel card becomes a note; no other
  example depends on it.
- [The full CanvasKit build adds ~0.9 MB to first load] → Cached at the edge like the previous
  binary; accepted for Lottie.
- [Upstream's `Reorder` and `Sprites` pages are mostly code; their ports are thin] → They are
  reduced, say what the original demonstrates, and name `code-behind`; a thin honest example is
  what the gallery promises, and omitting a page would need the same words in the README anyway.

## Open Questions

- ~~Whether `SkiaShaderCarousel` draws static slides.~~ It does: four static `UseCache=Image`
  slides draw, and a swipe plays the cube transition.
