## Why

External components are currently all-or-nothing bodyless contracts. That makes it impossible to
declare host-managed component state in NX, even when hosts need a typed state shape for wire
transfer and generated bindings.

## What Changes

- Allow concrete external component declarations to remain bodyless or to provide an optional
  state-only body.
- Require any external component body that is present to contain a `state { ... }` group and
  nothing else; reject empty external bodies and reject any rendered expression or other body
  content on external components.
- Preserve declared external-component state as part of the component contract exposed to hosts,
  while keeping that state host-managed rather than NX-evaluated behavior.
- Extend code generation so exported external components with declared state emit generated
  `<ComponentName>_state` data types for wire transfer, warning and skipping the synthesized
  companion when it would collide with an explicit exported declaration.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `component-syntax`: Allow concrete external component declarations to omit the body or to use a
  body that contains only `state { ... }`, and reject any other external-component body shape.
- `external-components`: Allow concrete external components to declare host-managed state without
  gaining an NX render body, and include that declared state in their external contract.
- `cli-code-generation`: Generate host-facing state data types for exported external components
  when they declare external state.

## Impact

- Affected specs for component parsing, external-component validation, and generated host type
  surfaces.
- Affected code in the syntax grammar, parser diagnostics, HIR/lowering for external component
  bodies, external component contract handling, and code generation.
- Affected tests for valid and invalid external component declarations, declared-state behavior,
  and generated TypeScript/C# output for exported external component state.
