# Review: add-discriminated-unions

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/discriminated-unions/spec.md, specs/runtime-output-format/spec.md, specs/cli-code-generation/spec.md, specs/dotnet-binding/spec.md
**Reviewed code:**
- `crates/nx-syntax/grammar.js`, `src/ast.rs`, `src/validation.rs`, parser fixtures, `tests/parser_tests.rs`, snapshots
- `crates/nx-hir/src/{lib.rs, lower.rs, prepared.rs, components.rs, scope.rs, unions.rs}`
- `crates/nx-types/src/{ty.rs, infer.rs, check.rs}`, `tests/type_checker_tests.rs`
- `crates/nx-interpreter/src/interpreter.rs`
- `crates/nx-api/src/{artifacts.rs, eval.rs}`
- `crates/nx-cli/src/codegen.rs`, `src/codegen/model.rs`, `src/codegen/languages/{typescript.rs, csharp.rs}`
- `bindings/dotnet/tests/NxLang.Runtime.Tests/NxUnionSerializationTests.cs`, `bindings/dotnet/README.md`
- `src/vscode/syntaxes/nx.tmLanguage.json`, `snippets/nx.json`, grammar tests
- `docs/src/content/docs/language-tour/{types.md, expressions.md}`, `docs/src/content/docs/reference/syntax/{types.md, expressions.md, if.md}`
- `examples/nx/types.nx`, `nx-grammar.md`, `nx-grammar-spec.md`

## Findings

### ✅ Verified - RF1 Abstract record bases miss union-case descendants in generated TS/C# polymorphism
- **Severity:** Medium
- **Evidence:** `crates/nx-cli/src/codegen/model.rs:417` — `collect_concrete_descendant_names` walks only `ExportedType::Record` entries via `records()` (line 435) and ignores `ExportedType::Union`. As a result, when a host serializes/deserializes through the abstract base type `EventBase` for the spec's `abstract type EventBase = { source:string } / type UiEvent extends EventBase = | clicked { x:int } | closed` example, the generated `EventBase` polymorphism dispatch contains no `JsonDerivedType(typeof(UiEventClicked), ...)` entries (`crates/nx-cli/src/codegen/languages/csharp.rs:387-394`) and the TS surface `export type EventBase = ...descendants...` (`typescript.rs:265-280`) likewise omits the union case members. The generated `UiEvent` itself dispatches correctly because `emit_union` writes its own `JsonDerivedType` list (csharp.rs:312-319), but mixed record-and-union hierarchies cannot round-trip through the shared base.
- **Recommendation:** Extend `collect_concrete_descendant_names` (and the TS abstract-record runtime-surface emitter) to also include union case classes for any `ExportedUnion` whose `base` resolves to the queried abstract record. The TS side should add each `Union<X>Case<Y>` interface name to the descendant list; C# should add a `JsonDerivedType(typeof(<UnionCaseClass>), "<Union>.<case>")` entry for each case (and a corresponding MessagePack derived-formatter entry where applicable).
- **Fix:** Added `ExportedPolymorphicDescendant` traversal for abstract records, including union cases whose union base resolves through the record hierarchy. TypeScript abstract record surfaces now include union case interfaces, and C# abstract record metadata now includes union case `JsonDerivedType` registrations and root MessagePack formatter coverage. Codegen tests assert `EventBase` includes `UiEvent.clicked` and `UiEvent.closed` descendants.
- **Verification:** Confirmed `crates/nx-cli/src/codegen/model.rs:286-315` walks both records and unions whose base resolves to the queried abstract via `union_extends_record` (climbs the record chain). `crates/nx-cli/src/codegen/languages/csharp.rs:380-396` and `:729-744` emit `JsonDerivedType(typeof(UiEventClicked), "UiEvent.clicked")` style entries for union cases. `crates/nx-cli/src/codegen/languages/typescript.rs:266-281` joins union case interface names into the abstract record's runtime surface. Tests `generates_typescript_discriminated_unions` and `generates_csharp_discriminated_unions` (codegen.rs:327, :699) assert `EventBase = UiEventClicked | UiEventClosed` and the C# `JsonDerivedType` block. `cargo test -p nx-cli discriminated_unions` passes.

