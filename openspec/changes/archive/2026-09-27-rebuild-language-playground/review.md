# Review: rebuild-language-playground

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/playground/spec.md, specs/sdk-wasm/spec.md,
specs/value-view/spec.md, specs/website/spec.md, specs/package-release-automation/spec.md,
specs/drawnui-nx-catalog/spec.md  
**Reviewed code:** the uncommitted working tree: `crates/nx-api/src/{nx_text.rs,eval.rs,diagnostics.rs,lib.rs}`,
`crates/nx-interpreter/src/{interpreter.rs,error.rs}`, `crates/nx-hir/src/lib.rs`, `crates/nx-cli/src/main.rs`,
`bindings/wasm/{native/build.rs,native/src/lib.rs,src/*,test/*,README.md}`, `packages/value-view/**`,
`sites/playground/{src/**,scripts/check-examples.mjs,worker/*,base.mjs,vite.config.ts,wrangler.jsonc,_headers,index.html,README.md}`,
`sites/website/{scripts/check-code-blocks*.mjs,src/expressive-code/*,starlight.config.mjs,astro.config.mjs,src/content/docs/**}`,
`.github/workflows/deploy-*.yml`, `docs/deployment*.md`, `specs/future.md`. Tasks 10.3 and 11.1 were out of
scope, as intended.  
**Checks run:** `cargo test -p nx-api -p nx-cli -p nx-interpreter`, `pnpm --filter @nx-lang/value-view test`,
`pnpm --filter @nx-lang/sdk-wasm test`, `sites/playground: pnpm test && pnpm run typecheck`,
`sites/website: pnpm test`, and `openspec validate rebuild-language-playground --strict` all pass. Behavior was also
checked with Playwright (Chromium) against the dev server at `http://127.0.0.1:5198/play`.

## Findings

