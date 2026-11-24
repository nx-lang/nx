## Why

NX needs one answer to the question every reactive UI system answers: when data changes, what has
to be computed again, and how does the host find out? Today it has partial answers that don't share
a model:

- The instance tree records which environment values each render read, and a root-value change
  renders again only the nodes that read one that changed.
- A child whose descriptor is unchanged keeps its render.
- A dispatch renders the dispatched component's whole body, even when the handlers patched a state
  field the body never reads.
- A top-level value is evaluated at every `reference`, and an environment value's default, an
  entry-module provide and a formula at every read that reaches them.
- The host learns what changed only by walking the tree and asking `is_settled` at each node.

The fourth gap is what makes design tokens slow. In `specs/ir-conformance/design-tokens-theme`, one
`env.color.accent` read costs 28,440 operations and one `env.button.radius` read costs 231,969,
mostly rebuilding `standardPalette` and the `color` and `text` defaults again and again.

Fixing each gap with its own mechanism would leave NX with several overlapping ones. This change
defines one general model, *tracked computations*, and applies it to all of them at once, so the
design can be judged as a whole rather than one phase at a time.

## What Changes

- **One model for reactivity.** A *source* is data that can change: an environment value (which an
  override, a host root value, an entry-module provide or a default supplies), a component's state
  field, and a prop. A *tracked computation* is a component render, a top-level value, an
  environment value's default, an entry-module provide or a captured formula. Each tracked
  computation records the sources it actually read at run time, at any depth, and what it found.
  Reading another tracked computation adds that computation's reads to its own record. A
  computation is reused while every recorded read would find the same thing, and is computed again
  otherwise. Reads are tracked, not predicted by the compiler, so a branch that reads a value only
  sometimes depends on it only when taken. The language gets no new syntax.
- **Within one call, reads compare by identity.** Every override a call pushes has an identity of
  its own, and a read records the override that supplied it. Checking a record is free and never
  depends on the size of a value. A new override counts as a change even when it sets an equal
  value.
- **Derived values are reused within one call**, in the Rust and TypeScript IR runtimes and in
  generated code. A value that reads no environment, such as `standardPalette`, is evaluated once
  per call.
- **A dispatch renders only when a field the render read changed.** A render records the state
  fields it read. After a dispatch's handlers run, the body renders again only when they patched one
  of those fields. Otherwise the instance takes the new state and keeps its rendered output. Both IR
  runtimes do this, since both have the component lifecycle. Handlers read state live when they run,
  as today, so a field that only handlers use never causes a render.
- **Props are sources too.** A node handed a new descriptor renders again only when a prop its last
  render read, or a prop one of the handlers it bound reads, differs. Otherwise it takes the new
  props and keeps its render, as it already does for a change that touches only `$env`.
- **The instance tree keeps derived results across renders, checked by revision.** Results that
  renders evaluated are held in the tree and offered to later renders of any node, so the render
  after a click reuses the palette. As in incremental engines such as Salsa, each root change and
  each changed result takes a new revision, and a kept result records its direct inputs. Checking
  one compares revision numbers, which is free. A read that an override above the node supplied,
  such as `env.colorMode=dark` on a section, is compared by value. A light and a dark section keep a
  variant each.
- **Equal results stop a change from spreading.** A result evaluated again is compared once with
  its old value. When they are equal, it keeps its old revision, and nothing that used it is
  evaluated or rendered again. So when `locale` changes and `direction` is still `ltr`, the nodes
  that read only `direction` are left alone.
- **Changes are pushed, then pulled.** Every source keeps an index of what read it directly, as the
  tree's `readers` index does for root values today. A root change marks possibly stale exactly what
  the indexes reach, evaluating nothing. The tree then validates those nodes from the root down,
  evaluating again only the results that are not valid, and renders a node only when one of its
  direct inputs changed. A dispatch decides from the instance's own state record.
- **The host gets a change set.** The tree reports which nodes rendered again, which were added and
  which were removed since the host last asked. A host updates what it derived from those nodes
  without walking the tree or comparing output. `is_settled` remains for hosts that walk.
- **The cost model changes.** Within a call, a reused result costs only the asking node's one
  operation. In the tree, revision checks are free, an override-supplied read is charged like the skip check,
  and the cutoff comparison is charged as a comparison. Counts stay a
  function of the images, the input and, for the tree, the state it holds. No result changes, and
  `docs/nx-ir-format.md`, the corpus's recorded counts and the tree's tests change to match.
- **A corpus program for reuse.** `environment-reuse` pins the within-call cases with their
  operation counts. The tree's cases are pinned by Rust tests, since the corpus has no tree.

## Capabilities

### New Capabilities

- `reactivity`: the tracked-computation model, the reads a computation records, reuse within one
  call, renders that follow state and prop reads, and push-based staleness.

### Modified Capabilities

- `nx-ir-format`: the cost rules charge a reused result nothing beyond the asking node, and
  environment lookups reuse defaults, provides and formulas.
- `component-instance-tree`: derived results kept across renders, prop-read skipping, push-based
  staleness, the change set, and read records that include reused results.
- `executable-code-generation`: generated code reuses derived results within one call.

## Impact

- `crates/nx-ir-runtime`: layer identities on the environment stack; a per-call record of tracked
  computations in `Machine`; reuse in `eval_reference`, `provided_or_default` and `source_value`;
  state and prop read records on `ComponentInstance`, stored and restored with it; the dispatch skip
  in `dispatch_component_actions`; and, in `tree.rs`, the kept results, the reverse indexes and the
  change set.
- `runtime/typescript`: within-call reuse, state read records and the dispatch skip, and `dist`
  rebuilt.
- `crates/nx-codegen`: the generated environment helpers and the thunks of top-level values that
  read the environment.
- Hosts of the tree (the DrawnUI fiddle's `nx-rust` language, the .NET binding where it wraps the
  tree) can switch from walking to the change set, and see smaller counts.
- `docs/nx-ir-format.md`, `specs/ir-conformance/README.md` and `specs/future.md`.

## Non-goals

- **A tree the runtime walks itself.** The host still visits the descriptors it finds, and keys stay
  the host's. The change set says what changed among the nodes the host created.
- **An instance tree in the TypeScript runtime.** The tree's parts are Rust-only until it has one.
- **Fine-grained reactivity.** No signals or subscriptions in the language, and nothing finer than a
  component body, as the reactivity direction in `specs/future.md` already decided.
- **Lazy record fields.** Reading `env.button.radius` still evaluates the whole `button` default
  once. Evaluating fields lazily would change when errors appear.
