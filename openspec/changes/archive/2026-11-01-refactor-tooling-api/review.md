# Review: refactor-tooling-api

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, implementation.md,
specs/rust-crate-layering/spec.md, specs/executable-code-generation/spec.md
**Reviewed code:** all staged and unstaged changes relative to `939b6ad`: workspace and crate manifests,
`Cargo.lock`, `crates/nx-program/**` (moved artifacts, diagnostics, schema, visualizers, workspace,
source graph, std sources, prelude, resolved types), `crates/nx-api/{src,tests}/**` (facade, eval
retargeting, relocated evaluation and codegen-parity tests), `crates/nx-codegen/**`,
`crates/nx-language-service/**`, `crates/nx-interpreter/**`, `crates/nx-cli/**`, `crates/nx-ffi/**`,
`crates/nx-graphics-skia/**`, `bindings/{node,wasm}/native/**`, `scripts/check-crate-layering*.mjs`,
`package.json`, `.github/workflows/build.yml`, `crates/README.md`, the `nx-api` and `nx-program`
READMEs. Also checked repository docs for stale references, and the validation logs in
`target/refactor-validation/` (rust, graphics, graphics-clippy and adapters logs, all `EXIT_STATUS=0`).

**Independent checks run:** `cargo tree` for `nx-codegen`, `nx-language-service`, `nx-program` (all
edge kinds) and `nx-lsp`: none reach `nx-api` or `nx-interpreter`, `nx-program` reaches no service
or engine, and `nx-lsp` does not pull in `nx-api`. I compared test names before and after for
every split file (`artifacts.rs`, `library_source_tests.rs`, `standard_library_tests.rs`,
`visualizer_declarations_tests.rs`), and no names are missing. I diffed the bodies of the moved
tests; the only differences are rustfmt reflow, `crate::` to `nx_codegen::` prefixes, and
`include_str!` depth fixes. The `#[test]` count across the five crates goes from 1822 to 1823, which
is the facade test. The replacement `runtime_helper_source` test helper returns the same content
`emit_codegen_program` writes to `nx-runtime.<ext>`. `CARGO_MANIFEST_DIR`-relative corpus paths keep
the same depth. `openspec validate refactor-tooling-api --strict` passes.

## Findings

### ✅ Verified - RF1 Developer docs still point at test commands that now run zero tests, including the corpus-regeneration command
- **Severity:** Medium
- **Evidence:** The IR corpus, IR runtime, instance tree and graphics tests moved from
  `nx-codegen/src/` to the `nx-api` `codegen_parity` integration test, but these docs still name
  the old crate:
  - `specs/ir-conformance/README.md:158`: `NX_UPDATE_CORPUS=1 cargo test -p nx-codegen --lib ir_corpus`.
    This filter now matches no tests. Cargo reports success, so someone regenerating expected files
    after an emitter change rewrites nothing and sees a passing run.
  - `crates/nx-ir-runtime/README.md:821-822`: `cargo test -p nx-codegen ir_runtime` and
    `cargo test -p nx-codegen ir_instance_tree`. The prose also says these tests "live in
    `nx-codegen`".
  - `sites/website/src/content/docs/reference/libraries/graphics.md:243`:
    `cargo test -p nx-codegen graphics_`.
  - `specs/future.md:108,1360,1706` cite `crates/nx-api/src/artifacts.rs` and
    `crates/nx-codegen/src/ir_runtime_tests.rs`, which no longer exist.

  Task 5.4 ("affected crate READMEs") is checked, but it missed these.
- **Recommendation:** Change the commands to `cargo test -p nx-api --test codegen_parity <filter>`.
  For the corpus that is `NX_UPDATE_CORPUS=1 cargo test -p nx-api --test codegen_parity ir_corpus`.
  Fix the "live in `nx-codegen`" prose and the `specs/future.md` paths. Leave the
  `prelude_image` and `cost_*` commands alone: those tests stayed in `nx-codegen`.