### ✅ Verified - RF1 In a real browser, runaway recursion (and legitimate recursion about 500+ calls deep) crashes the compiler worker instead of reporting the recursion limit
- **Severity:** High
- **Evidence:** In Chromium, `/play#code=…` for `let f(n:int): int = { f(n + 1) }\nlet root() = { f(0) }` shows
  "The compiler crashed on this source. It has been restarted…", not the runtime error. A terminating
  `f(n)` that counts down to 0 evaluates at n = 200 and 400 but crashes the worker at 600, 800 and 990. The same
  module on the page's main thread in the same browser returns `NxEvaluationError: Stack overflow: recursion depth
  1000 exceeded`. So the 16 MiB linear-memory stack (`bindings/wasm/native/build.rs`) is enough, but the dedicated
  worker's native (engine) stack runs out first. That is the limit design.md says "the module cannot raise", and
  it is reached by the plain recursion the spec scenario names, not only by "much heavier frames". The spec
  scenarios "Runaway recursion is a runtime error" (playground) and "Runaway recursion is a diagnostic, not a
  trap" (sdk-wasm) hold only under Node: `sites/playground/src/worker/session.test.mjs:56` and
  `bindings/wasm/test/sdk-wasm.test.ts` run the session on Node's main thread, which has a larger stack than a
  browser worker. `sites/playground/README.md:23` and design.md claim the behavior the browser doesn't have.
- **Recommendation:** Make the interpreter's recursion limit reachable inside a browser worker. Either give the
  wasm entry points a lower `max_recursion_depth`, set from `nx-api`/the wasm build and measured in Chromium and
  Firefox workers with some headroom, or shrink the per-NX-call native frames enough that depth 1000 fits. Add a
  regression test that runs in a worker with a browser-like stack: a Playwright check in the playground, or at
  least Node `worker_threads` with `resourceLimits.stackSizeMb` set to a browser worker's size. Then correct
  design.md ("ran out about 220 calls deep … a build script links it with 16 MiB") and the README sentence.
- **Fix:** `bindings/wasm/native/src/lib.rs` evaluates with `max_recursion_depth` lowered to `MAX_RECURSION_DEPTH = 200` through the new `nx-api` entry point `eval_program_artifact_nx_text_with_limits` (the CLI and .NET keep 1,000). Measured in a Chromium dedicated worker through the playground: the engine stack ran out at 325 (a `for` in the body), 371 (`n + f(n - 1)`), 411 (element function), 516 (record argument) and 517 (plain countdown) calls. After the change every one of those shapes, and runaway recursion, ends in `Stack overflow: recursion depth 200 exceeded` in the worker. Removing the `eval_expr` location wrapper changed the measured depths only by noise, so frame size was not the lever. New `bindings/wasm/test/recursion-stack.test.ts` runs each shape at 199 and 250, plus runaway recursion, in a Node `worker_threads` worker with a 0.5 MB stack (countdown traps about 300 deep there with the old limit, stricter than Chromium's 517); with the limit set back to 1,000 both of its tests fail. A trap that is still a stack overflow (heavier calls, other engines) is now named: `session.ts` turns `NxHostCrashedError` whose cause is "Maximum call stack size exceeded" or "too much recursion" into `NxStackOverflowError`, which `compileFailureMessage` words and `retrying` does not retry (tests in `session.test.mjs` and `worker.test.mjs`). design.md, `bindings/wasm/native/build.rs`, `bindings/wasm/README.md` and `sites/playground/README.md` now describe both stacks and the 200 limit; the playground spec gains "Recursion too deep for the browser is named".
- **Verification:** Verified in Chromium on the rebuilt module. Runaway recursion now shows `Stack overflow: recursion depth 200 exceeded` at 1:23 with a marker. A countdown at 250 errors at the limit. Countdown, `for`, element-function, record-argument, mutual and heavier record-building recursion at 199 all evaluate. `recursion-stack.test.ts` passes under a 0.5 MB worker stack. A body that nests three `for`s per call still traps at 199, but it now reads "The program recursed deeper than the browser can run…". It is not retried, and the next example evaluates normally. That is the named-trap path the new scenario describes. The 200-vs-1,000 difference from `nxlang run` is not reflected in the specs' parity wording (RF16).

### ✅ Verified - RF2 The output pane shows "Evaluating…" indefinitely when the first source it gets does not compile
- **Severity:** Medium
- **Evidence:** `sites/playground/src/output/OutputPane.tsx:57` shows "Evaluating…" whenever `outcome === null &&
  failure === null`. When the first evaluation settles with a compile error, `useEvaluation` leaves `outcome` null
  and only clears `evaluating` (`useEvaluation.ts`, the `result.outcome === null` branch), so the message never
  changes. Reproduced: `/play#code=` for `type A = { x:int }\nlet root() = <A x="s" />` still reads "Evaluating…"
  after 30 s, with the real error listed only under the editor. This is exactly what the website's "Open in
  playground" link on every `nx invalid` block produces (website spec "An example of an error … show the error it
  produces"), and it also happens for a shared link to a broken program.
- **Recommendation:** Show "Evaluating…" only while `evaluation.evaluating` is true. When the source has settled
  with no outcome, say that it does not compile and point at the problems listed under the source. Add a unit test
  for the pane's states (see RF14).
- **Fix:** The output state moved into `sites/playground/src/evaluation.ts` (`reduceEvaluation`, `outputNotice`). The pane shows "Evaluating…" only before the first answer; once a source settles with compile errors and no earlier value, it says the program does not compile yet and points at the problems under the source. Tests in `src/evaluation.test.mjs`, and checked in Chromium on a fresh load of `/play#code=` for `type A = { x:int }\nlet root() = <A x="s" />`.
- **Verification:** Verified. `/play#code=` for `type A = { x:int }\nlet root() = <A x="s" />` now reads "The program does not compile yet, so there is no value to show. The problems are listed under the source." `evaluation.test.mjs` covers the first-source-does-not-compile case.

### ✅ Verified - RF3 Fold toggles are drawn over the value's text when a foldable node does not start its line, and nested nodes get two toggles on one line
- **Severity:** Medium
- **Evidence:** `.fold` is `position: absolute; left: -1.3em` relative to the node's own inline span
  (`packages/value-view/src/styles.ts:69`). That is a gutter only for a node that starts at the line's indentation.
  In the `element-functions` example, the toggles for `<li` (after `content={`), `<div` (after `content=`) and the
  second `<li` (after `/> `) are painted over `={`, `=` and `>`, and a line such as `content={<li` carries three
  toggles: the property, the sequence and the record (screenshot of `/play/element-functions`). The `yields` rule
  in `render.ts:64-67`, which would let a child take the toggle, can never fire. A child always starts after its
  parent's first character (`name=`, `{`, `<`), and the only node that could start at the same offset is the
  root, which already never folds. The test "gives a top-level sequence's toggle to its first record" passes
  because the root is excluded, not because of `yields`.
