# Review: add-range-type-and-operators

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md` (D1–D8), `tasks.md` (47 tasks, all `[x]`), and all ten
delta specs (`language-prelude`, `range-expressions`, `nx-ir-format`, `typescript-ir-runtime`,
`cli-code-generation`, `dotnet-binding`, `editor-syntax-highlighting`, `language-http-service`,
`sdk-node`, `record-type-parameters`).

**Reviewed code:** the whole uncommitted change (90 paths, tracked + untracked).

- Rust: `crates/nx-api/{artifacts,source_graph,workspace,lib}.rs` + `prelude.nx` + `tests/ranges.rs`;
  `crates/nx-hir/{ast/expr,components,lib,lower,scope}.rs`;
  `crates/nx-types/{infer,check}.rs` + `tests/{record_type_parameters,ranges_without_a_prelude}.rs`;
  `crates/nx-interpreter/src/interpreter.rs`;
  `crates/nx-codegen/{builder,ir,ir_image,ir_explain,emit,runtime,model,lib,ir_tests,tests,prelude_image_tests}.rs`;
  `crates/nx-cli/{main,typegen,typegen/model,typegen/languages/{csharp,typescript}}.rs`;
  `crates/nx-syntax/{grammar.js,queries/highlights.scm,src/syntax_kind.rs,generated files,tests}`;
  `crates/nx-language-service`, `crates/nx-lsp`.
- TypeScript: `runtime/typescript/{src,test,dist,README}`; `packages/language-core`,
  `packages/language-http`; `bindings/node` (+ `native/src/lib.rs`), `bindings/wasm`;
  `sites/playground/src/{compile,render}`.
- .NET: `bindings/dotnet/src/NxLang.Sdk/NxRange.cs`, `bindings/dotnet/tests/.../NxRangeTests.cs`.
- Docs/specs/assets: `docs/nx-ir-format.md`, `docs/src/content/docs/**`, `nx-grammar.md`,
  `nx-grammar-spec.md`, `specs/future.md`, `specs/ir-conformance/{manifest.json,ranges/}`,
  `src/vscode/{syntaxes,test/grammar}`, `examples/nx/{ranges,generic-records}.nx`.

**Commands run:** `cargo test --workspace` (pass), `cargo clippy --workspace --all-targets` (clean),
`cargo fmt --all --check` (**fails** — RF4), `cargo test -p nx-cli` (218 pass),
`cargo build -p nx-ffi` + `dotnet test bindings/dotnet/NxLang.sln` (153 pass),
`runtime/typescript` `npm test` (294 ok / 0 fail, including the new `ranges` conformance program),
`pnpm -C packages/language-core test` (14/14), `pnpm -C packages/language-http test` (18/18),
`bindings/node` (25/25), `bindings/wasm` language-service (5/5), playground
`catalog.test.mjs` (16/16) + `evaluate.test.mjs` (4/4), `src/vscode` `pnpm test:grammar` (261/0),
`cargo test -q -p nx-syntax` (all green), `nxlang run` on both examples,
`openspec validate add-range-type-and-operators --strict` → `Change 'add-range-type-and-operators' is valid`.
Plus ~20 hand-built NX programs and `nxlang typegen`/`codegen` runs to probe edge cases.

## Findings

### ✅ Verified - RF1 A value- or element-namespace `Range` is not detected as hiding the prelude's, so a range expression type-checks and then fails at evaluation and at codegen
- **Severity:** High
- **Evidence:** `crates/nx-types/src/infer.rs` `prelude_range_name()` consults `self.record_origins`
  **first**, and only falls through to the Type/Element/Value namespace scan when that map has no
  `Range`. Because `apply_prelude_bindings` (`crates/nx-api/src/artifacts.rs`) skips a name
  **per namespace**, a module that declares `let Range = 5`, `let Range() = 5` or
  `let <Range /> = <thing />` still gets the prelude's `Range` in the Type/Element namespaces, so
  `record_origins` resolves it, `PreludeRangeName::Prelude` is returned, and no `range-hidden` is
  reported. Below the checker the rewritten `Expr::RecordLiteral { record: "Range" }` is resolved
  **by name**, where the local item wins. Reproduced (all three spellings):
  - interpreter — `let Range = 5` / `let r = {1..5}` / `let root() = { r.start }` →
    `Runtime error: Record type not found: Range` with **no diagnostic**.
  - `nxlang codegen --target nx-ir` on the same file →
    `error …:2:10: record construction of 'Range' does not reach a record declaration`.
  - `nxlang codegen --target typescript` emits uncompilable TS:
    `export const Range: number = 5;` beside `export const r: Range = ({ $type: "Range", … });`
    (a value used as a type, and no import of the prelude's `Range`).

  A Type-namespace shadow (`type Range = int`, `type Range = a | b`, `type Range = { … }`) is
  correctly rejected, so the gap is exactly the non-type namespaces. This violates
  `range-expressions` → "A range expression is rejected where `Range` is not the prelude's"
  ("In a module where the name `Range` refers to **anything** other than the prelude's declaration").
  A second, contributing inconsistency: `build_resolved_program` (`artifacts.rs` ~:2404) builds
  `declared_here` from **every** item name regardless of kind, so it withholds the prelude's `Range`
  from `visible_imports` for exactly these modules, while `apply_prelude_bindings` still binds it —
  the two shadowing rules disagree.
- **Recommendation:** decide the prelude-`Range` question from the module's **bindings**, not from
  `record_origins`: check `resolve_binding` in Type *and* Element *and* Value and require every
  hit to be the prelude's (or, minimally, report `range-hidden` whenever any namespace holds a
  non-prelude `Range`). Make `declared_here` in `build_resolved_program` namespace-aware so it
  agrees with `apply_prelude_bindings`. Add `nx-api/tests/ranges.rs` cases for `let Range = 5`,
  `let Range() = 5` and `let <Range />`.
- **Fix:** the prelude's names now hide by whole name rather than per namespace (`apply_prelude_bindings` skips an export whose name the module binds in *any* namespace, which is what "every use of that name in the module SHALL mean the module's own" says and what `declared_here` and codegen already assumed), and `prelude_range_name` asks the module's bindings before the record origins so the two rules cannot drift. Added `Item::span` so the secondary label points at whatever kind of declaration took the name. `crates/nx-api/tests/ranges.rs::a_local_range_in_any_namespace_disables_the_operator` covers `let Range = 5`, `let Range() = 5`, `component <Range />`, `type Range = int` and `type Range = | only`. The `range-expressions` requirement now states that the namespace does not matter, with a scenario for the non-type spellings.
- **Verification:** the chosen shape is right and the code matches it. `apply_prelude_bindings`
  (`crates/nx-api/src/artifacts.rs:836-845`) now skips an export whose name the module binds in
  Value, Type **or** Element, and `prelude_range_name` (`crates/nx-types/src/infer.rs:1576-1608`)
  scans those three namespaces before falling back to `record_origins`. Whole-name hiding is what
  `language-prelude`'s existing "every use of that name in the module SHALL mean the module's own"
  requires, so no delta was needed there, and it is what makes `declared_here`,
  `apply_prelude_bindings` and codegen's by-name lookup agree without a namespace-aware
  `visible_imports`. Re-ran all three original symptoms: `let Range = 5`, `let Range() = 5` and
  `let <Range />` now each report `range-hidden` at check time with a secondary label, and none
  reaches the interpreter or codegen. Confirmed no legitimate case broke: `type Range = int` still
  works as an alias and correctly rejects `<Range T=int/>` with "'Range' is not a generic record"
  (the untyped `let r = <Range T=int .../>` variant is silently accepted, but that is a pre-existing
  hole in element-tag checking for any primitive alias — `type Foo = int` behaves identically);
  the "generic record declared before the prelude existed" scenario evaluates; a wildcard import of
  a foreign `Range` accepts the element form and rejects the operator naming `shared/shapes.nx`;
  a selective import accepts the element form and generates correctly against the dependency's type;
  a sibling module that imports neither still gets the prelude's. One coherence wart the fix leaves
  behind is filed as RF23.

### ✅ Verified - RF2 Generated TypeScript for a prelude type does not compile when the output declares no record of its own
- **Severity:** High
- **Evidence:** `crates/nx-cli/src/typegen/languages/typescript.rs:70-79` gates the helper module on
  `has_records || !prelude_records.is_empty()` but passes **`has_records`** as `include_nx_record`
  to `render_nx_record_module` (`:230-251`); `render_module` (`:210-221`) likewise emits
  `NxRecord` only when `module_needs_nx_record(module)`. Every prelude record goes through
  `emit_concrete_record`, which unconditionally appends `extends NxRecord<"Range">` (`:461`).
  Reproduced with `export type Bounds = <Range T=int/>` (a module with no record of its own):
  - single-file: output begins `export interface Range<T> extends NxRecord<"Range"> { … }` with
    **no `NxRecord` declaration** → TS2304.
  - library: `_nx.ts` has the same problem, and `index.ts` re-exports only `Range`, so `NxRecord`
    is missing from the public surface too.

  The same holds for a module whose only declaration is abstract. The existing tests miss it
  because both new TypeScript cases use modules that declare concrete records.
- **Recommendation:** pass `has_records || !prelude_records.is_empty()` as `include_nx_record` in
  `emit_library` (both the helper-module call and `render_index`), and
  `module_needs_nx_record(module) || !prelude_records.is_empty()` in the `Inline` branch. Add a
  test whose only exported declaration is an alias over `<Range T=int/>` and assert the output
  type-checks under `tsc` (the suite already has that helper).
- **Fix:** `emit_library` computes one `needs_nx_record = has_records || !prelude_records.is_empty()` and passes it to both the helper module and the index, and the `Inline` branch emits `NxRecord` when a prelude record is referenced. Two tests added: single-file and library output whose only exported declaration is an alias over `<Range T=int/>`.
- **Verification:** re-ran the reproducer. Single-file output for `export type Bounds = <Range T=int/>`
  now opens with `export interface NxRecord<TType extends string = string>` before
  `export interface Range<T> extends NxRecord<"Range">`; library output declares it in `_nx.ts` and
  `index.ts` re-exports `export type { NxRecord, Range } from "./_nx"`. An abstract-only module that
  references the prelude (`export abstract type Base = { r:<Range T=int/> }`) also gets it. A module
  that references no prelude type (`export type Thing = | only`) gains nothing, so the
  "unchanged output" scenario still holds. Four tests cover it, including both new
  `..._whose_only_record_is_the_prelude_s_still_declares_nx_record` cases. No delta was needed: the
  `cli-code-generation` scenario already requires the helper module to export a usable `Range<T>`.

### ✅ Verified - RF3 `for` over a *nullable* range type-checks and fails only at runtime
- **Severity:** Medium
- **Evidence:** `InferenceContext::prelude_range_argument` (`crates/nx-types/src/infer.rs`) calls
  `ty.strip_nullable()`, and the `Expr::For` arm uses it directly, so a `<Range T=int/>?` iterable
  is accepted. `let maybe:<Range T=int/>? = null` / `let xs:int[] = { for i in maybe { i } }` gives
  no diagnostic and then `Runtime error: Type mismatch in for loop iteration: expected array, got
  null`. The list path is correctly strict — `int[]?` reports
  `For iterable must be an array, found int[]?` at check time. `range-expressions` says the
  iterable SHALL be "a list or a `<Range T=X/>`", not an optional one.
- **Recommendation:** in the `Expr::For` arm, match the range without stripping nullability (keep
  `strip_nullable` in `convert_literals`, where an optional `<Range T=float64/>?` field legitimately
  accepts `0..1` — that path is correct today). Add a checking test for the nullable iterable.
- **Fix:** `prelude_range_argument` matches the type as it stands; the binding-site caller strips the nullability itself. `for` over a `<Range T=int/>?` now reports at check time. Tests: `an_optional_range_does_not_iterate` and `a_range_binds_to_an_optional_range_field` (the binding site still accepts `range={0..1}` at an optional field).
- **Verification:** `for i in maybe` over a `<Range T=int/>?` now reports at check time
  (`For iterable must be an array, found <Range T=int/>?`) instead of reaching the interpreter, the
  optional-field binding site still accepts `range={0..1}` and yields a `float64` range, and plain
  range iteration is unaffected. Both new tests present and passing. Wording nit, not worth
  reopening: the nullable case falls to the generic "must be an array" message rather than the
  `range-not-iterable` phrasing that says "a list or a range of an integer type", so the diagnostic
  understates what the iterable position accepts.

### ✅ Verified - RF4 `cargo fmt --all --check` fails, so the change breaks CI; task 11.1 claims otherwise
- **Severity:** Medium
- **Evidence:** `cargo fmt --all --check` reports 17 hunks across 15 files —
  `crates/nx-api/src/artifacts.rs` (4), `crates/nx-types/src/infer.rs` (3),
  `crates/nx-codegen/src/prelude_image_tests.rs` (2), and one each in
  `crates/nx-api/tests/ranges.rs`, `crates/nx-cli/src/main.rs`, `crates/nx-cli/src/typegen.rs`,
  `crates/nx-cli/src/typegen/languages/typescript.rs`, `crates/nx-codegen/src/{ir,ir_tests,lib,tests}.rs`,
  `crates/nx-hir/src/{components,lib,lower}.rs`, `crates/nx-language-service/src/lib.rs`.
  `.github/workflows/build.yml:230` runs `cargo fmt --all --check`. I confirmed HEAD (`a3882f3`) is
  fmt-clean in a detached worktree, so all 17 are new. Task 11.1 is marked `[x]` with
  "verify all pass".
- **Recommendation:** run `cargo fmt --all`.
- **Fix:** `cargo fmt --all`; `cargo fmt --all --check` is clean. Task 11.1 now names the fmt check too.
- **Verification:** `cargo fmt --all --check` reports zero diffs, so the CI gate at
  `.github/workflows/build.yml:230` passes. Task 11.1 now reads "…`cargo clippy --workspace`,
  `cargo fmt --all --check`, the TypeScript runtime…".

### ✅ Verified - RF5 The tracked `runtime/typescript/dist` now imports a file that is git-ignored and untracked
- **Severity:** Medium
- **Evidence:** `git ls-files runtime/typescript` tracks only `dist/src/index.{js,d.ts}` and
  `dist/test/runtime.test.{js,d.ts}`; `git check-ignore -v runtime/typescript/dist/src/prelude-image.js`
  → `runtime/typescript/.gitignore:1:dist/`. But `runtime/typescript/dist/src/index.js:11` is now
  `import { NX_PRELUDE_IMAGE_BASE64 } from "./prelude-image.js";`. Committing as-is yields a tracked
  `dist` that cannot be loaded from a fresh clone (`ERR_MODULE_NOT_FOUND`), and
  `sites/playground` / `bindings/{node,wasm}` depend on `@nx-lang/ir-runtime` as `workspace:*`,
  which resolves straight to that file. CI happens to mask it by running `pnpm -r build` first.
- **Recommendation:** either `git add -f runtime/typescript/dist/src/prelude-image.{js,d.ts}`
  (consistent with how `index.js` is already force-added), or stop tracking `dist` entirely.
- **Fix:** `dist` rebuilt from the edited sources and `dist/src/prelude-image.{js,d.ts}` force-added, consistent with the already-tracked `index.js`.
- **Verification:** `git ls-files runtime/typescript` now lists `dist/src/prelude-image.d.ts` and
  `dist/src/prelude-image.js` beside `dist/src/index.{js,d.ts}`, so the import at
  `dist/src/index.js:11` resolves from a clone. The working-tree `dist` is a current build: I re-ran
  `npm test` (which builds first) and the file set did not move, and the built `index.js` carries
  `forRange`, `maxRangeLength` and `nx-ir-prelude-image`. One caveat that is not RF5's to carry, so
  it is filed as RF22: only the two *new* dist files are in the git index; the modifications to
  `dist/src/index.{js,d.ts}` and `dist/test/runtime.test.js` are unstaged, so a commit made from the
  index alone would ship the old runtime beside an orphan `prelude-image.js`.

### ✅ Verified - RF6 A contract field typed `<Range.Update T=int/>` cannot be generated in either language
- **Severity:** Medium
- **Evidence:** `ExportedTypeGraph::prelude_records()` (`crates/nx-cli/src/typegen/model.rs:522-541`)
  `filter_map`s to `ExportedType::Record` only, so the prelude's derived companions are dropped.
  Reproduced with `export type Patch = { change:<Range.Update T=int/> }`:
  `Warning: Type reference 'Range.Update' has no generated companion 'Range_update' to resolve to;
  the generated code will not compile until 'Range' is exported and the companion name is free`
  (a misleading message for a built-in), then C# emits `public Range.Update<long> Change` (not valid
  C#) and TypeScript emits `change: Range_Update<number>` with no such declaration. Both `prelude.nx`
  and `examples/nx/ranges.nx:38` advertise `apply(half, <Range.Update T=int end={9} />)`, and the
  `language-prelude` delta's "`Range` has its companions" scenario makes the companions part of the
  contract surface.
- **Recommendation:** include the prelude's update record and property union in
  `prelude_records()` / the reference scan so `rewrite_update_type_references` finds a companion, and
  map `Range.Update` in C# to the SDK's equivalent (or generate it). Add a typegen test for a field
  typed `<Range.Update T=int/>` and one typed `Range.Property`.
- **Status:** left open: no single high-confidence fix. Confirmed the recommendation is not mechanical — TypeScript could generate the companions into the helper module, but the .NET SDK names only `NxRange<T>`, so C# would need a new hand-written `NxRangeUpdate<T>`/`NxRangeProperty` surface (or a deliberate refusal). Which of those is right is the reviewer's open question 2 and wants a decision; today's behaviour is a misleading warning plus uncompilable output in both languages, so it should not be left as it is.
- **Decision:** generate them. The prelude's own comment says a declaration there is "an ordinary declaration — constructed, applied, emitted and generated through the same code as a user's own", and `language-prelude` already makes the companions part of the contract surface; a patch of a range is no more exotic across a host boundary than a patch of a user record, and the SDK already has the whole `NxUpdate`/`NxProperty` machinery. Refusing would have made the prelude the one declaration whose companions do not cross.
- **Fix:** `prelude_records()` became `prelude_declarations()`, keeping the derived update record and property union, so `rewrite_update_type_references` finds a companion and `record_type_params` answers for it. TypeScript emits them through the ordinary emitters, beside `Range` in the helper module (library output) or inline (single file), with the reference scan closed transitively. C# needed no new arm: the existing `Nx{other}` prelude rule already spells `Range_update` as `NxRange_update`, so the work was the SDK types — `NxRange_update<T>`, `NxRange_property` with its `NxRangeProperties<T>` key table, and the wire format — written to the shape the emitter generates for the same declaration.
  - A second, pre-existing defect surfaced on the way: **a member typed by *any* generic update companion did not compile in C#**, prelude or not. MessagePack's source generator rejects the open generic the companion carries (`MsgPack006`), and its alternatives fail too — a generic formatter named at a member is emitted with the type parameter's name rather than the argument (`CS0246`), and a closed formatter beside an open one in the same compilation is two formatters for one type (`MsgPack009`). The emitter now declares a closed formatter beside the contracts for each instantiation a member's own type is, and the member names it. That works for a companion another assembly declares — the prelude's, or a dependency's. The shape MessagePack has no working form for, a companion the module itself declares or one nested inside another type, is now a warning naming the field and the companion rather than a file that does not build.
- **Verification:** confirmed by execution, not only by the tests. `nxlang typegen` on the finding's own
  reproducer (`export type Patch = { change:<Range.Update T=int/> which:Range.Property }`) now emits no
  warning and produces, in C#, `global::NxLang.Nx.NxRange_update<long>` and
  `global::NxLang.Nx.NxRange_property` with a closed `NxRange_updateOfLongFormatter` beside them, and in
  TypeScript `Range_update<number>` / `Range_property` declared in the same output — which `tsc --noEmit
  --strict` accepts. The checked-in `Generated/prelude-companions.nx` → `PreludeCompanions.g.cs` fixture
  is compiled and round-tripped by `NxRangeCompanionTests`, and
  `checked_in_dotnet_prelude_companion_fixture_matches_typegen_output` reads it off disk, so drift fails
  the Rust suite too. `cargo test -p nx-cli` 225 pass; `dotnet test` 158 pass; the
  `cli-code-generation` delta's new scenario matches the output byte for byte.
  **One case the fix does not reach:** the closed formatter is emitted per module, so a *library* whose
  modules share a namespace and the same instantiation emits the class twice and does not compile.
  Recorded as RF22 rather than reopening this, because the failure and its fix are elsewhere.

### ✅ Verified - RF7 C# does not consult the prelude when resolving a *dependency library's* alias target
- **Severity:** Medium
- **Evidence:** `csharp_imported_alias_target_name`
  (`crates/nx-cli/src/typegen/languages/csharp.rs:1554-1609`) has its own `match` with no prelude
  arm; the `other` arm falls to `generated_type_name(other, dependency_namespace, true)` (`:1604-1608`).
  So a dependency exporting `export type IntRange = <Range T=int/>` over the *prelude's* `Range`
  makes the importing library emit `global::<DependencyNamespace>.Range<long>` — a type the
  dependency deliberately never generates. The `cli-code-generation` delta requires a prelude type to
  render as `global::NxLang.Nx.NxRange<…>`. TypeScript is unaffected (an imported alias renders as
  the alias name and the dependency's own file resolves it). *Code-read; I did not build the
  two-library fixture, so this is unconfirmed by execution.*
- **Recommendation:** add the prelude arm to `csharp_imported_alias_target_name` beside the one in
  `csharp_type_inner`, and extend the existing
  `an_applied_type_over_a_record_imported_from_a_dependency_keeps_its_arguments` test with a
  prelude-`Range` variant.
- **Fix:** reproduced first with a two-library fixture (`global::Test.Bounds.Range<long>`), then fixed by giving `csharp_imported_alias_target_name` the render context and the prelude arm beside the one in `csharp_type_inner`. A dependency that declares its own `Range` is imported under that name, which `prelude_record` already sees, so the existing dependency test is unaffected. Test: `an_imported_alias_over_a_prelude_record_resolves_to_the_sdk_type`.
- **Verification:** `csharp_imported_alias_target_name`
  (`crates/nx-cli/src/typegen/languages/csharp.rs:1607-1618`) now takes the render context and has
  the prelude arm, built the same way as the one in `csharp_type_inner` (`:1378`) —
  `format!("global::NxLang.Nx.Nx{other}")` in both, so the two cannot spell the SDK name
  differently. The new test passes and the pre-existing dependency-`Range` test is unaffected.
  Note the arm calls `context.graph.prelude_record`, so it inherits RF8's graph-wide scoping; that
  is already tracked there.

### ✅ Verified - RF8 typegen's prelude-shadow check is graph-wide rather than per-module
- **Severity:** Medium
- **Evidence:** `ExportedTypeGraph::prelude_record` (`crates/nx-cli/src/typegen/model.rs:549-566`)
  returns `None` when `self.declaration(name).is_some()` — and `declaration` scans **all** modules
  (`:569-574`) — or when **any** module has an `imported_types` entry with that visible name. The
  prelude's rule is per-module ("a module that declares one of them keeps its own",
  `crates/nx-api/src/prelude.nx:5-6`), and D6 says "resolution goes through the module's own
  namespace first". I tried to reproduce with a two-module library and found the *declaration* half
  is unreachable, because a same-library peer's `Range` shadows the prelude in every module of that
  library (the compiler rejected `<Range T=int/>` in the sibling with "Record 'Range' has no type
  parameters"). The `imported_types` half is not covered by that: a module-scoped `import` of a
  dependency's `Range` in module A is not visible in module B, yet it still suppresses
  `prelude_record` for B, which would then render a bare undefined `Range<…>` in both languages.
  *Unverified by execution — I did not build the dependency-library fixture.*
- **Recommendation:** scope the shadow test to the module being emitted (its own declarations plus
  its own `imported_types`), and add a two-module library test where one module imports a
  dependency's `Range` and the other uses the prelude's. (A cached shadow-name set would also fix
  the O(occurrences × (declarations + imports)) cost of the current scan.)
- **Status:** left open: the `imported_types` half is a real per-module/graph-wide mismatch, but the declaration half must stay graph-wide (a same-library peer does shadow library-wide, as the review found), and `prelude_record` has no module parameter — two of its four call sites have no module in hand. Splitting it wants the answer to the reviewer's open question 5 first: whether a module-scoped import of a foreign `Range` suppresses the prelude's for sibling modules of the same library.
- **Decision:** it does not. An `import` is written in one module and binds only there, which is what D6 and the prelude's own comment say; a *declaration* is different only because a same-library peer is visible without an import, and that is why the declaration half stays graph-wide.
- **Fix:** the shadow question is no longer asked of the graph. `ModuleTypes { graph, imports }` is the export graph as one module resolves names against it, and carries `prelude_declaration`, `prelude_record` and `record_type_params`; the declaration check inside it is graph-wide and the import check is the module's own. Both emitters build one per module they render — C#'s render context already held the module's imports, and TypeScript's `ts_type` chain took the graph where it now takes the view. The reviewer's finding understated the problem: `record_type_params` scanned `imported_types` graph-wide too, so a dependency's `Range` imported in module A supplied *type parameters* in module B as well; the view fixes both sites at once, and the rescan the finding noted is gone with it.
- **Verification:** `ModuleTypes` (`crates/nx-cli/src/typegen/model.rs:962-1057`) is the split the
  decision describes: `hides_prelude_name` asks `declares_over_the_prelude` of the graph and `import`
  of the module's own list, and `record_type_params` consults the same view, so the second site the
  finding did not reach is covered. Re-ran the mutation independently: replacing the per-module import
  check with a graph-wide scan makes
  `an_import_of_a_foreign_range_leaves_its_sibling_with_the_prelude_s` fail, so the test does pin the
  behaviour rather than merely pass beside it. Restored, and the full `cargo test -p nx-cli` is 225
  pass.

### ✅ Verified - RF9 The "a client cannot replace a context document" refusal misses the identity case, and the core service does not refuse at all
- **Severity:** Medium
- **Evidence:** `contextCollision` (`packages/language-core/src/answer.ts:52-55`) builds
  `contextIdentities` from `document.identity` **only when explicitly present**, and never derives
  a context document's identity from its URI the way the SDK does. Both READMEs and
  `packages/language-http/test/handler.test.ts:287-292` declare context documents *without* an
  explicit identity, so for the documented shape the identity branch is dead. Verified against the
  built `dist`: with `context = [{ uri: 'nx://tenant/catalog.nx', source }]`, a client document
  `{ uri: 'nx://other.nx', identity: 'tenant/catalog.nx' }` is **not refused** by
  `createSnapshotLanguageService` (it answers `null`); the HTTP handler returns 400 but only by
  falling through to the SDK's generic `Duplicate NX identity 'tenant/catalog.nx'`, not the context
  check, so the message does not say the client cannot replace the host's context. The
  `language-http-service` delta's scenario "A client cannot replace a context document" is
  therefore only satisfied for a URI collision. Related: the message at
  `packages/language-core/src/service.ts:93` and `packages/language-http/src/index.ts:131` says
  "A document with the identity '<x>'" while `<x>` may be a URI — and the URI branch is the one that
  actually fires.
- **Recommendation:** derive the identity from a context document's URI in `contextCollision` (mirror
  the SDK's derivation), or require `identity` on context documents. Fix the message to name a URI
  as a URI. Add a test that collides on `identity` with a distinct URI — the existing two tests both
  collide on the URI, and `service.test.ts:196` loosens its assertion to
  `/tenant\/catalog\.nx|nx:\/\/tenant\/catalog\.nx/`, which hides which value is named.
- **Fix:** `contextCollision` now derives a document's identity from its URI the way the SDK does when it declares none, and returns what collided (`uri` or `identity`) rather than a bare string. One `contextCollisionMessage(collision, "query" | "request")` serves both callers, so a URI is named as a URI. Tests: an identity collision under a distinct URI, in both `service.test.ts` and `handler.test.ts`, and the loose URI assertions tightened to the whole message. The `language-http-service` delta's scenario is split into a URI collision and an identity collision, so the identity case is a requirement rather than an accident of the SDK's duplicate check.
- **Verification:** exercised the built `dist` directly against a context of
  `[{ uri: "nx://tenant/catalog.nx", source }]` with no declared identity. A same-URI client document
  is refused naming it as a **uri**; a client document under `nx://other.nx` declaring
  `identity: "tenant/catalog.nx"` is now refused naming it as an **identity**; and two legitimate
  queries (`nx://tenant/form.nx`, `nx://other/catalog.nx`) are accepted. The delta splits the two
  cases into scenarios and both packages have a test for the identity case; all 15 + 19 pass.

  **On the duplication:** acceptable. `documentIdentity`
  (`packages/language-core/src/answer.ts:84-108`) mirrors `identity_from_uri`
  (`crates/nx-language-service/src/lib.rs:1603-1642`) closely, and — importantly — neither SDK's
  snapshot API exposes a workspace root (`NxLanguageSnapshotOptions` has only `buildContext` and
  `implicitImports`, and `createSnapshot` is handed only documents and implicit imports), so the
  Rust side is always called with `workspace_root: None` and the TS copy's "no root" assumption
  holds. **No false-refusal vector:** I checked the one case that looks like one —
  context `file:///a/catalog.nx` vs client `file:///b/catalog.nx` is refused on the derived identity
  `catalog.nx`, and Rust with no root derives `catalog.nx` for both too, so the SDK would have
  refused it as a duplicate identity anyway; the TS check only refuses earlier with a better message.
  Every way the two rules can drift (TS uses `document.identity` verbatim where Rust also trims and
  normalizes `\`, and TS returns `undefined` where Rust errors) makes TS refuse *less*, and the SDK
  still catches it — the safe direction. Worth a follow-up, not a reopen: there is no parity test
  between the two derivations, and exporting the derivation from the SDKs would remove the copy.
  Minor wording gap in the delta: the requirement sentence says only "the same identity", while the
  URI scenario also refuses a same-URI document that declares a *different* identity.

### ✅ Verified - RF10 The built-in prelude is linked with no shape or fingerprint check
- **Severity:** Medium
- **Evidence:** the module table records the prelude's fingerprint
  (`specs/ir-conformance/ranges/expected/app__main.nx.nxir.txt:2`:
  `links @nx/prelude.nx version "" fingerprint 16429037828798365428`), but linking's only guards are
  `resolved.version !== entryTable.version` (`runtime/typescript/src/index.ts:1554` — both `""`, so
  always passes) and the set of referenced declaration **names** (`:1563`). `NxPreparedModule` has a
  `fingerprint` and it is never compared for the built-in fallback. A runtime package whose baked-in
  prelude differs from the compiler that produced the image (a field type change, a new default)
  links silently and misbehaves at evaluation instead of failing with
  `nx-ir-link-missing-declaration`. The new test only covers a *missing name*, which is the one case
  the design already claimed. `docs/nx-ir-format.md` states the intent ("compatibility is decided by
  which declarations the runtime's prelude holds"), so name-only is arguably by design — but the
  staleness guard (task 5.5) runs only in this repository, not for an external host.
- **Recommendation:** compare the recorded prelude fingerprint against the built-in's when the
  fallback (rather than a host override) supplies it, and fail with a named diagnostic. At minimum
  add a test for a *changed* declaration shape, not just a missing name.
- **Status:** left open: whether prelude compatibility is name-only or fingerprinted is the contract this change documented in `docs/nx-ir-format.md` ("compatibility is decided by which declarations the runtime's prelude holds"), so tightening it to a fingerprint comparison would change a documented promise and could reject hosts whose prelude differs only cosmetically. The reviewer's open question 3; a decision is wanted before either the guard or the changed-shape test lands.
- **Decision:** the gap is real but the fingerprint is the wrong instrument. `docs/nx-ir-format.md:299` defines it as FNV-1a over the identity and the **source text**, so a reworded comment in `prelude.nx` flips it; gating linking on it would reject every image already in the wild after a cosmetic edit, for no safety gained. Name-only is genuinely too weak in the other direction: dropping `endInclusive` from `Range` leaves the name `Range` present and links silently.
- **Fix:** the prelude carries a version, and the check every library already goes through does the work. `nx_hir::PRELUDE_VERSION` (`"1"`) names the prelude's *contract* — bumped when a declaration's shape changes, left alone by an edit that changes no declaration — and `module_version` stamps it on the prelude's module-table entry, which was empty only because no host supplies the prelude. `resolved.version !== entryTable.version` stops being a tautology, so a mismatch is `nx-ir-link-version` with its existing message and the host's existing `allowVersionMismatch` opt-out. It applies to every supplier, not only the built-in fallback, and it travels in the image, which is what answers the finding's "an external host has no staleness test". The runtime re-exports `NX_PRELUDE_VERSION` from the generated image module.
- **Verification:** the check is no longer a tautology — the conformance expectation reads
  `links @nx/prelude.nx version "1"` and `module_version` (`crates/nx-codegen/src/ir.rs:601-606`)
  stamps `nx_hir::PRELUDE_VERSION` on the prelude's entry, so `resolved.version !== entryTable.version`
  (`runtime/typescript/src/index.ts:1617`) has two real values to compare and reports
  `nx-ir-link-version`. Both runtime tests pass and assert the right things: the mismatch test builds an
  image against `${NX_PRELUDE_VERSION}0` with a declaration name the runtime *does* hold, so only the
  version can catch it, and the opt-out test evaluates through `allowVersionMismatch`. The snapshot in
  `the_prelude_declaration_shape_is_pinned_to_the_prelude_version` holds each declaration's fields and
  field types, so the finding's `endInclusive` example fails it. The chosen instrument is the right one:
  the fingerprint does hash source text, so a comment edit would have rejected shipped images.

### ✅ Verified - RF11 `tryLinkNxIrProgram` / `tryPrepareNxIrProgram` can throw from the prelude fallback
- **Severity:** Medium
- **Evidence:** `runtime/typescript/src/index.ts:1519-1520` calls `builtInPrelude()`, which calls
  `decodeBase64` (throws a plain `Error` on a bad character, `:1490`) and `prepareNxIrModule`
  (throws `NxIrRuntimeError`, `:929-935`); neither is caught. Every other failure in
  `tryLinkNxIrProgram` becomes a diagnostic, and `tryPrepareNxIrProgram` (`:1632-1637`) advertises
  the same non-throwing contract. A corrupt or mis-generated `prelude-image.ts` therefore escapes
  the `try*` API shape. Secondary: on failure `preparedPrelude` stays `undefined`, so the
  decode+prepare is retried on every subsequent link.
- **Recommendation:** use `tryPrepareNxIrModule` and convert its diagnostics into a link diagnostic;
  make a bad base64 char a diagnostic too.
- **Fix:** `builtInPrelude` returns an `NxResult`; `decodeBase64` became `tryDecodeBase64`, and a bad character is an `nx-ir-prelude-image` diagnostic. The link's resolver wrapper reports it and answers `undefined`, so `try*` no longer throws. `preparedPrelude` is memoized only on success.
- **Verification:** `builtInPrelude` returns `NxResult<NxPreparedModule>`
  (`runtime/typescript/src/index.ts:1476-1489`), composing `tryDecodeBase64` and
  `tryPrepareNxIrModule` and memoizing `preparedPrelude` only on `prepared.ok`. Its one and only
  call site is the resolver wrapper (`:1540`), which pushes an `nx-ir-prelude-image` diagnostic and
  returns `undefined` on failure, so the link reports it as a missing module rather than throwing.
  `tryPrepareNxIrProgram` routes through `tryLinkNxIrProgram` with a `() => undefined` resolver, so
  the resolver-free path gets the same handling; `prepareNxIrProgram` is the only entry point that
  still throws, which is its documented contract. Runtime suite 295 ok / 0 not ok.

### ✅ Verified - RF12 `expressions.md`'s operator table contradicts the precedence sentence this change added
- **Severity:** Medium
- **Evidence:** the new prose at `docs/src/content/docs/reference/syntax/expressions.md:30` asserts
  "Binding is tightest at the top of the table", but the table it describes is not in precedence
  order: `==` `!=` (prec 80, the loosest binary) is at `:23`, **above** `..` `..=` (100, `:24`) and
  above `<` `<=` `>` `>=` (90, `:25`), and prefix `-` / `!` (130, the tightest) are at the bottom
  (`:27-28`). Task 10.1 asked for the new row "with their precedence" in the table; what landed is a
  row between *equality* and *relational*, plus a sentence that is true of the language and false of
  the table. (The prose's enumerated order is correct against `grammar.js:541-550`: additive 110,
  range 100, relational 90, equality 80.)
- **Recommendation:** move the `==` `!=` row below the relational row (and either hoist the two
  prefix rows or scope the sentence to the binary operators). Nit in the same block: "a braced value
  list takes one only in parentheses" over-states it — `{1..5}` is a single-value braced expression
  and is fine (and used a dozen times on the same page); only a list of two or more items needs the
  parentheses.
- **Fix:** the table is in precedence order, tightest first, with the prefix row at the top, `*` `/` `%` split from `+` `-`, and the relational row above equality; the sentence now reads as a summary of the order rather than a claim the table contradicted. The braced-list sentence says `{1..5}` is one braced value and only a list of several needs the parentheses.
- **Verification:** the table's row order now matches `grammar.js:535-554` exactly — prefix `-`/`!`
  (130), `*` `/` `%` (120), `+` `-` and the string `+` (110), `..` `..=` (100), relational (90),
  equality (80), `&&` (40), `||` (30) — and the heading line states "The table is in precedence
  order, tightest first" rather than the claim the old order contradicted. The braced-list sentence
  now reads "`{1..5}` is one braced value; a braced list of several takes each range in
  parentheses". Two new parser tests pin the relational and equality halves of the claim
  (`a_range_binds_tighter_than_a_comparison`, `a_range_binds_tighter_than_equality`), closing the
  gap RF20 named. Not reopened: member access and call are 140, tighter than everything listed, and
  are still absent from the table — pre-existing and arguably out of scope for an operator table.

### ✅ Verified - RF13 `maxRangeLength` is not honoured while evaluating defaults
- **Severity:** Low
- **Evidence:** `normalizeFields` (`runtime/typescript/src/index.ts:2949`) builds its own
  `EvalContext` with `options: {}` and evaluates record-field, prop and state defaults through it
  (`:2957`). A default containing a `forRange` therefore runs under the 1,000,000 default even when
  the host lowered `maxRangeLength` to sandbox untrusted IR. (`maxCallDepth` has the same hole, so
  the new option inherits it rather than creating it.) The `typescript-ir-runtime` delta says the
  limit is "a limit the host may set in its runtime options", with no exception for defaults.
- **Recommendation:** thread the caller's `NxRuntimeOptions` into `normalizeFields`/`normalizeValue`,
  and add a test with a `forRange` in a record-field default under a lowered limit.
- **Fix:** `normalizeFields` takes the caller's `NxRuntimeOptions` and builds its `EvalContext` with them; every call site passes `context.options` or its own. `constructComponentDescriptor`, `normalizeComponentState` and `applyComponentStatePatch` gained an optional trailing `options`, and `patchComponentState`/`normalizeActionInput` a required one. Test: a lowered `maxRangeLength` refuses a `forRange` in a state default, and the default limit admits it.
- **Verification:** `normalizeFields` now takes `options: NxRuntimeOptions` and builds its
  `EvalContext` with them (`runtime/typescript/src/index.ts:2982`, `:2993`); `grep "options: {}"`
  over the file finds nothing, and all 15 call sites pass `options` or `context.options`. The test
  at `test/runtime.test.ts:1521` refuses a `forRange` in a state default under
  `{ maxRangeLength: 10 }`. `maxCallDepth` benefits from the same threading.

  **On the widened signatures:** not a break. `constructComponentDescriptor`,
  `normalizeComponentState` and `applyComponentStatePatch` gained `options: NxRuntimeOptions = {}`
  as an optional *trailing* parameter, so every existing call still compiles. The two that gained a
  *required* parameter, `patchComponentState` and `normalizeActionInput`, are module-private
  (`function`, not `export function`), so no public surface changed.

### ✅ Verified - RF14 The TypeScript runtime recognizes a range structurally, so a float range with integral bounds iterates and an `int32` carrier is lost
- **Severity:** Low
- **Evidence:** `integerRange` (`runtime/typescript/src/index.ts:552-560`) checks only
  `$type === "Range"` and that the bounds are safe integers — not that the value came from the
  prelude's slot. So (a) any host- or user-declared record named `Range` with numeric bounds and a
  boolean `endInclusive` iterates, and (b) whether a `<Range T=float64/>` iterates depends on its
  *values*: the corpus's `0.0..1.5` is refused but `0.0..4.0` would iterate and bind `0,1,2,3`. The
  Rust interpreter is stricter here — it carries a narrow/wide carrier and `value_at` preserves it
  (`crates/nx-interpreter/src/interpreter.rs:4079-4107`), so an `int32` range yields `int32` items.
  Reachable only from host values or a hand-built image (the checker rejects both in NX source), but
  the runtime is the last line of defence at that boundary and the three backends disagree.
- **Recommendation:** resolve the iterable's declaration through the prelude slot, or at least
  document the structural contract; if the carrier matters to hosts, decide one answer across the
  interpreter, `nxRangeMap` and the IR runtime.
- **Decision:** the structural rule is the only rule available, and it is now written down and pinned
  rather than left implicit. Neither half of the finding is fixable at `integerRange`. A canonical
  value carries `$type` as a *bare declaration name with no module* — which is why
  `NxPreparedProgram.nominalShapesFor` is documented as answering more than one shape for a name —
  and a nominal type in the IR is `[1, slot, str]` with no type arguments, so `Range`'s `start` and
  `end` are typed `object` in the image and `<Range T=int/>` and `<Range T=float64/>` are one
  declaration of one shape. Nothing below the checker can recover either the declaring module or
  `T`. Provenance is therefore decided where the image does carry it, and already was: the checker
  emits a `forRange` node only when the iterable's declaration is the prelude's
  (`prelude_range_argument` tests `module_identity() == PRELUDE_MODULE_IDENTITY`), and a host value
  is normalized against the site's nominal type, which names its module by slot
  (`normalizeNominalValue` → `resolveReference(context.linked, ty.slot, ty.name)`). What is left is
  a host handing a look-alike to a range-typed site, which is the exposure every record of a shared
  name has; closing it means module-qualifying `$type` across JSON, MessagePack and the .NET SDK,
  which is a wire-format change far outside this one.
- **Decision (carrier, question 4):** the backends need not agree, because the carrier is not in the
  IR to agree on. NX's `int`/`int32`/`int64` are a distinction the checker draws; a backend binds
  items with whatever numeric types it has. The interpreter separates `Value::Int32`, `Value::Int`
  and `Value::Float`, so it binds a narrow item and refuses float bounds. Both JavaScript backends
  have one number type, so an `int32` item is an `int` — as *every* `int32` is there, not only a
  range's: `normalizePrimitiveValue`'s `int32` case range-checks and returns a plain `number` — and
  `4.0` and `4` are one value, so a float range with integral bounds iterates. Making the
  interpreter take the carrier from the checked item type instead of from the bound values was
  considered and rejected: the interpreter runs on a `LoweredModule` with no check result at all, so
  threading per-expression types in is an architectural change, not a fix. Its "the values decide"
  rule is the right rule for an engine without types, and RF15 already made it
  `start_is_narrow && end_is_narrow`.
- **Fix:** `docs/nx-ir-format.md` gains a "Recognizing a range" section stating the shape contract,
  the two erasures behind it, where provenance *is* decided, and the carrier rule; `nx-ir-format`
  gains the requirement "A range is recognized by shape, and its element type is erased" with two
  scenarios. `integerRange`'s doc says the same at the code. `a_modules_own_range_does_not_iterate`
  pins the checker half, and `the range the runtime iterates is the prelude's declaration` reads the
  built-in prelude's `Range` out of a linked program and asserts its three field names and their
  `object`/`object`/`boolean` types are the ones the runtime's literals require — the only thing
  tying those literals to `prelude.nx`, since a rename there would otherwise make every range stop
  iterating in silence. `nxRangeMap` says why it needs no `$type` guard where the IR runtime does:
  the emitter writes that call only at a site the checker proved, while the IR runtime evaluates
  images it did not emit.
- **Verification:** the reasoning holds against the artifacts. The pinned image shape
  (`prelude_image_tests.rs:156-175`) really does type `start` and `end` as `object`, so `T` is not
  recoverable below the checker and the structural rule is the only one available. The contract is
  written in all three places claimed — `docs/nx-ir-format.md:327-357` ("Recognizing a range"), the
  `nx-ir-format` delta's new requirement with its two scenarios, and `integerRange`'s doc comment
  (`runtime/typescript/src/index.ts:552-576`) — and each names where provenance *is* decided and what
  the carrier rule is. `a_modules_own_range_does_not_iterate` (`crates/nx-api/tests/ranges.rs:463`) and
  `the range the runtime iterates is the prelude's declaration` both pass. Accepting "no behaviour
  change" as the right answer here: the two halves are erasures in the IR, and closing them would be a
  wire-format change.

### ✅ Verified - RF15 `IntegerRange::count()` can overflow `u64` for a full-width closed range
- **Severity:** Low
- **Evidence:** `crates/nx-interpreter/src/interpreter.rs` `count()` computes
  `span = end.wrapping_sub(start) as u64` then `span + u64::from(end_inclusive)`. For
  `start = i64::MIN, end = i64::MAX, endInclusive = true`, `span == u64::MAX` and the `+ 1`
  overflows — a debug-build panic, a release-build wrap to `0` (silently yielding an empty list).
  Not reachable from NX source (`i64::MIN` is not writable as a literal — I confirmed
  `-9223372036854775808` is rejected), but reachable from a host-supplied `Range`. Related in the
  same struct: `narrow` is taken from `start` only, so a host record with an `Int32` start and a
  large `Int` end truncates every item through `value as i32`.
- **Recommendation:** use `checked_add`/saturating arithmetic for the `+1`, and either derive
  `narrow` from both bounds or reject a mixed-carrier range.
- **Fix:** `count()` saturates the `+ 1`, so a host-built closed `i64::MIN..=i64::MAX` stays an enormous count the operation budget refuses rather than wrapping to zero; `narrow` is now `start_is_narrow && end_is_narrow`, so a mixed-carrier host record does not truncate every item.
- **Verification:** `count()` is `span.saturating_add(u64::from(self.end_inclusive))`
  (`crates/nx-interpreter/src/interpreter.rs:5038-5047`), so the one unrepresentable count saturates
  at `u64::MAX` and the operation budget refuses it instead of the old wrap-to-zero that would have
  silently yielded nothing. `integer_range` now takes `narrow = start_is_narrow && end_is_narrow`
  (`:5078-5083`). A checked `int32` range still yields narrow items, because the checker gives both
  bounds one carrier — confirmed by running an `int32` `for`. The changed behaviour is exactly the
  intended one: a host record with an `Int32` start and a wide `Int` end now yields wide items
  rather than truncating every one through `as i32`. That the carrier rule for a mixed host record
  is unspecified is part of RF14, which now specifies it: the values decide, because the interpreter
  has no checked type to consult.

### ✅ Verified - RF16 `specs/future.md` still presents the retired text prelude as future work
- **Severity:** Low
- **Evidence:** `specs/future.md:787-795` ("Deleting the prelude") describes
  `@nx-lang/language-http`'s `prelude` option, "its shift helpers, and `server/compile.mjs`'s use of
  them" as things that still exist; `sites/playground/server/compile.mjs` no longer exists either.
  `:825-830` likewise says "The prelude arithmetic in `@nx-lang/language-core` would go with it".
  Task 10.5 only covered the integer-range-type note. Task 9.4's verification claim
  ("verify `grep -ri prelude packages/ bindings/` finds only the language prelude") is also not
  literally true: that grep returns 34 lines in 13 files — the deliberate migration sections in both
  READMEs, the two construction-time guards, their tests, gitignored `dist/` copies of those, plus
  `napi::bindgen_prelude` and the .NET `NxRange` files. The *code* removal is complete (verified:
  `prelude.ts`, `test/prelude.test.ts`, `PRELUDE_ORIGIN`, `shiftReport`, `shiftRelated`,
  `preludeOffsets`, `withPrelude`, `shiftPositionIn`, `shiftRangeOut`, `PreludeOffsets` are all gone
  from `packages/language-core/src/index.ts` and the `language-http` re-export block; no
  position-shifting logic remains in `answer.ts`).
- **Recommendation:** delete or rewrite `specs/future.md:787-795` and `:825-830`, and restate task
  9.4's check as something verifiable, e.g. `grep -rn -i prelude packages/*/src bindings/*/src`
  → only the two guard messages and `bindgen_prelude`.
- **Fix:** the answered "Deleting the prelude" section is deleted; "The catalog as a library artifact" now describes the catalog as a second source module named as an implicit import and drops the sentence about the removed prelude arithmetic; the fiddle history says "the host-context build in the wasm SDK". Task 9.4's check is restated as `grep -rn -i prelude packages/*/src bindings/*/src`, which finds exactly the two construction-time guards and the .NET SDK's `NxRange` (verified: 6 lines in 3 files).
- **Verification:** `grep -rn -i prelude specs/future.md` is now empty, so neither stale passage
  survives; "The catalog as a library artifact" reads correctly without them and the `int32` section
  RF21 points at is still there. The restated task 9.4 check runs clean in substance, but its count has
  moved since the RF6 fix: it now returns 15 lines in 5 files, because `NxRangeUpdate.cs` and
  `NxRangeProperty.cs` joined `NxRange.cs` in the SDK. Same category, wider than the task text's "the
  .NET SDK's `NxRange`" — noted under RF19 rather than reopening this.

### ✅ Verified - RF17 The `OnceLock` prelude build is safe only by an implicit single-module assumption
- **Severity:** Low
- **Evidence:** `prelude_library()` builds inside `OnceLock::get_or_init`, and that build runs
  `prepare_library_source_file` → `apply_prelude_bindings`, which would call `prelude_library()`
  again. It is saved only by the early return `if module.module_identity() ==
  nx_hir::PRELUDE_MODULE_IDENTITY`. D7 explicitly anticipates further modules under the reserved
  `@nx/` root; the moment the prelude library holds a second module, that module's preparation
  re-enters `get_or_init` and `std::sync::OnceLock` **deadlocks**.
- **Recommendation:** guard on "this module belongs to the prelude library" (e.g. a prefix or a flag
  threaded through the build) rather than on the one exact identity, and add a comment saying why.
- **Fix:** `nx_hir::PRELUDE_ROOT_PREFIX` names the reserved root, and both the `apply_prelude_bindings` re-entrancy guard and `build_resolved_program`'s prelude skip test it as a prefix, with a comment saying the deadlock it avoids.
- **Verification:** `PRELUDE_ROOT_PREFIX = "@nx/"` (`crates/nx-hir/src/lib.rs:108`) with a doc comment
  saying why it is a root rather than a list of identities, and both guards use it —
  `apply_prelude_bindings` (`crates/nx-api/src/artifacts.rs:789-798`), whose comment names the
  `OnceLock` deadlock outright, and the implicit-binding skip at `:2426`. A second module under `@nx/`
  would now return early instead of re-entering `get_or_init`, which is what D7 anticipates. The
  remaining exact-identity comparisons (`artifacts.rs:3064`, `builder.rs:250`) are about the one
  library's root path rather than re-entrancy, so they are correct as they stand.

### ✅ Verified - RF18 Diagnostic and code-shape nits in the new paths
- **Severity:** Low
- **Evidence:**
  - `infer_range`'s non-numeric message reuses the offender as the suggested `T`, so `1..5..9`
    advises `write the element form <Range T=<Range T=int/> start={…} end={…} endInclusive={false} />` —
    nonsense for that case (confirmed by running it).
  - `crates/nx-codegen/src/emit.rs` computes a `mapper` local and then duplicates the whole
    `format!` in both `if *over_range` branches; one branch could select `"nxRangeMap({}, …)"` vs
    `"Array.from({}).map(…)"` without the extra binding.
  - `packages/language-core/test/service.test.ts:20` and `:31-33` still describe the removed
    "combined text"/"the service's own shifting" mechanism and now read as wrong.
  - `packages/language-core/src/answer.ts:95-97` maps `withVersion` over **all** `report.documents`
    while `versions` holds only the client's, so a context document declared with a `version` comes
    back `version: null` — the comment says the opposite.
  - `docs/.../reference/syntax/modules.md:65` says "The prelude is three lines of ordinary NX" above
    a six-line block (the declaration itself matches `crates/nx-api/src/prelude.nx` exactly).
  - `docs/.../reference/syntax/for.md:43` is the only `…/types/#ranges` link in the docs tree;
    everything else writes `…/types#ranges`.
  - `nx-grammar-spec.md:64` still lists a nonexistent `ELLIPSIS (...)` token, which — if it existed —
    would beat `..` by longest match and invalidate the new disambiguation note at `:830-834`.
  - `bindings/node/README.md` never mentions the new `implicitImports` option, though the `sdk-node`
    delta adds it as public behaviour.
  - `NativeNxLanguageSnapshot::with_build_context` (`bindings/node/native/src/lib.rs:262-265`) calls
    `with_implicit_imports(vec![])` on any `Some`, clobbering, where the existing
    `workspace_build_context` (`:201-211`) treats `Some(empty)` as "leave it alone".
- **Recommendation:** address as cleanup; none changes behaviour today except the last two.
- **Fix:** all nine: the range-of-a-range diagnostic drops the element-form suggestion when the offender is itself a range; `emit.rs` selects the call around one shared `format!`; the two stale `service.test.ts` comments rewritten; `withVersion` leaves a document the request did not carry alone, so a context document keeps its own version; "three lines" → "one ordinary NX declaration"; `for.md`'s link is `types#ranges`; the nonexistent `ELLIPSIS` token removed from `nx-grammar-spec.md`; `bindings/node/README.md` gained an "Implicit imports" section; `with_build_context` treats `Some([])` as "leave it alone", as `workspace_build_context` does.
- **Verification:** checked all nine, the two behavioural ones by execution. `1..5..9` now reports
  "A range operand must be numeric, found `<Range T=int/>`: a range is not a numeric bound" with no
  element-form advice. `emit.rs:3155-3173` picks `(open, close)` around one `format!`.
  `service.test.ts:31-34` describes dispatch rather than combined text. `answer.ts:126-129` guards
  `withVersion` on `versions.has(answer.uri)`, with a comment that now matches the code.
  `modules.md:65` reads "one ordinary NX declaration" over a block byte-identical to
  `crates/nx-api/src/prelude.nx`. Every `#ranges` link in the docs tree is `types#ranges` — five of
  them, no `types/#ranges` left. `ELLIPSIS` appears nowhere in either grammar document.
  `bindings/node/README.md:154-170` documents `implicitImports`, including that an empty list leaves a
  build context's own alone — which is exactly what `with_build_context`
  (`bindings/node/native/src/lib.rs:257-268`) now does, via `.filter(|identities| !identities.is_empty())`.

### ✅ Verified - RF19 Task-completion mismatches
- **Severity:** Low
- **Evidence:** every task is `[x]`, but:
  - **11.1** ("Run … `cargo fmt`-gated CI equivalents … and verify all pass") — `cargo fmt --all --check` fails (RF4).
  - **9.4** — the stated grep verification does not hold as written (RF16).
  - **10.1** — the operator row did not land "with their precedence" in a precedence-ordered table (RF12).
  - **9.1** — names `bindings/node/test/sdk-node.test.ts`; the two cases landed in
    `bindings/node/test/language-snapshot.test.ts:201-232` (both exist and pass).
  - **10.3** — asks for the prelude's full source in *both* `modules.md` and the language tour's
    modules page; only `modules.md:67-73` carries it (the tour shows usage). Defensible, but the
    claim overstates.
  - **10.5** — asks for the `1..=10` spelling; `specs/future.md:43-66` uses `1..=5` /
    `type Rating = 1..=5`. Cosmetic.
  - **8.3** — the browser verification of the running playground cannot be confirmed from this
    review; the three behaviours have unit tests (`catalog.test.mjs`, `evaluate.test.mjs`) and pass.
  - Ten paths are still **untracked** (`crates/nx-api/src/prelude.nx`, `crates/nx-api/tests/`,
    `crates/nx-codegen/src/prelude_image_tests.rs`, `crates/nx-types/tests/ranges_without_a_prelude.rs`,
    `runtime/typescript/src/prelude-image.ts`, `bindings/dotnet/src/NxLang.Sdk/NxRange.cs`,
    `bindings/dotnet/tests/NxLang.Sdk.Tests/NxRangeTests.cs`, `examples/nx/ranges.nx`,
    `specs/ir-conformance/ranges/`, `src/vscode/test/grammar/range-operators.test.ts`). A
    `git commit -a` would drop all of them, including the `include_str!`'d `prelude.nx` — the build
    would not compile.
- **Recommendation:** fix RF4/RF12/RF16, correct the task text for 9.1/10.3/10.5, and `git add` the
  ten untracked paths explicitly.
- **Fix:** 11.1, 9.4, 10.3 and 10.5 corrected, 9.1 repointed at `language-snapshot.test.ts`; 10.1 is now satisfied as written because the table is in precedence order (RF12). All ten untracked paths are staged, and nothing of this change remains untracked — the two untracked directories left are other changes' proposals.
- **Verification:** **reopened — the untracked half has regressed.** The task text is correct now
  (11.1 holds: `cargo fmt --all --check` is clean, `cargo test --workspace`, `cargo clippy --workspace
  --all-targets`, `cargo test -p nx-cli` 225, `dotnet test` 158, the TS runtime suite 302 ok / 0 not ok,
  `pnpm test:grammar` 262 and `openspec validate --strict` all pass), and the original ten paths are
  staged. But the RF6 fix landed five more files that are untracked again, all of them this change's:
  `bindings/dotnet/src/NxLang.Sdk/NxRangeUpdate.cs`,
  `bindings/dotnet/src/NxLang.Sdk/NxRangeProperty.cs`,
  `bindings/dotnet/tests/NxLang.Sdk.Tests/Generated/prelude-companions.nx`,
  `bindings/dotnet/tests/NxLang.Sdk.Tests/Generated/PreludeCompanions.g.cs` and
  `bindings/dotnet/tests/NxLang.Sdk.Tests/NxRangeCompanionTests.cs`. This is not cosmetic: the SDK
  files are what `global::NxLang.Nx.NxRange_update<…>` resolves to, and
  `checked_in_dotnet_prelude_companion_fixture_matches_typegen_output`
  (`crates/nx-cli/src/typegen.rs:4046-4066`) reads the fixture pair off disk, so a `git commit -a`
  would break both `dotnet build` and `cargo test -p nx-cli`. `git add` them. While there: task 9.4's
  restated grep now returns 15 lines in 5 files rather than the three it names, because of those two
  new SDK files (RF16) — widen "the .NET SDK's `NxRange`" to its companions.
- **Fix (second pass):** the five files the RF6 fix landed are staged
  (`NxRangeUpdate.cs`, `NxRangeProperty.cs`, `prelude-companions.nx`, `PreludeCompanions.g.cs`,
  `NxRangeCompanionTests.cs`), and nothing of this change is untracked now — `git status` shows only
  the two other changes' proposal directories. Task 9.4's grep is restated as finding the two
  construction-time guards and "the .NET SDK's `NxRange` and its `NxRangeUpdate` / `NxRangeProperty`
  companions" (verified: the 15 lines in 5 files the grep now returns are exactly those).
- **Verification (second pass):** `git ls-files --others --exclude-standard` returns nothing of this
  change — the only untracked paths left are the two other changes' proposal directories, and all five
  .NET files plus the original ten are staged as `A`. So a `git commit -a` now carries the
  `include_str!`'d `prelude.nx`, the SDK companions and the fixture pair that
  `checked_in_dotnet_prelude_companion_fixture_matches_typegen_output` reads off disk. Task 9.4's
  restated text matches what its grep returns. The rest of the finding still holds: `cargo fmt --all
  --check` clean, `cargo test -p nx-cli` 226, `dotnet test` 158.

### ✅ Verified - RF20 Test-coverage gaps on the risky paths
- **Severity:** Low
- **Evidence:** the suite is otherwise strong (one named test per spec scenario in
  `crates/nx-api/tests/ranges.rs`, `ir_tests.rs`, `runtime.test.ts`, `service.test.ts`,
  `handler.test.ts`, the `ranges` conformance program, and the shipped example is compiled *and*
  evaluated by a test). What is not covered:
  - `for` over a nullable range (RF3) and a value/element-namespace `Range` (RF1).
  - TypeScript typegen with no record of the module's own (RF2) and per-module shadowing (RF8);
    a field typed `<Range.Update T=int/>` (RF6).
  - `a..b < c` grouping — only the additive half of the precedence claim is asserted in
    `crates/nx-syntax/tests/parser_tests.rs`, though the relational half is documented in both
    grammar documents.
  - TS runtime: non-integer / non-safe-integer bounds and a missing or non-boolean `endInclusive`
    (the `integerRange` validation branches at `src/index.ts:556-565` are reached only via the
    "not an object" path); the `MAX_SAFE_INTEGER` closed range the doc comment at `:541-545`
    explicitly claims terminates; the `count === limit` vs `count === limit + 1` boundary;
    `maxRangeLength` through `evaluateComponent` / `initializeComponent` / a field default.
  - An identity (rather than URI) context collision (RF9); a context document declared without an
    explicit `identity` compared against one with it.
  - `src/vscode/test/grammar/range-operators.test.ts` covers braced and `for`-header contexts only,
    not an unbraced property value (`<Slider range=0..1 />`), which goes through a different
    TextMate pattern set.
  - `NxRange_TracksThePreludeDeclaration` compares the JSON member names of an *evaluated* `1..=5`
    against `NxRange<long>`'s. It does catch a rename, a removal and a new required field (the
    rewrite builds the literal from fixed `PRELUDE_RANGE_*` names, so the construction stops
    checking), but not a new optional/defaulted field or a field *type* change
    (`endInclusive:boolean` → `int`). Reading `nx_api::prelude_library()`'s declared fields would be
    stronger.
  - `test/emitted-ir.test.mjs` / `corpus.test.mjs` also incidentally exercise the built-in fallback,
    since the corpus resolver supplies only the program's two modules — worth a comment so nobody
    "fixes" it by adding the prelude to the resolver.
- **Recommendation:** add the cases above as the corresponding findings are fixed.
- **Fix:** the remaining gaps are closed, and two of the listed items turned out not to be gaps.
  - **TS runtime `integerRange` branches.** `every shape that is not a range with integer bounds
    fails the same way` walks fourteen values through an `object`-typed parameter — `null`, a
    number, a string, an array, an object with no `$type`, one named `Interval`, fractional and
    non-safe-integer and `NaN` and string bounds, and `endInclusive` missing, `null` and a string —
    asserting `nx-ir-for` each time, then the shape that does work so a refusal cannot be the
    fixture's doing. An `object` parameter is what reaches those branches: a `<Range T=…/>` one is
    rejected by nominal normalization first.
  - **The `MAX_SAFE_INTEGER` claim.** `a closed range ending at the largest exact integer
    terminates` runs `top - 2 ..= top`, which is the case the doc comment names and the one a
    `i <= end` loop could not terminate on.
  - **The limit boundary.** `a range of exactly the limit runs and one integer more is refused`
    pins `count > limit`, including the refusal naming `11 times`.
  - **`maxRangeLength` through `evaluateComponent`.** `a lowered range limit holds while a component
    body is evaluated` covers the rendered body, beside the state default already covered.
  - **The TextMate unbraced property value is not a gap — it is invalid NX.** `rhs_expression`
    (`crates/nx-syntax/grammar.js:462`) admits an element, a literal, a signed numeric literal, a
    bare name or a braced expression, so `<Slider range=0..1 />` is a syntax error (confirmed by
    running it) and has no correct highlighting to pin. What *is* reachable and was untested is a
    braced property value and a call argument, which enter through different pattern sets from the
    braced-expression and `for`-header cases; both are now covered by `scopes a range in a braced
    property value and in a call argument`, with a comment recording why the unbraced form is absent.
  - **`NxRange_TracksThePreludeDeclaration`.** Strengthened to compare JSON value *kinds* as well as
    member names, with the expected kinds asserted outright, so `endInclusive:boolean` becoming an
    `int` fails. Reading `prelude_library()`'s declared fields from .NET was not the right lever: a
    field *type* change is already caught on the compiler side by the checked-in
    `Generated/PreludeCompanions.g.cs` fixture, whose `Patch` record is typed by the prelude's
    `Range` and its companions, and a *defaulted* field is caught by the existing member comparison
    because a default is materialized into the evaluated value.
  - **The incidental fallback coverage** is now commented, at the place it actually happens. Both
    notes in the finding were imprecise: `emitted-ir.test.mjs` has no case that reaches the prelude
    at all, and `corpus.test.mjs`'s damaged-image sweep runs against `trailing-element/input.nx`,
    which does not either. The real coverage is the corpus's main loop, whose resolver holds only
    each program's emitted modules while the `ranges` program's image links `@nx/prelude.nx`
    (verified in `specs/ir-conformance/ranges/expected/app__main.nx.stripped.nxir.txt:2`), so the
    built-in prelude supplies that slot on every run in both variants. The comment sits there, and
    `emitted-ir.test.mjs` carries the weaker, accurate form.
- **Verification:** re-ran all three and the counts hold: the TypeScript runtime suite is 302 ok /
  0 not ok, `pnpm test:grammar` 262 passing, `dotnet test` 158 passing. Each claimed case exists by
  name and is one of them — `every shape that is not a range with integer bounds fails the same way`,
  `a closed range ending at the largest exact integer terminates`, `a range of exactly the limit runs
  and one integer more is refused`, `a lowered range limit holds while a component body is evaluated`.
  The two reclassifications are right: `<Slider range=0..1 />` really is a syntax error (ran it —
  "unexpected syntax here"), and the replacement `scopes a range in a braced property value and in a
  call argument` covers the two reachable pattern sets with the reason recorded in a comment;
  `NxRange_TracksThePreludeDeclaration` now compares `JsonValueKind` per member and asserts the kinds
  outright, so `endInclusive:boolean` → `int` fails it. The corrected fallback note sits in
  `corpus.test.mjs:68-71` where the coverage actually is, with the weaker accurate form in
  `emitted-ir.test.mjs:68-72`.

### 🔴 Open - RF21 `int32` arithmetic wraps in the interpreter and widens in both JavaScript backends
- **Severity:** Medium
- **Scope:** pre-existing, not introduced by this change, and **already tracked** — `specs/future.md`
  has carried "`int32` overflow: the interpreter wraps, JavaScript does not" since
  `implicit-primitive-conversions`. What this change adds is the range path to it and the first
  actual three-backend measurement; both are now recorded in that section rather than duplicated.
  Re-found here while specifying RF14's carrier rule, because a range is one of the places a program
  meets it.
- **Evidence:** `crates/nx-interpreter/src/eval/arithmetic.rs:67,91,112` compute
  `(Value::Int32, Value::Int32)` with `wrapping_add` / `wrapping_sub` / `wrapping_mul`. Neither
  JavaScript backend has an `int32` at all: generated code emits bare `+`/`*` on doubles, and the IR
  runtime's `normalizePrimitiveValue` range-checks an `int32` at the boundary and then returns a
  plain `number`. Confirmed by running one program three ways:

  ```nx
  let lo:int32 = 2000000000
  let hi:int32 = 2000000002
  let root() = { for i in lo..hi { i * 2 } }
  ```

  the interpreter yields `-294967296 -294967294`; generated JavaScript and the NX IR runtime both
  yield `4000000000 4000000002`. It is not the range's doing — `let a:int32 = 2000000000` with
  `a + a` gives `-294967296` against `4000000000` the same way — and it needs no host value or
  damaged image, only ordinary checked NX.
- **Recommendation:** give `int32` the treatment `float32` already has. `float32` is the precedent
  and it agrees today: the emitters wrap every narrow operation in `Math.fround` and the IR carries
  the `fadd32` opcode family, so `let a:float32 = 0.1` with `a + b` produces the same f32 value under
  the interpreter and under generated JavaScript (verified — the two differ only in how `console.log`
  renders it, not in the value). `int32` was never given the parallel: there is no `iadd32` opcode
  and no narrowing wrapper in the emitted expression. So decide the semantics, state them in a
  capability, and implement them — wrapping is the C#/.NET answer and the one the interpreter
  already has, and JavaScript reproduces it with `| 0` for `+`/`-` and `Math.imul` for `*`. The
  alternative, that `int32` is a storage and boundary width only and arithmetic promotes, would mean
  dropping the interpreter's wrapping instead.
- **Status:** open, and out of scope here: it is neither introduced by ranges nor confined to them,
  and it is already queued in `specs/future.md` behind the range-enforcement change that also covers
  `int`'s ±(2^53−1) range. `docs/nx-ir-format.md` records the divergence so the carrier rule this
  change does state is not read as saying more than it does.

## New Findings Discovered During 2026-09-19 19:58 Verification

### ✅ Verified - RF22 The closed MessagePack formatter is emitted per module, so a C# library that uses one companion instantiation twice does not compile
- **Severity:** Medium
- **Introduced by:** the RF6 fix.
- **Evidence:** `closed_update_formatter_name` (`crates/nx-cli/src/typegen/languages/csharp.rs:1251-1259`)
  names the formatter from the rendered type alone — deliberately, so two instantiations cannot collide —
  and `write_header`'s file is per module, so every module with a member of that type declares the class.
  C# library output puts every module in one namespace regardless of directory, so two modules that each
  name the same instantiation emit `public sealed class NxRange_updateOfLongFormatter` twice into it.
  Reproduced with a two-module library whose `a.nx` declares
  `export type PatchA = { change:<Range.Update T=int/> }` and whose `b.nx` declares the same field on a
  `PatchB`, generated with `--csharp-namespace Demo`, then compiled: `error CS0101: The namespace 'Demo' already contains a definition for
  'NxRange_updateOfLongFormatter'`, plus two `CS0111`s. A nested module does not escape it — `sub/b.nx`
  still emits into `namespace Demo`. It reaches any companion from another assembly, so a dependency's
  generic update companion collides the same way, not only the prelude's.
- **Why the suite misses it:** `a_field_typed_by_a_prelude_companion_generates_in_both_languages` and the
  compiled `PreludeCompanions.g.cs` fixture are both single-module, and the one multi-module prelude test
  (`typescript_library_output_declares_a_prelude_range_once_in_the_helper_module`) is TypeScript, where
  the helper module already gives the declaration one home.
- **Recommendation:** emit each closed formatter once per generated namespace rather than once per
  module — the same "declare it once where the modules can all see it" move TypeScript already makes with
  its helper module. Then add the C# half of the two-module test: a library whose two modules both type a
  member `<Range.Update T=int/>`, asserting one formatter declaration across the output.
- **Fix:** a closed formatter is declared once per generated *namespace*. `render_module` takes a
  `ClosedFormatterPlacement`: single-file output is `Inline` and keeps the formatters beside the
  contracts, library output is `Shared` and declares none. `emit_library` instead collects them across
  every module — building each module's own `ModuleTypes` view, so the RF8 per-module resolution still
  decides what a name means — deduplicates by class name and writes one `_NxFormatters.g.cs`, under a
  basename no module of the library takes (the same free-name search TypeScript does for `_nx`). The
  file carries the union of the dependency `using`s of the modules that named a formatter, since a
  formatter over a *dependency's* companion renders that type unqualified; the prelude's renders
  `global::`-qualified and needs none. The file is emitted only when the library names at least one
  formatter, so no existing library output gained a file. Test:
  `csharp_library_output_declares_a_closed_prelude_formatter_once` — a two-module library, one module
  nested under `sub/`, asserting exactly one `public sealed class NxRange_updateOfLongFormatter` across
  all files, that it is in `_NxFormatters.g.cs`, that both members still carry
  `[MessagePackFormatter(typeof(NxRange_updateOfLongFormatter))]`, and that nothing warns.
  `write_library` in `typegen/test_support.rs` now creates a source's parent directory so the nested
  module is expressible. The `cli-code-generation` requirement is restated from "beside the contracts
  of a module" to once per generated namespace, with a scenario for two modules naming one
  instantiation, and task 12.8 records it.
- **Verification:** re-ran the exact reproducer that failed before — a two-module library, `a.nx` and
  `sub/b.nx`, each typing a member `<Range.Update T=int/>`, generated with `--csharp-namespace Demo`.
  One `NxRange_updateOfLongFormatter` now, in `_NxFormatters.g.cs`, and **compiling the three files in
  a clean project succeeds** where it previously gave `CS0101` and two `CS0111`s. The placement split
  is real: `emit_library` passes `ClosedFormatterPlacement::Shared` and collects across modules through
  each module's own `ModuleTypes`, so RF8's per-module resolution still decides what a name means;
  single-file output stays `Inline`. `closed_formatter_file_path` searches `_NxFormatters`,
  `_NxFormatters1`, … against the module paths, so it cannot take a module's name. The shared file is
  emitted only when a formatter exists — a library naming none gains no file.
  `csharp_library_output_declares_a_closed_prelude_formatter_once` is the reproducer's shape and is one
  of the 226 `nx-cli` tests now passing. One note, not a defect: the dependency-`using` union the fix
  carries is defensive today — an imported generic record's companion cannot be applied at all
  (`'Box.Update' is not a generic record, so it cannot be applied`), a limitation from the preceding
  `record-type-parameters` commit rather than this change, so no reachable NX source puts a
  dependency's generic companion in that file yet.

## New Findings Discovered During 2026-09-19 22:39 Verification

### ✅ Verified - RF23 Two instantiations of one generic update companion in a generated assembly do not compile, silently
- **Severity:** Medium
- **Scope:** the RF6 design rather than the RF22 fix — RF22 corrected *where* a closed formatter is
  declared, and this is about *how many* of them one assembly can hold. It reaches single-file output
  too, so it is not a library-only shape.
- **Evidence:** MessagePack's `MsgPack009` counts formatters against the **open** generic, not the
  closed type. The SDK's `NxRange_update<T>` already carries
  `[MessagePackFormatter(typeof(NxRange_updateFormatter<>))]`
  (`bindings/dotnet/src/NxLang.Sdk/NxRangeUpdate.cs:37`), which is in another assembly and so is not
  counted; one generated closed formatter is therefore fine, which is why every current test and the
  `PreludeCompanions.g.cs` fixture compile. Two are not. `export type Patch = { a:<Range.Update T=int/>
  b:<Range.Update T=float64/> }` generates `NxRange_updateOfLongFormatter` **and**
  `NxRange_updateOfDoubleFormatter`, and compiling that single file against the SDK gives, twice:
  `error MsgPack009: Multiple formatters for type NxLang.Nx.NxRange_update<T> found`. The same two
  instantiations split across two modules of a library land in `_NxFormatters.g.cs` together and fail
  identically. `nxlang typegen` emits **no warning** in either case, so this is the failure mode RF6 set
  out to remove — uncompilable C# generated in silence — for a contract that mixes an `int` range patch
  with a `float64` one.
- **Why the suite misses it:** every C# companion test and the compiled fixture use exactly one
  instantiation. `a_generic_update_companion_csharp_cannot_format_is_reported` covers the *declared*
  companion, which is a different branch.
- **Recommendation:** decide which of these the contract is, then state it in `cli-code-generation`
  beside the once-per-namespace rule:
  - suppress `MsgPack009` in the generated header as `MsgPack005`/`MsgPack006` already are — but only
    after confirming the generated resolver still picks the right formatter per closed type at runtime,
    since the analyzer is flagging a genuine ambiguity in its own model;
  - or have the member name the SDK's open `NxRange_updateFormatter<>` shim directly, if MessagePack
    can close it at a member (the RF6 note says a member names it with the type parameter's name and
    gets `CS0246`, so this needs a fresh check now that the shim lives in another assembly);
  - or warn, as the unreachable shapes already do, rather than emit a file that does not build.
  Whichever it is, add a two-instantiation case to the compiled `Generated/prelude-companions.nx`
  fixture so the answer is held by a build, not by a string assertion.
- **Decision:** the first option — suppress `MsgPack009` in the generated header — after ruling the
  second out by execution and confirming the first by round trip. The second is dead: naming the
  SDK's shim closed at the member, `[MessagePackFormatter(typeof(global::NxLang.Nx.NxRange_updateFormatter<long>))]`,
  still makes the source generator emit `new NxRange_updateFormatter<T>()` — `CS0246: The type or
  namespace name 'T' could not be found`, twice — so the shim living in another assembly changes
  nothing, and the RF6 note holds. The third would refuse an ordinary contract (an `int` range patch
  beside a `float64` one) for an analyzer's model rather than a real defect.
- **Fix:** `write_header` emits `#pragma warning disable MsgPack009` beside the two pragmas already
  there, with a comment saying what the analyzer is counting and why it is not the ambiguity it
  reads. The finding's caveat is answered by a build, not by argument: `Generated/prelude-companions.nx`
  gained `wide:<Range.Update T=float64/>` beside its `change:<Range.Update T=int/>`, so the compiled
  fixture now holds `NxRange_updateOfLongFormatter` *and* `NxRange_updateOfDoubleFormatter` over one
  open generic, and `GeneratedContract_RoundTripsThePreludeCompanions` sets `Wide.Start = 0.5` and
  `Change.End = 9` and asserts both come back — through MessagePack as well as JSON — with the other's
  fields unset. The resolver picks each member's own closed type; neither patch travelled through the
  other's formatter. `dotnet test` 158 pass, and the build is clean with the suppression and fails
  with `MsgPack009` twice without it (verified both ways). The Rust side is pinned by
  `two_instantiations_of_one_companion_each_get_a_closed_formatter`, which checks the pragma, both
  formatter declarations, that each attribute sits on the member of the matching instantiation, and
  that nothing warns. `Generated/UpdateRecords.g.cs` was regenerated for the new header line. The
  `cli-code-generation` requirement states the suppression and why, with a two-instantiation
  scenario, and task 12.9 records it.
- **Not touched, noted:** a companion *the module itself* declares is still a warning rather than a
  generated file. `MsgPack009` was one of the three reasons RF6 gave for that shape being
  unreachable, and it is now suppressed — so the branch may well generate today. It is a different
  shape (an open shim and a closed formatter for one type in *one* assembly, not two closed ones),
  it needs its own round trip to claim, and lifting the warning would change what
  `cli-code-generation` promises. Left as a follow-up rather than folded in here.
- **Verification:** all three claims confirmed by execution in a clean project against the SDK, not
  by reading the fix. (1) The reproducer compiles: `export type Patch = { a:<Range.Update T=int/>
  b:<Range.Update T=float64/> }` generates both closed formatters under the new pragma and builds
  clean, where before it gave `MsgPack009` twice. (2) **The suppression is sound at runtime** — the
  part the finding said had to be shown rather than argued. I round-tripped the generated contract
  through `MessagePackSerializer` myself with values that would expose a mix-up: `a` came back
  `{start: 7, end: 9}` as `long` with `endInclusive` unset, and `b` came back `0.5`, not truncated to
  `0`, so each member went through the formatter of its own closed type. (3) The rejected alternative
  really is dead: naming `global::NxLang.Nx.NxRange_updateFormatter<long>` at the member still makes
  the source generator emit `new NxRange_updateFormatter<T>()` → `CS0246`, twice, so the shim's
  assembly is irrelevant. Deleting the pragma from the generated file reproduces `MsgPack009` twice,
  so the suppression is load-bearing. The fixture carries the two-instantiation case and
  `GeneratedContract_RoundTripsThePreludeCompanions` asserts both members survive MessagePack as well
  as JSON. `cargo fmt --all --check` clean, `cargo test -p nx-cli` 227 (the new
  `two_instantiations_of_one_companion_each_get_a_closed_formatter` among them), `dotnet test` 158.
  The deferred follow-up above is now answered, and not in the direction the note guessed — see
  RF24.

## New Findings Discovered During 2026-09-20 00:03 Verification

### ✅ Verified - RF24 The "MessagePack cannot format this" warning is now false for the shape it was written for, and tells the author to restructure working NX
- **Severity:** Low
- **Scope:** the follow-up RF23's fix deferred, now settled by the round trip it said was needed. No
  code changed under it; what changed is that one of the warning's three reasons was suppressed.
- **Evidence:** `export type Bounds = { T:type start:T end:T }` with
  `export type Patch = { change:<Bounds.Update T=int/> }` still warns: "*'Patch.change' is typed by the
  generic update companion 'Bounds_update', which MessagePack cannot resolve a formatter for here …
  Patch 'Bounds_update' in NX and expose the record across the host boundary instead.*" It resolves
  fine now. The generated file declares `Bounds_update<T>` with its own open shim
  `[MessagePackFormatter(typeof(Bounds_updateFormatter<>))]` and the member carries no attribute, which
  `MsgPack006` used to reject and `MsgPack009` used to reject a second formatter for — both are
  suppressed in the header today. Compiled in a clean project against the SDK: **build succeeds**, and
  `MessagePackSerializer` round-trips `Patch { Change = new Bounds_update<long> { End = 9 } }` back with
  `End == 9` and `Start` unset (34 bytes). So the advice is to restructure NX that works.
- **Not the whole warning:** the *other* shape it covers is still genuinely broken and must keep
  warning. `export type Patch = { changes:<Range.Update T=int/>[] }` — the companion nested inside
  another type, so there is nowhere to put a member attribute — fails to build with
  `MsgPack006: Type must be of IMessagePackFormatter`, reported by `CSC` rather than at a line, which
  is why the file's `#pragma warning disable MsgPack006` does not reach it. Any fix has to split the
  two: the emitter currently reports them with one message and one branch.
- **Recommendation:** narrow the warning to the shape that still fails — a companion reached below the
  member's own type — and let a companion the module declares generate without one, since it compiles
  and round-trips. Add the round trip to the compiled fixture the way RF23 did, so the claim is held by
  a build. Then restate `cli-code-generation`, whose current sentence covers both shapes at once ("one
  the module declares, or one nested inside another type — SHALL be reported as a warning … rather than
  generated into a file that does not compile"), and correct the stale reason in the emitter comment at
  `crates/nx-cli/src/typegen/languages/csharp.rs:122-127`, which still gives "a second formatter for a
  companion this file declares is `MsgPack009`" as a reason after RF23 suppressed it.
- **The split is one axis narrower than the finding has it.** Reproducing all four shapes: what decides
  the outcome is **which assembly declares the companion**, not where in the member's type it sits. A
  companion the *generating library* declares compiles and round-trips both at the member's own type
  *and* below it — `Schedule { patch:<Range.Update T=int/> many:<Range.Update T=int/>[] }` over the
  `Range` that `update-records.nx` declares itself builds clean and returns `End == 9` and
  `Many[0].Start == 2`. Only a companion from **another assembly** needs the member to name a closed
  formatter, and only that one fails when the member's own type is not the instantiation. So the
  finding's "a companion reached below the member's own type" would still have warned for a working
  shape, the nested case of the library's own companion.
- **Fix:** the warning now skips any companion `graph().declaration(name)` finds — the generating
  library's own, at the member's type or below it — and fires only for one from another assembly that
  no closed formatter reached. Its message says that shape rather than the general one: "*reaches the
  generic update companion 'Range_update' below its own type, and that companion is declared in another
  assembly … Type the member as 'Range_update' itself, or patch it in NX*". The emitter comment is
  rewritten to the real reason: the generator reaches a library-declared companion through the
  declaration in the compilation, and a foreign one only through the member's attribute, below which
  `MsgPack006` comes from `CSC` itself where no file-level pragma reaches it — which is why that shape
  alone is named. Held by a build, as RF23 was: `Generated/update-records.nx`'s `Schedule` gained both
  `patch` and `many`, generated with no warning and no member attribute, and
  `AppliedTypeFields_RoundTripAsTheirInstantiations` asserts both come back through MessagePack and
  JSON. `a_generic_update_companion_csharp_cannot_format_is_reported` is inverted to match: the foreign
  nested case warns, the library's own warns nowhere at either depth, and the prelude's at the member's
  type still resolves through its closed formatter. The remaining failure is still a real one —
  `many:<Range.Update T=int/>[]` over the *prelude's* companion still gives
  `CSC : error MsgPack006: Type must be of IMessagePackFormatter`, confirmed by compiling it. The
  `cli-code-generation` requirement and its scenario are restated on the assembly axis, and task 12.10
  records it.
- **Not chased:** a field typed by a *dependency library's* generic update companion is detected by
  neither branch — `generic_update_companion` resolves through the graph or the prelude, and a
  dependency's companion is in neither — so it gets no closed formatter and no warning. Whether it
  compiles is untested. Out of this finding's scope and not raised by the review; worth its own look.
- **Verification:** **the correction to the finding is right, and I confirmed it the way the finding
  should have been.** Compiled the library's own companion at both depths in a clean project —
  `export type Patch = { change:<Bounds.Update T=int/> many:<Bounds.Update T=int/>[] }` over the
  module's own generic `Bounds` — and it builds clean and round-trips through `MessagePackSerializer`:
  `Change.End == 9` with `Start` unset, `Many[0].Start == 2`, `Many[1].End == 5`. So the axis is which
  assembly declares the companion, not depth, and my recommendation would indeed have kept warning for
  the nested case of a library's own companion, which works. The warning now fires on exactly the shape
  that still fails: `many:<Range.Update T=int/>[]` over the prelude's companion warns with the new
  message and still gives `CSC : error MsgPack006` when compiled, while the module's own warns nowhere
  at either depth and `change:<Range.Update T=int/>` resolves silently through its closed formatter —
  the three cases `a_generic_update_companion_csharp_cannot_format_is_reported` now asserts. The
  emitter comment at `csharp.rs:122-130` is rewritten to the real reason and the stale `MsgPack009`
  line is gone; the `cli-code-generation` requirement and its scenario are on the assembly axis. The
  claim is held by a build as asked: `Schedule` in `Generated/update-records.nx` carries both `patch`
  and `many`, and `AppliedTypeFields_RoundTripAsTheirInstantiations` asserts both survive MessagePack
  and JSON. `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets` clean,
  `cargo test --workspace` and `cargo test -p nx-cli` (227) pass, `dotnet test` 158.
- **Follow-up resolved, nothing to chase:** the dependency-library companion the note left untested is
  **unreachable from NX source**. Applying an imported generic record's companion is refused by the
  checker — `import { Box } from "../boxes"` with `<Box.Update T=int/>` gives *"'Box.Update' is not a
  generic record, so it cannot be applied"* — a limitation of the preceding `record-type-parameters`
  commit rather than of this change. So no contract can reach the branch that detects neither, and
  there is no shape to compile. Confirmed the alias route is blocked too: a dependency that exports
  `export type IntBoxPatch = <Box.Update T=int/>` — legal in the library that declares `Box` — is
  refused at the *consumer's* use of the alias, with the same diagnostic. Worth revisiting only if
  that limitation is lifted, which is now recorded in `specs/future.md` under "A dependency's generic
  record can be named but not patched", together with the warning that lifting it without giving
  typegen a dependency arm would turn this branch into silently uncompilable C#.
  Probing it also turned up an unrelated pre-existing gap, recorded beside it as "`typegen` emits C#
  for a type error `nxlang run` rejects": `patch:Box.Update` written with no type arguments is
  refused by the checker but generated by `nxlang typegen`, which emits `Box_update` with no
  arguments — `CS0305`, three times. Both reproduce unchanged at `a3882f3`, so neither belongs to
  this change.

## Verified correct (no action needed)

Recorded because these were the review's designated risk areas:

- **`InferenceContext::common_supertype`'s new applied-type arm is correct and complete.** The guard
  `!lhs_name.args().is_empty() || !rhs_name.args().is_empty()` is safe: `NamedType.args` is populated
  only for *record* instantiations — every `NamedType::applied` / `with_args` site in
  `crates/nx-types` is a record path, and component type parameters are `Type::Parameter` /
  `TypeParameterRef`, never `args` — and
  `openspec/specs/record-type-parameters/spec.md:59-60` forbids a generic record from being abstract
  or having `extends`, so neither lineage walk below could return anything but the declaration
  itself. **`common_component_supertype` needs no parallel change.** Equality is
  `NamedType::PartialEq` = `is_same_declaration_as` + `args ==`, i.e. exactly the notion assignment
  uses, and aliases are resolved before comparison — I confirmed `<Range T=Count/>` joined with
  `<Range T=int/>` yields `list <Range T=int/>[]`, not `object[]`. The `else` branch's
  `generic_common_supertype` (`semantics::common_supertype`) also answers `object` for two differing
  instantiations via `type_satisfies_expected` both ways, so **that function needs no parallel
  change** either; its other caller, the interpreter's `runtime_type_of_value`, only ever sees bare
  record names with no args.
- **The prelude injection.** Built once behind `OnceLock` (asserted by `Arc::ptr_eq`), bound last on
  both pipelines through the existing already-bound-then-skip path, inserted as `libraries[0]`, and
  hashed into the program fingerprint through `library.fingerprint` (which covers the source text).
  Positions are untouched, a library's exports exclude it, its source is always in the source map, and
  `@nx/prelude.nx` is refused as a workspace identity. Shadowing works for the derived companions
  too — I confirmed a module declaring its own `type Range = { low:int high:int }` resolves
  `Range.Update` and `Range.Property` to its own.
- **The rewrite leaves no trace.** `Expr::Range` is added to `scope.rs`'s
  `UndefinedIdentifierChecker` and `components.rs`'s `collect_handler_rewrites_in_expr`; the other
  `Expr` traversals with wildcard arms (`convert_literals`, `fold_constant`,
  `apply_string_conversions`, the `shape` test helper) are all keyed to specific kinds that a range
  cannot be, and the builder has an explicit "must be rewritten" diagnostic. The
  `endInclusive` literal is typed, and a test asserts all five construction positions rewrite.
- **`forRange` encoding.** Kind 22 with the `for` layout in `ir_image.rs`, the same layout in the TS
  runtime (`["int","str","optSlot","optStr","node","node"]`) and in `docs/nx-ir-format.md:136`;
  `ranges-v1` is listed exactly when a `forRange` is present and not for a mere construction;
  `NX_IR_SCHEMA_VERSION` stays 4; `explain` renders it as a `for` over a range; the prelude is
  emitted when named and, for an every-module request, only when referenced; an unused prelude leaves
  the module table untouched and no `expected/` file of another corpus program changed.
- **Three-backend semantics agree** on exclusive/inclusive, empty, reversed, one-element-closed,
  iteration order, zero-based index and not materializing the integers. All three compute the count
  once as `end > start ? end - start + inclusive : (end == start && inclusive ? 1 : 0)`
  (`IntegerRange::count`, `nxRangeMap`, the runtime's `integerRange`). The interpreter test compares
  against the generated-JS answer rather than a literal, and the conformance corpus compares the IR
  runtime against both. The two carrier-related divergences are RF14/RF15.
- **Grammar and editor.** `prec.left(100)` sits between additive (110) and relational (90);
  `real_literal` requires a digit after the dot, so `1..5` lexes as two `int_literal`s (confirmed in
  the generated lexer states, `parser.c:5176-5197`), and longest match is confirmed for `..=` over
  `..` (`:5131-5135`) and `..` over `.` (`:5168-5171`). `grammar.js`, `grammar.json`,
  `node-types.json` and `parser.c` share one mtime and are consistent; the `conflicts` array is
  unchanged. The TextMate operator rule precedes both the dot and the assignment rules, the float
  rule gained `\.(?!\.)`, and `pnpm test:grammar` passes 261/0.
- **.NET `NxRange<T>`** matches the emitter's wire shape exactly: `[MessagePackObject]`,
  `sealed class`, three properties in declaration order with `[Key("…")]` + `[JsonPropertyName("…")]`
  camelCase names, no `$type` member (which is correct — `$type` is only emitted for
  abstract/union hierarchies), value equality over the three, parameterless ctor. Cross-checked
  against the checked-in `Generated/UpdateRecords.g.cs`. `T:type` is never emitted as a field.
  C# maps nested positions correctly (`<Range T=int/>[]?` → `global::NxLang.Nx.NxRange<long>[]?`)
  and emits no `Range`/`NxRange` declaration.
- **The prelude fallback in the TS runtime** asks the host's resolver first, applies at any depth
  (it wraps `options.resolve` rather than seeding slot 0), caches the prepared module at module
  scope, and is safe to share (`NxPreparedModule` is fully `readonly`; `NxIrImage`'s only mutable
  state is idempotent lazy memos). `decodeBase64` is correct. The staleness test regenerates under
  `NX_UPDATE_PRELUDE_IMAGE=1` and names that command otherwise.
- **The text-prelude retirement** is complete in code: nothing prepends text or shifts positions,
  `answer.ts` has no dead parameters or half-removed transform, the `prelude` option is a
  construction-time `TypeError` naming `context` in both factories (asserted with
  `@ts-expect-error`, so the type-level removal is covered), and the wasm↔HTTP parity test now
  compares both transports over the one mechanism. Context documents join every query's document
  set, positions are the document's own, and a context document's diagnostic is reported under its
  own URI (tested against the real compiler).
- **Playground.** `PRELUDE_IDENTITY` in `sites/playground/src/compile/catalog.ts:46` is byte-identical
  to `nx_hir::PRELUDE_MODULE_IDENTITY`, and `classifyDiagnostic` maps such a label to
  `origin: "program"` with `span: null`.
- **`for` in element position** works over a range (`<row> for i in 0..3 { <Star /> } </row>`
  rendered three children), and `examples/nx/generic-records.nx` no longer shadows the prelude
  (`Range` → `Bounds`, consistently including the alias and both companions).

## Questions

- RF1: should the operator be *rejected* when `Range` is a value or a component (the delta's literal
  reading), or should the rewrite instead carry the prelude's origin so the construction resolves by
  address and keeps working? The second is closer to D3's "sugar that ends at the checker" and would
  also harden the interpreter's by-name `Range` check, but it is a larger change than the
  diagnostic.
- RF6: **answered** — yes, it crosses. A prelude declaration is an ordinary declaration, so its
  companions are generated like a user record's: into the helper module in TypeScript, and as
  `NxRange_update<T>` / `NxRange_property` in the SDK for C#. `cli-code-generation` and
  `dotnet-binding` now say so. See the finding.
- RF10: **answered** — neither. The fingerprint hashes source text, so gating on it would reject
  images over a reworded comment; the prelude now carries a *contract* version that the ordinary
  `nx-ir-link-version` check compares, for every supplier rather than only the built-in one. See the
  finding.
- RF14: **answered** — they need not, and cannot. The carrier is not recorded in the IR: type
  arguments are erased, so `Range`'s bounds are typed `object` and no consumer knows `T`. The item's
  carrier is whatever numeric type the evaluating backend has, which is why the interpreter binds
  `int32` and JavaScript binds a `number` — as it does for every `int32`, not only a range's.
  `nx-ir-format` now says so. See the finding.
- RF8: **answered** — no. An import binds in the module that wrote it; a declaration is library-wide
  only because a same-library peer is visible without an import. `ModuleTypes` now encodes exactly
  that split, and `cli-code-generation` states it. See the finding.

## Summary

**Verification status (2026-09-20 00:37).** All 24 findings are now settled: 23 ✅ Verified and one
open — **RF21**, which is out of scope and tracked in `specs/future.md`. RF24 is verified in this
fourth pass, and its fix corrected the finding rather than just implementing it: the axis is which
assembly declares the companion, not where in the member's type it sits, which I confirmed by
compiling and round-tripping both depths. The dependency-companion case that fix left untested turns
out to be unreachable from NX source, so nothing is outstanding behind it. No new findings this pass.
Everything in the repo passes — `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
`cargo fmt --all --check`, `cargo test -p nx-cli` (227), `dotnet test` (158), the TypeScript runtime
suite (302 ok / 0 not ok), `pnpm test:grammar` (262) and `openspec validate --strict`.
The paragraphs below are the
original review's and are kept as written.

**Fix pass (2026-09-19).** **RF19** and **RF22** are 🟡 Fixed, awaiting verification. RF19: the five
.NET files are staged and task 9.4's grep text widened to the SDK's companions. RF22: a closed
formatter is now declared once per generated namespace — library output writes them all into a shared
`_NxFormatters.g.cs` and the modules only name them — restated in the `cli-code-generation`
requirement with a two-module scenario, and covered by
`csharp_library_output_declares_a_closed_prelude_formatter_once`. **RF21** is left 🔴 Open on purpose:
giving `int32` the `float32` treatment is a semantics decision plus an opcode family across three
backends, which is a change of its own. Re-run after the fixes: `cargo test --workspace`,
`cargo clippy --workspace --all-targets`, `cargo fmt --all --check`, `cargo test -p nx-cli` (226),
`dotnet test bindings/dotnet` (158) and `openspec validate --strict` all pass.

**Fix pass 2 (2026-09-19).** **RF23** is 🟡 Fixed, awaiting verification: the generated C# header now
suppresses `MsgPack009`, chosen over the two alternatives after ruling the shim-at-the-member one out
by execution (`CS0246` again, even with the shim in another assembly) and confirming this one by round
trip — `Generated/prelude-companions.nx` gained a `float64` patch beside its `int` one, so the
compiled fixture holds two closed formatters over one open generic and
`GeneratedContract_RoundTripsThePreludeCompanions` proves each member came back through its own.
`Generated/UpdateRecords.g.cs` regenerated for the header line. **RF21** remains 🔴 Open, for the
reason recorded with it. Re-run: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
`cargo fmt --all --check`, `cargo test -p nx-cli` (227), `dotnet test bindings/dotnet` (158) and
`openspec validate --strict` all pass.

**Fix pass 3 (2026-09-20).** **RF24** is 🟡 Fixed, awaiting verification, and the split turned out to
run on a different axis than the finding proposed: what decides whether MessagePack can resolve a
formatter is which assembly declares the companion, not how deep in the member's type it sits. A
companion the generating library declares works at the member's type *and* below it, so the warning
now fires only for a foreign companion reached below the member's own type — the one shape that still
gives `CSC : error MsgPack006`. `Generated/update-records.nx` carries both depths of the library's own
companion, round-tripped by `AppliedTypeFields_RoundTripAsTheirInstantiations`. **RF21** remains
🔴 Open, for the reason recorded with it. Re-run: `cargo test --workspace`,
`cargo clippy --workspace --all-targets`, `cargo fmt --all --check`, `cargo test -p nx-cli` (227),
`dotnet test bindings/dotnet` (158, 0 warnings) and `openspec validate --strict` all pass.


The design is sound and the implementation follows it closely: the prelude really is an ordinary
library bound last on both pipelines, the operators really do end at the checker, `forRange` grows
the IR the documented way, and the test suite has a named case per spec scenario across five
languages plus a conformance program that compares three backends. Everything currently in the repo
passes: `cargo test --workspace`, `cargo clippy`, `cargo test -p nx-cli`, `dotnet test`, all
TypeScript suites, the grammar tests, and `openspec validate --strict`.

RF21 records that `int32` arithmetic wraps under the interpreter and widens under both JavaScript
backends, from ordinary checked NX and with no range involved. It is out of scope for this change and
was already tracked in `specs/future.md`; what is new is the range path to it and a measured
three-backend comparison, both added there.

Two High findings block archiving. **RF1** is a real soundness gap: a module that declares a value
or component named `Range` accepts `1..5` and then fails at evaluation, at NX IR codegen and in
generated TypeScript, with no diagnostic anywhere — which is exactly what the `range-hidden`
requirement exists to prevent. **RF2** makes generated TypeScript uncompilable whenever the output
references a prelude type but declares no concrete record of its own. Both are narrow to fix and
both are missed by the current tests for the same reason: the new cases all use the happy shape.

Beyond those, **RF3** (nullable range iterable), **RF6** (`Range.Update` in a contract) and **RF4**
(`cargo fmt` breaks CI, contradicting task 11.1) should be fixed before archiving; **RF5** would
ship a broken tracked `dist`; **RF9** leaves one `language-http-service` scenario half-enforced; and
**RF12**/**RF16** are documentation statements this change made untrue. The remaining findings are
polish, hardening at the host boundary, and test gaps. The late fix to
`InferenceContext::common_supertype` I was asked to scrutinize is correct and complete, and neither
`common_component_supertype` nor `nx_types::semantics::common_supertype` needs a parallel change —
reasoning recorded above.
- **Status (superseded by the Fix below):** partly addressed: the gaps belonging to fixed findings now have tests (`for` over a nullable range and a value/element-namespace `Range`, TypeScript typegen with no record of the module's own, an identity rather than URI context collision, `maxRangeLength` through a state default), and `a..b < c` and `1..5 == 1..=4` grouping are asserted in `crates/nx-syntax/tests/parser_tests.rs`. A field typed `<Range.Update T=int/>` (RF6), per-module shadowing (RF8) and a changed prelude declaration shape (RF10) now have theirs too, listed with those findings. Still open: an unbraced property value in the TextMate tests, and reading `prelude_library()`'s declared fields in `NxRange_TracksThePreludeDeclaration`. The structural range branches and the carrier (RF14) are covered by `a_modules_own_range_does_not_iterate` and `the range the runtime iterates is the prelude's declaration`.


## Follow-up: RF6, RF8, RF10 and RF14

All three are fixed, and each answered the open question it was waiting on; the decisions and the
work are recorded with the findings. In short:

- **RF10** — the prelude carries a contract version (`nx_hir::PRELUDE_VERSION`) that the existing
  `nx-ir-link-version` check compares, rather than the source-text fingerprint the finding proposed.
  A snapshot of the prelude's declaration shape sits beside the version so it cannot fall behind.
- **RF8** — `ModuleTypes` replaced the graph-wide prelude lookups with a per-module view: a
  declaration hides a prelude name library-wide, an import only in the module that wrote it. This
  also fixed a second site the finding did not reach, `record_type_params`.
- **RF6** — the prelude's `Range.Update` and `Range.Property` are generated like a user record's
  companions: into TypeScript's helper module, and as `NxRange_update<T>` / `NxRange_property` in
  the .NET SDK. Fixing it surfaced a pre-existing C# defect — no member typed by *any* generic
  update companion compiled, because MessagePack's source generator cannot resolve the open generic
  such a companion carries. The emitter now declares a closed formatter for each instantiation a
  member's own type is, and warns for the shapes MessagePack has no working form for instead of
  emitting a file that does not build.

- **RF14** — nothing changed behaviour, because neither half of the finding is fixable at the site
  it names: a canonical value carries `$type` as a bare name with no module, and a record's type
  arguments are erased in the IR, so no consumer can recover the declaring module or `T`. The
  structural rule is now a written contract — in `docs/nx-ir-format.md`, in an `nx-ir-format`
  requirement, and at `integerRange` — with the places provenance *is* decided named, and it is
  pinned by two tests. The carrier question is answered the same way: the carrier is not in the IR,
  so a backend binds items with whatever numeric types it has.

Full suite after the four: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
`cargo fmt --all --check`, `cargo test -p nx-cli` (225), `cargo build -p nx-ffi` +
`dotnet test bindings/dotnet/NxLang.sln` (158), the TypeScript runtime suite (298 ok / 0 not ok) and
`pnpm -r test` across the workspace all pass.
