## Why

NX needs one answer to the question every reactive UI system answers: when data changes, what has
to be computed again? Today it has two partial answers. The instance tree records which environment
values each component render read, so that a change to a root value renders again only the nodes
that read it, and it skips a child whose props are unchanged. Everything else is computed again
every time it is asked for. A top-level value is evaluated at every `reference`. An environment
value's default, an entry-module provide and a formula are evaluated at every read that reaches
them.

That second gap is what makes design tokens slow. In `specs/ir-conformance/design-tokens-theme`,
one `env.color.accent` read costs 28,440 operations and one `env.button.radius` read costs 231,969,
mostly rebuilding `standardPalette` and the `color` and `text` defaults again and again.

Fixing the token cost with a cache of its own would give NX a third mechanism. This change instead
defines one general model, *tracked computations*, that the tree's renders and these derived values
both follow. It implements the part that removes the token cost now, and lays out how state reads
and reuse across passes follow from the same model.

## What Changes

- **One model for reactivity.** A *source* is a piece of data that can change: an environment value
  as an override, the host's root values or a default supply it, a component's state field, or a
  prop. A *tracked computation* is a component render, a top-level value, an environment value's
  default, an entry-module provide or a captured formula. Each tracked computation records the
  sources it actually read, at any depth, and what it found. A computation is reused while every
  recorded read would find the same thing, and is computed again otherwise. Reads are tracked at
  run time, not predicted by the compiler, so a branch that reads a value only sometimes depends on
  it only when taken. The language gets no new syntax: authors write plain values and components.
- **Within one call, reads compare by identity.** Every override the call pushes has an identity of
  its own. A read records the identity of the override that supplied it, so checking a record is
  free and never depends on the size of a value. A new override counts as a change even when it
  sets an equal value.
- **Across calls, reads compare by value.** The instance tree already holds each render's
  environment reads with the values found, and renders a node again only when one of them differs.
  That is the same model, with values compared because the overrides of one call are gone by the
  next.
- **Derived values are reused within one call.** This change's implementation: the Rust and
  TypeScript IR runtimes and generated code keep the last result of each top-level value, default,
  entry-module provide and formula for the rest of the call, under the rule above. A value that
  reads no environment, such as `standardPalette`, is evaluated once per call.
- **The tree's read records become the model's records.** The records the tree already keeps for
  its renders are built from the same reads, including reads inside reused results, so the tree
  sees the same dependencies whether a derived result was evaluated or reused.
- **The cost model changes.** A reused result costs only the asking node's one operation.
  `docs/nx-ir-format.md`'s cost rules and every recorded corpus count that reads a value or default
  more than once change. No result changes. Counts stay a function of the images, the input and,
  for the tree, the state it holds.
- **A corpus program for reuse.** `environment-reuse` pins each case of the rule with its
  operation counts.
- **The next steps are recorded.** Tracking reads of state fields, so that a dispatch renders a
  body again only when it read a patched field, and keeping derived results in the tree across
  passes, so that a render after a dispatch reuses the palette, follow from the same model.
  `design.md` lays them out, and `specs/future.md` records them as the next change.

## Capabilities

### New Capabilities

- `reactivity`: the tracked-computation model, the reads a computation records, and reuse within
  one call.

### Modified Capabilities

- `nx-ir-format`: the cost rules charge a reused result nothing beyond the asking node, and
  environment lookups reuse defaults, provides and formulas.
- `executable-code-generation`: generated code reuses derived results within one call under the
  same rule.

## Impact

- `crates/nx-ir-runtime`: layer identities on the environment stack, a per-call record of tracked
  computations in `Machine`, reuse in `eval_reference`, `provided_or_default` and `source_value`,
  and the tree's read records built from the same reads.
- `runtime/typescript`: the same in the TypeScript IR runtime, and `dist` rebuilt.
- `crates/nx-codegen`: the generated environment helpers and the thunks of top-level values that
  read the environment.
- `docs/nx-ir-format.md`, `specs/ir-conformance/README.md` and `specs/future.md`.
- `specs/ir-conformance`: recorded operation counts go down, and `environment-reuse` is new.
- Hosts see no API change. A host that sets a budget sees smaller counts.

## Non-goals

- **State-read tracking and reuse across passes, in this change.** Both are designed here and are
  the next change, so each step can be measured on its own.
- **Fine-grained reactivity.** No signals or subscriptions in the language, and nothing finer than a
  component body, as the reactivity direction in `specs/future.md` already decided.
- **Lazy record fields.** Reading `env.button.radius` still evaluates the whole `button` default
  once. Evaluating fields lazily would change when errors appear.
