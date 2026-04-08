# Review: prepared-module-reference-resolution

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, all 5 spec files (symbol-resolution-model, source-analysis-pipeline, artifact-model, record-type-inheritance, resolved-program-runtime)  
**Reviewed code:** All 12 changed files in working tree diff: `crates/nx-hir/src/prepared.rs` (new), `crates/nx-hir/src/lib.rs`, `crates/nx-hir/src/lower.rs`, `crates/nx-hir/src/records.rs`, `crates/nx-hir/src/scope.rs`, `crates/nx-hir/Cargo.toml`, `crates/nx-api/src/artifacts.rs`, `crates/nx-types/src/check.rs`, `crates/nx-types/src/infer.rs`, `crates/nx-interpreter/src/interpreter.rs`, `crates/nx-interpreter/src/resolved_program.rs`, `crates/nx-interpreter/tests/resolved_program.rs`, `crates/nx-ffi/tests/ffi_smoke.rs`

## Findings

### ✅ Verified - RF1 Interpreter `resolve_item` still uses string-based `find_item()` as primary local lookup
- **Severity:** Medium
- **Evidence:** `crates/nx-interpreter/src/interpreter.rs:451` — `resolve_item` first tries `module.find_item(name)` which scans items by string name. This is the old pattern the design explicitly targets for removal. While entry points (`execute_function` at :221, `initialize_component` at :277) now correctly use `item_by_definition()`, the internal `resolve_item` method (used during expression evaluation for cross-module calls, element resolution, record resolution, etc.) still relies on name-based rescans as its first path.
- **Recommendation:** Refactor `resolve_item` to look up the item via the program's imported-item table and `item_by_definition()` first, falling back to a local `item_by_definition()` lookup for the current module's own definitions. This may require threading a `definition_id` through call sites or building a local name-to-definition index.
- **Fix:** Added `ResolvedProgram`-owned per-module local item tables and changed resolved-program `Interpreter::resolve_item` lookups to use module-qualified `ModuleQualifiedItemRef` entries for both local and imported symbols, leaving `find_item()` only as the bare-interpreter fallback.
- **Verification:** Confirmed. `resolve_item` now checks `program.local_item()` then `program.imported_item()` first (both using `item_by_definition()`), only falling through to `find_item()` when no `ResolvedProgram` exists (bare interpreter path). `build_local_items` in `resolved_program.rs:175` correctly indexes all module items by name with stable `LocalDefinitionId` references. All tests pass.

### ✅ Verified - RF2 `runtime_prepared_module` allocates a fresh `PreparedModule` clone on every call
- **Severity:** Medium
- **Evidence:** `crates/nx-interpreter/src/interpreter.rs:387-436` — `runtime_prepared_module` clones the entire `LoweredModule`, constructs bindings, and registers all imported peer modules every time it is called. It is invoked from `resolve_runtime_record_definition` (:2182) and `resolve_effective_record_shape` (:2225), both of which can be called many times during a single interpretation run (e.g., once per record construction or subtype check).
- **Recommendation:** Cache the constructed `PreparedModule` per `RuntimeModuleId` for the duration of interpretation, or restructure so record resolution uses the already-resolved program tables rather than rebuilding a prepared module at runtime.
- **Fix:** Added an interpreter-owned cache of `Arc<PreparedModule>` keyed by `RuntimeModuleId` so runtime record and shape resolution reuse the prepared module instead of rebuilding it on every call; added a private interpreter test that asserts repeated lookups return the same cached allocation.
- **Verification:** Confirmed. `Interpreter` now holds a `RefCell<FxHashMap<RuntimeModuleId, Arc<PreparedModule>>>` cache field. `runtime_prepared_module` checks the cache first at :417 and returns a clone of the `Arc` on hit. On miss, it builds the prepared module once, wraps it in `Arc`, inserts into the cache at :460, and returns it. Both the no-imports early-exit path (:424) and the full-import path (:460) populate the cache. All tests pass.

### ✅ Verified - RF3 `build_scopes` creates child scopes that `UndefinedIdentifierChecker` duplicates
- **Severity:** Low
- **Evidence:** In `crates/nx-hir/src/scope.rs`, `build_scopes` (lines ~278-310 in the new code) creates child scopes for functions (with params) and components (with props/state). Then `UndefinedIdentifierChecker::check()` (lines ~340-380) independently creates its own child scopes for the same items with the same bindings. The scopes created by `build_scopes` are never consumed by the checker — it uses a fresh clone of the scope manager and builds its own.
- **Recommendation:** Either remove the function/component child scope creation from `build_scopes` (since it only needs to seed the root scope with top-level bindings) or have `check_undefined_identifiers` reuse the scopes already built by `build_scopes` instead of re-creating them.
- **Fix:** Removed the unused function/component child-scope creation from `build_scopes`; the scope builder now only seeds root-level prepared bindings and leaves lexical-scope creation to `UndefinedIdentifierChecker`.
- **Verification:** Confirmed. `build_scopes` at :222-256 now only iterates prepared namespace bindings and defines root-scope symbols — no child scopes are created. All function/component lexical scope construction lives exclusively in `UndefinedIdentifierChecker::check()`. No duplication remains. All tests pass.

## Questions
- None

## Summary
- The implementation is well-structured and faithfully follows the design. The core `PreparedModule` abstraction cleanly separates raw definitions from visible bindings. Raw lowering is now strictly file-local, record validation has been correctly deferred to the prepared phase, and the `ModuleCopier`/`external_items` machinery has been fully removed. Library and program artifact construction correctly builds bindings instead of cloning HIR. Type inference and scope building have been migrated to use prepared bindings. All 15 tasks are complete and all tests pass. The three findings above are about residual string-based lookup in the interpreter (RF1), a performance concern with repeated `PreparedModule` construction at runtime (RF2), and a minor code duplication in scope building (RF3). None are correctness bugs — the implementation is functionally correct against the specs.
- Follow-up update: RF1, RF2, and RF3 have been addressed in code and are marked `🟡 Fixed` pending independent verification.
