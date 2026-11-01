# Implementation notes: refactor-tooling-api

## Baseline

Baseline revision: `939b6ad`. Commands and final results are recorded below; detailed local logs
were originally under `/tmp/nx-refactor-validation/`; post-reboot reruns are under
`target/refactor-validation/`.

| Check | Baseline | After refactor |
| --- | --- | --- |
| `cargo test --workspace` | All unit/integration suites pass; documentation tests pass separately from an untouched archive of the baseline revision (the initial doc stage overlapped the extraction) | All suites pass, including documentation tests |
| `dotnet test bindings/dotnet/tests/NxLang.Sdk.Tests/NxLang.Sdk.Tests.csproj` after `cargo build --release -p nx-ffi` | 173 pass. Initial run against an older native library failed the math test; rebuilding resolves it | 173 pass |
| `pnpm --dir bindings/node test` | 38 pass | 38 pass |
| `pnpm --dir bindings/wasm test` | 135 pass, including typecheck and debug-trap build | 135 pass; same checks |
| `pnpm --dir sites/playground test` | 156 pass; 26 examples match | 156 pass; 26 examples match |
| `bindings/wasm/dist/nx.wasm` (`wasm32-wasip1`, `wasm-release`) | 4,544,663 bytes | 4,584,637 bytes (+39,974; +0.88%) |

## Test relocation

Pure construction, schema and visualizer tests remain beside their implementation in `nx-program`.
The following execution-dependent tests move to `nx-api/tests/`; their assertions are preserved.
Shared fixture helpers are copied only where both layers need them; production helpers stay private.

### `crates/nx-api/tests/program_visualizer_declarations_tests_evaluation.rs` (1 tests)

- `a_props_record_of_an_imported_component_is_available_through_the_import`

### `crates/nx-api/tests/program_artifacts_evaluation.rs` (21 tests)

- `property_union_of_a_workspace_import_includes_inherited_fields`
- `a_generic_record_is_applied_and_constructed_across_a_library_boundary`
- `consumer_imports_only_explicit_exports`
- `program_artifact_uses_registry_backed_build_context`
- `imported_content_bindings_match_local_behavior`
- `program_artifact_survives_build_context_drop`
- `build_context_with_visible_roots_includes_transitive_dependencies_of_visible_library`
- `build_context_with_visible_roots_limits_visible_libraries`
- `an_evaluated_generic_component_record_serializes_without_its_type_argument`
- `a_bare_case_of_an_unimported_union_evaluates_as_the_qualified_form_does`
- `a_visible_name_does_not_capture_a_reference_to_a_different_definition`
- `a_foreign_case_inheriting_an_abstract_base_carries_its_base_fields`
- `eval_workspace_artifact_uses_selected_entry_root`
- `eval_workspace_artifact_reports_no_root_for_selected_entry`
- `eval_workspace_artifact_executes_across_workspace_imports`
- `workspace_program_artifact_survives_workspace_drop`
- `workspace_program_artifact_survives_build_context_drop`
- `implicit_import_puts_a_control_in_scope_without_an_import_line`
- `a_document_that_is_one_trailing_element_compiles_with_an_implicit_import`
- `a_named_import_of_the_implicit_module_leaves_its_other_names_in_scope`
- `an_aliased_wildcard_import_of_the_implicit_module_leaves_its_other_names_in_scope`

### `crates/nx-api/tests/program_standard_library_tests_evaluation.rs` (5 tests)

- `a_single_source_imports_a_standard_library_with_an_empty_context`
- `a_context_limited_to_one_host_root_still_sees_standard_libraries`
- `a_standard_library_can_be_an_implicit_import`
- `a_program_reaches_a_standard_library_through_a_host_library`
- `math_standard_library_signatures_identity_and_imports`

### `crates/nx-api/tests/program_library_source_tests_evaluation.rs` (7 tests)

- `a_library_loads_from_memory_under_its_logical_root`
- `one_in_memory_library_imports_another`
- `a_loaded_library_is_imported_implicitly`
- `a_written_import_of_an_implicit_library_is_not_a_duplicate`
- `a_workspace_module_inside_a_library_root_is_refused`
- `a_directory_library_whose_path_ends_in_a_workspace_identity_does_not_make_it_ambiguous`
- `a_union_case_field_default_declared_in_a_library_applies_to_a_tenant_construction`

### Code generation and parity

The following complete modules move from `nx-codegen/src/` to `nx-api/tests/codegen/` under the
`codegen_parity` harness: `tests`, `ir_corpus_tests`, `ir_runtime_tests`, `ir_instance_tree_tests`,
`math_tests`, `color_transport_tests`, `graphics_tests`, and `ir_image_tests`, plus the shared
`ir_runtime_sources` fixture. Graphics/instance/image tests share the corpus and linking helpers
with the parity tests, so they move together rather than duplicating those helpers. IR emitter,
prelude-image, bundle, runtime-helper and allocation/cost tests remain in `nx-codegen`.

The migrated JavaScript/TypeScript tests obtain the emitted runtime helper from `emit_program`
instead of a private module. `CodegenProperty` and `CodegenRecordField` gain public re-exports;
they are already public data types in codegen's public model fields. No expectation changes.

Before: 1811 `#[test]` cases across the four affected crates. After extraction: 1812 across those crates plus `nx-program`; the only new Rust test is the facade composition test. No test names or cases were lost.

## Adapter dependencies

| Adapter | Route through nx-api | Keep | Remove |
| --- | --- | --- | --- |
| CLI | codegen; resolved types via flat foundation exports | interpreter for existing `nxlang run`; syntax/HIR/types/diagnostics/IR/value for command-specific tooling | direct codegen and nx-program |
| Node | codegen and language service | IR/value for encoding and host values; napi transport | direct codegen and language service |
| WASM | codegen and language service | graphics, diagnostics, IR/value for transport and rendering | direct codegen and language service |
| C/.NET FFI | codegen | IR/value for encoding; interpreter dev dependency actually used by ffi_smoke to test interpreter conversion | direct codegen; unused HIR/syntax/types dev dependencies |
| Graphics preview | codegen and IR runtime | graphics and skia renderer; value in renderer tests | direct optional codegen and IR runtime |
| LSP | none | direct language service and protocol dependencies | none |

## Verification after refactor

- Published runtime package verification passes: the standalone host builds only `nx-ir`,
  `nx-ir-runtime` and `nx-value` among the NX crates.
- `pnpm run check:crate-layering` passes, including ten checks of the actual graph and locally
  injected metadata edges for normal, dev, build and target-specific forbidden dependencies.
- `cargo check -p nx-ir-runtime --target wasm32-unknown-emscripten` passes. The WASI SDK and all
  native workspace targets build; native preview tests, build, Clippy and the preview smoke check also pass.
- WASM grows by 0.88% with the facade's newly reachable IR runtime dependency. This is small;
  unused runtime code is stripped by the linker. No feature gates are needed.
- Source evaluation, CLI `run`, component lifecycle, NX text and value conversion still use
  the HIR interpreter; no engine switch, execution cache, CLI IR execution or interpreter removal.

Full workspace build, tests and Clippy with warnings denied pass. Native graphics/preview tests,
build, Clippy and the preview smoke check pass. Formatting, dependency checks and strict OpenSpec
validation pass. No changed golden, snapshot or diagnostic expectations were needed.
