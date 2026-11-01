## Why

NX hosts already compile source to IR and execute IR, while the HIR interpreter duplicates language
semantics primarily for legacy SDK entry points and tests. With `refactor-tooling-api` merged, we can
retire that engine and simplify the public APIs instead of rebuilding unused execution wrappers.

## What Changes

- **BREAKING**: Delete `nx-interpreter` and remove its source/artifact evaluation and component
  lifecycle APIs from `nx-api`, C FFI, .NET, Node and WebAssembly. Do not preserve their signatures
  with implementations that compile to IR, compatibility aliases, or public test helpers.
- Keep source/workspace/library construction, validation, diagnostics, schemas, language services,
  code generation, and program-aware value formatting. `nx-api` continues to expose execution
  through its existing `ir_runtime` namespace; runtime-only Rust hosts use `nx-ir-runtime` directly.
- Preserve `nxlang run <file.nx>`: compile NX source to IR, prepare/link all required modules and
  execute the selected entry module. Running an NX IR file directly is deferred to its own change.
- Keep one annotated-NX-text formatter, replacing its interpreter `Value` with a small
  formatter-owned model. It serves `formatValue` for the playground and other tooling, and CLI
  NX/JSON output through a read-only Rust runtime result view that keeps constant cases and
  function references. A `float32` is spelled by its declared type; at an `object` site it prints
  as its `float64` widening.
- **BREAKING**: Remove the legacy opaque component-snapshot binding contract and result wrappers.
  No snapshot conversion or replacement binding is added. Existing IR runtime instance storage,
  lifecycle, explicit-state rendering, tokens and atomic dispatch remain supported.
- **BREAKING**: Native source execution no longer applies the interpreter's special `int32`
  arithmetic wrapping. It follows the existing IR runtime's `int` arithmetic and existing backend
  precision/overflow limitations; this does not establish universal integer parity.
- CLI runtime failures use IR runtime diagnostic codes and debug locations. Retain a finite CLI
  operation budget; remove the interpreter-specific public `ResourceLimits` type.
- Preserve handwritten language-behavior expectations through test-only source-to-IR helpers,
  temporarily comparing both engines before deletion. Corpus/parity tests use the Rust IR runtime
  and explicit expectations rather than a public source-evaluation API.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `rust-ir-runtime`: Sole Rust evaluator; CLI source/IR execution, value spelling, and source-level validation.
- `resolved-program-runtime`: Resolved programs feed compilation; execution and stored instances refer to IR.
- `component-runtime-bindings`: Remove legacy artifact/source lifecycle and snapshot binding requirements.
- `sdk-node`: Remove source/artifact evaluation; retain tooling and direct callers to the TypeScript IR runtime.
- `sdk-wasm`: Remove `evaluateNx`; retain independent canonical-value formatting and tooling.
- `dotnet-binding`: Remove source/artifact execution and lifecycle wrappers; retain tooling and DTO serialization.
- `source-analysis-pipeline`: Preserve analysis gating for CLI compilation/execution without requiring `eval_source`.
- `value-view`: Consume annotated formatted values without depending on a source-evaluation method.
- `executable-code-generation`: Validate emitted behavior using source expectations and IR runtime parity.
- `nx-ir-format`: Generate and check corpus results through the Rust IR runtime.
- `typescript-ir-runtime`: Compare with the Rust IR runtime rather than the interpreter.
- `primitive-type-names`: Remove int32-specific native wrapping without extending the integer precision contract.
- `rust-crate-layering`: Drop `nx-interpreter` from the layering rules.
- `value-constraints`: Remove the interpreter-only constraint requirement.
- `artifact-model`, `component-action-handlers`, `conditional-result-types`, `content-properties`,
  `discriminated-unions`, `external-components`, `implicit-primitive-conversions`, `occurrence-types`,
  `playground`, `presence-operators`, `range-expressions`, `record-type-inheritance`,
  `record-type-parameters`, `runtime-standard-functions`, `sequence-model`, `update-records`,
  `visualizer-declarations`: Name the Rust IR runtime wherever a requirement names the interpreter
  (usually as one of the engines that must agree), with no change of behavior.

## Impact

- Prerequisites: `add-rust-ir-runtime` and `refactor-tooling-api` are complete. The latter is archived
  as `2026-11-01-refactor-tooling-api` and merged in PR #68. No crate extraction is repeated.
- Delete interpreter code and interpreter-dependent API types/conversions; update binding exports,
  generated C header and ABI versions, documentation, examples and tests together.
- Keep `nx-program` below codegen/tooling and independent of execution. No runtime cache is added to
  `ProgramArtifact`, and no public prepared-source execution wrapper is introduced.
- ReachMe and FiddleEngine use IR execution, and the playground already executes IR then formats
  values. No application caller of the legacy methods was found in the local source audit; SDK
  tests and documented examples do use them. Recheck callers at implementation time.
- No implementation is part of this proposal PR. LSP snippet APIs, compiler constant evaluation,
  new runtime bindings for .NET/C, and execution caching are separate future decisions.

## Accepted follow-up gaps

The user approved retirement before `complete-ir-construction-lowering`. Conditional property
fragments and positional record constructors are temporarily unsupported through IR execution.
Preserve their successful source expectations as ignored source-to-IR tests, each naming that
follow-up change; do not assert failure as the intended behavior. Confirm expectations against HIR
before deletion. These two gaps do not block retirement; other unexplained discrepancies still do.
