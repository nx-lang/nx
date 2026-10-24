## Context

Paths under `nx/`, `src/`, `dev/` and `docs/` are in `~/src/DrawnUi.FiddleEngine`. Paths under
`packages/`, `crates/`, `sites/` and `specs/` are in this repository.

**The fiddle today** (`main` at `de61b3a`, clean):

- **Pins.** Every `@nx-lang/*` package (`ir-runtime`, `language`, `monaco`, `sdk-wasm`,
  `value-view`, `viewer`) at exactly `0.8.0`; `nx/rust/Cargo.toml` names `nx-ir-runtime` and
  `nx-value` at `=0.8.0`. `drawnui-react` is `0.1.0-preview.17`.
- **Visual.** `nx/src/visual.ts` (`createVisualView`) drives `<nx-viewer>` over the main-thread
  language service's `sourceTree`, with `nx/visualizers.nx` as the host visualizer module. It never
  sets `editable`. The page (`Pages/FiddlePage.razor`) draws the Visual | Source bar for a language
  with `HasVisualView`, and calls `visual.show/hide/collapseAll/expandAll/reset` through
  `fiddle-react.js`.
- **Read-only.** A share opens read-only (`_viewMode`); `fiddleSetReadOnly` sets Monaco's `readOnly`
  and `domReadOnly`. An old-schema share plays its stored build until edited (`_nxOldPlaying`).
