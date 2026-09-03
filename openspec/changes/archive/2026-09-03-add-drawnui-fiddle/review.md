# Review: add-drawnui-fiddle

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, and all six delta specs under `specs/`  
**Reviewed code:** compiler/type-checker and grammar changes; TypeScript IR runtime changes and tests;
the authored DrawnUI sample-app code, catalog generator and generated catalog, compile server, renderer,
editor, gallery, examples, documentation, and deployment files. Vendored DrawnUI internals and generated
build output were inspected where they define integration boundaries, but were not reviewed line by line.  
**Verification run:** `cargo test --workspace`; TypeScript runtime `npm test`; sample-app `npm test`,
`npm run typecheck`, and `npm run build`; focused live probes of compile and HTTP failure paths.

## Findings

### ✅ Verified - RF1 A tiny malformed source can block the single-process compile service indefinitely
- **Severity:** High
- **Evidence:** `sample-apps/drawnui-react/server/compile.mjs:115` invokes the native compiler
  synchronously on the Node server's main thread. Calling `compile("@")` did not return within either
  the app's eight-second client timeout or a separate five-second process timeout (`timeout` exited
  124). While that call is running, the event loop cannot answer any other compile or static-file
  request, and aborting the browser request cannot interrupt it. The 256 KiB body limit at
  `server/compile.mjs:103` does not mitigate this one-byte input. This violates the requirements that
  compilation failures not break the editing session and that the deployed service continue serving
  the application.
- **Recommendation:** Run compilation in an interruptible worker process/thread with a server-side
  deadline and terminate/recycle the worker on timeout. Add an HTTP integration test that submits the
  reproducer, expects a bounded failure response, and proves a subsequent request still succeeds.
- **Fix:** The cause was in the compiler, not the source: the tree-sitter external scanner at
  `crates/nx-syntax/src/scanner.c` decided two lookaheads — the `@{` opener in typed text content
  and the start of an entity — by copying `TSLexer` and restoring the copy. That restores the
  lookahead character but not the position it was read from, and at end of input, where `advance`
  has nothing left to consume, the restored character came back on every pass and the scan never
  ended. `@` alone was enough; so was any file ending in `@` or `&` in text position. Both
  lookaheads now decide by consuming, which is why `is_entity_start` is gone. The parse trees are
  unchanged: `tree-sitter parse` output is identical across text, typed-text, entity, escape, and
  raw-content cases, differing only in the timing line. `cargo test --workspace` passes (40 suites),
  as do the app's tests, typecheck, and production build; the native addon was rebuilt so the fiddle
  compiles through the fixed parser. Covered by `test_scan_delimiter_at_end_of_file_terminates` and
  `test_scan_lone_at_in_embedded_text_is_literal` in `crates/nx-syntax/tests/parser_tests.rs`, which
  parse on a worker thread with a deadline so a regression fails the suite instead of hanging it;
  by `answers a stray delimiter at the end of the source` in `server/compile.test.mjs`; and by an
  HTTP test in the new `server/index.test.mjs` that posts the reproducer and then proves the service
  still answers. A 3000-case fuzz of short delimiter-heavy inputs finds no remaining hang, where a
  60-case sample of the same corpus exhausts a two-minute budget against the previous scanner.
- **Verification:** Confirmed causally, not just by assertion. Restoring `HEAD`'s `scanner.c` and
  running `test_scan_delimiter_at_end_of_file_terminates` reproduces the hang (`Parsing "@" ...`
  returns `None` after the 10s deadline); restoring the new scanner turns it green, so the test is a
  real regression guard rather than a passing assertion. An independent 20,000-case fuzz of short
  delimiter-heavy inputs (alphabet `<>{}@&\;#xA/ ab\n`, wrapped in plain text, typed text, and bare
  module positions, each parsed on a worker thread with a 5s deadline) found no hang; the worst
  single parse was 94ms. `cargo test --workspace` passes (48 suites, exit 0), as do the app's 12
  server tests and 12 example checks. `tree-sitter generate` reproduces the committed `parser.c` and
  `grammar.json` byte for byte, so the generated parser matches `grammar.js`.

  One correction to the fix note: parse trees are *almost* unchanged, not unchanged. A differential
  dump over 17 delimiter cases against the old scanner shows exactly one difference — `a & b` in
  typed text content now arrives as two adjacent `EMBED_TEXT_CHUNK` tokens (`"a "`, `"& b"`) instead
  of one, because the chunk now stops at every `&` and lets the next scan decide. The enclosing
  `EMBED_TEXT_RUN` still spans `"a & b"`, and `lower_element_content` reads the run's text rather
  than the chunks, so nothing downstream sees the split. Worth recording because a future consumer
  that walks chunks rather than runs would.

  The availability property the finding's title names is genuinely narrower than fixed, as the
  status note says. That residual is now tracked as RF6 rather than left inside this note.
- **Status:** The availability property this finding names is narrower than fixed. Compilation is
  still synchronous on the server's only thread with no deadline, so a future compiler hang would
  block the service the same way; what changed is that no known input produces one. Worth knowing
  before choosing a mitigation: a worker **thread** would not have helped, because `terminate()`
  interrupts JavaScript and cannot preempt a native call that never returns to it — only a child
  process can be killed reliably. Whether to pay for process isolation, and whether it is worth
  building at all against the design's stated plan to move compilation into the browser via WASM
  and delete this server, is a call left to the author rather than made here.

### ✅ Verified - RF2 A malformed URL path terminates the entire Node service
- **Severity:** Medium
- **Evidence:** `sample-apps/drawnui-react/server/index.mjs:99` calls `decodeURIComponent` without
  handling `URIError`, and the request dispatcher at `server/index.mjs:114` has no error boundary.
  Requesting `/%ZZ` produced an empty response, threw `URIError: URI malformed`, exited Node, and made
  the following request fail to connect. Thus any remote client can stop the single deployed service
  with one request.
- **Recommendation:** Catch URL construction/decoding and filesystem lookup errors at the request
  boundary and return 400 (or 404) without exiting. Add an integration test that requests a malformed
  escape and then successfully fetches `/`.
- **Fix:** `serveStatic` in `server/index.mjs` now decodes the path inside a `try`, answering 400
  `malformed path` when the decoder rejects it, and does so before checking whether `dist/` exists,
  so the answer does not depend on the SPA being built. The dispatcher wraps both routes in an error
  boundary — `failRequest` — that logs and answers 500 rather than letting anything escape, and the
  compile route's promise now carries a `.catch` into the same place instead of `void`. The static
  file stream also handles its own `error` event, since a read that fails after the headers are out
  arrives as an event no boundary can catch and an unhandled one on a stream ends the process too.
  Covered by
  `answers a malformed percent-escape and keeps serving` in the new `server/index.test.mjs`, which
  starts the real server on a free port, requests `/%ZZ`, asserts 400, and then proves a following
  request is still answered.

- **Verification:** Fix confirmed. `decodeURIComponent` and `new URL` are both inside the `try`, the
  400 is answered before the `dist/` check so it does not depend on the SPA being built, the
  dispatcher wraps both routes, the compile promise carries a `.catch`, and the read stream handles
  its own `error`. `serveStatic`'s traversal guard still holds: `url.pathname` always begins with
  `/`, so `normalize` resolves every `..` at the root and `join(distRoot, requested)` cannot escape
  `distRoot` — the `startsWith` check is not load-bearing on its own but is not reachable in a
  bypassable state either. `statSync` can still throw on a race, and the dispatcher's boundary
  catches it. The new test passes and exercises the real process over a real socket.

### ✅ Verified - RF3 Zero-width source diagnostics are misclassified as whole-program faults
- **Severity:** Medium
- **Evidence:** `sample-apps/drawnui-react/server/compile.mjs:36-40` treats a label as positioned only
  when it covers at least one byte, and `server/compile.mjs:52-62` consequently discards zero-width
  spans. Compiling `let root() = { <SkiaLabel Text="hi" />` yields a compiler label at the visitor's
  line 1, column 39 with `startByte == endByte` and message `Expected } here`; the service returns it
  as `{ origin: "program", span: null }`. The fiddle therefore cannot place the required Monaco marker
  at the insertion point even though the compiler supplied the exact authored-source position.
