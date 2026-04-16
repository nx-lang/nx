# Review: support-dual-annotated-csharp-dtos

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/cli-code-generation/spec.md, tasks.md  
**Reviewed code:** crates/nx-cli/src/codegen/languages/csharp.rs, crates/nx-cli/src/codegen.rs (tests), crates/nx-cli/src/codegen/model.rs, crates/nx-cli/src/main.rs (CLI integration tests)  

## Findings

### ✅ Verified - RF1 Two CLI integration tests fail after dual-annotation changes
- **Severity:** High
- **Evidence:** `cargo test -p nx-cli` reports 2 failures:
  - `test_cli_generate_file_emits_external_component_state_contract` ([main.rs:1043](crates/nx-cli/src/main.rs#L1043)) asserts `!stdout.contains("$type")` on the full TypeScript output. The output now includes `$type` from the component props contract (SearchBox record), causing the assertion to fail even though SearchBox_state correctly omits it.
  - `test_cli_generate_library_writes_csharp_external_component_state_output` ([main.rs:1269](crates/nx-cli/src/main.rs#L1269)) asserts `!search_box.contains("__NxType")` on the entire `search-box.g.cs` file. The file now contains both the SearchBox props record (which has `__NxType`) and SearchBox_state (which correctly omits it).
  
  The equivalent unit tests in [codegen.rs](crates/nx-cli/src/codegen.rs) were correctly updated to scope assertions to the state block, but these CLI integration tests were missed.
- **Recommendation:** Apply the same state-block-scoped assertion pattern used in the codegen unit tests. For each failing test, extract the SearchBox_state section from the output and assert the absence of `$type`/`__NxType` only within that section.
- **Fix:** Updated the CLI integration tests in [crates/nx-cli/src/main.rs](crates/nx-cli/src/main.rs) to scope the `$type` and `__NxType` absence checks to the `SearchBox_state` block instead of the entire generated output/file, matching the codegen unit-test pattern.
- **Verification:** Both tests now extract the `SearchBox_state` block and scope assertions correctly. Full test suite passes (84/84).

### ✅ Verified - RF2 `using System.Text.Json.Serialization` emitted for enum-only C# files
- **Severity:** Low
- **Evidence:** In [csharp.rs:56](crates/nx-cli/src/codegen/languages/csharp.rs#L56), `using System.Text.Json.Serialization;` is unconditionally emitted for every non-empty module. A library module containing only enums (which are excluded from dual annotations by design) will include an unused `using` directive, which may trigger compiler warnings (CS8019) in consuming projects that enable `TreatWarningsAsErrors`.
- **Recommendation:** Conditionally emit the `using` only when the module contains at least one record or external-state declaration. Alternatively, accept the unused import as harmless if no consuming project enables that warning level.
- **Fix:** Changed [crates/nx-cli/src/codegen/languages/csharp.rs](crates/nx-cli/src/codegen/languages/csharp.rs) to emit `using System.Text.Json.Serialization;` only when a module contains generated record or external-state declarations that actually need JSON attributes.
- **Verification:** The conditional correctly checks `!matches!(declaration.item, Alias(_) | Enum(_))`, emitting the `using` only for modules with Record or ExternalState declarations. Enum-only modules will no longer include the unused import.

## Questions
- None

## Summary
- The core C# generator logic in [csharp.rs](crates/nx-cli/src/codegen/languages/csharp.rs) is clean and correct. The `emit_dual_wire_name_attributes` and `emit_record_json_polymorphism_attributes` helpers are well-factored, and the guard for intermediate abstract types correctly prevents duplicate polymorphism metadata. The previously failing CLI integration tests are now aligned with the unit-test assertions, and the JSON serialization `using` is no longer emitted for enum-only C# modules.
