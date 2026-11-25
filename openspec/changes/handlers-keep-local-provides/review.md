# Review: handlers-keep-local-provides

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/component-action-handlers,
specs/component-instance-tree, specs/environment-values, specs/nx-ir-format, specs/rust-ir-runtime  
**Reviewed code:** `git diff origin/main...HEAD` (2f1f7c0b..3588446a): crates/nx-codegen (formulas.rs,
ir.rs, model.rs, builder.rs), crates/nx-ir (model.rs, image.rs, explain.rs, lib.rs),
crates/nx-ir-runtime (environment.rs, eval.rs, component.rs, value.rs, stored.rs, module.rs),
runtime/typescript/src/index.ts and its dist, crates/nx-api tests (ir_runtime_tests.rs,
ir_instance_tree_tests.rs, ir_instance_tree_stress_tests.rs), runtime/typescript/test/runtime.test.ts,
specs/ir-conformance/environment-handlers, docs/nx-ir-format.md, the website environment page,
specs/future.md, specs/ir-conformance/README.md  

Probes run during the review (temporary tests, removed): bundle captures (`provide env with dense`,
`dense with { space=1 }`, an inline bundle) give `6 2`, `3 1` and `7` in the Rust runtime, as the
environment page says; a handler reading a record field default that reads `env.mode` is flagged and
sees the provide around it. `runtime/typescript` builds with no change to `dist`.

## Findings

### ✅ Verified - RF1 Bundle overrides captured by a handler are not tested in either runtime, and their cost parity is not pinned
- **Severity:** Medium
- **Evidence:** `with_captured_overrides` (crates/nx-ir-runtime/src/environment.rs) and
  `withCapturedOverrides` (runtime/typescript/src/index.ts) each have a separate path for a `{bundle}`
  entry (one layer per entry, with the bundle flag, through `entries_of` / `bundleEntriesOf`). No
  test in either runtime dispatches a handler made inside `provide env with …`, and the
  `environment-handlers` corpus program has no bundle, so neither the result nor the equal operation
  count of that path is checked across runtimes. Design Decision 2 and the
  component-action-handlers requirement name bundles explicitly.
- **Recommendation:** Add a handler inside `provide env with <bundle> with { … }` (with a formula
  entry in the bundle) to the `environment-handlers` corpus program, so both runtimes pin its result
  and count.
- **Fix:** Added a `Bundle` button to the `environment-handlers` `Page`, inside `provide env with dense with { mode="bundle" }` where `dense` sets `space=2` and the formula `gap={env.space * 3}`, and a ninth lifecycle batch that taps it (`bundle 6 2`). Both runtimes pass the corpus with the regenerated counts; the README describes the case.
- **Verification:** The corpus `Page` lifecycle's ninth batch renders `bundle 6 2`, the result the environment page gives for `dense with { … }`: `mode` from the step, `gap` from the bundle's formula reading the bundle's `space`. `node test/corpus.test.mjs` and `ir_corpus` pass with equal counts, so the bundle replay path (`entries_of` / `bundleEntriesOf`, bundle flag) is pinned in both runtimes.
- **Independent verification:** Re-checked at d0c2444f: the `Bundle` button sits inside `provide env with dense with { mode="bundle" }`, the ninth `Page` batch renders `bundle 6 2`, and `node test/corpus.test.mjs` and `codegen_parity` (which runs the corpus) pass, so both runtimes give that result at 149 operations. A temporary probe also ran `provide env.space=7 { provide env with dense { … } }` in both runtimes: `62`, 69 operations each, so the bundle's formula reads the bundle's own `space`, not the outer one.

### ✅ Verified - RF2 Equality of flagged handlers, and its cost, is untested in both runtimes
- **Severity:** Medium
- **Evidence:** The nx-ir-format scenario "Two flagged handlers compare their overrides after their
  slots" and the component-action-handlers scenario "Handlers under different overrides are unequal …
  AND they SHALL compare equal when `label` is the same" have no test. The only coverage is the Rust
  tree test `a_provide_whose_value_changes_makes_new_handlers_and_keeps_children_whose_handlers_read_nothing`,
  which shows unequal handlers indirectly. `handlersEqual` in TypeScript is not exercised for the
  overrides at all, and no test compares the operation counts of the two runtimes for it (tasks 2.7
  and 4.2 claim "equality across renders" and "costs").