- **Run loop.** With HotReload on (`_autoCompile`, NX's `AutoRun` is true), an edit runs the snippet
  once it has no errors.
- **Catalog.** 11 abstract bases, 35 concrete controls, every property optional, no defaults. Records
  `Thickness`, `CornerRadius`, `SkiaShadow`, `SkiaGradient`, `SkiaBevel`, `SkiaPoint`; numbers are
  `float64`; unions such as `LayoutOptions = Start | Center | End | Fill`.

**NX since v0.8.0** (79 commits, PRs #59 to #63, "Unreleased" in `src/vscode/CHANGELOG.md`):

- `sourceEdit` on every language service: `setValue`, `setText`, `remove`, `addProperty`, `move`.
- `<nx-viewer>`: `editable`, `nx-edit`, `nx-open`, `nx-draft`; the unset-properties list; row menus;
  drag, Alt+Up/Down and Delete for list items.
- `@nx/visualizer` editor hints on `PropertyVisualizer` and `ValueVisualizer`.
- `@nx-lang/sdk-wasm`'s module ABI version 10. No `crates/nx-ir*` or `crates/nx-value` change, so NX
  IR stays at schema 6.
- The playground's host code: `sites/playground/src/reading/edits.ts` (`createEditQueue`),
  `ReadingView.tsx` (undo and redo keys left to the host), `App.tsx` (an immediate tree after an
  edit, a revision bump after an edit that changed nothing, Undo offered for a draft or an edit in
  flight).

## Goals / Non-Goals

**Goals:**

- Visual in the fiddle edits as the playground's does, through the fiddle's Monaco model, with one
  undo history for both views and the canvas following edits as it follows typing.
- DrawnUI's properties edit with ranges, steps and choices that fit DrawnUI, and the long
  unset-properties lists lead with what people set.
- The fiddle and NX end on a published v0.9.0, with no checkout paths in the fiddle's manifests.

**Non-Goals:**

- The six enhancements listed in the proposal as later changes.
- A catalog regeneration or a drawnui-react bump.
- Changing what a share stores, or the `/p/` player.
- Editing in Visual while the editor is read-only.

## Decisions

### 1. Checkout first, then release v0.9.0, then repin

As in `add-visual-view-to-drawnui-fiddle`: a fiddle branch works with `--nx ../nx`
(`pnpm install && pnpm -r build` here first) and the Rust player's crates by path, never merged in
that state. Fixes go into this repository with tests before the tag. v0.9.0 follows
`docs/deployment.md`; Bret approves the draft release before anything is published. The fiddle then
pins `0.9.0` everywhere and every suite passes without `--nx`.

The schema stays 6, so `CURRENT_SCHEMA` and `OLD_SCHEMAS` in `fiddle-nx.js` are unchanged and nothing
goes into `nx/retired/`. A share saved on `main` (NX 0.8) is played in task 1 and again after the
repin to confirm it.

*Alternative:* release from `main` now and integrate after. Rejected: the edit queue export (decision
3) and whatever the catalog shows about hints and ordering belong in the release.

### 2. Visual edits through the Monaco model

`createVisualView` takes a second dependency next to `sourceTree`: `sourceEdit(code, key, edit)`,
the runtime's call into the same language service (`nx/src/runtime.ts`), with the catalog and
implicit imports. While shown:

- `viewer.editable` is true unless the editor is read-only (`editor.getOption(readOnly)`, rechecked
  on `onDidChangeConfiguration`). `_viewMode` and an unedited old-schema share therefore read only.
- `nx-edit` goes to the edit queue (decision 3), which applies the answer with `pushUndoStop`,
  `executeEdits('nx-visual', …)`, `pushUndoStop`. The new text's tree is asked for at once, not after
  the typing pause; an edit that changes nothing gives the viewer its tree again, so the muted value
  returns.
- The model's change is what the page already listens to, so diagnostics, HotReload, the dirty state
  of a share and an old-schema share's switch to compiling follow without page changes.
- Ctrl/Cmd+Z and Y in the reading, outside a field, call the editor's `undo`/`redo` after the queue's
  pending edits (`queue.after`), then refresh, as the playground does.
- `nx-open` calls a new `openSource(range)` option the page passes to `show`: the page switches to
  Source and the view's `hide` selects that range, cursor at its start.

*Alternative:* keep Visual read-only. Rejected: editing is the point of the release, the fiddle is
where people try DrawnUI's properties, and read-only shares stay read-only.

### 3. `createEditQueue` becomes a viewer export

`sites/playground/src/reading/edits.ts` moves to `packages/viewer/src/edits.ts`, exported as
`createEditQueue`, typed against `Pick<NxLanguageService, "sourceEdit">` from
`@nx-lang/language-protocol`, which the viewer already depends on, with its tests (`edits.test.mjs`
becomes `test/edits.test.ts`, over the wasm SDK's snapshot rather than the playground's worker
session). The playground imports it. This mirrors how the selection helpers moved in 0.8.0.

### 4. Undo and Redo on the bar

The page's bar shows Undo and Redo beside Collapse all and Expand all while Visual shows, enabled by
a `canUndo`/`canRedo` the view reports (the model's history, plus a draft or an edit in flight, as the
playground counts it). The page stays language-agnostic: `visual` gains optional `undo()`, `redo()`
and an `onHistory(callback)`; a language without them shows no buttons. In Source, Monaco's own keys
already undo.

### 5. The catalog's hints and order

All in `nx/visualizers.nx`, on the base that declares each property, so they are inherited:

| Base | Hints |
| --- | --- |
| `SkiaControl` | `Opacity`, `AnchorX`, `AnchorY`, `HorizontalFillRatio`, `VerticalFillRatio` `min=0 max=1 step=0.05`; `Rotation step=1`; `Scale`, `ScaleX`, `ScaleY` `step=0.1`; `Row`, `Column`, `RowSpan`, `ColumnSpan`, `ZIndex` `min=0 step=1` (spans `min=1`); `HorizontalOptions`, `VerticalOptions` `editor=segmented`; `WidthRequest`, `HeightRequest` `min=0` |
| `SkiaLayoutBase` | `Spacing`, `RowSpacing`, `ColumnSpacing` `min=0` with choices 0, 4, 8, 12, 16, 24, 32 |
| `SkiaLabelBase` | `FontSize` `min=1` with choices 10, 12, 14, 16, 18, 20, 24, 32, 48; `FontWeight` `min=100 max=900 step=100`; `MaxLines` `min=0 step=1`; `FontFamily` choices of the registered fonts; `Text` `multiline`; `FontAttributes`, `VerticalTextAlignment` `editor=segmented` |
| `SkiaImageBase`, `SkiaSvg`, `SkiaLottie` | `Source` `placeholder="URL or resource"` |
| `SkiaShapeBase` | `StrokeWidth` `min=0`; `StrokeCap` `editor=segmented` |
| `SkiaEditor` | `Text` `multiline` |
| `SkiaGradient` (a record) | `StartXRatio`, `StartYRatio`, `EndXRatio`, `EndYRatio`, `Opacity` `min=0 max=1 step=0.05` |

The font choices are the names both engines register: `fiddle-react.js` (with the names
drawnui-react's `AddSymbols` and `AddEmojis` give) and `nx/rust/src/player.rs`, which also registers
`FontTextBold` and `FontGame`; a font one engine lacks would draw in another face there. A test reads
both and fails when the choices differ from what they share. A record such as `SkiaGradient` takes an `ElementVisualizer`
with the same `PropertyVisualizer` content as a component.

**Generic bases.** `SkiaLayoutBase` declares the type parameter `TItem`, and NX does not yet allow a
visualizer on a generic declaration (`visualizer-target-not-allowed`). Its hints and order therefore
sit on each layout container instead (`SkiaLayout`, `SkiaStack`, `SkiaRow`, `SkiaWrap`: `Type`,
`Spacing`, `Padding`; `SkiaGrid`, `SkiaDecoratedGrid`: `RowSpacing`, `ColumnSpacing`, `Padding`).
Allowing visualizers on generic components is an NX change for a later release.

**Order.** Each base lists its properties in the order people set them, so the unset list leads with
them. `SkiaControl`: `WidthRequest`, `HeightRequest`, `HorizontalOptions`, `VerticalOptions`,
`Margin`, `BackgroundColor`, `Opacity`, then the rest. `SkiaLayoutBase`: `Type`, `Spacing`,
`Padding`, then the rest. `SkiaLabelBase`: `Text`, `FontSize`, `TextColor`, `FontFamily`,
`FontAttributes`, `HorizontalTextAlignment`, then the rest. Whether a derived control's list puts its
own properties before or after its bases' is NX's merge rule; task 2 found the nearest visualizer's
properties lead, then each base's in turn, so a label reads `Text`, `FontSize`, `TextColor`,
`FontFamily` first and then `SkiaControl`'s.

The values above are a starting table. Task 5 reads every preset in Visual and adjusts.

### 6. Tests

- `nx/test/visual.test.ts` (jsdom, fake editor and service): `editable` follows read-only, an edit is
  one undo step, edits apply in order, a refused edit restores the value, undo after an edit in
  flight, `nx-open` reaching `openSource`, the immediate tree after an edit.
- `nx/test/visualizers.test.ts`: the existing no-diagnostics check now covers hint mismatches;
  add checks that the hinted properties carry their hints in a source tree, the font choices match
  `fiddle-react.js`, and the unset list of `SkiaLabel` starts with `Text`, `FontSize`, `TextColor`.
- `nx/visual-check.mjs` (headless Chromium): edit a number, a color, a union and a string in Visual,
  see the canvas change with HotReload on, undo from the bar and the keys, a read-only share that
  edits nothing, reorder and delete an item, Show in Source.

## Risks / Trade-offs

- **Long unset lists.** A `SkiaLabel` lists about 80 rows. The order and the viewer's filter field
  (over 12 rows) make it usable; descriptions and defaults (proposal enhancements 2 and 3) would make
  it good.
- **Edits re-run the snippet.** With HotReload on, each committed edit runs the snippet, as typing
  does. Scrubbing commits once on release, so a drag is one run.
- **Emscripten on KimmysPC.** `npm run runtime:rust` and `nx/rust/browser-check.mjs` need emsdk 6,
  which is not installed here (clang is, so the NX wasm build no longer needs the cached wasi-sdk).
  Task 1 installs emsdk into `~/src/emsdk`, or records that those two checks run on BretPC.
- **The viewer's API is unstable.** `@nx/visualizer` and editing are marked unstable; a later
  release can change the hints, and the visualizer test is what catches it.
