# Review: support-content-properties

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/content-properties/spec.md, tasks.md  
**Reviewed code:** crates/nx-hir/src/lib.rs, crates/nx-hir/src/lower.rs, crates/nx-hir/src/records.rs, crates/nx-types/src/check.rs, crates/nx-types/src/infer.rs, crates/nx-interpreter/src/interpreter.rs, crates/nx-api/src/artifacts.rs, crates/nx-syntax/grammar.js, crates/nx-syntax/queries/highlights.scm, crates/nx-syntax/tests/parser_tests.rs, crates/nx-interpreter/tests/simple_functions.rs, docs (6 doc files), nx-grammar-spec.md, nx-grammar.md, src/vscode/samples/basic.nx

## Findings

### ✅ Verified - RF1 Incomplete `children` → `content` terminology rename leaves struct field and internal methods using old name
- **Severity:** Medium
- **Evidence:** The design decision #6 states "Replace `children`-based implementation terminology for element body handling with `content` terminology throughout the affected compiler and runtime layers." However, the `Element` struct field is still named `children` at crates/nx-hir/src/lib.rs:421, and several internal methods retain `children`-based names:
  - `Element::children` field (crates/nx-hir/src/lib.rs:421)
  - `eval_child_expressions` method (crates/nx-interpreter/src/interpreter.rs:1714)
  - `normalize_child_values` method (crates/nx-interpreter/src/interpreter.rs:1733)
  - `lower_sequence_expr_from_children` method (crates/nx-hir/src/lower.rs:322)
  - Associated parameters (`child_exprs`, `child_values`) and comments referencing "child"
  - Field access sites in crates/nx-types/src/infer.rs:738, :754 and crates/nx-api/src/artifacts.rs:710-711
- **Recommendation:** Rename `Element::children` to `Element::content`, rename the three internal methods and their parameters/comments to use `content` terminology, and update all downstream field access sites. This is a mechanical rename with no behavioral risk.
- **Fix:** Renamed the HIR field to `Element::content`, renamed the interpreter/lowering helpers away from `children` terminology, and updated downstream type-checker/API access sites to use `content`.
- **Verification:** Confirmed `Element::content` field at lib.rs:421, `eval_content_expressions` at interpreter.rs:1717, `normalize_content_values` at interpreter.rs:1736, `lower_sequence_expr_from_items` at lower.rs:322, and all downstream access sites in infer.rs and artifacts.rs. Zero leftover `children` references remain outside tree-sitter API calls.

### ✅ Verified - RF2 No test for same-declaration duplicate content property diagnostic
- **Severity:** Medium
- **Evidence:** The spec scenario "Declaration cannot mark two content properties" (spec.md) requires that `type Foo = { content title:string content body:string }` is rejected. The lowering code at crates/nx-hir/src/lower.rs:448-454 does emit a diagnostic for this case, and the paren-style path at :1447-1453 does too. However, there is no test that exercises this path — `test_lower_record_inheritance_duplicate_content_property_diagnostic` only covers the cross-inheritance case (crates/nx-hir/src/lower.rs:2315). The within-declaration duplicate path is untested.
- **Recommendation:** Add a lowering test with source like `type Foo = { content title:string content body:string }` that asserts the "Only one content property is allowed" diagnostic is emitted. Also consider a test for the paren-style function equivalent: `let wrap(content a:string, content b:string) = a`.
- **Fix:** Added lowering coverage for the same-declaration record case so the spec-mandated duplicate-content diagnostic is now exercised directly.
- **Verification:** Confirmed `test_lower_duplicate_content_property_in_single_record_diagnostic` at lower.rs:2144 uses `{ content title:string content body:string }` and asserts "Only one content property is allowed". Test passes. No paren-function equivalent test was added, but the record test pins the core diagnostic path adequately.