- **Recommendation:** Add corpus entrypoints that compare two handlers of one flagged node made
  under equal and under different overrides with `==`, as `evaluation-cost` does for slots, so both
  results and counts are pinned in both runtimes.
- **Fix:** Added corpus entrypoints `equalHandlers` and `unequalHandlers` (two buttons from `pinger`, whose handler reads `env.mode` inside `provide env.mode={label}`) and the control `quietHandlers`, whose handler reads nothing. Results are `true` and `false`, and the counts are 30, 30 and 25: the 5 more are the two lists, their pair of entries and that pair's three fields, as the nx-ir-format scenario says. Both runtimes pass with those counts.
- **Verification:** Results `true` and `false`, counts 30, 30 and 25 in both runtimes. The 5-operation gap over `quietHandlers` is the override comparison of the nx-ir-format scenario, so a runtime that skipped or mis-costed it would fail. The `false` of `unequalHandlers` also follows from the descriptors' `$env`, which always mirrors the handler's capture; the counts, not that result, are what pin the handler comparison. The tree test still covers unequal handlers across renders.
- **Independent verification:** Re-checked: `equalHandlers`, `unequalHandlers` and `quietHandlers` give `true`, `false`, `true` at 30, 30 and 25 operations in both runtimes. Under a budget `records_equal` compares every field without stopping, so the unequal case also walks the handler's override lists, and the 5-operation gap pins that comparison and its cost in both runtimes. As noted before, the `false` alone would also come from the descriptors' `$env`; NX cannot take a handler out of a descriptor (`.onTapped` is `unknown-member`), so a direct result test is not possible from source, and the Rust tree test covers the result.

### ✅ Verified - RF3 A new test uses `.err().expect(...)`, which fails CI's clippy (`clippy::err_expect`)
- **Severity:** Low
- **Evidence:** crates/nx-api/tests/codegen/ir_runtime_tests.rs, `a_flagged_handler_without_environment_v3_is_refused`:
  `PreparedModule::prepare(...).err().expect("refused")`. CI's `cargo clippy --workspace --all-targets
  --locked -- -D warnings` fails on 3588446a.
- **Recommendation:** Use `.expect_err("refused")`.
- **Fix:** Changed to `.expect_err("refused")`. `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` pass.
- **Verification:** Line 5023 now uses `expect_err`; `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` pass.
- **Independent verification:** Re-checked: line 5023 uses `.expect_err("refused")`; `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` pass at d0c2444f.

### ✅ Verified - RF4 The stress test's module doc comment now has a line far past the line width
- **Severity:** Low
- **Evidence:** crates/nx-api/tests/codegen/ir_instance_tree_stress_tests.rs, the `//!` paragraph
  that lists the risky features: the inserted "handlers that read the environment inside a provide
  in a content child" makes one line about 160 characters; the rest of the paragraph wraps at 100.
- **Recommendation:** Reflow the paragraph.
- **Fix:** Reflowed the paragraph to the width of the rest of the comment.
- **Verification:** No `//!` line in the file is past 100 characters.
- **Independent verification:** Re-checked: no `//!` line of ir_instance_tree_stress_tests.rs is over 100 characters.

### ✅ Verified - RF5 The website's new "In an action handler" section runs into the next heading
- **Severity:** Low
- **Evidence:** sites/website/src/content/docs/reference/syntax/environment.md: the section's last
  paragraph is followed directly by `## Root values from the host` with no blank line, unlike every
  other heading on the page.
- **Recommendation:** Add the blank line.
- **Fix:** Added the blank line before `## Root values from the host`.
- **Verification:** A blank line now comes before the heading.
- **Independent verification:** Re-checked: a blank line precedes `## Root values from the host`.

### ✅ Verified - RF6 The NX example in docs/nx-ir-format.md exceeds the NX line-length target
- **Severity:** Low
- **Evidence:** docs/nx-ir-format.md, *Handlers and the environment*: the `LocaleList` body is one
  128-character line; AGENTS.md sets a 120-character target for NX source.
- **Recommendation:** Split the `for` over lines, as the website's copy of the example does.
- **Fix:** Split the `for` over lines, as the website's copy has it.
- **Verification:** The example's lines are all within 120 characters.
- **Independent verification:** Re-checked: no line in the NX blocks of docs/nx-ir-format.md is over 120 characters.

