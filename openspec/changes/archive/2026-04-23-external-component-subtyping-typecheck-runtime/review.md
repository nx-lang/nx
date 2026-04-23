# Review: external-component-subtyping-typecheck-runtime

## Scope

**Reviewed artifacts:**

- `openspec/changes/external-component-subtyping-typecheck-runtime/proposal.md`
- `openspec/changes/external-component-subtyping-typecheck-runtime/design.md`
- `openspec/changes/external-component-subtyping-typecheck-runtime/specs/external-components/spec.md`
- `openspec/changes/external-component-subtyping-typecheck-runtime/specs/braced-value-sequences/spec.md`
- `openspec/changes/external-component-subtyping-typecheck-runtime/tasks.md`

**Reviewed code (staged):**

- `crates/nx-types/src/infer.rs` (new `component_type_satisfies_expected`, `named_type_satisfies_expected`, `common_component_supertype`; integration into `type_satisfies_expected` and `common_supertype`).
- `crates/nx-interpreter/src/interpreter.rs` (new `effective_component_contract_by_name`, `component_resolution_runtime_error`, `component_type_satisfies_expected`; integration into `record_value_matches_expected_type`).
- `crates/nx-types/tests/type_checker_tests.rs` (two new positive-path tests for external-component subtyping and shared-base sequences).
- Interpreter unit tests in `crates/nx-interpreter/src/interpreter.rs` (two new positive-path tests mirroring the type checker scenarios).

## Findings

### ✅ Verified - RF1 Design doc calls record subtyping "structural" when it is nominal

- **Severity:** Medium
- **Evidence:** `design.md` §Decisions, first bullet: "Record subtyping (`is_record_subtype`) already answers 'is this named record a **structural subtype** of that named record?'". In `crates/nx-hir/src/records.rs` `is_record_subtype` resolves both names to record definitions and checks name equality or presence of `expected`'s name in the `actual`'s `ancestors` list (`effective_record_shape_resolved`). That is declared `extends` hierarchy — nominal, not structural. NX is intended to be nominal, and this phrasing mislabels existing behavior and could mislead future contributors.
- **Recommendation:** Replace "structural subtype" with "nominal subtype / declared subtype (via `extends`)" in `design.md`, and keep the rest of the decision unchanged.
- **Fix:** Reworded the rationale to describe nominal / declared `extends` subtyping; removed the incorrect “structural” label in `design.md`.
- **Verification:** `design.md` lines 35–40 now say "nominal subtype" and explicitly add "(declared `extends` chain / ancestor names, not field-shape compatibility)". No residual "structural" references in the design doc.

### ✅ Verified - RF2 Runtime subtype check accepts unverified "expected" names

- **Severity:** Medium
- **Evidence:** `crates/nx-interpreter/src/interpreter.rs:2537-2550` — the runtime `component_type_satisfies_expected` resolves only `actual` and checks `contract.ancestors.iter().any(|ancestor| ancestor == expected)`. The static counterpart in `crates/nx-types/src/infer.rs:1296-1315` resolves **both** `actual` and `expected`, then compares against `expected_contract.component.name`. If `expected` fails to resolve as a component contract (e.g., a type alias, a stale name, or a non-component binding that happens to share a string value), the runtime path may accept the value while the type checker would reject it, or at minimum accept on grounds the checker would consider unverified. Ancestor lists contain component names only in practice, so accidental matches are unlikely in-tree today, but the asymmetry weakens the "static and runtime agree" claim in the design.
- **Recommendation:** Mirror the static implementation: resolve the expected contract and compare names, returning `false` when the expected side is not a resolvable external component contract. Alternatively, resolve the expected contract only to validate, then reuse the ancestor check unchanged.
- **Fix:** `component_type_satisfies_expected` now resolves both sides via `effective_component_contract_by_name` and compares `expected_contract.component.name` to the actual contract’s name and ancestors, matching `infer.rs`.
- **Verification:** `interpreter.rs:2537-2562` resolves both contracts with `let (Ok(actual_contract), Ok(expected_contract)) = ...`, returns `false` if either lookup fails, compares `actual_contract.component.name == expected_contract.component.name` and then `actual_contract.ancestors` against `expected_contract.component.name`. This mirrors `infer.rs:1296-1315`. Full `nx-interpreter` suite passes (all prior tests plus the new negative test).

### ✅ Verified - RF3 No negative regression test for rejecting unrelated external components

- **Severity:** Medium
- **Evidence:** The new tests in `crates/nx-types/tests/type_checker_tests.rs` (`test_external_component_value_satisfies_abstract_base_type`, `test_external_component_sequence_uses_common_base_type`) and the interpreter unit tests only cover the positive case (derived values satisfy a base). Nothing demonstrates that a sibling external component that does not extend the expected base is rejected, e.g., `abstract external component <A /> external component <B extends A /> external component <C /> let x: A = <C />`. Without a negative test, a regression that accepts any external component for any base named type would pass silently.
- **Recommendation:** Add one type checker test and one interpreter test that expect a diagnostic / runtime type-mismatch when an external component that is not in the expected contract's lineage is used.
- **Fix:** Added `test_external_component_rejects_unrelated_for_abstract_base` in `type_checker_tests.rs` and `test_function_call_rejects_unrelated_external_component_for_abstract_base_param` in `interpreter.rs` (parameter coercion path). Delta spec scenarios added under `specs/external-components/spec.md`.
- **Verification:** `type_checker_tests.rs:608` asserts a type error for unrelated externals; `interpreter.rs:3968` asserts `RuntimeErrorKind::TypeMismatch` at parameter coercion. Both pass (`cargo test -p nx-types`, `cargo test -p nx-interpreter`). Matching scenarios are present in the `specs/external-components/spec.md` delta (lines 22–25 static, lines 42–46 runtime).

