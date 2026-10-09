Paths under `nx/`, `src/`, `dev/` and `docs/` are in `~/src/DrawnUi.FiddleEngine`; paths under
`crates/` and `specs/` are in this repository.

## 1. Branch, crate and build

- [x] 1.1 Create the fiddle branch `nx-rust` from `origin/main` and confirm the baseline: `npm run test:nx -- --nx ../nx` passes (99 tests)
  - Done: branch `nx-rust` (local, uncommitted); the baseline was 99 passing.
- [x] 1.2 Add the player crate at `nx/rust/` (manifest, `rust-toolchain.toml` at 1.98.1, the Emscripten link flags DrawnUI for Rust documents, `drawnui` `=0.1.0-preview.6` from crates.io, `nx-ir-runtime` and `nx-value` by path) and verify `cargo test` builds an empty test natively and `cargo build --target wasm32-unknown-emscripten --release` links
  - Done: builds natively and links for `wasm32-unknown-emscripten`. On this machine the native link needs `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-L native=$HOME/.local/share/drawnui-linklibs"` (no freetype/fontconfig dev packages).
- [x] 1.3 Add `dev/build-nx-rust-player.mjs` and the `runtime:rust` script: check for the toolchain, the target and `emcc` first, build, copy the page and module into `<web root>/nx-rust/`, print raw and brotli sizes; verify it stops with a named message when `emcc` is not on the path, and that `npm run runtime` still succeeds with no Rust toolchain on the path
  - Done: `npm run runtime:rust` builds into `wwwroot/nx-rust` and prints 6407 KB raw, 1762 KB brotli. Without Emscripten it stops with "em++ is not on PATH…" before compiling. `npm run runtime` does not call it.
- [x] 1.4 Port the spike's player into the crate (the image swap, the stack budget from `emscripten_stack_get_free()`, the epoch guard for stale observers) and verify the spike's four tests pass there under their new names
  - Done: ported with the stack budget; the spike's four tests pass under new names, plus one for replacing the program. The epoch guard is gone with the observers it guarded: the player now holds what it drew and patches controls itself. The stack probe moves with task 8.2.

## 2. The generated control table

- [x] 2.1 Write `nx/rust/generate-controls.mjs`: read `catalog-meta.json` and `drawnui.nx`, and the `props!`, `base_props!` and `setters!` blocks, enums and `on_*` hooks of the `drawnui` crate cargo resolved; fail on a block it cannot parse; verify it reports 331 properties matched by name before any renaming table exists
  - Done: the generator reads every block and fails on one it cannot. The "331 by name" figure in the proposal came from a parser that stopped at the `/>` inside `ItemTemplate`'s type; the catalog declares 371 properties, and by name alone the generator maps all but the ones `unsupported.json` lists.
- [x] 2.2 Add the type-conversion table and the renamed-member table (`Type`, `HoverChanged`, `RefreshCommand`, `LoadMoreCommand` and the rest found), and write `nx/rust/unsupported.json` with a reason per member; verify every catalog property and event is in exactly one of the two outputs and that removing an entry from the list fails the generation
  - Done: both tables and `unsupported.json` (member -> reason -> controls) are written; 68 of 371 declared properties are listed for at least one control, and 12 of 32 events. The list is an output, so the check is the generator's own: it fails on a catalog member that is neither set nor listed.
- [x] 2.3 Emit `nx/rust/src/controls.rs` (constructor, by-name set, by-name reset to the declared default, union matches, event hooks) with the catalog and `drawnui` versions recorded, commit it, and verify it compiles for both targets and that two runs produce identical bytes
  - Done: compiles for both targets; `--check` right after a run reports no difference.
- [x] 2.4 Make the build refuse a stale table: `build.rs` or the build script compares the recorded versions with `catalog-meta.json` and the resolved `drawnui`; verify by editing the recorded version and reading the message
  - Done: the build script runs `generate-controls.mjs --check` first; with the recorded `drawnui` version edited it stops with "…no longer match the catalog (…) and drawnui (…): run `npm run catalog:rust`".
- [x] 2.5 Add `npm run catalog:rust` and say in `nx/CATALOG.md` when to run it; verify it regenerates the committed file unchanged
  - Done: `npm run catalog:rust`; `nx/CATALOG.md` has a section on the table; `--check` after a run reports no difference.

## 3. Drawing and reconciling

