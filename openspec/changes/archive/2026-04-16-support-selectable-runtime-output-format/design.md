## Context

The current native runtime and managed .NET binding treat MessagePack as the only real host output
format. `crates/nx-ffi` always serializes successful values and diagnostics as MessagePack, then
exposes separate `*msgpack_to_json` helper exports for debug conversion. `bindings/dotnet` mirrors
that shape with `EvaluateBytes`, `InitializeComponentBytes`, and `DispatchComponentActionsBytes`
returning canonical MessagePack bytes plus helper APIs such as `ValueBytesToJson`.

That split no longer matches the product need. Some hosts, especially C#, need JSON as the primary
output so the returned payload can be forwarded directly to a client. At the same time, this change
must stay narrow:

- host inputs remain MessagePack or raw bytes; JSON input is explicitly out of scope
- component state snapshots remain opaque binary data owned by the host
- C# JSON consumption should use `JsonElement` rather than introducing a second generic object model

The good news is that most of the necessary primitives already exist. `nx-value` can serialize
`NxValue` directly to JSON, the component JSON helper structs already define the JSON shape for
render/effects plus base64 `state_snapshot`, and the managed model types already carry
`System.Text.Json` metadata.

## Goals / Non-Goals

**Goals:**
- Let native runtime evaluation and component lifecycle calls return either MessagePack or JSON on a
  per-call basis.
- Keep MessagePack as the default/raw binary format for callers that want compact runtime payloads.
- Remove the public post-processing MessagePack-to-JSON helper APIs in both Rust FFI and .NET.
- Add direct JSON workflows in the managed binding, with `JsonElement` used for returned NX JSON
  values in C#.
- Preserve the existing MessagePack input path for props, action batches, diagnostics, and opaque
  state snapshots.

**Non-Goals:**
- Accept JSON props, JSON action lists, or JSON component snapshots as host input.
- Redesign the component snapshot payload or make it human-readable beyond the existing base64 JSON
  projection.
- Replace the existing typed MessagePack generic APIs in .NET with a serializer-agnostic abstraction.
- Extend this change to program-build or library-loading APIs unless they already participate in the
  same runtime result flow.

## Decisions

### 1. Add a first-class output-format enum to value-returning runtime entry points

The native FFI should introduce an explicit output-format enum (for example, `NxOutputFormat`) and
accept it on the value-returning runtime entry points:

- `nx_eval_source`
- `nx_eval_program_artifact`
- `nx_component_init_program_artifact`
- `nx_component_dispatch_actions_program_artifact`

This keeps format choice on the original call, which is what the user asked for, and avoids a
parallel API matrix of `*_json` entry points.

Alternative considered: add separate JSON-specific entry points while keeping the current
MessagePack signatures unchanged. Rejected because it duplicates ABI surface, repeats tests and
documentation, and still leaves the debug-converter APIs conceptually alive.

### 2. Serialize JSON directly instead of converting from an intermediate MessagePack payload

The runtime should branch on the requested output format before writing the response buffer:

- success values serialize as MessagePack or JSON directly
- diagnostics serialize as MessagePack or JSON directly
- component init/dispatch JSON reuse the existing JSON object shape, including base64
  `state_snapshot`

That means the `nx_value_msgpack_to_json`, `nx_diagnostics_msgpack_to_json`,
`nx_component_init_result_msgpack_to_json`, and
`nx_component_dispatch_result_msgpack_to_json` exports can be removed instead of being repurposed.

Alternative considered: keep the current MessagePack-first implementation and internally call the
existing converter helpers when JSON is requested. Rejected because it preserves double work,
retains the old mental model, and keeps converter-specific failure modes alive in production code.

### 3. Keep inputs MessagePack-only and snapshots binary, even when outputs are JSON

Output format selection changes only the returned payload. It does not change how callers provide
NX values back to the runtime:

- evaluation still takes source text or a `ProgramArtifact`
- component init still accepts MessagePack props
- component dispatch still accepts raw snapshot bytes plus MessagePack action lists
- JSON component results continue to encode `state_snapshot` as base64 so the next call can decode
  it back to the same opaque bytes

This keeps the change small and prevents accidental introduction of a partially supported JSON input
story.

Alternative considered: pair JSON output with optional JSON input for props, actions, or snapshots.
Rejected because the user explicitly ruled out JSON input for now, and mixing both concerns would
expand validation, diagnostics, and compatibility work across every host API.

### 4. Split the managed API into explicit MessagePack and explicit JSON workflows

The .NET binding should keep the current MessagePack-centric generic APIs intact and add explicit
JSON workflows instead of making every generic API infer or abstract over both serializers.

Managed shape:

- raw byte methods accept an output-format argument and return the selected payload bytes
- existing generic `Evaluate<T>`, `InitializeComponent<T...>`, and `DispatchComponentActions<T...>`
  remain MessagePack-based
- new JSON convenience methods return `JsonElement`,
  `NxComponentInitResult<JsonElement>`, and `NxComponentDispatchResult<JsonElement>`
- managed exceptions parse diagnostics from the selected output format so failure behavior remains
  consistent

This gives C# callers two clean paths: compact MessagePack for typed in-process use, or direct JSON
for pass-through scenarios.

Alternative considered: add a format argument to the generic methods and deserialize with
MessagePack or `System.Text.Json` depending on the enum. Rejected because it produces confusing
serializer semantics for the same method name and makes generic type support difficult to reason
about, especially once `JsonElement` enters the picture.

### 5. Treat this as an ABI and public API cleanup, not a compatibility-preserving extension

Because the repository does not need backward-compatibility preservation yet, the simpler design is
to change the existing FFI signatures, bump the ABI version, regenerate the C header, update the
managed DllImports, and remove the old debug-converter APIs in the same change.

Alternative considered: keep the old signatures and helper methods for a deprecation window.
Rejected because it adds churn without user value and complicates documentation and tests during a
phase where simpler new code is preferred.

## Risks / Trade-offs

- JSON payloads are larger and slower than MessagePack -> keep MessagePack as the default and make
  JSON an explicit opt-in per call.
- `JsonElement` has document-lifetime semantics -> parse JSON with `JsonDocument`, clone the root
  element, and dispose the document before returning.
- Base64 `state_snapshot` is less ergonomic than a plain object -> document that snapshots are
  intentionally opaque and preserve exact bytes across round-trips.
- Removing helper APIs is a breaking managed/native change -> update tests, README examples, header
  generation, and spec documentation together so the new path is obvious.

## Migration Plan

1. Add the output-format enum and response-serialization branching in `crates/nx-ffi`, remove the
   converter exports, bump `NX_FFI_ABI_VERSION`, and regenerate the public header.
2. Update `bindings/dotnet` interop signatures and `NxRuntime` helpers so raw byte methods can
   request JSON directly, JSON convenience methods deserialize to `JsonElement`, and failures parse
   diagnostics from either MessagePack or JSON.
3. Replace converter-based tests with direct JSON-request tests across Rust FFI, .NET runtime
   tests, and README samples.

Rollback is straightforward because the change is isolated to the runtime/binding surface. Revert
the enum/signature changes, restore the converter helpers, and revert the managed JSON entry points
as one unit.

## Open Questions

- Do we want the same output-format selection on non-runtime APIs such as program-artifact build and
  library loading, or is runtime evaluation/lifecycle coverage sufficient for now?
- When JSON input is revisited later, should it use the same enum family or a separate input-format
  contract to keep request and response concerns independent?