- **Recommendation:** Put the toggles in a real per-line gutter. Give each line at most one toggle, for the
  outermost foldable node that starts on that line, or only for nodes that start their line. Toggles for nodes
  that start mid-line then either go away or sit in the gutter. Remove the dead `yields` branch and rename its test.
  Add a DOM test on a nested value such as `element-functions` asserting one toggle per line.
- **Fix:** `render.ts` gives a toggle only to a record, property or sequence that spans lines, is not the whole value, and starts its line (`startsLine`), so every toggle sits in the gutter or the indentation and a line has at most one. The dead `yields` branch is gone. New DOM test "puts toggles only beside nodes that start their line, at most one a line" on the `nested` fixture (markup like `element-functions`), and the old test now asserts only that the whole value has none. Checked in Chromium on `/play/element-functions`: three toggles, three lines, none over text. The value-view spec and design now say a node that starts partway along a line folds with the node that starts it, with a new scenario.
- **Verification:** Verified. `startsLine` gates the toggle, and the dead `yields` branch is gone. On `/play/element-functions` there are three toggles on three different lines, all in the indentation and none over text (screenshot checked). The new DOM test runs on a compiler-produced nested fixture. The value-view spec and design record that a node starting partway along a line folds with its line's node.

### ✅ Verified - RF4 Links to `nxlang.org/playground` remain in the root README and the VS Code extension's README
- **Severity:** Medium
- **Evidence:** `README.md:6`, `src/vscode/README.md:19` and `src/vscode/README.md:44` still link to
  `https://nxlang.org/playground`. Nothing redirects, so after deploy these go to the website's 404 page. The VS Code README ships inside
  the VSIX and becomes the Marketplace and Open VSX listing, so it keeps the broken link until the next extension
  release. Task 4.3's grep covered only `.github` and `docs`.
- **Recommendation:** Point all three at `https://nxlang.org/play`, and widen the grep in the task to
  `git grep -n "nxlang.org/playground" -- ':!openspec'`.
- **Fix:** `README.md` and both links in `src/vscode/README.md` point at `https://nxlang.org/play`. Task 4.3's check is now `git grep -n "nxlang.org/playground" -- ":!openspec"`, which finds nothing.
- **Verification:** Verified. `git grep -n "nxlang.org/playground" -- ':!openspec'` finds nothing.

### ✅ Verified - RF5 Component values get no declaration and no declared property types, so hover and "go to declaration" don't work on them
- **Severity:** Low
- **Evidence:** `Declarations::new` (`crates/nx-api/src/nx_text.rs:494`) collects only `Item::Record`,
  `Item::Union` and `Item::Function`. In the shipped `components` example, `<Counter start=3 />` and
  `<Button label="Reset" />` have `record` nodes with no `declaration`. Their `start`/`label` properties take the
  value's type (`int`, `string`) rather than the declared one, and have no declaration or `optional`, even though
  the source declares `component <Counter start:int = 0 />` and `external component <Button label:string … />`.
  Clicking `Counter` in the output does nothing, and hovering it shows only the default. The playground scenario
  "From the output to the declaration" covers any "type name, property or case … whose declaration is in the
  source".
- **Recommendation:** Resolve components, including external ones, and their props in `Declarations`, the way
  records are resolved, with an `nx-api` test on the components example. If that is deliberately out of scope,
  say so in design.md's `declaration` bullet and narrow the scenario.
