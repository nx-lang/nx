# Review: add-abstract-record-inheritance

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/record-type-inheritance/spec.md, tasks.md
**Reviewed code:** crates/nx-hir/src/records.rs, crates/nx-hir/src/lib.rs, crates/nx-hir/src/lower.rs, crates/nx-types/src/check.rs, crates/nx-types/src/infer.rs, crates/nx-types/tests/type_checker_tests.rs, crates/nx-interpreter/src/interpreter.rs, crates/nx-interpreter/src/error.rs, crates/nx-interpreter/tests/simple_functions.rs, crates/nx-api/src/eval.rs, crates/nx-syntax/grammar.js, crates/nx-syntax/src/validation.rs, src/vscode/syntaxes/nx.tmLanguage.json

## Findings

### ✅ Verified - RF1 Cascading duplicate diagnostics when a mid-chain base is invalid

- **Severity:** Medium
- **Evidence:** `validate_record_definitions` in [records.rs](crates/nx-hir/src/records.rs#L184) iterates every `RecordKind::Plain` record and calls `effective_record_shape` independently. `effective_record_shape` walks the full base chain on each call. If a record `B` has a valid abstract base `A`, but `A`'s base is concrete, both the call for `B` and the call for `A` will fail with the same error message ("A extends Concrete, but only abstract records may be extended"). Users see the same diagnostic twice pointing to the same root cause.
- **Recommendation:** Track which records have already produced an error during `validate_record_definitions` and skip descendants whose base chain is already known to be invalid. Alternatively, validate top-down and stop descending as soon as an invalid base is found.
- **Fix:** `validate_record_definitions` now memoizes per-record validation state, short-circuits descendants whose base chain is already invalid, and includes a lowering regression that asserts an invalid mid-chain base is reported only once.
- **Verification:** Confirmed. `validate_record_definitions` ([records.rs:190](crates/nx-hir/src/records.rs#L190)) now delegates to a `validate_record_definition` recursive helper that takes a `statuses: FxHashMap<Name, RecordValidationStatus>`. The helper returns early on a cached status (line 212), marks descendants as `Invalid` when their base chain fails without re-emitting the error, and a `push_unique_record_error` guard deduplicates any errors that could slip through edge cases. The logic correctly emits only one error for the root invalid base.

---

### ✅ Verified - RF2 `effective_record_shape` computed twice for every record construction in the interpreter

- **Severity:** Medium
- **Evidence:** Both `eval_record_constructor_call` ([interpreter.rs:1218](crates/nx-interpreter/src/interpreter.rs#L1218)) and `eval_element_expr` ([interpreter.rs:1317](crates/nx-interpreter/src/interpreter.rs#L1317)) call `effective_record_shape` to resolve field metadata (for arg-count validation and `children`-field detection, respectively), then immediately call `build_record_value` → `build_record_value_from_definition` ([interpreter.rs:2062](crates/nx-interpreter/src/interpreter.rs#L2062)), which calls `effective_record_shape` a second time to iterate fields. For deep inheritance chains this doubles the ancestor-walk cost on every construction.
- **Recommendation:** Accept the resolved `EffectiveRecordShape` as a parameter to `build_record_value_from_definition` (or introduce an internal overload that takes a pre-resolved shape) so call sites can pass the already-computed shape instead of re-resolving.
- **Fix:** Record construction now flows through a `build_record_value_from_shape` helper that consumes a pre-resolved `EffectiveRecordShape`, so constructor calls and record-like element evaluation reuse the shape they already computed instead of resolving it twice.
- **Verification:** Confirmed. `build_record_value_from_shape` ([interpreter.rs:2034](crates/nx-interpreter/src/interpreter.rs#L2034)) takes an `EffectiveRecordShape` directly and no longer calls `effective_record_shape` internally. Both `eval_record_constructor_call` (line 1241) and `eval_element_expr` (line 1329) now pass their pre-computed `record_shape` to it directly. The `build_record_value` helper path (line 2031) also resolves once and passes through. No double computation remains.

---

### 🔴 Open - RF3 `lower_source_module` in nx-api short-circuits on any lowering diagnostic, suppressing downstream type errors

- **Severity:** Medium
- **Evidence:** [eval.rs:44-63](crates/nx-api/src/eval.rs#L44) returns `Err` immediately when `module.diagnostics()` is non-empty. Previously the function returned `Ok(module)` unconditionally and callers aggregated errors. Now a single record-inheritance error (e.g., a concrete base) prevents the API caller from ever seeing type-checker diagnostics from the same file. A developer fixing the inheritance error in isolation might also have unrelated type errors that are hidden until the first fix is applied.
- **Recommendation:** Collect lowering diagnostics as `Err`-level items but continue to run the type checker and aggregate all diagnostics before returning, consistent with how `check_str`/`check_file` in `nx-types/src/check.rs` handle lowering errors alongside type errors.
- **Status:** Left open in this pass. `nx-api` currently exposes parse/lower/runtime evaluation only; aggregating type-check diagnostics would require threading `nx-types` through the public API and redefining the error contract for `eval_source` and the component helpers.

---

### ✅ Verified - RF4 `UndefinedVariable` error variant used when a record type cannot be resolved in the interpreter

- **Severity:** Low
- **Evidence:** The interpreter's `effective_record_shape` helper ([interpreter.rs:~1757](crates/nx-interpreter/src/interpreter.rs#L1757)) converts `Ok(None)` from `effective_record_shape_for_name` into `RuntimeErrorKind::UndefinedVariable`. This fires for any call-site that constructs a record whose name doesn't resolve—usually a bug during development—and the resulting runtime error says the *name* is undefined rather than the record type. `UndefinedVariable` is semantically for variable bindings, not type resolution.
- **Recommendation:** Introduce a `RecordTypeNotFound` error kind (or reuse `TypeMismatch` with a clearer message), so the runtime error accurately communicates "record type not found" rather than "variable undefined".
- **Fix:** Added `RuntimeErrorKind::RecordTypeNotFound` and routed unresolved record-shape lookups through it. The interpreter now reports missing record types distinctly from missing variable bindings, with a direct regression test for a record literal referencing an unknown record type.
- **Verification:** Confirmed. `RuntimeErrorKind::RecordTypeNotFound { name: SmolStr }` is defined at [error.rs:79](crates/nx-interpreter/src/error.rs#L79) with a dedicated doc comment. The interpreter's `effective_record_shape` helper ([interpreter.rs:1769](crates/nx-interpreter/src/interpreter.rs#L1769)) now converts `Ok(None)` into `RecordTypeNotFound` instead of `UndefinedVariable`. A regression test in error.rs (line 354) verifies the new variant's Display output.

---

### ✅ Verified - RF5 Missing type-checker-level test for the concrete-base rejection scenario

- **Severity:** Low
- **Evidence:** The spec scenario "Concrete record cannot be extended" is covered only in the HIR lowering test `test_lower_record_inheritance_validation_diagnostics` ([lower.rs](crates/nx-hir/src/lower.rs)). `check_str` in `crates/nx-types` now surfaces lowering diagnostics (via `lowering_diagnostics`), but there is no test in `type_checker_tests.rs` that calls `check_str` with a concrete-base declaration and asserts the expected diagnostic is present. If the propagation path from `lowering_diagnostics` to the type-check result is ever broken, this gap would prevent catching it.
- **Recommendation:** Add a `check_str`-level test mirroring the HIR test for concrete-base rejection, asserting that the `"record-base-not-abstract"` (or the lowering-error code) diagnostic appears in the `TypeCheckResult`.
- **Fix:** Tightened the `check_str` regression to assert that the concrete-base rejection appears in the `TypeCheckResult` as a `lowering-error`, so the propagation path is now covered explicitly.
- **Verification:** Confirmed. `test_concrete_record_cannot_be_used_as_base` ([type_checker_tests.rs:267](crates/nx-types/tests/type_checker_tests.rs#L267)) calls `check_str` with a three-record hierarchy where a concrete record is used as a base, and asserts that a diagnostic with `code() == Some("lowering-error")` and message containing `"only abstract records may be extended"` is present in the result. This explicitly covers the propagation path from HIR validation through `check_str`.

---

## Questions

- ✅ Verified: both paths are now covered. `test_record_inheritance_accepts_concrete_leaf_for_abstract_return_type` ([type_checker_tests.rs:199](crates/nx-types/tests/type_checker_tests.rs#L199)) asserts that `let make(): UserBase = { <User .../> }` type-checks without errors. `test_derived_record_return_satisfies_abstract_ancestor_return_type` ([simple_functions.rs:304](crates/nx-interpreter/tests/simple_functions.rs#L304)) executes the same pattern and asserts the returned value has `type_name = "User"` with all expected fields present at runtime.

## Summary

The implementation is correct across all layers: grammar, HIR lowering, type checking, interpreter, and VS Code editor support. All spec scenarios from `specs/record-type-inheritance/spec.md` are covered by tests. The inheritance cycle detection, effective-field merging, subtype compatibility, and abstract non-instantiability rules work as designed. The main concerns are operational quality issues—cascading duplicate diagnostics (RF1), redundant computation (RF2), and an early-exit in the public API that hides downstream errors (RF3)—rather than behavioral bugs or spec gaps.
