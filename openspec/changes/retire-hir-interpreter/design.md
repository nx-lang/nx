## Context

`refactor-tooling-api` is merged. `nx-program` owns program construction, standard sources and
resolved types; codegen and language service depend on it. `nx-api` composes those services and
re-exports `nx-ir-runtime`. The extraction and dependency guard are already complete.

The remaining interpreter consumers are legacy `nx-api` evaluation/component/value code, adapters
in C/.NET/Node/WASM, `nxlang run`, and tests. ReachMe and FiddleEngine use IR execution. The
playground compiles to IR, executes the TypeScript runtime and calls `artifact.formatValue()`;
its tests explicitly reject calls to `artifact.evaluateNx()`. SDK exposure and test coverage alone
are not reasons to retain an unused public execution surface. These observations are a local
caller audit, not a claim to have enumerated every external package consumer.

## Goals / Non-Goals

**Goals:**

- One Rust evaluator, with explicit compilation followed by IR execution.
- Remove unused public execution APIs rather than migrate their wrappers to a different engine.
- Preserve CLI execution, tooling APIs, canonical-value formatting and language-behavior coverage.
- Preserve the existing standalone IR runtime contracts and acyclic crate layering.

**Non-Goals:**

- API compatibility for removed functions, interpreter snapshots, or interpreter-specific errors.
- New source execution convenience APIs, a prepared executable abstraction or execution caches.
- New .NET/C IR runtime bindings, LSP evaluation features or compile-time execution.
- New checked/exact integer semantics, IR schema changes or performance optimization.

## Decisions

### 1. Remove the legacy execution surface

Remove these flat Rust exports, their implementations and execution-only result/error types:

- `eval_source`, `eval_program_artifact`, `eval_program_artifact_function`,
  `eval_program_artifact_nx_text`, `eval_program_artifact_nx_text_with_limits`, `EvalResult`.
- `initialize_component_source`, `evaluate_component_source`,
  `dispatch_component_actions_source`, and their three `*_program_artifact` counterparts;
  the associated init/evaluate/dispatch result and eval-result wrappers.
- `ResourceLimits`, `to_nx_value`, `from_nx_value`, `entry_result_to_nx_value` and
  `FromNxValueError`, whose contracts depend on interpreter values.
- The interpreter-Value-based `format_nx_text` signature. Replace CLI formatting with an IR-value
  formatter; retain `format_program_artifact_value`, `FormatValueError`, `NxValueText`,
  `NxValueNode` and `NxValueRole` for canonical values and annotations.

Keep `load_program_artifact_from_source` and `load_library_artifact_from_directory` or equivalent
existing construction entry points: they analyze/build and do not execute. Their diagnostic gating
remains. Move surviving helpers out of execution-only modules as needed without another crate split.

Remove corresponding adapters end to end:

| Surface | Remove | Retain |
| --- | --- | --- |
| C FFI | `nx_eval_source`, `nx_eval_program_artifact`, `nx_component_init_program_artifact`, `nx_component_evaluate_program_artifact`, `nx_component_dispatch_actions_program_artifact` and execution-only handles/models | Build/validate/library/codegen/explain and their diagnostics |
| .NET | All `NxRuntime.Evaluate*`, `InitializeComponent*`, `EvaluateComponent*`, `DispatchComponentActions*` source/artifact overloads and execution-only result wrappers | Construction, validation, IR generation/explanation, generated DTOs and serialization/update helpers |
| Node | Artifact `evaluateJson`/`evaluateBytes`, `evaluateJsonFromSource`/`evaluateBytesFromSource`, execution-only options | Build/validate/library/schema/language/IR APIs |
| WASM | Artifact `evaluateNx` and native `nx_wasm_program_evaluate_nx` | Build/validate/library/schema/language/IR APIs and `formatValue` |