### 🔴 Open - RF4 Missing test for cross-module (imported) abstract external base

- **Severity:** Low
- **Evidence:** `tasks.md` 2.1 explicitly calls out "Resolve the effective external component contract by visible name (**including import targets**)", and `interpreter.rs:2485-2489` resolves `target_module` via `resolve_item` before calling `effective_component_contract_for_name`. No test exercises that import path for external components. Record-inheritance tests do cover cross-module cases; external-component subtyping does not.
- **Recommendation:** Add an integration test that places the abstract external base in one module and the derived external component in another (similar to existing `component-contract-inheritance` import scenarios), asserting the binding type-checks and evaluates.
- **Status:** Not implemented in this pass: needs a multi-module `ResolvedProgram` (or equivalent) harness for interpreter/type-check integration comparable to existing cross-module record tests; still a worthwhile follow-up.

### ✅ Verified - RF5 Ambiguous wording in `specs/external-components/spec.md` second requirement

- **Severity:** Low
- **Evidence:** The requirement says the check succeeds when the runtime component type name equals the expected name "**or appears in the effective external component ancestor list for the expected abstract base contract**". Ancestors are a property of the **actual** component's contract, not the expected's. The current phrasing inverts the relationship and could be read as "look up ancestors of the expected and see whether the actual's name is there."
- **Recommendation:** Reword to: "...or the expected name appears in the actual runtime component's effective ancestor list." Keep the rest.
- **Fix:** Rewrote the requirement paragraph so the expected side must resolve to a component contract and the ancestor check is clearly on the actual value’s effective contract.
- **Verification:** `specs/external-components/spec.md:29-33` now reads "expected name resolves to an external component contract and either the runtime component type name matches that contract’s component name or that contract’s component name appears in the actual runtime component value’s effective ancestor list." Direction is correct; matches the fixed implementation.

### ✅ Verified - RF6 Duplicated subtype helper pattern across `nx-types` and `nx-interpreter`

- **Severity:** Low
- **Evidence:** `record_type_satisfies_expected` + `component_type_satisfies_expected` + `effective_component_contract_by_name` / `effective_component_contract` pairs appear in both crates with the same semantics. Divergence (see RF2) has already appeared. The decision to keep them separated is not discussed in `design.md`.
- **Recommendation:** Either (a) extract a shared subtype helper in `nx-hir` (alongside `is_record_subtype`) that both crates can call, or (b) add a short note in `design.md` explaining why duplication is acceptable and document the invariant that the two must stay in sync. Option (a) would also resolve RF2 by construction.
- **Fix:** Chose (b): added an **Implementation note** under Risks / Trade-offs in `design.md` documenting the duplication and the invariant to keep rules aligned with `nx_hir`.
- **Verification:** `design.md:76-80` contains the Implementation note calling out `PreparedModule` vs `LoweredModule`, absence of a shared helper today, and the invariant to keep both aligned with `nx_hir::is_record_subtype` and the type checker’s `component_type_satisfies_expected`. Acceptable as option (b) under the original recommendation.

### ✅ Verified - RF7 `tasks.md` lines wrap mid-description and truncate in tooling

- **Severity:** Low
- **Evidence:** Continuation lines under each checkbox (`- [x] 1.1 ... external component contract` followed by `ancestry (`extends` chain) ...`) are rendered by `openspec instructions apply --json` as truncated task descriptions (the JSON only captures up to the soft line break). Visual review is fine but automation sees half the sentence.
- **Recommendation:** Rewrite each task on a single line (or join the wrapped segment with a trailing space such that it remains a single list-item paragraph), matching the style used in recently archived changes like `archive/2026-04-17-support-nested-and-nullable-list-types/tasks.md`.
- **Fix:** Collapsed each task in `tasks.md` to a single line per checkbox item.
- **Verification:** `tasks.md` is single-line per checkbox (lines 3–5, 9–11). `openspec instructions apply --change "external-component-subtyping-typecheck-runtime" --json` now returns the full sentence for each task description (e.g. task 1 ends with "`Type::Named` vs `Type::Named`.").

## Questions

- Should concrete-extends-concrete external component inheritance be explicitly covered here, or is that combination excluded by component-syntax rules elsewhere? If concrete-extends-concrete is disallowed today, it would be worth citing the existing spec in `design.md` Non-Goals.
- Is there an intended story for `common_component_supertype` when lineages have no shared ancestor (the current behavior falls through to `generic_common_supertype` → likely `object`)? A brief note in the braced-value-sequences delta spec would make that explicit.

## Summary

The implementation remains a small, nominal subtyping extension for external components aligned with
record `extends` chains. A follow-up fix pass corrected the design doc wording (RF1), aligned
runtime component subtyping with the static checker by resolving both contracts (RF2), added
negative type-check and interpreter coercion tests plus delta spec scenarios (RF3), clarified the
external-components spec wording (RF5), documented the interpreter/type-checker duplication
invariant (RF6), and put `tasks.md` descriptions on single lines for tooling (RF7). RF4 (cross-module
import coverage) and the two open design questions are still outstanding for a later change or pass.

**Follow-up:** RF4 and the Questions section remain open.

**Verification pass:** RF1, RF2, RF3, RF5, RF6, RF7 verified (code inspection + full `nx-types` and
`nx-interpreter` test suites pass). No new findings discovered.