- **Fix:** `Declarations` in `crates/nx-api/src/nx_text.rs` now resolves a `Shape` for records and components alike, external ones included (their props are `RecordField`s with a base, as records are). A component value gets its declaration, and its props their declared types, optionality and declarations. New test `a_component_and_its_props_are_declared_in_the_source`; design.md and the SDK types' docs mention components.
- **Verification:** Verified. Component and external-component values carry declarations, and their props carry declared types and declarations; covered by `a_component_and_its_props_are_declared_in_the_source`. In the browser, clicking `Counter` in the `components` output selects `Counter` in its declaration, and hovering `start` shows `(property) Counter.start: int` from the language service. Hovering `Counter` or `Button` themselves still shows only the default hover, because of a separate language-service issue (RF15).

### ✅ Verified - RF6 The address can miss an edit: it is updated only after an evaluation succeeds, so a quick example switch or a failed evaluation loses the edit from history
- **Severity:** Low
- **Evidence:** `App.tsx:101` (`onSettled`) runs `replaceState` only from `useEvaluation`'s success paths
  (`useEvaluation.ts:92`). The catch branch for a crashed or timed-out compiler returns first, so that source never
  reaches the address, and a reload loses it. Also, `chooseExample` (`App.tsx:117`) calls `pushState` at once. If
  the visitor edits and picks an example within the 350 ms debounce, or before a slow evaluation answers, the entry
  Back returns to still holds the old `/play/<id>` and not the edit. That breaks "Leaving edits behind" and "Edits
  keep the address current".
- **Recommendation:** Tie the address to the edit, not to the evaluation. In `chooseExample`, when
  `edited.current` is set, encode the current source and `replaceState` before the `pushState`. Update the address
  on the debounce itself (or in `finally`), so a failed evaluation still records the source.
- **Fix:** The address policy moved into `sites/playground/src/address.ts` (`createAddressKeeper`). An edit reaches the address after the typing pause regardless of the evaluation, so a crashed or timed-out source is still kept; choosing an example first writes any edit not yet in the address (waiting for its pause or still being encoded) into the current entry, then pushes; Back drops a waiting edit rather than writing it into the entry it arrived at. `useEvaluation` no longer drives the address. Tests in `src/address.test.mjs` cover the pause, choose-right-after-edit, choose-while-encoding, nothing-edited and Back cases; checked in Chromium (edit, choose at once, Back returns the edit).
- **Verification:** Verified. `createAddressKeeper` owns the policy, and `address.test.mjs` covers the pause, choose-during-pause, choose-while-encoding and Back cases. In Chromium: an edit, then an example chosen at once, then Back returns the `#code=` entry holding the edit. Forward returns to `/play/hello`.

### ✅ Verified - RF7 The editor can open on stale text if the source changes before Monaco finishes loading
- **Severity:** Low
- **Evidence:** `NxEditor.tsx:85` creates the model from `value` captured by an effect with `[]` dependencies,
  after `registration.ready` (Shiki and the grammar) resolves. The `[value]` effect that syncs later changes is a
  no-op while `editor.current` is null. So if the visitor picks an example from the drop-down, which works before
  the editor loads, or presses Back during that load, the editor shows the first source while `App` state, the
  output and the address show the chosen one. The pattern predates this change, but the toolbar drop-down now
  makes it easy to hit.
- **Recommendation:** Read the latest value from a ref (`latestValue.current`) when creating the model, as
  `latestChange` and `latestTheme` already do.
- **Fix:** `NxEditor.tsx` creates the model from `latestValue.current`, a ref kept current like `latestChange` and `latestTheme`.
- **Verification:** Verified. The model is created from `latestValue.current`.

### ✅ Verified - RF8 Diagnostic markers jump back to stale positions on every keystroke
- **Severity:** Low
- **Evidence:** The marker effect's dependencies are `[diagnostics, value]` (`NxEditor.tsx:158`), and it maps
  each diagnostic's line and column onto the current text with `offsetOf`. Monaco already moves markers along with
  an edit. Re-deriving them from the last evaluation's line and column puts them back at the old coordinates, for
  example on the wrong line after a newline is inserted above an error, until the next evaluation 350 ms or more
  later.
- **Recommendation:** Set markers only when `diagnostics` changes, mapped against the source those diagnostics
  came from. Carry that source with the evaluation, as `outcomeSource` already does, and drop `value` from the
  dependencies.