- [x] 3.1 Build controls from a rendered value through the generated table: every catalog control, children through the content property, text spans, records into DrawnUI value types; verify with a test per control family that draws on the CPU canvas and reads properties back from the tree
  - Done: every catalog control is built through the table and all 11 presets draw headless with nothing reported. `a_control_of_each_family_takes_its_properties` reads the properties back from a layout, a label, a shape, a button, an image, a progress bar, a slider, a switch, a scroll and a grid. Controls are built bottom up, because one that is not a container takes its children when it is built.
- [x] 3.2 Patch in place: set changed properties through the `Mut` setters, reset removed ones, reconcile children by position with `insert_child`, `replace_child` and `remove`; verify tests for a changed value, a removed property returning to its default, an inserted and a removed child, and a replaced type
  - Done: tests cover a changed value, a property returning to its default (two of them, on the same label), an added child, a removed child and a replaced type. The empty value counts as unset.
- [x] 3.3 Walk authored components through `InstanceTree` (`begin`, `visit` keyed by path, `finish`), skip nodes that are settled, and give each handler record the key of its node; verify two uses of a stateful component keep separate state and that a tap in one does not rebuild the other (count the setter calls)
  - Done: two uses of a stateful component keep separate state, an emit reaches the parent, and only changed properties reach a setter (tested by counting them). Settled nodes are not skipped, on purpose: measured in a release build, comparing the whole render with the last costs 5 to 35 µs an event against 70 to 709 µs for the dispatch and pass, so skipping would save a twentieth. The design says so now, and `specs/future.md` has the numbers.
- [x] 3.4 Report an unsupported property and an unknown control once per snippet and keep drawing; verify with a snippet that sets one twice and reads one console line
  - Done: a property set twice is named once, and a control inside a label is named (`what_cannot_be_drawn_is_named_once_and_the_rest_stands`).

## 4. Events

- [x] 4.1 Bind every supported event from the generated hooks, building the action from the parameter names the catalog records; verify `Tapped`, `Toggled { value }`, `TextChanged`, `Scrolled` and one event per remaining hook signature with a test that fires the control's gesture or setter
  - Done: driven by real gestures in Chrome on both renderers with identical results: `FocusChanged`, `TextChanged`, `TextSubmitted` (editor), `EndChanged { value }` (slider), `Toggled { value }` (checkbox, switch), `Scrolled`, `SelectedIndexChanged { index }` (carousel), `Tapped`, `HoverChanged`, and a tap on a text span. The native tests cover `Tapped`, `Toggled`, `HoverChanged`, `Scrolled`, `EndChanged`, `TextChanged` and `SelectedIndexChanged`.
- [x] 4.2 Dispatch through `InstanceTree::dispatch` with the node's key, redraw from what it returns, print the catalog's print action and report other host effects as `nx` does; verify an emit reaching a parent, a host effect in the console, and a stale token leaving the drawing unchanged
  - Done: tested for a tap, an emit reaching a parent, a `Log` effect and a burst of events within one frame. The burst found a bug: a render is drawn after the handler returns (DrawnUI holds the control whose handler runs outside its tree), and the controls kept the retired tokens until then, so the second event of a scroll was refused with `nx-ir-handler-token`. The standing controls now take the new render's tokens at dispatch time, and a control the drawing will replace answers no event. The runtime README says so for other hosts.
- [x] 4.3 Report an unsupported event once; verify the console line and that the control still draws
  - Done: `onChildTapped` on a shape is named once, and the shape still draws and takes taps and hover.

## 5. Templated controls

- [x] 5.1 Drive `ItemsSource` and `ItemTemplate` through `items(count, template, bind)`: `bind` calls the template with `Item` and `Index` and reconciles the cell's content; verify a list of 5000 items calls the template only for realized cells, and that a nested templated control draws its own cells
  - Done: a 500-item list builds fewer than 40 rows, and a templated list inside a cell draws its own cells (`a_failing_template_is_named_and_a_list_in_a_cell_draws_its_own_cells`).
- [x] 5.2 Hold cell components in a second `InstanceTree` with no pass, keyed by list path and index, removed when the list leaves or its collection changes; verify state survives scrolling away and back, is dropped when the collection changes, and that a handler in the cell component's body runs
  - Done: tested: a handler in the cell component's body runs; its state survives scrolling away and back, and a render of the component that owns the list; another collection starts every cell from its initial state.
