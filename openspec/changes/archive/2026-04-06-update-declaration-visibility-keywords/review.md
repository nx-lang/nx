# Review: update-declaration-visibility-keywords

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/declaration-visibility/spec.md, tasks.md  
**Reviewed code:** crates/nx-syntax/ (grammar.js, syntax_kind.rs, syntax_node.rs, highlights.scm, parser_tests.rs, fixtures), crates/nx-hir/ (lib.rs, lower.rs, scope.rs), crates/nx-api/src/ (artifacts.rs, component.rs, eval.rs), crates/nx-interpreter/tests/ (edge_cases.rs, error_handling.rs, floats.rs, interpreter_direct_hir.rs, loops.rs, recursion.rs), crates/nx-types/src/infer.rs, src/vscode/ (nx.tmLanguage.json, basic.test.ts), docs/src/content/docs/, nx-grammar.md, nx-grammar-spec.md  

## Findings

### ✅ Verified - RF1 nx-types test code still references removed Visibility::Public variant
- **Severity:** High
- **Evidence:** `crates/nx-types/src/infer.rs` contains 11 occurrences of `nx_hir::Visibility::Public` (lines 1165, 1196, 1245, 1267, 1292, 1332, 1373, 1402, 1412, 1441, 1458). The `Visibility` enum no longer has a `Public` variant — it was renamed to `Export`. Running `cargo check --package nx-types --tests` produces 11 compilation errors. The non-test library code compiles because `cargo check` without `--tests` skips test modules, which masked the breakage.
- **Recommendation:** Replace all 11 `nx_hir::Visibility::Public` references in `crates/nx-types/src/infer.rs` with `nx_hir::Visibility::Export` (or `Internal` where the test intent is to exercise default visibility).
- **Fix:** Replaced all 11 stale `Visibility::Public` test references in `crates/nx-types/src/infer.rs` with `Visibility::Export`, restoring `nx-types` test-target compilation.
- **Verification:** All 11 `Visibility::Public` references confirmed replaced with `Visibility::Export`. Zero stale references remain. `cargo check --package nx-types --tests` compiles cleanly.

### 🔴 Open - RF2 Spec scenario for multi-root-module program visibility is unimplemented
- **Severity:** Medium
- **Evidence:** The spec (`specs/declaration-visibility/spec.md`) requires: "Other root modules in the same program can resolve a default declaration — WHEN a non-library program includes `helpers.nx` containing `let answer() = 42` AND `main.nx` references `answer()` THEN analysis SHALL resolve `answer()` successfully." However, `build_program_artifact_from_source` in `crates/nx-api/src/artifacts.rs` always creates a single root module (`let root_modules = vec![root_artifact];` at line 1032). The library-local peer resolution loop at line 1610 only applies to library modules, not root modules. There is no mechanism for cross-root-module resolution in non-library programs.
- **Recommendation:** This is an infrastructure gap — multi-root-module programs are not yet supported. Either implement multi-root resolution for programs, or update the spec to defer this scenario to a future change. The follow-up is now tracked in `specs/future.md` under **Manifest-Rooted Packages**, which proposes `package.nx` with `kind: app | library` as the broader design needed to make this case implementable cleanly.
- **Status:** Left open. This review-fix pass addressed only high-confidence, localized fixes; multi-root non-library program support needs the broader package/artifact-model work now noted in `specs/future.md`.

### ✅ Verified - RF3 Stale doc comments reference "Public" visibility
- **Severity:** Low
- **Evidence:** Two doc comments still say "Public" in a visibility-adjacent context: `crates/nx-hir/src/lib.rs:323` ("Public action type name") and `crates/nx-hir/src/ast/expr.rs:229` ("Public action type name expected at invocation time"). While these refer to the action type name concept rather than the visibility modifier, the word "Public" is now potentially confusing since the `Public` variant was removed.
- **Recommendation:** Update both comments to say "Exported action type name" or simply "Action type name" for clarity.
- **Fix:** Updated both HIR comments to say "Exported action type name" so the terminology no longer refers to the removed `Public` variant.
- **Verification:** Both comments confirmed updated: `lib.rs:323` now reads "Exported action type name" and `ast/expr.rs:229` reads "Exported action type name expected at invocation time". No remaining "Public action type name" references in `crates/nx-hir/`.

### ✅ Verified - RF4 Synthesized root function unconditionally uses Export visibility
- **Severity:** Low
- **Evidence:** In `crates/nx-hir/src/lower.rs:1662`, when a source file contains a root element (direct markup), an implicit `root()` function is synthesized with `visibility: Visibility::Export`. There is no documentation explaining why this is always `Export` rather than following the default `Internal` visibility.
- **Recommendation:** Add a code comment explaining the design intent — the root function is the program entry point and must be discoverable by the runtime, so `Export` visibility is correct. No functional change needed.
- **Fix:** Added a lowering comment explaining that the synthesized `root()` function stays `Export` because the runtime must be able to discover the implicit entry point.
- **Verification:** Comment confirmed present at `lower.rs:1663-1665`: "This entry point must remain discoverable by the runtime even though omitted source visibility now lowers to internal visibility." Rationale is clear and accurate.

## Questions
- RF2: Should the multi-root-module program scenario be deferred to a separate change, or is it expected to be implemented as part of this change? The spec lists it as a requirement.

## Summary
- The core visibility model (grammar, parser, HIR, lowering, library export filtering) is correctly implemented across all layers.
- Grammar docs, VS Code syntax highlighting, and language reference pages are consistently updated.
- All existing tests pass for the modified packages (`nx-syntax`, `nx-hir`, `nx-api`, `nx-interpreter`).
- **Fixed:** RF1 migrated the remaining `nx-types` test code from `Visibility::Public` to `Visibility::Export`, and RF3/RF4 updated the stale visibility-adjacent comments for clarity.
- **One medium-severity finding (RF2) remains open:** A spec scenario for cross-root-module visibility in non-library programs has no implementation path yet.