- **Fix:** Updated corpus/runtime/graphics commands and moved-file references.
- **Verification:** Almost complete. Running each documented filter with `--list` shows it matches
  tests:
  - `cargo test -p nx-api --test codegen_parity ir_corpus`: 10 tests, including
    `corpus_artifacts_are_pinned` and `corpus_explained_text_matches_the_committed_images`, the
    tests that honor `NX_UPDATE_CORPUS`. The old `-p nx-codegen --lib ir_corpus` lists 0.
  - `ir_runtime`: 94 tests.
  - `ir_instance_tree`: 42 tests.
  - `graphics_`: 19 tests.

  The `specs/future.md` paths now point at `crates/nx-program/src/artifacts.rs` and
  `crates/nx-api/tests/codegen/ir_runtime_tests.rs`. A repository grep finds no other stale command
  or path outside `openspec/changes/`, and the `prelude_image` and `cost_*` commands correctly stay
  on `nx-codegen`. One sentence is still wrong: in `crates/nx-ir-runtime/README.md:821-824`, "So do
  the cost tests" now follows "live in `nx-api/tests/codegen`". That says the cost tests live there,
  but they are in `crates/nx-codegen/tests/cost/` and the command beside them correctly says
  `-p nx-codegen`. Reword it, for example: "The cost tests, which … also need the compiler and live
  in `nx-codegen` (`cargo test -p nx-codegen --test cost_differential` and `--test cost_allocation`)."
- **Verification (second pass):** Fixed. `crates/nx-ir-runtime/README.md:822` now reads "The cost
  tests remain in `nx-codegen`; they run generated host values …", followed by the unchanged
  `-p nx-codegen --test cost_differential` / `--test cost_allocation` commands. That matches
  `crates/nx-codegen/tests/cost/`. The commands checked in the first pass (all of which match tests
  under `--list`) are unchanged.

### ✅ Verified - RF2 The root README still tells Rust hosts to call `nx_codegen::` directly, not the new external entry point
- **Severity:** Low
- **Evidence:** `README.md:155-183` documents `nx_codegen::emit_js_program_module`,
  `nx_codegen::JsProgramModuleOptions`, `nx_codegen::javascript_runtime_helper_source` and
  `nx_codegen::javascript_runtime_abi` for "Rust hosts". The proposal and spec make `nx-api` the
  Rust entry point for external callers, with codegen at `nx_api::codegen`
  (spec "nx-api is the external Rust entry point"). `nx-codegen` is `publish = false`, so an
  external host can only reach it through `nx-api`. The `crates/README.md` and `nx-api/README.md`
  updates say this, but the top-level guidance contradicts them.
- **Recommendation:** Change the README example and prose to `nx_api::codegen::…`. The paragraph
  saying host wrappers are "supplied outside `nx-codegen`" can stay.

- **Fix:** Updated external Rust examples to the nx_api::codegen namespace.
- **Verification:** `README.md:155-183` now uses `nx_api::codegen::emit_js_program_module`,
  `JsProgramModuleOptions`, `javascript_runtime_helper_source` and `javascript_runtime_abi`. All of
  them resolve through `pub use nx_codegen as codegen`. The sentence about wrappers being supplied
  outside `nx-codegen` stays, as recommended. No `nx_codegen::` guidance for external hosts is left
  in the README.

### ✅ Verified - RF3 The layering check does not enforce `nx-program`'s full "SHALL NOT depend on" list for dev-dependencies
- **Severity:** Low
- **Evidence:** The rust-crate-layering spec says `nx-program` "SHALL NOT depend on
  `nx-interpreter`, `nx-codegen`, `nx-language-service`, `nx-ir-runtime` or `nx-api`". Cargo rejects
  only normal cycles. `scripts/check-crate-layering.mjs:24-25` checks only `nx-api` (upward) and
  `nx-ir-runtime` for `nx-program`. A dev-dependency from `nx-program` on `nx-interpreter` or
  `nx-codegen` would pass Cargo and the check. That is exactly the shortcut design decision 4 warns
  against (keeping an evaluation test in place by adding a test-only upward edge), and it would also
  link two copies of `nx-program` types into those tests. The check meets the narrower wording of
  the "Lower crates never depend on nx-api" requirement, so this is hardening, not a spec violation.
- **Recommendation:** Add `nx-interpreter`, `nx-codegen` and `nx-language-service` to the edges
  forbidden from `nx-program` (any kind). Optionally also reject `nx-codegen` and
  `nx-language-service` → `nx-interpreter` for any kind. Extend the injected-edge tests to match.

