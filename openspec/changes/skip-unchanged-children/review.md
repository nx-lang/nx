# Review: skip-unchanged-children

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/component-instance-tree/spec.md  
**Reviewed code:** crates/nx-ir-runtime/src/tree.rs (`visit`, `rebind`, `Across`, `bound_properties`,
`token`), crates/nx-ir-runtime/src/component.rs (`Rebinding`, `ComponentInstance::rebound`),
crates/nx-api/tests/codegen/ir_instance_tree_tests.rs, crates/nx-ir-runtime/README.md, specs/future.md  

**Checks run:** `cargo test -p nx-api --test codegen_parity ir_instance_tree` (47 passed),
`openspec validate skip-unchanged-children --strict` (valid). I ran throwaway probes against a copy
of the repo in the scratchpad. The working tree was not touched.

**What holds up:** I traced the skip through the cases below and found no defect in them.

- **Soundness.** A body can observe a handler only through `==` and its `ActionHandler` record, and
  a bound emit handler never reaches any body's scope. Handlers that are only checked for presence
  therefore cannot change output.
- **Rebinding coverage.** It reaches props, `handler_props`, state, `handlers` and captures of
  rebuilt handlers. It is memoized per handler allocation, so allocations stay distinct and RF1
  routing by identity is preserved.
- **Mutation order.** `rebind` builds every new instance before it mutates anything, so a failing
  visit changes nothing, and `atomically` restores `nodes` and `visited`.
- **`visited` updates.** Entries are moved to the rebound instance only where they named the
  pre-rebind one, which keeps `finish`/`walked_anew` consistent.
- **`through` updates.** `through` is repointed for every rebound node, so `read_from_current`
  and `creator`'s staleness check hold afterwards.
- **Descendants through an older instance.** Descendants initialized through an older (pre-dispatch)
  instance are correctly left for their own visit. That visit resolves tokens through the old and
  new instances and pairs them correctly.
- **Settledness.** It is preserved as designed.

## Findings

### ✅ Verified - RF1 The comparison and the rebinding walk captured values without a meter, so their cost scales with handlers × capture size outside the operation budget
- **Severity:** Medium
- **Confidence:** Medium. The mechanism is confirmed by reading the code and by a probe. I did not
  build a worst case that does damage.
- **Evidence:**
  - `Across::handlers` calls `handlers_equal(old, new, &Meter::free(), ...)` (tree.rs, `impl
    Across`).
  - `Rebinding::value`/`fields` walk every `Seq`/`Record` in every capture of every handler a kept
    node or a rebound descendant holds (component.rs, `Rebinding::handler`). Only `Handler`
    allocations are memoized. Shared `Seq`/`Record` allocations are walked again for each handler
    that captures them.
  - A body that makes N handlers in a `for`, each capturing a frame slot that holds a large value
    (a prop or a loop item of size S), costs about N + S operations to render. The skip then does
    about N × S unmetered work per visit: in `values_equal` for content handlers, and in
    `Rebinding` for the node's own handlers. That holds even when the only change is a bound emit
    handler, which is the common case the change targets.
  - The language's `==` charges this work, but the tree does not. The cost is not exponential,
    though: a probe with a 2^20-node shared record DAG in a capture showed the lifecycle charges
    the DAG's tree size when it builds it (a 2M budget refused the render), and the rebinding visit
    then took about 1 s against about 70 s for the render.
  - This bypasses `maxOperations`, which hosts set to bound work per call.
- **Recommendation:** Either charge the walks to the visit's `RuntimeOptions` budget (pass a real
  `Meter`), or memoize `Rebinding` by `Seq`/`Record` allocation as well as by handler. Ideally,
  also skip capture slots whose static type cannot hold a handler. Add a test with many handlers
  that capture one large prop under a tight `max_operations`. If the design keeps "costs no
  operations", state the bound in the spec and README.
