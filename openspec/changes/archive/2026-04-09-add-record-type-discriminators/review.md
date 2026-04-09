# Review: add-record-type-discriminators

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/cli-code-generation/spec.md  
**Reviewed code:** crates/nx-cli/src/codegen/model.rs, crates/nx-cli/src/codegen/languages/typescript.rs, crates/nx-cli/src/codegen/languages/csharp.rs, crates/nx-cli/src/codegen.rs (tests)  

## Findings

### ✅ Verified - RF1 `NxRecord` helper interface is not exported in generated TypeScript output
- **Severity:** Medium
- **Evidence:** The design (Decision 2) specifies `export interface NxRecord<TType extends string = string>`, but [typescript.rs:101](crates/nx-cli/src/codegen/languages/typescript.rs#L101) emits `"interface NxRecord<TType extends string = string>"` without the `export` keyword. The test at [codegen.rs:121](crates/nx-cli/src/codegen.rs#L121) asserts on `"interface NxRecord<TType extends string = string>"` (no export), confirming the deviation is baked into the test as well. Without the export, consumers cannot use `NxRecord` as a generic constraint for discriminated-union handling (e.g., `function process<T extends NxRecord>(item: T)`).
- **Recommendation:** Change the emitted string to `"export interface NxRecord<TType extends string = string>"` and update the test assertion to match.
- **Fix:** Single-file TypeScript output now emits `export interface NxRecord...`, library output emits one shared `_nx.ts` helper that exports `NxRecord`, and the library barrel re-exports it for consumers.
- **Verification:** Confirmed. `emit_nx_record` at typescript.rs:150 now emits `export interface NxRecord`. Library mode emits `_nx.ts` with the exported helper, barrel re-exports via `export type { NxRecord } from "./_nx"`, and the test at codegen.rs:121 asserts on the `export` keyword. All 16 tests pass.

### ✅ Verified - RF2 `NxRecord` is duplicated in every library module that contains records
- **Severity:** Low
- **Evidence:** In library generation, `render_module` at [typescript.rs:66-69](crates/nx-cli/src/codegen/languages/typescript.rs#L66-L69) emits a local `NxRecord` definition in every module file that has at least one record. In a library with many record-bearing modules, this produces identical `NxRecord` declarations in every generated file. While structurally compatible due to TypeScript's structural typing, this is redundant and could confuse consumers who see the same interface defined in multiple places.
- **Recommendation:** Consider emitting `NxRecord` in a shared internal module (e.g., `_nx.ts`) and importing it where needed, or emitting it once in the barrel `index.ts` and importing from there. This can be deferred if the current approach is acceptable for the initial rollout.
- **Fix:** Library generation now writes one shared `_nx.ts` helper and imports `NxRecord` from that module only in record-bearing generated files, removing per-module duplication.
- **Verification:** Confirmed. `emit_library` at typescript.rs:42-47 emits one `_nx.ts` file. Record-bearing modules import via `import type { NxRecord } from "./_nx"` (typescript.rs:78-83). Non-record libraries skip the helper entirely. Tests assert NxRecord import in cross-module abstract family test (codegen.rs:306, 322) and assert no NxRecord in non-record libraries (codegen.rs:228, 269).

### ✅ Verified - RF3 No dedicated test for standalone concrete root records with `$type` discriminator
- **Severity:** Low
- **Evidence:** The spec scenario "Concrete record includes a literal `$type`" covers `export type ShortTextQuestion = { label:string }` (a concrete root record with no base). The test `generates_typescript_exported_aliases_and_action_records_only` at [codegen.rs:102-125](crates/nx-cli/src/codegen.rs#L102-L125) covers an `action` record extending `NxRecord<"SearchRequested">`, and the abstract family test covers concrete derived records. However, there is no test asserting that a plain concrete root `type` (non-action, non-derived) like `export type Payload = { data:string }` generates `$type: "Payload"` via `NxRecord<"Payload">`. The code path is shared with actions, so this is likely correct, but the spec scenario is not directly covered.
- **Recommendation:** Add a test case with a standalone `export type` record and assert that the generated output includes `extends NxRecord<"RecordName">`.
- **Fix:** Added a dedicated TypeScript test that generates a standalone concrete root record and asserts it extends `NxRecord<"Payload">`.
- **Verification:** Confirmed. New test `generates_typescript_concrete_root_records_with_discriminators` at codegen.rs:128-144 uses `export type Payload = { data:string }` and asserts both `export interface NxRecord` and `export interface Payload extends NxRecord<"Payload">`. Directly covers the spec scenario.

### ✅ Verified - RF4 No C# emitter test for multi-level abstract inheritance chain
- **Severity:** Low
- **Evidence:** The model test `collects_transitive_concrete_descendants_across_modules` at [model.rs:357-392](crates/nx-cli/src/codegen/model.rs#L357-L392) verifies that the shared model correctly traverses `Question -> TextQuestion (abstract) -> ShortTextQuestion (concrete)`. However, the C# emitter tests only cover single-level inheritance (`Question -> ShortTextQuestion`) at [codegen.rs:323-343](crates/nx-cli/src/codegen.rs#L323-L343). There is no test verifying that intermediate abstract records correctly inherit the discriminator without re-declaring it, or that the leaf concrete record's `override` chains correctly through multiple levels.
- **Recommendation:** Add a C# generation test with `abstract Question -> abstract TextQuestion -> ShortTextQuestion` and assert that `TextQuestion` does not re-declare `__NxType`, while `ShortTextQuestion` overrides it with its own name.
- **Fix:** Added a multi-level C# emitter test covering `Question -> TextQuestion -> ShortTextQuestion` and asserting only the leaf concrete record redeclares `__NxType`.
- **Verification:** Confirmed. New test `generates_csharp_multi_level_abstract_record_discriminators` at codegen.rs:381-415 covers the full chain. Asserts `Question` declares abstract `__NxType`, `TextQuestion` inherits without redeclaring (explicit block extraction and negative assertion at line 412), and `ShortTextQuestion` overrides with its own name. All 16 tests pass.

## Questions
- None

## Summary
- The implementation is well-structured and all 14 existing tests pass. The shared model cleanly separates abstract-record metadata from the language emitters, and both TypeScript and C# emitters correctly handle the core discriminator patterns. The main actionable finding (RF1) is that the `NxRecord` helper is not exported, which diverges from the design and limits downstream generic usage. The remaining findings are test coverage gaps at low severity.