- **Fix:** Reject foundation dependencies on all three services, with injected dev-edge coverage.
- **Verification:** `scripts/check-crate-layering.mjs` adds a `foundationBoundary` rule that rejects
  any `nx-program` dependency, of any kind, on `nx-interpreter`, `nx-codegen` or
  `nx-language-service`. Together with the existing `nx-api` and `nx-ir-runtime` rules, this covers
  the spec's full "SHALL NOT depend on" list. The three new injected dev-edge tests each assert the
  exact named violation. `pnpm run check:crate-layering` passes 13 of 13 tests: the real graph, 9
  injected edges of normal, dev and build kind, and the 3 new ones. The optional
  `nx-codegen`/`nx-language-service` → `nx-interpreter` rule was not added. That is acceptable,
  since the recommendation listed it as optional.

### ✅ Verified - RF4 Four call sites duplicate program diagnostic conversion, and `nx-api` now copies every preserved source on each error conversion
- **Severity:** Low
- **Evidence:** Before the move, `nx-api`'s `program_artifact_error_diagnostics` passed the borrowed
  `&program.source_map`. It now builds `source_entries()` (allocates and sorts a `Vec`), and
  `diagnostics_to_api_with_source_entries` copies each source into a new
  `FxHashMap<String, Arc<str>>` (`crates/nx-program/src/diagnostics.rs:115-118`). That includes
  embedded standard-library sources such as graphics, color and math. The same four-line
  `source_entries().into_iter().map(|e| (e.identity, e.source))` pattern appears in
  `crates/nx-api/src/eval.rs:96-103`, `bindings/node/native/src/lib.rs:618-622`,
  `bindings/wasm/native/src/lib.rs:922-926` and `crates/nx-ffi/src/lib.rs:1182-1186`. Output is
  unchanged, but the work is avoidable. Design decision 1 prefers "narrow accessors or shared
  functions in the foundation" for crossing private storage.
- **Recommendation:** Add one function to `nx-program`, for example
  `program_diagnostics_to_api(program: &ProgramArtifact, diagnostics: &[Diagnostic], fallback: &str)`.
  It would call `diagnostics_to_api_with_sources` with the borrowed `source_map`. Use it from
  `nx-api` and the three adapters, so the copy disappears and the conversion lives in one place.

- **Fix:** Added shared artifact diagnostic conversion borrowing preserved storage; migrated all four callers.
- **Verification:** `nx_program::program_diagnostics_to_api`
  (`crates/nx-program/src/diagnostics.rs:280-287`) calls `diagnostics_to_api_with_sources` with the
  borrowed `&program.source_map`. That is the exact path `nx-api` used before the move, so output is
  unchanged and nothing is copied or sorted. All four callers use it now:
  - `crates/nx-api/src/eval.rs:94`
  - `bindings/node/native/src/lib.rs:619`
  - `bindings/wasm/native/src/lib.rs:922`
  - `crates/nx-ffi/src/lib.rs:1182-1186`

  The function is flat-exported from `nx-api` so adapters reach it through the facade. It is not
  called anywhere else. Nit: the function sits after the `#[cfg(test)] mod tests` block in
  `diagnostics.rs`; moving it above would match the file's layout. The adapter edits left unformatted
  import lists, which are tracked separately as RF7.

### ✅ Verified - RF5 The facade's flat surface grows silently through a glob re-export, and the mechanical import edits left scattered `use` items
- **Severity:** Low
- **Evidence:** `crates/nx-api/src/lib.rs:59` uses `pub use nx_program::*;`. Together with the new
  `pub use diagnostics::diagnostics_to_api;` and `pub use diagnostics::LineIndex;`
  (`crates/nx-program/src/lib.rs:54-57`, written as separate statements away from the main
  `diagnostics` re-export), this adds `LineIndex` (previously `pub(crate)`) and `diagnostics_to_api`
  (previously not exported) to `nx-api`'s public API. Any future `nx-program` export will join it
  automatically. Design: "Add only the accessors the split requires", and the external Rust surface
  should be deliberate. The mechanical edits also left split imports and stray blank lines:
  - `crates/nx-api/src/eval.rs:7-12`: three `use nx_program` statements.
  - `crates/nx-codegen/src/builder.rs:16-18`, `emit.rs:19-21`, `ir.rs:22-24` and `model.rs`: two
    `nx_program` imports, followed by a blank line before `use nx_types`.
  - `crates/nx-api/tests/codegen/tests.rs`: a blank line where the `nx_interpreter` import was.
- **Recommendation:** Merge the `nx-program` re-exports into the existing `pub use diagnostics::{…}`
  list. Then either list the flat `nx-api` re-exports explicitly, or keep the glob but mark
  `LineIndex` and `diagnostics_to_api` `#[doc(hidden)]` (or keep them out of the glob) because only
  `nx-api` internals need them. Merge the duplicate `use nx_program` lines and remove the stray
  blank lines.