- **Fix:** Markers are set only when an evaluation reports (`[diagnostics, diagnosticsSource, editorReady]`), positioned by line and column in the source they were reported against (`Evaluation.diagnosticsSource`, new), not re-derived from the current text, so Monaco moves them with edits between evaluations. Checked in Chromium: inserting two lines above an error moves its marker down two lines at once and it stays there through the pause.
- **Verification:** Verified. The markers effect keys on `diagnostics`/`diagnosticsSource` and positions spans against the source they came from. In Chromium, inserting two lines above an error moves the marker down at once (147 to 183 px), and it stays there after the next evaluation.

### ✅ Verified - RF9 A runtime error raised inside prelude code loses its location entirely
- **Severity:** Low
- **Evidence:** `locate_error` (`crates/nx-interpreter/src/interpreter.rs:2328`) keeps the innermost location,
  which for a failure inside a prelude function is in the prelude's module. `located_runtime_error_diagnostics`
  (`crates/nx-api/src/eval.rs:203`) then drops the label because `error.module()` is not the entry module. The
  entry-module call site is known further out but is never recorded. So the output pane and the source pane show
  such errors with no span, even though the visitor's call is right there.
- **Recommendation:** Also record the outermost location in the entry module, the first span recorded while
  unwinding through an entry-module expression, and label that when the innermost location is elsewhere. Add an
  `nx-api` test that calls a prelude function which fails.
- **Fix:** The prelude declares only the `Range` type and runs no code, so the prelude case cannot occur; the same gap was real for a program built from several modules, an error inside an imported function. `RuntimeError` now also records, while unwinding, the first location it passes in each other module (`with_outer_site`, `site_in`), and `nx-api` labels the entry module's site. New test `a_runtime_error_in_an_imported_function_is_labeled_at_the_call` (a workspace whose `app/main.nx` calls `divide(0)` from `ui/math.nx` is labeled at line 3 of `app/main.nx`).
- **Verification:** Verified. The premise was right that the prelude runs no code, and the real case is a cross-module call. `with_outer_site`/`site_in` record the innermost site per module while unwinding, and `nx-api` labels the entry module's site. `a_runtime_error_in_an_imported_function_is_labeled_at_the_call` covers it. The extra work is on the error path only.

### ✅ Verified - RF10 The value-view tests build their values with a TypeScript copy of the formatter's layout
- **Severity:** Low
- **Evidence:** `packages/value-view/test/fixtures.ts` re-implements the layout rules: sorting, the complex/simple
  split, indentation, quoting and node placement. It does this to build every `NxValueText` the element tests
  use. design.md rejects a second formatter so that the viewer's input is "the CLI's text by construction". The
  element tests can pass against layouts Rust never produces, and they won't notice when `nx_text.rs` changes
  layout. They already miss shapes such as a record nested mid-line (see RF3).
- **Recommendation:** Commit JSON fixtures produced by `evaluateNx()`: a small script, or a test in
  `@nx-lang/sdk-wasm` that writes them and fails when they drift. Load those in the element tests, and delete the
  layout copy.
- **Fix:** The TypeScript layout copy is gone. `packages/value-view/test/fixtures/sources.json` holds the sources, `scripts/update-fixtures.mjs` (`pnpm run update-fixtures`) writes `test/fixtures/values.json` with `evaluateNx()`, and `test/fixtures.test.ts` fails when the committed values differ from what the compiler returns now. The element tests load those values, and the new `nested` fixture is a record nested mid-line. `@nx-lang/sdk-wasm` is a dev dependency.
- **Verification:** Verified. The TypeScript layout copy is gone. Element tests load `test/fixtures/values.json`, produced by `evaluateNx()` through `scripts/update-fixtures.mjs`, and `fixtures.test.ts` fails on drift. `@nx-lang/sdk-wasm` is a workspace dev dependency, so the package's tests need the wasm build first, which `pnpm -r` ordering gives.

