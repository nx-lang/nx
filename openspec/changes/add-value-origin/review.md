# Review: add-value-origin

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/value-origin/spec.md,
specs/typescript-ir-runtime/spec.md, specs/rust-ir-runtime/spec.md  
**Reviewed code:** `git diff 133d2cf..HEAD` limited to `runtime/typescript/src/index.ts`,
`runtime/typescript/test/{runtime.test.ts,corpus.test.mjs}`, `runtime/typescript/README.md` (the
committed `dist` was rebuilt from `src` into a scratch directory and is identical),
`crates/nx-ir-runtime/src/{origins,value,eval,normalize,update,component,helpers,lib}.rs`,
`crates/nx-ir-runtime/tests/{origins,corpus}.rs`, `crates/nx-ir-runtime/README.md`,
`crates/nx-codegen/src/ir_corpus_tests.rs`, `specs/ir-conformance/**` (README and every
`expected/origins.json`), `docs/nx-ir-format.md`  
**Checks run:** `cargo test -p nx-ir-runtime`, `cargo test -p nx-codegen --lib ir_corpus`,
`node dist/test/runtime.test.js`, `node test/corpus.test.mjs` (all pass), and
`openspec validate add-value-origin --strict` (valid). To test paths the corpus does not cover, I
compiled a scratch program with `nxlang codegen --target nx-ir`. It has a component whose state
holds two records the program built (`held`, `spare`), renders `<Panel item={held} />`, and has an
owned handler `<Chooser onChosen=<Update held={action.item} /> />`. I drove it through both runtimes:
a Node script against `dist`, and a scratch crate against `nx-ir-runtime`.

## Findings

### ✅ Verified - RF1 The TypeScript runtime gives an origin to a record the host passes back into a dispatch, which the Rust runtime does not, so the two runtimes report different origins
- **Severity:** Medium
- **Evidence:** A dispatch given a report uses the instance's lineage map as the call's origin map
  (`runtime/typescript/src/index.ts:2281-2284`). Origins are keyed by object identity, and
  `readHostValue` keeps the host's objects as they are (`index.ts:2904ff`). `normalizeActionInput`
  (`index.ts:4282`) and `normalizeNominalValue` (`index.ts:4984`, `4997-5014`, `5026-5033`,
  `5059-5066`) then call `carryOrigin` with the host's object as `from`. The runtime gives the host
  its internal record objects in `state` (`{ ...state }` / `{ ...working }`) and on
  `instance.state`. So when a host passes one of those objects back in an action, the record looks
  like one the program built. Scratch repro: initialize `Holder` with a report, then dispatch
  `Chooser.Chosen` with `item: init.state.spare`. TS reports
  `/children/0/item → <Item name="spare" />`. A structural copy of the same value
  (`{ ...init.state.spare }`) reports nothing, and the Rust runtime reports nothing for the same
  call (`from_host` builds fresh records). This breaks "a record the host passed in SHALL have no
  origin" (`specs/value-origin/spec.md:18`) and "The IR runtimes report the same origins". It also
  makes the comment at `index.ts:2538` ("A value a host passes is never in one") untrue. The
  result depends on object identity, which a host cannot see.
- **Recommendation:** Don't carry origins from host input. One option is to have
  `normalizeActionInput`, and the normalization of a dispatched entry's fields, carry with no
  origin map, for example by passing an `evaluation` whose `origins` is `undefined` while
  normalizing batch input. The other is to return copies rather than internal objects in `state`.
  Add a TS test, plus a corpus lifecycle that passes a state record back, so `origins.json` pins
  the Rust answer.