- **Recommendation:** Distinguish a genuine whole-program sentinel by its known coordinates/context,
  not by span width. Preserve zero-width visitor spans (expanding to a one-column Monaco marker only at
  presentation time if necessary), and add a test for a missing token at EOF.
- **Fix:** `isPositioned` in `server/compile.mjs` no longer tests width. It tests for the
  whole-program sentinel instead — an empty span at byte 0, line 1, column 1, which is where
  `codegen-missing-semantic-data` reports and which in combined coordinates is the catalog's first
  character, so a diagnostic genuinely there is an application fault too and is reported the same
  way. An insertion point such as `Expected } here` therefore keeps its position and is shifted into
  the visitor's coordinates like any other. Because a Monaco marker with no width draws nothing,
  `NxEditor.tsx` widens an empty span by one column when building the marker; the span itself stays
  exact. Covered by `marks an insertion point, which the compiler reports as an empty span` in
  `server/compile.test.mjs`, which asserts the reviewer's reproducer now arrives as `origin:
  "source"` at line 1 with `startColumn === endColumn`. The existing whole-program test still
  passes, so the sentinel is still classified as `program`.

- **Verification:** Fix confirmed. `isPositioned` no longer inspects width; `isProgramSentinel`
  tests the exact coordinates `codegen-missing-semantic-data` reports (`startByte`/`endByte` 0,
  line 1, column 1), which a visitor diagnostic can never carry because the visitor's source starts
  at line `prefix.lines + 1`. The reviewer's reproducer now returns `origin: "source"` at line 1
  with `startColumn === endColumn`, and `NxEditor.tsx:83-93` widens exactly that empty case by one
  column when building the Monaco marker while leaving the span itself exact. The whole-program test
  still classifies the sentinel as `program`, so the branch was narrowed rather than removed.

## New Findings Discovered During 2026-09-03 03:02 Verification

Scope of this pass beyond verifying the three fixes: an independent review, from scratch, of every
change to NX itself — the tree-sitter grammar and external scanner, `nx-types` inference and
checking, `nx-codegen` tests, the TypeScript IR runtime and its tests, and the documentation edits.
The DrawnUI sample app was out of scope here except where RF1–RF3 land in it.

### ✅ Verified - RF4 A component body cannot see its inherited props, so reads of them go unchecked
- **Severity:** Medium
- **Evidence:** `crates/nx-types/src/infer.rs:537` binds `component.props` and `component.state`,
  and `nx_hir::Component::props` is documented at `crates/nx-hir/src/lib.rs:574` as *declared* props
  — the base component's props are not in it. Scope building does resolve an inherited prop, so no
  undefined-identifier diagnostic fires, and the type environment then has no binding for the name,
  so every use of it infers vacuously. Reading an inherited prop at an incompatible site reports
  nothing:

  ```nx
  abstract external component <Node />
  abstract external component <Base extends Node n:int />
  external component <Txt v:string />
  component <A extends Base /> = { <Txt v={n} /> }        // no diagnostic
  component <B extends Node n:int /> = { <Txt v={n} /> }   // "Property 'v' on 'Txt' expects string, found int"
  ```

  `<Txt v={n.nope} />` is likewise silent for the inherited `n` and reports `Member access not yet
  implemented: .nope` for the declared one. This is the central feature of the change:
  `specs/component-syntax/spec.md` requires that "the component's props and state SHALL be bound by
  name while its body is checked" and that "a binding site inside a component body SHALL be checked
  against the same declared type it would be checked against in a function body." An inherited prop
  is a prop of the component and the body may legitimately read it, so both sentences are unmet for
  it. It matters in practice because the DrawnUI catalog is built almost entirely on inheritance
  from `SkiaControl`, so the bodies this change exists to check are the ones most likely to read an
  inherited name.
- **Recommendation:** Bind the component's *effective* props — walk the base chain the way
  `component_contract_of` / `component_lineage` already do in `infer.rs`, or reuse the
  `effective_props` the lowering pass computes — rather than `component.props`. Add the inherited
  counterpart of `property_type_mismatch_in_a_component_body_is_reported` to
  `crates/nx-types/tests/component_body_checking.rs`; the existing
  `an_inherited_record_field_reads_like_a_declared_one` covers inherited *record* fields, which is a
  different path, and passes either way.
- **Fix:** `infer_component` now binds the component's effective props — the contract
  `effective_component_contract_for_name` resolves — before its declared fields, resolving each
  inherited field's type in the module that declared it, the way an inherited record field is
  already resolved. Declared props are still bound afterwards, so nothing changes for them when the
  contract cannot be resolved. Three tests added to `crates/nx-types/tests/component_body_checking.rs`:
  an inherited prop at a mismatched site is reported, it reports *the same* diagnostic a declared
  prop reports, and an inherited prop at a matching site stays clean. Measured the cost, since the
  contract is resolved once per component: `test_typecheck_performance_large` (5,000 components) is
  unchanged within noise — 1.17–1.50s with the lookup, 1.46–1.62s with it removed.
- **Verification:** Verified, and causally rather than by assertion: the reproducers were run against
  both this tree and a `HEAD` worktree built for the purpose. The finding's own program —
  `component <A extends Base /> = { <Txt v={n} /> }` with `n:int` reached through `Base` — is silent
  at `HEAD` (it evaluates to `<A n=1 />`) and now reports `Property 'v' on 'Txt' expects string,
  found int` at the binding site, which is the same diagnostic the declared-prop spelling gets.
  `infer_component` binds the props of the contract `effective_component_contract` resolves, each
  inherited field's type resolved in the module that declared it, and falls back to the declared
  props when no contract resolves, so nothing changes where there is no base chain to read. The
  three named tests are present in `crates/nx-types/tests/component_body_checking.rs` and pass.

### ✅ Verified - RF5 Prop and state defaults are checked in declaration order, so a forward reference escapes the check
- **Severity:** Medium
- **Evidence:** `crates/nx-types/src/infer.rs:537-551` checks each field's default and *then* binds
  the field, inside one pass over `props.chain(state)`. A default that names a field declared later
  therefore resolves against an environment that has no binding for it and is accepted whatever its
  type:

  ```nx
  component <A extends Node a:string = {b} b:int = 1 />   // no diagnostic
  component <A extends Node b:int = 1 a:string = {b} />   // "Default value for 'A.a' expects string, found int"
  ```

  The same hole covers a state default naming a later state field, and — because props are chained
  ahead of state — *every* prop default that names a state field. Scope building accepts these
  references, so they are legal spellings whose types are simply never checked.
  `specs/component-syntax/spec.md` states the requirement unconditionally: "A prop or state default
  SHALL be checked against the declared type of the field it defaults."
- **Recommendation:** Split `infer_component` into two passes over the same list — bind every prop
  and state field first, then check the defaults — so a default is checked identically wherever it
  appears in the declaration. Add a test with the offending field declared after the default that
  uses it.
- **Fix:** `infer_component` is now three passes over the same fields — bind the effective props,
  bind every declared prop and state field, then check the defaults — so a default is checked
  against an environment that already holds every field the component declares. Both spellings in
  the finding now report `Default value for 'A.a' expects string, found int`, and a test asserts the
  two declaration orders produce identical diagnostics rather than merely both producing one. The
  prop default naming a state field is covered by its own test.
- **Status:** Superseded in part by RF10, which found what this fix bought and what it cost. Order
  independence made a *matching-type* forward reference look checked while neither runtime could run
  it, so the rule is now the one both runtimes actually implement: a default sees the fields
  materialized before it, and a name from later in the declaration is an undefined identifier. What
  this finding asked for still holds — a default is checked against its own declared type wherever it
  appears, and no default goes unchecked because of where it sits. The two tests asserting the two
  orders produce identical diagnostics were replaced by tests for the new rule.
- **Verification:** Verified as amended by RF10, which is the right way to read it: the hole this
  finding names is closed, though not by the order independence the fix first bought. Both spellings
  now diagnose — `a:string = {b} b:int = 1` reports `Undefined identifier 'b'` at the name, and
  `b:int = 1 a:string = {b}` reports `Default value for 'A.a' expects string, found int` — where at
  `HEAD` the first spelling was accepted and failed only when it ran, with `Runtime error: Undefined
  variable: b`. A prop default naming a state field is reported the same way rather than accepted
  vacuously. So no default escapes checking because of where it sits, which is what the finding
  asked for; see RF10's verification for the rule that replaced order independence.

### ✅ Resolved - RF6 The residual RF1 raises: compilation is still synchronous with no deadline
- **Severity:** Medium
- **Evidence:** Promoted out of RF1's status note so it stays tracked rather than buried in a
  resolved finding. `sample-apps/drawnui-react/server/compile.mjs:135` still calls into the native
  compiler on the Node server's only thread, with no deadline and nothing that can interrupt it. The
  scanner defect that made this reachable is fixed and fuzz-tested, so no known input triggers it;
  the structural property — one non-terminating or merely slow compile stops the service for every
  visitor — is unchanged. RF1's note is right that a worker *thread* would not help, because
  `terminate()` cannot preempt a native call that never returns to JavaScript; only a child process
  can be killed.
- **Recommendation:** This is the author's call, not the reviewer's, and the argument for doing
  nothing is real: `design.md` plans to move compilation into the browser via WASM and delete this
  server. Decide explicitly — either accept the exposure and say so in the README's deployment
  section, or put compilation in a killable child process with a server-side deadline. Do not ship
  it as an untrusted public fiddle without one of the two.
- **Status:** Left open, because which of the two to take is the author's call and not one a fixer
  should make silently. Half of the first option is done: the README's deploying section now states
  that `POST /api/compile` compiles synchronously on the server's only thread with no deadline, that
  only a child process could interrupt it, and what that means before untrusted traffic is pointed
  at it. That is documentation, not acceptance — the finding stays open until the author either
  accepts the exposure or asks for the child process, which is a contained piece of work: spawn the
  compile, kill it on a deadline, report the timeout as a diagnostic.
- **Status:** Decided by the author on 2026-09-03: the exposure is accepted for now. That is the
  first of the recommendation's two options, and it is already documented — the README's deploying
  section states that `POST /api/compile` compiles synchronously on the server's only thread with no
  deadline, that only a child process could interrupt it, and what that means before untrusted
  traffic is pointed at it. Nothing further is owed here. If the fiddle is ever put in front of
  untrusted traffic before compilation moves into the browser, this is the finding to reopen, and
  the work it names is a contained afternoon: spawn the compile, kill it on a deadline, report the
  timeout as a diagnostic.

### ✅ Verified - RF7 The server's liveness tests fail unless the SPA was built first
- **Severity:** Low
- **Evidence:** Both tests in `sample-apps/drawnui-react/server/index.test.mjs` end with
  `assert.ok(next.status < 500)`, but `serveStatic` answers `503` when `dist/` is absent — which is
  correct behaviour and proof the process is alive, the very thing the assertion is trying to
  establish. Moving `dist/` aside and running `node --test server/index.test.mjs` fails both tests
  with `the server should still be answering, got 503`. `npm test` does not build, so on a fresh
  checkout the suite fails for a reason unrelated to what it tests, and a future real regression
  here reads as the same failure.
- **Recommendation:** Assert what the tests mean — that a response arrived at all (the `fetch`
  resolving is itself the liveness proof) — or accept `503` explicitly as a live answer. Keep the
  `/%ZZ` assertion at `400`, which already holds without a build because the decode is checked
  before the `dist/` lookup.
- **Fix:** Both tests now call `assertStillServing`, which fetches `/`, drains the body, and asserts
  only that a status came back — the exchange completing is the liveness proof, since a dead process
  refuses the connection and `fetch` rejects. Verified the way the finding was: with `dist/` moved
  aside, `node --test server/index.test.mjs` passes both tests where it failed both before. The
  `/%ZZ` assertion stays at `400`.
- **Verification:** Verified the way the finding was found. With `sample-apps/drawnui-react/dist`
  moved aside, `node --test server/index.test.mjs` passes both tests, where the finding reproduced
  both failing at `503`. `assertStillServing` fetches `/`, drains the body, and asserts only that an
  integer status came back, so the exchange completing is the proof rather than the status; the
  `/%ZZ` assertion is still `400`. `dist/` was restored afterwards and the app's full `npm test` (12
  server tests, 12 example checks) passes.

### ✅ Verified - RF8 A mismatched record discriminator from a host is silently relabelled
- **Severity:** Low
- **Evidence:** `runtime/typescript/src/index.ts:919` strips `$type` from an incoming record value
  and stamps the declared type's name in its place. Dropping it is what
  `specs/typescript-ir-runtime/spec.md` asks for, and the reason is sound — record construction
  stamps a discriminator the declared field list cannot account for. But the same code path serves
  host input through `constructComponentDescriptor`, so `{ $type: "SomethingElse", name: "Ada" }`
  supplied for a `User`-typed prop is accepted and returned as `{ $type: "User", name: "Ada" }`. The
  spec's own "Public boundary validation still rejects malformed host input" scenario does not list
  this case, so this is a hardening gap rather than a contradiction.
- **Recommendation:** Ignore `$type` when it is absent or already names the declared type, and
  reject it with `nx-ir-boundary-type` when it names a different one. The union branch immediately
  below (`index.ts:933-946`) is the precedent: it validates the discriminator against the declared
  type and only then strips it before normalizing fields. That keeps every scenario the spec
  requires and closes the boundary.
- **Fix:** The record branch reads `$type` before dropping it: absent is accepted, a discriminator
  equal to the declared type's display name is accepted, and anything else fails with
  `nx-ir-boundary-type` — `Expected Card props.owner to be a User, got 'Ghost'.` Both cases asserted
  in the existing record test, and the requirement plus two scenarios added to
  `specs/typescript-ir-runtime/spec.md`.
- **Status:** One limit of the fix, recorded rather than left to be discovered: it cannot accept a
  *subtype's* discriminator, because `NxIrRecordDeclaration` carries fields and no base, so this
  runtime cannot tell a derived record from a foreign one. Nothing regressed by it, though — record
  inheritance does not survive this runtime today for an unrelated reason. `type User extends
  Entity` bound to an `Entity`-typed field evaluates under the interpreter and already failed here
  with `Unknown User field 'id'`, because the IR's record declaration lists declared rather than
  effective fields. That gap is real, predates this change, and wants a finding of its own.
- **Verification:** Verified by running the three host inputs through the runtime against IR emitted
  for `type User = { name:string } external component <Card owner:User />`: a plain object with no
  discriminator is accepted; `{ $type: "User", ... }` is accepted and comes back stamped `User`; and
  `{ $type: "Ghost", ... }` is rejected with `nx-ir-boundary-type: Expected Card props.owner to be a
  User, got 'Ghost'.` That is exactly the three-way behaviour the recommendation asked for, and it
  is asserted in the record test. One note on the evidence rather than on the fix: the same three
  inputs run against a `HEAD` build reject *any* discriminator with `Unknown Card props.owner field
  '$type'`, because the `$type` strip the finding describes was itself part of this change and
  predates the pass that opened the finding. `HEAD` is therefore not this finding's before-state and
  cannot serve as its causal control; what it does show is that the accepting half of the fix —
  a value's own discriminator — is new behaviour too, not just the rejecting half.

### ✅ Resolved - RF9 An uncommitted scratch file marked "not meant to be kept" is in the working tree
- **Severity:** Low
- **Evidence:** `docs/scratch-highlighting.nx` is untracked and its own second line reads "Not real
  UI, and not meant to be kept — delete when done testing." A `git add -A` before archiving would
  commit it into `docs/`.
- **Recommendation:** Delete it, or move it under a path the repository already ignores, before the
  change is committed.
- **Status:** Left open deliberately. Deleting an untracked file destroys the only copy, and this
  one was written during this work, so whether it is still wanted is the author's to say rather than
  mine to infer from a comment inside it. The risk it names is narrow and easy to avoid meanwhile:
  stage with explicit paths rather than `git add -A`. Say the word and I will delete it.
- **Status:** Closed by the author on 2026-09-03: the file stays for now and will be removed by hand
  when it has served its purpose. It is untracked, so it reaches the change only through a
  `git add -A`; staging with explicit paths is the whole of what this needs.

### Notes that are not findings
- `crates/nx-syntax/tests/performance_tests.rs:92` (`test_parse_performance_large`, threshold 10,000
  lines/sec on a debug build) failed once here at 9,429 lines/sec inside a full `cargo test
  --workspace` run and passed on every standalone run and on a second full workspace run. It is
  pre-existing, untouched by this change, and load-sensitive — but it is why a `cargo test
  --workspace` result should be read from the exit code rather than from a piped tail, which reports
  the pipe's status instead.
- The committed generated artifacts are consistent: `tree-sitter generate` reproduces `src/parser.c`
  and `src/grammar.json` unchanged, and `runtime/typescript/dist/` matches its sources after `npm
  test` (which runs `tsc`).
- `docs/displaylist-proposal/` and `docs/drawnui-proposal/samples/` are new untracked proposal
  content, small and self-consistent; nothing compiles or tests against them.
- The documentation edits are accurate against the implemented behaviour, including the honest note
  in `docs/drawn-ui-proposal-nx-enhancements.md` that component prop defaults are now type checked,
  so `external component <C x:float64 = 0 />` is rejected where it used to be accepted. Confirmed:
  that source now reports `Default value for 'C.x' expects float64, found int`.

## Questions
- None.

## Summary
- At review time, three findings were open: one high-severity service-availability issue and two
  medium-severity robustness/diagnostic issues.
- The declared Rust, runtime, sample-app, example, typecheck, and production-build checks otherwise
  pass. The implementation broadly matches the change artifacts, but the service should not be
  deployed as an untrusted public fiddle until RF1 and RF2 are addressed.
- The verification pass that followed opened six more findings. Four of those are now fixed, and RF6
  and RF9 are resolved by the author's decision rather than by a change: the compile exposure is
  accepted and documented, and the scratch file is being kept until its owner is done with it.
- This section records the state at the first pass. For where the report stands now, read
  "Verification pass — 2026-09-03 18:40" at the end: twelve findings, eleven closed, and RF11's
  substitutability half the only one still open.

## Fixes applied
All three findings are fixed and awaiting verification. RF1's cause turned out to be a compiler
defect rather than an application one — an external scanner that could not terminate at end of input
— so the repair is in `crates/nx-syntax/src/scanner.c`; read RF1's status note before treating the
availability concern it raises as closed, because the isolation the recommendation asks for was
deliberately not built.

Changed: `crates/nx-syntax/src/scanner.c`, `crates/nx-syntax/tests/parser_tests.rs`,
`sample-apps/drawnui-react/server/compile.mjs`, `server/index.mjs`, `server/compile.test.mjs`,
`server/index.test.mjs` (new), `src/editor/NxEditor.tsx`. Artifacts updated to match: an impact
bullet in `proposal.md`, section 1d and tasks 4.7 and 4.8 in `tasks.md`, and two scenarios in
`specs/drawnui-fiddle/spec.md`.

Verified: `cargo test --workspace` (40 suites, exit 0); the app's `npm test` (12 server tests, 12
examples compiling and evaluating), `npm run typecheck`, and `npm run build`; `openspec validate
add-drawnui-fiddle --strict`.

## Verification pass — 2026-09-03 03:02
- RF1, RF2, and RF3 are all verified fixed. RF1's compiler fix was confirmed causally by restoring
  the old scanner and watching its regression test hang, and by an independent 20,000-case fuzz.
- Six new findings were opened by the from-scratch review of the NX-side changes: RF4 and RF5 are
  real gaps in the component-body checking this change adds, RF6 is RF1's residual promoted out of a
  status note, and RF7–RF9 are low-severity test, boundary, and housekeeping items.
- Full checks re-run here: `cargo test --workspace` (48 suites, exit 0 — with the flaky perf test
  noted above), `runtime/typescript` `npm test`, the app's `npm test` (12 server tests, 12 examples),
  `tree-sitter generate` reproducibility, and `openspec validate add-drawnui-fiddle --strict`.

## Fixes applied — second pass, 2026-09-03 06:20
Four of the six new findings are fixed and awaiting verification: RF4, RF5, RF7 and RF8. RF6 and RF9
were left open for the author to decide, and both were decided on 2026-09-03 — RF6 by accepting the
compile exposure, which the README already states, and RF9 by keeping the scratch file until its
owner deletes it by hand. Neither needed a change; each carries the decision in its own status note.

Changed: `crates/nx-types/src/infer.rs` (effective props bound, defaults checked in their own pass),
`crates/nx-types/tests/component_body_checking.rs` (six tests, five of which fail without the fix),
`runtime/typescript/src/index.ts` and `test/runtime.test.ts` (record discriminator checked before it
is dropped), `sample-apps/drawnui-react/server/index.test.mjs` (liveness asserted by the exchange
completing), and `sample-apps/drawnui-react/README.md` (the compile exposure RF6 names, stated in
the deploying section).

Artifacts updated to match: two impact bullets reworded in `proposal.md`; tasks 1.7, 1b.9, 1b.10,
4.9 and 10.5 in `tasks.md`; the effective-props and declaration-order sentences plus two scenarios
in `specs/component-syntax/spec.md`; and the discriminator sentence plus two scenarios in
`specs/typescript-ir-runtime/spec.md`.

Verified: `cargo test --workspace` (exit code read directly, not through a pipe: 26 suites ok, the
one failure being `nx-syntax`'s `test_parse_performance_medium`, which fails on this machine at
HEAD too — 7,403 lines/sec with these changes stashed against a 10,000 threshold, so it is the
pre-existing load-sensitivity already noted above and not a regression); `runtime/typescript`
`npm test`; the native addon rebuilt so the app compiles through the changed type checker, then the
app's `npm test` (12 server tests, 12 examples compiling and evaluating), `npm run typecheck`, and
`npm run build`; `server/index.test.mjs` re-run with `dist/` moved aside; `openspec validate
add-drawnui-fiddle --strict`.

## New Findings Discovered During 2026-09-03 12:33 NX-Only Review

This pass reviewed only changes to NX itself: `nx-syntax`, `nx-types`, NX IR generation, and the
TypeScript IR runtime. The DrawnUI React website was intentionally excluded.

### ✅ Verified - RF10 Component defaults can compile to unresolved slots and fail only at runtime
- **Severity:** Medium
- **Evidence:** The RF5 fix pre-binds every declared prop and state field before checking any
  default at `crates/nx-types/src/infer.rs:557-590`, so a matching-type forward reference appears
  valid to inference. The undefined-identifier checker only visits the component body at
  `crates/nx-hir/src/scope.rs:329-362`; it never visits prop or state defaults. Code generation,
  however, builds each default before adding that field to its lexical scope
  (`crates/nx-codegen/src/builder.rs:464-492`), and both runtimes materialize defaults in declaration
  order. Consequently this successfully analyzes and generates NX IR:

  ```nx
  abstract component <Node />
  external component <Leaf extends Node />
  component <A extends Node a:int = {b} b:int = 1 /> = { <Leaf /> }
  let root() = { <A /> }
  ```

  The IR contains `{ "tag": "slot", "slot": "unresolved:b" }`; native evaluation then reports
  `Undefined variable: b`, while the TypeScript runtime reports `nx-ir-slot: Local slot 'b' was not
  bound`. Replacing `b` with a wholly undefined `{missing}` is also accepted by analysis and IR
  generation. Thus RF5 catches a mismatched forward reference but turns a matching one into a
  runtime failure, conflicting with the existing declaration-order initialization contract.
- **Recommendation:** Define one legal default-expression scope and use it in undefined-name
  checking, inference, code generation, and both runtimes. The smallest consistent choice is to
  expose props and state fields only after they have been materialized, rejecting forward/self
  references statically. If declaration-order-independent defaults are intended instead, implement
  dependency ordering and cycle diagnostics in both runtimes. In either case, make IR generation
  reject any `unresolved:*` slot and add matching-type forward, self-reference, and unknown-name
  regression tests.
- **Fix:** Took the first option — one scope for a default, used everywhere. A default now sees the
  fields materialized before it and nothing else: the effective props in declaration order, then the
  state. `UndefinedIdentifierChecker` visits defaults for the first time and defines each field only
  after checking its own default, so a forward reference, a self reference and an unknown name are
  all reported at the name, with a span. `infer_component` walks the same order for the same reason,
  so inference and scope checking agree rather than one accepting what the other rejects — which
  also means the RF5 fix's order independence is gone, deliberately: it made a forward reference
  *look* checked while the runtime could not run it. Naming an inherited prop or an earlier field
  still works, and `<A a:int = {b} b:int = 1 />` now reports `Undefined identifier 'b'` where it used
  to build IR that failed on evaluation.
- **Fix:** Code generation reports a name that reaches neither a binding nor a declaration instead of
  emitting `unresolved:<name>`. One legitimate case had to be kept: the base of a dotted import
  alias — `import { value as One.value }` binds the whole dotted name, so `One` is a spelling rather
  than a value — which is emitted unresolved as before, since the member's own reference is what
  runs. Tests: seven in `crates/nx-types/tests/component_body_checking.rs` covering forward, self,
  prop-naming-state, inherited, earlier-field and state-naming-prop, plus one in
  `crates/nx-codegen/src/tests.rs` asserting the program does not emit.
- **Verification:** Verified, and causally against a `HEAD` build. All three of the finding's static
  cases now report at the name with a span, where at `HEAD` each generated IR that failed only when
  it ran: the matching-type forward reference `a:int = {b} b:int = 1` reports `Undefined identifier
  'b'` (was `Runtime error: Undefined variable: b`), the self reference `a:int = {a}` reports
  `Undefined identifier 'a'`, and the unknown name `a:int = {missing}` reports `Undefined identifier
  'missing'`. The legitimate spellings still compile and evaluate: a default naming an inherited
  prop and one naming an earlier prop give `<A a=7 b=7 n=7 />`, a state default naming a prop and a
  module-level value gives `<A a=10 />`. The props-before-state order is enforced in the direction
  both runtimes materialize — a prop default naming a state field is reported, a state default
  naming a prop is accepted — and `UndefinedIdentifierChecker` and `infer_component` walk that same
  order, so scope checking and inference agree rather than one accepting what the other rejects. The
  code-generation half is in place at `crates/nx-codegen/src/builder.rs:801-816`, reporting
  `codegen-unresolved-name` where an `unresolved:<name>` slot used to be emitted, and the two
  existing tests asserting no `unresolved:` reaches the output still pass, so the dotted-import-alias
  case the fix note deliberately keeps was not broken by the guard.

### ✅ Verified - RF11 Generated IR drops record and union inheritance, breaking native-runtime parity
- **Severity:** High
- **Evidence:** Record declarations and record-construction expressions are built from only the
  declaration's local `record.properties` at `crates/nx-codegen/src/builder.rs:289-298` and
  `crates/nx-codegen/src/builder.rs:1797-1809`; the NX IR record declaration carries fields but no
  base relationship. Union cases have the same local-field shape. The TypeScript runtime therefore
  cannot recover an effective inherited schema. For:

  ```nx
  abstract type Base = { name:string }
  type User extends Base = { role:string }
  let root() = { <User name="Ada" role="admin" /> }
  ```

  native evaluation returns `{ "$type":"User", "name":"Ada", "role":"admin" }`, but evaluating
  the successfully generated IR fails with `nx-ir-boundary-field: Unknown User field 'name'`. If
  `Base.name` instead has default `"anon"` and the construction omits it, the TypeScript runtime
  succeeds but silently returns only `{ "$type":"User", "role":"admin" }`, dropping the inherited
  default. A union case extending an abstract record behaves the same way. Binding a `User` to a
  `Base`-typed field also cannot work correctly because `normalizeNominalValue` has neither ancestry
  nor an effective derived schema with which to distinguish a valid subtype from the foreign
  discriminator RF8 now rejects. This violates the record-inheritance effective-field/default and
  substitutability contracts, plus this change's requirement that valid generated IR match native
  evaluation without `nx-ir-boundary-*` failures.
- **Recommendation:** Preserve inheritance semantics in NX IR. Either emit effective fields and
  defaults for each record/union case plus enough ancestry metadata to validate subtypes, or encode
  bases and resolve effective schemas in the runtime. Normalize a derived value using its derived
  effective schema, preserve its discriminator and fields when accepted at a base-typed boundary,
  and add native-versus-TypeScript parity tests for an explicit inherited field, an inherited
  default, a derived value passed as an abstract base, and a union case with shared base fields.
- **Fix:** Took the first option's first half — the IR now carries effective fields. A record
  declaration, a record construction and a union case each emit the base chain's fields before their
  own, with each inherited field's default built in the module that declared it and its type
  resolved there, reusing the machinery `build_effective_component_fields` already used for props
  (components have always emitted effective props; records were the inconsistency). The finding's two
  reproducers now agree with the interpreter exactly: `{"$type":"User","name":"Ada","role":"admin"}`
  where the runtime used to report `Unknown User field 'name'`, and the inherited default no longer
  vanishes. A union case carries its base's fields the way the interpreter materializes them, base
  first. Three parity tests added to `runtime/typescript/test/emitted-ir.test.mjs`.
- **Status:** Substitutability is *not* fixed, and I am leaving that half to the author rather than
  guessing. Passing a derived value at a base-typed field still fails, because the IR carries no
  ancestry and `normalizeNominalValue` therefore cannot tell a subtype from the foreign discriminator
  RF8 rejects. Emitting effective fields does not foreclose the ancestry work — it is what the
  derived side of that check needs anyway — but the shape of the metadata, and whether the runtime
  should validate a subtype at all rather than the compiler, is a design decision worth making
  deliberately. It is the last of RF11's four parity cases, and the only one still open.
- **Verification:** Reopened, on the fix note's own disclosure. Three of the finding's four parity
  cases are fixed, confirmed causally against a `HEAD` build; the fourth is not, and the finding's
  title is not yet true of every case it names.

  Fixed and verified — each disagreeing at `HEAD` and agreeing now:
  - an inherited field supplied explicitly: both runtimes give
    `{"$type":"User","name":"Ada","role":"admin"}`, where `HEAD`'s IR gave `nx-ir-boundary-field:
    Unknown User field 'name'`
  - an inherited default omitted at the construction: both give `name: "anon"`, where `HEAD`'s IR
    silently dropped the field
  - a union case over a base (`type Shape extends Base = | circle { r:int } | square { s:int }`):
    both give `{"$type":"Shape.circle","name":"c","r":2}` supplied and `"anon"` defaulted, where
    `HEAD`'s IR gave `Unknown Shape.circle field 'name'` and dropped the default

  Still failing — substitutability, the finding's fourth case. For

  ```nx
  abstract type Base = { name:string }
  type User extends Base = { role:string }
  external component <Card owner:Base />
  let root() = { <Card owner={<User name="Ada" role="admin" />} /> }
  ```

  native evaluation gives `{"$type":"Card","owner":{"$type":"User","name":"Ada","role":"admin"}}`
  while the emitted IR fails in the TypeScript runtime with `nx-ir-boundary-type: Expected Card
  props.owner to be a Base, got 'User'.` This is not a regression introduced by RF8's discriminator
  check: the same program already fails at `HEAD`, there with `Unknown User field 'name'`. It is the
  gap the fix note names — the IR carries no ancestry, so `normalizeNominalValue` cannot tell a
  subtype from a foreign discriminator — and the remaining work is a design call (what ancestry
  metadata to carry, and whether the compiler or the runtime should validate a subtype at all)
  rather than something a fixer should settle silently, the way RF6 and RF9 were left to the author.
  Nothing else is blocked on it: every other check passes, and the three parity tests added for this
  finding are real regression guards for the three cases they cover.
- **Fix (substitutability, fourth pass):** The IR now carries each record's and union's base chain,
  nearest first, as declaration references, and the TypeScript runtime uses it: a value whose `$type`
  names a record extending the expected one is normalized against *its own* schema and keeps its own
  discriminator, instead of being rejected as foreign. The finding's fourth reproducer now agrees
  with the interpreter exactly — both give
  `{"$type":"Card","owner":{"$type":"User","name":"Ada","role":"admin"}}` — and so does the union-case
  form, where `<Figure.circle r=2 />` at a `Shape`-typed property gives
  `{"$type":"Figure.circle","name":"anon","r":2}` on both sides.

  The chain holds declaration ids, not names, because that is the only identity that survives
  separate modules: two records named `Card` are two types, and only the id says which one a
  base-typed site meant. A record that does not extend the expected type is still rejected with the
  message RF8 introduced, so that check is preserved rather than widened.

  The author chose this over putting identity in the wire discriminator. `$type` is output a host
  reads and the interpreter emits the same names, so qualifying it would change every record value
  in both runtimes to buy disambiguation that an in-runtime tag could add later without touching the
  IR or the wire. What is left is the residue of that choice: a `$type` is a name, so where two
  declarations of that name extend the same base the runtime cannot tell them apart. It reports the
  ambiguity rather than guessing, since picking one would normalize against the wrong field list and
  hand back a quietly wrong value. Nothing here forecloses adding identity later.

  Tests: `nx_ir_records_and_unions_carry_their_base_chain` in `crates/nx-codegen/src/tests.rs`
  (nearest-first, past the immediate base, cross-module, and empty for a record with no base); two
  interpreter-parity cases in `runtime/typescript/test/emitted-ir.test.mjs` for the derived record
  and the union case at a base-typed property; and two in `runtime/typescript/test/runtime.test.ts`
  for a declared non-subtype still being rejected and for the shared-name ambiguity being reported.
- **Verification (fourth pass):** Verified, and causally. The finding's fourth case now agrees with
  the interpreter byte for byte — both runtimes give
  `{"$type":"Card","owner":{"$type":"User","name":"Ada","role":"admin"}}` where the third-pass tree
  failed with `nx-ir-boundary-type: Expected Card props.owner to be a Base, got 'User'` — and so does
  the union-case form, `{"$type":"Card","shape":{"$type":"Figure.circle","name":"anon","r":2}}`. With
  that, all four of the finding's parity cases hold and the finding is closed.

  The causal control was the IR itself rather than a `HEAD` build, which is a sharper instrument
  here: deleting `bases` from the emitted document and re-running the same program restores exactly
  the failure the finding named, on both the in-program path and the host path. The base chain is
  therefore load-bearing rather than incidental, and the emitted chain holds identities, not names
  (`User (m0:d1) bases=["Base:m0:d0"]`).

  Positions the fix note does not name were checked too, and all agree with the interpreter: a
  grandchild at a grandparent-typed property (`C extends B extends A` at an `A`-typed prop), so the
  chain is walked past the immediate base; a derived value at a nullable base-typed property; one
  derived value at a `Base[]` property, which still takes the singleton coercion; a derived value at
  a record *field* typed at the base; a derived value returned from a function declared `:Base`; and
  a base's own default applied to a derived value at a base-typed site (`name` defaults to `"anon"`
  on the `User`, not on a narrowed `Base`).

  RF8's check is preserved rather than widened, which was the risk in this fix: at a `Base`-typed
  boundary a declared non-subtype (`Ghost`) and an undeclared name (`Nope`) are both still rejected
  with `Expected Card props.owner to be a Base, got '...'`, and because a derived value is normalized
  against *its own* schema, an unknown extra field on it is still rejected. In NX source the
  non-subtype case never reaches the runtime at all — analysis rejects it with `Property 'owner' on
  'Card' expects Base, found Ghost`. The shared-name ambiguity is reported rather than guessed at,
  and the deferral behind that — `$type` is a name, and identity belongs alongside it rather than
  inside it — is written up in `specs/future.md` with the four call sites that make qualifying `$type`
  a language-wide contract change. The residue this pass could find is exactly the residue that
  write-up describes. `docs/nx-ir-format.md` matches what is emitted: `schemaVersion` is `2` in a
  freshly generated document.

### ✅ Verified - RF12 List aliases bypass singleton-list normalization and content binding
- **Severity:** Medium
- **Evidence:** `build_type_ref_with_prepared` emits every named alias as a nominal reference
  (`crates/nx-codegen/src/builder.rs:1832-1880`), while the IR represents a type-alias declaration
  with no target (`crates/nx-codegen/src/ir.rs:916-918`). The runtime consequently returns an alias
  value unchanged at `runtime/typescript/src/index.ts:960-962`, and `isListTypeRef` recognizes only
  direct arrays through nullable wrappers at `runtime/typescript/src/index.ts:785-790`. Two valid
  programs therefore disagree across runtimes: `type Ints = int[]` with a prop `xs:Ints` and
  `xs={3}` evaluates to `xs:[3]` natively but `xs:3` in TypeScript; a content prop typed through
  `type Items = Item[]` binds one child as `[child]` natively but as `child` in TypeScript. The latter
  directly violates the new requirement that one child of a list-typed content property remain a
  list—the property is semantically list-typed even though its spelling is an alias.
- **Recommendation:** Preserve or eliminate alias indirection before runtime normalization: either
  emit the resolved structural target in each IR type reference, or carry alias targets in IR and
  resolve them recursively with cycle protection. Use that same resolved type in both
  `isListTypeRef` and `normalizeValue`, and add parity tests for ordinary and content properties
  typed through single- and multi-hop nullable list aliases.
- **Fix:** Took the first option: `build_type_ref` resolves an alias to what it stands for, following
  it through the module that declared it and carrying the aliases already on the path so a cyclic
  one stops at the repeat rather than recursing. Both spellings are now the same program — `xs:Ints`
  given `xs={3}` evaluates to `[3]`, and a content property typed `type Items = Item[]` binds one
  child as a list — verified against the interpreter through a two-hop nullable alias on both an
  ordinary and a content property. Nothing was needed in the runtime: with the alias resolved,
  `isListTypeRef` and `normalizeValue` already see an array. An alias declared by a module reached as
  an interface rather than compiled from source keeps its nominal reference, since there is no target
  to read there.
- **Verification:** Verified, and causally against a `HEAD` build. All three cases the recommendation
  names agree between the runtimes now and disagreed at `HEAD`: `type Ints = int[]` with a prop
  `xs:Ints` given `xs={3}` gives `[3]` from both (`HEAD`'s IR gave `3`); a content property typed
  `type Items = Item[]` with one child binds `[{...}]` from both (`HEAD`'s IR bound the bare child);
  and a two-hop nullable alias `type Ints = int[] type MaybeInts = Ints?` given `xs={3}` gives `[3]`
  from both (`HEAD`'s IR gave `3`). A cyclic alias (`type A = B type B = A`) is rejected by analysis
  in both the interpreter and code generation with `Type alias 'A' forms a cycle`, so the alias walk
  cannot be reached with a cycle to recurse on.

### Questions for this pass
- None.

### Summary of this pass
- Three new NX findings are open: one high-severity IR/runtime inheritance failure and two
  medium-severity static-checking/runtime-parity failures. All three are now fixed and awaiting
  verification; see "Fixes applied — third pass" below, including the one half of RF11 left open on
  purpose.
- `cargo test -p nx-types -p nx-syntax -p nx-codegen` and `runtime/typescript`'s `npm test` pass;
  the focused cross-runtime probes above fail because those cases are not covered by the suites.
- No DrawnUI React website code was reviewed, and no implementation code was changed during this
  pass.

## Fixes applied — third pass, 2026-09-03 17:00
All three NX findings from the 12:33 pass are fixed and awaiting verification. Each was reproduced
first, against both runtimes, and each fix is checked by a test that fails without it.

RF10 changed a language rule rather than patching a symptom: a prop or state default now has one
scope — the fields materialized before it — and undefined-name checking, inference and code
generation all use it. That deliberately reverses the order independence the RF5 fix introduced two
passes ago, because order independence made a forward reference *look* checked while neither runtime
could run it. RF11 and RF12 are both IR-emission fixes: the IR now carries a record's, a
construction's and a union case's effective fields, and resolves type aliases to what they stand
for.

Changed: `crates/nx-hir/src/scope.rs`, `crates/nx-types/src/infer.rs`,
`crates/nx-types/tests/component_body_checking.rs`, `crates/nx-codegen/src/builder.rs`,
`crates/nx-codegen/src/tests.rs`, `runtime/typescript/test/emitted-ir.test.mjs`.

Artifacts updated to match: three impact bullets in `proposal.md`; tasks 1b.10 through 1b.13 in
`tasks.md`; the default-scope rule and three scenarios in `specs/component-syntax/spec.md`; the
effective-field requirement and two scenarios in `specs/record-type-inheritance/spec.md`; and the
alias sentence and scenario in `specs/typescript-ir-runtime/spec.md`.

Verified: every reproducer in RF10, RF11 and RF12 re-run against both runtimes and now agreeing;
`cargo test --workspace` (exit 0, read from the exit code); `runtime/typescript` `npm test` with
three new interpreter-parity cases; the native addon rebuilt and the app's `npm test` (12 server
tests, 12 examples compiling and evaluating), `npm run typecheck` and `npm run build`; `openspec
validate add-drawnui-fiddle --strict` at 86/86 tasks.

Still open after this pass: the substitutability half of RF11, which needs ancestry in the IR and a
decision about where a subtype should be validated. RF6 and RF9 are resolved — the author accepted
the compile exposure the README documents, and will delete the scratch file by hand — so nothing
else in this report is waiting on anyone.

## Verification pass — 2026-09-03 18:40
Verified the seven findings the second and third fix passes marked fixed: RF4, RF5, RF7, RF8, RF10,
RF11 and RF12. Six are verified; RF11 is reopened, because three of its four parity cases are fixed
and the fourth — substitutability — still fails, which its own fix note discloses and which this
pass reproduced.

Method: rather than reading the fixes and agreeing with them, each finding's reproducer was run
against both this tree and a `git worktree` built at `HEAD`, so a fix is credited only where the
before-state actually reproduces the finding and the after-state does not. Every NX-side reproducer
was run through both runtimes — the native interpreter via `nxlang run --format json`, and the
emitted NX IR via `nxlang codegen --target nx-ir` evaluated in `runtime/typescript` — and compared,
since parity between the two is what RF10, RF11 and RF12 are about. RF7 was re-run the way the
finding was found, with `dist/` moved aside. One correction to a finding's evidence rather than to
its fix is recorded in RF8's note: `HEAD` is not that finding's before-state, because the `$type`
strip it describes was itself part of this change.

- **Verified fixed (6):** RF4, RF5, RF7, RF8, RF10, RF12.
- **Reopened (1):** RF11 — inherited fields, inherited defaults and union-case base fields all match
  the interpreter now and did not at `HEAD`; a derived value passed at a base-typed field still
  fails in the TypeScript runtime with `nx-ir-boundary-type: Expected Card props.owner to be a Base,
  got 'User'.` Not a regression — the same program fails at `HEAD` too. What is left needs ancestry
  in the IR and an author's decision about where a subtype should be validated.
- **New findings (0):** nothing new was found. In particular, the RF10 rule change was probed for
  collateral damage — defaults naming an inherited prop, an earlier prop, a module-level value, and
  a state default naming a prop all still compile and evaluate — and none was found.

Checks re-run here: `cargo test --workspace` (48 suites, exit 0 read from the exit code; the
load-sensitive parse-performance test passed this time); `runtime/typescript` `npm test` (21 cases,
including the three new interpreter-parity cases); `sample-apps/drawnui-react` `npm test` (12 server
tests, 12 examples compiling and evaluating), `npm run typecheck`, and `npm run build`;
`server/index.test.mjs` re-run with `dist/` moved aside and `dist/` restored afterwards; `openspec
validate add-drawnui-fiddle --strict`. The native addon the app compiles through is newer than every
changed crate source, so the app's checks exercise the fixed type checker rather than a stale build.

Open after this pass: RF11's substitutability half, waiting on the author the way RF6 and RF9 were.
Everything else in this report is verified, resolved, or closed by an author's decision.

## Fixes applied — fourth pass, 2026-09-03 20:10
One finding was open: RF11's substitutability half, which the third pass left to the author because
it needed a design decision rather than a fix. The author asked for the best long-term design rather
than picking from the options offered, and it is now fixed.

The decision was to carry ancestry in the IR and resolve it by name at the boundary, and *not* to
put declaration identity in the wire discriminator. `$type` is user-visible output that the
interpreter emits identically, so qualifying it would change every record value in both runtimes;
the identity that is actually missing is needed only between constructing a value and handing it to
a base-typed field, and can be added inside the runtime later with no IR or wire change. The chosen
layer forecloses nothing, which the rejected one would not have.

Changed: `crates/nx-codegen/src/model.rs`, `crates/nx-codegen/src/builder.rs`,
`crates/nx-codegen/src/ir.rs`, `crates/nx-codegen/src/tests.rs`, `crates/nx-codegen/src/emit.rs`
(match arms only — the JS target emits structural TypeScript types and has no nominal boundary
check), `runtime/typescript/src/index.ts`, `runtime/typescript/test/runtime.test.ts`,
`runtime/typescript/test/emitted-ir.test.mjs`, and `docs/nx-ir-format.md`, whose title and
`schemaVersion` line still said `1` while both sides of the code had been at `2`.

Artifacts updated to match: two impact bullets in `proposal.md`; task 1b.14 in `tasks.md`; the
substitutability paragraph and three scenarios in `specs/record-type-inheritance/spec.md`; and the
discriminator paragraph — which had said a discriminator naming any other type is rejected, and is
no longer true of a subtype — plus two scenarios in `specs/typescript-ir-runtime/spec.md`.

Verified: both of RF11's remaining reproducers re-run against the interpreter and the TypeScript IR
runtime and now agreeing byte for byte; `cargo test --workspace` (48 suites, exit 0 read from the
exit code); `runtime/typescript` `npm test` (25 cases, four of them new); the native addon rebuilt
and the app's `npm test` (12 server tests, 12 examples), `npm run typecheck` and `npm run build`;
`openspec validate add-drawnui-fiddle --strict` at 87/87 tasks.

Nothing in this report is open. Every finding is verified, fixed and awaiting verification, or
closed by an author's decision.

## New Findings Discovered During 2026-09-03 21:20 Verification

### ✅ Verified - RF13 A host can construct an abstract record at a base-typed boundary
- **Severity:** Low
- **Evidence:** NX IR carries `is_abstract` for components only (`crates/nx-codegen/src/ir.rs:183`);
  a record declaration carries `fields` and now `bases`, but nothing that says the record is
  abstract. The TypeScript runtime therefore cannot tell an abstract base from a concrete one at a
  boundary declared at that base's type. For

  ```nx
  abstract type Base = { name:string }
  type User extends Base = { role:string }
  external component <Card owner:Base />
  ```

  `constructComponentDescriptor(program, "Card", { owner: { name: "x" } })` returns
  `{"$type":"Card","owner":{"$type":"Base","name":"x"}}`, and supplying `{ "$type": "Base", "name":
  "x" }` explicitly returns the same. NX itself rejects that value: `<Base name="x" />` fails
  analysis with `Cannot instantiate abstract record 'Base'`. So a host can obtain a value stamped
  with a type the language cannot construct, that no NX code can produce, and that a `$type`-branching
  consumer will not have a case for.
- **Notes:** Not reachable from NX source — analysis rejects abstract construction — and not a
  regression from the fourth pass. It follows from the no-discriminator rule RF8 introduced, which
  the fourth pass extended for subtypes without revisiting the abstract case. The runtime spec states
  that rule for a record-typed field without excluding abstract records
  (`specs/typescript-ir-runtime/spec.md`, the discriminator paragraph and the "A host record with no
  discriminator is accepted" scenario), so the spec permits it as written.
- **Recommendation:** Carry a record's abstractness in the IR the way a component's already is, and
  reject a value being normalized *as* an abstract record — discriminator absent or naming the
  abstract type itself — with `nx-ir-boundary-type`, while continuing to accept a discriminator that
  names a subtype of it. Exclude abstract records in the discriminator paragraph of
  `specs/typescript-ir-runtime/spec.md`, and add a host-boundary test for both spellings.
- **Fix:** Taken as recommended. A record declaration now carries `isAbstract` from HIR through
  `CodegenDeclarationKind::Record` to `NxIrDeclarationKind::Record`, the way a component's already
  travels; a union has no abstract modifier in the grammar, so it gains nothing. The TypeScript
  runtime rejects a value normalized *as* an abstract record with `nx-ir-boundary-type`, in all
  three spellings rather than the two the finding named: no discriminator, a discriminator naming
  the abstract type itself, and — the case the base chain introduced — a discriminator naming an
  *intermediate* abstract record, which passes the base check and is still a type with no values.
  `NominalShape` carries `isAbstract` so `resolveSubtype` can report that last one by name rather
  than as a type the program does not have. A concrete subtype is accepted exactly as before.
  Verified on the finding's own reproducer compiled by `nxlang codegen --target nx-ir`: both host
  spellings are now rejected and `{ "$type": "User", ... }` still normalizes through. Tests:
  `nx_ir_records_carry_whether_they_are_abstract` in `crates/nx-codegen/src/tests.rs`, and "rejects
  an abstract record at a base-typed field and accepts one extending it" in
  `runtime/typescript/test/runtime.test.ts`, covering all three rejections and the acceptance.
- **Verification:** Verified, and causally. On a three-level chain compiled through `nxlang codegen
  --target nx-ir` (`abstract type A`, `abstract type B extends A`, `type C extends B`, plus an
  unrelated concrete `Loose`), the emitted IR carries `A: isAbstract=true`, `B: isAbstract=true`,
  `C: isAbstract=false`, `Loose: isAbstract=false`, and all three rejections hold at a prop declared
  `v:A`: no discriminator gives `Expected Card props.v to be a concrete type extending A, got an
  object with no '$type' discriminator naming one.`, `$type: "A"` gives `... got abstract 'A'.`, and
  the intermediate `$type: "B"` gives `... got abstract 'B'.` The causal control is the IR itself:
  deleting `isAbstract` from the generated document and re-running accepts both again, returning
  `{"$type":"A","a":"1"}` and `{"$type":"B","a":"1","b":"2"}` — the exact values the finding named.

  The widening is the right one and it is real. The intermediate-abstract case is reachable only
  because RF11's fourth pass made a base chain walkable, so it could not have been found before that
  fix, and neither of the two spellings in the finding covers it.

  Nothing was over-rejected, which was this fix's risk, since it inserts a gate into the same path
  RF8 and RF11 share. A concrete subtype is still accepted and keeps its own fields
  (`{"$type":"C","a":"1","b":"2","c":"3"}`); an unrelated concrete record is still rejected with
  RF8's own message rather than the new one (`Expected Card props.v to be a A, got 'Loose'.`); a
  concrete record-typed property still takes a plain host object with no discriminator, which is the
  rule RF8 established and which this fix narrows only for abstract types; `null` at a nullable
  abstract-typed property is still `null`; and a union case at an abstract base's type is still
  accepted, since a union carries no abstract flag and its cases are values. Abstract *actions* are
  covered too, not just records — `abstract action Search` emits `isAbstract=true` and rejects a
  plain host object, while `<Submitted />` extending it passes. Every in-program parity case from
  RF11 was re-run and still matches the interpreter, so the gate does not fire on values the program
  constructs itself.

  `docs/nx-ir-format.md` documents the flag and correctly states that unions carry none.

## Verification pass — 2026-09-03 21:20
One finding was awaiting verification: RF11, whose substitutability half the fourth pass fixed after
the 18:40 pass reopened it. It is verified, and RF11 is now closed. One new low-severity finding was
opened.

Method: the fourth pass's own two reproducers were re-run against both runtimes, then extended with
six positions the fix note does not name — a grandchild at a grandparent-typed property, a nullable
base-typed property, a `Base[]` property given one value, a record field typed at the base, a
function returning at the base's type, and a base default reaching a derived value. The causal
control was the emitted IR rather than a `HEAD` build, which is sharper for this fix: deleting
`bases` from the generated document restores exactly the failure the finding named, so the base
chain is demonstrably what carries the fix. RF8's rejection path was re-probed in the same run,
since widening it was the risk this fix ran.

- **Verified fixed (1):** RF11.
- **Reopened (0):** none.
- **New findings (1):** RF13 — the IR does not record which records are abstract, so at a boundary
  declared at an abstract base a host object with no discriminator, or one naming the abstract type,
  is accepted and stamped with that abstract type's name, a value NX itself refuses to construct.
  Low severity, not reachable from NX source, and permitted by the runtime spec as currently worded.

Checks re-run here: `cargo test --workspace` (48 suites, exit 0 read from the exit code, including
`nx_ir_records_and_unions_carry_their_base_chain`); `runtime/typescript` `npm test` (25 cases, four
of them the fourth pass's); the native addon confirmed newer than every changed crate source, then
the app's `npm test` (12 server tests, 12 examples compiling and evaluating), `npm run typecheck`
and `npm run build`; `openspec validate add-drawnui-fiddle --strict`.

Open after this pass: RF13 only, which is low severity and needs an author's call on whether an
abstract record should be constructible from host input at all. Every other finding in this report is
verified or closed by an author's decision.

## Fixes applied — fifth pass, 2026-09-03 22:10
One finding was open: RF13, the abstract-record boundary hole the 21:20 pass opened. It is fixed as
recommended, and widened by one case the finding did not name — an intermediate abstract record,
which the fourth pass's base chain made reachable and which the two spellings in the finding do not
cover.

Changed: `crates/nx-codegen/src/model.rs`, `crates/nx-codegen/src/builder.rs`,
`crates/nx-codegen/src/ir.rs`, `crates/nx-codegen/src/tests.rs`,
`runtime/typescript/src/index.ts`, `runtime/typescript/test/runtime.test.ts`, and
`docs/nx-ir-format.md`.

Artifacts updated to match: two impact bullets in `proposal.md`; task 1b.15 in `tasks.md`; a
paragraph and a scenario in `specs/record-type-inheritance/spec.md`; and in
`specs/typescript-ir-runtime/spec.md` the no-discriminator sentence, which had said a value carrying
none is accepted without qualification and is no longer true of an abstract type, plus a new
paragraph and two scenarios.

Verified: the finding's reproducer compiled through `nxlang codegen --target nx-ir` and run against
the built runtime — `{ "name": "x" }` and `{ "$type": "Base", ... }` both rejected, `{ "$type":
"User", ... }` still accepted and keeping its own fields; `cargo test --workspace` (48 suites, exit
0 read from the exit code); `runtime/typescript` `npm test` (26 cases, one of them new); the native
addon rebuilt and `bindings/node` `npm test` (11 tests); the app's `npm test` (server tests and 12
examples), `npm run typecheck` and `npm run build`; `openspec validate add-drawnui-fiddle --strict`
at 88/88 tasks.

Nothing in this report is open. Every finding is verified, fixed and awaiting verification, or
closed by an author's decision.

## Verification pass — 2026-09-03 23:00
One finding was awaiting verification: RF13, which the fifth pass fixed. It is verified, and RF13 is
now closed. No findings were reopened and none were newly opened.

Method: the finding's reproducer was extended to a three-level chain so the intermediate-abstract
case the fix added could be exercised, and every rejection was read off a document compiled by
`nxlang codegen --target nx-ir` rather than a hand-built one. The causal control was again the IR:
deleting `isAbstract` from the generated document restores exactly the two acceptances the finding
named. Because this fix inserts a gate into the path RF8 and RF11 share, the pass spent most of its
effort on over-rejection rather than under-rejection — concrete subtypes, unrelated records, plain
host objects at concrete record types, nullable absence, union cases at an abstract base, abstract
actions, and every in-program parity case from RF11.

- **Verified fixed (1):** RF13.
- **Reopened (0):** none.
- **New findings (0):** nothing new was found.

Checks re-run here: `cargo test --workspace` (48 suites, exit 0 read from the exit code, including
`nx_ir_records_carry_whether_they_are_abstract`); `runtime/typescript` `npm test` (26 cases); the
native addon confirmed newer than every changed crate source, then `bindings/node` `npm test` (11
tests), the app's `npm test` (12 examples compiling and evaluating), `npm run typecheck` and `npm run
build`; `openspec validate add-drawnui-fiddle --strict`.

Nothing in this report is open. All thirteen findings are verified or closed by an author's decision.
