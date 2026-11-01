# Migrating from HIR execution

NX source is analyzed and compiled to NX IR, then executed by an IR runtime. `nxlang run file.nx`
keeps its syntax, `--format nx|json` and `--output` behavior. It emits and links the entry and its
required modules. Direct `.nxir` input remains future work.

## Removed execution APIs

| Surface | Removed |
| --- | --- |
| Rust `nx-api` | `eval_source`, `eval_program_artifact`, `eval_program_artifact_function`, `eval_program_artifact_nx_text`, `eval_program_artifact_nx_text_with_limits`, `EvalResult` |
| Rust components | `initialize_component_source`, `evaluate_component_source`, `dispatch_component_actions_source`, their `*_program_artifact` counterparts and init/evaluate/dispatch result wrappers |
| Rust values | `ResourceLimits`, `to_nx_value`, `from_nx_value`, `entry_result_to_nx_value`, `FromNxValueError`, interpreter-Value `format_nx_text` |
| C | `nx_eval_source`, `nx_eval_program_artifact`, `nx_component_init_program_artifact`, `nx_component_evaluate_program_artifact`, `nx_component_dispatch_actions_program_artifact` |
| .NET | All `NxRuntime.Evaluate*`, `InitializeComponent*`, `EvaluateComponent*`, `DispatchComponentActions*` overloads, execution-only output/result models |
| Node | Artifact `evaluateJson`, `evaluateBytes`, `evaluateJsonFromSource`, `evaluateBytesFromSource`, execution-only options |
| WASM | Artifact `evaluateNx`, native `nx_wasm_program_evaluate_nx` |

Program/library/workspace construction, analysis, validation, diagnostics, schemas, codegen,
language services, annotated formatting, DTO serialization and update helpers remain.
Native C ABI is 14; WASM ABI is 12. Rebuild wrappers and native modules together.

## Compile and execute

Rust callers compose `nx_api::codegen` and `nx_api::ir_runtime`; the complete example is in
[`nx-api`](../crates/nx-api/README.md). Runtime-only Rust callers use `nx-ir-runtime` directly.
Node and WASM callers use `@nx-lang/ir-runtime`:

```ts
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";

// artifact was built by either SDK; provide the selected entry's logical identity.
const modules = new Map(artifact.generateNxIr({ modules: [] }).map(image =>
  [image.identity, prepareNxIrModule(image.bytes)]));
const program = linkNxIrProgram(modules.get("main.nx"), {
  resolve: identity => modules.get(identity)
});
const value = evaluateFunction(program, "root");
// With the WASM SDK:
const annotated = artifact.formatValue(value);
```

Pass `formatValue(value, { type: "float64" })` when the root's declared type is needed to recover
number spelling. The canonical host representation erases constant-case identities at object
sites; the Rust CLI uses a read-only rich result view to preserve them. Runtime float32 values use
a rounded float64 carrier; declared float32 sites print float32 spelling, while an object site
prints its float64 widening.

**C and .NET have no replacement execution binding in this change.** They retain compilation and
DTO facilities. Hosts needing execution must use an existing Rust/TypeScript IR runtime or request
a separate runtime binding.

## Components, diagnostics and numeric behavior

Use existing IR component initialization, pure evaluation, dispatch and instance-storage APIs.
Legacy opaque snapshot envelopes are removed, with no conversion or replacement transport.
IR instances retain their existing image-fingerprint validation. Parent-bound handlers travel via
parent-instance tokens rather than captured handler bodies supplied as host values.

CLI runtime failures carry `nx-ir-*` codes and debug source locations where available; without
debug/source information they name the declaration and omit invented coordinates. Each CLI run
has a fresh 1,000,000-operation budget. The operation unit is IR work; it is not the old HIR unit.
The CLI also uses the runtime defaults of 100 nested calls and a 1 MiB evaluation stack budget;
recursive programs can reach either limit earlier than with the retired interpreter. Runtime
library defaults and playground limits are unchanged.

The old int32 arithmetic wrap is removed: `2000000000 + 2000000000` with int32 operands, returned as `int`, yields
`4000000000` on the existing IR path. Rust still uses wrapping i64 arithmetic; JavaScript numbers
have their existing precision limits beyond 2^53. This change does not promise universal integer
parity or checked arithmetic.

When a float64-typed conditional or presence join selects an integer arm, the existing IR carrier
can remain an integer: JSON and NX at an `object` site print `1` rather than the old `1.0`. The
numeric value is unchanged. Corpus conversion/occurrence goldens explicitly record this difference.

## Temporary source limitations

Conditional property fragments and positional record constructors are deferred to
`complete-ir-construction-lowering`. For now use ordinary properties whose values are conditional,
and markup record construction, such as `<User name="Bob" age=42 />`, respectively. Their original
success assertions remain as ignored source-to-IR regression tests naming the follow-up change;
run them explicitly while implementing it and remove the ignores as they pass. Existing ignored
parser tests remain separately labeled.
