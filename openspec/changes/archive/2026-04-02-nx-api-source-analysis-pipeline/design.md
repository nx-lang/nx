## Context

`nx-api` currently exposes three source-driven runtime entry points:

- `eval_source`
- `initialize_component_source`
- `dispatch_component_actions_source`

All three call `lower_source_module`, which parses source, lowers HIR, and returns early on any
parse or lowering error. If lowering succeeds, the helpers move directly into entrypoint-specific
validation and interpreter execution.

`nx-types::check_str` already runs a broader static pipeline:

1. parse source
2. lower HIR
3. build scopes
4. infer and check types

That broader pipeline is the behavior we want, but `check_str` is not a drop-in dependency for
`nx-api` today:

- it is packaged as a type-checking helper rather than a shared analysis boundary
- it stamps lowering and type-check diagnostics with empty label file names
- it does not currently define the runtime gating rule that source-driven execution must stop if
  static analysis reports any errors

The result is an inconsistent developer experience. `nx-api` callers can fix one lowering issue,
rerun, and only then discover independent scope or type errors that were already present in the
same source file.

## Goals / Non-Goals

**Goals:**
- Introduce a single shared source-analysis entry point beside `nx-types::check_str`.
- Reuse that analysis path from `nx-api` source-driven runtime helpers.
- Aggregate parse, lowering, scope, and type diagnostics before runtime execution.
- Preserve the lowered `Module` from analysis so `nx-api` can interpret without reparsing or
  relowering.
- Preserve caller-provided `file_name` and source spans in lowering, scope, and type diagnostics.
- Keep runtime-only validation and interpretation as a second phase that runs only after static
  analysis succeeds.

**Non-Goals:**
- Change the success payloads or wire formats exposed by `nx-api`, `nx-ffi`, or the .NET binding.
- Introduce multi-file analysis, caching, or incremental compilation in this change.
- Reclassify runtime-only failures such as invalid snapshots, invalid host payloads, or missing
  root/component entrypoints as static-analysis diagnostics.
- Expand `lower_source_module` into a larger pipeline helper.

## Decisions

### 1. Add a shared `analyze_str` entry point in `nx-types`

`nx-types` will own a new public analysis entry point, tentatively
`analyze_str(source, file_name) -> SourceAnalysisResult`, colocated with `check_str` in
`crates/nx-types/src/check.rs`.

`SourceAnalysisResult` should carry the data that both `check_str` and `nx-api` need:

- `module: Option<Arc<Module>>`
- `type_env: TypeEnvironment`
- `diagnostics: Vec<Diagnostic>`
- `source_id: SourceId`

This keeps the full static pipeline in one crate and gives `nx-api` a reusable result that already
contains the lowered module for later interpretation.

`check_str` and `check_file` should become thin wrappers around the new analysis core so that there
is only one implementation of parse/lower/scope/type orchestration.

Alternative considered: teach `nx-api::lower_source_module` to perform scope building and type
checking. Rejected because it pushes compiler-pipeline logic into the runtime layer and makes the
helper name misleading.

### 2. Make diagnostic construction file-name aware inside the shared analysis core

The shared analysis path must preserve the caller-provided `file_name` across every static phase.
Today, parse diagnostics already do that, but `nx-types` lowering and inference diagnostics build
labels with an empty file name.

The analysis core should therefore pass `file_name` explicitly into each phase that can construct
diagnostics:

- lowering-diagnostic conversion in `check.rs`
- type-check diagnostics emitted from `InferenceContext`
- any future scope diagnostics returned from `nx-hir`

If a phase still returns diagnostics without label file names, the shared analysis layer should
normalize those diagnostics before returning them. The important rule is that no public analysis
entry point may discard the caller's file name.

Alternative considered: reuse `check_str` as-is and patch empty file names inside `nx-api`.
Rejected because the helper we are trying to standardize on is itself the layer that currently
drops the file name.

### 3. Make `nx-api` a strict two-phase pipeline: analyze first, execute second

Each source-driven runtime helper in `nx-api` should adopt the same structure:

1. call `nx_types::analyze_str`
2. convert static diagnostics to `NxDiagnostic`
3. if any returned diagnostic has error severity, return all static diagnostics immediately
4. otherwise run entrypoint-specific validation and interpreter execution against the analyzed
   module

Entrypoint-specific checks remain outside the shared analysis phase:

- `eval_source` still validates that a `root` function exists
- `initialize_component_source` still validates host props and component lookup
- `dispatch_component_actions_source` still validates snapshots and action payloads

This preserves the existing public contract for runtime-only failures while ensuring that static
analysis errors always win and are reported together.

Alternative considered: continue interpreting when static analysis produced non-fatal errors.
Rejected because the desired contract is "all static errors first, no interpretation on error."

### 4. Narrow or remove `lower_source_module`

Once `nx-api` consumes the shared analysis result, `lower_source_module` no longer needs to sit on
the public source-to-runtime path.

Preferred outcome:

- remove `lower_source_module` from the runtime flow entirely
- inline any remaining parse/lower-only behavior into a clearly named local helper if tests still
  need it

If a helper remains, it should be named for exactly what it does, such as
`parse_and_lower_source`, and it should not return partially analyzed results.

Alternative considered: keep the helper and expand it to return analysis plus diagnostics.
Rejected because it hides the architectural boundary between static analysis and runtime execution.

### 5. Preserve deterministic diagnostic ordering and coverage

The shared analysis path should return diagnostics in stable phase order:

1. parse diagnostics
2. lowering diagnostics
3. scope diagnostics
4. type diagnostics

Tests should assert both aggregation and gating:

- `nx-types` tests for file-name-aware lowering/type diagnostics and aggregate results
- `nx-api` tests proving `eval_source`, `initialize_component_source`, and
  `dispatch_component_actions_source` return combined static errors without interpreting
- binding-level smoke tests only where necessary to confirm unchanged serialization of the richer
  diagnostic set

Stable ordering reduces churn in snapshot-style tests and makes the resulting API easier for hosts
to reason about.

## Risks / Trade-offs

- Dependency expansion from `nx-api` to `nx-types` → acceptable because the dependency direction
  remains one-way and matches the intended architectural boundary.
- Diagnostic count and ordering will change for some failing inputs → mitigate with explicit tests
  that codify the new phase order and by updating any brittle assertions.
- Type checking on partially invalid modules can surface more errors than before → mitigate by
  keeping the same recovery model already used by `check_str` and only skipping later phases when
  parsing fails completely.
- A new public analysis API adds another surface in `nx-types` → mitigate by making `check_str`
  delegate to it so the new API becomes the single source of truth rather than parallel behavior.

## Migration Plan

1. Add the shared analysis result type and `analyze_str` implementation in `nx-types`.
2. Refactor `check_str` and `check_file` to delegate to that shared implementation.
3. Add an `nx-types` dependency to `nx-api` and switch source-driven runtime helpers to the
   two-phase analyze-then-execute flow.
4. Remove or rename `lower_source_module` so it no longer implies a partial analysis pipeline.
5. Update tests and any runtime-facing documentation that describes source diagnostics.

Rollback is straightforward: revert the `nx-api` call sites to the old parse/lower helper and
leave the new analysis API unused. No persisted data or wire-format migration is involved.

## Open Questions

None for this change. The scope is intentionally limited to single-source analysis reuse and
runtime gating; incremental or multi-file analysis remains separate future work.