### ✅ Verified - RF2 Duplicate-case diagnostics emitted twice (once by syntax, once by HIR)
- **Severity:** Low
- **Evidence:** `crates/nx-syntax/src/validation.rs:207-256` produces a `Diagnostic` with code `duplicate-union-case` per duplicate. `crates/nx-hir/src/unions.rs:189-200` produces another `UnionValidationError::DuplicateCase` (code `union-duplicate-case`) for the same duplicates, which `crates/nx-types/src/check.rs:140-145` flushes into the artifact diagnostics. End-to-end pipelines that run both stages report two messages with different codes/wording for the same underlying mistake (e.g. `type LoadState = | idle | idle`).
- **Recommendation:** Pick a single source of truth. Either (a) drop the duplicate-case check from `nx-syntax/src/validation.rs::validate_union_definitions` and let HIR own it (so case identity uses the same `Name` semantics as the rest of HIR validation), or (b) keep the syntax check but suppress the HIR duplicate when the syntax tree already produced a `duplicate-union-case` diagnostic.
- **Fix:** Kept the syntax diagnostic as the public parse-stage signal and suppressed the HIR duplicate-case validation error when `duplicate-union-case` is already present. Added an end-to-end type-checker regression test that verifies only the syntax diagnostic remains.
- **Verification:** `crates/nx-types/src/check.rs:140-152` checks for an existing `duplicate-union-case` diagnostic and skips HIR `union-duplicate-case` errors when present. Test `test_duplicate_union_case_syntax_diagnostic_suppresses_hir_duplicate` (`crates/nx-types/tests/type_checker_tests.rs:276`) asserts exactly one duplicate diagnostic with code `duplicate-union-case`. `cargo test -p nx-types union` passes.

### ✅ Verified - RF3 `Type::is_compatible_with` ignores union-base-record subtyping for `==`/`!=`
- **Severity:** Low
- **Evidence:** `crates/nx-types/src/ty.rs:368-370` only adds `(Type::UnionCase, Type::Union)` compatibility. `infer.rs::type_satisfies_expected` (1845-1872) extends this with `(UnionCase, Named(base))` and `(Union, Named(base))` rules, but `Eq/Ne/Lt/...` in `infer_binop` (`infer.rs:644-655`) call `lhs.is_compatible_with(rhs)` from the generic `ty.rs` implementation, which has no access to `union_defs`. A comparison like `if event == defaultEvent { ... }` where `event: UiEvent` and `defaultEvent: EventBase` would therefore be rejected with `type-mismatch` even though the values are structurally compatible (and the same comparison works for plain abstract-record subtyping because both sides are `Type::Named`).
- **Recommendation:** Route equality/comparison compatibility through `InferenceContext::type_satisfies_expected` (or factor a context-aware compatibility helper) so union-extends-record subtyping participates in `==`/`!=` checks. Add a regression test under `crates/nx-types/tests/type_checker_tests.rs` covering `UnionCase == Named(base)` and `Union == Named(base)`.
- **Fix:** Routed comparison compatibility through `InferenceContext::type_satisfies_expected` in both directions. Added a regression test covering both `UiEvent == EventBase` and `UiEvent.clicked != EventBase`.
- **Verification:** `crates/nx-types/src/infer.rs:644-656` now uses `self.type_satisfies_expected(lhs, rhs) || self.type_satisfies_expected(rhs, lhs)` for `Eq | Ne | Lt | Le | Gt | Ge`, so the union-extends-record subtyping rules in `type_satisfies_expected` apply to comparisons. Test `test_union_cases_compare_with_extended_abstract_record` (`crates/nx-types/tests/type_checker_tests.rs:259`) covers both directions. `cargo test -p nx-types union` passes.

