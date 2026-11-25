## 1. Rust IR runtime

- [x] 1.1 Give each environment layer a per-call sequence number (`EnvState`), the host's layers
  and caps included, and a helper that returns a value's token over the layers below a given
  position.
- [x] 1.2 Add a per-call derived-result cache to `Machine`: entries keyed by derived result (value
  declaration, environment default, targeted entry-module provide, bundle entry-module provide with
  the value read, formula layer), each holding the value and its recorded reads with their tokens
  and the values found for the instance tree's read list.
- [x] 1.3 Record reads during a derived evaluation, nested evaluations and reuses included, against
  every derived result being evaluated around them, with tokens taken over the layers below where
  each evaluation began (before its own cap) and computed again, not copied, when a reused result's
  reads are added to an enclosing record.
- [x] 1.4 Reuse in `eval_reference` (value declarations), `provided_or_default` (defaults and
  entry-module provides, the bundle arm keyed by provide and value read) and `source_value`
  (formulas) when every recorded token matches; charge nothing beyond the asking node on reuse.
- [x] 1.5 Replay a reused result's reads into the instance tree's read list, keeping the
  "found under the call's own overrides" flag, so the skip check sees the same reads.
- [x] 1.6 Narrow what a handler captures to the slots its node reads (`slots_read` of the handler
  node, computed from the image at no charge and cached per node), held and compared in ascending
  slot index, so handler comparisons and the written and stored forms cover only those slots; live
  state reads unchanged.
- [x] 1.7 Unit tests for each scenario in the `reactivity` and `nx-ir-format` deltas, the tree's
  reader-through-a-reused-default case and the bundle that reads a value it sets included, and a
  test that a reused shallow default lets a deep read succeed alike in both runtimes.

## 2. TypeScript IR runtime

- [x] 2.1 The same layer identities, cache, read recording and reuse in
  `runtime/typescript/src/index.ts`, following the Rust rule exactly. Its dispatch is unchanged.
- [x] 2.2 The same narrowed handler capture as task 1.6, slot for slot, in ascending slot index.
- [x] 2.3 Tests for the same scenarios, the `component-action-handlers` ones included, and rebuild
  `runtime/typescript/dist`.

## 3. Generated code

- [x] 3.1 Layer identities and a per-call cache in the generated environment state
  (`crates/nx-codegen/src/runtime/environment.js`), used by `nxEnvRead`, `nxProvidedOrDefault`,
  formulas and the thunks of top-level values that read the environment, a bundle provide keyed by
  the value read.
- [x] 3.2 A test-only evaluation counter in the generated helpers, off unless the test harness turns
  it on, counting each evaluation of a default, provide, formula or top-level value thunk.
- [x] 3.3 Tests for the `executable-code-generation` scenarios, reading the counter to show that a
  default is evaluated once.

## 4. Component lifecycle (Rust, for the tree)

- [x] 4.1 Record on `ComponentInstance`, in memory only, the state fields and props a render read:
  slots the body evaluated, slots `capture_formula` captured, and the prop slots of each handler
  node the body binds (`slots_read`). The stored form is unchanged.
- [x] 4.2 Split `ComponentInstance` so that its render part (handlers, generation and tokens) is
  shared by an instance whose dispatch or prop skip kept its render, and make
  `ComponentInstance::is` compare that part. Props, handler props, state and records stay outside
  it.
- [x] 4.3 An internal dispatch entry for the tree: one call that applies the handlers as
  `dispatch_component_actions` does, compares each patched field the render read (charged as `diff`
  charges a field), and renders only when one differs or the tree says the node's other records do
  not hold; otherwise it returns the new state and effects with no output written.
- [x] 4.4 An internal entry that normalizes a descriptor's props without rendering, charged as
  initialization charges normalizing them, and an internal initialization from props already
  normalized, so a node whose recorded prop differs is charged for normalization once.
- [x] 4.5 Leave `dispatch_component_actions` and the TypeScript dispatch rendering on every batch;
  check that the corpus's lifecycle results and the `rust-ir-runtime` and `typescript-ir-runtime`
  tests are unchanged.

## 5. Instance tree (Rust)

- [x] 5.1 Node records of direct inputs in read order (root reads with revisions and edges, kept
  results used, override-supplied reads with values, reads under the render's own overrides as
  today) and a per-node `verified` revision; replace the leaf-read check of
  `environment_reads_hold` in `visit`, `set_environment` and dispatch with the record check.
