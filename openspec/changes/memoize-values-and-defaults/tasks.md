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

## 4. Corpus and cost model

- [ ] 4.1 Add the `environment-reuse` corpus program with entrypoints for each `nx-ir-format`
  scenario, and run it in generated JavaScript too.
- [ ] 4.2 Regenerate every program's `operations.json`, and check that all three engines agree.
- [ ] 4.3 Record `design-tokens-theme`'s counts before and after in the PR description.
- [ ] 4.4 Run the cost differential and allocation tests.

## 5. Documentation

- [ ] 5.1 `docs/nx-ir-format.md`: the cost table's node row, the environment section's lookup and
  cost paragraphs, and a section on reusing derived results.
- [ ] 5.2 `specs/ir-conformance/README.md`: describe `environment-reuse`.
- [ ] 5.3 `specs/future.md`: remove "Memoizing defaults", "A bundle's values are computed at every
  read" and most of "Reading a token is expensive". Rewrite the reactivity direction's steps in
  terms of the tracked-computation model, with state-read tracking and reuse across passes in the
  tree (design Decision 9) as the next change.
