# Review: preserve-csharp-literal-defaults

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/cli-code-generation/spec.md, tasks.md
**Reviewed code:**
- crates/nx-cli/src/typegen/model.rs (default model + extraction)
- crates/nx-cli/src/typegen/languages/csharp.rs (emission, warnings, literal rendering, string escaping)
- crates/nx-cli/src/typegen.rs (tests)

All 9 tasks are marked complete and the 3 new tests plus the full `typegen` suite (78 passed) run green.

## Findings

### ✅ Verified - RF1 Non-nullable reference field with an unsupported default loses its `default!` initializer
- **Severity:** Medium
- **Evidence:** In [csharp.rs:462-479](crates/nx-cli/src/typegen/languages/csharp.rs#L462-L479), the `default!` fallback is guarded by `field.default_value.is_none()`. When a field has `Some(ExportedFieldDefault::Unsupported)` and is a non-nullable reference type (e.g. `title:string = { someExpr }`), `csharp_default_initializer` returns `None` (first branch skipped), the else-if is skipped because `default_value.is_none()` is false, and the field is emitted as `public string Title { get; set; }` with **no** `= default!`. Before this change the emitter always added `default!` to non-nullable reference fields, so generated DTOs will now produce CS8618 nullable warnings for any reference field with an unsupported default expression. The design (Decision 2) intends generation to "continue and warn" while keeping output valid — dropping `default!` regresses output validity. The warning test at [typegen.rs](crates/nx-cli/src/typegen.rs) only exercises a `bool` (value type) field, so this path is uncovered.
- **Recommendation:** Drop the `field.default_value.is_none()` guard (or replace with `!matches!(field.default_value, Some(ExportedFieldDefault::Literal(_)))`). Since the literal case is already handled by the first branch, the `default!` branch only needs `field_type.is_reference && !field_type.is_nullable`. Add a regression test for a reference-typed field with an unsupported default asserting `= default!;` is still emitted.
- **Fix:** Removed the `default_value.is_none()` guard so unsupported defaults still fall back to `default!` for non-nullable reference properties, and extended the unsupported-default warning test with a `string` field that asserts `= default!;` remains emitted.
- **Verification:** Confirmed at [csharp.rs:466](crates/nx-cli/src/typegen/languages/csharp.rs#L466) the else-if is now `field_type.is_reference && !field_type.is_nullable` with no `default_value` guard, so `Some(Unsupported)` reference fields correctly fall through to `= default!`. The `warns_when_csharp_literal_default_initializer_cannot_be_preserved` test now includes `title:string = { "hello" + "world" }` and asserts `public string Title { get; set; } = default!;` is emitted. Tests pass (78). Fix correct and complete.

### ✅ Verified - RF2 Nullable `f32?` float literal default is rendered without the `f` suffix, producing invalid C#
- **Severity:** Low
- **Evidence:** `csharp_float_literal` detects single-precision via exact string match `type_name == "float"` ([csharp.rs:508-510](crates/nx-cli/src/typegen/languages/csharp.rs#L508-L510)). For a nullable single `f32?`, `csharp_type` sets `text = "float?"` ([csharp.rs:640-644](crates/nx-cli/src/typegen/languages/csharp.rs#L640-L644)), so `is_float` is `false`. A default like `small:f32? = 0.5` is then rendered as `= 0.5` (double literal) on a `float?` property, which C# rejects (no implicit `double`→`float` conversion). Non-nullable `f32` works because the tested path uses exactly `"float"`.
- **Recommendation:** Detect float-ness on the unwrapped/base type rather than the display text (e.g. compare against `text.trim_end_matches('?')`, or carry a numeric-kind flag on `CSharpType`). Add coverage for nullable float/f32 defaults.
- **Fix:** Updated float literal rendering to detect `float?` by trimming the nullable suffix before checking the C# type name, and added coverage for `f32? = 0.5` emitting `0.5f`.
- **Verification:** Confirmed [csharp.rs:505-506](crates/nx-cli/src/typegen/languages/csharp.rs#L505-L506) now uses `field_type.text.trim_end_matches('?')` before the `== "float"` check, so `float?` is detected as single-precision. The record-default test adds `maybeSmall:f32? = 0.5` and asserts `public float? MaybeSmall { get; set; } = 0.5f;`. Tests pass (78). Fix correct and complete.

### ✅ Verified - RF3 External-state warning branch is dead code
- **Severity:** Low
- **Evidence:** `collect_warnings` calls `collect_unsupported_default_warnings` for `ExportedType::ExternalState` ([csharp.rs:101-103](crates/nx-cli/src/typegen/languages/csharp.rs#L101-L103)), but `export_external_state` always sets `default_value: None` ([model.rs:1030-1037](crates/nx-cli/src/typegen/model.rs#L1030-L1037)), so an external-state field can never be `Some(Unsupported)`. The branch can never fire. This matches the design Non-Goal (external state contracts intentionally don't carry defaults), so it is harmless but misleading.
- **Recommendation:** Remove the `ExternalState` arm from the match (or add a comment that external-state defaults are intentionally never populated, referencing the Non-Goal). Optional cleanup.
- **Fix:** Removed the unreachable `ExternalState` warning arm from C# warning collection.
- **Verification:** Confirmed the `collect_warnings` match at [csharp.rs:85-102](crates/nx-cli/src/typegen/languages/csharp.rs#L85-L102) now handles only `Record` and `Union`, with `_ => {}` for the rest; the `ExternalState` arm is gone. No behavior change (the branch was unreachable). Tests pass (78). Fix correct and complete.

## Questions
- None.

## Summary
The change is well-structured and matches the spec scenarios: literal defaults for records, union case fields, and external component props are emitted correctly, literals take precedence over `default!`, the string escaper was hardened, and unsupported defaults warn. One behavioral gap (RF1) can produce CS8618 warnings in generated code for reference fields with unsupported defaults — a real regression worth fixing before archiving, ideally with a covering test. RF2 and RF3 are minor edge-case/cleanup items. Spec scenarios as written are all satisfied by the implementation.