### ✅ Verified - RF11 Properties set on `<nx-value>` before the element is defined are lost
- **Severity:** Low
- **Evidence:** `NxValueElement` (`packages/value-view/src/index.ts`) exposes `value`, `describe`, `highlighter`,
  `themes`, `truncated` and `stale` as accessors, but it does not re-apply own properties that were set on the
  element before it was upgraded. A host that sets `el.value = …` before the module has run creates an own data
  property that hides the accessor, and the element renders nothing. Examples are a server-rendered or HTML
  `<nx-value>` with the package imported lazily, or a React tree that renders before the import resolves. The
  playground avoids this only because it imports the package at the top of `OutputPane.tsx`.
- **Recommendation:** In the constructor, or in `connectedCallback`, apply the standard "upgrade property"
  step for each accessor: if `Object.hasOwn(this, name)`, read the value, `delete this[name]`, and set it again.
  Add a DOM test that sets `value` before `defineNxValueElement()`.
- **Fix:** The constructor applies the upgrade-property step for `value`, `describe`, `highlighter`, `themes`, `truncated` and `stale`. New DOM test sets `value` and `stale` on an element before its tag is defined, defines it, and checks it renders and is stale.
- **Verification:** Verified. The constructor re-applies own properties through the accessors after the shadow DOM is built, and "takes the properties a host set before the definition" covers `value` and `stale`.

### ✅ Verified - RF12 A value at the output cap blocks the main thread for about a third of a second on each evaluation, and the cut can split a character
- **Severity:** Low
- **Evidence:** For a 3,000-row value cut to 100,000 characters (9,747 nodes, about 40,000 spans), Chromium logs
  main-thread long tasks of 127 ms and 334 ms on load and 231 ms after each edit. That is Shiki tokenizing the
  whole text synchronously (`index.ts:264`) plus building the DOM, so typing in such a program stutters on every
  pause. The spec asks for the page to "stay responsive" at the cap. Also, `valueOutcome`
  (`sites/playground/src/compile/evaluate.ts:79`) cuts at a UTF-16 index, which can leave a lone high surrogate at
  the end of the text.
- **Recommendation:** Above a size threshold, render the text uncolored, or color it in chunks from an idle
  callback, and consider a lower cap for coloring than for text. Move the cut back to a code-point boundary, and
  preferably to the last line break before the cap, so no half-drawn node and no broken character is left behind.
- **Fix:** The worker cuts at the last line break within the cap, or for one long line at the cap moved back off a high surrogate (`cutPoint` in `compile/evaluate.ts`, tests in `src/compile/evaluate.test.mjs`; the session test now checks the last line is whole). The element colors only text up to `MAX_COLORED_CHARACTERS` (20,000) and shows longer text uncolored, still folding and hovering (DOM test; README, spec and scenario updated). Measured in Chromium with a 3,000-row value at the cap: no main-thread long task on load or after an edit (a control 120 ms block is detected by the same observer).
- **Verification:** Verified. `cutPoint` cuts at the last line break, or off a high surrogate for one long line, with `evaluate.test.mjs` covering both. Text over 20,000 characters is shown uncolored. In Chromium, a 3,000-row value at the cap now ends on a whole line (99,988 characters) with the notice shown. After an edit the longest main-thread task is 52 ms, down from 231 ms. The only long task on load, about 104 ms, is the same one a plain `/play/hello` load shows.

### ✅ Verified - RF13 Two spec scenarios disagree with the implementation and with the value-view spec
- **Severity:** Low
- **Evidence:**
  - The playground's "A long value folds" says the pane "SHALL offer to fold each record and the sequence". When
    `root` returns a sequence, though, the sequence is the whole value, and value-view's "Long values fold" and
    the implementation give the whole value no toggle.
  - "Document titles" says a fragment's source names "the NX Playground alone". `App.tsx:97` names the example
    whenever the source equals one, and Share on an unedited example produces such a link.
- **Recommendation:** Reword the fold scenario to "each record, and each nested sequence", and the title scenario
  to "names the example when the source is exactly an example's, and the NX Playground alone otherwise", or
  change the code to match the current wording.
- **Fix:** Reworded both scenarios to match the code: the playground's fold scenario now says each record, and each nested record, property and sequence that starts its line, with the whole value not folding; the title scenario names the example whenever the source is exactly an example's, and the NX Playground alone otherwise.
- **Verification:** Verified. The playground fold scenario and the title scenario now match the code and the value-view spec.

