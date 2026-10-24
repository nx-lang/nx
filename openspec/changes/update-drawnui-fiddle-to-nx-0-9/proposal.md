## Why

The DrawnUI fiddle (`~/src/DrawnUi.FiddleEngine`) is on NX 0.8.0, whose Visual view only reads. Since
then NX `main` has made Visual an editor: `sourceEdit` and `<nx-viewer>`'s `editable` (values edited
in place, unset properties added, rows reset or removed), editor hints in `@nx/visualizer` (`min`,
`max`, `step`, `choices`, `editor`, `multiline`, `placeholder`), and reordering and deleting items
(PRs #59 to #63). None of it is released, and the fiddle, NX's public DrawnUI playground, still
reads only. Its DrawnUI catalog is also the hardest test these features have had: a `SkiaLabel` has
about 85 optional properties and no defaults, so an unset-properties list in declaration order, with
no ranges or choices, is long and unhelpful. The fiddle is the cheapest place to find what the editing
API gets wrong for a real design system before it is published.

## What Changes

- **Build the fiddle against the NX checkout, then release NX v0.9.0 and repin.** The fiddle works
  with `--nx ../nx` (and the Rust player's runtime crates by path) until everything passes; fixes the
  integration finds land in this repository first; then v0.9.0 is cut by `docs/deployment.md`, and the
  fiddle moves every `@nx-lang/*` package and `nx-ir-runtime`/`nx-value` to `0.9.0` and passes every
  suite without `--nx`. Nothing is published until Bret approves the release draft. NX IR stays at
  schema 6, so no runtime is retired and shares from NX 0.8 keep drawing with the current one.
- **Editing in the fiddle's Visual view.** The reading is `editable` whenever the editor is: each
  `nx-edit` becomes a `sourceEdit` answer applied to the Monaco model as one undoable step, so Ctrl+Z
  and Ctrl+Y work across both views, and the snippet re-runs as it does after typing (with HotReload
  on). A share opened read-only, or an old-schema share not yet edited, reads as before and edits
  nothing. Visual's bar gains Undo and Redo while Visual shows. `nx-open` (Show in Source, Enter on an
  expression) switches to Source with that value selected.
- **The edit queue moves into `@nx-lang/viewer`.** The playground's `createEditQueue` (edits applied in
  order, each against the text the one before left, with `after` for undo) is what any Monaco host
  needs, so it becomes a viewer export before the release, and both hosts use it.
- **The DrawnUI catalog's visualizers learn DrawnUI** (`nx/visualizers.nx`):
  - ranges and steps for the numbers DrawnUI bounds: `Opacity`, the ratio properties
    (`AnchorX`/`AnchorY`, `HorizontalFillRatio`, the gradient's `*Ratio`s) `0` to `1` by `0.05`;
    `Rotation` by `1`; `Scale*` by `0.1`; `Row` and `Column` by `1` from `0`, their spans from `1`,
    `MaxLines` and `ZIndex` by `1`; `FontWeight` `100` to `900` by `100`; no range where DrawnUI
    uses values outside one (`-1` sizes, negative scales and spacing);
  - choices: `FontSize` and `Spacing`/`RowSpacing`/`ColumnSpacing` on a type scale; `FontFamily` the
    fonts both engines register (`FontText`, `FontTextTitle` and the symbol and emoji fallbacks);
  - segmented editors for short unions (`LayoutOptions`, `TextAlignment`, `FontAttributes`,
    `StrokeCap`, `ScrollOrientation`), multiline for a label's and an editor's `Text`, a placeholder
    for an image's, an SVG's and a Lottie's `Source`;
  - the order of each control's properties, so the unset-properties list leads with what people set
    first (size and layout, then color and text, then the rest).
  The visualizer test already fails on any diagnostic, so a hint that does not fit its property's type
  (`editor-hint-mismatch`) or a bad range (`editor-hint-range`) fails it too.
- **Docs**: `README-NX.md` (Visual edits; what it edits and what it leaves to Source), `nx/CATALOG.md`
  (the hints and how to add one), `AGENTS.md` (the pins), and this repository's changelog section.

### Suggested DrawnUI enhancements, not in this change

Each is a later change of its own, for Bret to pick:

1. **Canvas and Visual select each other.** Selecting an element in Visual outlines the control on the
   canvas, and a tap with a modifier on the canvas selects its element in Visual. The renderer already
   keys instances by position; it would need to carry each element's source-tree key to the control.
   This is the largest win for a design-system playground.
2. **DrawnUI's defaults in the reading.** The generator could record each property's DrawnUI default
   from drawnui-react (`FontSize` 12, `Opacity` 1, `HorizontalOptions` Start), so unset rows read
   `12 default` and offer Reset to default. Declaring them as NX defaults would change what a snippet
   passes to DrawnUI, so it likely wants a display-only default in `@nx/visualizer` instead: an NX
   feature first.
3. **Descriptions from drawnui-react's JSDoc** in hover cards and the unset-properties list.
4. **Compact editors for `Thickness`, `CornerRadius` and `SkiaShadow`.** They read and edit as nested
   elements today. A record editor (four linked number fields, or a shorthand such as `16, 12`) would
   be a viewer feature, useful to any design system.
5. **Live preview while a number is scrubbed**, drawing each step without committing it to the undo
   history, using `nx-draft`.
6. **A palette of choices for `Color`**, from a snippet's own `let` colors or a DrawnUI theme.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `fiddle-nx-visual-view`: "Visual is read-only and leaves the run loop alone" becomes "Visual edits
  the snippet through the editor": edits go through the editor's model and history, re-run the
  snippet as typing does, and are off while the editor is read-only. New: Undo and Redo on the bar,
  and a value opened in Source.
- `fiddle-nx-catalog-visualizers`: new requirements for the editor hints and the property order the
  catalog's visualizers give.
- `viewer`: the package exports an edit queue for hosts.

## Impact

- **Fiddle repository** (a new branch from `main`): `package.json` pins and `package-lock.json`,
  `nx/rust/Cargo.toml`, `nx/src/visual.ts`, `nx/visualizers.nx`, `nx/test/visual.test.ts` and
  `visualizers.test.ts`, `nx/visual-check.mjs`, `src/DrawnUi.Fiddle/Pages/FiddlePage.razor` (Undo and
  Redo on the bar), `wwwroot/fiddle-react.js` and `fiddle-nx.js` (the read-only state and the open
  request reaching the view), and the docs. No catalog regeneration is expected (the drawnui-react pin
  stays); the catalog image is re-emitted only if the compiler changed its bytes.
- **This repository**: `createEditQueue` into `packages/viewer` with its tests, the playground
  importing it; fixes the integration finds; the v0.9.0 changelog section and release.
- **Toolchain on KimmysPC**: the Rust player's browser build needs Emscripten 6, which is not on this
  machine; the native `test:nx:rust` suite does not. Installing emsdk or running that check elsewhere
  is part of the work.
- **drawfiddle.com**: its maintainer redeploys the runtime; no new files are served.
