## Why

NX components currently must be fully concrete declarations with an inline NX body, which makes it
impossible to define reusable component contracts or describe UI components implemented outside NX.
Adding component inheritance and external component declarations lets libraries share prop and emit
contracts without duplicating them and gives hosts a first-class way to bind NX component APIs to
non-NX UI implementations.

## What Changes

- Extend component declarations to support the `abstract` and `external` modifiers plus a single
  `extends` clause on component signatures.
- Allow abstract and external component declarations to omit `= { ... }` and define only their
  public API: props, defaults, and emitted actions.
- Add component contract inheritance so derived components inherit all props and emitted actions
  from their parent component and may add new ones.
- Validate that component inheritance applies only to component contracts, not implementation
  details: abstract components cannot declare state or bodies, and derived components do not
  inherit state from their base component.
- Allow `abstract external component` declarations so hosts can publish external component APIs that
  also participate in inheritance hierarchies.

## Capabilities

### New Capabilities
- `component-contract-inheritance`: Define component-to-component inheritance for public props and
  emitted actions, including validation of inherited APIs and duplicate declarations.
- `external-components`: Define bodyless component declarations that model externally implemented UI
  components through NX-visible props and emits contracts.

### Modified Capabilities
- `component-syntax`: Expand component declaration syntax to accept `abstract`, `external`, and
  `extends`, and to allow bodyless declarations when the component is abstract or external.

## Impact

- Affected specs for component parsing, validation, and public component contract modeling.
- Affected code in the syntax grammar, parser/lowering pipeline, semantic analysis, and component
  diagnostics.
- Affected tests for valid and invalid component declarations, inheritance behavior, and emitted
  action visibility on derived and external components.