### ✅ Verified - RF14 The page's address, history and output-state logic has no committed tests
- **Severity:** Low
- **Evidence:** `routes.test.mjs` covers only the pure route parser. `openLocation`, `onSettled`,
  `chooseExample`, the `popstate` reload and `declaredName` in `App.tsx` have no tests. Neither do the
  `useEvaluation` transitions (stale, "Evaluating…", failure) or the `OutputPane` states. Tasks 6.3, 8.1–8.5 and
  9.1 were verified with ad hoc Playwright runs that aren't in the repository. RF2 and RF6 are both in this
  untested code, and RF1 went unnoticed because the only recursion test runs outside a browser.
- **Recommendation:** Move the history policy and the evaluation state reducer into plain modules and give them
  node tests, including the edit-then-choose race and a first source that doesn't compile. Also commit a small
  Playwright smoke test under `sites/playground`: open an example, hover and navigate, share, go Back, follow an
  invalid link, and run runaway recursion in the worker. CI can run it against `vite preview`.
- **Fix:** The history policy (`src/address.ts`), the evaluation state (`src/evaluation.ts`) and `declaredName` (now in `src/editor/positions.ts`) are plain modules with Node tests (`address.test.mjs`, `evaluation.test.mjs`, `positions.test.mjs`, the last on nodes from a real evaluation), covering the edit-then-choose race and a first source that doesn't compile. RF1 has a committed regression test under a browser-sized stack (`bindings/wasm/test/recursion-stack.test.ts`). A committed Playwright smoke test is not included: Playwright and a browser are not dependencies of the workspace or its CI today, so adding them is a separate decision; the browser checks for this round were run by hand as before.
- **Verification:** Verified. The address policy, the evaluation reducer, `declaredName` and the output cut are plain modules with Node tests, including the edit-then-choose race and the first-source-does-not-compile case. RF1 has a regression test under a browser-sized stack. Leaving the Playwright smoke test out is a reasonable call while no browser is in CI, and the risky logic is now tested. The remaining gap is browser-only integration: this pass found RF15 only by hand.

## New Findings Discovered During 2026-09-27 16:50 Verification

