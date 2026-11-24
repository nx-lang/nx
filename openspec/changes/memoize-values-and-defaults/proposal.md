## Why

Reading a design token is slow. In `specs/ir-conformance/design-tokens-theme`, one
`env.color.accent` read costs 28,440 operations and one `env.button.radius` read costs 231,969,
even though the brand sets the radius outright. Three things multiply:

- an environment value's default, an entry-module provide and a formula are evaluated again at
  every read that reaches them;
- reading one field of a record default evaluates the whole record, so `env.button.radius` also
  evaluates `env.color` twice and all of `env.text`;
- a top-level value is evaluated again at every `reference`, so each of the dozen
  `standardPalette.…` reads in `env.color`'s default builds all 55 colors.

NX evaluation is pure, so evaluating one of these again where nothing it depends on has changed
gives the same value. Most of this work can be skipped without changing any result. Doing it before
UI libraries depend on `@nx/design` keeps token reads cheap from the start, and makes the tokens
cheap to read without asking authors to restructure them.

The reactivity design agreed with `skip-unchanged-children` and `add-environment-values` already
tracks, at run time, which environment values a component body read, so that a node renders again
only when one of them changed. This change applies the same idea one level down: a derived value is
evaluated again only when an environment value it actually read is found from different overrides.

## What Changes

- **Results are reused within one evaluation.** Within one call of an evaluation API, a runtime
  keeps the result of each evaluation of four kinds of *derived result*:
  - a top-level value declaration's value, checked against its declared type;
  - an environment value's default, checked against its type;
  - an entry-module provide's value, a bundle provide's bundle included;
  - a captured formula's value.

  With each result it records the environment values the evaluation read, at any depth, and where
  each read found its value. A later use of the same derived result reuses the result when every
  recorded read would still find its value from the same overrides. Otherwise the result is
  evaluated again, and the new result replaces the old one.
- **Reads are tracked, not predicted.** The record is the reads the evaluation made, not a static
  read set the compiler works out. A default that reads `env.colorMode` in only one branch depends
  on `colorMode` only when it takes that branch. Both runtimes make the same reads in the same
  order, so they make the same reuse decisions and count the same operations.
- **An override counts as a change whatever its value.** A read whose nearest override is a
  different `envProvide`, `env.` attribute, `$env` entry or bundle entry counts as changed, even
  when the override sets an equal value. Comparing values would cost operations, and an identity
  check costs none.
- **Values that read no environment are evaluated once per call.** A top-level value with no
  environment reads, such as `standardPalette`, records none, so its first result is reused for the
  rest of the call.
- **The cost model changes.** A reused result costs nothing beyond the node that asked for it: the
  `reference` or `envRead` node's one operation. The first evaluation, and each evaluation again,
  is charged as today. `docs/nx-ir-format.md`'s cost rules and the conformance corpus's recorded
  operation counts change to match. No result changes.
- **Generated code reuses results the same way.** Generated JavaScript and TypeScript keep the
  same derived results for one call of a generated entry function or one component evaluation,
  under the same rule, so all three engines give the same values with similar costs. Generated code
  counts no operations, so only its speed changes.
- **A corpus program for reuse.** `environment-reuse` pins the rule's cases with their operation
  counts: a value read twice, a default read twice, a default evaluated again after an override of
  a value it reads, a default reused across an override of a value it doesn't read, a branch that
  reads a value only sometimes, an override with an equal value, a formula read twice, and reads
  inside nested provides.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `nx-ir-format`: the cost rules charge a reused derived result nothing beyond the node that asked
  for it, and environment lookups reuse defaults, provides and formulas under the rule above.
- `executable-code-generation`: generated code reuses derived results within one call under the
  same rule.

## Impact

- `crates/nx-ir-runtime`: a per-call cache in `Machine`, layer identities on the environment stack,
  read recording during derived evaluations, and reuse in `eval_reference`,
  `provided_or_default` and `source_value`.
- `runtime/typescript`: the same cache in the TypeScript IR runtime, and `dist` rebuilt.
- `crates/nx-codegen`: the generated environment helpers (`nxEnvironment`, `nxWithEnvironment`,
  `nxFormula`) and the code emitted for top-level values that read the environment.
- `docs/nx-ir-format.md`, `specs/ir-conformance/README.md` and `specs/future.md`: cost rules,
  the new corpus program, and the entries this change resolves.
- `specs/ir-conformance`: every program's recorded operation counts that read a value or a default
  more than once go down, and `environment-reuse` is new.
- Hosts see no API change. A host that sets a budget sees smaller counts.

## Non-goals

- **Reuse across calls.** The cache lives for one call, so the operation count stays a property of
  the images and the input alone. A result kept across calls would make a call's count depend on
  the calls before it. Each component render is one call, so a palette is still built once per
  render. Keeping results across calls, for example by charging a reused result the operations its
  first evaluation cost, is left for later.
- **Lazy record fields.** Reading `env.button.radius` still evaluates the whole `button` default
  once. Once that result is reused, its other fields cost nothing more, and evaluating fields lazily
  would change when errors appear.
- **Tracking reads of component state.** Re-rendering only the bodies that read a patched state
  field is step 2 of the reactivity plan in `specs/future.md`, and is a change of its own.
