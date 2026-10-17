## Why

Since v0.7.0, NX has gained a reading view of its source: the `@nx-lang/viewer` package and its
`<nx-viewer>` element. It is shaped by visualizer declarations (`@nx/visualizer`), and the
playground's NX pane shows it as **Visual** beside the **Source** editor. None of this is released
yet, and the DrawnUI fiddle (`~/src/DrawnUi.FiddleEngine`) still pins `@nx-lang/*` and the runtime
crates at `0.7.0`. The fiddle is the public DrawnUI playground for NX, so it should show the reading
too. A reading is only as good as its visualizers, though, and the 35 controls of the fiddle's
DrawnUI catalog have none. Without them, every control reads as a bare `component` icon and its
name, colors are plain strings, and nothing names a label by its text.

The fiddle is also the first host outside this repository to use the viewer and host visualizer
modules, the cheapest place to find what their API gets wrong before more hosts rely on them.

## What Changes

- **Release NX v0.8.0** from this repository. It is the first release with `@nx-lang/viewer`,
  `@nx/visualizer` and visualizer declarations, and with NX IR schema 6. The fiddle is built against
  this checkout (`--nx ../nx`) until the release, then repinned: every `@nx-lang/*` package at
  `0.8.0`, `@nx-lang/viewer` added, and `nx-ir-runtime` and `nx-value` at `=0.8.0` in `nx/rust/`.
- **A Visual and Source switch over the fiddle's editor** while NX is selected, on either engine
  (React or Rust). It works as the playground's does: Visual first and the default, the reading from
  the language service the page already runs, refreshed after typing pauses and marked stale in
  between, colored by the editor's theme, with Lucide icons loaded when Visual is first shown. Collapse
  all and Expand all are offered, folds are kept across switches, and the selection crosses over. C#
  and TSX keep the editor alone. The page stays language-agnostic: a language offers a visual view
  through the `IFiddleLanguage` contract and its registration script, and the page draws the switch
  for any language that does.
- **Visualizers for the DrawnUI catalog**, in a hand-written host visualizer module the fiddle
  ships beside the generated catalog and names when it asks for a source tree. Every concrete
  control gets a Lucide icon. A label, a span, a button and an editor take their `Text` as the title,
  a control's `Tag` reads as its id, and abstract bases carry what their controls share, so it is
  inherited. A snippet's own visualizer declarations apply as well, as in the playground.
- **Two aliases in the generated catalog** that the reading uses and type checking ignores:
  `Color` for the string properties that hold a color, which read with a swatch, and `uitext` for the
  strings shown to people (`Text`, `PlaceholderText` and the like), which read as prose. Hover and
  completion show the alias names.
- **Old shares keep drawing.** The served schema-5 runtime is frozen as `nx/retired/s5/` before the
  upgrade, following `nx/retired/README.md`, so `nx` links shared on NX 0.4 to 0.7 keep playing. The
  editor's old-syntax note names the current NX version from the runtime instead of the stale
  constant `"0.4"`.
- **Docs**: the fiddle's `README-NX.md`, `nx/CATALOG.md` (a visualizer section; the stale "same
  derivation as the playground's catalog" section goes), `AGENTS.md` (the stale `0.4.0` pin) and
  `docs/ADDING-A-LANGUAGE.md` (the visual-view hook).
- **Fixes in this repository** for anything the integration finds in `@nx-lang/viewer`,
  `@nx-lang/sdk-wasm`'s source-tree query or host visualizer modules, made before the release.

Not in this change: editing in the Visual view; a retired DrawnUI-for-Rust player for schema-5
`nx-rust` shares (those are a week old, and the player already reports a schema it cannot read);
descriptions or categories for catalog controls taken from drawnui-react's JSDoc; Geist or any new
font in the fiddle.

## Capabilities

### New Capabilities

- `fiddle-nx-visual-view`: the fiddle's Visual and Source switch for NX: when it shows, how the
  reading is computed, refreshed, colored and iconed, folding, the selection crossing between the
  views, failure, and the language-agnostic contract through which a language offers a visual view.
- `fiddle-nx-catalog-visualizers`: the DrawnUI catalog's visualizers: the host visualizer module and
  how it is shipped and named, what every control shows (icon, title, id), the `Color` and `uitext`
  aliases the generator writes, and the checks that keep the module in step with the catalog and with
  Lucide.

### Modified Capabilities

- `fiddle-nx-language`: the requirement on shares from another NX IR schema changes. A share whose
  schema has a retired runtime draws with it, schema 5 among them, and only a schema with no retired
  runtime is reported.
- `viewer`: an element reads as a line where its properties fit on one line in the pane and as a
  group with a row to each where they do not, measured as it draws and again as the pane resizes,
  whatever the source's line breaks; an element that reads as written inside a group keeps its
  indentation once; and the package gives a host what it needs to carry a selection and a scroll
  position to and from its editor (`keyAtRange`, `rangeOfKey`, `keyAtTop`, `topOf`, `scrollToKey`).
  Found reading the fiddle's presets, whose long tags wrap several attributes to a line, which a rule
  following the source's line breaks read poorly.
- `source-tree`: an element function (`let <Name … />`) carries a visualizer as a component does, so
  a property typed by an alias with a format, such as the Glass preset's `Glow Color:Color`, shows its
  swatch. Found reading that preset, whose `Glow` colors had none.
- `component-syntax`: a concrete external component with no body may declare a prop named `State`,
  as DrawnUI's `SkiaSpriteSet` does. The reservation exists because a bare `State` inside a body
  names the state record, and such a component has no body. Found when the catalog failed to compile
  on the checkout.

## Impact

- **Fiddle repository** (`~/src/DrawnUi.FiddleEngine`, a new branch from `main`):
  - `package.json` pins, `nx/rust/Cargo.toml` and `nx/nx-packages.mjs` / `nx/test/run.mjs` (the
    viewer and value-view in the checkout mapping);
  - `nx/src/` (the source-tree query, the viewer and its glue), the new `nx/visualizers.nx`, the
    generator's two aliases and a regenerated `nx/catalog/`;
  - `dev/build-nx-runtime.mjs` (the icons bundle);
  - `src/DrawnUi.Fiddle/Languages/IFiddleLanguage.cs`, `NxLanguage.cs`, `NxRustLanguage.cs`,
    `Pages/FiddlePage.razor`, `wwwroot/fiddle-react.js` and `wwwroot/fiddle-nx.js`;
  - `nx/retired/s5/`.
- **New dependency**: `lucide` (in the icons bundle only, which is fetched on first use).
- **The private drawfiddle.com host**: nothing beyond what it already loads, since the switch comes
  with the engine. Its maintainer redeploys the runtime.
- **This repository**: the v0.8.0 release and its changelog section, and possible viewer or language
  service fixes. No requirement here changes unless a fix needs one.
