## 1. HIR Component Groundwork

- [x] 1.1 Add a lightweight HIR component item plus a component-signature prepass that preserves declared props and emitted actions and synthesizes public `Component.Action` record items for inline emits
- [x] 1.2 Detect matching `on<ActionName>` properties on component invocations, lower them to a dedicated lazy action-handler expression, and bind the implicit `action` name while lowering the handler body as the appropriate shared or `Component.Action` action type
- [x] 1.3 Add lowering diagnostics and tests for unknown handler bindings, prop/handler name collisions, shared emits, public inline emitted action names, and ordinary `on...` props that are not handlers

## 2. Interpreter Handler Runtime

- [x] 2.1 Add a runtime action-handler value plus execution-context capture support so handler bodies remain lazy when a component invocation is evaluated
- [x] 2.2 Add an interpreter API for invoking a lowered action handler with an emitted action input and an implicit `action` binding
- [x] 2.3 Normalize handler results to non-empty action lists and add runtime validation for shared action inputs, synthesized `Component.Action` inline emit records, and invalid empty or non-action results

## 3. Coverage and Documentation

- [x] 3.1 Add interpreter and HIR test coverage for captured variables, shared and inline emitted actions, public `Component.Action` reuse in other NX code, and single-action versus multi-action handler results
- [x] 3.2 Update component examples and documentation to show `on<ActionName>` handler bindings, public `Component.Action` names for inline emits, and clarify that full component init/render/dispatch support remains a follow-up change
