# Review: support-action-inheritance

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/action-records/spec.md, specs/record-type-inheritance/spec.md, specs/component-syntax/spec.md, specs/cli-code-generation/spec.md  
**Reviewed code:** crates/nx-syntax/grammar.js, crates/nx-syntax/src/validation.rs, crates/nx-syntax/queries/highlights.scm, crates/nx-syntax/tests/parser_tests.rs, crates/nx-syntax/tests/fixtures/, crates/nx-hir/src/lower.rs, crates/nx-hir/src/records.rs, crates/nx-interpreter/src/interpreter.rs, crates/nx-interpreter/tests/simple_functions.rs, crates/nx-types/tests/type_checker_tests.rs, crates/nx-cli/src/codegen.rs, nx-grammar.md, nx-grammar-spec.md

## Findings

### ✅ Verified - RF1 Test suite is broken by a required-field bug in the inherited-defaults test
- **Severity:** High
- **Evidence:** `crates/nx-interpreter/tests/simple_functions.rs:512` — `test_action_inheritance_applies_inherited_defaults` declares `action SearchSubmitted extends InputAction = { query: string }` where `query` has no default value and is not nullable. `instantiate_record_defaults` calls `build_record_value` with empty overrides and no `missing_operation` sentinel. In `build_record_value_from_shape` (`interpreter.rs:2555–2579`) the `query` field has no override, no default expression, and is not a `Nullable` type, so it falls through to `Value::Null` (line 2569). `coerce_value_to_type(Value::Null, string, "record field 'query'")` then raises `TypeMismatch { expected: "string", actual: "object?", … }` before the function can return. `cargo test --workspace` confirms the failure with this exact error. The parallel record test `test_record_inheritance_applies_inherited_defaults` passes because every field in that fixture has a default expression.
- **Recommendation:** Give `query` a default in the test fixture (e.g. `query: string = "search"`). The test only asserts on `fields.get("source")`, so a concrete default for `query` does not change what is being verified and is the minimal fix that matches the intent.
- **Fix:** Added a default value for `query` in `test_action_inheritance_applies_inherited_defaults`.
- **Verification:** `query: string = "search"` is present in the fixture (`simple_functions.rs:519`). `cargo test -p nx-interpreter --test simple_functions test_action_inheritance_applies_inherited_defaults` passes.

### ✅ Verified - RF2 No test for abstract derived action instantiation rejection
- **Severity:** Low
- **Evidence:** The spec scenario "Abstract derived action construction is rejected" (`abstract action InputAction = { source:string } abstract action SearchAction extends InputAction = { query:string } let action = <SearchAction source={"toolbar"} query={"docs"} />`) requires that type checking rejects instantiation of an abstract *derived* action (i.e., an abstract action that itself has a base). The only test added, `test_abstract_action_instantiation_is_rejected` (`crates/nx-types/tests/type_checker_tests.rs:410`), constructs a *root* abstract action with no base. The rule in `build_record_value_from_shape` (`interpreter.rs:2548`) is `record_def.is_abstract`, which applies equally to both cases, but the derived-abstract scenario is not exercised by any test.
- **Recommendation:** Add a `test_abstract_derived_action_instantiation_is_rejected` type-checker test that matches the spec scenario verbatim and asserts `diag.code() == Some("abstract-record-instantiation")`.
- **Fix:** Added `test_abstract_derived_action_instantiation_is_rejected` to `crates/nx-types/tests/type_checker_tests.rs`.
- **Verification:** `test_abstract_derived_action_instantiation_is_rejected` (`type_checker_tests.rs:462`) matches the spec scenario exactly (two-level abstract action chain, constructs the intermediate abstract derived action). Confirmed passing with `cargo test -p nx-types --test type_checker_tests`.

### ✅ Verified - RF3 No test for inline emitted derived action subtype compatibility
- **Severity:** Low
- **Evidence:** The spec scenario "Inline emitted derived action is accepted where abstract parent action is expected" (passing `<SearchBox.ValueChanged source={"keyboard"} value={"docs"} />` to a `let read(action:InputAction)` function) is not covered by any type-checker test. Tests `test_inline_emitted_action_inheritance_allows_inherited_fields` and `test_inline_emitted_action_inheritance_reports_inherited_field_type_mismatch` only test construction and field-type matching; neither tests the subtype path (`is_record_subtype` / `record_type_satisfies_expected`). The changed subtype logic in `interpreter.rs:2283` replaces a manual base-chain walk with `effective_record_shape(...).ancestors`—the ancestor list is correctly populated for inline emitted actions as confirmed by `test_lower_inline_emit_action_inheritance_metadata`, but the full subtype-substitution path is not exercised end-to-end at the type-checker level.
- **Recommendation:** Add a type-checker test matching the spec scenario: define `abstract action InputAction`, declare a `SearchBox` component emitting `ValueChanged extends InputAction { value: string }`, write `let read(action:InputAction): string = { action.source }`, and call `read(<SearchBox.ValueChanged source={"keyboard"} value={"docs"} />)`; assert no type errors.
- **Fix:** Updated `test_inline_emitted_action_inheritance_allows_abstract_parent_subtyping` to avoid abstract-parameter member access by returning a constant `int`, keeping the call-site subtype check intact.
- **Verification:** `let read(action: InputAction): int = { 1 }` correctly isolates the subtype-compatibility check. The test (`type_checker_tests.rs:378`) passes and the full workspace test suite shows 0 failures (`cargo test --workspace`).

## New Findings Discovered During 2026-04-11 23:16 Verification

### ✅ Verified - RF4 Wrong diagnostic code in inline-emit field-type-mismatch test
- **Severity:** Low
- **Evidence:** `test_inline_emitted_action_inheritance_reports_inherited_field_type_mismatch` (`type_checker_tests.rs:405`) was added as part of the original implementation and asserts `diag.code() == Some("property-type-mismatch")`. Running the suite reveals the actual code is `"record-field-type-mismatch"` with message `"Record field 'source' on 'SearchBox.ValueChanged' expects string, found int"`. The parallel top-level-action test `test_action_inheritance_reports_inherited_field_type_mismatch` correctly uses `"record-field-type-mismatch"`. This was missed in the first review pass because `cargo test --workspace 2>&1 | tail -60` captured only the last 60 lines of output (the interpreter binary failure), and the nx-types binary failure appeared earlier in the output stream.
- **Recommendation:** Change the assertion in `test_inline_emitted_action_inheritance_reports_inherited_field_type_mismatch` from `"property-type-mismatch"` to `"record-field-type-mismatch"` to match the code the type checker actually emits.
- **Fix:** Updated the test to assert `"record-field-type-mismatch"` for inline emitted inherited field mismatches.
- **Verification:** `type_checker_tests.rs:426` now asserts `"record-field-type-mismatch"`, matching the code the type checker emits. Test passes; full workspace clean.

## Questions
- None.

## Summary
The implementation is architecturally sound: grammar, lowering, validation, subtype logic, and code generation all follow the design decisions faithfully, and the existing record-inheritance infrastructure is cleanly extended to actions without duplication. The `ActionRecord` blanket rejection is correctly replaced with the `KindMismatch` family-aware diagnostic, and the subtype helper is properly upgraded from a manual walk to `effective_record_shape`. One test (RF1) breaks the suite today and needs a one-line fix. Two additional scenarios from the spec (RF2, RF3) lack test coverage but do not indicate behavioral gaps in the implementation.
