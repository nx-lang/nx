## Context

Executable TS/JS codegen currently supports eager, non-reactive NX programs that produce
serializable values, but rejects component declarations and action-handler expressions. NX's
runtime model already treats component initialization/evaluation as an explicit entry operation:
the selected component's body is evaluated with normalized props and state, while component element
expressions elsewhere produce component-shaped values. ReachMe's server-driven UI flow needs that
same distinction in generated TypeScript/JavaScript so a server can evaluate a parent component and
return child component descriptors for a client to display.

The generated backend should stay host-neutral. It should not know how a browser, React app, or
native client displays `SearchBox`; it should only produce deterministic JSON-compatible component
descriptors and provide entry APIs for evaluating named concrete components.

## Goals / Non-Goals

**Goals:**

- Generate executable TypeScript and JavaScript for concrete NX component declarations.
- Preserve the semantic distinction between component descriptors and component body evaluation.
- Emit component descriptor functions plus schema entry objects with JSON-friendly initialization
  and evaluation APIs.
- Emit state types and helper functions for concrete components that declare state, including JSON
  normalization at schema boundaries.
- Preserve component inheritance for props, defaults, content, and generated contracts where
  TypeScript types can represent the NX inheritance graph.
- Keep component execution eager, deterministic, and non-reactive in this phase.
- Reject action-handler bindings in executable TS/JS codegen with actionable diagnostics.

**Non-Goals:**

- Add generated dispatch APIs, state update reducers, effects, subscriptions, invalidation, or
  reactive dependency tracking.
- Generate React/Vue/Svelte/native adapters or otherwise display component descriptors.
- Execute a child component's implementation body when a parent body constructs `<Child />`.
- Change the `nxlang typegen` DTO-only surface.
- Change the Rust/native/.NET component runtime APIs in this proposal.
- Support action-handler invocation or action effect generation in TS/JS codegen.

## Decisions

### 1. Component expressions construct descriptors, not rendered child bodies

When generated code evaluates `<Component ... />`, it should normalize the target component's
effective props, apply defaults, bind content, and return a record-like descriptor:

```json
{ "$type": "SearchBox", "placeholder": "Docs" }
```

It must not evaluate `SearchBox`'s component body as a side effect of constructing that descriptor.
Only an explicit component entry API call evaluates a component body. This keeps server-driven UI
handoff natural: evaluating a parent component can return a tree/list of child component
descriptors that the caller displays.

Alternatives considered:

- Recursively evaluate child component bodies. Rejected because it removes the host/client's chance
  to own rendering and lifecycle for child components.
- Restrict component expressions to component bodies only. Rejected because functions that return
  component descriptors are useful composition tools.

### 2. Function element calls continue to evaluate immediately

Element syntax that resolves to a function remains call syntax. `<MyFunction ... />` should
evaluate the function and substitute its returned value into the surrounding expression. If that
function returns `<SearchBox />`, the surrounding expression receives the `SearchBox` descriptor.

This keeps the existing function/component distinction explicit:

- functions are executable helpers;
- components are displayable UI atoms unless selected as an entry component.

### 3. Generated schemas represent component entry points

Generated component functions construct descriptor values. They normalize props/defaults and return
plain JSON-compatible component records without evaluating the component body. Generated schema
objects are the public executable surface for selecting a component and evaluating its body. The
schema provides host-facing operations such as:

- `initializeJson(props)` -> `{ rendered, state }`
- `evaluateJson(props, state?)` -> `rendered`
- `tryInitializeJson(props)` -> result object instead of throwing
- `tryEvaluateJson(props, state?)` -> result object instead of throwing

The `Json` suffix is used because these APIs are the host boundary that accepts JSON-compatible
props and state. Typed generated helpers such as `SearchBox(props)`, `initialSearchBoxState`, and
`renderSearchBox` remain available for generated-module callers that already have typed values.

In TypeScript, abstract NX components should emit type contracts only when needed for inherited
props. Concrete components should emit descriptor functions and schema objects. In JavaScript, the
same value surface should be emitted without TypeScript-only syntax.

### 4. State is represented by generated types and helpers for concrete components only

Component state should be represented by a generated TypeScript state type such as
`SearchBoxState`, plus helper functions such as `initialSearchBoxState` and `renderSearchBox` where
state exists. This gives hosts a stable JSON-compatible object to swap, validate, and pass back into
evaluation.

State is not inherited. Abstract components are contract-only and do not declare or emit state
helpers. Concrete components with state should emit:

- initial-state materialization from component props and state defaults;
- schema-boundary normalization from JSON-compatible input.

If `evaluate` receives no state, it should materialize initial state from props. If required state
cannot be materialized because a non-nullable state field lacks both input and default, the
generated API should fail through the normal generated error path.

### 5. Use throwing APIs plus result-returning wrappers

The primary generated APIs should throw `NxRuntimeError` for invalid props, invalid state, missing
required fields, unsupported generated constructs, or other runtime failures. This matches common
JavaScript practice for imperative APIs and keeps normal application code straightforward.

Generated `try*` wrappers should catch `NxRuntimeError` and return a discriminated result object:

```ts
type NxResult<T> =
  | { ok: true; value: T }
  | { ok: false; diagnostics: readonly NxDiagnostic[] };
```

This gives HTTP/RPC boundary code a convenient no-throw path without forcing every consumer into
manual result plumbing.

### 6. Action handlers are diagnostic-only in this phase

Component action-handler bindings currently lower as lazy callbacks in the interpreter. Generated
TS/JS should not silently drop or serialize them in this change. If executable codegen sees an
action-handler binding or a component descriptor that would require preserving a handler callback,
it should return a diagnostic and emit no executable output.

This protects behavior until generated dispatch/effect semantics are designed.

## Risks / Trade-offs

- [Descriptor shape is confused with ordinary records] -> Keep the wire shape record-like for
  compatibility, but preserve component metadata in the codegen model so normalization and typing
  can distinguish component descriptors from data records.
- [Component inheritance does not map cleanly for every imported base] -> Preserve inherited
  contracts through generated normalization and allow an internal fallback when needed, with tests
  proving the generated behavior remains equivalent.
- [State omission hides missing required state] -> Only omit state by materializing initial state
  through the same default rules as initialization; missing required state remains an error.
- [Generated JSON validation becomes inconsistent with interpreter coercion] -> Reuse the same type
  metadata already preserved in `CodegenProgram` and add parity tests for props, state, enums,
  nullable fields, lists, and defaults.
- [Action handlers are needed sooner than expected] -> Keep diagnostics explicit and scope a later
  dispatch/effects change once handler serialization and invocation semantics are agreed.

## Migration Plan

1. Extend `CodegenProgram` with component contract, state, body, and descriptor metadata while
   preserving current non-component codegen behavior.
2. Add component descriptor emission for component element expressions and keep function element
   calls eager.
3. Add generated component descriptor functions, state helpers, and schema JSON
   initialize/evaluate APIs.
4. Add diagnostics for action-handler bindings and unsupported component constructs.
5. Add parity and generated-output tests for TS/JS targets and CLI codegen.

Rollback is limited to removing the new component codegen support and returning components to the
existing unsupported-construct diagnostic path.

## Open Questions

- Should generated TypeScript expose a named component descriptor type for each component, or keep
  descriptor output structurally typed until a later host-adapter change needs stronger types?