- **Fix:** A dispatch now reads its batch as new objects whenever an origin map is in play (`freshHostValue` in `dispatchBatch`), as Rust's `from_host` does, so a record the host passes back is in no map; defaults built while normalizing it still get origins. The comment on `instanceOrigins` now says why. Tests: `a record the host passes back in a dispatch has no origin, even one the runtime handed it` (TS, fails without the fix) and `a_record_the_host_passes_back_in_a_dispatch_has_no_origin` (Rust, parity). The new corpus program `held-records` passes a host record in a `Chooser.Chosen` action, so `origins.json` pins the answer (no entry) for both runtimes. The spec now says a record the host passes back has no origin either, with a scenario.
- **Verification:** Verified. `dispatchBatch` now copies the batch with `freshHostValue` whenever an origin map is in play (`runtime/typescript/src/index.ts:2288-2293`). I reran my scratch repro: `init.state.spare` passed back and a structural copy of it both report no entry for `/children/0/item`, and the Rust probe also reports none, so the two runtimes agree. The new TS and Rust tests and the `held-records` batch 4 (`origins.json`: no `/children/0/item` entry) pin this. The spec now has the scenario. The copy itself causes a regression for host values that are not plain records; that is filed separately as RF7.

### ✅ Verified - RF2 Any dispatch without a report permanently drops the origins of every record held in state, in both runtimes
- **Severity:** Low
- **Evidence:** Every dispatch re-normalizes the whole state (`patchComponentState`,
  `index.ts:2456`; and the same in Rust), and normalization rebuilds each record. In a call
  without a report, the rebuilt record gets no origin. In Rust, `Machine::carried` returns `None`
  whenever the call does not track origins (`crates/nx-ir-runtime/src/eval.rs:297-302`), even
  when the record it rebuilds has one. In TS, a dispatch with a lineage but no report leaves
  `evaluation.origins` undefined (`index.ts:2281-2284`), so `carryOrigin` does nothing. Scratch
  repro in both runtimes: initialize with a report (`/children/0/item → <Item name="first" />`),
  dispatch once with a report (origin still there), dispatch once without, then dispatch with a
  report again. The `held` record now has no entry, although no call built it again. The spec
  says a record keeps its origin when "stored in or read from component state"
  (`specs/value-origin/spec.md:13`). The READMEs give the reason as "a record built during such a
  call has no origin later", but this record was not built in that call. A host that asks for a
  report only while a preview is visible loses selection for state-held records for the rest of
  the instance's life.
