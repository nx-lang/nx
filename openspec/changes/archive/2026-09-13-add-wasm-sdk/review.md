# Review: add-wasm-sdk

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md` (including its deviations note),
`measurements.md`, `specs/sdk-wasm/spec.md`, `specs/sdk-node/spec.md`, `specs/playground/spec.md`

**Reviewed code:**

- `bindings/wasm/` — `native/src/lib.rs`, `src/{abi,host,module,browser,node,api,language,errors,types}.ts`,
  `scripts/build-wasm.mjs`, `scripts/clean.mjs`, `package.json`, `tsconfig*.json`, `vitest.config.ts`,
  `README.md`, and every file under `test/`
- `scripts/fetch-wasi-sysroot.mjs`, `Cargo.toml`, `rust-toolchain.toml`, `pnpm-workspace.yaml`, `.gitignore`
- `packages/language-core/` — `src/{index,prelude,cache,answer,service}.ts`, `test/`, `package.json`;
  and `packages/language-http/src/index.ts` + `package.json` against their pre-change versions
- `sites/playground/` — `src/compile/{catalog,worker,index,types}.ts`, `src/worker/{protocol,nx.worker,channel,session,index}.ts`,
  `src/language/worker.ts`, `src/App.tsx`, `src/editor/{EditorView,NxEditor}.tsx`, `src/paths.ts`,
  `src/render/useNxDrawing.ts`, `src/gallery/Gallery.tsx`, `server/index.mjs`, `vite.config.ts`,
  `scripts/{compile-example,check-examples,emit-example-ir}.mjs`, `package.json`, `tsconfig.json`,
  `Dockerfile`, `README.md`, and the new/changed tests
- `.github/actions/setup-rust-node/action.yaml`, `.github/workflows/{build,deploy-playground}.yml`,
  `.railway/railway.ts`, `docs/deployment-setup.md`, `specs/future.md`, `bindings/node/README.md`
- Deleted files reviewed against their replacements: `server/{compile,language,watchdog,port}.mjs`,
  `scripts/dev.mjs`, `src/compile/http.ts` and their tests

**Verified while reviewing:** `WebAssembly.Module.imports(dist/nx.wasm)` lists 13 entries, all in
`wasi_snapshot_preview1`; `pnpm --filter @nx-lang/language-core test` (13 pass);
`node --test src/worker/channel.test.mjs src/compile/worker.test.mjs` (12 pass).

Overall the change is well built: the ABI is small and self-describing, the prelude arithmetic is
genuinely shared, parity with the Node SDK is checked byte for byte, the server is reduced to what
it actually does, and the docs and deployment config were carried along carefully. The findings
below are mostly about what happens on a slow first load.

## Findings

### ✅ Verified - RF1 The worker's request deadline covers the compiler module's download, so a slow first load kills the worker instead of waiting for it
- **Severity:** High
- **Evidence:** `createWorkerChannel` starts the 10 s deadline when a request is *sent*, not when
  the worker starts working on it ([channel.ts:147-154](sites/playground/src/worker/channel.ts#L147-L154)),
  and the worker only resolves `ready` after `WebAssembly.compileStreaming(fetch(nxModuleUrl))`
  finishes ([nx.worker.ts:16-31](sites/playground/src/worker/nx.worker.ts#L16-L31)). In the editor
  view the worker starts on mount and the first compile is sent 350 ms later
  ([EditorView.tsx:37-39](sites/playground/src/editor/EditorView.tsx#L37-L39),
  [useNxDrawing.ts:38](sites/playground/src/render/useNxDrawing.ts#L38)); on the gallery there is no
  head start at all — `startNxWorker` is never called there, so `ensureWorker()` runs inside the
  first `send`, meaning the download begins with the deadline already ticking, and all twenty
  previews are sent at once with `debounceMs = 0`
  ([Gallery.tsx:29](sites/playground/src/gallery/Gallery.tsx#L29)).
  `measurements.md` records the module arriving at **73.5 s** on Chrome "Fast 3G" — more than seven
  times the deadline. When the timer fires, `replaceWorker` terminates the worker (aborting the
  in-flight module fetch, so nothing is cached to resume from) and rejects *every* pending request
  with `TimeoutError` ([channel.ts:92-99](sites/playground/src/worker/channel.ts#L92-L99)). The
  constant's own comment — "a cold compile of the catalog and an example takes tens of
  milliseconds; this is far above anything healthy"
  ([channel.ts:17-23](sites/playground/src/worker/channel.ts#L17-L23)) — shows the budget was sized
  against compile time only. The result on a slow connection is not "a failed compiler costs one
  request" (playground spec) but a loop: every attempt restarts the download and is cut off at 10 s.
  None of the channel tests cover a request whose worker has not finished starting.
- **Recommendation:** Separate readiness from service time. Have the worker post a `ready` (or
  `failed`) message once the module is compiled, and have the channel either withhold the deadline
  timer until the worker is ready, or give the first request a much larger startup budget than
  subsequent ones. A worker that has not become ready should not be terminated by a request's
  deadline at all — terminating it discards the partial download, which is the one thing that makes
  a slow load eventually succeed.
- **Fix:** Readiness is now separate from service time. The worker posts a `ready` notice once its
  module is compiled ([protocol.ts](sites/playground/src/worker/protocol.ts),
  [nx.worker.ts](sites/playground/src/worker/nx.worker.ts)), and the channel arms a request's
  deadline only when the worker is ready: each pending entry holds its timer behind `arm`/`disarm`,
  and `markReady` arms everything already queued. A request sent while the module is downloading
  waits without a timer, so a slow load is waited for rather than terminated and started over. Three
  channel tests cover it — a request held past the deadline while loading, a deadline that starts at
  readiness, and the existing overrun test now driven through a ready worker — and all three fail
  against the previous behaviour. Design D6 and the playground spec record the rule.
- **Verification:** Confirmed. `WorkerReady` is part of the protocol, `nx.worker.ts` posts `{ kind:
  "ready" }` once `compileStreaming` resolves — registered before `self.onmessage`, so readiness
  always precedes the first answer — and the channel holds each pending entry's timer behind
  `arm`/`disarm`, arming at send only when `ready` and arming everything queued in `markReady`. The
  details that could have gone wrong are right: `ready` is reset in both `ensureWorker` and
  `replaceWorker`, so a replacement worker's requests wait again rather than inheriting readiness;
  `arm` is idempotent (`timer ??=`), so `markReady` cannot double-arm a request already armed; and a
  load that *fails* posts no readiness notice but answers every request with `NxModuleLoadError`, so
  nothing is left waiting. 18/18 channel and seam tests pass, including "a request sent before the
  worker is ready waits for the module rather than timing out", which asserts the worker is
  untouched three times past a 20 ms deadline — the previous code terminated it at 20 ms. Design D6
  and the playground spec ("A compiler that has not loaded yet is waited for") record the rule. One
  consequence of removing the bound is new and is opened below as RF10.

### ✅ Verified - RF2 Nothing retries after a compiler failure, so a transient one leaves the page permanently blank
- **Severity:** Medium
- **Evidence:** `useNxDrawing`'s effect re-runs only on `[source, compile, debounceMs]`
  ([useNxDrawing.ts:99](sites/playground/src/render/useNxDrawing.ts#L99)); a rejected compile sets
  `failure` and stops ([useNxDrawing.ts:43-53](sites/playground/src/render/useNxDrawing.ts#L43-L53)).
  The messages the diagnostics pane shows promise otherwise — "keep editing and the next compile
  will run" ([worker.ts:37-41](sites/playground/src/compile/worker.ts#L37-L41)) — which is true in
  the editor, where the visitor types, but not on the gallery, where nothing ever edits a preview's
  source. A single deadline expiry (RF1, but also a genuine hang) therefore leaves all twenty
  gallery cards reading "The compiler did not answer within 10s." with no way back except a reload.
  The playground spec's "the next compile SHALL succeed without reloading the page" is only met
  where a later compile happens to be requested.
- **Recommendation:** Retry once on a `TimeoutError`/`WorkerStoppedError`/`NxHostCrashedError`
  failure — the channel already starts a fresh worker on the next request, so a single re-issue from
  `useNxDrawing` (or a retry affordance in the preview/diagnostics pane) is enough. Guard it so a
  reproducible crash does not spin.
- **Fix:** `compileInBrowser` retries once when the failure is one a fresh worker can answer
  (`TimeoutError`, `WorkerStoppedError`, `NxHostCrashedError`); a module that would not load is not
  retried, since the same fetch would fail the same way, and a second failure is reported as before.
  The retry is factored as `retrying(attempt)` so it is driven in tests without a worker
  ([compile/worker.ts](sites/playground/src/compile/worker.ts)); three tests cover the retry, the
  fault that is not retried, and the fault that repeats. A gallery preview now redraws on its own
  after a transient failure instead of waiting for an edit that never comes.
- **Verification:** Confirmed. `retrying(attempt)` re-issues once for `TimeoutError`,
  `WorkerStoppedError` and `NxHostCrashedError`, and passes everything else straight to
  `compileFailureMessage`; `NxModuleLoadError` is correctly excluded and a second failure is
  reported rather than retried again. Because the channel has already discarded the failed worker,
  the second attempt runs against a fresh one, so a gallery preview that is never edited now redraws
  itself. Three new tests cover the retry, the fault that is not retried, and the fault that repeats
  (asserting exactly two attempts); all pass.

### ✅ Verified - RF3 The deploy workflow's `paths` filter no longer covers everything the image is built from
- **Severity:** Medium
- **Evidence:** `sites/playground/Dockerfile` copies `bindings/node/native`
  ([Dockerfile:32](sites/playground/Dockerfile#L32) — needed so cargo can load the workspace) and all
  of `scripts` ([Dockerfile:35](sites/playground/Dockerfile#L35)), but
  `.github/workflows/deploy-playground.yml` dropped `bindings/node/**` and added only
  `scripts/fetch-wasi-sysroot.mjs`. A commit that touches `bindings/node/native/Cargo.toml` or another
  file under `scripts/` changes what the image builds from without triggering a deploy, so production
  silently lags `main` until some other path happens to change. Task 5.3 records "the workflow's path
  filter matches what the Dockerfile copies" as verified; it does not.
- **Recommendation:** Restore `bindings/node/native/**` and widen `scripts/fetch-wasi-sysroot.mjs`
  to `scripts/**` in the `paths` list — or, if the intent is that the Node crate cannot affect the
  image, say so in the Dockerfile comment and add a check that keeps the two lists honest.
- **Fix:** The `paths` filter lists `bindings/node/native/**` (with a comment naming why the image
  copies it) and `scripts/**` in place of the single fetch script, so it covers everything
  `sites/playground/Dockerfile` copies.
- **Verification:** Confirmed. The `paths` list now carries `bindings/node/native/**` (with a
  comment naming why the image copies it) and `scripts/**`. Cross-checked against every `COPY` in
  `sites/playground/Dockerfile` — `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`,
  `packages`, `bindings/wasm`, `bindings/node/native`, `runtime/typescript`, `crates`, `scripts`,
  `Cargo.toml`, `rust-toolchain.toml`, `src/vscode`, `sites/playground` — and every one is matched
  by a filter entry. Minor leftover, not worth reopening: task 5.3's own text still describes the
  narrower original filter (`scripts/fetch-wasi-sysroot.mjs`), so the task now understates what
  shipped.

### ✅ Verified - RF4 The playground README still credits `@nx-lang/language-client` for hover and completion
- **Severity:** Low
- **Evidence:** [README.md:92](sites/playground/README.md#L92) — "`src/editor/` | the editor view, and
  Monaco through `@nx-lang/monaco` (grammar, highlighting, hover, completion) and
  `@nx-lang/language-client`". The dependency was removed from
  [package.json](sites/playground/package.json) and `NxEditor.tsx` now registers
  `createWorkerLanguageService()` from `src/language/worker.ts`. It is the only stale reference left
  in the tree (grepped for `API_ROOT`, `language-client`, `sdk-node`, `language-http`,
  `compileOverHttp`, `createHttpLanguageService`, `server/compile`, `watchdog`, `dev:all`,
  `port.mjs`), and task 4.8 claims the README was verified against the tree.
- **Recommendation:** Replace the trailing clause with `src/language/`, which the table already
  lists two rows above.
- **Fix:** The row now reads "the editor view, and Monaco through `@nx-lang/monaco` (grammar,
  highlighting, hover, completion) over `src/language/`".
- **Verification:** Confirmed. [README.md:92](sites/playground/README.md#L92) now reads "… (grammar,
  highlighting, hover, completion) over `src/language/`", which matches what `NxEditor.tsx`
  registers. A grep for `language-client` across `sites/playground` returns nothing.

### ✅ Verified - RF5 No test asserts the module imports only WASI preview 1
- **Severity:** Low
- **Evidence:** The sdk-wasm spec has an explicit scenario ("Module needs only standard WASI
  imports" — *every import SHALL belong to the WASI preview 1 namespace*) and task 1.4 records
  verifying it with a Node script, but nothing in `bindings/wasm/test/` inspects
  `WebAssembly.Module.imports`. I checked the built module by hand and it holds today (13 imports,
  all `wasi_snapshot_preview1`). A future crate dependency that pulls in an `env` import would be
  caught indirectly — instantiation would fail — but with an opaque `LinkError` rather than a
  message naming the offending import, and only if the failing path is exercised.
- **Recommendation:** Add a few lines to `loader.test.ts` (or a new `module.test.ts`) asserting the
  import module names are exactly `wasi_snapshot_preview1`, and print the offending names on
  failure.
- **Fix:** [loader.test.ts](bindings/wasm/test/loader.test.ts) asserts the shipped module's imports
  are all in `wasi_snapshot_preview1`, and names any that are not rather than counting them.
- **Verification:** Confirmed. `loader.test.ts` filters `WebAssembly.Module.imports(nxModule)` to
  entries outside `wasi_snapshot_preview1` and asserts the list is empty, mapping each to
  `module.name` so a stray `env` import fails with the symbol named rather than as an opaque
  `LinkError` at instantiation. The test passes, and I re-checked both shipped modules
  independently: `nx.wasm` and `nx-debug-trap.wasm` each have zero foreign imports.

### ✅ Verified - RF6 `resultRecordSize` is dead, and the result record is read with hard-coded offsets
- **Severity:** Low
- **Evidence:** [abi.ts:22](bindings/wasm/src/abi.ts#L22) exports `resultRecordSize = 12` and nothing
  in the repository reads it (grepped across `.ts`/`.mjs`). Meanwhile
  [host.ts:340-343](bindings/wasm/src/host.ts#L340-L343) reads the record at `result`, `result + 4`
  and `result + 8` as bare numbers. The one place the layout is named is the one place it is not
  used, so a change to `NxWasmResult` in `native/src/lib.rs` has nothing to fail against.
- **Recommendation:** Either delete the constant, or use it — derive the three field offsets from it
  (or name them beside it) so the loader and the crate's `#[repr(C)]` struct stay tied together.
- **Fix:** `resultRecordSize` is gone. [abi.ts](bindings/wasm/src/abi.ts) exports
  `resultStatusOffset`, `resultPointerOffset` and `resultLengthOffset`, documented as the crate's
  `#[repr(C)] NxWasmResult` on a 32-bit target, and `WasmHost.#read` reads the record through them.
- **Verification:** Confirmed. `resultRecordSize` is gone (no occurrences remain in the repository),
  `abi.ts` exports `resultStatusOffset`, `resultPointerOffset` and `resultLengthOffset` documented
  against the crate's `#[repr(C)] NxWasmResult` on a 32-bit target, and `WasmHost.#read` reads all
  three fields through them rather than through `result`, `result + 4`, `result + 8`. The loader and
  the crate now have one place to agree.

### ✅ Verified - RF7 A `startWorker()` failure leaves a pending entry and a live deadline behind
- **Severity:** Low
- **Evidence:** `send` registers the pending entry and its timer, then calls
  `ensureWorker().postMessage(...)` inside the promise executor
  ([channel.ts:157-159](sites/playground/src/worker/channel.ts#L157-L159)). If `options.startWorker()`
  throws — `new Worker(...)` can, for example under a CSP that blocks worker scripts — the executor's
  throw rejects the promise, but `finish()` never runs: the entry stays in `pending` and its 10 s
  timer stays armed. When it fires, `replaceWorker` runs and rejects whatever else is in flight with
  a spurious `TimeoutError`.
- **Recommendation:** Move the `ensureWorker().postMessage(...)` into a `try`/`catch` that calls
  `finish()` before rethrowing, or start the worker before constructing the promise.
- **Fix:** `send` starts the worker inside a `try` that calls `finish()` before rethrowing, so a
  refused `new Worker(...)` leaves no pending entry and no armed timer behind. Covered by a test
  whose `startWorker` throws once: the next request survives past the point the leaked timer would
  have fired, and fails against the previous behaviour.
- **Verification:** Confirmed. `send` wraps `ensureWorker().postMessage(...)` in a `try` that calls
  `finish()` — disarm, delete from `pending`, drop the abort listener — before rethrowing, so a
  refused `new Worker(...)` leaves nothing behind. The new test refuses the first construction, then
  proves the *next* request survives 50 ms past a 20 ms deadline with its worker never terminated,
  which is exactly what the leaked timer would have done.

### ✅ Verified - RF8 `compileWithCatalog` can turn a successful compile into a throw, and lost the old null-safe diagnostics check
- **Severity:** Low
- **Evidence:** [catalog.ts:127-133](sites/playground/src/compile/catalog.ts#L127-L133) disposes the
  artifact in a `finally` around the `return`. A throw from `finally` replaces the return value, so if
  `nx_wasm_program_free` traps, a compile that produced IR is reported as `NxHostCrashedError`
  instead. The old `server/compile.mjs` used `artifact.dispose?.()` inside a `try` whose `catch`
  classified the error, so the same trap was handled rather than propagated. Separately,
  [catalog.ts:138](sites/playground/src/compile/catalog.ts#L138) reads
  `(error as { diagnostics?: unknown }).diagnostics` where the predecessor used `error?.diagnostics`
  ([old compile.mjs:132](sites/playground/server/compile.mjs#L132)); a thrown `null`/`undefined` now
  raises a `TypeError` from inside the error path.
- **Recommendation:** Wrap the `dispose()` so a crash on release does not mask a result the caller
  already has (or dispose after capturing the result), and restore the optional chain in
  `diagnosticsOf`.
- **Fix:** The artifact is released through a `release()` helper called on both paths before the
  return, so a trap inside `dispose` can no longer replace a result the caller already has — the
  host remembers it crashed, so the next call reports it and the worker replaces the host.
  `diagnosticsOf` reads `.diagnostics` through an optional chain again, so a thrown `null` is
  rethrown rather than raising a `TypeError`.
- **Verification:** Confirmed. Both the success and failure paths capture the result first and
  release through `release()`, which swallows a throw from `dispose()`, so a trap on free can no
  longer replace IR the caller already holds — and the host's own crashed flag still surfaces it on
  the next call, so nothing is hidden. `diagnosticsOf` reads `.diagnostics` through `?.` again, so a
  thrown `null` is rethrown instead of raising a `TypeError` from inside the error path. The catalog
  tests (source/catalog/program origins, the zero-width insertion point, the size and type guards)
  still pass, and `check-examples` compiles and evaluates all twenty examples.

### ✅ Verified - RF9 `pnpm -r test` rebuilds the wasm module twice from scratch
- **Severity:** Low
- **Evidence:** `@nx-lang/sdk-wasm`'s `test` script is
  `pnpm run build && pnpm run typecheck && node scripts/build-wasm.mjs --debug-trap && vitest run`
  ([package.json](bindings/wasm/package.json)). Both CI jobs run `pnpm -r build` and then
  `pnpm -r test` ([build.yml:233-236](.github/workflows/build.yml#L233-L236),
  [deploy-playground.yml:59-63](.github/workflows/deploy-playground.yml#L59-L63)), so the release
  module is built, built again by `test`, and then a `--debug-trap` module is built into a separate
  `--target-dir` ([build-wasm.mjs:36](bindings/wasm/scripts/build-wasm.mjs#L36)) — a cold compile of
  every crate for `wasm32-wasip1`, on the critical path of every deploy. The separate target
  directory is the right call for feature isolation, but paying for it unconditionally is not.
- **Recommendation:** Drop `pnpm run build` from the `test` script (CI and the `rebuild`/`pretest`
  path already cover it) and skip the `--debug-trap` build when
  `dist/nx-debug-trap.wasm` is newer than the crate's sources, or gate it behind a separate script
  the trap tests depend on.
- **Fix:** The `--debug-trap` build shares the repository's target directory instead of building
  into `target/wasm-debug-trap`. `debug-trap` is a feature of the top crate alone and changes nothing
  below it, so cargo rebuilds that one crate and reuses every dependency it already built: the trap
  build drops from a cold compile of the whole graph to **1.7 s** here. The two builds overwrite one
  artifact, which is harmless because each is copied to its own name in `dist/` before the next runs
  — verified by building in both orders and checking that `dist/nx.wasm` exports no `nx_wasm_trap`
  and `dist/nx-debug-trap.wasm` does. `pnpm run build` stays in the `test` script: it matches
  `bindings/node`, it is what makes `pnpm --filter @nx-lang/sdk-wasm test` work on a clean tree, and
  with a shared target directory it is a cargo freshness check rather than a rebuild.

## New Findings Discovered During 2026-09-13 22:53 Verification

### ✅ Verified - RF10 A module download that stalls rather than fails now waits forever, with nothing on screen but "Drawing…"
- **Severity:** Low
- **Evidence:** The RF1 fix removes the only upper bound on waiting for the module, which is right
  for a slow load and leaves a stalled one uncovered. A request sent before readiness is held with
  no timer at all ([channel.ts](sites/playground/src/worker/channel.ts), `arm`/`markReady`), and the
  worker breaks that wait in exactly two ways: it posts `ready` when `compileStreaming` resolves, or
  its rejection answers each request with `NxModuleLoadError`
  ([nx.worker.ts](sites/playground/src/worker/nx.worker.ts)). A fetch that connects and then stalls
  mid-body settles neither — `compileStreaming` never resolves or rejects, no `ready` is posted, and
  `onerror` never fires, since the worker itself is healthy. The request then waits indefinitely,
  `useNxDrawing` leaves `compiling: true`, and the editor (and all twenty gallery cards) read
  "Drawing…" with no message and no recovery. This is narrower than the case RF1 described — a
  trickling or half-open connection, a proxy that accepts and never finishes — but before the fix the
  10 s deadline at least produced something readable, and now nothing does.
- **Recommendation:** Give the *load* its own budget, separate from and far larger than the request
  deadline — a minute or two is generous against the 73.5 s Fast 3G measurement. On expiry, fail the
  waiting requests with an `NxModuleLoadError`-shaped fault: `compileFailureMessage` already words
  that as "Reload the page to try again", and `retrying` already declines to retry it, so the
  slow-load behaviour the fix is about is kept while the upper bound comes back.
- **Fix:** The load has its own budget now, two minutes by default — well above the 73.5 s the
  throttled measurement took against an uncompressed, uncached origin, and far below waiting
  forever. The fetch and compile moved out of the worker shell into
  [module.ts](sites/playground/src/worker/module.ts): `loadNxModule(url, { deadlineMs, fetch,
  compileStreaming })` aborts the fetch when the budget passes, which rejects it or errors the body
  stream the compile is reading, and frees the connection either way. Every failure — a fetch that
  fails, a module the browser refuses, a download that stalls — is reported as before under the name
  `NxModuleLoadError`, so `compileFailureMessage` still words it "Reload the page to try again" and
  `retrying` still declines to retry it; a stall says the budget ("did not finish within 120s")
  rather than whatever the abort happened to throw. The request deadline is untouched, so a slow
  load is still waited for. Five tests in
  [module.test.mjs](sites/playground/src/worker/module.test.mjs) cover the arrival, a download slower
  than a request's deadline, the stall (asserting the fetch was aborted and the wording), a failed
  fetch and a refused module. Design D6 and the playground spec record the budget.
- **Verification:** Confirmed. The fetch and compile moved into
  [module.ts](sites/playground/src/worker/module.ts), where `loadNxModule` arms an `AbortController`
  at `DEFAULT_LOAD_DEADLINE_MS` (120 s) and clears it in a `finally`, so no timer outlives the load.
  The abort genuinely settles the case the finding was about, not just the easy half of it: before
  the response headers it rejects the fetch, and after them it errors the body stream that
  `compileStreaming` is reading, so that promise rejects either way. Every failure still arrives
  named `NxModuleLoadError`, so `compileFailureMessage` words it "Reload the page to try again" and
  `retrying` declines to retry it — the disposition the recommendation asked for — and a stall says
  the budget rather than whatever the abort threw. `channel.ts` is untouched, so RF1's behaviour is
  preserved; I re-ran those tests to be sure. 37 worker and compile-seam tests pass, the full site
  suite is 58/58 with all 20 examples checked, and `typecheck` is clean. Two limits worth recording,
  neither a reason to reopen: the stall test drives a fetch that rejects on abort, so it covers the
  abort plumbing and the wording but not a browser erroring a half-read body stream — that rests on
  the fetch and WebAssembly contracts rather than on a test, which is inherent to stubbing; and
  `loadNxModule` now names two different functions, this one and the Node entry's filesystem loader
  in `@nx-lang/sdk-wasm`, though no file imports both.

## Questions

- `measurements.md` records the first compiled drawing at 83.0 s on Fast 3G with the module arriving
  at 73.5 s — a gap under the 10 s deadline. Given RF1, how was that run set up? If the app bundle
  (4.73 MiB) had to arrive before the editor could mount and post a compile, the deadline should have
  fired well before 73.5 s. Knowing whether that run went through a worker restart would say how
  close the shipped behaviour is to the livelock RF1 describes.
  - **Answer (not verified):** the run's network log was not kept, so this cannot be reconstructed
    from what is recorded. The likeliest reading is that it did restart: the editor mounts once the
    app bundle is parsed and posts its first compile 350 ms later, which on that connection is well
    before 73.5 s, so the deadline would have fired and `73.5 s` would be the last of several
    attempts rather than one uninterrupted fetch. With RF1 fixed that path no longer exists. Worth a
    re-measure on a throttled connection before the deviation note is read as evidence of anything;
    `measurements.md` is left as it was taken.
- The tasks' deviation note says the worker is started eagerly on the editor view and lazily on the
  gallery. Was leaving the gallery without `startNxWorker()` deliberate? Starting it on gallery mount
  costs nothing extra (the gallery compiles regardless) and would give the module a head start on the
  twenty previews that are sent immediately.
  - **Answer:** deliberate, and left as it is. A preview's `useNxDrawing` runs with `debounceMs = 0`,
    so its first compile — and with it `ensureWorker()` — happens in the same tick a mount effect
    would have fired in; `startNxWorker()` there would add a call and change nothing observable.
    With RF1 fixed, the absence of a head start no longer has a deadline consequence either.

## Summary

*(The summary below is the review as written. All ten findings are now fixed and verified — see each
finding's **Fix** and **Verification** notes. RF10 was opened during verification as the residue of
the RF1 fix: with the request deadline no longer covering the download, nothing bounded a download
that stalls instead of failing. The load now carries its own two-minute budget, and no finding
remains open.)*

- Solid, carefully staged change. The wasm ABI, the loader's trap contract, the extraction of
  `@nx-lang/language-core`, and the byte-for-byte parity tests with the Node SDK all do what the
  design said, and the deployment, CI and documentation changes were carried through properly.
- The one substantive problem is RF1: the worker's request deadline is measured from `send` and so
  includes the 2 MB module's download, which by the change's own measurements takes seven times the
  budget on a slow connection. Combined with RF2 (no retry), a slow first load does not degrade — it
  fails and stays failed, which is exactly what the "a failed compiler costs one request"
  requirement is meant to prevent.
- RF3 is a deployment-freshness gap worth fixing before the next deploy; the rest are small.
- **Verification:** Confirmed, and measured. The `--debug-trap` build now runs `--features
  debug-trap` against the shared target directory. `pnpm --filter @nx-lang/sdk-wasm test` completes
  end to end in **6.1 s** here, with cargo reporting `Finished \`wasm-release\` profile ... in
  1.46s` for the trap build — matching the note's claim and replacing what was a cold compile of the
  whole crate graph on every CI run. The overwrite is safe as described: checked directly,
  `dist/nx.wasm` exports no `nx_wasm_trap` and `dist/nx-debug-trap.wasm` does. 24 tests across 6
  files pass.

