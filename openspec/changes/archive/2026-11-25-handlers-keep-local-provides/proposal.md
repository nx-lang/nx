## Why

An action handler runs in its component's environment and loses the `provide` blocks and `env.`
attributes written around it. A handler that reads `env.` inside a local provide sees the value from
outside the block, so a button inside `provide env.locale="fr-FR"` formats its message in the
component's locale. The two IR runtimes and the instance tree all do this today, and `specs/future.md`
lists it as the one correctness gap left in environment values ("Handlers lose local provides").

Probes against `origin/main` (2f1f7c0b) show the gap. `environment mode: string = "outer"`, and
`Page` binds `onTapped=<State.Update last={env.mode} />` in four places:

| Where the handler is written | Rust tree | Rust bare API | TypeScript runtime | Expected |
| --- | --- | --- | --- | --- |
| `provide env.mode="block" { <Button … /> }` | `outer` | `outer` | `outer` | `block` |
| `<Button env.mode="attr" … />` | `outer` | `outer` | `outer` | `attr` |
| `provide env.mode={env.mode + "!"} { … }` | `outer` | `outer` | `outer` | `outer!` |
| `<Card env.mode="card" onLogged=… />`, an emit | `outer` | `card` or `outer` (1) | `card` or `outer` (1); `outer` by the parent's token | `card` |
| content of `<Box>` inside `provide env.mode="page"`, where `Box` puts it in `provide env.mode="box"` | `outer` | `page` or `outer` (1) | `page` or `outer` (1) | `page` |

(1) The bare API runs a handler that the dispatched instance did not bind in the environment the host
gives for that call. A host that gives the child's environment gets the overrides on the child's
descriptor, and a host that gives the root gets none.

Generated JavaScript is not affected, because it refuses action handlers
(`executable-code-generation`, "Generated component code rejects action-handler bindings").

## What Changes

- **A handler sees the overrides where it is written.** Its body runs in the environment of the
  instance it runs against, with the overrides of the `provide` blocks and `env.` attributes around
  the handler (an element's own `env.` attributes included) inside it. An override that is a formula
  is computed at each read in the handler's body when the handler runs, as every environment value
  is. Overrides around the place where another body puts the element do not apply, just as they don't
  apply to the handler's locals.
- **Only a handler that can read the environment captures overrides.** The compiler finds this from
  the handler body's static read set, with the same analysis that marks formulas, and flags the
  `actionHandler` node. The flag is a trailing cell behind a new required feature, `environment-v3`.
  An unflagged handler captures nothing, and its cost and `==` are unchanged.
- **What is captured.** A flagged handler holds the overrides of the `envProvide` nodes being
  evaluated around it, outermost first, in `$env` entry form: plain values as evaluated where
  written, and formulas as captured data. It does not hold the component's own `$env` chain or the
  root values. Those come from the instance the handler runs against, so they are current when it
  runs.
- **Equality and storage.** Two handlers made by one flagged node compare their captured slots,
  then their captured overrides, as two lists compare. Two handlers that differ only in an
  override around them are no longer equal (**BREAKING** for `==` on such handlers). A stored Rust
  instance writes each handler's captured overrides and checks them when it is restored. Rebinding
  walks them as it walks captured slots.
- **Instance tree.** A handler runs against its creator in the creator node's current environment,
  with its captured overrides inside. The captured overrides come from values the render already
  records, so the tree gets no new kind of read and its skip rules are unchanged.
- **Bare API.** A dispatch runs every handler in the environment the host gives for the call, with
  the handler's captured overrides inside. That is exact for a handler the dispatched instance bound.
  A host that wants a parent-bound handler to see exactly its creator's environment dispatches the
  parent's token against the parent, as the tree does.
- `specs/future.md` drops the limitation and records what stays for later.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `environment-values`: a handler's body sees the overrides written around the handler.
- `component-action-handlers`: a handler that can read the environment captures the overrides
  around it. Its body runs with them inside the environment of the call, and its equality covers
  them. Both IR runtimes do this alike.
- `component-instance-tree`: a handler runs in its creator node's current environment with its
  captured overrides. Captured overrides are not new reads.
- `nx-ir-format`: the `actionHandler` node's environment flag, the `environment-v3` feature, the
  cost of comparing captured overrides, and a corpus program.
- `rust-ir-runtime`: a stored instance keeps the overrides its handlers captured.

## Impact

- **Compiler.** `crates/nx-codegen/src/formulas.rs` marks handlers whose body can read the
  environment. `crates/nx-codegen/src/ir.rs` emits the flag and the feature. `crates/nx-ir` gets
  the feature constant, the layout and the `explain` text.
- **Rust IR runtime** (`crates/nx-ir-runtime`):
  - `environment.rs`: the `$env` entries of the provides being evaluated, kept beside the layers.
  - `eval.rs`: handler creation.
  - `value.rs`: `Handler` and `handlers_equal`.
  - `component.rs`: `invoke_handler` and rebinding.
  - `stored.rs`: the stored form.
  - `module.rs`: the reader and the feature check.
  - `tree.rs`: no new logic. Tests cover the tree.
- **TypeScript IR runtime** (`runtime/typescript/src/index.ts`): the same, and `dist` is rebuilt.
- **Generated code:** no change.
- **Corpus:** a new program, `specs/ir-conformance/environment-handlers`.
- **Docs:** `docs/nx-ir-format.md`, the environment reference page on the website, and
  `specs/future.md`.