- **Recommendation:** Keep carrying origins that already exist, even in calls without a report,
  and only stop recording new ones. In Rust, drop the `tracks_origins` gate in `carried`: without
  a lineage the field is `None`, so this costs one `Option` clone. In TS, when a dispatch has a
  lineage and no report, let `carryOrigin` read and write the lineage while `originated` stays
  off. If the current behaviour is intended, say so in the spec and both READMEs ("a call without
  a report drops the origins of the records it re-normalizes, state included").
- **Fix:** Existing origins are now carried in every call, and only new ones need a report. Rust: `Machine::carried` no longer checks `tracks_origins`. TS: `Evaluation` has `recording` beside `origins`; a dispatch always uses its instance's lineage, `originated` records only when `recording`, and `carryOrigin` reads and writes the lineage either way. Tests: `a record held in state keeps its origin through a dispatch given no report` (TS) and `a_record_held_in_state_keeps_its_origin_through_a_dispatch_given_no_report` (Rust); both fail without the fix. The spec (new requirement text and a scenario), `docs/nx-ir-format.md`, both READMEs and the option doc comments now say that a rebuilt record keeps its origin in every call.
- **Verification:** Verified. Rust `Machine::carried` no longer checks `tracks_origins` (`crates/nx-ir-runtime/src/eval.rs:299-302`). TS dispatch always adopts the lineage, and `recording` gates only `originated` and the report walk (`index.ts:2284-2286`, `2556-2561`, `4234`). I reran my repro in both runtimes (initialize with a report, dispatch without, then dispatch with one): `/children/0/item → <Item name="first" />` survives in both. The no-report path for a host that never asks is unchanged: with no lineage, `evaluation.origins` stays `undefined`, nothing is copied and nothing is carried. The spec, READMEs and docs now state the rule, which answers my open question.

### ✅ Verified - RF3 No test covers origins held in state across a dispatch, or `callFunction` and `evaluateComponent` with a report
- **Severity:** Medium
- **Evidence:** No corpus lifecycle renders a record the program built and kept in state.
  `question-flow`'s questions are module-level `let`s that are evaluated again on every render.
  Its state holds only host answers and scalars, and the other lifecycle programs
  (`handlers`, `host-values`, `evaluation-cost`, `two-module`) hold only scalars or constant cases.
  So the riskiest code is never exercised: the TS `instanceOrigins` lineage
  (`index.ts:2281-2284`, `2367-2370`) and Rust's held `Record::origin` across dispatch. RF1 and
  RF2 are both on that path. The corpus tests in both runtimes (`tests/corpus.rs`,
  `test/corpus.test.mjs`) and the unit tests call only `evaluateFunction`, `initializeComponent`
  and `dispatchComponentActions`. `callFunction`/`call_function` and
  `evaluateComponent`/`evaluate_component` take the reporting path (`entryResult`,
  `reportedOutput`, `machine.report`) but nothing tests them. The docs also say a report "changes
  neither a call's operation count nor what it returns" (`docs/nx-ir-format.md`, both READMEs).
  The corpus check compares values with and without a report, but not `usage.operations`.
- **Recommendation:** Add a corpus program with a component whose state holds a record the
  program built and that it renders: the `Holder` shape above, or a `question-flow` variant that
  keeps the current question in state. Include a batch that does not change that field, so
  `origins.json` pins state-held origins in both runtimes. Add one `callFunction` and one
  `evaluateComponent` origin test per runtime. In the corpus `check_origins`/`checkOrigins`
  helpers, also compare the usage report with and without origins.
- **Fix:** New corpus program `specs/ir-conformance/held-records`: `Holder` keeps two records it built in its state and renders one. Its lifecycle taps a button that leaves the record as it is, swaps the two, taps again, stores one an owned handler builds, takes one from the host, and dispatches an empty batch. Its `origins.json` is regenerated, so both runtimes' corpus tests pin state-held origins across dispatch. New unit tests in both runtimes for `callFunction`/`call_function` and `evaluateComponent`/`evaluate_component` with a report. The corpus helpers (`checkOrigins`, `check_origins`) now also run under an unreachable budget and compare the usage report's operations with and without an origins report. `manifest.json`, the corpus README and the format document name the program.
- **Verification:** Verified. `specs/ir-conformance/held-records` keeps two built records in state and renders one. Its pinned origins go `first`, `first` (a tap that leaves it), `spare` (after the swap), `spare`, `fresh` (built by an owned handler), none (taken from the host), none (the empty batch). Both runtimes' corpus tests check these, with and without debug sections. The new unit tests cover `callFunction`/`call_function` and `evaluateComponent`/`evaluate_component`. `check_origins`/`checkOrigins` now compare `usage.operations` with and without a report. Everything passes: `cargo test -p nx-ir-runtime` (origins.rs 12/12), all of `cargo test -p nx-codegen` (357 lib tests, which include the interpreter-backed corpus results and the coverage manifest), `cargo test -p nx-interpreter`, and the TS `runtime.test.js`, `corpus.test.mjs`, `emitted-ir.test.mjs` and `bench/core.test.mjs`. The rebuilt `dist` matches `src`. Other corpus consumers are not affected. `damage.rs` sweeps the new program and passes. The bench harness, the SDK node, wasm and dotnet tests, the standard-library tests and the source-tree tests read fixed program lists, and none of those lists includes `held-records`.

### ✅ Verified - RF4 An invalid `usage` option leaves the previous call's origins entries in the report
- **Severity:** Low
- **Evidence:** `evaluate` calls `clearedUsage(options.usage)` before `clearedOrigins(options.origins)`
  (`runtime/typescript/src/index.ts:2575-2576`). When `usage` is refused with `nx-ir-options`,
  `origins.entries` is never cleared. Repro: `evaluateFunction(p, "root", [], { origins })`
  leaves 3 entries. `evaluateFunction(p, "root", [], { origins, usage: Object.freeze({}) })` then
  throws `nx-ir-options`, and `origins.entries.length` is still 3. The spec says every call given
  a report SHALL clear it when it begins, and a call that fails SHALL leave it empty
  (`specs/value-origin/spec.md`, "An IR runtime reports origins to a host that asks").
- **Recommendation:** Clear both reports before refusing either one: try to clear each, remember
  any failure, then fail. Add the case to the "cannot write to" test.
- **Fix:** `evaluate` now clears both reports before it refuses either: each clear runs, the first refusal is kept and thrown after both. Test: `a usage report the runtime cannot write to still leaves the origins report cleared` (fails without the fix). Only TS is affected; Rust's `Usage` cannot be refused.
- **Verification:** Verified. `evaluate` now runs both clears and throws the first refusal afterwards (`index.ts:2607-2622`). My repro (`{ origins, usage: Object.freeze({}) }` after a successful call) now leaves `origins.entries.length === 0`, and the new test pins it.

### ✅ Verified - RF5 The spec says an origin matches an element node, but `apply`/`merge`/`diff` and range origins are not element expressions, and nothing tests the match
- **Severity:** Low
- **Evidence:** `specs/value-origin/spec.md:75` and `:80` say the source tree node of an origin
  "is the element node" and that "exactly one element node" has its offsets. The same spec
  (`:15`) gives records built by `apply`, `merge` or `diff` the origin of the applying
  expression, and ranges the origin of the range expression. The tests confirm both:
  `apply(ada(), patch())` (`crates/nx-ir-runtime/tests/origins.rs`, `runtime.test.ts`) and
  `0.0..1.5` (the `ranges` library test). Neither is an element node, so for these origins the
  scenario "From preview to source tree" cannot hold. That scenario also has no test in this
  change or in the source-tree code.
- **Recommendation:** Reword the requirement to match on "the expression node" (or "the element
  or expression node"), or limit the "exactly one element node" scenario to element origins.
  Add one test that resolves corpus origins against the source tree's byte offsets. That can live
  with `add-source-tree`, but this change should name where it lives.
- **Fix:** The matching requirement now says an origin's node is the element node for a record an element constructed, and an expression node that is no element for a range or a record that `apply`, `merge` or `diff` built. "Exactly one element node" is limited to element origins, and a scenario covers the rest. New test in this change: `corpus_origins_are_the_byte_ranges_of_syntax_nodes` (`crates/nx-codegen/src/ir_corpus_tests.rs`, with `nx-syntax` as a dev-dependency). It parses each module and checks every recorded origin against the syntax tree, whose byte offsets the source tree carries: one element node for an element origin, and none but some node for the rest. A check against the language service's source tree itself is left to `add-source-tree`, which owns that tree.
- **Verification:** Verified. The requirement now names the element node for element origins and an expression node that is no element for ranges and `apply`/`merge`/`diff`, with a scenario for each. `corpus_origins_are_the_byte_ranges_of_syntax_nodes` checks every recorded origin in the corpus against `nx-syntax`'s tree and passes. Deferring the check against the language service's own source tree to `add-source-tree` is reasonable, because that tree carries the same byte offsets.

### ✅ Verified - RF6 A restored Rust component instance loses the origins of its state, and nothing documents it
- **Severity:** Low
- **Evidence:** The serialized form does not carry `Record::origin`, and
  `restore_component_instance` rebuilds records with `Value::record`, which has no origin
  (`crates/nx-ir-runtime/src/stored.rs:285`). After a restore, every state-held record reports no
  origin, which the spec's "stored in or read from component state" does not allow for. The
  README's origins section and the spec don't mention it. The TypeScript runtime has no
  equivalent API, so this is a Rust-only behaviour.
- **Recommendation:** Document it in the Rust README and the `rust-ir-runtime` delta: a restored
  instance's state records have no origin. Alternatively, serialize the module identity and node
  index when an origin is present.
- **Fix:** Documented rather than serialized: the `rust-ir-runtime` delta now states that the serialized form holds no origins, so a restored instance holds none, with a scenario. The README says so under *Storing an instance* and *Where records came from*, and so does `docs/nx-ir-format.md`. Test: `a_restored_instance_holds_no_origins` pins it, and checks that the original instance keeps them.
- **Verification:** Verified. The `rust-ir-runtime` delta, the Rust README (*Storing an instance* and *Where records came from*) and `docs/nx-ir-format.md` now say a restored instance holds no origins. `a_restored_instance_holds_no_origins` pins this and checks that the original instance keeps its origins.

## New Findings Discovered During 2026-10-10 09:02 Verification

### ✅ Verified - RF7 The RF1 fix copies every object in a dispatched batch, which breaks function and handler values the host holds and changes which diagnostic deep input fails with
- **Severity:** Medium
- **Evidence:** `freshHostValue` (`runtime/typescript/src/index.ts:2568-2580`) copies every
  non-array object it meets into a new plain object, recursively. It runs on the whole batch
  whenever the dispatch has an origin map (`index.ts:2291-2293`). That includes every dispatch
  whose instance was initialized with a report, even a dispatch given no report. But
  `readHostValue` accepts two kinds of non-plain object as host scalars and passes them through by
  identity: function values (`FunctionReferenceValue` instances, `isFunctionReference`) and
  handler values (members of the `madeHandlers` WeakSet) (`index.ts:3077-3087`). The runtime hands
  both to the host inside `state` and `instance.state`. `freshHostValue` loses their identity and
  then walks into `linked`, `declaration` and `captured`, which form a cyclic graph, until the
  stack overflows.
  - Scratch repro: a component with `state { op:Op = {double} spare:Op = {triple} }` and
    `<Picker onPicked=<Update op={action.op} /> />`. The host dispatches
    `Picker.Picked { op: init.state.spare }`. With neither call given a report it succeeds
    (`op` becomes `triple`). With a report on the dispatch, or only on the initialization (the
    dispatch itself given none), it fails with `nx-ir-resource-limit` "Maximum call stack size
    exceeded". Before the fix it succeeded in all four combinations.
  - The copy is also recursive, while `readHostValue` is deliberately iterative. A batch whose
    action carries an unknown field nested 200,000 lists deep fails with
    `nx-ir-boundary-field` without a report, and with `nx-ir-resource-limit` with one.
  - So a call's result now depends on whether this call or an earlier one asked for origins. That
    contradicts "its result SHALL be exactly what it is without this requirement" and the docs'
    "the report changes neither a call's operation count nor what it returns". Rust is not
    affected: its host values are `NxValue` data.
- **Recommendation:** Copy only the plain records and lists, the objects that can be keys of an
  origin map. Leave every host scalar as it is (`madeHandlers` members, `isFunctionReference`
  values, and anything else `isHostScalar` accepts). Make the copy iterative like
  `readHostValue`, or do it inside `readHostValue` when the call has an origin map, so no second
  pass is needed. An alternative that avoids copying: key the "host input" exclusion on a WeakSet
  of the objects `readHostValue` returned for this call, and have `carryOrigin` skip those. Add a
  TS test that dispatches a function value taken from `state` after an initialization given a
  report, with and without a report on the dispatch, and one for deep input.
- **Fix:** `freshHostValue` and its second pass are gone. The copy now happens inside
  `readHostValue` itself, through a new `fresh` flag (also on `readHostValues`), so it is the
  same iterative walk with its own stack. With `fresh`, every list and plain object is copied
  where the walk already copies, and the `isReadAsPassed` shortcut is skipped. Host scalars,
  function and handler values among them, are returned as they are, since the walk never enters
  them. `dispatchComponentActions` passes `fresh` when the call has an origin map or the instance
  has a lineage, the same condition as before.
  - The `held-records` program gains a `Tool` component whose state holds two function values,
    plus a `Picker` whose owned handler stores one passed in an action. Its corpus images and
    explanations are regenerated, and the existing `Holder` spans are unchanged.
  - A `Tool` lifecycle could not go in the corpus: the interpreter that records results refuses a
    `Function` record in dispatch input. So the case is pinned by unit tests in both runtimes
    instead.
  - TS test "a function value the host passes back from the state is read as it is, whatever asks
    for origins". It runs all four combinations of a report on initialization and on dispatch,
    and the rendered value must be 6.
  - TS test "deep input a dispatch keeping origins reads fails as it does without them". It uses a
    200,000-deep list in an unknown action field and expects `nx-ir-boundary-field` with and
    without a report.
  - Both TS tests fail against the previous fix ("Maximum call stack size exceeded") and pass now.
  - Rust parity test `a_function_value_the_host_passes_back_from_the_state_is_read_as_it_is` runs
    the same four combinations. Rust is unaffected, since its host values are `NxValue` data.
  - The deep-input case has no Rust counterpart. Rust refuses that input under its own
    `maxValueNesting` limit whether or not origins are tracked, which this change does not alter.
- **Verification:** Verified. `freshHostValue` is gone. `dispatchComponentActions` now reads the batch with `readHostValues(..., fresh)` when the call has an origin map or the instance has a lineage (`runtime/typescript/src/index.ts:2267-2268`). With `fresh`, the iterative walk in `readHostValue` skips the read-as-passed shortcut and returns a copy of every list and plain object (`index.ts:2935-3005`). Host scalars, which the walk never enters (handler values, `FunctionReferenceValue`s), stay as they are. I reran my repros. Dispatching `init.state.spare` (a function value) now succeeds in all four report combinations, and `op` becomes `triple` each time. The 200,000-deep unknown field fails with `nx-ir-boundary-field` with and without a report. RF1 still holds: the internal `state.spare` record and a copy of it both report no origin, and the Rust probe agrees. The RF2 and RF4 repros are unchanged too. Extra probes all give the same result with and without a report: an own `__proto__` member, a member set to `undefined` (and the host's object is left unmodified), and two batch entries. The new TS tests and the Rust parity test pass. The `held-records` `Holder` spans still resolve to the same elements. Also passing: `cargo test -p nx-ir-runtime` (origins.rs 13/13), `cargo test -p nx-codegen` (357), `cargo test -p nx-interpreter`, and the TS `runtime.test.js`, `corpus.test.mjs`, `emitted-ir.test.mjs` and `bench/core.test.mjs`. The rebuilt `dist` matches `src`.

## Questions
- RF2: is losing state origins after a call without a report intended? The READMEs' wording ("a
  host that reports from initialization on") hints that it is. If so, the spec should say so
  explicitly instead of "keeps its origin when stored in component state".
  - **Answer:** No. It is fixed (see RF2), and the spec now says a rebuilt record keeps its origin
    in every call.

## Summary
- Both runtimes implement the report as designed. They record origins only for element, record,
  union-case and component-descriptor nodes and for `apply`/`merge`/`diff`, collect entries in
  the host-conversion walk, clear the report at the start of a call and fill it only on success.
  Without a report nothing is recorded or collected, and results are unchanged; the corpus checks
  this in both runtimes, with and without debug sections. Corpus parity holds through
  `origins.json`, all tests pass, and the committed `dist` matches `src`.
- The main problem is one parity break outside the corpus (RF1): TypeScript tracks origins by
  object identity, so a host that passes a state record back gets an origin that Rust doesn't
  give. Persisting origins in state across dispatch, which this change exists to support, has no
  test at all (RF3). The rest are low-severity spec and doc consistency issues and an edge case
  in clearing the report.
- Verification (2026-10-10 09:02): RF1 to RF6 are verified, and the parity, state and clearing
  repros now give the same answer in both runtimes. The new `held-records` program breaks no other
  corpus consumer. One new issue is open: RF7, a regression from the RF1 fix. The TS batch copy
  breaks function and handler values the host holds, and changes the diagnostic for deep input,
  in any lineage that ever asked for origins.
- Second verification: RF7 is verified. All seven findings are now closed, and nothing new was
  found.
