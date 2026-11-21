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

### 🟡 Fixed - RF1 The comparison and the rebinding walk captured values without a meter, so their cost scales with handlers × capture size outside the operation budget
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

### 🟡 Fixed - RF2 Many-to-one pairings are accepted, contrary to the spec's "paired one-to-one" rule
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

### 🟡 Fixed - RF3 A kept child gets a new emit handler that nothing checks against the emit, which the lifecycle would check
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

### 🟡 Fixed - RF4 Plain values are compared with `NxValue ==`, which counts `-0.0` and `0.0` as equal, so the skip now spans parent renders where the body could tell them apart
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

### 🟡 Fixed - RF5 Tests miss several of the risky paths
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

### 🟡 Fixed - RF6 New doc lines exceed the line-length target
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - Module doc in crates/nx-ir-runtime/src/tree.rs:9 is 171 columns: "...handlers when it hands
    the same ones; and dropped when ... A handler found in one instance's output may have been
    bound by an enclosing".
  - crates/nx-ir-runtime/README.md:312 is far past the wrap width used by the surrounding text.
- **Recommendation:** Rewrap both to match the neighboring lines.
- **Fix:** Rewrapped the module doc in tree.rs and the README bullet.

## Questions
- Should the rebinding and comparison work count against `maxOperations`, given that hosts use the
  budget to bound each call? (RF1)
- Task 4.3 is still open. That is expected at this stage, since it includes the review loop.

## Summary
- I found no correctness bug in the skip or the rebinding for the scenarios in the spec, including
  emit routing to the parent's current instance, content handlers that run against their owner,
  tokens that stay valid, rebinding through descendants, RF1 nesting, and all-or-nothing behavior
  inside `atomically`.
- The main concern is RF1: unmetered work that scales with handlers × capture size. The rest are
  small issues: a spec/code mismatch on one-to-one pairing, a missing emit-type check on the
  presence-only path, float signed-zero equality, test gaps, and wrapping.