### ✅ Verified - RF4 Missing-leading-pipe diagnostic falls back to generic "Syntax error"
- **Severity:** Low
- **Evidence:** `crates/nx-syntax/tests/fixtures/invalid/union-missing-leading-pipe.nx` (`type LoadState = idle | loading`) parses `type LoadState = idle` as a valid `type_definition`; the unexpected `|` then produces an ERROR node whose text is roughly `| loading`. The hint in `crates/nx-syntax/src/validation.rs:674-679` is gated on `trimmed_error.starts_with("type ")`, so it never fires for this case, and the test (`tests/parser_tests.rs:2059-2072`) only checks `!result.is_ok()` without asserting on diagnostic quality. Authors miswriting a union therefore get a generic "Syntax error" instead of the `UNION_DEFINITION_SYNTAX` hint that already exists in the codebase.
- **Recommendation:** Detect the pattern from the parent context instead of the trimmed error text. One option: if the ERROR node's previous sibling token is a `=` whose enclosing `type_definition` (or its surrounding module item) is followed by `|`, emit the union hint. Alternatively, lower the bar in `analyze_error_context` so that any error whose immediate surrounding text contains `type ... = ... |` triggers `UNION_DEFINITION_SYNTAX`.
- **Fix:** Extended syntax error context analysis to detect an unexpected leading `|` after a `type ... =` prefix on the same line. The parser fixture test now asserts the union-specific message and syntax hint.
- **Verification:** `crates/nx-syntax/src/validation.rs:674-681` matches both `type ... |` patterns and a leading `|` whose preceding line text starts with `type ` and contains `=` (helper `looks_like_type_definition_prefix`, line 712). Test `test_parse_union_definition_requires_leading_pipe` (`crates/nx-syntax/tests/parser_tests.rs:2059`) asserts the message contains "Invalid discriminated union definition" and the note contains "Expected: type UnionName". `cargo test -p nx-syntax union` passes.

### ✅ Verified - RF5 No regression test for "union cannot extend a union" (spec scenario uncovered)
- **Severity:** Low
- **Evidence:** `specs/discriminated-unions/spec.md` requires (Requirement: "Union case values are compatible with their owning union", scenario "Union cannot be extended after declaration") that `type LoadState = | idle  type MoreLoadState extends LoadState = | failed { message:string }` be rejected. The HIR check in `crates/nx-hir/src/unions.rs::resolve_union_base_record` correctly returns `Err(InvalidUnionBaseReason::NotRecord)` when the base resolves to `Item::Union`, but `crates/nx-hir/src/lower.rs` only tests the abstract-vs-concrete record case (`test_validate_union_base_must_be_abstract_record`, line 2429). There is no test that exercises `extends <union>`.
- **Recommendation:** Add a HIR test alongside `test_validate_union_base_must_be_abstract_record` that lowers `type A = | idle  type B extends A = | failed { message:string }` and asserts the validation error contains "could not be resolved" or "does not resolve to an abstract record" so future refactors of `resolve_union_base_record` cannot silently regress this rule.
- **Fix:** Added a HIR regression test that lowers `type MoreLoadState extends LoadState = ...` where `LoadState` is itself a union and asserts the validation error says the base does not resolve to an abstract record.
- **Verification:** `test_validate_union_base_cannot_be_union` (`crates/nx-hir/src/lower.rs:2450`) lowers `type MoreLoadState extends LoadState = | failed { message:string }` and asserts a diagnostic containing "does not resolve to an abstract record". `cargo test -p nx-hir union` passes.

### ✅ Verified - RF6 Duplicate content-property diagnostics for cases that name the base's content field
- **Severity:** Low
- **Evidence:** When a case payload field shares both a name and `is_content` with a base content field, `validate_case_inherited_field_collisions` (`crates/nx-hir/src/unions.rs:225-260`) fires both `DuplicateInheritedField` (line 238) and `DuplicateContentProperty` (line 249). The second branch is unreachable in practice without the first also firing, so users get two diagnostics for what is logically one mistake.
- **Recommendation:** Skip the `DuplicateContentProperty` arm when the same field already triggered `DuplicateInheritedField` in this iteration (e.g., short-circuit with `continue` after pushing the inherited-field error, or only check content collisions for fields whose name differs from any inherited field).
- **Fix:** Short-circuited inherited field collisions before content-property checks for the same case field. Added a HIR regression test that verifies an inherited content field collision reports exactly one inherited-field diagnostic and no duplicate content-property diagnostic.
- **Verification:** `crates/nx-hir/src/unions.rs:237-246` now `continue`s after pushing `DuplicateInheritedField`, preventing the subsequent `DuplicateContentProperty` branch from firing for the same field. Test `test_validate_union_inherited_content_field_collision_reports_once` (`crates/nx-hir/src/lower.rs:2492`) asserts exactly one inherited-field diagnostic and zero content-property diagnostics. `cargo test -p nx-hir union` passes.

