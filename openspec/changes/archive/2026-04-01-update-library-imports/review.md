# Review: update-library-imports

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs (declaration-visibility, contenttype-directive, module-imports)  
**Reviewed code:** crates/nx-hir/src/lib.rs, crates/nx-hir/src/lower.rs, crates/nx-hir/src/link.rs, crates/nx-hir/src/scope.rs, crates/nx-interpreter/src/interpreter.rs, crates/nx-interpreter/tests/*, crates/nx-api/src/eval.rs, crates/nx-cli/src/main.rs, crates/nx-syntax/grammar.js, crates/nx-syntax/src/syntax_kind.rs, crates/nx-syntax/src/syntax_node.rs, crates/nx-syntax/queries/highlights.scm, crates/nx-syntax/queries/locals.scm, crates/nx-syntax/tests/parser_tests.rs, crates/nx-syntax/tests/fixtures/*, crates/nx-types/src/check.rs, crates/nx-types/src/infer.rs, src/vscode/syntaxes/nx.tmLanguage.json, src/vscode/samples/*, src/vscode/test/grammar/*, docs/src/content/docs/*, examples/nx/*, nx-grammar.md, nx-grammar-spec.md

## Findings

### ✅ Verified - RF1 Duplicated library-linking + diagnostic-rendering code in CLI
- **Severity:** Medium
- **Evidence:** `crates/nx-cli/src/main.rs:190-215` and `crates/nx-cli/src/main.rs:317-343` contain identical code blocks: `link_local_libraries` call, error handling, diagnostic conversion loop, rendering, and early return. The same pattern also appears in `crates/nx-api/src/eval.rs:47-69`.
- **Recommendation:** Extract a helper function (e.g., `lower_and_link_module`) that performs lowering, library linking, and diagnostic checking in one place. The CLI's `run_file` and `generate_types` functions and the API's `lower_source_module` should all call this shared helper.
- **Fix:** Added shared `nx_hir::lower_source_module` handling for parse/lower/link diagnostics, and updated the CLI and API entry points to use it instead of duplicating that pipeline.
- **Verification:** Confirmed. `nx_hir::lower_source_module` (lib.rs:57-131) centralizes parse/lower/link/validate. CLI uses it via `load_source_module` wrapper (main.rs:299-304) in both `run_file` and `generate_types`. API delegates via `lower_hir_source_module` wrapper (eval.rs:22-33). ~105 lines of duplicated code eliminated. All tests pass.

### 🔴 Open - RF2 Library root for entry file is always its parent directory
- **Severity:** Low
- **Evidence:** `crates/nx-hir/src/link.rs:711-715` sets `current_library_root` to the canonical parent of the entry file path. For a deeply nested file like `src/app/pages/home.nx`, the library root becomes `src/app/pages/` rather than any broader project root. This means peer-file visibility is limited to siblings in the immediate parent directory, which could be surprising if a user considers a broader directory to be their "library."
- **Recommendation:** This is a design-level observation rather than a bug — the current behavior is consistent with the design doc's statement that "the file's parent directory is treated as its library root." Consider documenting this behavior more prominently in the language docs so users understand that peer-file visibility only extends to the containing directory. No code change required unless the design intention changes.
- **Status:** Left open. The current behavior still matches the approved design for this change, so I did not alter library-root semantics during the fix pass.

### ✅ Verified - RF3 No test for `private` visibility on `enum` declarations
- **Severity:** Low
- **Evidence:** The link.rs tests cover `internal` and `private` on `let` declarations (`test_link_same_library_exposes_internal_but_not_private`), and the lowering tests verify visibility on functions, values, type aliases, and records. However, there are no tests that verify `private enum` or `internal enum` declarations are correctly filtered during library linking.
- **Recommendation:** Add a link.rs test that declares a `private enum` and an `internal enum` in a peer file and verifies they are correctly filtered based on visibility scope.
- **Fix:** Added `test_link_same_library_exposes_internal_enum_but_not_private_enum` to cover enum visibility filtering during same-library linking.
- **Verification:** Confirmed. Test at link.rs:1039-1061 creates `internal enum SharedMode` and `private enum HiddenMode` in a peer file, asserts SharedMode is linked and HiddenMode is not, with no diagnostics. Test passes.

### ✅ Verified - RF4 Duplicate import diagnostic does not reference the first import location
- **Severity:** Low
- **Evidence:** `crates/nx-hir/src/link.rs:808-817` — when a duplicate import is detected, the diagnostic only includes the span of the second import. The first import's span (stored in the HashMap value that `insert` returns) is discarded with `if let Some(_)`.
- **Recommendation:** Capture the first import span and include it in the diagnostic message (e.g., "Library '...' is imported more than once in this file; first imported at line X") to help users locate both import statements.
- **Fix:** Duplicate import diagnostics now retain the first import span and report its line/column in the error message.
- **Verification:** Confirmed. link.rs:845-865 now captures the first import span via `seen_import_roots.insert()`, converts it to line/column via `line_col_for_span()` helper (link.rs:721-743), and appends "first imported at line N, column N" to the diagnostic. Test at link.rs:1064-1094 asserts the message contains "first imported at line 1, column 1".

### ✅ Verified - RF5 Component visibility modifier not covered in VS Code structured grammar patterns
- **Severity:** Low
- **Evidence:** The VS Code TextMate grammar at `src/vscode/syntaxes/nx.tmLanguage.json` has structured begin/end patterns for `let` (line 136), `type` (line 164), and `action` (line 183) declarations that include optional `(private|internal)\\s+` groups. However, there is no structured pattern for `component` declarations — `component` is only matched as a standalone keyword (line 92). While `private`/`internal` keywords are highlighted independently (line 93), the lack of a structured component pattern means the grammar cannot provide scoped highlighting for the full `private component <Foo/>` declaration form.
- **Recommendation:** Add a structured pattern for component declarations that includes the optional visibility modifier, similar to the existing patterns for `let`, `type`, and `action`. This would enable more precise scoping for component names and their visibility modifiers.
- **Fix:** Added a structured `meta.definition.component.nx` pattern with optional visibility support and covered it with a grammar tokenization test.
- **Verification:** Confirmed. nx.tmLanguage.json:198-215 adds `meta.definition.component.nx` pattern with `((private|internal)\\s+)?` capture group 3 scoped as `storage.modifier.visibility.nx`, consistent with let/type/action patterns. Test at basic.test.ts:108-114 verifies `private component <SearchBox ...>` tokenizes correctly. VS Code grammar tests pass (65 passing).

### ✅ Verified - RF6 Missing test for selective import of non-existent declaration
- **Severity:** Low
- **Evidence:** The design specifies that selective imports for declarations not exported by the library should produce a diagnostic. While link.rs:857-866 implements this, no test in link.rs or the interpreter tests verifies this error path.
- **Recommendation:** Add a test that uses `import { NonExistent } from "../lib"` where the library doesn't export `NonExistent`, and verify the appropriate diagnostic is emitted.
- **Fix:** Added `test_link_missing_selective_import_reports_diagnostic` to verify the missing-export diagnostic for selective imports.
- **Verification:** Confirmed. Test at link.rs:1097-1122 creates a library with only `Button`, attempts `import { NonExistent }`, and asserts diagnostic contains "does not export 'NonExistent'". Test passes.

## Questions
- None

## Summary
- The implementation is thorough and well-structured across all layers (grammar, parser, HIR, linking, interpreter, docs, VS Code). All 23 tasks are complete and tests pass across the workspace. The core library resolution, visibility enforcement, and deferred ambiguity logic are correctly implemented. Findings are limited to code duplication in the CLI/API layer (RF1), minor test coverage gaps (RF3, RF6), a diagnostic quality improvement (RF4), and a VS Code grammar refinement (RF5). RF2 is an observation about the library root assumption that may warrant documentation. No behavioral bugs or regressions were identified.