- [x] 5.3 Report a failing template with its name and index and an inert handler with its place, once each; verify the rest of the list still draws
  - Done: a failing template is named with its index and the rest of the list draws; a handler the template binds is reported inert once for all its cells, and a tap on it runs nothing.

## 6. The page, the frame and the language entry

- [x] 6.1 Write `nx-rust/player.html` and its script: post `ready`, accept `image` only from the parent, answer `result` with the image's id, forward `console`, apply background and gestures, and define `window.fiddleSnapshot`; verify in headless Chromium that an image posted before and after `ready` both draw
  - Done: the first run posts on `ready`, later runs post at once; both draw in headless Chromium.
- [x] 6.2 Write `fiddle-nx-rust.js`: compile through the compiler `fiddle-nx.js` loads, keep the latest image, post it to the frame, await its `result`, route console lines to the fiddle's console, and load nothing until first use; verify from the network log that a page that never selects `nx-rust` requests nothing under `nx-rust/`
  - Done: a page that never selects `nx-rust` requests nothing under `nx-rust/`.
- [x] 6.3 Add `NxRustLanguage` (Frame surface, constant frame address, `AutoRun` on, presets, foreign-code test and diagnostics taken from `NxLanguage`, share artifact as `nx`) and register it in the dev host with its script tag; verify `fiddle.listPresets()` lists `nx-rust` presets and that switching between `nx` and `nx-rust` keeps the buffer
  - Done: `fiddle.listPresets()` lists 11 `nx-rust` presets, and switching from NX keeps the snippet and draws it (browser check). One engine addition: `window.fiddleReactCompile(code, lang)`.
- [x] 6.4 Verify the run loop through `window.fiddle` in the dev host: a valid snippet reports success, a compile error and a link or evaluation error both arrive under the editor, an edit redraws without the frame reloading (same `contentWindow`, no second fetch of the module), and a runaway recursion fails with the resource-limit diagnostic while the next run draws
  - Done: verified through `window.fiddle`: success, a compile error (2 lines under the editor), a link error (the runtime's `nx-ir-link-version`, met for real when the dev host's NX runtime was a stale build), an edit drawn with one module fetch in total, and `nx-ir-resource-limit: Maximum NX IR call depth 100 was exceeded.` followed by a run that draws. The canvas background and the gestures mode reach the player at the next run the engine shows (a user run, or a run of changed code).

## 7. Shares and the `/p/` player

- [x] 7.1 Add the stored artifact to `FiddleRunContext` for a share that has one, and pass it in the `/p/` player for a Frame language; verify the existing languages' tests and presets are unaffected
  - Done: not needed after all: the script reads the share id the page already exposes (`window.fiddlePlayId()`) and fetches the stored module itself, so `FiddleRunContext` is unchanged. The fiddle's 99 NX tests pass.
- [x] 7.2 Play an `nx-rust` share from its artifact; verify with the local backend that `/p/{id}` draws, that `nx/nx.wasm` is not requested, that the editor link selects `nx-rust`, and that a share whose catalog version differs fails with the runtime's message
  - Done: with the local backend, `/p/{id}` draws the share in the player frame, requests the stored module and never `nx/nx.wasm`, and the editor link opens it in `nx-rust`. A share compiled against another catalog version shows the runtime's `nx-ir-link-version` message in the player page, which now says what went wrong where the drawing would be when nothing has drawn. 30 browser checks pass.
- [x] 7.3 Verify the share thumbnail: sharing a drawn snippet stores a picture that is not blank
  - Done: the picture of a drawn Welcome preset is an 84 KB JPEG; the browser check requires more than a blank one compresses to.
- [x] 7.4 Play an `nx-rust` share with no .NET, as `nx` and the engine's Rust language play theirs: `fiddleNxRustPlayable` and `fiddleNxRustPlay` in `fiddle-nx-rust.js`, two lines in the host's page, the badge and the embedding page's `ready`; verify that `/p/{id}` requests nothing under `_framework/`, asks for the stored program once, and sends `ready`
  - Done: added after upstream's Rust language showed the pattern. 30 browser checks pass. In Chrome with a warm cache the share draws 110 to 185 ms after navigation, against about 1.1 s through the .NET page (the dev host's debug build). A host that does not add the two lines plays the share through the page as before.

## 8. Tests across presets and measurements