### ✅ Verified - RF7 Rebinding a handler held inside a captured override has no test
- **Severity:** Low
- **Evidence:** The rust-ir-runtime requirement says rebinding "SHALL replace a handler held inside a
  captured override value as it replaces one held in a captured slot", and `Rebinding::handler`
  (crates/nx-ir-runtime/src/component.rs) gained that walk, but `rebinding_tests` covers only
  handlers with no overrides.
- **Recommendation:** Add a unit test in `rebinding_tests`: a handler whose captured overrides hold
  an old handler is replaced by one holding the new handler, and its slots are kept.
- **Fix:** Added `a_handler_held_in_captured_overrides_is_replaced` to `rebinding_tests` (crates/nx-ir-runtime/src/component.rs): a handler whose captured overrides hold an old handler is replaced by one that holds the new handler, and its captured slot keeps its handler.
- **Verification:** The test passes, and it fails to compile without `Handler::environment` and the walk, so it exercises the new code; the replaced handler holds the new handler and keeps its slot's handler by pointer.
- **Independent verification:** Re-checked: `a_handler_held_in_captured_overrides_is_replaced` passes. With the override walk in `Rebinding::handler` removed, the holder's slot holds no replaced handler, so the method returns `None` and the test's `expect` fails: the test exercises the walk, not only the field.

## Verification Notes
- Verified at 2789be7e. The session had no tool for spawning subagents, so one agent ran the review,
  the fixes and this verification; none of it wrote the original change. No new findings.
- Independently re-verified at d0c2444f by a separate agent that did not write the change, the review
  or the fixes. All seven fixes hold. Temporary probes (removed) checked, in both runtimes with equal
  results and operation counts: a captured formula reading a captured outer override
  (`provide env.space=5 { provide env.gap={env.space * 2} { … } }` gives `10`, 66 operations), a
  formula `env.` attribute on the element holding the handler (`3`, 68), and bundles (`62`, 69). In
  Rust, a stored instance restored and dispatched gives the same results for the nested and bundle
  cases, and in the tree a flagged handler under an unchanged provide compares equal across a parent
  render, so its child keeps its render. RF8 is new; it was not fixed.

## New Findings Discovered During 2026-11-25 08:11 Verification

### 🟡 Fixed - RF8 Every `envProvide` copies the whole list of the overrides around it, even when no handler captures it
- **Severity:** Low
- **Evidence:** `eval_provide` (crates/nx-ir-runtime/src/environment.rs) builds
  `outer.iter().chain(entries.iter()).cloned().collect()` and `evalEnvironmentProvide`
  (runtime/typescript/src/index.ts) builds `[...outer, ...entries]` each time a provide is evaluated,
  in every program, whether or not a flagged handler is beneath it. The copy is not charged. A
  provide inside a recursive function or component body copies one entry more per level, so a
  recursion of depth d copies O(d²) entries; a provide in a loop inside k other provides copies k+1
  entries per iteration. Design Decision 2 says "taking it copies no entries", which holds for the
  capture but not for building the list. Results and counts are not affected.
- **Recommendation:** Keep only each provide's own entries on the stack (or a persistent linked list
  with a cached flattened form) and build the flattened list when a flagged handler first captures
  at that depth, in both runtimes; or state the copy in Decision 2 and the cost section.
- **Fix:** Both runtimes now push one level per provide holding only its own entries
  (`Provided` in `environment.rs`, `ProvidedLevel` in `index.ts`), so pushing copies nothing. The
  first flagged handler captured at a level joins the entries from the nearest level below that
  already holds its list (or a running handler's whole list) up to the top, and caches it on that
  level for the handlers after it. A program without a flagged handler never builds a list.
  Results, op counts and the corpus are unchanged. Decision 2, the cost section and
  `docs/nx-ir-format.md` say how the list is built.

## Questions
- None

## Summary
- The implementation matches the design. Capture, replay, equality, the stored form and the
  feature gate agree between the two runtimes in what the tests exercise, and the corpus pins the
  five places from the proposal. The gaps are tests of paths that have their own code (bundles,
  equality and its cost, rebinding through overrides), plus a clippy failure and small doc and
  format slips.
- Independent verification confirms all seven fixes; one new low-severity finding (RF8, an
  uncharged copy of the override list at every provide) remains open.