- **Fix:** Made facade exports explicit, consolidated foundation exports and imports.
- **Verification:** The API-surface part is fixed:
  - `crates/nx-api/src/lib.rs:59-79` replaces the glob with an explicit list. I checked it name by
    name against the baseline `nx-api` exports: every artifact, diagnostic, visualizer, schema and
    workspace name is there, plus the 7 resolved types and the new `program_diagnostics_to_api`.
    `LineIndex` and `diagnostics_to_api` are left out, and future `nx-program` exports no longer
    join the facade automatically.
  - `nx-program`'s diagnostics re-exports are merged into one list.
  - The duplicate `use nx_program` lines in `eval.rs`, `builder.rs`, `emit.rs` and `ir.rs` are
    merged, and the blank lines in `builder.rs`, `emit.rs` and `model.rs` are gone.

  Two problems remain:
  - **Regression:** consolidating the imports removed the `eval.rs` test module's
    `use nx_program::{…, LibraryRegistry}`, and `nx-api`'s lib tests no longer compile (RF6).
  - **Incomplete cleanup:** stray blank lines are still at `crates/nx-codegen/src/ir.rs:23` (between
    `use nx_program` and `use nx_ir`; the baseline had none) and at
    `crates/nx-api/tests/codegen/tests.rs:2` and `:18` (where the moved `crate::` and
    `nx_interpreter` imports used to be).
- **Verification (second pass):** Fixed.
  - `crates/nx-codegen/src/ir.rs` now has one `use nx_program::{ProgramArtifact, ResolvedItemKind,
    RuntimeModuleId};` in sorted position after `use nx_ir::{…}`, with no blank line.
  - `crates/nx-api/tests/codegen/tests.rs:1-22` is one contiguous import block: the leading
    `CodegenProperty`/`CodegenRecordField` import moved down beside the other `nx_codegen` imports,
    and the blank line where `nx_interpreter` used to be is gone.
  - A scan of `builder.rs`, `emit.rs`, `ir.rs`, `model.rs` and `eval.rs` finds no blank line after a
    `use nx_program` import.
  - The regression this cleanup caused is resolved under RF6.

  Nit: `tests.rs` still keeps two separate `use nx_api::{…}` and two `use nx_codegen::{…}` statements
  (the second ones hold the resolved types and the two model re-exports). This is harmless and
  rustfmt-clean, so I am not reopening for it.

## New Findings Discovered During 2026-10-31 22:08 Verification

### ✅ Verified - RF6 The RF5 import consolidation breaks `nx-api`'s lib-test build, so `cargo test --workspace` fails
- **Severity:** High
- **Evidence:** The RF5 fix removed `use nx_program::{build_program_artifact_from_source,
  LibraryRegistry};` from `#[cfg(test)] mod tests` in `crates/nx-api/src/eval.rs` (the module now
  starts at `:434` with `use super::*;`). `use super::*` still provides
  `build_program_artifact_from_source` from the merged top-level import, but `LibraryRegistry` is not
  imported at the top of `eval.rs`. As a result, `LibraryRegistry::new()` at `eval.rs:574`, `:685`,
  `:755` and `:823` fails. The rerun log `target/refactor-validation/fixed-tests.log` ends with
  `error[E0433]: cannot find type LibraryRegistry in this scope` (4 errors) and
  `could not compile nx-api (lib test)`. The log was written 3 seconds after the last edit to
  `eval.rs`, and the current file still lacks the import. Workspace tests and Clippy
  `--all-targets` cannot pass, and the evaluation unit tests in `eval.rs` do not run.
- **Recommendation:** Add `use nx_program::LibraryRegistry;` to the `eval.rs` test module (or add
  `LibraryRegistry` to the top-level import if production code ever needs it). Then rerun
  `cargo test --workspace` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- **Verification:** Fixed.
  - The `eval.rs` test module now has `use nx_program::LibraryRegistry;` at `:436`, after
    `use super::*;`. The redundant `build_program_artifact_from_source` import is dropped because
    `use super::*` already provides it.
  - The current `target/refactor-validation/fixed-tests.log` is a fresh run, started after this edit
    by the same command, and still in progress. It contains no compile errors. It shows
    `nx_api` unit tests passing (116 of 116) and the `codegen_parity`, `facade` and four
    `program_*_evaluation` integration binaries running.
  - `fixed-clippy.log` shows `cargo clippy --workspace --all-targets -- -D warnings` checking
    `nx-api` and every dependent crate with no warnings or errors.
  - The final workspace totals are still pending in `fixed-tests.log`.

