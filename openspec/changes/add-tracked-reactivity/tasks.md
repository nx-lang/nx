## 1. Rust IR runtime

- [ ] 1.1 Give each environment layer a per-call sequence number (`EnvState`), the host's layers
  included, and a helper that returns a value's token over the layers below a given position.
- [ ] 1.2 Add a per-call derived-result cache to `Machine`: entries keyed by derived result (value
  declaration, environment default, entry-module provide, formula layer), each holding the value
  and its recorded reads with their tokens and the values found for the instance tree's read list.
- [ ] 1.3 Record reads during a derived evaluation, nested evaluations and reuses included, against
  every derived result being evaluated around them.
- [ ] 1.4 Reuse in `eval_reference` (value declarations), `provided_or_default` (defaults and
  entry-module provides) and `source_value` (formulas) when every recorded token matches; charge
  nothing beyond the asking node on reuse.
- [ ] 1.5 Replay a reused result's reads into the instance tree's read list, keeping the
  "found under the call's own overrides" flag, so the skip check sees the same reads.
- [ ] 1.6 Unit tests for each scenario in the `reactivity` and `nx-ir-format` deltas, the tree's
  reader-through-a-reused-default case included.

## 2. TypeScript IR runtime

- [ ] 2.1 The same layer identities, cache, read recording and reuse in
  `runtime/typescript/src/index.ts`, following the Rust rule exactly.
- [ ] 2.2 Tests for the same scenarios, and rebuild `runtime/typescript/dist`.

## 3. Generated code

- [ ] 3.1 Layer identities and a per-call cache in the generated environment state
  (`crates/nx-codegen/src/runtime/environment.js`), used by `nxEnvRead`, `nxProvidedOrDefault`,
  formulas and the thunks of top-level values that read the environment.
- [ ] 3.2 Tests for the `executable-code-generation` scenarios, including that a default is
  evaluated once.

## 4. Component lifecycle (Rust and TypeScript)

- [ ] 4.1 Record the state fields and props a render's body reads, and the props and state slots read
  by each handler it binds (`slots_read`), on `ComponentInstance`; store and restore the records.
- [ ] 4.2 In `dispatch_component_actions` (both runtimes), compare the ending state with the
  starting state field by field, charged as `diff`; render only when a read field changed, and
  report whether it rendered.
- [ ] 4.3 Tests for the `reactivity` dispatch scenarios in both runtimes, with equal counts.

## 5. Instance tree (Rust)

- [ ] 5.1 Prop-read skip in `visit`: compare only recorded props, keep the render and take the new
  descriptor when they are equal.
- [ ] 5.2 Kept derived results in the tree, offered to each render's `Machine`, checked by value at
  the use, charged as the skip check's reads and comparisons.
- [ ] 5.3 Reverse indexes for kept results beside `readers`; `set_environment` marks stale only what
  they name; keep them current as nodes and results change.
- [ ] 5.4 The change set and `take_changes`, consistent with `is_settled`; a dispatch that renders
  nothing leaves the node settled.
- [ ] 5.5 Tests for every `component-instance-tree` scenario.

## 6. Corpus and cost model

- [ ] 6.1 Add the `environment-reuse` corpus program with entrypoints for each `nx-ir-format`
  scenario, and run it in generated JavaScript too.
- [ ] 6.2 Regenerate every program's `operations.json`, and check that all three engines agree.
- [ ] 6.3 Record `design-tokens-theme`'s counts before and after in the PR description.
- [ ] 6.4 Run the cost differential and allocation tests.

## 7. Documentation

- [ ] 7.1 `docs/nx-ir-format.md`: the cost table's node row, the environment section's lookup and
  cost paragraphs, and a section on reusing derived results.
- [ ] 7.2 `specs/ir-conformance/README.md`: describe `environment-reuse`.
- [ ] 7.3 `specs/future.md`: remove "Memoizing defaults", "A bundle's values are computed at every
  read" and most of "Reading a token is expensive". Rewrite the reactivity direction and the
  instance-tree sections in terms of the tracked-computation model, marking state reads, prop reads,
  kept results and the change set done, and keep reuse finer than a body and a self-walking tree as
  later work.
- [ ] 7.4 Document the change set and the new skips in the instance tree's module docs and
  `docs/` where the tree is described, with an example of a host that updates from the change set.