### ✅ Verified - RF3 Case-insensitive `Element` type comparison lacks justification and risks accepting malformed type names
- **Severity:** Low
- **Evidence:** `named_type_is_element_like` at crates/nx-types/src/infer.rs:1069 uses `eq_ignore_ascii_case("element")`, and `type_satisfies_expected` at :1092 does the same. This means `element`, `Element`, `ELEMENT`, `eLeMeNt` are all treated identically as the element supertype. The NX language spec and design doc do not mention case-insensitive type resolution. The rest of the type system uses exact name matching for records, functions, enums, etc.
- **Recommendation:** Either document the intended case-insensitivity for `Element` with a comment explaining the rationale, or switch to exact case matching (`"Element"`) to be consistent with the rest of the type system. If the language intends `element` and `Element` to both be valid, that's fine — just add a comment.
- **Fix:** Switched the special-case `Element` supertype checks to exact-case matching and added an inference unit test that distinguishes `Element` from lowercase `element`.
- **Verification:** Confirmed zero `eq_ignore_ascii_case` calls remain in infer.rs. Exact comparisons `== "Element"` at lines 1071 and 1094. Unit test `test_element_supertype_requires_exact_case` at infer.rs:1218 asserts `div` satisfies `Element` but not `element`. Test passes.

### ✅ Verified - RF4 Intrinsic element content injection unconditionally uses hardcoded `"content"` key without checking for name collision
- **Severity:** Low
- **Evidence:** The intrinsic/native element fallback path at crates/nx-interpreter/src/interpreter.rs:1658-1665 always injects body content under `Some("content")`. If an intrinsic element also has a named property called `content` passed explicitly (`<div content="foo">bar</div>`), `inject_element_content_field` would return an error because `fields.contains_key("content")` would be true. This is likely the desired behavior (consistent with the NX-defined path), but there is no test exercising this intrinsic-element double-supply scenario.
- **Recommendation:** Add a test that verifies `<div content="foo">bar</div>` produces the expected content-binding-conflict error at runtime, to pin the intrinsic behavior explicitly.
- **Fix:** Added an interpreter test that asserts intrinsic elements reject simultaneous named `content` input and body content with the expected runtime conflict.
- **Verification:** Confirmed `test_intrinsic_element_named_and_body_content_conflict_is_rejected` at simple_functions.rs:924 uses `<div content="named">body</div>` and asserts error contains "both a 'content' property and element body content". Test passes.

### ✅ Verified - RF5 `mixed_content` grammar rule's text-run regex may reject valid leading characters
- **Severity:** Low
- **Evidence:** The `_mixed_text_run` token at crates/nx-syntax/grammar.js:569 uses a complex regex that explicitly excludes text runs starting with `i` (if), `f` (for), `e` (else) unless followed by certain patterns. While this is necessary to avoid ambiguity with control flow keywords in element bodies, the regex `[^\s<{ife]` as the first character class excludes ALL of `i`, `f`, and `e` as single-character text runs. A text body like `<Foo>i</Foo>` or `<Foo>f</Foo>` would not be parsed as a text run. This may be acceptable since single-character text is an edge case, but it's undocumented.
- **Recommendation:** Add a comment in grammar.js explaining the text-run keyword exclusion trade-off. Consider whether single-letter text content like `<Label>i</Label>` needs to work (it could be written as `<Label>{"i"}</Label>` as a workaround).
- **Fix:** Documented the mixed-content keyword-prefix trade-off inline in `grammar.js`, including the brace-based escape hatch for ambiguous text.
- **Verification:** Confirmed comment at grammar.js:568-569: "Keep bare if/for/else prefixes available for control-flow items in mixed content. Text that would otherwise collide with those prefixes can still be written via braces." Trade-off and workaround are documented.

## Questions
- None

## Summary
- The implementation remains functionally complete and the review follow-ups are now applied. Targeted tests pass across `nx-hir`, `nx-types`, `nx-interpreter`, and `nx-api`, and the `nx-syntax` follow-up is documentation-only.
- The `children` → `content` terminology gap is addressed in HIR, lowering, interpreter helpers, and downstream metadata/type-checking access sites.
- Same-declaration duplicate-content coverage and intrinsic content-conflict runtime coverage are now pinned, and `Element` supertype matching now requires exact-case `Element`.
- All 5 findings verified on 2026-04-06. Full test suite passes across nx-hir, nx-types, nx-interpreter, nx-syntax, and nx-api.
