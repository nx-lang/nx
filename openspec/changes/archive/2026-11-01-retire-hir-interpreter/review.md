# Review: retire-hir-interpreter

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/** (all 31 deltas), implementation-audit.md,
test-inventory.json, api-test-inventory.json, binding-test-inventory.json, proposal-review.md (context),
`complete-ir-construction-lowering` proposal/design/tasks (context only), docs/hir-interpreter-retirement.md
**Reviewed code:** `be7f870..d949683` (commits 0eee4f3 and d949683) plus the working-tree edits to
`crates/nx-api/src/{eval.rs,nx_text.rs}`, `crates/nx-api/tests/runtime_result_view.rs`,
`bindings/wasm/test/sdk-wasm.test.ts` and `scripts/verify-package.mjs`. Edits made during the review to
`crates/nx-cli/src/main.rs`, `bindings/wasm/test/execution-support.ts`, `tasks.md` and `validation.md` were only
glanced at. Focus areas: `crates/nx-api/src/{eval,format_value,nx_text,lib}.rs`,
`crates/nx-ir-runtime/src/{value,component,normalize}.rs`, `runtime/typescript/src/index.ts`,
`crates/nx-codegen/src/builder.rs`, `crates/nx-cli/src/main.rs`, `crates/nx-ffi/src/lib.rs`, `bindings/c/nx.h`,
.NET interop/ABI, Node/WASM native and TS wrappers, `crates/nx-api/tests/{support,semantics}/**`, corpus goldens,
crate-layering script, binding/API READMEs, value-view and website scripts.
**Method:** Read the code and compared the old CLI (`target/retire-validation/baseline-target/debug/nxlang`, built from
`be7f870`) with the current one (`target/debug/nxlang`) on the same inputs. Ran the website code-block checker
and the value-view fixture evaluator without writing anything. Ran a trial `openspec archive` in a scratch copy
of `openspec/` under `/tmp`. The repository and its validation logs were not changed.

The approved deferral of conditional property fragments and positional record constructors is not counted as a
defect. All six deferred tests are ignored with `complete-ir-construction-lowering` named and still assert
success. Removing the source execution APIs, old snapshot envelopes and C/.NET execution is intended. The
unchecked archive and final-validation tasks are not findings by themselves.

## Findings

### ✅ Verified - RF1 Value-view fixtures and the website code-block check still call the removed `artifact.evaluateNx()`, so `pnpm -r test` fails
- **Severity:** High
- **Evidence:** `packages/value-view/scripts/update-fixtures.mjs:24` (`return [name, artifact.evaluateNx()]`) is
  imported by `packages/value-view/test/fixtures.test.ts:6-14`, which is part of the package `test` script.
  `sites/website/scripts/check-code-blocks.mjs:129` (`text = artifact.evaluateNx().text`) runs from
  `sites/website/package.json` `test` for every `nx output` block (e.g. `docs/index.mdx`,
  `reference/syntax/functions.md`). Both fail now:
  - `node --no-warnings scripts/check-code-blocks.mjs` → `TypeError: artifact.evaluateNx is not a function` (exit 1).
  - `evaluateFixtures()` → `artifact.evaluateNx is not a function`.

  Task 4.3 ("verify … source paths require no removed method") is checked, but these in-repo callers were missed.
  Stale docs also remain in `packages/value-view/src/index.ts:4,200`, `src/types.ts:3` and
  `test/fixtures.ts:2`, which still describe values "as `evaluateNx()` returns it".
- **Recommendation:** Move both scripts to compile-then-run: `generateNxIr({ modules: [] })`, then prepare/link with
  `@nx-lang/ir-runtime`, `evaluateFunction(program, "root")`, then `artifact.formatValue(value, { type: <root's declared type> })`.
  Decide whether website `nx output` blocks should show canonical-host spelling or the CLI's rich spelling. The two
  differ for cases at object sites. Review any `values.json` or output-block diff instead of regenerating it
  blindly. Update the value-view doc comments, and add these two suites to the 6.4 validation list.

- **Fix:** Migrated both remaining script callers to all-module IR prepare/link/evaluate followed by formatValue at the schema root type, added their runtime dev dependencies, and updated value-view comments. These JavaScript consumers explicitly use canonical-host spelling. Value-view: 38 tests with unchanged fixtures; website: 13 tests and 251 code blocks pass.
- **Verification:** Both scripts now run the IR path: emit all modules, prepare/link with `@nx-lang/ir-runtime`, `evaluateFunction`, then `formatValue` at the root schema's `resultType`. Both declare `@nx-lang/ir-runtime` as a dev dependency, and the value-view doc comments no longer mention `evaluateNx()`. Rerun: `check-code-blocks.mjs` reports 251 blocks with 0 failures (exit 0). `evaluateFixtures()` gives output identical to the committed `values.json` (6 fixtures, compared with JSON.stringify), and `git diff` shows no fixture change. Cosmetic only: the new block in `check-code-blocks.mjs:130-140` is over-indented.

### ✅ Verified - RF2 The planned `openspec archive` (task 6.2) aborts, because the main specs were already synced by hand and `component-runtime-bindings` would rebuild as an empty spec
- **Severity:** Medium
- **Evidence:** Main specs were edited in the implementation commits (`openspec/specs/**`,
  `component-runtime-bindings/spec.md` deleted), and an empty `openspec/specs/component-runtime-bindings/` directory is
  left on disk. Running `openspec archive retire-hir-interpreter --yes` on a scratch copy prints many "treating it as
  already removed" warnings, then: `Validation errors in rebuilt spec for component-runtime-bindings … Spec must have
  at least one requirement — Aborted. No files were changed.` With that delta directory and the empty main directory
  removed, the same trial archive succeeds with "Specs already in sync; no files changed."
- **Recommendation:** Delete `specs/component-runtime-bindings/` from the change, because the capability is retired and
  its main spec is already gone. Remove the empty `openspec/specs/component-runtime-bindings/` directory, and say in
  proposal.md that the capability is removed. Alternatively, plan to archive with `--skip-specs`. Reword task 6.2,
  since its deltas already reached `openspec/specs`.

- **Fix:** Task 6.2 now explicitly archives with --skip-specs because deltas are already synced and the retired main capability is intentionally absent. Removed the empty main-spec directory. A scratch-copy archive using the documented command exits 0 (archive-review-fix.txt); final archive waits for this verification.
- **Verification:** `openspec/specs/component-runtime-bindings/` is gone, and task 6.2 now says to archive with `--skip-specs`. On a fresh scratch copy, `openspec archive retire-hir-interpreter --yes --skip-specs` exits 0, and `openspec validate --specs --strict` passes 95/95 afterwards. A sync check (a full archive on a scratch copy without the retired capability's delta) shows every delta already in main specs except the two requirements misfiled under REMOVED (see RF7). That is RF7's problem, not this archive path's.

### ✅ Verified - RF3 `nxlang run` prints IR-emission and some runtime failures without code, location or the CLI's diagnostic renderer
- **Severity:** Medium
- **Evidence:** In `crates/nx-cli/src/main.rs`, `execute_program` maps `emit_nx_ir` errors with `e.to_string()`.
  It does not use the existing `render_codegen_diagnostics` (used by the other codegen commands at lines 545/600/688).
  `runtime_error_text` formats `"{code} {location}: {message}"` with `unwrap_or_default()` for the location.
  Reproduced:
  - `<button if true { label="Save" } else { label="No" } />` → bare `conditional element property fragments are not
    supported by executable codegen yet` (no `error`, code or `file:line:col`, although the builder diagnostic has a span).
  - `<div content="x">hello</div>` → bare message, with the same loss.
  - int32 result out of range → `nx-ir-boundary-type : Expected return value for 'a' …` (stray separator, no module or
    declaration).
  - Recursion past depth 100 → `nx-ir-resource-limit : Maximum NX IR call depth 100 was exceeded.`

  Design decision 2 asks for the existing display with `nx-ir-*` codes and debug locations, and for a module or
  declaration when no source is available. The CLI test only covers the division-by-zero case, which has a location.
- **Recommendation:** Render emission diagnostics with `render_codegen_diagnostics`. Convert runtime diagnostics into
  `nx_diagnostics::Diagnostic`, labeled from `source`, and render them through `render_diagnostics_cli`, so static
  and runtime errors look alike. When there is no source, use `declaration`. When neither exists, print no empty
  location. Add CLI tests for an emission rejection, a boundary failure and a resource limit.

- **Fix:** Emission diagnostics keep code/span and use the CLI diagnostic renderer. Runtime diagnostics use the same renderer when a valid source span is present; source-free failures name their declaration or the known selected entry root, with no invented coordinates or empty separator. Prepare/link errors also use this path. Added emission, int32 boundary and recursion-limit cases; all 272 CLI tests pass.
- **Verification:** `execute_program` now renders emission diagnostics through `render_diagnostics_cli` with code and span. Prepare/link/runtime errors use the same renderer when the source span is valid, and otherwise print `error <declaration>: [code] message`. I reproduced:
  - the conditional fragment and the intrinsic content conflict both print `error c.nx:1:…` with a code and a source excerpt;
  - the int32 boundary failure and the call-depth limit print `error c.nx::root: [code] …`, with no stray separator;
  - division by zero keeps its location.

  The new CLI test passes in the workspace run. When the runtime gives no declaration, the fallback names the entry `root` that was being evaluated rather than the failing callee (e.g. `a`). The message still names the callee, so I accept that.

### ✅ Verified - RF4 Lifecycle coverage accounting is boilerplate, and at least one deleted behavior has no IR replacement test
- **Severity:** Medium
- **Evidence:** All 39 `redundant` entries in `api-test-inventory.json` have the same coverage string ("semantic
  equivalents covered by semantics/update_records.rs, semantics/function_values.rs, codegen/ir_runtime_tests.rs and
  codegen/ir_instance_tree_tests.rs"). None is mapped to a specific test. The deleted
  `component.rs::a_host_record_whose_lineage_only_shares_a_name_with_the_expected_base_is_rejected` checked that a host
  record whose base is a *different* same-named `Shape` in another module is rejected. I found no Rust or TypeScript IR
  test asserting that rejection: `resolve_subtype`'s module-target check is untested. Other cases cover only the
  positive cross-module path and an ambiguous duplicate name. Design §4/§6 makes coverage accounting, not test counts,
  the gate.
- **Recommendation:** Map each `redundant` entry to the named replacement test(s). Port the cases without one, at least
  the same-name lineage rejection through `call_function` or `initialize_component` in both runtimes.

- **Fix:** Replaced every generic lifecycle coverage entry with named replacement tests or a precise retired-adapter policy. Added module-origin lineage rejection in Rust and TypeScript, explicit-state numeric/polymorphic cases, nominal/property-union boundaries, malformed/untyped dispatch, constant-case effects and imported action/declaration/default coverage. Direct runtime component APIs select the declaring module explicitly for imported components. External flat-API synthetic initialization and blanket component-record conversion bans are classified as retired adapter policies, not falsely claimed as retained runtime operations.
- **Verification:** `api-test-inventory.json` no longer has boilerplate: 34 redundant entries with 13 distinct targets, and 12 interpreter-internal entries, each with a specific reason. Every `path::test` it cites exists. The new native tests (`a_host_record_with_only_a_same_named_base_is_rejected`, `component_boundary_widths_and_nominal_cases_are_validated`, `imported_component_actions_use_the_declaring_modules_payload_and_defaults`, `component_explicit_state_preserves_numeric_sites_and_polymorphic_results`) pass in `workspace-tests-final.txt`. I ran `emitted-ir.test.mjs`, which includes the TS same-named-base rejection, and it passes. The rejection test uses a base declared in another module that is not exported, and the matching positive case is accepted. Classifying the removed synthetic external-instance and blanket component-record bans as retired adapter policy is reasonable.

### ✅ Verified - RF5 Binding READMEs were edited mechanically: Node lost its retained-API examples, and .NET still documents the removed lifecycle and snapshots
- **Severity:** Medium
- **Evidence:**
  - `bindings/node/README.md:64,193,290,301`: the code samples under Importing, Program Artifacts, Diagnostics and
    Errors, and Resource Lifecycle were each replaced by "See the compile-then-run example below." That leaves those
    sections without any example of the retained APIs, and the "below" section is only a link. Line 286 still says
    "evaluation failures throw `NxEvaluationError`".
  - `bindings/dotnet/README.md:374-386` still describes initialization returning `StateSnapshot`, dispatch, evaluation
    with explicit state, MessagePack component inputs and base64 `state_snapshot`. Lines 179 and 189 still mention
    "evaluation or component lifecycle calls" and "immediate interpreter evaluation".

  Task 6.3 is checked.
- **Recommendation:** Restore Node examples for the retained build, workspace, IR, diagnostics and disposal APIs,
  without evaluation calls, and show the compile-then-run snippet once. Delete or rewrite the .NET lifecycle and
  snapshot bullets and the evaluation wording.

- **Fix:** Restored concrete Node imports, workspace/artifact generation, structured diagnostics, disposal and one compile-then-run example. Removed .NET opaque snapshot/lifecycle bullets and stale interpreter/evaluation wording; retained DTO/MessagePack guidance.
- **Verification:** .NET is fixed: the lifecycle/snapshot bullets and the interpreter/evaluation wording are gone. Node is not. The restored examples at `bindings/node/README.md:295` (Diagnostics), `:313` (Resource Lifecycle) and `:344` (Execute emitted IR) call `NxProgramArtifact.build(source, "main.nx")`, and no such method exists. The Node SDK exposes `NxProgramArtifact.buildSource(source, { fileName, buildContext })` (`bindings/node/src/index.ts:380`), so every copied example throws `TypeError`. Remaining work: switch to `NxProgramArtifact.buildSource("…", { fileName: "main.nx" })`, check that `modules.get("main.nx")` matches the identity that call emits, and ideally compile or doctest the snippets.

- **Fix:** Follow-up: corrected all three restored source examples to the actual Node buildSource(source, { fileName }) API; the compile-then-run snippet is executed directly against the built SDK to confirm main.nx selection and output 42.
- **Verification:** Second pass: all three Node examples (`README.md:295,313,344`) now call `NxProgramArtifact.buildSource(source, { fileName: "main.nx" })`, which matches `bindings/node/src/index.ts:380`. I ran the compile-then-run snippet against the built SDK (`dist/`) and it printed `42`, so `modules.get("main.nx")` matches the emitted identity. My scratch copy of the snippet was deleted.

### ✅ Verified - RF6 Results whose static type is float64 but whose IR carrier is an int changed their CLI JSON and object-site spelling, and the corpus goldens were rebaselined to the carrier
- **Severity:** Low
- **Evidence:** `let halfOr(o?:int) = { o ?? 1.5 }  let root() = { halfOr(1) }`:
  - `--format json`: old `1.0`, new `1`.
  - With `root(): object`: NX format old `1.0`, new `1`.

  The `conversions` (`joined*Division`, `joinedList*`) and `occurrence-lifting` (`presentWidened`) goldens were changed
  in the same direction. The audit attributes this to "existing IR numeric carriers": the emitter does not widen an
  int arm of a float64 join, so the Rust runtime carries `Int`. That is a quiet output difference against "unchanged
  invocation and output". `docs/hir-interpreter-retirement.md` does not list it.
- **Recommendation:** Prefer widening at emission when a join's static type is float64, so the Rust results match the
  static type and the original goldens return. Otherwise, record it as a declared output difference in the migration
  doc and the corpus README.

- **Fix:** Recorded the existing IR join-carrier spelling difference in the migration guide, corpus README and design. The explicitly reviewed goldens remain the documented existing-IR spelling; emitter widening is outside retirement.
- **Verification:** Resolved by documentation, which my recommendation allowed. `docs/hir-interpreter-retirement.md`, `specs/ir-conformance/README.md` and design.md each state that float64 joins selecting an integer arm keep the integer carrier and print `1` instead of `1.0` in JSON and at object sites, with the numeric value unchanged. They also note that the goldens record this.

### ✅ Verified - RF7 The new nested component validation changes both runtimes at every typed site, but no spec delta describes it and only the host-input path is tested
- **Severity:** Low
- **Evidence:** `normalize.rs:354-379` and `runtime/typescript/src/index.ts:5409-5418` now normalize any
  component-typed value whose `$type` is absent or names the declaration. Normalization runs for function parameters,
  results and typed `let` values (`eval.rs:962,1282,1336`), not only for host input. This fills defaults, rejects
  unknown props, re-splits `on<Emit>` handlers and charges per-field operations. The behavior is not in any delta
  under `specs/`. The Rust and TS tests (`semantics/update_records.rs:712`, `emitted-ir.test.mjs`) exercise only host
  initialization of `Wrap`. The cross-type early return keeps the previous subtype behavior, and I confirmed a
  program-built descriptor through a typed parameter and through a typed prop still gives the same output as before.
- **Recommendation:** Add a requirement and scenario, in `rust-ir-runtime`/`typescript-ir-runtime`, saying
  component-typed values naming their declaration are validated recursively. Add one program-internal parity case
  covering a descriptor with a defaulted prop and an `on<Emit>` handler through a typed parameter or result. Assert
  equal values and operation counts in both runtimes.

- **Fix:** Added matching main/delta requirements for recursive component descriptor normalization and a shared emitted-source fixture passing a descriptor with default and emit handler through typed parameters/results. Rust and TypeScript both assert the same canonical value and 16 operations; native-output parity passes.
- **Verification:** The tests are good. `runtime/typescript/test/retirement-descriptor.nx` passes a descriptor with a defaulted prop and a bound `onTap` handler through a typed parameter and result. Native `program_internal_component_descriptors_keep_defaults_handlers_and_cost` and the TS case in `emitted-ir.test.mjs` both assert the same canonical value and 16 operations, and both pass (I ran the TS suite). The spec delta is wrong, though. In `specs/rust-ir-runtime/spec.md:143` and `specs/typescript-ir-runtime/spec.md:320`, `### Requirement: Component descriptors are recursively validated at typed sites` sits under `## REMOVED Requirements`. A normal archive would therefore delete the requirement from both main specs: the scratch sync check printed "- 1 removed" for each and dropped the text. `--skip-specs` hides this from the archive, but the change would still record the requirement as removed. `openspec validate --strict` doesn't catch it. Remaining work: move both blocks into `## ADDED Requirements` in their deltas. The main-spec text can stay. Then rerun a non-skip scratch archive and confirm it reports "Specs already in sync".

- **Fix:** Follow-up: moved the new requirements into each existing ADDED section, before subsequent MODIFIED/REMOVED sections. Main-spec text remains unchanged. A normal scratch archive with only the already-retired capability delta omitted is checked for zero spec changes.
- **Verification:** Second pass: both `Component descriptors are recursively validated at typed sites` blocks now sit in their deltas' `## ADDED Requirements` sections (`rust-ir-runtime/spec.md:117`, `typescript-ir-runtime/spec.md:31`). A normal, non-skip archive on a fresh scratch copy (with only the retired capability's delta removed) reports `Totals: + 0, ~ 0, - 0` and "Specs already in sync; no files changed", and `git status` shows no change under `openspec/specs`. `openspec validate retire-hir-interpreter --strict` passes.

### ✅ Verified - RF8 The intrinsic `content=`-plus-body conflict is rejected only at IR emission, as "unsupported construct", and a website example now fails
- **Severity:** Low
- **Evidence:** `builder.rs:2331-2344` reports this through `unsupported_diagnostic` (`codegen-unsupported-construct`).
  That code means "not supported yet", but this is invalid source. Analysis, validation, the language service and
  `buildProgramArtifact` still accept the source. The complete example in
  `sites/website/src/content/docs/language-tour/elements.md:24-34` (and `reference/syntax/elements.md`) now fails IR
  emission in the playground. `specs/future.md:1586-1593` still says the collision "is found only by the interpreter".
- **Recommendation:** Report it as a checker diagnostic with its own code, or at least use a non-"unsupported" code.
  Update the `future.md` entry, and fix or mark the website examples it names.

- **Fix:** Emission now reports intrinsic-content-conflict instead of unsupported codegen. Website examples use tip= so their body owns content; future.md accurately tracks moving the diagnostic into analysis. The website suite passes all code blocks.
- **Verification:** `builder.rs` now emits `intrinsic-content-conflict` with a primary label. `nxlang run` on `<div content="x">hello</div>` prints `error c.nx:1:1: [intrinsic-content-conflict] …` with an excerpt. Both website Tooltip examples use `tip=`, and the `future.md` entry now tracks moving the check into analysis. Minor overstatement: its last sentence says website output checks catch emission failures, but only blocks followed by an `nx output` block are executed, and these examples have none.

### ✅ Verified - RF9 `nxlang run` now uses the runtime's default call depth of 100, which rejects recursion the HIR CLI ran, and this is not documented
- **Severity:** Low
- **Evidence:** `down(105)` and `down(120)` return `0` with the baseline CLI. The new CLI gives `nx-ir-resource-limit …
  call depth 100`. In a debug build, `sum(150)` also hits the 1 MiB stack budget. Per the removed WASM comment, HIR's
  default was 1,000. Design §2 allows the runtime defaults, but the migration doc mentions only the operation budget.
- **Recommendation:** Document the call-depth and stack limits that now apply to `nxlang run` in
  `docs/hir-interpreter-retirement.md` and the CLI help. Or choose an explicit CLI call depth after measuring stack
  use, and record why.

- **Fix:** Documented the 100-call and 1 MiB stack defaults alongside the operation budget in the migration guide and run help. The runtime defaults are retained as designed; recursion-limit diagnostics are tested.
- **Verification:** The migration doc names the 100-call and 1 MiB stack defaults, and `nxlang run --help` now says "Each run allows 1,000,000 IR operations, 100 nested calls and a 1 MiB evaluation stack." `down(120)` reports `[nx-ir-resource-limit] Maximum NX IR call depth 100 was exceeded.`, and the CLI test covers this case.

### ✅ Verified - RF10 Small leftovers: stale doc references and dead CLI plumbing
- **Severity:** Low
- **Evidence:**
  - `crates/nx-api/src/nx_text.rs:10` links to [`format_nx_text`], which is now `#[cfg(test)] pub(crate)`, so the
    rustdoc link is broken.
  - `nx_text.rs:2331` says "`evaluateNx` spells it".
  - `crates/nx-cli/src/main.rs` `RunOutput` formats NX and JSON eagerly, so every run does both. (A working-tree
    edit made during this review already removed `format_output`'s unused `_result_type` parameter and its stale
    doc comment.)
  - `bindings/wasm/src/host.ts:245,718` keep the blank lines left where `evaluateNx` was removed.
  - The working-tree edit to `sdk-wasm.test.ts` loosens the failure assertion from the `nx-ir-division-by-zero` code to
    message text.
- **Recommendation:** Fix the doc comments. Format only the requested output. Remove the blank lines. Assert on the diagnostic code (e.g. `error.diagnostics[0].code`) rather than message text.

- **Fix:** Removed stale formatter references, redundant WASM blank lines and the dead output type argument. CLI now returns the rich value and formats only the requested output; round-trip tests explicitly request both. WASM failure recovery asserts nx-ir-division-by-zero through structured diagnostics. Updated corpus/runtime test comments to describe IR execution.
- **Verification:** I confirmed all of the following:
  - The `nx_text.rs` module doc no longer links to `format_nx_text`, and the test doc now says `formatValue`.
  - `host.ts` has no doubled blank lines.
  - `format_output` takes the artifact and `FunctionValue` and formats only the requested output, so `RunOutput` is gone.
  - The WASM failure-recovery test asserts `diagnostics[0].code === "nx-ir-division-by-zero"`.
  - `execution-support.ts` links the actual entry identity instead of the first map entry.

## New Findings Discovered During 2026-11-01 02:05 Verification

### ✅ Verified - RF11 The duplicated website output-check block is mis-indented and the future.md note overstates what the website check executes
- **Severity:** Low
- **Evidence:** `sites/website/scripts/check-code-blocks.mjs:130-140` puts the new prepare/link/evaluate/format block
  at a deeper indent than the surrounding `try`, so it reads as if nested. The same sequence is copied in
  `packages/value-view/scripts/update-fixtures.mjs:28-38` and `bindings/wasm/test/execution-support.ts`. The new
  `specs/future.md` text ("Website output checks execute emitted IR and therefore catch emission failures") applies
  only to blocks followed by an `nx output` block, not to complete blocks such as the Tooltip examples.
- **Recommendation:** Reformat the block. Consider a small shared helper in each package's scripts, since all three
  copies must keep choosing the entry identity and the root `resultType` the same way. Reword the future.md
  sentence to say only blocks with an `nx output` block are executed.

- **Fix:** Aligned the website block with its surrounding try and clarified that only complete blocks followed by nx output are executed. Kept the small package-local script compositions; no production source-evaluation convenience API is introduced.
- **Verification:** Second pass: the block in `check-code-blocks.mjs:130-140` is now aligned with its `try`. `specs/future.md:1574` now says only website blocks followed by an `nx output` block execute emitted IR. Keeping three package-local copies is an acceptable choice. Each copy now selects the entry identity and root `resultType` the same way.

## Questions
- Answered by the fixes: website and value-view output use canonical-host `formatValue` spelling at the root's
  declared type (RF1), and `nxlang run` keeps the runtime's call-depth default, which is now documented (RF9).

## Summary
- The core retirement is sound:
  - The interpreter crate and its imports are gone, and the layering guard passes.
  - C header, FFI and .NET P/Invoke agree on ABI 14, with only fixed-JSON exports retained. WASM ABI 12 is matched by
    the loader.
  - The runtime result view is read-only and compiler-free.
  - Canonical and runtime inputs feed one formatter. The working-tree constrained-float32 fix is correct.
  - CLI output matched the baseline for the records, sequences, cases, functions and float32 sites I tried.
  - The approved deferrals are handled as agreed.
- Two problems should be fixed before the final checks:
  - In-repo callers of the removed `evaluateNx()` break `pnpm -r test` (RF1).
  - The archive step will abort as configured (RF2).
- CLI diagnostic rendering (RF3), coverage mapping (RF4) and README quality (RF5) are medium follow-ups for this
  change. The rest are low-severity spec, doc or polish items.
- Verification (2026-11-01):
  - Verified: RF1–RF4, RF6 and RF8–RF10.
  - Reopened: RF5 (Node README examples call a nonexistent `NxProgramArtifact.build`) and RF7 (the new requirement
    is filed under `## REMOVED Requirements` in both runtime deltas).
  - New: RF11, a low-severity polish item.
  - Fix RF7 before archiving. RF5 and RF11 are documentation and polish.
- Second verification (2026-11-01):
  - Verified: RF5, RF7 and RF11. All findings RF1–RF11 are now verified, and none are open.
  - Checked spelling-only diffs in `sites/playground/src/examples/nx/{graphics,gradients}.out.nx`. Both files are
    identical to `HEAD` once a trailing `.0` is dropped from whole numbers. Every changed line only adds `.0` at
    float sites (e.g. `opacity=1.0`, `x=90.0`), which follows from the verified constrained-type formatter fix.
  - The `packages/language-http` oversized-upload `ECONNRESET` failure is not caused by this change: nothing under
    that package changed since `be7f870`, and its source uses none of the removed SDK APIs.
  - Remaining non-review work: the final aggregate checks and the planned `--skip-specs` archive.