### ✅ Verified - RF7 The RF4 adapter edits leave the Node and WASM import lists unformatted, so the CI format check fails
- **Severity:** Medium
- **Evidence:** Removing `diagnostics_to_api_with_source_entries` from the `use nx_api::{…}` lists
  left `build_workspace_program_artifact,` alone on its line. `cargo fmt --all --check` reports
  diffs in `bindings/node/native/src/lib.rs:6` and `bindings/wasm/native/src/lib.rs:21`, and
  nowhere else. The CI `rust` job runs `cargo fmt --all --check`
  (`.github/workflows/build.yml`, right after the new layering step), so this fails the build.
- **Recommendation:** Run `cargo fmt --all`, then confirm `cargo fmt --all --check` is clean.
- **Verification:** Fixed. `cargo fmt --all --check` on the current tree reports no differences.
  The Node and WASM `use nx_api::{…}` lists are reflowed (`bindings/node/native/src/lib.rs:8-12`,
  `bindings/wasm/native/src/lib.rs:23-30`) and still name the same items minus
  `diagnostics_to_api_with_source_entries`. `cargo fix` did not touch anything beyond the files
  already under review: the unstaged diff covers the same files as before, plus the
  `tests.rs` import cleanup. `fixed-adapters.log` passes.

## Questions
- Design "Risks" calls for mechanical move commits kept separate from the facade and adapter
  commits. Everything is staged together at the moment. Will the work be split that way when it
  is committed?
- implementation.md reports 1811 → 1812 `#[test]` cases. A raw `#[test]` count over the same five
  crates gives 1822 → 1823. The +1 delta agrees. Is the documented figure meant to exclude
  something, such as tests in doc comments or tests under `cfg` gates?
- The untracked change `add-visualizer-icon-expressions` (proposal.md:47, tasks.md:6) still names
  `crates/nx-api/src/std/visualizer/visualizer.nx`. That is outside this change, but it will need
  the `nx-program` path once this lands.

## Summary
- The layering is correct and verified: `nx-program` owns the foundation and the resolved types,
  with no edge to any service, engine or `nx-api`. `nx-codegen` and `nx-language-service` no
  longer reach the interpreter or `nx-api`. `nx-lsp` stays light. The `codegen`,
  `language_service` and `ir_runtime` namespaces exist, and the facade test and the README doctest
  exercise build → emit → run against the interpreter-backed result. Adapters (CLI, FFI, Node,
  WASM, preview) reach services through `nx-api`. `nxlang run` and every flat evaluation path still
  use the interpreter, and no persisted behavior changed.
- Coverage is preserved: every relocated test exists with an unchanged body, and the workspace,
  adapter, graphics and playground validation logs pass.
- One Medium docs finding: stale test commands, including a corpus-regeneration command that now
  silently does nothing. Four Low findings cover the layering-check scope, duplicated diagnostic
  conversion, and API-surface tidiness. None blocks correctness. RF1 and RF2 mean task 5.4 is not
  complete, even though it is checked.
- Verification (2026-10-31 22:08): RF2, RF3 and RF4 are verified. RF1 is reopened for one cost-test
  sentence that is now inaccurate. RF5 is reopened for leftover blank lines. The fixes introduced
  two regressions: RF6 (High), where the `nx-api` lib tests no longer compile because the `eval.rs`
  test module lost its `LibraryRegistry` import, and RF7 (Medium), where `cargo fmt --check` fails in
  the Node and WASM bindings. Rerun the workspace tests, Clippy and the format check after fixing
  these.
- Second verification (2026-10-31 22:12): RF1, RF5, RF6 and RF7 are verified against the current
  files. All seven findings are now ✅ Verified, and no new issues were found. Formatting is clean,
  Clippy with warnings denied passes, the adapters pass, and the `nx-api` lib tests compile and
  pass. The full `cargo test --workspace` run in `fixed-tests.log` was still in progress when I
  checked, with no failures so far.

## Follow-up fixes

- RF1: Clarified that cost tests remain in nx-codegen.
- RF5: Removed the remaining import gaps.
- RF6: Restored the test-scoped LibraryRegistry import.
- RF7: Ran cargo fmt after import fixes; format verification passes.
