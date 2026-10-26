Paths under `nx/`, `src/`, `dev/` and `docs/` are in `~/src/DrawnUi.FiddleEngine`; paths under
`packages/`, `crates/`, `sites/` and `specs/` are in this repository.

## 1. Branch, baseline and the checkout

- [x] 1.1 Create the fiddle branch `nx-0.9` from `main` and record the baseline on the 0.8.0 pins: `npm run runtime`, `npm run test:nx` (count the tests) and `npm run test:nx:rust` pass; save a share of two NX presets from the local backend for task 7.3
  - Done 2026-10-24: branch `nx-0.9`; `test:nx` 118 pass, 1 skipped (119); `test:nx:rust` 25 pass. On KimmysPC the Rust tests link only with `RUSTFLAGS="-L <dir>"` naming a directory of `libfreetype.so`/`libfontconfig.so` symlinks (no `-dev` packages and no sudo). Two presets compiled by NX 0.8 saved for 7.3 (and played in 1.4).
- [x] 1.2 In this repository run `pnpm install && pnpm -r build` (with the system clang; the cached wasi-sdk only if it fails), then in the fiddle verify `npm run runtime -- --nx ../nx` and `npm run test:nx -- --nx ../nx` pass, with the summary line naming the checkout
- [x] 1.3 Point `nx/rust/Cargo.toml` at the checkout's `nx-ir-runtime` and `nx-value` by path (never merged in that state); verify `npm run test:nx:rust -- --nx ../nx` passes. Install emsdk 6 into `~/src/emsdk` for `npm run runtime:rust` and `nx/rust/browser-check.mjs`, or record in the PR that those two run on BretPC
  - Done: `test:nx:rust -- --nx ../nx` 25 pass. emsdk 6.0.13 installed in `~/src/emsdk`; `runtime:rust -- --nx ../nx` builds (unset `RUSTFLAGS` so `.cargo/config.toml`'s apply). The dev host needs the `wasm-tools` workload, installed into a user-local SDK at `~/.dotnet` (10.0.111).
- [x] 1.4 Confirm the IR schema is still 6 (`crates/nx-ir/src/model.rs`) and that `fiddle-nx.js`'s `CURRENT_SCHEMA` needs no change; verify the shares from task 1.1 play in `/p/` on the checkout build
  - Done: schema still 6; the two NX 0.8 shares play in `/p/` on the checkout build.

## 2. Probe the editing APIs against the catalog

- [x] 2.1 In a fiddle test, ask the runtime's language service `sourceEdit` for `setValue` on a preset's `FontSize`, `addProperty` of `Opacity` on a `SkiaLabel`, `remove` of a `Margin`, and `move` of a stack's child; verify each answer applies to the text and the result compiles. Fix anything refused wrongly in `crates/nx-language-service` with a `source_edit_tests` case first
- [x] 2.2 With a scratch visualizer module ordering `SkiaControl`'s and `SkiaLabelBase`'s properties, read the unset-property order the source tree gives a `SkiaLabel`; if the derived base's order does not lead, or a base's order is lost when a derived control has its own visualizer, fix the merge in `crates/nx-api/src/program_visualizers.rs` with a `visualizer_declarations_tests` case
  - Done: the nearest visualizer's properties lead, then each base's; no NX fix needed. Found instead that `SkiaLayoutBase` is generic and cannot have a visualizer yet (design decision 5).
- [x] 2.3 Verify a `PropertyVisualizer` on a record (`SkiaGradient`) and on an abstract base reaches a concrete control's tree with its hints, and that an `editor=segmented` hint on a `LayoutOptions` property is accepted with no `editor-hint-mismatch`

## 3. The edit queue in `@nx-lang/viewer`

- [x] 3.1 Move `sites/playground/src/reading/edits.ts` to `packages/viewer/src/edits.ts`, export `createEditQueue` and its types from the package against a `{ sourceEdit }` shape the viewer declares, move `edits.test.mjs` into the viewer's tests, and document it in `packages/viewer/README.md` (Editing); verify `pnpm --filter @nx-lang/viewer test`
- [x] 3.2 Import it in the playground (`App.tsx`); verify the playground's tests and type check pass and Visual edits, undo and redo still work in a browser

## 4. Editing in the fiddle's Visual view

- [x] 4.1 Expose `sourceEdit(code, key, edit, signal?)` from `nx/src/runtime.ts` over the existing service with the catalog and implicit imports, and pass it to `createVisualView`
- [x] 4.2 In `nx/src/visual.ts`, set `viewer.editable` from the editor's `readOnly` option (and on `onDidChangeConfiguration`), route `nx-edit` through `createEditQueue` applying with `pushUndoStop`/`executeEdits('nx-visual')`/`pushUndoStop`, ask the tree at once after an applied edit, give the viewer its tree again after an unchanged one, and send Ctrl/Cmd+Z and Y from the reading to the editor through `queue.after`; verify with new `nx/test/visual.test.ts` cases (editable follows read-only, one undo step, edits in order, refused edit restores, undo after an edit in flight, immediate tree)
- [x] 4.3 Add an `openSource(range)` option to `show`; `fiddle-react.js` gives it a callback the page answers by switching to Source and selecting the range; route `nx-open` to it; verify in `visual.test.ts` and in the browser that Show in Source and Enter on an expression open Source with the value selected
- [x] 4.4 Add optional `undo`, `redo` and `onHistory` to the `visual` registration contract (`fiddle-react.js`, `docs/ADDING-A-LANGUAGE.md`), report `canUndo`/`canRedo` from the model's history plus `nx-draft` and edits in flight, and show Undo and Redo on the bar in `FiddlePage.razor` while Visual shows; verify C# and TSX are unchanged and the bar's buttons undo a Visual edit, including with a field open
- [x] 4.5 Verify with the dev host that a Visual edit with HotReload on runs the snippet once, that a read-only share and an unedited old-schema share edit nothing, and that a share made from Visual after an edit stores the edited source

## 5. The catalog's visualizers

- [x] 5.1 Add the hints and the property order of design decision 5 to `nx/visualizers.nx`, with comments in the module's style; verify `npm run test:nx -- --nx ../nx` reports no diagnostics in the module
- [x] 5.2 Extend `nx/test/visualizers.test.ts`: the hinted properties carry their hints in a source tree, `FontFamily`'s choices equal the font names `fiddle-react.js` registers, and a `SkiaLabel`'s unset list starts with `FontSize`, `TextColor`, `FontFamily`; verify each check fails when broken on purpose (a misnamed font, a hint on a `string` property expecting a number, a reordered list)
- [x] 5.3 Read and edit every NX preset in Visual and adjust the table where a range, step or choice is wrong for what the preset does; verify by screenshots of a few edited properties, for the PR
  - Done: every NX preset read in Visual; screenshots of the label's unset list, FontSize and FontFamily choices and the segmented HorizontalOptions.

## 6. Browser checks and docs

- [x] 6.1 Add editing cases to `nx/visual-check.mjs`: a number, a color, a union and a string edited, the canvas changing with HotReload on, Undo from the bar and the keys, an item reordered and deleted, Show in Source, a read-only share editing nothing; verify it passes against `--nx ../nx`, with the existing checks
  - Done: `nx/visual-check.mjs` all passed (with 19 new editing checks) and `nx/rust/browser-check.mjs` all passed, against `--nx ../nx`.
- [x] 6.2 Update `README-NX.md` (Visual edits, what it leaves to Source, read-only shares), `nx/CATALOG.md` (editor hints and order: where they go and how to add one) and `AGENTS.md`; verify `readme.test.ts` compiles every NX block

## 8. Live edits (added after testing)

- [x] 8.1 In `packages/viewer`, fire `nx-edit` with `live: true` while a value changes (field typing after 300 ms, steppers, arrows, scrubbing, the color picker and hex field), end each gesture with a commit or, on cancel, the opening value; commit a closed field's stepper clicks after 500 ms or on the next interaction; commit a color popover closed by a press elsewhere; verify with `editing.test.ts` cases for each
- [x] 8.2 Give `createEditQueue` live edits through an optional `preview`, rebased on the gesture's start, superseded ones skipped, the commit one step, a cancel no step, and an open gesture recorded before another edit or `after`; verify with `edits.test.ts` cases
- [x] 8.3 Implement `preview` in the playground's editor and the fiddle's view (`model.applyEdits`); verify with `visual.test.ts` cases and the playground's tests, and a browser check that a stepper click in an open field changes the canvas and the color picker's live drag is one Undo
- [x] 8.4 Update the viewer README, the changelog's 0.9.0 section, `README-NX.md`; rebuild and restart the local fiddle for testing
- [x] 8.5 Give a property written on a line its row's menu: a click on its name selects it (drawn just around it), its `⋯` hangs past the line's end while selected, and Delete removes it; verify with an `editing.test.ts` case and a browser check in the fiddle
- [x] 8.6 Remove dragging a number's name sideways (Bret: it overloads a name that is clicked to select), leaving the steppers to drag; hang an inline property's `⋯` right after it; verify with the viewer tests and a browser check
- [x] 8.7 Show a selected property's value tools (steppers, chevron) with the pointer elsewhere, without opening its editor; verify with an `editing.test.ts` case and a browser check in Bret's Chrome
- [x] 8.8 Undo and Redo select, scrolled into view, the property or element they changed (`keyOfChange`, `spanOfChanges` in the viewer; the fiddle's view and the playground); keep the steppers inside the reading's visible edge; verify with `selection.test.ts`, `visual.test.ts` cases and a browser check
- [x] 8.9 Show the value tools only for the selected property (and an open editor), never under the pointer; select a value's property when its editor opens; verify with `editing.test.ts` cases and a browser check in Bret's Chrome

## 7. Release NX v0.9.0 and repin

- [x] 7.1 In this repository, write the `0.9.0` changelog section from "Unreleased" (adding `createEditQueue` and any fixes from tasks 2 and 3), and make sure every suite is green: `cargo test --workspace`, `pnpm -r test`, `dotnet test bindings/dotnet/NxLang.sln` after `cargo build --release -p nx-ffi`
  - Done 2026-10-24: `## 0.9.0` in `src/vscode/CHANGELOG.md` (with `createEditQueue`). `pnpm -r test` and `dotnet test` (172) pass; `cargo test --workspace --no-fail-fast` passes 3413 tests, with one timing test (`a_patch_that_supplies_a_large_value_again_does_not_walk_it_again`, a 20 s budget in a debug build) over budget under the parallel run on KimmysPC and passing alone in 10 s.
- [ ] 7.2 Tag `v0.9.0` and review the draft release (NuGet, npm, the three crates, the VSIXs) with Bret; publish only once Bret approves; verify each registry serves 0.9.0
- [ ] 7.3 In the fiddle, pin every `@nx-lang/*` at `0.9.0`, move `nx/rust/Cargo.toml` back to crates.io at `=0.9.0`, `npm install`, and re-emit `nx/catalog/drawnui.nxir` only if `npm run runtime` reports it stale; verify `npm run runtime`, `npm run test:nx`, `npm run test:nx:rust`, the Visual check and the Rust browser check pass without `--nx`, and the shares from task 1.1 still play
- [ ] 7.4 Commit, push `nx-0.9` and open the fiddle PR, with the screenshots and a note for the drawfiddle.com maintainer to redeploy the runtime; verify CI is green
