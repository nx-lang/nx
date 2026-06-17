## 1. Codegen Model And Diagnostics

- [x] 1.1 Extend `CodegenProgram` with component declarations, effective component contracts,
  component bodies, external/concrete/abstract flags, state fields, inherited props/defaults,
  emitted-action metadata, and component provenance.
- [x] 1.2 Build component model data from `ProgramArtifact` and resolved modules without
  rediscovering visible names during emission.
- [x] 1.3 Preserve component state default expressions and prop default expressions with owning
  module metadata so inherited defaults can be emitted from the correct module context.
- [x] 1.4 Add codegen diagnostics for unsupported component action-handler bindings, missing
  component semantic data, unsupported dispatch/effect constructs, and illegal direct evaluation
  of abstract components.

## 2. Component Descriptor Construction

- [x] 2.1 Update expression building so element expressions that resolve to concrete components
  become component descriptor construction nodes instead of unsupported constructs.
- [x] 2.2 Emit component descriptor construction as record-like JSON-compatible objects with `$type`,
  normalized props, content, inherited defaults, and deterministic field ordering.
- [x] 2.3 Ensure component descriptor construction does not evaluate the referenced component body,
  including when descriptors are nested inside another component body.
- [x] 2.4 Preserve function element-call behavior so `<Function ... />` still evaluates the function
  and substitutes its returned value.
- [x] 2.5 Support cross-module and imported-library component references using module-qualified
  references and deterministic imports.

## 3. Generated Component Functions, Schemas, And State

- [x] 3.1 Emit TypeScript and JavaScript component descriptor functions and schema entry objects for
  concrete components, plus abstract/base TypeScript contracts where needed for inherited component
  props.
- [x] 3.2 Preserve NX component inheritance in generated prop normalization and add deterministic
  normalization fallback behavior where direct inheritance is not represented in the generated
  surface.
- [x] 3.3 Emit state types and helper functions for concrete components that declare state,
  including initial-state materialization from props and JSON input normalization.
- [x] 3.4 Ensure abstract components do not emit state helpers or direct initialize/evaluate APIs.
- [x] 3.5 Emit schema `initializeJson` APIs that return rendered output plus JSON-compatible
  initial state.
- [x] 3.6 Emit schema `evaluateJson` APIs that evaluate with explicit state when supplied and use
  initial state when state is omitted.
- [x] 3.7 Emit JavaScript output equivalent to TypeScript output with all type-only syntax removed.

## 4. Runtime Helpers And Error Handling

- [x] 4.1 Add generated runtime helper support for component descriptor normalization and
  JSON-compatible prop/state validation only where shared helper code is simpler than inline
  emission.
- [x] 4.2 Add generated `NxRuntimeError`, diagnostic shape, and `NxResult<T>` TypeScript surface for
  component API failures.
- [x] 4.3 Emit `tryInitialize` and `tryEvaluate` wrappers that convert thrown
  `NxRuntimeError`s into result objects with diagnostics.
- [x] 4.4 Keep generated runtime helpers free of reactive signals, subscriptions, invalidation,
  dispatch, state-update reducers, effects, and action-handler invocation APIs.

## 5. Verification

- [x] 5.1 Add snapshot tests for generated TypeScript and JavaScript component descriptor functions,
  state helpers, descriptor construction, inherited props/defaults, and helper imports.
- [x] 5.2 Add interpreter parity tests proving component descriptors from generated JavaScript match
  interpreter output for external components, concrete components, inherited defaults, content, and
  cross-module component references.
- [x] 5.3 Add parity tests proving generated component entry initialization and evaluation match
  interpreter component runtime behavior for stateless components, state defaults, explicit state,
  omitted state, enums, nullable fields, and lists.
- [x] 5.4 Add tests proving child component descriptors inside parent component bodies are not
  deep-rendered by generated output.
- [x] 5.5 Add tests proving function element calls remain eager and can return component
  descriptors.
- [x] 5.6 Add negative tests proving action-handler bindings fail codegen with diagnostics and emit
  no executable output.
- [x] 5.7 Add CLI tests for `nxlang codegen` emitting component-capable TypeScript and JavaScript
  output from file and workspace inputs.
- [x] 5.8 Run targeted `cargo test -p nx-codegen`, `cargo test -p nx-cli`, and a TypeScript
  compiler smoke test when `tsc` is available.

## 6. Documentation

- [x] 6.1 Update README or CLI documentation to describe executable component codegen, component
  descriptors, generated component entry APIs, and unsupported action-handler/dispatch behavior.
- [x] 6.2 Document the distinction between `<Component ... />` descriptor construction and
  `<Function ... />` eager function calls for generated TS/JS output.