Do not remove shared diagnostic exceptions solely because their names mention evaluation: retain
ones still used by building or formatting, changing documentation to match their remaining use.
Refresh the generated C header and bump native/WASM ABI versions with wrappers in the same change.
Document the explicit breaking removals. There is no replacement .NET/C execution API in this scope.
Node/WASM callers execute emitted images with `@nx-lang/ir-runtime`; Rust callers use the existing
`nx_api::ir_runtime` namespace or `nx-ir-runtime` directly.

### 2. CLI execution composes existing services

`nxlang run` accepts NX source or NX IR with an explicit input-format choice; existing source-file
invocations retain their meaning. Use `--input-format nx|ir` (default `nx`) and repeatable
`--ir-module <identity>=<path>` catalog entries, documented and tested in CLI help. For source, preserve the existing import/build context behavior, reject
static errors, emit all required module images with debug information, prepare them and link the
selected entry. For IR, prepare the input image and resolve external modules by identity from an
explicit caller-supplied module catalog (with the runtime's existing prelude handling). A missing
module is a link diagnostic, not an implicit compile or filesystem search by untrusted identity.
Both paths execute the same linked-program API and share output, runtime options and errors.

The CLI owns per-invocation prepared modules and the linked program. No execution cache is attached
to `nx-program::ProgramArtifact`, so no invalidation, clone-sharing or cross-layer dependency is
introduced. Ordinary runtime callers can already retain prepared modules and linked programs.

Keep a CLI default of 1,000,000 IR operations per invocation, with exhaustion and reset tests.
Use existing runtime defaults for the other safety limits unless a measured CLI stack constraint
requires a lower bound; document any such adjustment. The operation unit changes from HIR work to
IR work, not an identical amount of computation. Leave library `RuntimeOptions` defaults and
playground limits unchanged. Do not introduce a facade-owned `ResourceLimits` replacement.

Convert CLI runtime diagnostics to the existing display/structured output with `nx-ir-*` codes and
debug source locations. Without source/debug information, identify the module/declaration and omit
unavailable line/column information rather than fabricate a location. Static errors still prevent
IR emission/execution. Repeated source compilation is not a new API responsibility.

### 3. Preserve formatting without preserving execution wrappers

There are two distinct formatting inputs:

- Canonical values from hosts: retain `format_program_artifact_value` / WASM `formatValue` and
  annotations, using program declarations/types and canonical values directly. Remove the current
  conversion through `nx_interpreter::Value`. Keep established canonical JSON ambiguities and
  explicit type/state options; do not silently promise recovery of erased information.
- CLI runtime results: retain NX spelling and JSON output, including enum cases at `object` sites,
  function references and `float32` spelling. Expose the smallest read-only runtime result/value
  view needed by the formatter without exposing mutable runtime internals or adding a source API.
  Preserve float32 provenance through constants, operations and typed normalization if necessary.
  Test nested/union/object positions and round-trip formatting explicitly. The runtime remains
  compiler-independent; program/HIR lookups for annotated formatting stay above it.

Do not add value-returning lifecycle variants only to replace APIs being deleted. Existing canonical
lifecycle results and runtime instance storage remain sufficient. Rich function-result access is
justified by CLI spelling and must not alter the existing canonical result contract or IR schema.

### 4. Remove the legacy snapshot binding contract

Delete the opaque interpreter snapshot transport and its wrappers along with the APIs that consume
it. Do not serialize `ComponentInstance` to a new MessagePack binding snapshot, add an old snapshot
reader or implement a new binding compatibility layer. The existing IR runtime's instance storage
and restoration are unchanged and continue to validate image fingerprints and captured values.

Preserve component semantics (defaults, explicit state, update validation, handler tokens, stale
invocations, atomic dispatch, effects and cross-module calls) in tests through existing IR runtime
APIs. Tests of the removed binding envelope/overloads/transport are intentionally removed with an
inventory explaining their disposition. DTO and serialization tests remain independent of execution.
The pending `support-stateful-component-value-evaluation` change must be rebased/re-scoped if it
still proposes interpreter or deleted binding work; it is not a prerequisite to preserve new wrappers.

### 5. Integer semantics follow the existing IR runtime

Remove only interpreter-specific `int32` wrapping. For example, adding two `2000000000` values yields
`4000000000` on the IR path. This does not promise exact integer equality across Rust i64 wrapping
and JavaScript number arithmetic beyond the shared safe range. Keep existing backend limitations
and deferred checked arithmetic. Include boundary tests around 2^31, 2^53 and i64 overflow that
state the applicable backend's actual contract rather than asserting false universal parity.

### 6. Preserve semantic expectations before deletion

Use integration-test helpers above the compiler/runtime boundary (for example under
`nx-api/tests/support`): build source, emit all modules, prepare/link and invoke an IR runtime API.
They are not exported from production crates or bindings. Keep lower runtime tests compiler-free.

Inventory interpreter integration/unit tests and current facade/binding tests by behavior:

- Port handwritten source/expected-value and diagnostic tests to source-to-IR helpers.
- Port lifecycle/explicit-state/host-boundary tests to existing runtime APIs; assert expected values,
  tokens/effects and state transitions rather than removed wrapper shapes.
- Retain build/schema/format/DTO/interop tests against the surviving APIs.
- Remove interpreter-internal and deleted API/envelope tests with a recorded reason for each group.

Run both engines temporarily where the interpreter supports the case. Record declared differences
(int32 wrapping, runtime codes/limit units) separately from regressions. Known interpreter failures
must be positive IR tests, not forced to pass in the old engine. Current examples include
`a_tool_list_holds_a_subtype_declared_in_a_host_library`,
`an_omitted_property_takes_a_library_default_that_names_a_library_agent` in
`crates/nx-api/tests/codegen/ir_runtime_tests.rs`, and imported workspace values read by `root`.

Use the Rust runtime for corpus generation and native parity, but retain independent handwritten
expectations. Never silently bless changed goldens by regenerating from the implementation under
test. Every changed result requires a declared behavior change or a separately reviewed correction.
Generated JS and TypeScript IR comparisons remain useful; two runtimes consuming the same wrong IR
cannot alone detect an emitter defect. The final differential inventory is a deletion gate.

## Risks / Trade-offs

- API removal breaks external callers not present in the local audit: document exact removals and
  migration paths; no claim of backward compatibility.
- Independent interpreter oracle is lost: retain source expectations, reviewed corpus results and
  generated-JS/two-runtime comparisons before deleting it.
- Source-reachable emitter gaps can block CLI migration: audit all rejection paths, not only
  `nx-ir-unsupported-construct`; fix required coverage or resolve a prerequisite before deletion.
- Formatting can lose erased type information: independently test rich CLI results and canonical
  host formatting, preserving each input contract.
- Cold CLI execution adds compilation/emission: record small/large/multi-module timings and WASM
  size before/after; investigate material regressions before claiming completion.
- Removed SDK wrapper tests reduce raw counts: coverage accounting, not equal test counts, is the gate.

## Migration Plan

1. Recheck consumers, audit IR coverage and record baseline behavior/tests/performance.
2. Port semantic tests and prepare independent formatting/runtime-result support while both engines exist.
3. Move CLI source and direct IR execution to the runtime; verify output, diagnostics and limits.
4. Remove legacy facade/binding APIs and obsolete tests; retain tooling/format/DTO coverage and update ABIs.
5. Replace corpus/parity oracle calls and complete the differential/coverage inventory.
6. Delete the interpreter in a separate final implementation commit, sweep specs/docs and validate.

No implementation is authorized by this proposal PR. Review the updated architecture before applying.

## Open Questions

- Acceptable cold-run regression thresholds should be settled from the baseline during review
  before CLI cutover; flag spellings above are proposed for this review.
- Which minimal read-only IR result interface best preserves CLI spelling without exposing runtime
  internals? The implementation must demonstrate the object/enum/function/float32 cases first.
