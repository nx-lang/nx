# Review: add-nx-rust-to-drawnui-fiddle

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `specs/fiddle-nx-rust-language/spec.md`  
**Reviewed code:** uncommitted working trees of two repositories.  
- `~/src/nx` (`support-fiddle-rust-player`): `crates/nx-codegen/src/ir_instance_tree_tests.rs`,
  `crates/nx-ir-runtime/README.md`, `specs/future.md`, and `crates/nx-ir-runtime/src/tree.rs` and
  `eval.rs` as the API the player calls.
- `~/src/DrawnUi.FiddleEngine` (`nx-rust`): `nx/rust/` (`src/lib.rs`, `src/player.rs`,
  `src/values.rs`, `generate-controls.mjs`, `src/controls.rs` and `unsupported.json` as its output,
  `web/player.html`, `tests/player.rs`, `tests/snippets/`, `make-images.mjs`, `test.mjs`,
  `browser-check.mjs`, `Cargo.toml`, `.cargo/config.toml`), `dev/build-nx-rust-player.mjs`,
  `src/DrawnUi.Fiddle/Languages/NxRustLanguage.cs`, `wwwroot/fiddle-nx-rust.js`, the changes to
  `fiddle-react.js`, `FiddlePresetsNx.cs`, the dev host, `package.json`, and the four documents.
  `FiddlePage.razor` and `drawnui 0.1.0-preview.6` (`controls/list.rs`, `host_web.rs`, `lib.rs`)
  were read for what the new code relies on.

**What was run:** `cargo test -p nx-codegen ir_instance_tree` (40 pass), `cargo test -p
nx-ir-runtime --doc` (10 pass, 1 ignored), `cargo clippy -p nx-codegen -p nx-ir-runtime
--all-targets -- -D warnings` and `cargo fmt --check -p nx-codegen` (clean), `npm run test:nx:rust`
(17 pass, 1 ignored), `npm run test:nx` (99 pass), `nx/rust/browser-check.mjs` (20 checks pass),
the ignored measurement test in release (numbers agree with `specs/future.md`). Findings marked
"probe" were reproduced with scratch snippets, a scratch test crate and Playwright scripts, all
under the session scratchpad (`review2/probe/`); nothing in either repository was changed.

## Findings

### ✅ Verified - RF1 A component that renders itself crashes the browser tab, and a few hundred nested components hang it
- **Severity:** High
- **Evidence:** The requirement "A snippet cannot take the page down" covers "the depth the
  player's stack allows". Nothing bounds the player's own recursion. `render`
  (`nx/rust/src/lib.rs:743-784`) calls itself once per level of the output and visits a component at
  each descriptor; `build`, `patch`, `children`, `rebind`, `takes` and `forget` recurse the same
  way, and DrawnUI's layout recurses over what they built. The runtime's limits do not reach this:
  the call depth of 100 and `max_operations` are per call, and each `visit` is its own call;
  `max_stack_bytes` is measured from where each call begins (`Stack::begin`,
  `crates/nx-ir-runtime/src/eval.rs:126`), so the stack the player uses between two visits is not
  counted. `options()` is also built once per pass (`lib.rs:265`, `313`), so a visit deep in the
  walk is given the budget that was free at the top, which is more than is left. The design says
  the budget is set "at each call into the runtime".
  Probe, in the dev host: `component <N extends DrawnNode /> = { <SkiaLayout Type=Column><N
  /></SkiaLayout> }` with `<N />` as the top-level element crashes the whole tab under `nx-rust`
  (Playwright: "Target crashed", twice out of two), editor included. Under `nx` the same snippet
  prints `NX: the program failed to run: Maximum call stack size exceeded` and the next run draws.
  A component nested 400 deep through an `n` prop leaves the tab unresponsive; 150 deep draws.
  Natively both abort the test process with "has overflowed its stack". The module is linked with
  `-sSTACK_OVERFLOW_CHECK=1` (`nx/rust/.cargo/config.toml:12`), which checks a cookie after the
  fact and does not trap an overflow when it happens.
- **Recommendation:** Give the pass a depth and fail it like a limit. In `render`, count the
  nesting of authored components and of controls and return a resource-limit diagnostic past a
  bound that the 1 MiB stack is measured to survive, layout and paint included (150 levels drew;
  the bound should be well under what fails). Also compare `emscripten_stack_get_free()` with a
  margin before each recursion, and compute `max_stack_bytes` at each `visit`, not once per pass.
  Because `load` and `dispatch` already run the pass inside `atomically`, a failure there leaves
  the drawing standing. Consider `-sSTACK_OVERFLOW_CHECK=2` so that an overflow which still gets
  through is a trap the page reports, not corrupted memory. In this repository: every Rust host
  that walks recursively meets this, so consider a depth limit on `InstanceTree::visit` (the nodes
  above a node are already counted by `depth`), and say in the README that `max_stack_bytes`
  covers one call and not the host's recursion between calls. Add a native test and a browser
  check for the self-rendering component.
- **Fix:** The player bounds its own depth: a pass fails with `nx-rust-drawing-nesting` past 128 nested controls and components (`Walk::deeper` in `nx/rust/src/lib.rs`; 300 levels by this count were seen to draw), inside `atomically`, so the drawing and the program behind it stand. Each `visit` is given the stack free where it is made, not where the pass began. Tests: `a_component_that_draws_itself_fails_the_run_and_the_next_program_draws` natively, and two browser checks (the run fails with the diagnostic, the next snippet draws, the tab lives). The runtime README's *Small stacks* now says the budget covers one call and not the host's recursion between calls. Then, on the user's decision, the runtime's own limit: `InstanceTree::visit` refuses a node more than `RuntimeOptions::max_component_depth` instances deep (100 by default) with `nx-ir-resource-limit` naming `maxComponentDepth`; tests `a_component_that_renders_itself_ends_at_the_component_depth` and `a_host_sets_the_component_depth`, and the player's test now meets both diagnostics. Not done: `-sSTACK_OVERFLOW_CHECK=2`, which departs from the link flags DrawnUI for Rust documents.
- **Verification:** Fixed for what the finding names. In the player, `Walk::deeper` counts every
  component and every control record and is asked before the visit (`nx/rust/src/lib.rs:775-795`,
  `:830`, `:846`), and `options()` is built at each visit (`:833`). In the runtime, `visit` takes
  the depth from the parent's chain before it changes anything (`crates/nx-ir-runtime/src/tree.rs:340-353`).
  Run: `cargo test -p nx-codegen ir_instance_tree` (42 pass, the two new tests among them), the
  player's `a_component_that_draws_itself_fails_the_run_and_the_next_program_draws` (passes), and
  the browser check's two checks (pass). Probed in the dev host: `recursive_bare.nx` ends with
  `nx-ir-resource-limit` naming the component depth under the editor and the next run draws; a
  finite drawing 124 levels deep draws. Probed natively: a tap that takes a drawing past the bound
  says the diagnostic in the console, and the drawing stands and answers the next tap. A scratch
  test of the runtime's limit at its edges: a limit of 0 refuses a root and a limit of 1 the second
  node; a node visited again under a deeper parent is refused, keeps its output, and is still in
  the pass's record (`finish` keeps it); a held node past a lowered limit is refused unchanged.
  Not covered by this fix, and reported on its own: a cell starts both counts again (RF15).

### ✅ Verified - RF2 A new program rebuilds every control, so an edit loses the scroll position the spec, the design and README-NX say it keeps
- **Severity:** High
- **Evidence:** `Inner::load` removes every mounted control before it draws
  (`nx/rust/src/lib.rs:284-289`: "every list is drawn anew"), so nothing is patched in place across
  a run. The spec requires "A run that changes only property values SHALL update the controls
  already drawn, so that what the engine holds for them, such as a scroll position, is kept", with
  the scenario "A scroll position survives a value edit" (`spec.md:57-69`). `design.md:88-93` says
  a new image "reconciles the controls in place where the type at a position is unchanged, so a
  scroll position survives", and `README-NX.md:137` tells users "a scroll position survives an edit
  that changes only values". Probe: a scrolled list loaded again with one color changed is another
  `SkiaScroll` control, and its offset goes from -450 to 0. No test covers it:
  `a_patch_keeps_the_scroll_position` (`tests/player.rs:121`) taps a button, which is a dispatch,
  and the browser check only asserts that an edit "is drawn".
- **Recommendation:** Reconcile on load as on a dispatch: keep `self.drawn` and call `draw`
  against the new render. What the removal was protecting against is cells that show the old
  program's items; handle that directly by making every list unequal across a load (clear
  `self.lists`, or give `List` the program's epoch), so each templated control is reset and its
  cells are bound again against the new cell tree, and let `patch` rebind every handler. Add a
  native test (load, scroll, load a variant, same `ControlId`, same offset) and a browser check.
  If rebuilding is what is wanted, change the requirement, the design section and the README
  sentence instead.
