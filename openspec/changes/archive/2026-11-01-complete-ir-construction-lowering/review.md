# Review: complete-ir-construction-lowering

## Scope
**Reviewed artifacts:** proposal.md, design.md (including "Selected representation"), tasks.md, delta specs
for `source-analysis-pipeline`, `executable-code-generation`, `rust-ir-runtime`, `nx-ir-format` and
`typescript-ir-runtime`; the related main specs `nx-ir-format`, `executable-code-generation` and `rust-ir-runtime`.
**Reviewed code:** tracked working-tree changes against HEAD in `crates/nx-types` (record-call rejection,
fragment numeric patterns, intrinsic content conflict), `crates/nx-codegen` (builder, IR emitter, JS/TS emitter,
runtime helpers), `crates/nx-ir`, `crates/nx-ir-runtime`, `runtime/typescript/src`, the CLI, language-service and FFI
tests, the `nx-api` semantics/codegen tests, the TypeScript runtime tests, the docs (`docs/nx-ir-format.md`,
`docs/hir-interpreter-retirement.md`, website syntax/style pages, `specs/future.md`), the corpus manifest, the two new
corpus programs `conditional-properties` and `conditional-function-values`, and samples of the regenerated corpus
goldens. No tests were run; validation is being rebuilt separately.

## Findings