- **Fix:** The comparison and the rebinding now run as one `Program::metered` call under the visit's options: `handlers_equal` gets the call's charged meter, and `Rebinding` charges one operation per handler, list and record it walks, and memoizes lists and records by allocation as well as handlers, so a shared capture is walked once. Spec, design and README say so; test `rebinding_a_kept_child_is_charged_to_the_operation_budget` checks that a visit under a budget of one is refused and changes nothing.
- **Verification:** Verified. The comparison and the rebinding now run inside one `Program::metered` call, and `handlers_equal` gets that call's charged meter. `Rebinding` charges for each handler, list and record it walks and memoizes lists and records by allocation. Lists are keyed by `Arc<[Value]>::as_ptr`, empty lists are skipped, and each memoized value is held so its address cannot be reused. The memo is order-independent because each result depends only on handler replacements, which are memoized deterministically. `rebinding_a_kept_child_is_charged_to_the_operation_budget` passes. Two side effects of the metered call are new findings: RF7 (usage/origins reports) and RF8 (the comparison is not memoized).

### ✅ Verified - RF2 Many-to-one pairings are accepted, contrary to the spec's "paired one-to-one" rule
- **Severity:** Low
- **Confidence:** High (by reading the code)
- **Evidence:**
  - The spec says: "When the old handlers are not paired one-to-one with new ones, the descriptors
    SHALL be treated as different."
  - `Rebinding::pair` (component.rs) rejects only one old handler mapped to two new ones. Two old
    handlers that map to the same new handler are accepted. `(None, Some)`/`(Some, None)` are also
    handled only from the old side.
  - The behavior looks harmless, because the rebound `handlers` list holds the new handler at both
    positions, which is what a re-initialization would produce, since `Tokens` numbers each
    occurrence. Still, the spec and the code disagree, and the design describes only the
    one-to-many direction.
- **Recommendation:** Change the spec to match the design: an old handler that would be replaced by
  two different new handlers makes them different. Alternatively, add a reverse check in `pair`.
  Add a test for the fallback, for example a parent that hands one handler to two props in one
  render and two allocations in the next.
- **Fix:** Kept many-to-one (harmless, matches what a new render writes) and changed the spec and design to say exactly that: one old handler replaced by two different new ones makes the descriptors different. `Rebinding::pair` documents it; unit tests `rebinding_tests` cover one-to-many refused, many-to-one allowed, and kept/replaced mixing refused.
- **Verification:** Verified. The spec and design now state the actual rule (one-to-many is refused, many-to-one is allowed). The doc comment on `Rebinding::pair` agrees, and three `rebinding_tests` unit tests cover the cases. The doc comment on `Across::handlers` still says "paired one to one" (see RF9).

### ✅ Verified - RF3 A kept child gets a new emit handler that nothing checks against the emit, which the lifecycle would check
- **Severity:** Low
- **Confidence:** Low. The type checker may already rule this out for every program that compiles.
- **Evidence:**
  - For a property in `bound`, `Across::descriptors` calls `handlers(old, new, false, ...)`, which
    checks only that both tokens resolve. The new handler is then installed into `handler_props`
    by the rebinding.
  - `split_handler_properties` (component.rs) is skipped. On initialization it refuses a handler
    whose `component`/`emit` do not match the emit.
  - If a parent can bind different handler values at one place across renders (a conditional
    `onLogged={...}`), the tree could hold a binding the lifecycle would refuse. That would break
    the "only through the lifecycle" guarantee in the modified first requirement.
- **Recommendation:** In the presence-only branch, also require `old.component == new.component &&
  old.emit == new.emit`, and `action_name` if relevant. This is cheap and restores the lifecycle's
  check.
- **Fix:** The presence-only branch of `Across::handlers` now also requires the same `component`, `emit` and `action_name`, the check `split_handler_properties` makes. Spec and design say so.
- **Verification:** Verified. The presence-only branch of `Across::handlers` now requires the same `component`, `emit` and `action_name`, the identity that `split_handler_properties` checks.

### ✅ Verified - RF4 Plain values are compared with `NxValue ==`, which counts `-0.0` and `0.0` as equal, so the skip now spans parent renders where the body could tell them apart
- **Severity:** Low
- **Confidence:** Low-Medium. It depends on whether NX output can expose the sign of zero, for
  example through `1.0 / x` or formatting.
- **Evidence:**
  - `Across::values` falls back to `old == new` on `NxValue`, which uses the derived float
    equality.
  - Before this change, `same()` was used only for a descriptor read from the same parent instance,
    where the values were identical. Now a parent that renders `0.0` and then `-0.0` keeps the
    child.
  - NaN is handled conservatively (unequal, so the child is re-initialized).
- **Recommendation:** Compare floats by bits (`to_bits`), or document that the language treats them
  as equal.
