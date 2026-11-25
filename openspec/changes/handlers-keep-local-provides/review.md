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

### 🟡 Fixed - RF1 Bundle overrides captured by a handler are not tested in either runtime, and their cost parity is not pinned
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

### 🟡 Fixed - RF2 Equality of flagged handlers, and its cost, is untested in both runtimes
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

### 🟡 Fixed - RF3 A new test uses `.err().expect(...)`, which fails CI's clippy (`clippy::err_expect`)
- **Severity:** Low
- **Evidence:** crates/nx-api/tests/codegen/ir_runtime_tests.rs, `a_flagged_handler_without_environment_v3_is_refused`:
  `PreparedModule::prepare(...).err().expect("refused")`. CI's `cargo clippy --workspace --all-targets
  --locked -- -D warnings` fails on 3588446a.
- **Recommendation:** Use `.expect_err("refused")`.
- **Fix:** Changed to `.expect_err("refused")`. `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` pass.

### 🟡 Fixed - RF4 The stress test's module doc comment now has a line far past the line width
- **Severity:** Low
- **Evidence:** crates/nx-api/tests/codegen/ir_instance_tree_stress_tests.rs, the `//!` paragraph
  that lists the risky features: the inserted "handlers that read the environment inside a provide
  in a content child" makes one line about 160 characters; the rest of the paragraph wraps at 100.
- **Recommendation:** Reflow the paragraph.
- **Fix:** Reflowed the paragraph to the width of the rest of the comment.

### 🟡 Fixed - RF5 The website's new "In an action handler" section runs into the next heading
- **Severity:** Low
- **Evidence:** sites/website/src/content/docs/reference/syntax/environment.md: the section's last
  paragraph is followed directly by `## Root values from the host` with no blank line, unlike every
  other heading on the page.
- **Recommendation:** Add the blank line.
- **Fix:** Added the blank line before `## Root values from the host`.

### 🟡 Fixed - RF6 The NX example in docs/nx-ir-format.md exceeds the NX line-length target
- **Severity:** Low
- **Evidence:** docs/nx-ir-format.md, *Handlers and the environment*: the `LocaleList` body is one
  128-character line; AGENTS.md sets a 120-character target for NX source.
- **Recommendation:** Split the `for` over lines, as the website's copy of the example does.
- **Fix:** Split the `for` over lines, as the website's copy has it.

### 🟡 Fixed - RF7 Rebinding a handler held inside a captured override has no test
- **Severity:** Low
- **Evidence:** The rust-ir-runtime requirement says rebinding "SHALL replace a handler held inside a
  captured override value as it replaces one held in a captured slot", and `Rebinding::handler`
  (crates/nx-ir-runtime/src/component.rs) gained that walk, but `rebinding_tests` covers only
  handlers with no overrides.
- **Recommendation:** Add a unit test in `rebinding_tests`: a handler whose captured overrides hold
  an old handler is replaced by one holding the new handler, and its slots are kept.
- **Fix:** Added `a_handler_held_in_captured_overrides_is_replaced` to `rebinding_tests` (crates/nx-ir-runtime/src/component.rs): a handler whose captured overrides hold an old handler is replaced by one that holds the new handler, and its captured slot keeps its handler.

## Questions
- None

## Summary
- The implementation matches the design. Capture, replay, equality, the stored form and the
  feature gate agree between the two runtimes in what the tests exercise, and the corpus pins the
  five places from the proposal. The gaps are tests of paths that have their own code (bundles,
  equality and its cost, rebinding through overrides), plus a clippy failure and small doc and
  format slips.