- [x] 5.2 Prop-read skip in `visit`: normalize the new props, compare the recorded ones with
  `Across` semantics under `Program::metered`, rebind through `rebinding`/`rebind` when the parent's
  instance or a handler changed, and give the instance the new props without rendering.
- [x] 5.3 Dispatch skip in `dispatch_chain` through the entry of 4.3: keep the rendered output,
  tokens and generation, take the new state, leave the node settled and its children's `through`
  matching.
- [x] 5.4 Kept derived results: which results are kept (none of their reads under a layer the
  render pushed; no formulas or in-render bundles), the internal runtime option that passes them to
  a render's `Machine`, entry into the call's cache with tokens over the host layers, a revision
  counter, `changed` and `verified` per variant, and variants told apart by read classes and
  override-supplied values, tried in the order kept. Each variant also keeps the flattened set of
  its leaf reads with their supplier class, used for the "no layer the render pushed touches it"
  check and the tokens it enters the call with, not for validation.
- [x] 5.5 Validation: classify each read at the use, check inputs in read order and stop at the
  first that changed, then evaluate again; check an edge by deep verify (validate the used result
  first, evaluate it again with the cutoff when not valid, then compare its `changed` with the
  user's `verified`); look up override-supplied values as the skip check does,
  formulas included; equality cutoff by written identity (number kind, `-0.0`, NaN, origins when
  collected), charged as a comparison.
- [x] 5.6 Reference counts or reverse edges for variants, dropping a variant no record references
  when the tree operation ends; the extended `readers` index, override-supplied reads indexed under
  the root values their lookup read; `set_environment` marks possibly stale through the indexes,
  then pulls in depth order.
- [x] 5.7 Add the kept results, revision counter, reverse indexes and change set to `atomically`'s
  snapshot.
- [x] 5.8 The change set and `take_changes` as a diff by key (replacement as removed and added,
  every dropped node listed, rebinding as rendered, a new descriptor or state alone not listed),
  consistent with `is_settled`.
- [x] 5.9 Tests for every `component-instance-tree` and `reactivity` tree scenario, including the
  modified scenarios of the existing requirements, a `set_environment` whose render fails after a
  kept result was evaluated again, and a dispatch skip followed by a tap on the old output.
- [x] 5.10 A differential stress test of the tree: seeded random root changes (values written
  differently that compare equal included), dispatches and passes over a program that mixes the
  risky features, each step checked against a fresh initialization of every node and against the
  change set.

## 6. Corpus and cost model

- [x] 6.1 Add the `environment-reuse` corpus program with entrypoints for each `nx-ir-format`
  scenario, the bundle that reads a value it sets read for two values included, and run it in
  generated JavaScript too.
- [x] 6.2 Regenerate every program's `operations.json`, which also takes the smaller handler
  comparisons of the narrowed capture, and check that all three engines agree. `results.json` stays
  unchanged except where a program compares handlers that differ only in a slot they do not read;
  accept those diffs and list them in the PR description.
- [x] 6.3 Record `design-tokens-theme`'s counts before and after in the PR description.
- [x] 6.4 Run the cost differential and allocation tests.

## 7. Documentation

- [x] 7.1 `docs/nx-ir-format.md`: the cost requirement's first sentence (the tree's held state),
  the cost table's node row, the environment section's lookup and cost paragraphs, and a section on
  reusing derived results, with where an evaluation begins, the bundle key and the note on limits.
- [x] 7.2 `specs/ir-conformance/README.md`: describe `environment-reuse`.
- [x] 7.3 `specs/future.md`: remove "Memoizing defaults", "A bundle's values are computed at every
  read" and most of "Reading a token is expensive". Rewrite the reactivity direction and the
  instance-tree sections in terms of the tracked-computation model, marking state reads in the tree,
  prop reads, kept results, the change set and the narrowed handler capture ("What a handler
  captures") done, and keep reuse finer than a body, a dispatch skip in the bare APIs and a
  self-walking tree as later work.
- [x] 7.4 Document the change set, the new skips, kept results and their cost in the instance
  tree's module docs and `docs/` where the tree is described, with an example of a host that
  updates from the change set.