### ✅ Verified - RF15 Hovering a component's name in the output shows only the default hover, because the language service answers nothing at the first character of a component's name
- **Severity:** Low
- **Evidence:** In `/play/components`, hovering `Counter` or `Button` in the output shows `Counter` / `Button`,
  the element's default. The playground's `describe` asks for the hover at `declaredName(...).start`, the first
  character of the name. Called directly with that position, `languageService.hover` returns `null` for
  `component <Counter …>` (line 8, character 11) and for `external component <Button …>` (line 6, character 20).
  One character later it returns ```` ```nx\ncomponent <Counter start:int />\n``` ````. Records answer at their
  first character, which is why `Task` works. So the output hover scenario ("the same hover the source pane shows
  for the declaration") fails for components. The source pane also shows nothing when the pointer is on a
  component name's first letter.
- **Recommendation:** Fix the language service's hover range for component and external-component names so it
  includes the first character, and add a language-service test that hovers at a component name's first
  character. Asking at the middle of the name in `describe` would hide the symptom, but it is not the fix.
- **Fix:** The cause was general, not specific to components: in `crates/nx-language-service/src/positions.rs`, `child_at` preferred the child ending at the offset over the one starting there, so at the position before `B` in `<Button` it chose the `<` token, and any name written directly after `<` (component, external component and element-function declarations, and tags in use) had no hover on its first character; a record's name answered only because a space precedes it. Punctuation that ends at the offset now yields to a name that starts there; everything else keeps the old order, which completion relies on. New test `hover_answers_on_the_first_character_of_a_name_after_an_angle_bracket` covers the three declaration forms and two tags in use, and fails with the change reverted. The wasm module and the Node SDK's native library were rebuilt (the SDK hover parity test compares the two). Checked in Chromium on `/play/components`: hovering `Counter` and `Button` in the output shows `component <Counter start:int />` and `external component <Button label:string />`.
- **Verification:** Verified. `child_at` now prefers a word that starts at the offset over punctuation that ends there, and only when both exist, so positions after whitespace and inside tokens are unchanged. `cargo test -p nx-language-service -p nx-lsp` passes (139), including `hover_answers_on_the_first_character_of_a_name_after_an_angle_bracket`. In Chromium, on a fresh `/play/components`, hovering `Counter` and `Button` in the output shows `component <Counter start:int />` and `external component <Button label:string />`, the source's own hovers. Nearby positions probed through the playground's language service still behave: hover at the first character of a member after `.`, of a name after `{`, and of a tag in use; property completion at `<Card |title=`.

### ✅ Verified - RF16 The specs still promise output identical to `nxlang run`, but the wasm build now stops recursion at 200 calls deep where the CLI allows 1,000
- **Severity:** Low
- **Evidence:** RF1's fix lowers `MAX_RECURSION_DEPTH` to 200 for `evaluateNx()` only
  (`bindings/wasm/native/src/lib.rs`). design.md and both READMEs record this. The requirement text in
  `specs/sdk-wasm/spec.md` ("identical to what `nxlang run` prints for the same source") and in
  `specs/playground/spec.md` ("The text SHALL be identical…") is unqualified. A program that recurses between 200
  and 1,000 calls deep prints a value from `nxlang run` but fails the playground, `check-examples` and the website's
  `nx output` check. The last two evaluate through the same module, so a docs page could document output the
  docs check then rejects.
- **Recommendation:** Qualify both requirements: text identical to `nxlang run` for any program within the
  module's recursion limit of 200 calls, with the limit named. Add an sdk-wasm scenario for recursion just past
  200 ending in the recursion-limit diagnostic. Alternatively, have the website's check say so when an `nx output`
  mismatch is a recursion-limit error.
- **Fix:** The sdk-wasm requirement now states the one difference from `nxlang run`: calls nest at most 200 deep in the module, against 1,000 for the command line, and deeper programs fail with the recursion-limit error. It gains the scenario "Recursion past the module's limit but within the command line's" (250 deep: an error from `evaluateNx()`, a value from `nxlang run`), and the parity scenario notes its corpus stays within the limit. The playground's output requirement and the website's `nx output` requirement are qualified the same way. New test in `bindings/wasm/test/parity.test.ts` runs the 250-deep program through both and asserts exactly that.
- **Verification:** Verified. The sdk-wasm requirement names the 200-call limit against the CLI's 1,000 and adds the 250-deep scenario. The playground output requirement and the website `nx output` requirement are qualified the same way. The new parity test asserts `0` from `nxlang run` and `recursion depth 200 exceeded` from `evaluateNx()`. `pnpm --filter @nx-lang/sdk-wasm test` passes (58), and `openspec validate --strict` passes.

## Questions
- RF1: what recursion depth should NX promise in the browser? A lower wasm-only limit changes what a program can
  do in the playground compared with `nxlang run`. The alternative is shrinking the interpreter's frames.
  - **Answer (fix pass):** 200 in the WebAssembly build, 1,000 elsewhere. Removing the one frame this change added
    per expression moved the measured depths only by noise, so shrinking frames enough to reach 1,000 in a browser
    worker would be a larger interpreter change of its own. The limit is recorded in design.md and the SDK README.

## Summary
- The Rust side is solid. The formatter moved without changing the CLI's output, nodes are recorded during the
  single layout walk, UTF-16 offsets are right, including before non-BMP characters, and declarations and
  declared property types resolve for records, payload cases and inherited fields. Runtime errors now carry
  spans. The SDK, the share codec, the `nx output` check, the "Open in playground" plugin and the `/play` move are
  clean and tested. Hover, stale fallback, navigation, keyboard focus, dark mode and the 375 px layout all work in
  the browser.
- The one blocking problem is RF1. The headline robustness promise, that runaway recursion becomes a runtime
  error rather than a crash, fails in a real browser worker even for moderately deep legitimate recursion, and
  only Node tests cover it.
- RF2 (a permanent "Evaluating…" for sources that don't compile, which every `nx invalid` docs link opens), RF3
  (fold toggles painted over the text) and RF4 (stale `/playground` links in shipped READMEs) should be fixed
  before merge. The rest are smaller correctness, test and spec-wording items.