- **Fix:** `Inner::load` keeps the mounted controls and draws the new render over them, as after a dispatch. What it forgets is every handler and every list, so each control is bound again as it is drawn and each templated control is given its collection anew and binds its cells against the new program's cell tree; `forget` now goes by how a control was made, not by what is remembered. Tests: `an_edit_that_changes_a_value_keeps_the_controls_and_the_scroll_position` (same `ControlId`s, same offset, no control made, the new program's handler runs) and `a_list_shows_the_new_programs_items_after_an_edit`.
- **Verification:** `Inner::load` no longer removes anything: it clears the handlers and the lists
  and draws the new render over `self.drawn` (`nx/rust/src/lib.rs:273-296`), `patch` binds every
  control again (`:523`) and resets every templated control, whose list is no longer held
  (`:530-535`), and `forget` reads `gone.templated` (`:665`). Both tests pass, and the first would
  fail on the old code (`mounts()` is 0 after the edit). Probed natively beyond them: a scrolled
  templated list (`cells.nx`, and the Cells preset) keeps its scroll control and its offset
  (-1800 before and after) across a load, with no control made, and shows the new program's cells.
  What standing controls cost that rebuilt ones did not is RF14.

### ✅ Verified - RF3 `ready` is set once and never questioned, so a frame that is not a live player makes every run wait 15 seconds and fail
- **Severity:** Medium
- **Evidence:** `fiddle-nx-rust.js` sets `ready = true` on the frame's first `ready` (`:45`) and
  clears it only when no frame exists (`:176-179`). Two ways it becomes untrue:
  1. Another Frame language takes the frame. The engine keeps `_frameUrl` when both languages are
     on the Frame surface (`FiddlePage.razor:1409-1410`), and `RustLanguage` ships in the engine.
     After `nx-rust`, then a Rust build, then `nx-rust` again, the frame shows `rs/run.html`,
     `frameWindow()` returns it, and the script posts the program there and waits. The run fails
     after `RESULT_TIMEOUT_MS`, so `FrameUrl` is null and the engine never puts the player back.
     Probe (the frame element given `rs/run.html`, then `fiddleNxRustRun` called twice): 15,022 ms
     and 15,023 ms, both `NX: the DrawnUI for Rust player did not answer.` Only a page reload ends
     it. `event.source === frameWindow()` cannot tell the pages apart either: a frame keeps its
     window object across navigations.
  2. The player stops. The release profile is `panic = "abort"`, and `player.html:44-46` reports
     an abort as a console line. The module is then dead, `RunAsync` keeps returning the same
     address, so the engine never replaces the frame, and every later run waits 15 seconds.
- **Recommendation:** Decide readiness from the frame, not from history: require that the frame's
  address is the player's (same origin, so `contentWindow.location.pathname` can be read) and
  reset `ready` when it is not, which sends the run down the first-run path and returns the
  player's address. Have the frame tell the page when it stops, and have the script then answer
  the waiting run at once and return an address the engine sees as new (a counter in the query),
  so a dead player is replaced on the next run. Add a browser check that swaps the frame's page
  and runs.
- **Fix:** `frameWindow()` in `fiddle-nx-rust.js` answers only when the frame's address is the player's, so another Frame language's page sends a run down the first-run path, which returns the player's address. The player page posts `stopped` when its module ends; the script answers whoever waits at once and raises a generation that `NxRustLanguage` puts into the address (`?g=1`), so the next run the engine shows replaces the frame. Browser checks cover the stopped player (replaced, then draws). The swapped-frame case has no browser check: with only this language registered the engine's own address does not change, so the swap cannot be reproduced through the page.
- **Verification:** `frameWindow()` reads the frame's address (`fiddle-nx-rust.js:29-37`), a run
  with no live player takes the first-run path (`:209-212`), `stopped` answers whoever waits and
  raises the generation (`:74-83`), and `NxRustLanguage.FrameFor` puts it in the address
  (`NxRustLanguage.cs:30-39`). Probed in the dev host: with `about:blank` put in the frame,
  `fiddleNxRustRun` answers in 14 ms with `success: true`, where it waited 15 s. A real stop (the
  snippet of RF15 overflows the stack during layout) posts `stopped`, and the next run shows
  `player.html?gestures=lock&g=2` and draws; the browser check's two checks pass. Two notes that do
  not reopen it: one stop raised the generation twice (RF21), and the first case is still verified
  by simulation only, as the Fix note says.

### ✅ Verified - RF4 A run made before the frame is ready reports success for a program the player rejects
- **Severity:** Medium
- **Evidence:** `fiddleNxRustRun` returns `{ success: true }` whenever the frame is not up yet
  (`fiddle-nx-rust.js:174-179`), and a failed `result` nobody awaited goes to the console only
  (`:51-52`). The spec says a program that fails to link or evaluate "SHALL fail with the runtime's
  diagnostic under the editor" (`spec.md:47-51`) and that deep recursion "SHALL fail with the
  runtime's resource-limit diagnostic" (`:184-187`). That holds from the second run on. Probe: a
  fresh `/app`, `fiddle.setCode(<runaway recursion>, 'nx-rust')`, one run: `fiddle.getState()`
  says `✓ Compiled and rendered` with no errors, while the frame shows `nx-ir-resource-limit` and
  the console carries the line. The first run of a page is the one that opens a share's editor
  link, a preset, or a language switch. The browser check passes because its failing snippets are
  never a page's first run. `docs/ADDING-A-LANGUAGE.md` describes the behavior; the spec and the
  Done note of task 6.4 do not.
- **Recommendation:** When a failed `result` arrives that no run awaited, ask the engine for
  another run (the frame is ready by then, so that run awaits the result and lists the error), or
  give the page a way to take errors after `RunAsync` has returned. Otherwise narrow the two
  scenarios to say where a first run reports. Add a browser check whose first `nx-rust` run fails.
- **Fix:** When a failed `result` arrives that no run awaited, the script asks the editor to run again (`window.fiddle.run()`), once; that run finds the frame ready, awaits the answer and lists the error under the editor. Outside the editor (the `/p/` player) the frame shows the message itself. Browser check: a fresh page whose first `nx-rust` run is a runaway recursion ends with `nx-ir-resource-limit` in `fiddle.getState().errors`. `docs/ADDING-A-LANGUAGE.md` describes it.
- **Verification:** The listener asks for one run when a failed `result` finds nobody waiting, and
  not in the `/p/` player (`fiddle-nx-rust.js:58-73`); that run finds the frame ready and awaits
  its answer, so it cannot ask again. The browser check `a page's first run lists what the player
  found` passes (25 of 25). The run it asks for is a user run, which the engine follows with a run
  of its own (RF16); that one awaits its answer too.

