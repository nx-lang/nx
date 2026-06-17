## Why

NX can already emit executable TypeScript and JavaScript for non-reactive data-oriented programs,
but component declarations and component invocations are still rejected by executable code
generation. ReachMe needs generated TS/JS to evaluate server-authored component entry points and
return component descriptor values that clients can display without requiring the Rust interpreter
at runtime.

## What Changes

- Extend executable TypeScript/JavaScript generation to model component declarations and component
  invocations.
- Emit generated TypeScript/JavaScript descriptor functions and schema entry objects for concrete
  components, with inherited props/defaults preserved in generated normalization.
- Emit generated component state types and helper functions only for concrete components that
  declare state; abstract components remain contract-only and do not receive state helpers.
- Treat `<Component ... />` expressions as atomic component descriptor construction: generated code
  normalizes props, content, and defaults and returns a record-like JSON value whose `$type` is the
  component name, but does not evaluate that component's implementation body.
- Preserve the existing function-call distinction: `<Function ... />` continues to evaluate the
  function and substitute its returned value into the surrounding expression.
- Add generated schema entry APIs for initialization and evaluation from JSON-compatible props and
  optional JSON-compatible state. Initialization returns rendered output plus initial state;
  evaluation returns rendered output using supplied state or initial state when state is omitted.
- Keep generated component execution eager and non-reactive for this change. State updates,
  dispatch, action-handler invocation, and effects remain host-owned or deferred.
- Reject component action-handler bindings in executable TS/JS codegen with diagnostics rather than
  silently dropping them.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `executable-code-generation`: Add TypeScript/JavaScript component declaration and component
  invocation generation, generated component descriptor functions, schema JSON
  initialize/evaluate APIs, and diagnostics for unsupported action-handler bindings.

## Impact

- Updates `crates/nx-codegen` model construction and emitters to preserve component contracts,
  component state declarations, component bodies, and component-valued expressions.
- Updates generated TS/JS runtime helpers only as needed for component descriptor normalization,
  JSON input normalization, and typed runtime errors.
- Updates `nxlang codegen` behavior through the existing `nx-codegen` API without changing the
  `nxlang typegen` DTO-only surface.
- Adds parity and generated-output tests for component descriptors, component entry evaluation,
  inherited props/defaults, state initialization/evaluation, function-vs-component element
  behavior, and unsupported action handlers.
