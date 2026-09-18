Paths without a repository prefix are in `~/src/DrawnUi.FiddleEngine`; `nx:` marks this repository.
Every fiddle command runs in side-by-side mode (`--nx ../nx`). Commits are local to
`nx-linkable-ir`; nothing is pushed and no pin in `package.json` changes.

## 1. Baseline: this checkout and the fiddle branch

- [x] 1.1 Build this checkout (`nx:` `pnpm install && pnpm -r build`, plus the wasm SDK build if `bindings/wasm` has no fresh `nx.wasm`) and verify `pnpm -r test` is green, so a later fiddle failure is not a stale NX build
- [x] 1.2 Re-emit `nx/catalog/drawnui.nxir` with `npm run catalog -- --nx ../nx` (the committed image predates `emits` in the layout), run `npm run test:nx -- --nx ../nx` and verify it passes on the pending image rework as it stands
- [x] 1.3 Remove what is left of the JSON encoding (`drawnui.nxir.json` references in `README.md`, `AGENTS.md`, `nx/CATALOG.md`, `nx/src/modules.d.ts`, scripts) and verify `grep -rn "nxir.json"` outside `node_modules` finds nothing
- [x] 1.4 Commit the image rework (including `.gitattributes` and the new image) as one commit and verify `git status` is clean
- [x] 1.5 `git fetch origin` and merge `origin/main` into `nx-linkable-ir`; resolve any conflict, rerun `npm run test:nx -- --nx ../nx`, and verify `git merge-base --is-ancestor origin/main HEAD` succeeds (expected: already up to date)
- [x] 1.6 Run `npm run runtime -- --nx ../nx` and verify the summary line names the checkout and `src/Fiddle.DevHost/wwwroot/nx` holds a fresh `nx-runtime.js` and `nx.wasm`; verify that naming a directory that is not a checkout fails before building

## 2. Catalog: DrawnUI events as emits

- [x] 2.1 Port event detection from `nx:sites/playground/scripts/generate-catalog.mjs` into `nx/generate-catalog.mjs`: an optional function-typed member returning `void` is an emit on the component for its declaring class, primitive parameters become payload fields, engine-object parameters are dropped and recorded; verify with a generator test that `SkiaControl` declares `Tapped` and `SkiaToggle` declares `Toggled { value:boolean }`
- [x] 2.2 Record each event's parameter names in order under the component's entry in `catalog-meta.json`, extend the metadata types in `nx/src/values.ts`, and verify `tsc` passes through `npm run test:nx -- --nx ../nx`
- [x] 2.3 Regenerate (`npm run catalog -- --nx ../nx`), review the diff of `drawnui.nx`, `omitted.json` and `catalog-meta.json` for events the playground's generator never met, and verify `omitted.json` holds no event callback while `ItemTemplate` is still listed; note the catalog image's size before and after in the commit message
- [x] 2.4 Add runtime tests: `onTapped` on a `SkiaShape` inside a component compiles, `onToggled=<Update on={action.value} />` compiles, and `onNoSuchEvent` fails with an `L{line}:` error; verify they pass
- [x] 2.5 Update `nx/CATALOG.md` ("How it is derived": events, payloads, dropped parameters) and commit

## 3. Renderer: instances and handlers

- [x] 3.1 Copy `nx:sites/playground/src/render/instances.ts` to `nx/src/instances.ts` (imports and types adapted, logic unchanged, origin noted in the header) and port `instances.test.mjs` as `nx/test/instances.test.ts`; verify its cases pass: tap patches state, live state across taps, emit reaches the parent, handler in content patches its owner, child state survives a parent redraw, failed dispatch restores
- [x] 3.2 Port the handler-record-to-callback binding (and the `ContextMenu` convention) from the playground's `values.ts` into `nx/src/values.ts`, using the recorded parameter names to build the action; verify with a `values.test.ts` case that a callback's arguments become the action's fields
- [x] 3.3 Rework `nx/src/draw.tsx` to draw authored components through the instance tree instead of `renderAuthoredComponent`, dropping unvisited nodes at the end of a pass, and to strip and report token-less handlers from `root`; verify existing draw tests still pass
- [x] 3.4 In `nx/src/runtime.ts`, hold the tree per mounted `NxProgram` keyed to its image, redraw on a completed dispatch, and send host effects, dispatch failures and the inert-handler notice to the console; extend the React stub in `nx/test/support.ts` with `useRef`/`useState`; verify with runtime tests: two taps read `Taps: 2`, a new image resets state, a failing handler leaves the element tree unchanged, and mounting a share string (no compiler host created) dispatches
- [x] 3.5 Commit

## 4. Presets