- **Fix:** `Across::values` compares floats by `to_bits`, so `-0.0` and `0.0` (and NaNs with different bits) count as different and the child is initialized again.
- **Verification:** Verified. `NxValue::Float` is compared by `to_bits`. Descriptors come from runtime output, which writes only `Float`; host `Float32` is widened on the way in, so leaving `Float32` uncovered is fine. A NaN with identical bits is now kept, which is sound because the body is deterministic for identical bits.

### ✅ Verified - RF5 Tests miss several of the risky paths
- **Severity:** Low
- **Confidence:** High
- **Evidence:** No test covers these paths:
  - the one-to-many fallback (RF2);
  - a bound emit handler present in one descriptor and absent in the other;
  - a kept child that was dispatched against before its parent rendered, whose children were
    initialized through its pre-dispatch instance. This exercises `rebind`'s `continue` branch and
    the later `Across` visit of those children.
  - a descendant already visited in the pass when its ancestor is rebound (the `visited` rewrite in
    `rebind`).
- **Recommendation:** Add a focused test for each path.
- **Fix:** Added `an_emit_handler_bound_in_one_render_and_not_the_next_is_a_changed_descriptor`, `a_kept_child_dispatched_against_before_its_parent_rendered_is_rebound_with_what_is_under_it`, `a_node_visited_before_its_ancestor_is_rebound_counts_as_visited`, and unit tests for the pairing fallback (an NX program that pairs one handler to two in one render and two in the next is not practical to write, so `Rebinding::pair` is tested directly).
- **Verification:** Verified. The four new tests and the unit tests cover the paths listed. The dispatched-then-kept test exercises the `continue` branch of `rebinding` and the later `Across` visit of the Card through the Panel's pre-dispatch instance. `cargo test -p nx-api --test codegen_parity ir_instance_tree`: 51 passed. `cargo test -p nx-ir-runtime rebinding`: 3 passed.

### ✅ Verified - RF6 New doc lines exceed the line-length target
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - Module doc in crates/nx-ir-runtime/src/tree.rs:9 is 171 columns: "...handlers when it hands
    the same ones; and dropped when ... A handler found in one instance's output may have been
    bound by an enclosing".
  - crates/nx-ir-runtime/README.md:312 is far past the wrap width used by the surrounding text.
- **Recommendation:** Rewrap both to match the neighboring lines.
- **Fix:** Rewrapped the module doc in tree.rs and the README bullet.
- **Verification:** Verified for the cited lines: tree.rs:9 and README.md:312 are rewrapped. The fix commit added new long lines, which are tracked separately as RF9.

## New Findings Discovered During 2026-11-21 20:28 Verification

### ✅ Verified - RF7 The new `Program::metered` call resets the caller's usage and origins reports, so a visit's reports now depend on which path it took
- **Severity:** Low
- **Confidence:** High for the mechanism, Medium for how much it matters to hosts
- **Evidence:** `metered` goes through `Program::call` (component.rs), which clears
  `options.usage` and `options.origins` and then records its own result. The effect depends on
  the path a visit takes:
  - **Kept:** the usage report holds the cost of the comparison and rebinding, and the origins
    report is cleared and left empty, although the node's output (and the origins it has) is
    unchanged. Before this commit, a visit that did not render left both reports as they were.
    The same-descriptor fast path still makes no call, so it leaves them untouched. A host that
    reads `origins` after each visit therefore gets nothing for a kept node and must instead rely
    on `Arc::ptr_eq` of `rendered`.
  - **Fallback to initialization:** the comparison runs as its own call with its own
    `max_operations`, and then `initialize_component` clears `usage` and reports only its own
    cost. So one visit can spend close to twice the budget, and the usage report understates
    what it used.
  - **Documentation:** none of this is in the README's tree section or the spec. The spec says
    "the operation budget and the input limit SHALL apply to each of those calls separately" only
    for lifecycle calls.
- **Recommendation:** Document what a visit leaves in `usage` and `origins` on each path, for
  example "a kept node's origins are those of its earlier render". Better still:
  - Keep `origins` as it was in the metered call, for example with a `call` variant that does not
    touch origins.
  - Either charge the comparison and the following initialization to one budget and one usage
    record, or state that they are separate.
  - Add a test with a `Usage` and an `Origins` across a kept visit and a fallback visit.
