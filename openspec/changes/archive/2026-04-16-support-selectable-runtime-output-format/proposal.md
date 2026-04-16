## Why

NX currently treats MessagePack as the only real host output format and treats JSON as a debug-only
conversion step layered on top of MessagePack bytes. That blocks production scenarios where a host,
especially C#, needs to receive NX results as JSON and immediately forward them to a client without
doing an extra conversion pass or carrying a MessagePack dependency through that path.

## What Changes

- Add a first-class runtime output format option so hosts can request either MessagePack or JSON for
  evaluation and component lifecycle results.
- Keep host input formats unchanged in this change: props, actions, and other caller-supplied NX
  values remain MessagePack-only.
- Return JSON directly from native runtime entry points instead of generating MessagePack first and
  converting it afterward through separate helper APIs.
- Update the .NET binding to expose JSON-oriented result APIs for C# callers and represent returned
  NX JSON values with `JsonElement`.
- Remove debug-only MessagePack-to-JSON helper APIs such as `ValueBytesToJson`,
  `DiagnosticsBytesToJson`, `ComponentInitResultBytesToJson`, and
  `ComponentDispatchResultBytesToJson`. **BREAKING**

## Capabilities

### New Capabilities
- `runtime-output-format`: Allow native runtime entry points to produce either MessagePack or JSON
  result payloads, while keeping NX input payloads MessagePack-only in this phase.

### Modified Capabilities
- `component-runtime-bindings`: Component initialization and dispatch results can be returned
  directly in the caller-selected output format instead of only as canonical MessagePack payloads.
- `dotnet-binding`: The managed API adds explicit JSON result support for C# callers, uses
  `JsonElement` for returned NX JSON values, and removes the debug conversion helpers.

## Impact

- Affected native ABI and Rust FFI implementation in `crates/nx-ffi`.
- Affected managed API surface and documentation in `bindings/dotnet`.
- Affected runtime and binding tests that currently assume debug-only MessagePack-to-JSON
  conversion helpers.