- [x] 4.1 Make Welcome interactive: a page component with `count:int` state, the button's `onTapped=<Update count={count + 1} />`, and a label built with `"..." + count`; verify it compiles and the generic dispatch test (4.5) changes its drawing
- [x] 4.2 Re-port Text and Shapes from the playground's current `text.nx` and `shapes.nx` (tapped span, tapped markdown link, `ContextMenu`), substituting the fiddle's fonts and host-served assets; verify they compile and draw
- [x] 4.3 Add the Controls preset from the playground's `looks.nx` (child `Card` emitting `Logged`, parent patching `last`), with fonts and assets substituted; verify it compiles and that a child's event changes the parent's label in the test
- [x] 4.4 Go over Layouts with the new conversion rules (integer-typed props where they read better, string concatenation where a caption was hard-coded) without adding interaction it does not need; update every preset's description in `NxAll` to say what is interactive; verify all presets compile
- [x] 4.5 Extend `nx/test/runtime.test.ts`: for each preset, draw, call the first bound callback with stub arguments, redraw and assert the tree changed; assert presets meant to be static bind none; raise "at least four presets" to five; verify the 128 KB share budget holds for all
- [x] 4.6 Commit

## 5. NX fixes, if any

- [x] 5.1 For each failure traced to NX rather than the fiddle, add a failing test in the owning package, fix it, rebuild the checkout and rerun the fiddle; verify `nx:` `cargo test --workspace` and `pnpm -r test` are green. Found and fixed:
  - `nx:bindings/wasm/test`: three tests still expected IR schema 3 and one used `1 + "x"` as its compile failure, which the implicit conversions made legal; they expect 4 and use `1 - "x"`. Stale tests, no product change.
  - `nx:crates/nx-language-service`: property completions and hover ignored a component's emits, so `onTapped` was never offered inside a tag (found by task 7.3). Three failing tests first, then `on<Emit>` properties listed after the props, inherited ones included; delta spec under `specs/editor-language-service`.
  - `nx:crates/nx-syntax`: a comment as the first thing inside a `for` body in content position made the loop emit nothing valid (`L{n}: malformed expressions cannot be emitted`), while the same comment after the element was fine. `SyntaxNode::children()` used `named_child`, and a comment is a named node in `extras`, so a wrapper's first child could be the comment and lowering lowered it as the expression it wrapped (`crates/nx-hir/src/lower.rs:1537`). `children()` now skips comments; `children_with_tokens()` keeps them for the prose scan in `validation.rs` that needs them. Failing test first in `syntax_node.rs`, then the one-line filter; `cargo test --workspace` 1996 green. Found by the Cards preset, whose `for` body opens with a comment. `child`, `child_count`, `next_sibling` and `prev_sibling` read the same view (see 12.2), so the i-th child is the i-th construct however the source is commented.
  - Not NX: the inert-handler notice appeared three times in the browser because the engine mounts one image several times; the fiddle now reports once per prepared program.

## 6. Docs

- [x] 6.1 Update the fiddle's `README.md` and `AGENTS.md`: handlers and state in NX snippets, handlers need a component, reports go to the console, the branch builds only with `--nx` until the NX release; verify every command quoted there runs as written
- [x] 6.2 Commit

## 7. End-to-end in a browser

- [x] 7.1 Run `npm run runtime -- --nx ../nx`, start `node dev/local-backend.mjs` (5299) and `dotnet run --project src/Fiddle.DevHost` (5041), and verify with headless Playwright through `window.fiddle` that every NX preset loads with `getState()` reporting success, no errors and no Monaco error markers
- [x] 7.2 Tap Welcome's button (coordinates from the DrawnUI accessibility overlay) and verify by screenshot that the count changed and that no request for `nx.wasm` followed the tap; do the same for one switch in Controls and verify the parent's label changed
- [x] 7.3 Verify the editor services still answer against the new catalog: completion after `<` lists DrawnUI components, completion inside a tag offers `onTapped`, hover on a property describes it, a color literal shows a swatch
- [x] 7.4 Share the interactive Welcome (`window.fiddleReactEmit(code,'nx')`, `POST /api/share`), open `/p/{id}`, and verify it posts `ready`, a tap changes the drawing, and the page never requests `nx.wasm`; verify the editor link reopens in NX with the same source
- [x] 7.5 Verify a C#-only and a TSX-only session still request neither `nx-runtime.js` nor `nx.wasm`, and that TSX presets draw as before
- [x] 7.6 Put a handler on the top-level element of a snippet and verify it draws, nothing happens on tap, and the console carries the inert-handler notice once

## 8. Close out

