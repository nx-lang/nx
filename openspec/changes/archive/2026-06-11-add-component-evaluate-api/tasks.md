## 1. Interpreter Core

- [x] 1.1 Add an interpreter result type and public method for evaluating a component with explicit props and explicit state.
- [x] 1.2 Factor shared component lookup, effective contract resolution, prop binding, state binding, and body evaluation so initialization and evaluation reuse the same normalization rules where appropriate.
- [x] 1.3 Implement strict explicit-state validation for evaluation, including missing required state fields, unknown state fields, type coercion, enum values, and nullable fields.
- [x] 1.4 Ensure evaluation rejects abstract components, handles stateless components with empty state, and preserves existing external component behavior.
- [x] 1.5 Add interpreter tests for state-driven rendering, stateless rendering, invalid state input, abstract component rejection, and action-handler non-invocation.

## 2. API Layer

- [x] 2.1 Add `nx-api` component evaluation result and program-artifact evaluation function that converts host `NxValue` props/state into interpreter values and rendered output back to `NxValue`.
- [x] 2.2 Add source convenience evaluation helper that runs shared source analysis, builds a transient `ProgramArtifact`, and fails with static diagnostics before execution.
- [x] 2.3 Add API tests for artifact evaluation, source diagnostics, imported-library resolution, JSON-compatible rendered values, enum coercion, and polymorphic record output.

## 3. Native FFI

- [x] 3.1 Add an artifact-first C ABI entry point for component evaluation with component name, props bytes, state bytes, and selected output format.
- [x] 3.2 Serialize successful evaluation output as the rendered value directly in MessagePack or JSON, without lifecycle wrapper fields.
- [x] 3.3 Return diagnostics through the existing error payload path for static, input, and runtime failures.
- [x] 3.4 Update generated FFI header coverage and add smoke tests for MessagePack output, JSON output, invalid state diagnostics, and imported-library artifact reuse.

## 4. Managed .NET Binding

- [x] 4.1 Add raw-byte `NxRuntime.EvaluateComponentBytes` overloads for source, source with `NxProgramBuildContext`, and `NxProgramArtifact`.
- [x] 4.2 Add typed `NxRuntime.EvaluateComponent<TProps, TState, TElement>` overloads for source, source with `NxProgramBuildContext`, and `NxProgramArtifact`.
- [x] 4.3 Add `NxRuntime.EvaluateComponentJson` overloads that return the rendered body as `JsonElement` directly.
- [x] 4.4 Ensure source overloads build transient `NxProgramArtifact`s and then call the native artifact-first evaluation entry point.
- [x] 4.5 Add managed tests for typed rendering, JSON rendering without wrapper fields, raw-byte format selection, invalid state diagnostics, imported-library build contexts, enum DTO mapping, and polymorphic DTO mapping.

## 5. Documentation And Verification

- [x] 5.1 Update the .NET binding README with `EvaluateComponent` examples and clarify when to use evaluation versus initialization/dispatch.
- [x] 5.2 Update relevant rustdoc/API comments to document pure component evaluation, explicit state ownership, and direct rendered-value output.
- [x] 5.3 Run Rust component/API/FFI tests covering the new evaluation path.
- [x] 5.4 Run managed .NET binding tests covering the new evaluation APIs.
- [x] 5.5 Run OpenSpec verification for `add-component-evaluate-api`.