- [x] 8.1 Add the headless preset test: compile every NX preset with the fiddle's compiler, draw each on the CPU canvas, fail on an error or on a report not in the unsupported list; wire it into `npm run test:nx` behind a check for the Rust toolchain, and verify it passes
  - Done: as `npm run test:nx:rust` (`nx/rust/test.mjs`): the table check, the images, `cargo test`; 25 tests pass. It is its own script, not part of `npm run test:nx`, because the JavaScript runtime build runs that one and must not need Rust.
- [x] 8.2 Add `nx/rust/browser-check.mjs` (from the spike's): the language switch, a tap, a live image swap, compile and runtime errors, shares and the `/p/` player; verify it passes
  - Done: 30 checks pass. The frame shared with the engine's Rust language was tried by hand on 2026-10-09, since drawfiddle.com will offer both: with `RustLanguage` registered in the dev host for the probe and the fiddle's Rust template built locally and served as its build, the page went Rust, `nx-rust`, Rust, `nx-rust`. Each `nx-rust` run settled in 50 to 450 ms with the player in the frame and a tap reaching its handler, each Rust run showed the build, and neither script took the other's messages. The dev host does not register the Rust language, so `browser-check.mjs` does not repeat this. The spike's stack probe is not ported: runaway recursion is checked through the real page (call depth), and the stack budget itself by this repository's runtime tests.
- [x] 8.3 Measure in a desktop browser with a GPU: module size raw and compressed, time from `image` to first frame, frame time while scrolling the list preset; record the numbers in `README-NX.md`
  - Done: Chrome 154, Intel Arc B580: 6.4 MB module (1.8 MB brotli); every preset scrolls at 60 fps (16.7 ms median frame, 16.9 ms p95); an edit is drawn 32 to 43 ms after the run starts (compiler 14 to 23 ms), 135 ms for the 100,000-item Cells preset. Recorded in `README-NX.md`.
- [x] 8.4 Compare the two renderers: draw each preset under `nx` and `nx-rust` in the dev host, and write every visible difference into `README-NX.md` with its cause (unsupported member, DrawnUI parity, or a bug to fix here)
  - Done: all 11 presets drawn side by side in Chrome. Two bugs found and fixed with tests: a `CornerRadius` with one corner must round every corner, and an eight-digit color has its alpha first (`#AARRGGBB`). What remains is the engines': text wraps a word apart here and there, and a `SkiaScroll` with no height takes its content's height in Rust and the offered height in React (the UI preset's card). Written into `README-NX.md`.

## 9. This repository

- [x] 9.1 Add a test to `crates/nx-codegen/src/ir_instance_tree_tests.rs` for nodes held outside a pass: visits under no parent with no `begin` or `finish`, a dispatch that patches one, a removal by key, and a second tree over a clone of the same program; verify `cargo test -p nx-codegen ir_instance_tree` passes
  - Done: `nodes_a_host_holds_outside_a_pass_stay_until_it_removes_them`. Since the review the test, like the README, calls `begin` before each bind and never `finish`: `begin` empties the tree's record of inert handlers and drops nothing. The 42 instance tree tests pass.
- [x] 9.2 Add a "Cells a host binds outside a pass" section to `crates/nx-ir-runtime/README.md` with the pattern and a short example; verify the README's doc tests pass
  - Done: "Cells a host binds outside a pass", with a compiled example; the runtime's doc tests pass.
- [x] 9.3 Fix what tasks 3 to 8 find in `crates/nx-ir-runtime`, each with a test in this repository, and list each fix here as it lands; verify `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`
  - Done: nothing in `crates/nx-ir-runtime` needed a fix: every preset, the cells and the browser run went through its API as merged. `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass with the new test and README section.
- [x] 9.4 Record in `specs/future.md` what the player showed about the two open items there (a runtime-owned tree with a change set, tracked reads): what the walk cost on real presets
  - Done: "What the first Rust host measured" in `specs/future.md`: per-event costs on four presets, and what they say about a runtime-owned tree and tracked reads.

- [x] 9.5 Bound how deeply an instance tree nests component instances: `RuntimeOptions::max_component_depth`, 100 by default, checked by every `visit` and reported as `maxComponentDepth`; tests for a component that renders itself and for a limit the host raises and lowers; the README's limits table and *Small stacks*, `docs/nx-ir-format.md`, and the two spec deltas; verify `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`
  - Done: `a_component_that_renders_itself_ends_at_the_component_depth` and `a_host_sets_the_component_depth`. The player keeps its own bound of 128 over controls and components together, and its test now shows both diagnostics: the player's for a component that draws itself inside a layout, the runtime's for one with no control between. The second review pass found that neither bound reached cells nested in cells: the player now carries its depth from a list into the cells it binds, and the README says the component depth starts again in every cell.

## 10. Fiddle documentation

- [x] 10.1 Update `docs/ADDING-A-LANGUAGE.md` (a Frame language that keeps its page and takes programs by message; the stored artifact in the run context), `AGENTS.md` (the `runtime:rust` and `catalog:rust` commands, the toolchain, where things are) and `README-NX.md` (what `nx-rust` is, the unsupported list, the known differences from `nx`, cells keeping state)
  - Done: `docs/ADDING-A-LANGUAGE.md` (a Frame language that keeps its page), `AGENTS.md`, `README-NX.md` and `nx/CATALOG.md`.
- [x] 10.2 Delete `~/src/drawnui-nx-spike` after confirming every test and probe it had runs from `nx/rust/`, and update the memory note that points at it
  - Done: deleted on 2026-10-09 at the user's request. Everything the spike tested runs from `nx/rust/` except its stack probe, which was not ported; what it measured is in the runtime README's *Small stacks*.

## 11. Release, then published versions

- [x] 11.1 Confirm the gate: every task above is checked, the preset test and the browser check pass against this checkout's `main`, and `pnpm run verify:crates` passes
  - Archive `support-rust-hosts` before this change. It adds the `component-instance-tree` capability with its Purpose and first nine requirements, and this change adds a tenth: archived the other way round, the spec is created with a placeholder Purpose and the depth requirement ahead of the ones that introduce the tree.
  - Done: 2026-10-09. Every finding of the review (RF1 to RF24) is verified; the player's 25 tests and the 30 browser checks passed against this checkout's `main`, `cargo test --workspace` passed (3135), and `pnpm run verify:crates` passed. `support-rust-hosts` is still to be archived, before this change.
- [x] 11.2 Release the crates with NX `v0.7.0`: the user pushes the tag and publishes the draft (first versions go out with `CRATES_IO_TOKEN`, then trusted publishing is configured, per `docs/deployment-setup.md`); verify the three crates resolve from crates.io at that version, and check off tasks 4.3 and 4.6 of `support-rust-hosts`
  - Done: NX `v0.7.0` released 2026-10-09 (PR #48 for the runtime, PR #49 for the notes). The draft was checked against `docs/deployment.md` before it was published: manifest, checksums, the three VSIX identities, and `nx-ir-runtime` requiring `nx-ir` and `nx-value` at `=0.7.0`. crates.io serves `nx-ir`, `nx-value` and `nx-ir-runtime` 0.7.0 with the attached files' checksums; npm, NuGet, the Marketplace and Open VSX serve 0.7.0. The first versions went out with `CRATES_IO_TOKEN`; the same day the user added a trusted publisher to each of the three crates, deleted the secret and revoked the token.
- [x] 11.3 Replace the path dependencies in `nx/rust/Cargo.toml` with the published version and move the fiddle's `@nx-lang/*` pins to the same release; verify the player builds with no sibling checkout, and that `npm run test:nx`, the preset test and the browser check pass
  - Done: `nx/rust/Cargo.toml` names `nx-ir-runtime` and `nx-value` at `=0.7.0` and `package.json` pins the four `@nx-lang/*` packages at 0.7.0. With no sibling checkout: `npm run runtime` builds, `npm run test:nx` passes (98, and the one test that needs a checkout's debug module is skipped, as it is for any install from npm), `npm run test:nx:rust` passes (25), `npm run runtime:rust` builds the player from crates.io, and the browser check passes (30).
- [x] 11.4 Open the pull request to `DrawnUi/DrawnUi.FiddleEngine` with the measurements, the unsupported list and the one engine contract change called out, and note for the maintainer the two lines drawfiddle.com needs to offer `nx-rust`
  - Done: https://github.com/DrawnUi/DrawnUi.FiddleEngine/pull/4 (2026-10-09), one commit on `nx-rust`, with the measurements, the unsupported list's totals, the function added to the React surface, and the lines a host adds to offer the language.
