## Why

A component's `T.Property` and `T.Update` describe its state today, while the same names on a record or
an action describe that type's fields. So `Counter.Property` lists state fields and never props, and
nothing names a component's props. A host or tool that wants to reason about props, such as the
previewer changing props or a table naming the column a component sorts by, has no type to use.
There is also no type for a component's state as a whole, so a state snapshot can't be passed to a
function, compared with `diff`, or patched with `apply`.

This change makes the derived names mean one thing everywhere: `T.Property` and `T.Update` describe
the fields a value of `T` is written with, which for a component are its props. State moves to its
own derived record, `T.State`, which gets the usual `T.State.Property` and `T.State.Update`.

## What Changes

- **BREAKING** A component's `T.Property` is derived from its props, not its state. Every component
  has one, whether or not it has state.
- **BREAKING** A component's `T.Update` is a patch of its props, not its state. Every component has
  one. A handler can't return it to change its own component, because props belong to the parent.
- A component with a `state` group gets a derived record `T.State` with exactly its state fields.
  `T.State` is an ordinary record type: it has `T.State.Update` and `T.State.Property`, works with
  `apply`, `merge`, `diff` and `changed`, and can be written as a literal.
- **BREAKING** A handler changes its component's state by returning `T.State.Update`, not
  `T.Update`. The wire form of a state patch carries `$type` `<Component>.State.Update`.
- **BREAKING** Inside a component body, a bare `State` names the component's `T.State`, so a
  handler writes `<State.Update count={count + 1} />` and a type or case writes `State.Property`.
  The bare `Update` and `Property` shorthands are removed. Outside a component a bare `State` is an
  ordinary name.
- `State` joins `Update` and `Property` as a reserved emitted-action name.
- NX IR names a component's state record, and both IR runtimes recognize a state patch by that
  record rather than by the component.
- `typegen` emits `<Name>_update` and `<Name>_property` for a component from its props, and
  `<Name>_state_update` and `<Name>_state_property` beside `<Name>_state` for a component with state.
- Docs, examples and conformance programs move to the new spellings.

## Capabilities

### New Capabilities

- `component-state-type`: the derived `T.State` record of a component with state, the bare `State`
  name inside a component body, and `State` as a reserved emitted-action name.

### Modified Capabilities

- `update-records`: a component's `T.Update` patches its props, and every component has one; the
  bare `Update` tag is removed.
- `property-references`: a component's `T.Property` lists its props, and every component has one;
  the bare `Property` name is removed.
- `component-action-handlers`: a handler patches state with the component's `T.State.Update`, and
  returning the component's props patch is rejected.
- `component-syntax`: the reserved emitted-action names gain `State`.
- `nx-ir-format`: a component names its state record, and a state patch targets that record.
- `cli-code-generation`: component companions follow the new derivations, and state gains update
  and property companions.
- `typescript-ir-runtime`: the state-patch operation takes `<Component>.State.Update` and rejects a
  component's props patch. The rest of its scenarios change spelling only.
- `component-type-parameters`: the generic-component scenario covers both property unions.
- Spelling only, where scenarios write a state patch as `<State.Update …/>`: `component-instance-tree`,
  `component-runtime-bindings`, `rust-ir-runtime`, `playground`, `source-tree`, `viewer` and
  `fiddle-nx-language`.

## Impact

- **Compiler:** derived declarations and bare-name resolution in `crates/nx-hir` (`lower.rs`,
  `lib.rs`); handler result routing, intrinsics and diagnostics in `crates/nx-types/src/infer.rs`.
- **Codegen and runtimes:** state metadata in `crates/nx-codegen`. State patch recognition in
  `crates/nx-ir-runtime`, `runtime/typescript` and the JS emitter. The HIR interpreter changes only
  as much as its existing tests need, since it is being retired.
- **Bindings and tooling:** `crates/nx-api` derived exports, `crates/nx-cli` typegen, the .NET
  generated fixtures and tests.
- **Sources:** about 15 `.nx` files and 10 IR conformance programs use the old spellings, along
  with six website pages, `docs/nx-ir-format.md` and the playground's counter example.
- **Not changed:** the host-owned state snapshot the runtime bindings pass in and out stays a field
  map, so hosts that hold state don't change. The previewer's `setProps` keeps its current shape;
  typing it as `T.Update` is a possible follow-up.
