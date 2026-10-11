# Review: add-preview-pane

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/playground/spec.md, specs/sdk-wasm/spec.md,
specs/value-view/spec.md  
**Reviewed code:** `git diff 9c83a64..HEAD`: crates/nx-api/src/{nx_text.rs,eval.rs,lib.rs},
bindings/wasm/native/src/lib.rs, bindings/wasm/src/{abi.ts,api.ts,host.ts,types.ts}, bindings/wasm/test,
packages/value-view/src/{index.ts,render.ts,styles.ts,types.ts} and tests, sites/playground/src/preview/*,
worker/{session.ts,protocol.ts}, compile/evaluate.ts, App.tsx, output/OutputPane.tsx, app.css, examples (counter),
READMEs  
**Checks run:** `cargo test -p nx-api` (pass), `cd bindings/wasm && pnpm test` (1 failure in
`recursion-stack.test.ts`, which also fails on a build of 9c83a64, so it is not from this change),
`cd packages/value-view && pnpm test` (pass), `cd sites/playground && pnpm test` and `npx tsc --noEmit` (pass),
`openspec validate add-preview-pane --strict` (valid). Node probes against the built module confirmed RF1–RF4.

## Findings

### 🟡 Fixed - RF1 Preview cannot run any source that uses a declaration from a library (`import "@nx/agent"`)
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `sites/playground/src/preview/workbench.ts:142` calls `build.artifact.generateNxIr({ debug: true })`
  with no `modules`, which per `NxIrEmitOptions.modules` emits only the entry module. `programFromImages` then
  cannot link the library. Probe: a source with `import "@nx/agent"`, a component `<C />` with state and a
  `let limits = <AgentLimits maxSteps=6 />` answered `start` with `startFailed` / `nx-ir-link-missing-module`:
  "Module 'playground.nx' links against '@nx/agent/agent.nx', which the resolver did not supply." `components` is
  then `[]`, so the picker disappears and the heading reads "The component would not start", which looks like a
  fault in the visitor's component.
- **Failure scenario:** A visitor starts from the "An AI agent and its tools" example (or any source that imports
  a library and uses one of its declarations), adds a component and switches to Preview. Nothing can be previewed.
- **Recommendation:** Emit every module (`generateNxIr({ debug: true, modules: [] })`, entry first). With that
  change the agent example links and runs (verified by probe). Add a workbench test with an imported library.
  Also map a link failure to its own problem message, not to "would not start".
- **Fix:** `#programFor` emits every module (`generateNxIr({ debug: true, modules: [] })`). A program that does not prepare or link is now its own problem, `programFailed`, headed "The program could not be prepared to run" and marked in the editor. Test: `runs a source that imports a library` (workbench.test.mjs).

### 🟡 Fixed - RF2 After a host crash or a replaced worker, Preview reports "does not compile" for a compiling program and stays stuck
- **Severity:** Medium
- **Confidence:** High
- **Evidence:**
  - `workbench.ts:101` `forgetBuild()` drops the build, and `#state()` (`workbench.ts:268`) returns
    `session: null` whenever `build === undefined`, even though the session still exists. design.md says "A
    replaced host loses the artifact but not the session", but the page can no longer see it.
  - The page reads that answer as a lost session (`model.ts` `sessionLost`, `usePreview.ts:44`) and sends `start`.
    `#start` → `#programFor` sees no build and answers `{ kind: "doesNotCompile" }` (and discards the live
    session). A fresh worker (after a timeout or a `WorkerStoppedError`) has no build either, so the "Start again"
    button (`usePreview.ts:71`) gets the same answer.
  - Because `asked === compiled` after that start, `callFor` sends nothing more until the visitor edits.
  - Probe: `keepBuild` + `start` + `forgetBuild` + `goTo` gave `session: null, problem: null`. A following
    `start` gave `doesNotCompile`. A new `PreviewWorkbench` answered `start` with `doesNotCompile`.
- **Failure scenario:** The compiler traps once (or a request overruns the 10 s deadline). The visitor presses a
  handler button. The preview switches to "The program does not compile yet…" while Output shows a value. Pressing
  "Start again" after a worker failure shows the same message.
- **Recommendation:** Let `#state()` build the view without a build (or keep a fallback for formatting), and add
  a problem kind such as `needsBuild`. Then the page re-sends `evaluate` for the compiled source before `start`, or
  `start`/`reload` carry the source so the worker can build it when it has no matching build. Add tests for
  `forgetBuild` and for a fresh session that receives `start` first.
- **Fix:** The workbench takes a `compile` callback (worker/session.ts builds with the current host) and builds a source it holds no build of. It keeps the latest evaluation build plus one other: the build the session's current step runs, so `#state()` formats each step with its own program's declarations (this also answers the second open question). After `forgetBuild` the session stays visible and its source is rebuilt on demand; a fresh worker's `start` builds the source itself, so "Start again" works. Test: `keeps showing the session after its host's builds are lost, and starts in a fresh worker`.

### 🟡 Fixed - RF3 A failed `setProps` is recorded as applied, so later edits run with the old props without any notice
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `workbench.ts:227` sets `this.#rootProps = props` before `session.setProps(root.props)` at line
  229. When `setProps` throws, the notice is shown once. The next reload compares against the new props, finds no
  change and never retries them. Probe: the component's label divides by `start`. Changing `<Counter start=5 />` to
  `start=0` gave a `failed` notice (division by zero). A following label-only edit answered `notice: null` with
  output `label="B 20"` (still rendered from `start=5`) and `state.source` equal to the editor's source. So the
  page shows it as current, not stale.
- **Failure scenario:** The visitor briefly types a prop value the component cannot render, then makes any other
  edit. The preview silently keeps running the old props while the source shows the new ones. This breaks "When
  an edit changes the props of the element `root` returns, the session SHALL render the current state with the new
  props".
- **Recommendation:** Update `#rootProps` only after `setProps` succeeds, or keep the props that failed and retry
  them on every reload until they succeed. Treat the failed props as stale in `PreviewState` so the page marks the
  output out of date. Add a test.
- **Fix:** `#rootProps` is updated only after `setProps` succeeds, so every later reload tries the failed props again and answers a `failed` notice each time. The page now marks the preview out of date whenever the last reload's notice is `failed` (App.tsx `previewStale`). Test: `tries props that failed again on every reload, and says so each time`.

### 🟡 Fixed - RF4 `formatValue` reads declarations from the entry module only, so library unions and generic fields lose their spelling
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - `crates/nx-api/src/nx_text.rs:572` (`Declarations::new`) indexes only the entry module's items. The
    canonical conversion (`canonical_record`, `expect_name` at line 758) therefore finds no field type for records
    declared in an imported library, and none for a generic record's type parameter `T`.
  - Probe 1: `<HttpTool ... method=post />` from `@nx/agent`. `evaluateNx` spells `method=post`, and `formatValue`
    of the IR runtime's value spells `method="post"`. The case node and its hover are lost too.
  - Probe 2: `type Rng = { T:type min:T max:T }`, `<Rng T=float64 min={1.0} max={2.0} />`. `evaluateNx` gives
    `<Rng max=2.0 min=1.0 />`, and `formatValue` gives `<Rng max=2 min=1 />`.
  - Both values hold only declared properties, so they fall under the "The same text as root's" scenario.
- **Failure scenario:** Once RF1 is fixed, a component that renders library records (an `HttpMethod`, a library
  `float64` field) or generic records shows different text in Preview than in Output, which is what the design's
  "one spelling of values across the page" goal rules out.
- **Recommendation:** Build `Declarations` over every module of the resolved program, keyed by the name the
  runtime writes in `$type`. For a generic record, carry the applied type arguments, or fall back to the
  record-level `T` binding when the IR has it. Add parity tests that cover a library union and a generic float
  field.
- **Fix:** `Declarations` now indexes every module of the resolved program, the entry module first so its names win, for both `evaluateNx` and `formatValue`. A library's declarations give types, unions and optionality but no spans, since spans index the entry source. Test: `a_library_union_and_record_spell_as_root_does` (nx_text.rs) checks text and node parity with `evaluateNx` for a library union and a library record. The generic-record part is not fixed: canonical JSON carries no type argument (`<Rng T=float64 …>` has no `T` in the value), so a whole `float64` held by a field typed `T` cannot be told from an `int`. Fixing it needs the IR runtime to report type arguments; left as a known gap.

### 🟡 Fixed - RF5 Forward forgets which branch the visitor left once any step is dispatched or reloaded
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `workbench.ts:220` and `workbench.ts:244` clear the whole `#left` map on every reload and every
  dispatch. Memory of other branches is lost even though their ticks are untouched. The spec says "Forward SHALL go
  to the child the visitor last left, or to the newest child when they have left none".
- **Failure scenario:** Step 2 has children 3 and 5. The visitor is at 3 and goes Back to 2, then Back to 1, then
  dispatches a new branch from 1. They then choose step 2 from the branch picker and press Forward. Forward goes to
  5, the newest child, not to 3, the child they last left.
- **Recommendation:** Do not clear `#left` on dispatch or reload. A new tick only changes the entry of its own
  parent, which `#goTo` already overwrites when the visitor leaves. Drop entries only for ticks the session pruned.
- **Fix:** `#left` is no longer cleared on dispatch or reload, only on start and stop; a stale entry is harmless because Forward checks it against the step's children. Test: `Forward remembers the branch left, whatever ran since`.

### 🟡 Fixed - RF6 Placing action buttons re-sorts every pending marker for every text stretch
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `packages/value-view/src/render.ts:95` does `[...markers].sort(...)` inside `appendText`, which
  runs once per text stretch between nodes. The cost is O(stretches × markers log markers). A component that
  renders a list of buttons up to the 100,000-character cut has thousands of handler markers and tens of thousands
  of stretches, all rendered synchronously on the main thread on every tick.
- **Failure scenario:** A preview of `for i in 0..1500 { <Button label={i} onTapped=... /> }` takes noticeably long
  to repaint after each press.
- **Recommendation:** Sort the marker offsets once into an array. Keep a cursor into it: stretches are visited in
  increasing text order, so each marker is reached once. That makes placement linear overall.
- **Fix:** `renderValue` sorts the marker offsets once and walks them with a cursor as stretches are appended in text order, so placement is linear. value-view's 37 tests pass.

### 🟡 Fixed - RF7 `formatValue(undefined)` fails with an internal "Format request is not valid" error
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `bindings/wasm/src/host.ts:696` sends `{ value }`. `JSON.stringify` drops an `undefined`
  member, so `FormatValueRequest.value` (`crates/nx-api/src/eval.rs:270`) is missing and the call fails as
  `FormatValueError::Request`. That maps to an internal error, not an `NxEvaluationError`: "missing field `value`".
  The signature takes `value: unknown`, and the doc comment promises `NxEvaluationError` for values outside the
  canonical format.
- **Failure scenario:** A host formats an absent optional value it holds as `undefined` and gets an internal
  error, not the documented error.
- **Recommendation:** Reject `undefined` in the wrapper with a `TypeError`. Alternatively, mark `value` with
  `#[serde(default)]` so it reads as `null` (the empty value). Document whichever is chosen.
- **Fix:** `formatValue(undefined)` throws `TypeError` in the wrapper before calling the module; documented in the doc comment and the README, with a test.

### 🟡 Fixed - RF8 Task 6.1 is unchecked although validation passes
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `tasks.md:37` still shows `- [ ] 6.1 Run openspec validate add-preview-pane --strict`.
  Running it now reports "Change 'add-preview-pane' is valid".
- **Recommendation:** Check the task off, since it is verified.
- **Fix:** Task 6.1 checked off; `openspec validate add-preview-pane --strict` passes.

## Questions
- `bindings/wasm` `recursion-stack.test.ts` ("forLoop within the limit" traps under a 0.5 MB worker stack)
  fails here. A build of 9c83a64 fails the same way, so this change did not cause it. Is it a known environment
  flake?
  - **Answer:** It fails the same way in this container on every run, and passes in CI (the Build jobs on this PR
    are green), so it is specific to this environment's build, not to the change.
- After a failed reload, `#view` spells the old program's output with the new build's declarations (and fetches
  action schemas from it). Is that intended, or should the workbench keep the artifact that matches the running
  tick?
  - **Answer:** Not intended. With the RF2 fix the workbench formats each step with the build of the source its
    program came from, compiled on demand when it is not kept.

## Summary
- The SDK formatter entry, pointers, ABI bump and value-view actions are solid and well tested. Parity with
  `evaluateNx` holds for every playground example that does not import a library. The preview workbench covers the
  main spec scenarios with Node tests.
- The main gaps are in the playground's robustness. Sources that use a library cannot be previewed (RF1). Recovery
  after a crash or a worker replacement ends in a false "does not compile" (RF2). A props change that fails to
  render is silently dropped on the next edit (RF3). RF4 is a parity gap that becomes visible once RF1 is fixed.