### 🔴 Open - RF7 Property-list match expressions are not lowered to `Expr::Match`
- **Severity:** Low
- **Evidence:** `crates/nx-hir/src/lower.rs:1289` only matches `VALUE_IF_MATCH_EXPRESSION | ELEMENTS_IF_MATCH_EXPRESSION` when building `Expr::Match`. The grammar also defines `property_list_if_match_expression` (`crates/nx-syntax/grammar.js:822-834`), but no HIR handler exists for it (verified: `grep -n "PROPERTY_LIST_IF_MATCH" crates/nx-hir/` returns nothing). This is pre-existing behavior, but it now means union narrowing inside element property lists (`<Foo if state is { LoadState.failed => message=... }`) silently falls through to the catch-all error branch instead of getting union-aware narrowing/exhaustiveness.
- **Recommendation:** Either (a) document the limitation explicitly in design.md / a follow-up so future authors know property-list match arms do not narrow, or (b) add a lowering arm for `PROPERTY_LIST_IF_MATCH_EXPRESSION` that builds the same `Expr::Match` shape as the value/element variants. (a) is acceptable for this change since the spec scenarios do not exercise property-list matches, but the gap should be acknowledged.
- **Status:** Left open. This is a pre-existing, broader lowering gap outside the spec scenarios exercised by this change; it should be handled as a follow-up rather than folded into the review-fix patch.

## Questions
- Is RF1 (abstract-base polymorphism) considered out of scope for this change because the spec scenarios only require dispatch through the union root, or is the omission unintentional? The runtime serialization is fine when the host uses `LoadState`/`UiEvent` as the target type, but using the abstract record base will fail to deserialize union cases.
- For RF2, do we want both diagnostic codes (`duplicate-union-case` from syntax, `union-duplicate-case` from HIR) to remain in the public surface, or should they be unified? Choosing one will avoid future churn for tooling consumers that key off the codes.
- **Resolution note:** RF1 was treated as an in-scope consistency gap and fixed. RF2 keeps `duplicate-union-case` as the public parse-stage diagnostic and suppresses the HIR duplicate in end-to-end analysis.

## Summary
The change implements discriminated unions cleanly across all eight task groups. Parser, HIR, prepared metadata, type checker, interpreter, runtime serialization, code generation, .NET binding, VS Code grammar, and docs all line up with the spec scenarios, and the test coverage at each layer is solid (especially `crates/nx-types/tests/type_checker_tests.rs` and `bindings/dotnet/tests/NxLang.Runtime.Tests/NxUnionSerializationTests.cs`).

The review-fix pass addressed RF1 through RF6. RF7 remains open as a follow-up because property-list match lowering is a broader pre-existing gap outside the exercised spec scenarios.

## Fix Results
- **Fixed:** RF1, RF2, RF3, RF4, RF5, RF6
- **Open:** RF7 remains open as a follow-up for property-list match lowering/documentation.
- **Verification:** `cargo test -p nx-cli discriminated_unions`; `cargo test -p nx-types union`; `cargo test -p nx-hir union`; `cargo test -p nx-syntax union`

## Verification Results - 2026-04-26 15:40
- **Verified fixed:** RF1, RF2, RF3, RF4, RF5, RF6
- **Reopened:** none
- **New findings:** none
- **Tests run:** `cargo test -p nx-cli discriminated_unions` (2 passed), `cargo test -p nx-types union` (16 passed), `cargo test -p nx-hir union` (7 passed), `cargo test -p nx-syntax union` (5 passed). RF7 remains intentionally open as a follow-up.