### ✅ Verified - RF1 Property lists changed from name-sorted to authored order in every image without a spec delta for the existing requirement that says they are sorted
- **Severity:** Medium
- **Evidence:** `crates/nx-codegen/src/ir.rs:1298-1311` and the builder (the `sort_by` before
  `build_union_case_for_tag` was removed) now emit all property lists in source order. As a result, every existing
  corpus image changed (e.g. `specs/ir-conformance/records/expected/main.nx.nxir.txt`, `snippet`, `large-catalog`,
  `question-flow`), and direct properties are now evaluated in authored order rather than by name. The main spec still
  requires the old order: `openspec/specs/nx-ir-format/spec.md:777-779` ("each argument as a name and a node, sorted by
  argument name as a property list is, since … a stable order keeps the artifact canonical"). None of the deltas
  modifies it, and the proposal's Impact section does not mention that all images change. After sync, the
  main spec will contradict `docs/nx-ir-format.md:152,168,1104` and the implementation.
- **Recommendation:** Add a `MODIFIED` requirement to `specs/nx-ir-format/spec.md` for "NX IR encodes a call of a
  function-typed value by name". Keep its heading and scenarios, and replace the sorting sentence with authored order.
  State in the `nx-ir-format` delta that property lists, for all modules and not only spread-bearing ones, keep authored
  order and are evaluated in that order. Mention in the proposal Impact that canonical images for unchanged sources
  change bytes.
- **Fix:** Updated the existing named-call requirement through a MODIFIED format delta; specified authored evaluation order for every property list and documented unchanged-source image byte changes and why no ABI bump is needed.
- **Verification:** The `nx-ir-format` delta now has a `MODIFIED` "NX IR encodes a call of a function-typed value by name" requirement. It keeps the heading and both main-spec scenarios and replaces sorting with authored order and evaluation. The ADDED gating requirement states that all property lists, including those in modules without spreads, keep and evaluate in authored order. The proposal Impact section explains the byte changes for unchanged sources and why no schema or ABI bump is needed. `openspec validate --strict` passes, and `docs/nx-ir-format.md` agrees in lines 152, 168 and 1104.

### ✅ Verified - RF2 New generated-JavaScript spread paths for update records, numeric patterns and intrinsic body content are never executed by a test
- **Severity:** Medium
- **Evidence:** The corpus cases that use these paths (`patch`, `numeric` and `content` in
  `specs/ir-conformance/conditional-properties/program.json`) all take arguments. The generated-JavaScript corpus
  harness runs only argument-free entrypoints (`crates/nx-api/tests/codegen/ir_corpus_tests.rs:1328`), so these cases
  reach only the two IR runtimes. The handwritten JavaScript tests
  (`crates/nx-api/tests/codegen/tests.rs:3325`, `:3370`, and the order and default tests) never use
  `<X.Update if …>`, an `if n is { > 1 => … }` fragment, or a fragment combined with body content. As a result,
  three new code paths have no execution coverage:
  - the update-record spread branch (`crates/nx-codegen/src/emit.rs:3804-3822`, `nxCleared`/`hasOwnProperty` handling)
  - the interval-pattern test generation in the bag-only `Match` emission (`emit.rs:3553-3563`)
  - intrinsic spread plus content through `nxElement`

  Task 2.5 requires explicit expected results across generated JavaScript for each target kind.
- **Recommendation:** Add generated-JavaScript (and TypeScript type-check) tests with handwritten expectations:
  - `patch(true)` gives `note: null` and `patch(false)` gives no key
  - `numeric(0)` and `numeric(2)`
  - `content(true)` and `content(false)`

  Alternatively, extend the JavaScript corpus harness to run argument cases.
- **Fix:** Added six executed Rust IR/generated-JavaScript cases with handwritten patch, numeric-pattern and body-content expectations, each also checked as generated TypeScript.
- **Verification:** `conditional_properties_preserve_updates_numeric_matches_and_content` (`crates/nx-api/tests/codegen/tests.rs:3502`) runs `patch(true/false)`, `numeric(0/2)` and `content(true/false)` on Rust IR and generated JavaScript, and type-checks the generated TypeScript. The expectations are handwritten. `assert_json_values_eq` is strict `serde_json` equality, so `"note": null` versus an absent key is checked. TypeScript IR gets the same `patch` presence check through the corpus `stableJson` comparison of `results.json`. All four targeted tests pass in `/tmp/nx-review-fix-codegen5.log`.

### ✅ Verified - RF3 Generated JavaScript still evaluates update-record properties sorted by name, diverging from the authored order the IR now uses
- **Severity:** Low
- **Evidence:** `crates/nx-codegen/src/emit.rs:3827-3828` (`emit_update_record_object`, non-spread path) still sorts
  properties by name before emitting the object literal. The builder and IR emitter now preserve authored order, and the
  spread branch above follows authored order too. The `executable-code-generation` delta requires authored evaluation
  order in Rust IR, TypeScript IR and generated JavaScript. In `<S.Update z={1 / 0} a={bad(0)} />`, the IR fails with
  division by zero and generated JavaScript fails with the constraint error.
- **Recommendation:** Remove the sort so update-record literals are emitted in authored order like the other targets.
  Add `Update` to the per-target order test in `tests.rs` (`conditional_properties_evaluate_once_in_authored_order_for_each_target`).
- **Fix:** Removed sorting from non-spread update records. The order instrumentation now covers both conditional and direct update-record construction.
- **Verification:** `emit_update_record_object`'s non-spread path no longer sorts (`emit.rs`). The order test now covers `makeUpdate` (conditional) and `makeDirectUpdate` (direct), with expected `[1,2,3,4]` and `[1,2,3]`, and passes.

### ✅ Resolved - RF4 No test distinguishes omission from an explicitly empty binding for a defaulted field
- **Severity:** Low
- **Evidence:** The `executable-code-generation` scenario "An omitted property retains its default" requires an
  explicitly empty binding to keep its distinct semantics. Every record case uses `note?:string` with no default
  (`conditional-properties/main.nx:1`, `runtime/typescript/test/emitted-ir.test.mjs` addition, `tests.rs:3325`). For
  that field, omission and `note={}` produce the same output `{ "$type": "User", "name": "Default" }`. An implementation
  that dropped empty bag fields, or applied the default when the selected value is empty, would still pass. Only the
  update-record `patch` case checks presence, and only on the IR runtimes (see RF2).
- **Recommendation:** Add a case with a defaulted field that admits empty (e.g. `tags:string* = {"a"}`) where one
  branch binds `tags={}` and the other omits it. Assert the expected outputs (empty versus default) on Rust IR,
  TypeScript IR and generated JavaScript.
- **Status:** The suggested record declaration is invalid under existing source rules: `tags: string*` reports `optional-in-type-slot`, and `tags?: string+ = { "a" }` reports `optional-property-with-default`. A defaulted required property cannot accept an explicit empty binding; a new checker regression covers rejection. The RF2 update-record cases now verify empty versus absent key presence across generated JavaScript and Rust IR as well as the existing TypeScript corpus. Please reassess whether RF4 is a valid source scenario rather than changing these language rules.
- **Verification:** Resolved as not reachable from source. A property that admits zero cannot also have a default: `tags:string*` reports `optional-in-type-slot` (`crates/nx-types/src/infer.rs:7514`), and `tags?:T = …` reports `optional-property-with-default` (`crates/nx-syntax/src/validation.rs:1215`). Explicit empty is therefore only legal on optional fields that have no default. There, omission and `{}` have the same canonical result for construction, an absent key, so there is no observable difference to test. The one place they differ is update records (clear versus leave unchanged). `patch(true/false)` now checks that difference with strict key presence on Rust IR, generated JavaScript and TypeScript IR (corpus). The new checker regression `defaulted_required_properties_reject_explicit_empty_conditional_bindings` pins `record-field-type-mismatch` for `{}` at a defaulted required field. No language-rule change is needed.

### ✅ Verified - RF5 The codegen intrinsic-content-conflict check is now an unreachable duplicate of the new checker diagnostic
- **Severity:** Low
- **Evidence:** `crates/nx-types/src/infer.rs:4176-4186` now reports `intrinsic-content-conflict` during source
  analysis for any property path, direct or conditional, that binds `content` on an unresolved (intrinsic) tag with
  body content. Codegen runs only on checked programs, so the builder's check
  (`crates/nx-codegen/src/builder.rs:2558-2572`) can no longer fire. It also inspects only `property.name`, so it
  would miss spread entries whose name is empty. The new checker test covers only the conditional case.
- **Recommendation:** Delete the builder check and keep the single checker rule. Extend
  `intrinsic_conditional_content_cannot_conflict_with_body_content` with the direct `<box content=1><child /></box>`
  case, so the rule move is covered.
- **Fix:** Removed the redundant builder diagnostic and covered direct and conditional content conflicts in the shared checker.
- **Verification:** The builder no longer emits `intrinsic-content-conflict`; the only producer is `crates/nx-types/src/infer.rs:4183`. `intrinsic_content_cannot_conflict_with_body_content` covers the direct and conditional cases and passes in `/tmp/nx-review-fix-checker2.log`.

### ✅ Verified - RF6 The linear-growth test compares only two sizes of flat `if` fragments, so fixed overhead can hide superlinear growth and nested fragments are untested
- **Severity:** Low
- **Evidence:** `crates/nx-api/tests/codegen/tests.rs:3370-3392` asserts `size(8) <= 2 * size(4)` on total image
  and module size, which includes fixed module overhead. It only uses independent `if … else …` fragments. The
  `nx-ir-format` delta scenario "Nested fragments do not multiply constructions" and task 2.5 call out nested fragments
  explicitly.
- **Recommendation:** Compare increments instead (e.g. `size(16) - size(8)` ≈ `2 * (size(8) - size(4))` within a
  small tolerance). Add a nested-fragment series and a condition-list or match series.
- **Fix:** Compared equal-sized size increments at 4, 8 and 12 fragments, covering independent, nested and condition-list shapes with only 64 bytes of tolerance.
- **Verification:** The test now compares equal increments at 4, 8 and 12 fragments for independent, nested and condition-list shapes, on both the IR and generated-JavaScript sizes, with a 64-byte tolerance. Quadratic growth would fail unless it adds under about 2 bytes per unit, and Cartesian growth would fail by orders of magnitude. The test passes.

### ✅ Verified - RF7 The "Required features" section of the IR format doc does not list `conditional-properties-v1`
- **Severity:** Low
- **Evidence:** `docs/nx-ir-format.md:507-521` enumerates when each feature is listed. The new feature appears only in
  the property-list paragraph (`:168-175`), so the section that claims to be the full list is now incomplete.
- **Recommendation:** Add a clause such as: "one whose property list contains a `$spread` entry lists
  `conditional-properties-v1`".
- **Fix:** Listed conditional-properties-v1 in the Required features section.
- **Verification:** The "Required features" section now begins with the `$spread` → `conditional-properties-v1` clause.

### ✅ Verified - RF8 The executable-code-generation delta duplicates the IR feature-gating requirement owned by the format and runtime deltas
- **Severity:** Low
- **Evidence:** `specs/executable-code-generation/spec.md` adds "Conditional property lists use a versioned
  representation" (encoding, reader validation, both runtimes' preparation and old-runtime refusal). The same
  obligations are in the `nx-ir-format` delta ("Conditional property spreads are feature gated") and the
  `rust-ir-runtime` and `typescript-ir-runtime` deltas. After sync, the encoding would be specified in two capabilities
  that can drift apart.
- **Recommendation:** Move the two scenarios (older runtime refuses by name, spread without feature is malformed)
  into the `nx-ir-format` delta. Drop the requirement from `executable-code-generation`, which should keep only
  source-level execution behavior.
- **Fix:** Moved feature refusal and malformed-spread scenarios into the format delta and removed the duplicated encoding requirement from executable-code-generation.
- **Verification:** The duplicate requirement was removed from `executable-code-generation`. Both refusal scenarios are now in the `nx-ir-format` ADDED requirement, and strict validation passes.

### ✅ Verified - RF9 The new runtime helpers were inserted between `nxIsEmpty`'s doc comment and its function
- **Severity:** Low
- **Evidence:** In `crates/nx-codegen/src/runtime.rs:138-157`, the block comment "The empty value is the empty
  sequence…" now sits directly above `/** The fields selected… */ export function nxBindings`, followed by
  `nxPatternMatches`, and `nxIsEmpty` has no comment. The JavaScript copy (`:823-841`) has the same misplacement.
- **Recommendation:** Move `nxBindings` and `nxPatternMatches` above that comment, or after `nxIsEmpty`, in both
  helper copies.
- **Fix:** Moved the empty-sequence documentation immediately above nxIsEmpty in both helper copies.
- **Verification:** `nxBindings` and `nxPatternMatches` now come before the empty-value comment, and the comment sits directly above `nxIsEmpty` in `runtime.rs`.

### ✅ Verified - RF10 The new corpus source does not follow the NX style guide's annotation spacing
- **Severity:** Low
- **Evidence:** `specs/ir-conformance/conditional-properties/main.nx:60` declares
  `external component <SequenceCard values:int+ />`. The rest of the file and the style guide use `name: type`.
- **Recommendation:** Write `values: int+` and regenerate the affected goldens. Only spans and text should change.
- **Fix:** Fixed values: int+ spacing and regenerated the affected corpus images, explanations and origin snapshots; the non-updating corpus suite passes.
- **Verification:** `main.nx:60` now reads `values: int+`. The regenerated goldens pass the non-updating corpus suite, including explained text, images, origins and pinned artifacts, in `/tmp/nx-review-fix-corpus2.log`.

## Questions
- None. The ABI question is answered in the proposal Impact section (RF1). The corpus log shows `conditional-properties` is not in the generated-JavaScript skip list; only `conditional-function-values` is skipped, as an expected function-typed named call.

## Summary
- The core design is implemented coherently across the checker, builder, IR emitter, both IR runtimes and generated
  JavaScript:
  - record calls are rejected semantically
  - fragments use lazy `if`/`ifIs` bags spread through `$spread`
  - the feature is gated in both readers
  - the six ignores are gone
  - docs are updated
- Positional-record rejection coverage is good: imports, aliases, inheritance, generics, shadowing, the language
  service, the CLI and the single-diagnostic guarantees all have tests.
- The main gaps are an unreconciled main-spec requirement for the global property-order change (RF1) and missing
  generated-JavaScript execution coverage for several new emitter branches (RF2). The rest are low-severity
  consistency, test-strength and hygiene items.

## Verification Summary (2026-11-01)
- Verified RF1–RF3 and RF5–RF10. Resolved RF4 as not a reachable source scenario. No findings were reopened and no
  new findings were found.
- This pass inspected code, specs and logs only; no builds were launched. Evidence comes from
  `/tmp/nx-review-fix-{codegen5,checker2,corpus2,ts3,clippy}.log`. In the corpus log, 108 JavaScript entrypoints were
  compared and `conditional-properties` was not skipped.
- The TypeScript rerun log covers the runtime and emitted-IR suites, not `corpus.test.mjs`. Before archive, run the
  TypeScript corpus suite once more against the RF10-regenerated `conditional-properties` goldens (e.g. `npm test` in
  `runtime/typescript`), unless the earlier full run already used them.
