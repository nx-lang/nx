## Context

NX already supports component initialization and component action dispatch through the interpreter,
`nx-api`, native FFI, and the managed .NET binding. Initialization renders a component body and
returns an opaque host-owned state snapshot. Dispatch consumes that snapshot and actions, invokes
bound handlers for effects, and returns a next snapshot.

That lifecycle shape is useful for NX-managed component instances, but some hosts need a simpler
pure render operation: pass current props and current state explicitly, evaluate the component body,
and receive the rendered value. Dynamic question-flow runtimes are one use case: the server can
persist answers and cursor state, update them after a user submission, then ask NX to render the
current flow shape from those transparent inputs.

The existing implementation already has most of the required internal machinery:

- component lookup through `ProgramArtifact` / `ResolvedProgram`
- effective component prop contracts, including inherited props and handler-name validation
- prop and state field normalization/coercion helpers
- component body evaluation that produces a `Value`
- raw `NxValue`, JSON, MessagePack, FFI, and managed binding output paths

## Goals / Non-Goals

**Goals:**

- Add an artifact-first component evaluation operation:
  `component + props + current state -> rendered value`.
- Keep evaluation pure and side-effect-free from the runtime API perspective.
- Reuse existing component prop/state coercion and rendered-value serialization rules.
- Support source convenience APIs by building transient program artifacts above the native layer.
- Expose managed typed, raw-byte, and `JsonElement` workflows.
- Keep existing initialization and dispatch APIs unchanged.

**Non-Goals:**

- Add NX language syntax for reducers, state-update actions, hooks, or effects.
- Change `DispatchComponentActions` to return rendered output.
- Make opaque lifecycle snapshots transparent or replace the snapshot lifecycle APIs.
- Generate state companion DTOs for concrete non-external components.
- Integrate this API into ReachMe question flow runtime in this change.

## Decisions

### 1. Add a pure evaluate operation instead of extending dispatch

The new runtime operation should not depend on an opaque snapshot or action batch. Its contract is
the direct render function hosts need:

```text
EvaluateComponent(programArtifact, componentName, props, state, outputFormat) -> rendered
```

This avoids overloading dispatch with a mode where no actions are dispatched and no effects are
expected. It also preserves the current dispatch contract, which intentionally focuses on effects
and snapshot round-tripping.

Alternative considered: change `DispatchComponentActions` to also return the rendered component.
Rejected for this change because it broadens the lifecycle API and still leaves hosts without a
clean way to render from transparent server-owned state.

### 2. Reuse component initialization normalization, but do not create snapshots

The interpreter should share component lookup, effective contract resolution, prop coercion, state
coercion, and body evaluation logic with initialization. Evaluation should bind props first, then
bind state fields from the supplied state record, then evaluate the body in a context containing
both sets of variables.

Evaluation should not call snapshot encode/decode helpers. It should return only the rendered value.

For state input, the first implementation should treat the supplied state as the current state and
validate it against the component's declared state fields. Missing required state fields should be
diagnostics/errors rather than implicit runtime state. This keeps ownership clear: the host is
responsible for state mutation and for passing the current value on each evaluation call.

Alternative considered: reuse initialization's state default materialization for missing state
fields. That is useful for lifecycle initialization but less clear for repeated pure evaluation,
where silently filling missing current-state fields could hide host persistence bugs.

### 3. Keep native component evaluation artifact-first

The native C ABI should expose program-artifact evaluation, mirroring the current artifact-first
direction for component lifecycle work:

```text
nx_component_evaluate_program_artifact(...)
```

Source-based convenience should live in `nx-api` and managed .NET helpers by building a transient
`ProgramArtifact`, then calling the artifact-first path. This keeps imported-library resolution
centralized in `ProgramBuildContext` and avoids adding source-resolution policy to the native
evaluate entry point.

Alternative considered: add a source-based C ABI evaluate entry point. Rejected because the current
managed source helpers already build transient artifacts, and adding another native source path would
increase API surface without improving the intended host workflow.

### 4. Return the rendered value directly

Unlike initialization and dispatch, evaluation does not produce lifecycle metadata. Raw MessagePack
and JSON outputs should therefore encode the rendered value directly, just like root evaluation,
rather than returning `{ rendered: ... }`.

The managed typed API can return `TElement`, and JSON convenience can return `JsonElement`.

Alternative considered: return an object wrapper with a `rendered` field for symmetry with
`InitializeComponent`. Rejected because it suggests future lifecycle metadata where this API
intentionally has none and makes pass-through JSON less ergonomic.

### 5. Preserve action-handler values as runtime-only host input restrictions

Evaluation accepts host-supplied props and state through the existing MessagePack input path. The
current raw-value input conversion intentionally rejects `ActionHandler` records because handlers
are runtime-only values produced by NX component invocation. This should remain true for
`EvaluateComponent`.

If hosts later need to evaluate components with handler props from outside NX, that should be a
separate design. This change is for data/state-driven rendering, not host-authored handler binding.

## Risks / Trade-offs

- State DTO ergonomics may be weaker for concrete components because generated companion state
  contracts currently focus on external components. → Hosts can use hand-written DTOs, anonymous
  records, dictionaries, or JSON workflows in this phase; generated concrete state contracts can be
  proposed separately if needed.
- Missing-state strictness may make initial render slightly more explicit. → This is preferable for
  server-owned state because it catches persistence bugs; hosts can still use props for context-only
  render inputs.
- Adding another component runtime path increases API surface. → The operation is intentionally
  narrow and shares implementation with initialization rather than introducing a new execution model.
- Source convenience helpers rebuild transient artifacts. → Artifact-first overloads are available
  for hot paths and can be reused by hosts that cache build results.

## Migration Plan

This is additive. Existing `Evaluate`, `InitializeComponent`, and `DispatchComponentActions`
callers should continue to behave the same way.

Implementation should proceed from the core outward:

1. Add interpreter support for evaluating a concrete component body with normalized props and
   explicit state.
2. Expose the operation through `nx-api` for `ProgramArtifact` and source convenience helpers.
3. Add native FFI artifact-first entry point and output-format serialization.
4. Add managed .NET raw-byte, typed, and JSON APIs.
5. Update README documentation and tests across interpreter, API, FFI, and managed binding layers.

Rollback is straightforward because the new API surface is additive: remove the evaluate entry
points and tests without changing existing lifecycle behavior.
