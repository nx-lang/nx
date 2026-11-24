## 1. Rust IR runtime

- [ ] 1.1 Give each environment layer a per-call sequence number (`EnvState`), the host's layers
  and caps included, and a helper that returns a value's token over the layers below a given
  position.
- [ ] 1.2 Add a per-call derived-result cache to `Machine`: entries keyed by derived result (value
  declaration, environment default, targeted entry-module provide, bundle entry-module provide with
  the value read, formula layer), each holding the value and its recorded reads with their tokens
  and the values found for the instance tree's read list.
- [ ] 1.3 Record reads during a derived evaluation, nested evaluations and reuses included, against
  every derived result being evaluated around them, with tokens taken over the layers below where
  each evaluation began (before its own cap) and computed again, not copied, when a reused result's
  reads are added to an enclosing record.
- [ ] 1.4 Reuse in `eval_reference` (value declarations), `provided_or_default` (defaults and
  entry-module provides, the bundle arm keyed by provide and value read) and `source_value`
  (formulas) when every recorded token matches; charge nothing beyond the asking node on reuse.
- [ ] 1.5 Replay a reused result's reads into the instance tree's read list, keeping the
  "found under the call's own overrides" flag, so the skip check sees the same reads.
- [ ] 1.6 Unit tests for each scenario in the `reactivity` and `nx-ir-format` deltas, the tree's
  reader-through-a-reused-default case and the bundle that reads a value it sets included, and a
  test that a reused shallow default lets a deep read succeed alike in both runtimes.

## 2. TypeScript IR runtime

- [ ] 2.1 The same layer identities, cache, read recording and reuse in
  `runtime/typescript/src/index.ts`, following the Rust rule exactly. Its dispatch is unchanged.
- [ ] 2.2 Tests for the same scenarios, and rebuild `runtime/typescript/dist`.

## 3. Generated code

- [ ] 3.1 Layer identities and a per-call cache in the generated environment state
  (`crates/nx-codegen/src/runtime/environment.js`), used by `nxEnvRead`, `nxProvidedOrDefault`,
  formulas and the thunks of top-level values that read the environment, a bundle provide keyed by
  the value read.
- [ ] 3.2 A test-only evaluation counter in the generated helpers, off unless the test harness turns
  it on, counting each evaluation of a default, provide, formula or top-level value thunk.
- [ ] 3.3 Tests for the `executable-code-generation` scenarios, reading the counter to show that a
  default is evaluated once.

## 4. Component lifecycle (Rust, for the tree)

- [ ] 4.1 Record on `ComponentInstance`, in memory only, the state fields and props a render read:
  slots the body evaluated, slots `capture_formula` captured, and the prop slots of each handler
  node the body binds (`slots_read`). The stored form is unchanged.
- [ ] 4.2 Split `ComponentInstance` so that its render part (handlers, generation, tokens, props
  and records) is shared by an instance whose dispatch kept its render, and make
  `ComponentInstance::is` compare that part.
- [ ] 4.3 An internal dispatch entry for the tree: one call that applies the handlers as
  `dispatch_component_actions` does, compares each patched field the render read (charged as `diff`
  charges a field), and renders only when one differs or the tree says the node's other records do
  not hold; otherwise it returns the new state and effects with no output written.
- [ ] 4.4 An internal entry that normalizes a descriptor's props without rendering, charged as
  initialization charges normalizing them.
- [ ] 4.5 Leave `dispatch_component_actions` and the TypeScript dispatch rendering on every batch;
  check that the corpus's lifecycle results and the `rust-ir-runtime` and `typescript-ir-runtime`
  tests are unchanged.

## 5. Instance tree (Rust)

- [ ] 5.1 Node records of direct inputs in read order (root reads with revisions and edges, kept
  results used, override-supplied reads with values, reads under the render's own overrides as
  today) and a per-node `verified` revision; replace the leaf-read check of
  `environment_reads_hold` in `visit`, `set_environment` and dispatch with the record check.
- [ ] 5.2 Prop-read skip in `visit`: normalize the new props, compare the recorded ones with
  `Across` semantics under `Program::metered`, rebind through `rebinding`/`rebind` when the parent's
  instance or a handler changed, and give the instance the new props without rendering.
- [ ] 5.3 Dispatch skip in `dispatch_chain` through the entry of 4.3: keep the rendered output,
  tokens and generation, take the new state, leave the node settled and its children's `through`
  matching.
- [ ] 5.4 Kept derived results: which results are kept (none of their reads under a layer the
  render pushed; no formulas or in-render bundles), the internal runtime option that passes them to
  a render's `Machine`, entry into the call's cache with tokens over the host layers, a revision
  counter, `changed` and `verified` per variant, and variants told apart by read classes and
  override-supplied values, tried in the order kept.
- [ ] 5.5 Validation: classify each read at the use, check inputs in read order and stop at the
  first that changed, then evaluate again; look up override-supplied values as the skip check does,
  formulas included; equality cutoff by written identity (number kind, `-0.0`, NaN, origins when
  collected), charged as a comparison.
- [ ] 5.6 Reference counts or reverse edges for variants, dropping a variant no record references
  when the tree operation ends; the extended `readers` index, override-supplied reads indexed under
  the root values their lookup read; `set_environment` marks possibly stale through the indexes,
  then pulls in depth order.
- [ ] 5.7 Add the kept results, revision counter, reverse indexes and change set to `atomically`'s
  snapshot.
- [ ] 5.8 The change set and `take_changes` as a diff by key (replacement as removed and added,
  every dropped node listed, rebinding as rendered, a new descriptor or state alone not listed),
  consistent with `is_settled`.
- [ ] 5.9 Tests for every `component-instance-tree` and `reactivity` tree scenario, including the
  modified scenarios of the existing requirements, a `set_environment` whose render fails after a
  kept result was evaluated again, and a dispatch skip followed by a tap on the old output.

## 6. Corpus and cost model

- [ ] 6.1 Add the `environment-reuse` corpus program with entrypoints for each `nx-ir-format`
  scenario, the bundle that reads a value it sets read for two values included, and run it in
  generated JavaScript too.
- [ ] 6.2 Regenerate every program's `operations.json`, check that `results.json` is unchanged and
  that all three engines agree.
- [ ] 6.3 Record `design-tokens-theme`'s counts before and after in the PR description.
- [ ] 6.4 Run the cost differential and allocation tests.

## 7. Documentation

- [ ] 7.1 `docs/nx-ir-format.md`: the cost requirement's first sentence (the tree's held state),
  the cost table's node row, the environment section's lookup and cost paragraphs, and a section on
  reusing derived results, with where an evaluation begins, the bundle key and the note on limits.
- [ ] 7.2 `specs/ir-conformance/README.md`: describe `environment-reuse`.
- [ ] 7.3 `specs/future.md`: remove "Memoizing defaults", "A bundle's values are computed at every
  read" and most of "Reading a token is expensive". Rewrite the reactivity direction and the
  instance-tree sections in terms of the tracked-computation model, marking state reads in the tree,
  prop reads, kept results and the change set done, keeping the premise that a handler captures the
  whole frame, and keep reuse finer than a body, a dispatch skip in the bare APIs and a self-walking
  tree as later work.
- [ ] 7.4 Document the change set, the new skips, kept results and their cost in the instance
  tree's module docs and `docs/` where the tree is described, with an example of a host that
  updates from the change set.