### ✅ Verified - RF5 `RefreshIndicator` is listed as "given to its scroll" but nothing does that, and using one replaces the scroll's content
- **Severity:** Medium
- **Evidence:** `generate-controls.mjs:37-40` puts `RefreshIndicator` in `SPECIAL` ("Catalog
  components the player handles itself, outside the table") and `unsupported.json:6` carries the
  reason "given to its scroll, not drawn as a control". No code in `nx/rust/src` mentions it.
  `build` finds no constructor, draws a red `NX: no RefreshIndicator` label, and a `SkiaScroll`
  takes its first child as its content. Probe: `<SkiaScroll><RefreshIndicator>…</RefreshIndicator>
  <SkiaLayout>…</SkiaLayout></SkiaScroll>` draws only the red label; the layout is dropped
  ("SkiaScroll draws 1 of the 2 controls inside it"). The spec requires every catalog control to
  be drawn by the DrawnUI for Rust control of its name; the list only excuses properties and
  events.
- **Recommendation:** Give the indicator to the scroll (DrawnUI for Rust's scroll has a refresh
  indicator next to `on_refresh`), or, until then, leave a `RefreshIndicator` out of what the
  scroll is built with, so the content stands, and state the true reason in the list. Add the
  snippet to `tests/snippets`.
- **Fix:** The interim the report offers: a `RefreshIndicator` is left out of the drawing where the render is walked, so the scroll keeps its content, and is named once in the console. `unsupported.json` now gives the true reason. Test: `a_refresh_indicator_is_left_out_and_the_scroll_keeps_its_content`. Giving the indicator to the scroll is not done.
- **Verification:** Reopened for one case; what is left is Low. The case in the finding is fixed:
  `refresh.nx` draws `content` and says the one line (the test passes), and `unsupported.json`
  gives the true reason. But `render` asks whether a value is a `RefreshIndicator` before it visits
  a descriptor (`nx/rust/src/lib.rs:823-826`, `:854-857`), so an indicator that a component renders
  is not seen. Probe (scratch test): with `component <Pull extends DrawnNode /> = {
  <RefreshIndicator><SkiaLabel Text="pull" /></RefreshIndicator> }` and `<Pull />` as the first
  child of the `SkiaScroll` of `refresh.nx`, the only label drawn is the red `NX: no
  RefreshIndicator`, the layout is dropped, and the console has the three lines of the original
  report (`draws no RefreshIndicator`, `RefreshIndicator draws 0 of the 1 controls inside it`,
  `SkiaScroll draws 1 of the 2 controls inside it`). Leave the indicator out by what a place
  rendered to, after the visit: test the result of `render` in the array and property branches, or
  filter where the children of a record are read (`child_records`). Add the snippet to the test.
- **Fix:** (second pass) A control that is left out is now turned into nothing where the walk meets its record, so it does not matter whether the snippet wrote it or a component rendered it: `render` has one branch for `LEFT_OUT` in place of the two tests made before a visit. The empty place this leaves in a control's children showed a second fault: `patch` and `rebind` cut a non-container's children by position, so an empty place ahead of a scroll's content removed the content. They now take the values that hold the first `capacity` controls (`holding`). `refresh.nx` has a second scroll whose indicator a component draws, and the test expects both contents and the one line.
- **Verification:** (2026-10-09 13:02) `render` has one branch for a control that is left out
  (`nx/rust/src/lib.rs:855-860`), and it meets the record wherever the walk does, a component's
  output included. The test passes with both scrolls, and the snippet that reopened this
  (`<Pull />` ahead of the content) now draws `content` and says the one line. The second fault
  was real: the old code cut a non-container's records by position, so `[nothing, content]` gave
  the scroll nothing. `holding` (`:919-925`) takes the values up to the control after the last one
  held, and `patch` and `rebind` use it (`:569`, `:620`). Probe (scratch test): a scroll whose
  first place is an `if` holding a `RefreshIndicator`, toggled by a button inside the content,
  keeps the same scroll control and its content through off, on and off again, the content's
  buttons answer each time, and no control is made after the first six. Nothing else counts places
  by position: `children` (`:419-443`), `rebind` (`:610-613`), `takes` (`:721`), `build` (`:455`)
  and the span index (`:588`, `:738-739`) all count the records, and the generated `make` is
  handed built controls only.

### ✅ Verified - RF6 `runtime:rust -- --out` names the directory it deletes, where `runtime -- --out` names the web root
- **Severity:** Medium
- **Evidence:** `dev/build-nx-rust-player.mjs:20-21` takes `--out` as the output directory itself
  and `:52` runs `rmSync(OUT_DIR, { recursive: true, force: true })`. The sibling command takes the
  host's web root (`dev/build-runtimes.mjs:19-24`, `AGENTS.md:50-52`: "each runtime into its own
  subdirectory of that web root"). The change's own wording is the web root too ("built … into the
  host's web root", "`<web root>/nx-rust/`"). `npm run runtime:rust -- --out <host>/wwwroot`,
  written by analogy, deletes the host's whole web root after a build of several minutes. `--out`
  with no value throws a `TypeError` instead of the message the sibling prints.
- **Recommendation:** Take the web root, as `build-runtimes.mjs` does, and write into
  `<out>/nx-rust`, refusing a missing value with the same message. Update
  `docs/ADDING-A-LANGUAGE.md:230` and the script's header.
- **Fix:** `--out` names the host's web root, as for `npm run runtime`; the player goes into its `nx-rust` subdirectory, which is the only thing emptied. `--out` with no value prints the sibling's message and exits 2 (run). The script's header and `docs/ADDING-A-LANGUAGE.md` say so.
- **Verification:** `--out` is read as the web root and only `<web root>/nx-rust` is removed
  (`dev/build-nx-rust-player.mjs:23-29`, `:60`). Run: `node dev/build-nx-rust-player.mjs --out`
  prints the sibling's message and exits 2 before it looks for a toolchain. The header (`:4-8`) and
  `docs/ADDING-A-LANGUAGE.md` agree. The build itself was not run with `--out`.

### ✅ Verified - RF7 The cell tree only grows: `inert` has no end without `begin`, and the README teaches the same pattern
- **Severity:** Low
- **Evidence:** `InstanceTree::visit` appends to `inert` on every visit under no node, also when
  the descriptor is unchanged (`crates/nx-ir-runtime/src/tree.rs:355`, `399`), and only `begin`
  clears it (`:125-128`). The player never calls `begin` on the cell tree, so a template that
  hands a component a handler (`<Row onPicked={<Log … />} />`, probed) adds an entry at every
  bind of every cell, for as long as the program stands, and `atomically` clones the vector at
  every dispatch in a cell. `inert_told` (`lib.rs:610-615`) exists to step over that growth. The
  new README section says "`begin` and `finish` are never called on this one"
  (`crates/nx-ir-runtime/README.md:383`) and "reports it through `inert`" (`:396`) with no word on
  it. Two smaller costs of the same design: `drop_cell_nodes` calls `remove` once per root and
  each `remove` scans every node (`tree.rs:616-629`), so dropping a list's nodes is quadratic in
  the cells ever bound; and a component nested in a cell's component is never dropped when the
  output stops having its place, so it comes back with its old state, unlike in the drawing's
  tree.
- **Recommendation:** Call `begin()` on the cell tree before each bind: it only clears the record
  of a pass, and then `inert()` is what this bind found and `inert_told` can go. Say so in the
  README section, or add a way to take the list. Note the removal cost there, or drop the cell
  tree whole when a list's nodes are all it holds.
- **Fix:** The player calls `begin()` on the cell tree before each bind, so `inert()` is what that bind found; `inert_told` is gone. The README section says to do so and why, and the runtime test follows the same pattern. The README also states the two costs that stand: `remove` looks at every node, and a component nested in a cell's component is dropped only by `remove`. Neither cost is changed in code.
- **Verification:** `cell_render` calls `begin()` on the cell tree before the template's output is
  visited and reads `inert()` after (`nx/rust/src/lib.rs:617-627`); `inert_told` is gone. The
  README section says to call `begin` and why, its example does, and it states both costs
  (`crates/nx-ir-runtime/README.md:353-356`, `:378-379`, `:407-411`); the doc tests pass (10, 1
  ignored). `nodes_a_host_holds_outside_a_pass_stay_until_it_removes_them` follows the pattern and
  passes, and `a_handler_a_template_binds_is_reported_inert_once_and_its_cell_draws` still reads
  one line for three cells.

### ✅ Verified - RF8 The gestures dropdown and the background reach the player only at the next run of changed code
- **Severity:** Low
- **Evidence:** `OnGesturesChanged` and `OnCanvasBgChanged` (`FiddlePage.razor:1340-1358`) act on
  the .NET and React surfaces only, and `RunInFrameAsync` takes a new address only for a user run
  or other code (`:3148-3152`). Probe: three seconds after the dropdown changed, the frame's
  address is still `player.html?gestures=lock`; it changes after the next run with edited code.
  `NxRustLanguage.cs:24-25`, `docs/ADDING-A-LANGUAGE.md:218-219` and `design.md` ("changing the
  dropdown reloads the frame") say otherwise. The Done note of task 6.4 says the gestures mode
  "does not yet" reach the player, which the code has since overtaken.
- **Recommendation:** Run again when either setting changes while a Frame language is selected
  (one line in each handler), or say in the three places that the setting applies from the next
  run.
- **Fix:** The second option: the three places now say that the mode, and the background, reach the frame at the next run the engine shows (a user run or a run of changed code). The engine's handlers are not changed, because they also serve upstream's Rust language.
- **Verification:** The three places now say what the code does: `NxRustLanguage.cs:24-29`, the
  last bullet of section 3d in `docs/ADDING-A-LANGUAGE.md`, and "What the page reads once goes into
  its address" in `design.md`; the Done note of task 6.4 agrees. The engine's condition is
  unchanged (`FiddlePage.razor:3151-3152`).

### ✅ Verified - RF9 `Player` forwards only some of `App`, so accessibility calls from the web host are dropped
- **Severity:** Low
- **Evidence:** `impl App for Player` (`nx/rust/src/player.rs:89-165`) forwards 19 methods by hand.
  `App` has defaults that do nothing for the rest, so `prepare`, `page_font`, `back`, `location`
  and the seven `accessibility_*` methods never reach the `Ui`. The web host calls
  `accessibility_activate` and `accessibility_focus` (`drawnui/src/host_web.rs:269`, `275`), and
  the table sets the catalog's `Accessibility*` properties, so a control can be labelled for a
  screen reader and then not be activated by one. A method DrawnUI adds later is lost the same
  way, silently.
- **Recommendation:** Forward every method of the trait (a test that fails when `App` gains one
  would keep it so), or take the image inside the `Ui` and hand the `Ui` itself to `drawnui::run`.
- **Fix:** `impl App for Player` forwards every method of the trait, the eleven with defaults included (`page_font`, `prepare`, `back`, `location` and the seven `accessibility_*`), with a comment that a method DrawnUI adds must be added. No test fails when `App` gains one.
- **Verification:** Compared by script: `pub trait App` in `drawnui 0.1.0-preview.6`
  (`src/lib.rs`) declares 30 methods and `impl App for Player` forwards the same 30
  (`nx/rust/src/player.rs:88-197`). Nothing fails when the trait gains one, as the Fix note says;
  the comment above the `impl` is the only guard.

### ✅ Verified - RF10 "Every catalog member is accounted for" holds only for what the generator looks at
- **Severity:** Low
- **Evidence:** The list is an output of the same pass that builds the table, so the closing check
  (`generate-controls.mjs:322-336`) cannot fail, as the Done note of task 2.2 says. What it does
  not look at reaches an author only as a console line at run time:
  - Union cases with no enum variant. A catalog case is matched by name against the Rust enum
    (`:375-385`). `Squricle` and `Custom` (shape type), `FillWords`, `FillWordsFull`,
    `FillCharacters`, `FillCharactersFull` (text alignment) and `Shimmer` (touch animation) have no
    variant, so `Type=Squricle` is "does not take" at run time and is in no list.
  - Structural members. `Children`, `ItemsSource` and `ItemTemplate` are skipped for every control
    (`:56`, `:268`), also where the control cannot take them (`SkiaScrollBar`).
  - `TextSpan`. Its 16 properties are converted by hand in `values.rs:373-396` and the generator
    skips the component, so a property the catalog adds is dropped with no line at all.
  - Hooks. A hook whose closure or `impl` header the patterns do not match is passed over
    (`:191`, `:200`), not refused as an unreadable property block is.
  Also dead: `DECLINED` is empty, and `:320` assigns `undefined` where nothing was set.
- **Recommendation:** Compare each used union's cases with the enum's variants and list the
  missing ones; list a structural member a control cannot take; check `TextSpan`'s declared
  properties against a list in the generator; fail on an `on_*` hook it cannot read. Remove the
  two dead pieces.
- **Fix:** The generator now lists, in a new `values` section of `unsupported.json`, the cases of a catalog union that the enum a property is held in has no variant for; lists `Children`, `ItemsSource` and `ItemTemplate` for the controls that cannot take them; fails when the catalog's `TextSpan` properties differ from the ones `values.rs` converts; and fails on a control's `on_*` hook it cannot read. `DECLINED` and the `undefined` assignment are removed.
- **Verification:** Read in `generate-controls.mjs`: `noteCases` (`:263-273`, called at `:328`),
  the structural members listed where a control cannot take them (`:295-306`), the `TextSpan`
  check (`:356-365`; `SPAN_PROPERTIES` holds the 16 properties `values.rs:378-393` converts), and
  the failure on a hook that cannot be read (`:209-211`). `unsupported.json` has a `values` section
  with the seven cases the finding named, and `Children` and `ItemsSource` listed for
  `SkiaScrollBar` and `SkiaScroll`. `generate-controls.mjs --check` passes (run by `npm run
  test:nx:rust`). A hook under an `impl` header of a form the generator does not know is still
  passed over (`:208`); listed by script, those are four today (`Cx::on_finished`, two effects'
  `on_compilation_error`, `TextSpan::on_tapped`), none a control's. Not exercised: the two new
  failures, which need another catalog or another crate source to meet.

### ✅ Verified - RF11 Scenarios and risky paths with no test
- **Severity:** Low
- **Evidence:**
  - "An event for a drawing that was replaced changes nothing" (`spec.md:139-141`) and task 4.2's
    "a stale token leaving the drawing unchanged" have no test; `unbind` (`lib.rs:588-594`) is
    exercised only by chance.
  - Task 4.1 asks for one event per hook signature. The native tests fire `bool` and no-argument
    hooks. `usize`, `Option<usize>` (with its -1), `f32` and `&str` payloads were checked by hand
    in Chrome once, and `Option<usize>` (`CurrentIndexChanged`) not at all.
  - A second program after a first one is tested for replacing the drawing, not for what it
    keeps (RF2), and nothing tests the first run of a page (RF4) or recursion through components
    (RF1).
  - `tests/snippets/deep.nx` is compiled by `make-images.mjs` and used by no test.
  - `every_preset_draws` guards with `names.len() >= 6` where there are 11 presets.
- **Recommendation:** Add a test that fires an event between a dispatch and its drawing on a
  control the render replaces; fire a carousel, a slider and an editor event natively; delete
  `deep.nx` or use it; assert the preset count the C# file has.
- **Fix:** Added `an_event_on_a_control_the_next_drawing_replaces_changes_nothing` (shown to fail when replaced controls keep their handlers) and `a_number_a_text_and_an_index_reach_their_handlers` (`f32` from a slider, `&str` from an editor, `usize` from a carousel). The tests behind RF1, RF2 and RF4 are listed there. `deep.nx` is deleted, and `every_preset_draws` asserts the number of presets the C# file declares. Still not fired natively: `Option<usize>` (`CurrentIndexChanged`).
- **Verification:** Both tests are there and pass (`nx/rust/tests/player.rs:459`, `:474`; 23 pass,
  1 ignored). The first has teeth by reading: an event on the scroll that the next drawing replaces
  would dispatch a retired token and put `nx-ir-handler-token` in the console the test requires to
  be empty. `deep.nx` is gone, and `every_preset_draws` compares the images with the count in
  `FiddlePresetsNx.cs` (`:189-190`). Still unfired natively, as the Fix note says: `Option<usize>`
  with its -1 (`nx/rust/src/values.rs:235-239`). The paths this pass found untested are in RF14 and
  RF15.

### ✅ Verified - RF12 Notes in the artifacts that the code no longer matches
- **Severity:** Low
- **Evidence:**
  - Task 8.2 is checked for "taps, a live image swap, nested components, a templated list, the
    stack probe; verify it passes with `--quick`". `browser-check.mjs` has no `--quick`, no nested
    components and no templated list (the Done note covers only the stack probe).
  - Task 6.4's note on gestures (RF8). Tasks 8.1, 7.2 and 8.2 give 11 tests and 16 or 17 checks;
    there are 17 tests and 20 checks.
  - `design.md` Risks still says "4.78 MB of WebAssembly" and the alternatives say "362
    properties", against 6.4 MB and 371 elsewhere in the same file; `proposal.md` gives the
    spike's size as the player's. "At each call into the runtime" for the stack budget is once
    per pass (RF1).
  - "A failing template … SHALL be reported in the console with the template and the index"
    (`spec.md:151`): the line names the control, `the ItemTemplate of a SkiaStack failed for item
    2`, not the template function.
  - `AGENTS.md` (fiddle): "so both commands need one" follows a list of four commands.
- **Recommendation:** Correct the notes before the change is archived, and either name the
  template function in the line (the function record carries it) or reword the requirement.
- **Fix:** Corrected: task 8.2's description and the counts in the task notes; `design.md` (6.4 MB, 371, the gestures paragraph, and what the review changed); `proposal.md`'s size; the fiddle's `AGENTS.md`. The stack budget is now set at each call, as the design says. The failing-template line names the template's function: `the template Outer of a SkiaStack failed for item 2`.
- **Verification:** Each item read where it was corrected: task 8.2 and the counts (23 tests and 25
  checks are what ran today), `design.md` (6.4 MB, 371, the gestures paragraph), `proposal.md`'s
  size, the fiddle's `AGENTS.md`, the stack budget set at each visit (`nx/rust/src/lib.rs:833`),
  and the template's function name in the line (`:642-647`, asserted at `tests/player.rs:316`).
  Notes that have drifted since these corrections are RF18.

### ✅ Verified - RF13 Small simplifications in the player
- **Severity:** Low
- **Evidence:** `list_of` (`lib.rs:802-805`) is `list` under another name, there because a local
  is called `list`. `patch` copies a templated control's whole collection (`lib.rs:513`,
  `.to_vec()`) before comparing it with the one it holds, on every render of the component that
  owns the list; for the 100,000-item preset that is a copy and a comparison per event. `Inner`
  keeps every console line of a program (`console`, never trimmed) so that tests can read them,
  and `Player` keeps an index into it.
- **Recommendation:** Rename the local and drop `list_of`; compare the held `List` with the
  borrowed items and template first and copy only when they differ; drain `console` into `SAID`
  on publish outside tests, or cap it.
- **Fix:** `list_of` is gone. `patch` compares a templated control's collection with the one held where it is, and copies it only when it changed. The player drains the console as it hands lines to its page (`NxApp::take_console`), and keeps no index into it.
- **Verification:** `list_of` is gone. `patch` compares the held list with the borrowed items and
  template and copies only when they differ (`nx/rust/src/lib.rs:525-535`). The player takes the
  lines as it publishes them and keeps no index (`:215-217`, `nx/rust/src/player.rs:76-81`). The
  pass still copies a collection once per render where it meets `ItemsSource` (`lib.rs:849-853`),
  which the finding did not ask to change.

## New Findings Discovered During 2026-10-09 12:31 Review

**Reviewed in this pass:** the fixes for RF1 to RF13 as new code, in both working trees; the
component depth (`crates/nx-ir-runtime/src/tree.rs`, `eval.rs`, `error.rs`, `lib.rs`, the two new
tests and `descend` in `crates/nx-codegen/src/ir_instance_tree_tests.rs`, the README's limits
table, "Nesting ends" and *Small stacks*, `docs/nx-ir-format.md`); the two new spec deltas and the
new scenario; task 9.5 and the design notes; `proposal.md` and `tasks.md` against the code as it
stands; in the fiddle, `nx/rust/src/lib.rs`, `player.rs`, `web/player.html`, `fiddle-nx-rust.js`,
`NxRustLanguage.cs`, `generate-controls.mjs`, `dev/build-nx-rust-player.mjs`, `tests/player.rs`,
the snippets, `browser-check.mjs` and the four documents. `FiddlePage.razor`,
`fiddle-intellisense.js` and the engine's `RustLanguage.cs` were read for how a run reaches a
Frame language.

**What was run:** `cargo test -p nx-codegen ir_instance_tree` (42 pass), `cargo test -p
nx-ir-runtime --doc` (10 pass, 1 ignored), `cargo clippy --workspace --all-targets -- -D warnings`
and `cargo fmt --check -p nx-codegen -p nx-ir-runtime` (clean), `openspec validate
add-nx-rust-to-drawnui-fiddle --strict` (valid), `npm run test:nx:rust -- --nx ../nx` (23 pass, 1
ignored), `npm run test:nx -- --nx ../nx` (99 pass), `nx/rust/browser-check.mjs` (25 checks pass).
Findings marked "probe" were reproduced with scratch snippets, a scratch test crate that depends
on the player and the runtime by path, Playwright scripts against the running dev host, and a
scratch copy of `openspec/` for the archive, all under the session scratchpad (`review2/pass2/`).
Nothing in either repository was changed but this file.

### ✅ Verified - RF14 After an edit or a second Run, a control that stands is no longer said to have an unsupported property, an unbound event or dropped children
- **Severity:** Medium
- **Evidence:** Since the fix for RF2 a load keeps the controls, and three kinds of report are made
  only when a control is built or a value changes. `patch` skips a property the control was last
  given the same value for (`nx/rust/src/lib.rs:501`) and remembers the value whether or not the
  control took it (`:512`), so `does not set` and `does not take` (`:507-510`) are said once per
  control, not once per program. `has no … event` (`:467-469`), `draws N of the M controls inside
  it` (`:470-472`) and `draws no X` (`:455`) are said in `build` alone. `load` forgets what was
  said (`:275-276`) and builds nothing where a control stands. Probe (scratch test, the
  repository's own images loaded twice, as `a_list_shows_the_new_programs_items_after_an_edit`
  loads one): `unsupported` says `NX: DrawnUI for Rust does not set SkiaLabel.IsParentIndependent.`
  at the first load and not at the second; `events` says `SkiaShape has no ChildTapped event` at
  the first and nothing at the second. In the dev host: Run on `unsupported.nx` lists the line, Run
  again does not (a user run clears the console, `FiddlePage.razor:3039`), and neither does any
  later run that leaves the label alone. The scenarios "An unsupported property is named once" and
  "An unsupported event is named once" (`spec.md:101-104`, `:143-146`) and `README-NX.md:143-146`
  say a snippet that sets one is told. What `bind`, `children` and the pass report (an inert
  handler, a value that is no control, a control left out) is said again at each load, as it
  should be. The two tests that cover the reports load their snippet once.
- **Recommendation:** Do not remember a value the control did not take (`at.props.insert` for
  `Done` and `Gone` only), so it is offered and named again, once per program through `say_once`.
  Keep on `Drawn` what its build said (the events with no hook, the count it could not hold) and
  say it again from `patch`. Load each of `unsupported` and `events` a second time in its test.
- **Fix:** A value the control did not take is no longer remembered, so it is offered and named at every patch, once per program through `say_once`. What the build found is kept on `Drawn` (`missing`, `unbound`) and said from `patch`, with the count of controls a control cannot hold, which `patch` reads from the record. `build` says nothing now. `patches` counts properties that were set. Tests: `what_cannot_be_drawn_is_named_once_and_the_rest_stands` and `an_event_carries_its_payload_and_an_event_with_no_hook_is_named_once` load their snippet again and expect the same lines.
- **Verification:** (2026-10-09 13:02) `patch` makes the three reports from what `Drawn` keeps and
  from the record (`nx/rust/src/lib.rs:506-517`), a value is remembered only when it was set
  (`:524-534`), and `build` says nothing. Both tests load twice and pass (24 pass, 1 ignored).
  Probed natively (the lines of `unsupported` and `events` are the same at the second load) and in
  the dev host (three Runs of `unsupported.nx` list the line three times). On the unset path: a
  refused value is not in `props`, so nothing is reset when the render stops setting it, and a
  value taken earlier is still reset when the property goes (probe: `Type=Ellipse`, then
  `Squricle`, which is refused, then no `Type` gives `Rectangle`, with one more patch counted).
  The cost is one `apply`, one formatted line and one lookup in `said` per refused property per
  patch, and none where nothing is refused, which is every preset. `patches` counts what was set,
  and `the_counter_draws_and_a_tap_patches_it_in_place` still reads 6. One effect of a refused
  value is older than this fix and is RF23.

### ✅ Verified - RF15 A list whose template draws the same list nests without end: the player's bound and the component depth both start again in every cell
- **Severity:** Medium
- **Evidence:** `cell_render` starts each cell's walk at depth 0 (`nx/rust/src/lib.rs:616`) and
  visits a cell's components under no node (`:624`, `:833`), which the runtime counts as depth 1
  (`crates/nx-ir-runtime/src/tree.rs:340-343`). DrawnUI binds a cell during layout (`:256-258`,
  `:636`), so a templated control inside a cell is laid out, and binds its own cells, inside the
  layout of the cell that holds it. The comment on `MAX_NESTING` gives layout and paint sharing the
  stack as the reason for the bound (`:747-756`); through cells that depth has none. Probe:
  ```nx
  let ones = { for n in 1..=1 { n } }

  let <Cell Item:int Index:int />: DrawnNode =
    <SkiaLayout Type=Column>
      <SkiaLabel Text={"level " + Item} />
      <SkiaStack TItem=int ItemsSource={ones} ItemTemplate={Cell} />
    </SkiaLayout>

  <SkiaStack TItem=int ItemsSource={ones} ItemTemplate={Cell} />
  ```
  Natively the test process aborts (`has overflowed its stack`, SIGABRT). In the dev host, three
  runs of three: the run reports `✓ Compiled and rendered`, then the console carries `NX: the
  player stopped: Uncaught RangeError: Maximum call stack size exceeded`; the tab lives, and the
  next run gets a new player (`player.html?gestures=lock&g=2`) and draws. So the fix for RF3
  contains it, but "A snippet cannot take the page down" (`spec.md:179-182`) asks for a diagnostic
  under the editor and a player that stays loaded, and this gets neither. A finite drawing passes
  the bound the same way: 61 components, each in a layout, with a list at the bottom whose one cell
  nests 61 more, draws 125 nested layouts, where one walk is held to 128 levels, which is 64
  layouts each in a component (scratch `deepcells.nx`, drawn natively and in the dev host).
  `nested_cells.nx` shows lists in cells are meant to work.
- **Recommendation:** Carry the depth into cells. Keep on `List` the depth its control was walked
  at, which the walk knows where it meets `ItemTemplate`, and start a cell's `Walk` there; a list
  built inside a cell takes the depth of that cell's walk. A cell past the bound then fails as a
  failing template does (`the template Cell of a SkiaStack failed for item 0:
  nx-rust-drawing-nesting…`) and the rest of the drawing stands. Add the snippet to
  `tests/snippets` with a native test and a browser check. In the runtime README's "Cells a host
  binds outside a pass", say that the component depth counts from each cell's own root, so a host
  whose layout nests cells in cells bounds that itself.
- **Fix:** The walk leaves its depth on each templated control it meets (a `depth` property beside the catalog's, as `node` is left on a handler), `patch` keeps it on `List`, one deeper for the cell's own box, and `cell_render` starts the cell's walk there. A cell past 128 fails as a failing template does and the rest stands. Tests: `a_template_that_draws_a_list_of_itself_ends_at_the_depth_a_drawing_may_have` natively (`recursive_cells.nx`), and two browser checks (the cell is named, no `player stopped`, the same frame draws the next snippet). The runtime README's cells section says the component depth starts again in every cell. A new scenario in the spec.
- **Verification:** (2026-10-09 13:02) The walk leaves its depth on a record that has an
  `ItemTemplate`, counted with the record itself (`nx/rust/src/lib.rs:877-881`); `patch` keeps one
  more on `List`, also when nothing else of the list changed (`:553-564`); and `cell_render`
  starts the cell's walk there (`:642`). The native test passes, and the browser check's two
  checks pass (29 of 29). Probed natively: the snippet of this finding draws 42 levels and names
  the cell; a template that is the list alone draws 64 cells in cells; the finite drawing of the
  finding (61 levels, then a cell with 61 more) now has its cell refused; nothing aborts. In each
  template that draws itself the controls nested in DrawnUI come to 128 under the host.
  - *Whether the depth leaks.* Not found to. It is one of the properties `patch` keeps for itself
    (`:501-503`), so it never reaches a setter and is never remembered, and `takes` does not read
    it. Probe: a `SkiaScrollBar`, which draws no cells, given `ItemsSource` and `ItemTemplate` is
    told `does not set` of those two and nothing of `depth`. A component of the snippet's own with
    a property of that name is visited by the branch above and gets none.
  - *Whether 64 cells in cells is safely inside the stack.* Yes, in headless Chromium on x64, by
    a wide margin. Measured in the served player: the unused part of the 1 MiB WebAssembly stack
    was filled with a pattern from the page and its low-water mark read after the run, and a
    fraction of the JavaScript engine's own stack was used up by recursion before each frame. 64
    cells in cells peak at 100 KB of the 1 MiB; a scroll and a list per cell at 86 KB; this
    finding's snippet at 88 KB. All three draw with 75% of the engine's stack gone (the first also
    with 85%); at 90% the first and the third stop and the second still draws. So each needs at
    most a quarter of it, and the first under a sixth. The heavier case is one drawing without
    cells: 126 nested layouts peak at 400 KB, draw at 85% and stop at 90%. Not measured: another
    engine, a phone, a GPU backend.
  - The requirement's first sentence still says "under the editor", where the new scenario says
    the console; that wording is in RF22.

### ✅ Verified - RF16 Run starts the program twice, and any run of unchanged code starts it again: the script posts whatever it compiled
- **Severity:** Medium
- **Evidence:** The engine asks a Frame language to run also when the code is the code on screen
  (`FiddlePage.razor:3025-3030`), and after every user run it asks for diagnostics (`:3175-3176`),
  whose clean pass calls `AutoCompile` (`fiddle-intellisense.js:489`, `FiddlePage.razor:2749-2756`).
  `fiddleNxRustRun` compiles, replaces `latest` and posts every time
  (`fiddle-nx-rust.js:196-216`), and each `image` is a new program with new instances
  (`nx/rust/src/lib.rs:266-269`, `:294`). `NxRustLanguage` does not hand the script
  `context.UserRun` (`NxRustLanguage.cs:75`). The engine's own Rust language starts again only for
  a user run (`RustLanguage.cs:61-62`), and the page says of an auto run of the code on screen
  "keep the frame running, do not restart it" (`FiddlePage.razor:3150`). Probe (dev host): one
  `fiddle.run()` hands the frame two `image` messages. With the Welcome preset: Run, a tap half a
  second later prints `Clicked 1 times!`, and a tap three seconds after that prints `Clicked 1
  times!` again, because the count was reset in between. Under `nx` an auto run of unchanged code
  ends before it reaches the language (`:3026`), so the two languages differ here, and the
  difference is not among those `README-NX.md` lists. It is also why the probe of RF14 saw each
  console line twice after a Run.
- **Recommendation:** Pass `context.UserRun` to the script. For a run that is not the user's, when
  the frame holds a live player and the compiled image and the background are those of `latest`,
  post nothing and answer with what the frame said of that program. Add a browser check: Run, tap,
  wait two seconds, tap, and read a count of 2.
- **Fix:** `NxRustLanguage` passes `context.UserRun`, and `fiddleNxRustRun` hands the frame nothing for a run that is not the user's when a live player holds the same image and background: it answers with what the frame said of that program, which `latest.result` keeps. A user run starts the program again, as the engine's Rust language does. Browser checks: one Run hands the player one `image` (shown to count two with the condition switched off), and a count made after Run is still there 2.5 s later. That second check passes without the fix when the page's own run lands before the first tap, so the first is the one that proves it. The spec has a scenario. `README-NX.md` says, in its description and not among the differences, that Run resets a snippet's state as it does under `nx` and in the React and Rust languages: a probe in the dev host showed a Run of unchanged code resets the Welcome counter under both `nx` and `tsx`, so this is no difference from `nx`.
- **Verification:** (2026-10-09 13:02) `NxRustLanguage` passes `context.UserRun`
  (`NxRustLanguage.cs:76`), and the script answers a run that is not the user's from `latest`
  when a live player holds the same image and background (`fiddle-nx-rust.js:213-220`), keeping
  what the frame said where the result arrives and where a run stops waiting (`:61`, `:239`). The
  two browser checks pass. Probed in the dev host: one `fiddle.run()` hands the frame one `image`
  (two before); with the Welcome preset, Run, a tap, three seconds, a tap reads `Clicked 2
  times!`; a program the player rejects, run by the user, is still listed with its error four
  seconds later, with one `image` handed over.
  - *A result that has not arrived when the page's own run comes.* The engine does not start a
    run while another is under way (`FiddlePage.razor:3003-3008`), so a run that awaited has its
    result kept before the next begins. Only the first run of a frame returns without one. The
    page's own run is then answered as that run was, with success; the answer the frame gives
    later finds nobody waiting, and a failure asks the editor for a user run (`:67-70`), which
    hands the program over and awaits. So the error still reaches the editor. By reading; the
    timing was not reproduced.
  - *A share played through the .NET page.* By reading only: the frame is found under its player
    test id (`:30`), a failure is said in the console and not run again (`:68`), and a second run
    of the same share would be answered from `latest`. The dev host plays a share without .NET,
    so no check or probe goes this way.
  - (2026-10-09 14:58) The Fix note above was edited after this verification, to say that
    `README-NX.md` gives Run's reset in its description and not among the differences. That is
    right, and the edit says what it changed, so it stands. Probed in the dev host: after two
    taps, Run on unchanged code and one more tap prints `Clicked 1 times!` under `nx`, under
    `tsx` (the React Welcome) and under `nx-rust`.

### ✅ Verified - RF17 The two nesting diagnostics name a cause that may not be the cause, and the runtime's carries a key as long as the tree is deep
- **Severity:** Low
- **Evidence:** `visit` formats the key into the message and ends it with "A component that renders
  itself never ends." (`crates/nx-ir-runtime/src/tree.rs:351`). A host that keys nodes by their
  path, as the README's `walk` does (`crates/nx-ir-runtime/README.md:214-232`) and the player does
  (`nx/rust/src/lib.rs:837`), gets a key with a segment per level: for `recursive_bare.nx` the line
  under the editor is `NX: nx-ir-resource-limit: The 'N' at 'root/body/body/…` with a hundred
  `/body`, about 600 characters (dev host probe), and longer from a cell. The last sentence is said
  whatever the cause: for a root refused at a limit of 0 (scratch test), and for data that is
  finite and deep. The player's message ends the same way (`lib.rs:781`): a finite drawing 71
  components deep, which `nx` draws, is told "A component that draws itself never ends." (dev host
  probe). The spec delta requires the key in the message (`specs/component-instance-tree/spec.md:7-8`)
  and the test asserts `'Loop' at '101'` (`ir_instance_tree_tests.rs:328-331`).
- **Recommendation:** Keep the component and the limit. Give the key elided in the middle past a
  length, or name the node's parent instead; word the last sentence as a possibility ("as a
  component that renders itself would") or leave it out. Change the spec sentence and the
  assertion with it.
- **Fix:** The runtime's message no longer carries the key, which the host passed and has: `The 'N' component would be nested more than 100 component instances deep. A component that renders itself nests without end.` The last sentence states what such a component does and no longer says this drawing is one; the player's says the same of a component or a template. The spec delta asks for the component in the message, not the key, and the test asserts that.
- **Verification:** (2026-10-09 13:02) The runtime's message names the component and the limit and
  no key (`crates/nx-ir-runtime/src/tree.rs:343-352`); the delta asks for the component, and the
  test asserts `'Loop' component`. The player's says the same of a component or a template
  (`nx/rust/src/lib.rs:815`). Both read as a statement about such programs, not about this one.
  Seen in the dev host for `recursive_bare.nx`: one line of about 150 characters under the editor.
  `cargo test -p nx-codegen` passes (360). `design.md:258` still says the diagnostic names "the
  component and the key"; that is in RF22.

### ✅ Verified - RF18 The proposal and three task notes no longer describe the change
- **Severity:** Low
- **Evidence:**
  - `proposal.md:54-57` says "Modified Capabilities: None … this change adds a test and a README
    section for it, not a requirement." The change now carries `specs/component-instance-tree/spec.md`
    (a requirement added) and `specs/rust-ir-runtime/spec.md` (a requirement modified), and adds
    public API (`RuntimeOptions::max_component_depth`, `NX_DEFAULT_MAX_COMPONENT_DEPTH`). The
    Impact entry for this repository (`:66-70`) names neither.
  - `tasks.md:93`: "the 40 instance tree tests pass"; 42 do.
  - `tasks.md:42`: the Done note of 4.1 gives the native tests as `Tapped`, `Toggled`,
    `HoverChanged` and `Scrolled`; `a_number_a_text_and_an_index_reach_their_handlers` now fires
    `EndChanged`, `TextChanged` and `SelectedIndexChanged` too.
  - `tasks.md:92-93`: task 9.1 describes visits "with no `begin` or `finish`"; since RF7 the test
    and the README call `begin` before each bind, and the Done note does not say so.
  - The sentence that lists the limits' names is edited by this change in two places
    (`crates/nx-ir-runtime/src/error.rs:41-45`, `specs/rust-ir-runtime/spec.md:6-10`) and in both
    still lists the names shared with the TypeScript runtime without `maxInputSize`, which the
    runtime reports (`crates/nx-ir-runtime/src/input.rs:259`) and the README's table has. The
    omission is older than this change.
- **Recommendation:** List the two capabilities under Modified Capabilities and the option under
  Impact; correct the three notes; add `maxInputSize` to the sentence while it is open.
- **Fix:** `proposal.md` lists `component-instance-tree` and `rust-ir-runtime` under Modified Capabilities and the option under Impact. The notes of tasks 4.1 and 9.1 are corrected, and the counts are 42 tree tests, 24 player tests and 29 browser checks. `maxInputSize` is in the sentence in `error.rs` and in the delta.
- **Verification:** (2026-10-09 13:02) Each item read: `proposal.md` lists both capabilities under
  Modified Capabilities and the option under Impact; the Done notes of tasks 4.1 and 9.1 say what
  the tests do; `maxInputSize` is in `crates/nx-ir-runtime/src/error.rs:41-45` and in the delta.
  The counts in tasks 8.1, 8.2 and 9.1 are what ran today (24 player tests, 29 browser checks, 42
  tree tests), and the change validates strict. The Done notes of tasks 7.2 and 7.4 still give 25
  browser checks; that is in RF22.

### ✅ Verified - RF19 The delta for `component-instance-tree` archives cleanly only after `support-rust-hosts`
- **Severity:** Low
- **Evidence:** `openspec/specs/` has no `component-instance-tree` yet; `support-rust-hosts` adds
  it, and that change is not archived. Tried both orders on a scratch copy of `openspec/`. With
  `support-rust-hosts` archived first, the capability gets its Purpose and nine requirements, this
  change appends the tenth, `rust-ir-runtime` takes both changes' edits (they modify different
  requirements), and `openspec validate --specs --strict` passes all 73. With this change first,
  the archive also succeeds, but it creates `component-instance-tree/spec.md` with the Purpose
  `TBD - created by archiving change add-nx-rust-to-drawnui-fiddle`, and archiving
  `support-rust-hosts` afterwards warns `delta Purpose ignored; component-instance-tree already
  has one` and puts its nine requirements after the depth requirement. So `ADDED Requirements` is
  the right form for the delta, and the order matters. Task 11.2 finishes `support-rust-hosts`'
  tasks before this change's, but nothing says which is archived first.
- **Recommendation:** Say in `tasks.md` (group 11) that `support-rust-hosts` is archived before
  this change.
- **Fix:** Task 11.1 says `support-rust-hosts` is archived before this change, and why.
- **Verification:** (2026-10-09 13:02) The note is under task 11.1 (`tasks.md:114`) and says what
  the trial on a scratch copy showed: the other order leaves a placeholder Purpose and the depth
  requirement ahead of the nine that introduce the tree. The archive was not tried again.

### ✅ Verified - RF20 `README-NX.md` does not say that `nx-rust` refuses drawings `nx` draws
- **Severity:** Low
- **Evidence:** "What differs from NX" (`README-NX.md:141-160`) has no word on depth. Under
  `nx-rust` a drawing may nest 128 controls and components together (`nx/rust/src/lib.rs:756`) and
  100 component instances (the runtime's default); under `nx` neither bound exists. Probe (dev
  host): a component nested 71 deep, each use inside a `SkiaLayout`, draws under `nx` and fails
  under `nx-rust` with `nx-rust-drawing-nesting`. Nor does the section say that a
  `RefreshIndicator` is left out, which only `unsupported.json` records. The design says that
  where the two languages differ, this change records it.
- **Recommendation:** Add a bullet for the two bounds and their diagnostics, and one for the
  refresh indicator.
- **Fix:** `README-NX.md` has a bullet for the two bounds and their diagnostics, cells included, and one for the refresh indicator. (A third, on which runs start the program again, was removed after the review: Run resets state under `nx` too, so it is not a difference.)
- **Verification:** (2026-10-09 13:02) "What differs from NX" in `README-NX.md` has the three
  bullets, and each says what the player does: the two bounds, their diagnostics, a cell counting
  from its list and being named in the console; the indicator left out and said once; Run, Ctrl+S
  and an edit handing a program over, and the page's own run after a Run or an undone edit leaving
  it running. Section 3d of `docs/ADDING-A-LANGUAGE.md` has the matching point for a language's
  author.
  - (2026-10-09 14:58) The third bullet has since been taken out, and the Fix note edited to say
    so. The finding asked for the bounds and the indicator, which are there; the bullet on runs
    described something `nx` does too (probe under RF16), so its removal is right and this stays
    verified. The description's sentence, that Run resets a snippet's state as under `nx` and in
    the React and Rust languages, agrees with the probe for `nx` and `tsx` and with
    `RustLanguage.cs:61-62` for Rust.

### ✅ Verified - RF21 Small simplifications in the new code
- **Severity:** Low
- **Evidence:**
  - `nx/rust/src/lib.rs:332` wraps `tree.dispatch` in `atomically`; `dispatch` is `atomically`
    itself (`crates/nx-ir-runtime/src/tree.rs:470`).
  - `pass` runs inside `atomically` (`lib.rs:806`), and neither caller needs it to: `load` passes
    over a new tree it drops on failure (`:269-271`), and `dispatch` calls it inside its own
    (`:337-340`). An event clones the node map three times where once would do.
  - `visit` walks the parent's chain twice, once in `is_under` and once in `depth`
    (`tree.rs:331`, `:342`); one walk answers both.
  - Every uncaught error in the player's page posts `stopped` (`nx/rust/web/player.html:45-52`)
    and every `stopped` raises the generation (`fiddle-nx-rust.js:77-78`): one stop gave `g=2`
    (probe of RF15).
- **Recommendation:** Drop the two inner `atomically`; have one walk return whether the key is in
  the chain and how long the chain is; post `stopped` once per page.
- **Fix:** `pass` no longer runs inside `atomically`, and a cell's dispatch calls `dispatch` directly: an event copies the node map once. `visit` walks the parent's chain once (`depth_apart_from` answers both whether the key is in it and how long it is). The player's page posts `stopped` once.
- **Verification:** (2026-10-09 13:02) `pass` begins and ends a pass and nothing more
  (`nx/rust/src/lib.rs:835-842`); its two callers are `load`, over a tree it drops on failure, and
  the dispatch, inside its own `atomically` (`:278`, `:344-347`). A cell's dispatch calls
  `dispatch` (`:339`). `visit` makes one walk, `depth_apart_from`
  (`crates/nx-ir-runtime/src/tree.rs:327-343`, `:622-635`), and the scratch test of the limit's
  edges gives the results it gave before (0 refuses a root, 1 the second node, a re-parented node
  refused and unchanged, a held node past a lowered limit refused). The page says `stopped` once
  (`nx/rust/web/player.html:45-54`): two errors raised in the frame put one line in the console
  and the next run shows `player.html?gestures=lock&g=1`.

## New Findings Discovered During 2026-10-09 13:02 Verification

**What was run for this verification:** `cargo test -p nx-codegen` (360 pass), `cargo test -p
nx-ir-runtime --doc` (10 pass, 1 ignored), `cargo clippy --workspace --all-targets -- -D warnings`
and `cargo fmt --check -p nx-codegen -p nx-ir-runtime` (clean), `openspec validate
add-nx-rust-to-drawnui-fiddle --strict` (valid), `npm run test:nx:rust -- --nx ../nx` (24 pass, 1
ignored), `npm run test:nx -- --nx ../nx` (99 pass), `nx/rust/browser-check.mjs` (29 checks pass),
and the scratch probes named in the notes above (`review2/pass2/`), against the dev host and the
player as rebuilt at 12:43.

### ✅ Verified - RF22 Three places still say what the last fixes changed
- **Severity:** Low
- **Evidence:**
  - `design.md:258` says the runtime's diagnostic names "`maxComponentDepth`, the component and
    the key". Since the fix for RF17 the message has no key.
  - `tasks.md:73` and `:77` (the Done notes of 7.2 and 7.4) say "25 browser checks pass"; 29 do,
    as the note of 8.2 says.
  - The requirement "A snippet cannot take the page down" says a program that nests deeper than
    the player draws "SHALL end with a diagnostic under the editor"
    (`specs/fiddle-nx-rust-language/spec.md:192-194`). Its new scenario, and the code, have a cell
    past the bound left empty and named in the console while the run succeeds (`:209-213`).
- **Recommendation:** Drop "and the key" from the design note, give the two task notes the count
  of 29, and add to the requirement's sentence that a cell past the bound is left empty and named
  in the console.
- **Fix:** The design note names the limit and the component, the notes of 7.2 and 7.4 say 29 browser checks, and the requirement says a cell past the bound is left empty and named in the console while the rest of the drawing stands.
- **Verification:** (2026-10-09 14:58) All three read: `design.md:258` says the diagnostic names
  `maxComponentDepth` and the component; the Done notes of 7.2 and 7.4 (`tasks.md:73`, `:77`) say
  29 browser checks, which is what ran today; and the requirement
  (`specs/fiddle-nx-rust-language/spec.md:197-201`) now has the sentence on a cell, in step with
  its scenario. The note of 8.1 says 25 player tests, which is also what ran. The change validates
  strict.

### ✅ Verified - RF23 A value a control refuses leaves it showing the value it took before
- **Severity:** Low
- **Evidence:** `patch` offers a value, and on `Rejected` says so and goes on
  (`nx/rust/src/lib.rs:525-534`); the control keeps whatever an earlier render set, and `props`
  still holds that. Probe (scratch test): a shape drawn with `Type=Ellipse`, then rendered with
  `Type=Squricle`, which DrawnUI for Rust has no variant for, is told `SkiaShape.Type does not
  take "Squricle"` and stays an ellipse. Rendered with `Squricle` when no `Type` had been set just
  before, the same control is a rectangle, the default. So what a record draws depends on what
  stood there before. This is older than the fix for RF14, which made the bookkeeping true
  (`props` no longer claims the refused value) and left the control as it was.
- **Recommendation:** Treat a refused value as no value for the control: when `props` holds an
  earlier value for that property, reset it to its default and forget it, as the unset path does.
  Add the case to `a_property_no_longer_set_returns_to_its_default_and_children_come_go_and_change`
  or to the `unsupported` snippet.
- **Fix:** On `Rejected`, `patch` now resets the property to its default and forgets it when an earlier render had set it, so a record draws the same whatever stood there before. Test: `a_value_a_control_refuses_leaves_the_default_not_the_value_before_it` (`refused.nx`: `Ellipse`, then `Squricle`, is a rectangle and the same control). A scenario in the spec.
- **Verification:** (2026-10-09 14:58) On `Rejected`, `patch` says the line, and where `props`
  held an earlier value it forgets it and sets the default (`nx/rust/src/lib.rs:531-539`); the
  property is then no longer among those the unset path looks at, so it is reset once. The new
  test passes (25 pass, 1 ignored) and would fail on the old code, which left `Ellipse`. Probed
  with the four-step snippet of the finding: `Ellipse`, then `Squricle` gives `Rectangle` with two
  patches counted (the button's text and the reset); no `Type`, then `Squricle` twice more, each
  count one patch and stay `Rectangle`; the line is said once; loading the program again draws
  `Ellipse` and does not say it. The scenario "A value a control refuses draws as no value"
  (`spec.md:113-117`) says what the test shows.

## New Findings Discovered During 2026-10-09 14:58 Verification

**What was run for this verification:** `npm run test:nx:rust -- --nx ../nx` (25 pass, 1 ignored),
`nx/rust/browser-check.mjs` (29 checks pass), `openspec validate add-nx-rust-to-drawnui-fiddle
--strict` (valid), and two scratch probes (`review2/pass2/`): the refused-value test, and a
Playwright script that taps, runs unchanged code and taps again under `nx`, `tsx` and `nx-rust`.
The NX crates were not tested again; nothing in them was said to have changed.

**The four changes not tied to a finding:**
- *`README-NX.md`, Run's reset moved out of the differences.* Right; see the notes added under
  RF16 and RF20.
- *`FiddlePresetsNx.cs`, the `nx-rust` Welcome says what draws it.* It does: in the dev host,
  `nx-rust-welcome` reads "drawn by **DrawnUI for Rust**; the same engine draws **Rust** apps on
  mobile and desktop.", `nx-welcome` is unchanged, the browser check's "switching from NX keeps
  the snippet" still passes, and the player's tests draw the templates from the constants, which
  did not change. One thing follows from how it is done: RF24.
- *`tasks.md` 8.2, the note on the frame shared with the engine's Rust language.* Not verified,
  and not verifiable from the repositories: the dev host registers no Rust language and the rig
  is not kept. What the note describes agrees with the code as read under RF3 (`frameWindow()`
  reads the frame's address, and each script takes messages only from its own page).
- *The Questions section.* It says "None" for two questions the user is reported to have
  answered. Nothing in the answers as reported (Run starts the program again; drawfiddle.com will
  offer both languages) is at odds with the code; the answers themselves were not seen here.

### ✅ Verified - RF24 Nothing notices if the `nx-rust` Welcome stops being reworded
- **Severity:** Low
- **Evidence:** `NxRustAll` replaces one sentence of every template's code by its exact text
  (`src/DrawnUi.Fiddle/FiddlePresetsNx.cs:76-79`), and the sentence is in `NxWelcome` alone
  (`:102`). `string.Replace` changes nothing when the text is not found. So an edit to that
  sentence of the `nx` Welcome, which upstream owns, leaves the `nx-rust` copy saying it is drawn
  by DrawnUi.React on CanvasKit, with no test or check failing: the player's tests and
  `npm run test:nx` read the constants, not `NxRustAll`, and the browser check loads `nx-welcome`
  and switches. By reading; the replacement works today (probe above).
- **Recommendation:** Have `browser-check.mjs` load `nx-rust-welcome` and require that its code
  names DrawnUI for Rust and not CanvasKit, or assert in `NxRustAll` that the Welcome's code
  changed.
- **Fix:** `browser-check.mjs` loads `nx-rust-welcome` and requires its code to say it is drawn by DrawnUI for Rust and to name neither DrawnUi.React nor CanvasKit (30 checks now). The task notes give the new count.
- **Verification:** (2026-10-09 15:01) The check is at `nx/rust/browser-check.mjs:233-238`, and
  the browser check passes with it (all passed; the file has 30 checks, and the notes of 7.2, 7.4
  and 8.2 say 30). It would fail if the rewording stopped. Shown by running its two expressions
  over the `NxWelcome` constant, which is the text a copy that was not reworded has: that text
  fails the check, the same text with the sentence replaced passes, and the text with one word of
  the sentence changed, so that the replacement finds nothing, fails. No other template holds the
  sentence, and the reworded Welcome has no other mention of CanvasKit or DrawnUi.React to trip
  the second expression. A read of the buffer made too early could not pass it by mistake either:
  the page the check runs on holds the `nx` Welcome's code from the share before the template is
  loaded.

## Questions
- None

## Summary
- The instance tree is used correctly: dispatch and pass are one atomic change, handlers are read
  at dispatch time, the two trees are kept apart, a failed load or dispatch leaves the drawing and
  its instances as they were, and the frame protocol checks source and origin on both sides. The
  tests that exist test what their names say, and the new runtime test and README example are
  sound.
- Two requirements are not met: a snippet can take the page down through component recursion
  (RF1), and an edit does not keep what the engine holds for the controls (RF2). Both are
  reproduced and neither has a test.
- The page's side of the protocol trusts state it set once (RF3) and reports success before the
  player has answered (RF4). RF5 and RF6 are narrow but concrete.
- Not checked: the browser build itself (`npm run runtime:rust` was not run, so the toolchain
  messages and the sizes are unverified), the GPU measurements in `README-NX.md`, `cargo test
  --workspace` and workspace-wide clippy in this repository, `npm run runtime` without a Rust
  toolchain, and RF3's first case against a real Rust build (the frame swap was simulated).
- The release tasks (11.x) should wait for RF1 at least, since its fix may add to the runtime API.

### Second pass, 2026-10-09 12:31
- Verification: 12 of the 13 fixes are verified (RF1 to RF4, RF6 to RF13). RF5 is reopened for one
  case, an indicator that a component renders; what is left of it is Low.
- New findings: eight, RF14 to RF21. The three Medium ones are all in the fiddle and all
  reproduced: reports that an edit no longer repeats (RF14, which the fix for RF2 brought), cells
  that nest past every bound (RF15), and a program started twice by one Run (RF16). The five Low
  ones are wording, notes, archive order, a README gap and small simplifications.
- The component depth, to the questions asked of it. The count is right at the boundary: a root is
  1, the 101st node is refused, a limit of 0 refuses a root and a limit of 1 the second node
  (probed). Checking on every visit is right: it can only be seen when a host lowers the limit,
  and it costs one more walk of the parent's chain, which `is_under` already makes (RF21). Nothing
  grows the tree's depth past the check: `dispatch` makes no node, `atomically` only puts back
  what was there, and a node visited again under another parent is checked against that parent
  and loses what was under it. A refused visit returns before anything is written: the node, its
  output and the pass's record of it were as before (probed). The limits table, "Nesting ends",
  *Small stacks*, `docs/nx-ir-format.md`, `error.rs` and the two deltas agree with the code and
  with each other on the name, the default and what is counted; the one gap in those lists is
  older than this change (`maxInputSize`, in RF18). The delta archives cleanly after
  `support-rust-hosts` and badly before it (RF19). What the limit does not reach is a second tree
  whose nodes all sit under no node (RF15).
- No finding in: the check in `visit` and its two tests (re-parenting and the limits of 0 and 1
  are not in them, and behaved when probed), the README's cells example and doc tests, the build
  script, the generator, `Player`'s forwarding, and the protocol for a stopped player and a first
  run that fails, beyond RF16 and the double count in RF21.
- Not checked: `cargo test --workspace` (only the narrower commands were run, since one was said to
  be under way); `npm run runtime:rust` (the build the dev host served, made at 12:13, was used;
  it carries the runtime's limit, as the `recursive_bare` probe showed); RF15 and RF16 in a browser
  with a GPU (headless Chromium with software WebGL only); RF3's first case against a real Rust
  build; the generator's two new failure paths; `pnpm run verify:crates`; the `/p/` player beyond
  the browser check.
- None of the open findings adds to the runtime's API. RF17 changes a message, a spec sentence and
  an assertion, and RF15 adds a sentence to the runtime README, so both are better done before the
  release tasks (11.x) than after.

### Verification, 2026-10-09 13:02
- All nine fixes are verified: RF5 and RF14 to RF21. Every finding from RF1 to RF21 is now
  verified. Two Low findings are new, RF22 (three leftovers in the artifacts) and RF23 (a refused
  value leaves the earlier one showing, which is older than these fixes).
- To the four questions asked. The depth left on a templated control's record was not found to
  leak: no setter, no report and no comparison reads it. Not remembering a refused value is right
  on the unset path and costs one call and one formatted line per refused property per patch,
  nothing on the presets; RF23 is the one effect near it. A page's own run that comes before the
  earlier run's answer is answered as that run was, and a failure that arrives later still
  reaches the editor through the run the script asks for; that and the share played through the
  .NET page were judged by reading, not run. Cells at the bound are well inside both stacks in
  headless Chromium: about a tenth of the 1 MiB WebAssembly stack and at most a quarter of the
  JavaScript engine's; one drawing of 126 nested layouts is the heavier case at about four tenths
  of the WebAssembly stack.
- Not checked: `cargo test --workspace` beyond `nx-codegen` and the runtime's doc tests;
  `npm run runtime:rust` (the player the dev host served, built at 12:43, was used); another
  browser engine, a phone or a GPU for the stack measurements; a share played through the .NET
  page; the timing of a page's own run arriving before a first run's result; RF3's first case
  against a real Rust build; the archive order again; `pnpm run verify:crates`.

### Verification, 2026-10-09 14:58
- RF22 and RF23 are verified, so every finding from RF1 to RF23 is. One Low finding is new, RF24:
  the rewording of the `nx-rust` Welcome fails silently if the sentence it replaces is edited.
- The four changes made outside a finding were looked at as new work. Moving Run's reset out of
  "What differs from NX" is right: Run on unchanged code resets the Welcome counter under `nx`,
  `tsx` and `nx-rust` alike (probed). The Fix notes of RF16 and RF20 were edited after their
  verification; each says what changed, and a dated line under each Verification note records it.
  The reworded Welcome is what the dev host serves. The hand-run probe noted under task 8.2 and
  the user's answers behind "Questions: None" could not be checked from here.
- Not checked: the NX crates again (`cargo test`, clippy, `pnpm run verify:crates`), since nothing
  in them was said to have changed; `npm run test:nx`; a share played through the .NET page; the
  frame shared with a real Rust build; another browser engine or a GPU.

### Verification, 2026-10-09 15:01
- RF24 is verified: the new browser check fails on a Welcome that was not reworded. Every finding
  from RF1 to RF24 is verified and none is open. Run for this: `nx/rust/browser-check.mjs` (all
  passed) and the check's expressions against the `NxWelcome` constant. Nothing else was run.