- [x] 8.1 Run the full suites one last time (`nx:` `cargo test --workspace`, `pnpm -r test`, `dotnet test bindings/dotnet/NxLang.sln`; fiddle `npm run test:nx -- --nx ../nx` and `dotnet build DrawnUi.FiddleEngine.slnx`) and verify all green
- [x] 8.2 Run `openspec validate update-fiddle-to-latest-nx --strict` and verify it passes; record in this file what remains after the change: tag the NX release, repin the fiddle, open its pull request

## 9. Printing: a snippet's own lines in the console pane

Added after the first close-out: the fiddle's CONSOLE pane stayed empty on an NX tap while the C#
and TSX twins printed, because an NX handler has no print statement and because the runtime's own
reports went to the browser console only, not through the channel that fills the pane.

- [x] 9.1 In `nx/src/runtime.ts`, send every report (host effect, failed dispatch, unknown control, inert handler) through `window.fiddleReactOnConsole` when the page has set it, and always through the browser console; verify with a runtime test that installs a stub hook and sees each kind of report arrive on both
- [x] 9.2 Read an unhandled `Log` action carrying `text` as a line to print, written as that text alone, with every other host effect still named in full; verify both shapes in a runtime test
- [x] 9.3 Give the Welcome preset `action Log = { text:string }`, `emits { Log }` and a handler that both patches `taps` and emits `Log`, so a tap prints `Button clicked N time(s)` as its twins do; verify with a preset test that two taps print those exact lines and the label still counts
- [x] 9.4 Document it: the `Log` convention and the two channels in the fiddle's `README.md`, the preset's description in `NxAll`, and decision 3 of this change's design; verify `npm run test:nx -- --nx ../nx` is green and the preset descriptions still say what is interactive
- [x] 9.5 Verify in the browser: load NX Welcome in the dev host, tap three times, and read `fiddle.getConsole()` and the CONSOLE pane; compare the lines with the C# and React Welcome presets' own
- [x] 9.6 Rerun the close-out suites (8.1) after the change and commit
- [x] 9.7 Align the whole starter with its twins, not only the one line: both twins also report the title changing (C# by observing the button's `Text`, TSX with an effect), so the handler that changes it says so; `titleOf(count)` feeds the button and the label above it; the twins' captions on a `SkiaButton`. Verify in the dev host that all three presets tapped three times print the same six lines, and that the NX screen matches
- [x] 9.8 Record what the alignment ran into in NX: a double quote cannot be written in an NX string literal. `crates/nx-syntax/grammar.js` lexes `seq('\\', /./)` inside a string, so `"a\"b"` is one token, but `unquote_string_literal` (`crates/nx-hir/src/lower.rs:335`) only strips the outer quotes, so the value keeps the backslash and `"` cannot be produced at all. Left for a decision rather than fixed, because it is a language-semantics change, and folded into `specs/future.md` with the rest of the string-literal question: the fiddle's mirror label drops the twins' quotes around the title meanwhile

## 10. The gallery reads like its twins

Added after the second close-out, from two questions about how the NX presets sit beside the C# and
TSX ones.

- [x] 10.1 Rename the component each preset draws from `Page` to `App`, as the TSX templates name their root component, in the presets, the README's examples and the file's own doc comment; verify with a test that a preset declaring `component <App` ends in `<App />`, that no preset names a `Page`, and that a preset with no component ends in a closing tag
- [x] 10.2 Add the Cards preset, the twin of the engine's TSX `ReactCards`: `type Plan` and a list of records, `for plan, index in plans` for the twin's `PLANS.map((plan, i) =>`, a gradient per plan, and the tapped card outlined. Verify the three cards draw with the second picked, that tapping the third moves the outline, and that it sits second in `NxAll` as Cards does in the TSX gallery
- [x] 10.3 Fix the NX defect the Cards port turned up (see 5.1: a comment first in a `for` body), rebuild the checkout's wasm, and verify the preset compiles with the comment where it belongs
- [x] 10.4 Verify in the browser: all six NX presets load with no errors and no markers after the rename, and Cards is indistinguishable from `react-cards` before and after tapping the third card
- [x] 10.5 Update the README's preset table and the artifacts (the naming scenario in the spec, decision 4, this file), rerun the suites and commit

## 11. The C# templates that port

Added after the third close-out: which of the C# gallery's thirteen templates an NX snippet can draw
whole, given the catalog the generator produces and what the language offers today.

- [x] 11.1 Read each C# template against the catalog and record the verdict: three port (UI, Editor, Glass), the rest want something that is not there — `VisualEffects` (Effect), `ItemsSource` and recycling (Cells), animation, subclassing or a raw canvas (Spinner, Custom, SKSL, Raw), the host's `Fiddle` API (Data I/O, Weather) — or are already shown (Looks, by Controls). Kids is the near miss; the reason it stays out is in decision 4
- [x] 11.2 Add UI: the twin's palette and screen with its observers turned around, each switch patching the state that draws it and emitting `Log`, the slider's `EndChanged` feeding the percent label, the sign-out shape keeping `AnimationTapped=Ripple`. Verify it compiles, draws and answers its first event
- [x] 11.3 Add Editor: `SkiaEditor` with `onTextChanged` patching state that the label under it echoes, and a second `Send`-key editor reporting `TextSubmitted`. Verify, and note in the snippet that NX currently has no way to get a string's length, which is why the echo repeats the text where the twin counts characters
- [x] 11.4 Add Glass: `SkiaBackdrop` over gradient circles, with alpha written into each colour. It holds no state, so verify it binds no handler and add it to the test's static presets. The four circles differ only in colour, size and offset, so they are one `Glow` element function taking those three; verify the canvas is pixel-for-pixel what the spelled-out version drew
- [x] 11.5 Verify NX puts the drawn element last — anything after it is a syntax error — which is what decides Kids; check it against the compiler rather than the grammar, and keep no test for it here (it is a language fact, not a fiddle one)
- [x] 11.6 Update the README's preset table with the three new sizes, decision 4 and this file; rerun `npm run test:nx -- --nx ../nx` and `dotnet build DrawnUi.FiddleEngine.slnx`, verify in the browser, and commit

## What remains after this change

- Commit the NX side of this change on `enhance-nx-fiddle-support` (the wasm SDK's test expectations, the language service's handler properties, the named-child accessors skipping comments, the playground renderer's stale-event report, the `specs/future.md` entry, this change's artifacts); it is in the working tree, uncommitted.
- Tag the NX release that publishes IR schema 4, the action-handler runtime and the handler-property completions.
- Repin the fiddle's `@nx-lang/*` devDependencies to that release, run `npm run catalog`, `npm run runtime` and `npm run test:nx` without `--nx`, and drop the "builds only with `--nx`" paragraph from the fiddle's `AGENTS.md`.
- Push `nx-linkable-ir` (twelve local commits on top of `02fa142`) and open the fiddle's pull request.
- Confirm NX's string quote and escape semantics, and fix the bugs in what is there today. Recorded in `specs/future.md` under "String Literal Quotes And Escapes: The Semantics Are Unconfirmed", with the measured behavior, the four candidate mechanisms and five numbered bugs. Found by the fiddle's Welcome preset, which wants the twins' `"Clicked 3 times!"` in a label and drops the quotation marks meanwhile.
- Noted, not changed: element-name completion lists every emit's action with its `.Update` and `.Property` companions (`AnimatedFramesRenderer.Finished.Update`), which crowds the list now that the catalog declares 43 emits. Worth a look in the language service.
- Noted: `dotnet test bindings/dotnet/NxLang.sln` loads `target/{debug,release}/libnx_ffi.so`, which `cargo test --workspace` does not rebuild; after an IR change run `cargo build -p nx-ffi` (and `--release`) first.

## 12. Review fixes

From `review.md` (findings RF1–RF7); each fixed item is marked `🟡 Fixed` there for the reviewer to
verify.

- [x] 12.1 RF1: an event that reaches a drawing a dispatch has already replaced is reported rather than dropped in silence — `reportStale` on the draw context, said every time as a failed dispatch is; the delta spec has a scenario, design decision 3 says why it cannot be re-dispatched (tokens are numbered per generation), and the playground's copy reports it to its console. Verify: the stale-callback test asserts one warning naming `SkiaButton.onTapped`
- [x] 12.2 RF2: `child`, `child_count`, `next_sibling` and `prev_sibling` skip comments as `children()` does, so `child(0)` is the first construct for the four `positions.rs` callers that read a property's name that way; a test in `syntax_node.rs` covers index and sibling access over a commented list
- [x] 12.3 RF3: reverted the unrelated `crates/nx-syntax/tests/fixtures/valid/function.nx` edit (it dropped `/>: Element`, which no task here asks for and which was the only fixture covering a component's return annotation)
- [x] 12.4 RF4: a program that fails to prepare, link or evaluate is held like a session, so the failure is neither retried nor said again on every render; the version-mismatch test renders twice and mounts the image again, and asserts one line
- [x] 12.5 RF5: a host effect names the component, which is what the runtime carries; the delta spec and design say the component rather than the instance, the instance key being a position path that tells a reader less
- [x] 12.6 RF6: `readEvent` is exported and tested directly (`nx/test/generator.test.ts`) for the rule with no output to read — a kept parameter after a dropped one is refused; the dispatch test's stub arguments are guarded by a test that no two catalog events share a name and differ in payload
- [x] 12.7 RF7: the handler-property requirement opens by saying it refines the completions and hover requirements, and defers to their terms instead of restating them