- **Fix:** `Program::metered` no longer goes through `Program::call`: it builds a `Machine` on the options and meters the work, leaving the usage and origins reports as the last lifecycle call wrote them. The doc comment, README, design and spec say the reports are left alone and that a visit that compares and then initializes spends a budget on each. Test `keeping_a_child_leaves_the_usage_report_as_the_last_call_wrote_it` checks the usage report is unchanged by a kept visit.
- **Verification:** Verified. `metered` now builds a `Machine` directly instead of going through `Program::call`, so it neither clears nor writes `usage` or `origins`. The `Machine` collects origins internally, but only `call` copies them into the report. The budget and stack limits still apply, and the resource-limit error still names the component. README, design and spec now say that the reports are left alone and that a fallback visit spends one budget on the comparison and one on the initialization. `keeping_a_child_leaves_the_usage_report_as_the_last_call_wrote_it` passes. Origins have no test, but leaving them alone follows from the same code path.

### ✅ Verified - RF8 The spec and README say a value many handlers capture is walked once, but the comparison walks it once for every handler pair
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - The spec says "The comparison and the rebinding SHALL count against the visit's operation
    budget ... and SHALL walk a value many handlers capture once". The README and design say the
    same.
  - Only `Rebinding` memoizes. `Across::handlers` calls `handlers_equal` for each pair of visible
    handlers, and `values_equal` walks every capture in full each time. So N content handlers that
    each capture one value of size S still cost about N × S. That work is now metered, so the
    budget bounds it and it is no longer a safety problem, but the spec is wrong about it.
  - The README also says "one operation for each value they walk". `Rebinding` charges only
    handlers, lists and records, while `values_equal` also charges scalars and text length.
- **Recommendation:** Narrow the wording to "the rebinding walks a shared value once; the
  comparison costs what `==` costs", or memoize pairs in `Across` by `(old, new)` allocation.
- **Fix:** Reworded spec, design and README: comparing handlers costs what `==` costs; only the rebinding walks a shared value once, at one operation per handler, list and record.
- **Verification:** Verified. Spec, design and README now say that the comparison costs what `==` costs, that the rebinding costs one operation per handler, list and record, and that only the rebinding walks a shared value once.

### ✅ Verified - RF9 The fix commit adds doc lines past the wrap width and leaves a stale "one to one" comment
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - crates/nx-ir-runtime/src/tree.rs:951, the doc of `Across::handlers`, is 138 columns. The same
    comment still says "and paired one to one", which contradicts the many-to-one rule RF2 settled
    on.
  - The README bullet line "operation for each value they walk, and a value many handlers capture
    is walked once. A handler inside" is 104 columns, against about 100 for its neighbors.
- **Recommendation:** Rewrap both. Change the comment to "and paired so that no earlier handler
  has two replacements".
- **Fix:** Rewrapped the `Across::handlers` doc (also updated it from "paired one to one") and the README bullet.
- **Verification:** Verified. The `Across::handlers` doc is rewrapped and now states the rule as "never one earlier handler replaced by two new ones". The README bullet is rewrapped. No changed doc line in tree.rs, component.rs or that README section is longer than 100 columns.

## Questions
- ~~Should the rebinding and comparison work count against `maxOperations`, given that hosts use
  the budget to bound each call? (RF1)~~ Answered by the fix: yes. RF7 asks whether that charge
  should share one budget and one usage report with a fallback initialization.
- Task 4.3 is still open. That is expected at this stage, since it includes the review loop.

## Summary
- Second verification (2026-11-21 20:31): RF7–RF9 are verified and no new findings were
  opened. All nine findings are closed.
- Verification (2026-11-21 20:28): RF1–RF6 are verified. RF7–RF9 are new findings, all Low. They
  concern side effects of the metered call on the reports, spec wording about memoization, and
  wrapping. No correctness regression was found in the fix commit.
- I found no correctness bug in the skip or the rebinding for the scenarios in the spec, including
  emit routing to the parent's current instance, content handlers that run against their owner,
  tokens that stay valid, rebinding through descendants, RF1 nesting, and all-or-nothing behavior
  inside `atomically`.
- The main concern is RF1: unmetered work that scales with handlers × capture size. The rest are
  small issues: a spec/code mismatch on one-to-one pairing, a missing emit-type check on the
  presence-only path, float signed-zero equality, test gaps, and wrapping.
