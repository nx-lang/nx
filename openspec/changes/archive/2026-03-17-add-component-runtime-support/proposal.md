## Why

Component syntax, emitted actions, and action handler callbacks now exist, but NX still stops short
of executing components end to end. This change closes that gap by defining how components lower,
initialize, and dispatch through a stateless Rust runtime so host environments can render
components, persist state, and feed actions back into the interpreter.

## What Changes

- Add lowering and interpreter support for component definitions so a host can initialize a
  component with props and receive both the rendered element and the component's initial state.
- Add a runtime binding contract for dispatching a batch of actions against a prior component state
  and returning the next state snapshot plus the ordered list of effect actions produced during
  dispatch.
- Define how component `state`, emitted actions, and `on<ActionName>` handlers participate in the
  init/dispatch lifecycle while keeping the Rust interpreter stateless between calls. In this
  change, state is initialized during component startup and then carried through dispatch; the
  declarative actions that mutate state land in a follow-up change.
- Preserve the existing host-owned state model by requiring callers to pass prior state into
  dispatch and store returned state between invocations.
- Add tests that ensure strong coverage for this NX core functionality, including component
  initialization, dispatch/effect plumbing, and effect action output. State-update behavior tests
  will be added when the declarative state-update actions land in a subsequent change.
- Update runtime-facing docs to describe the component initialization and dispatch contract.

## Capabilities

### New Capabilities
- `component-runtime-bindings`: Define the host/runtime contract for initializing components and
  dispatching actions against serialized component state.

### Modified Capabilities
- `component-syntax`: Extend component declarations and invocations with executable lowering rules
  for props, rendered output, and initial state materialization.
- `component-action-handlers`: Define how emitted action handlers are applied during component
  dispatch and how their results contribute to the returned effect action list.

## Impact

- `crates/nx-hir` lowering for component definitions, invocations, state fields, and emitted-action
  metadata
- `crates/nx-interpreter` component initialization and batched dispatch execution paths
- runtime-facing value models and bindings in `crates/nx-api`, `crates/nx-ffi`, and any exposed
  host bindings that surface component init/dispatch
- Documentation, examples, and tests that currently describe full component runtime behavior as a
  deferred follow-up
