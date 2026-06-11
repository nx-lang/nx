## Why

Hosts need a pure way to render NX components from explicit inputs without starting a stateful
component lifecycle. Dynamic survey and UI runtimes can then keep domain state outside NX, update it
after user input, and ask NX to re-evaluate the component body against the current props and state.

## What Changes

- Add a public component evaluation runtime operation that takes a `ProgramArtifact`, component name,
  prop record, and explicit state record, then returns the rendered component body.
- Reuse existing component prop/state normalization semantics so defaults, type coercion, imported
  component definitions, enum values, and polymorphic records behave consistently with component
  initialization.
- Keep this API side-effect-free: it does not dispatch actions, invoke handlers, update state,
  return effects, or produce an opaque lifecycle snapshot.
- Add native/FFI and managed .NET binding support, including MessagePack and JSON result formats and
  source convenience overloads implemented through transient `ProgramArtifact`s.
- Preserve existing `InitializeComponent` and `DispatchComponentActions` behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `component-runtime-bindings`: Add the component evaluation operation and its raw runtime result
  contract.
- `dotnet-binding`: Add managed APIs for component evaluation over source and `NxProgramArtifact`
  workflows.

## Impact

- Affects `nx-interpreter`, `nx-api`, `nx-ffi`, and `bindings/dotnet`.
- Adds runtime, FFI, managed binding, and documentation/test coverage for component evaluation.
- Does not require parser or syntax changes.
- Does not change existing action dispatch, state snapshot, or effect-handler contracts.
