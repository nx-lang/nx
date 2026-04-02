## 1. Shared Analysis Core in `nx-types`

- [x] 1.1 Add a shared source-analysis result type and `analyze_str` implementation in `crates/nx-types/src/check.rs` that runs parse, lowering, scope building, and type checking while preserving the lowered module
- [x] 1.2 Thread the caller-provided `file_name` through lowering, scope, and type diagnostics so static-analysis labels keep the original file identity and source spans
- [x] 1.3 Refactor `check_str` and `check_file` to delegate to the shared analysis path and add regression tests for aggregated lowering-plus-type diagnostics and file-name fidelity

## 2. Adopt the Shared Pipeline in `nx-api`

- [x] 2.1 Add the `nx-types` dependency to `crates/nx-api` and update `eval_source`, `initialize_component_source`, and `dispatch_component_actions_source` to call the shared analysis entry point before any runtime work
- [x] 2.2 Preserve entrypoint-specific second-phase validation in `nx-api`, including `no-root`, invalid host input, and invalid snapshot errors, but only after static analysis completes without errors
- [x] 2.3 Narrow, remove, or rename `lower_source_module` so the remaining helper surface reflects true parse/lower behavior rather than a partial analysis pipeline

## 3. Cross-Layer Regression Coverage

- [x] 3.1 Add `nx-api` tests proving source evaluation returns combined static diagnostics and does not execute runtime behavior when analysis reports errors
- [x] 3.2 Add component-source tests proving initialization and dispatch return aggregated static diagnostics and skip component lifecycle work when analysis fails
- [x] 3.3 Add or update FFI and .NET smoke coverage as needed to confirm the richer diagnostic sets and preserved file names serialize through existing bindings without changing success payloads

## 4. Documentation and Cleanup

- [x] 4.1 Update crate-level docs and runtime-facing documentation to describe the shared source-analysis path and the analyze-then-execute contract for source-driven APIs
- [x] 4.2 Remove stale comments or future-work notes that describe the old parse/lower short-circuit behavior once the new pipeline is in place
