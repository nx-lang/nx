## Why

Component declarations can now describe emitted actions, but component call sites still have no
first-class way to react to them. Adding action handler properties now gives lowering and the
interpreter a stable callback model for emitted actions before full component runtime support lands.

## What Changes

- Allow component call sites to bind emitted actions through `on<ActionName>` properties such as
  `onSearchSubmitted=<DoSearch search={action.searchString} />`.
- Treat action handler values as callback expressions with an implicit `action` parameter bound to
  the emitted action payload.
- Define every inline emitted action as a public component-scoped action type named
  `<Component>.<Action>` so it can be referenced elsewhere in NX code.
- Lower component action handlers into explicit runtime metadata instead of leaving them as ordinary
  element props.
- Add interpreter support for invoking a lowered handler and requiring it to produce one or more
  actions.
- Add validation, examples, and tests for declared emits, matching `on...` handlers, and handler
  result shapes.
- Defer full component initialization, rendering, and dispatch orchestration to the follow-up
  component runtime change.

## Capabilities

### New Capabilities
- `component-action-handlers`: Lower and execute component action handlers bound through
  `on<ActionName>` properties.

### Modified Capabilities
- `action-records`: Make inline emitted actions available as public action record types through
  qualified names such as `SearchBox.ValueChanged`.
- `component-syntax`: Define `on<ActionName>` properties on component invocations as emitted action
  handler bindings rather than ordinary props, and treat inline emitted action definitions as public
  `Component.Action` types.

## Impact

- `crates/nx-hir` item and element lowering for component invocations and handler metadata
- action/type resolution for public component-scoped emitted action names
- `crates/nx-interpreter` runtime evaluation for handler callbacks and emitted action results
- Component examples, diagnostics, and tests that cover handler binding and invocation semantics
