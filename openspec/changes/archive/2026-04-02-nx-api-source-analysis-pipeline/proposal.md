## Why

`nx-api` source-driven runtime helpers currently stop after parse/lower and either return only
lowering diagnostics or proceed straight to interpretation. That diverges from
`nx-types::check_str`, which continues through scope building and type checking, so API callers can
miss downstream static errors in the same source file.

## What Changes

- Add a shared source-analysis entry point at the analysis boundary that performs parse, lowering,
  scope building, and type checking in one pass.
- Reuse that shared analysis result from `nx-api` source-driven entry points such as
  `eval_source`, `initialize_component_source`, and `dispatch_component_actions_source`.
- Return the full set of static analysis diagnostics from source-driven runtime entry points before
  any interpreter execution, instead of short-circuiting on the first lowering error.
- Preserve the lowered `Module` in the shared analysis result so runtime entry points can interpret
  without reparsing or relowering the same source.
- Preserve caller-provided `file_name` and source spans across lowering, scope, and type
  diagnostics returned through `nx-api`.
- Narrow or remove `lower_source_module` after the shared analysis path exists so its name again
  reflects true parse/lower behavior.

## Capabilities

### New Capabilities
- `source-analysis-pipeline`: Define the shared static-analysis contract for source-driven runtime
  entry points, including full diagnostic aggregation and reuse of the lowered module.

### Modified Capabilities
- `component-runtime-bindings`: Source-based component initialization and dispatch return complete
  static diagnostics and do not enter runtime execution when analysis reports errors.

## Impact

- `crates/nx-types` analysis entry points and diagnostic construction
- `crates/nx-api` source-driven evaluation/component helpers and any helper layering around
  `lower_source_module`
- Runtime-facing bindings in `crates/nx-ffi` and `bindings/dotnet` that surface `nx-api`
  diagnostics
- Tests covering aggregated lowering/scope/type diagnostics and runtime short-circuit behavior
