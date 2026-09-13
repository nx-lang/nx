# Review: resolve-inherited-companion-field-types

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `specs/cli-code-generation/spec.md`

**Reviewed code:** working-tree diff for
`crates/nx-cli/src/typegen/model.rs`,
`crates/nx-cli/src/typegen.rs`,
`crates/nx-api/src/artifacts.rs`,
`crates/nx-codegen/src/tests.rs`,
`bindings/dotnet/tests/NxLang.Sdk.Tests/NxUpdateRecordTests.cs`,
`docs/add-update-actions-followsup.md`.
Emitter code read for context (`typegen/languages/typescript.rs`, `typegen/languages/csharp.rs`).
The other modified files in the working tree (`crates/nx-hir/src/scope.rs`,
`crates/nx-language-service/src/lib.rs`, `crates/nx-syntax/tests/parser_tests.rs`,
`crates/nx-types/tests/*`) belong to other in-flight changes and were excluded.

**Verification run:** `cargo test --workspace` (all green), `cargo test -p nx-cli` (178 passed),
`dotnet test --filter NxUpdateRecordTests` (14 passed), `cargo fmt --all --check` (clean),
`cargo clippy -p nx-cli --all-targets` (no new warnings in changed code),
`openspec validate resolve-inherited-companion-field-types --strict` (valid).
Additionally exercised the CLI (`nxlang typegen`) on scratch libraries to check the peer path and
the visible-name-collision path end to end; both emit correct output.

## Findings

### ✅ Verified - RF1 The peer branch picks a bare exported name without checking that the generating module's imports do not already claim it, so TypeScript types the inherited field with the wrong `Tag`
- **Severity:** Medium
- **Evidence:** `crates/nx-cli/src/typegen/model.rs:1121-1133` returns `item.name` unconditionally
  for a peer origin. The dependency branch guards the symmetric case (`name_is_taken` at
  `model.rs:1142-1146` qualifies to `<library>.<name>` when a graph declaration or an existing
  import already owns the name), but the peer branch has no equivalent check. The TypeScript
  emitter resolves `imported_types_by_visible_name` *before* `graph.owner_module`
  (`languages/typescript.rs:631-645`), so an explicit import wins over the peer declaration.
  Reproduced with the CLI: library `ui` = `tag.nx` (`export type Tag = a | b`), `named.nx`
  (`export abstract type Named = { name:string tag:Tag }`), `user.nx`
  (`import { Tag } from "../other"` + `export type User extends Named = { email:string other:Tag }`).
  Generated `user.ts`:

  ```ts
  import type { Tag } from "other";
  export interface User_update { $type: "User.Update"; name?: string; tag?: Tag; ... }
  ```

  The inherited `tag` is silently typed by `other`'s `Tag` record instead of `ui`'s `Tag` union,
  and it compiles. C# renders bare `NxOptional<Tag>` for the same field, which resolves to the peer
  — so the two emitters disagree about what the field means.

  This is an incompleteness rather than a regression: before the change the field also carried the
  bare name `Tag` and the old `warn_about_unresolvable_inherited_fields` stayed silent because the
  graph declared `Tag`. But this pass is the code that now owns "what name should this field
  carry", and it already solves the mirror-image problem one branch away.
- **Recommendation:** In the peer branch, apply the same taken-name test against
  `imported_types` (a peer exported name shadowed by an import is not safely referenceable as a
  bare name) and either qualify it, or warn with the D5 wording. A regression test with the shape
  above, asserting `user.ts` does not type `tag` with the imported symbol, would lock it down.
- **Fix:** The peer branch now checks `imported_types` for a visible name equal to the peer's
  exported name and warns with the D5 wording ("the import of 'Tag' from 'other' shadows the peer
  declaration 'Tag' it resolves to; import 'Tag' under a qualifier so the generated code can
  reference both"), leaving the name as written. A peer cannot be qualified the way a dependency
  can, since the graph declares it under its bare name and neither emitter resolves a qualified
  peer, so the warning is the honest outcome. Design D2 step 3 records the rule. Regression test
  `library_update_companion_warns_when_an_import_shadows_the_peer_an_inherited_field_names` in
  `typegen.rs` asserts the warning for both languages.
- **Verification:** Confirmed. `model.rs:1128-1152` checks `imported_types` for a visible name
  equal to the peer's exported name before returning the bare name, and the kind check moved ahead
  of it so an unrepresentable peer kind still reports its own reason. Re-ran the exact CLI
  reproduction from the Evidence above: generation now prints `Update companion 'User_update'
  inherits field 'tag' typed 'Tag' from a base declared in another module, and the import of 'Tag'
  from 'other' shadows the peer declaration 'Tag' it resolves to; import 'Tag' under a qualifier so
  the generated code can reference both`. Leaving the written name matches the requirement's
  "SHALL leave the field's written type name in the generated output", and I agree a peer is not
  qualifiable without an emitter change: `ImportedType` carries a `library_name` that both emitters
  turn into a package/namespace reference, so there is no shape for a same-library alias. Design D2
  step 3 records the rule. The regression test asserts the warning for both languages; it does not
  assert the generated output, which is fine since the warning is the behavior under test. See RF8
  for the spec side of this.

### ✅ Verified - RF2 The single-source (non-library) generation path and the wildcard-import scenario lost their end-to-end coverage
- **Severity:** Low
- **Evidence:** The replaced test `update_companion_warns_when_an_inherited_field_type_is_not_imported`
  drove `generate_types_with_warnings` (single `.nx` file input, i.e.
  `ExportedTypeGraph::from_artifact_with_warnings`) for both a selective import and a wildcard
  import (`import "./named"`). All three replacements in `crates/nx-cli/src/typegen.rs:2660-2857`
  use `generate_library_types_with_warnings`. The source path is now only asserted at the model
  level (`model.rs` tests `inherited_companion_field_typed_by_a_dependency_export_synthesizes_an_import`
  and friends), so no test checks that the synthesized `ImportedType` actually reaches the
  emitters in source mode, and nothing at all covers a wildcard import reusing an already-present
  `ImportedType` instead of appending a duplicate. `update_companion_carries_fields_inherited_from_an_imported_base`
  (`typegen.rs:2582`) is source mode but its inherited field is `string`, so it exercises none of this.
- **Recommendation:** Keep one source-mode generation assertion (selective import) and one
  wildcard-import assertion alongside the library-mode tests; both are a few lines given the
  existing `generate_types_with_warnings` helper.
- **Fix:** `source_update_companion_resolves_an_inherited_field_type_through_either_import_form`
  drives `generate_types_with_warnings` for a selective and a wildcard import, in both languages,
  asserting `tag?: Tag` / `NxOptional<global::Test.Named.Tag>` with no `User_update` warning, and
  that the wildcard case keeps the bare visible name (no `named_Tag`), which is what reusing the
  existing `ImportedType` rather than appending a duplicate produces. Note that single-file
  TypeScript output emits no import lines at all (`emit_single_file` passes
  `include_imports = false`, predating this change and applying to explicit imports too), so in
  source mode the visible name on the field is the observable.
- **Verification:** Confirmed. `typegen.rs:2716-2775` loops over both import forms and both
  languages against `generate_types_with_warnings`, asserting `tag?: Tag`,
  `NxOptional<global::Test.Named.Tag>`, absence of `named_Tag`, and no `User_update` warning. I
  checked the `include_imports = false` claim at `languages/typescript.rs:99-104` and `:140-157`:
  single-file TypeScript output suppresses both the import lines and the assumed-package warning,
  and that predates this change, so the visible name on the field really is the only observable
  there. The wildcard form does exercise the reuse branch (`model.rs:1155-1160`) — a duplicate
  import would have surfaced as `named_Tag`.

### ✅ Verified - RF3 Two negative assertions in the new tests are vacuous
- **Severity:** Low
- **Evidence:** `crates/nx-cli/src/typegen.rs:2699` and `:2719` assert no warning contains
  `"does not import"`. That substring was only ever produced by
  `warn_about_unresolvable_inherited_fields`, which this change deletes; `grep -rn "does not import" crates/`
  finds no warning site left. The assertions can never fail, including if the new pass regresses
  and starts warning for the resolvable case.
- **Recommendation:** Assert on the warning family that can actually fire — e.g. no warning
  mentioning `User_update` — as the alias test already does at `typegen.rs:2760` and `:2782`.
- **Fix:** Both assertions now check that no warning mentions `User_update`.
- **Verification:** Confirmed at `typegen.rs:2677` and `:2697`. `grep -rn "does not import" crates/`
  no longer matches anything in `nx-cli`, so no vacuous assertion is left behind.

### ✅ Verified - RF4 The peer path and the qualified-visible-name path are asserted only in the model, never through an emitter
- **Severity:** Low
- **Evidence:** `inherited_companion_field_typed_by_a_peer_resolves_without_an_import`
  (`model.rs`) asserts the `TypeRef` and that no `ImportedType` was added, but nothing checks that
  TypeScript then emits the cross-module relative import that makes the bare name resolve; the
  claim rests on `add_imported_symbol`'s `graph.owner_module` fallback
  (`languages/typescript.rs:646-661`). Likewise
  `synthesized_import_is_qualified_when_the_module_declares_the_same_name` stops at
  `visible_name == "named.Tag"` and never checks that the emitters sanitize it. I confirmed both by
  running `nxlang typegen` on scratch libraries — TypeScript emits `import type { Tag } from "./tag";`
  for the peer case and `import type { Tag as named_Tag } from "named";` plus `tag?: named_Tag;`
  for the collision case, and C# emits `global::Test.Named.Tag` — so the behavior is correct today,
  but nothing in the suite would catch a regression in either.
- **Recommendation:** Add the peer case and the collision case to the `typegen.rs` generation
  tests, asserting the emitted import line and the field type for both languages.
- **Fix:** `library_update_companion_resolves_an_inherited_field_type_declared_by_a_peer` asserts
  `import type { Tag } from "./tag";` plus `tag?: Tag;` in TypeScript and `NxOptional<Tag>` (same
  namespace, unqualified) in C#, with no warnings.
  `update_companion_qualifies_a_resolved_import_that_collides_with_a_local_declaration` asserts
  `tag?: named_Tag;` beside `own?: Tag;` in TypeScript and `NxOptional<global::Test.Named.Tag>`
  beside `NxOptional<Tag>` in C#.
- **Verification:** Confirmed. The peer test (`typegen.rs:2782-2822`) asserts the relative import
  line `import type { Tag } from "./tag";` alongside `tag?: Tag;`, plus unqualified
  `NxOptional<Tag>` in C# and empty warnings — that is the whole recommendation for that case.
  The collision test (`typegen.rs:2832-2876`) asserts `tag?: named_Tag;` beside `own?: Tag;` and
  `global::Test.Named.Tag` beside `NxOptional<Tag>`, which covers the sanitization of the field
  type in both languages. Residual, accepted: that test runs in source mode, where TypeScript emits
  no imports, so the *aliased import line* for a qualified visible name is still unasserted. I
  checked it by hand — library-mode generation of the same shape emits
  `import type { Tag as named_Tag } from "named";` matching `tag?: named_Tag;` — and the plain
  import-line rendering is already asserted by the dependency test at `typegen.rs:2670`, so only
  the `X as Y` spelling rests on manual confirmation. Not worth another test.

### ✅ Verified - RF5 Test helpers are duplicated, and the new C# helper leaves three existing copies of the same code in place
- **Severity:** Low
- **Evidence:** `write_library` is defined identically in `crates/nx-cli/src/typegen.rs:2623` and
  `crates/nx-cli/src/typegen/model.rs:2136`. `csharp_companion_body` (`typegen.rs:2649`) factors out
  the `split_once("public sealed class X") / split("public sealed class").next()` dance, but three
  other sites still do it inline (`typegen.rs:2471`, `:2543`, `:2897`) — and the diff *reformatted*
  those three rather than switching them to the new helper, which reads as churn without the payoff.
- **Recommendation:** Move `write_library` into a shared test-support module for the `typegen`
  tree, and convert the three remaining inline splits to `csharp_companion_body`.
- **Fix:** `write_library` lives in `crates/nx-cli/src/typegen/test_support.rs` (`#[cfg(test)]`),
  used by both test modules; the three inline splits (`Counter_update`, `Saved_update`, `Form`)
  now call `csharp_companion_body`.
- **Verification:** Confirmed. `grep -rn "fn write_library" crates/nx-cli/src/` finds exactly one
  definition, in `crates/nx-cli/src/typegen/test_support.rs`, declared `#[cfg(test)] mod test_support;`
  at `typegen.rs:5-6` so it costs nothing in a release build. The three inline splits are gone —
  `typegen.rs:2474`, `:2485`, and `:2552` now call `csharp_companion_body`, and the reformatting
  churn the finding objected to went with them.

### ✅ Resolved - RF6 A transitive origin generates an import from a library the generated package does not depend on
- **Severity:** Low
- **Evidence:** In the alias scenario (`typegen.rs:2721-2800`), library `people` depends only on
  `named`, yet the generated `User.ts` carries `import type { Tag } from "tags";` and the C# output
  references `global::Test.Tags.Tag`. For npm and NuGet a direct import needs a direct dependency;
  a consumer of the generated `people` package has no declared edge to `tags`. The design's Risks
  section covers the *namespace/package name* derivation for a transitive origin and points at the
  existing "assumed package" warning, but that warning says the package *name* may be wrong, not
  that the dependency is undeclared. The test asserts only that no warning mentions `User_update`,
  so it does not notice.
- **Recommendation:** No code change needed in this change, but record it as a follow-up (the
  generated package manifest / project references need the transitive origin) or extend the
  assumed-package warning to say so when the origin library is not a direct dependency.
- **Fix:** Recorded as item 6 of `docs/add-update-actions-followsup.md`, with both options named.
  No code change.
- **Verification:** Confirmed — `docs/add-update-actions-followsup.md:200-210` states the problem
  with the concrete `people`/`named`/`tags` shape, notes that the existing assumed-package warning
  addresses the package *name* and not the missing edge, and names both remedies (manifest/project
  references, or widening the warning). Deferring was the right call; this is a packaging concern
  outside the change's stated scope. Resolved as deferred, not fixed.

### ✅ Verified - RF7 The retained warning no longer tells the author what to do
- **Severity:** Low
- **Evidence:** `model.rs:1090-1095` produces e.g. `Update companion 'User_update' inherits field
  'tag' typed 'Tag' from a base declared in another module, and 'Tag' resolves to 'Tag' in library
  'named', which does not export it; the generated code will not resolve it`. The type name appears
  three times, and unlike the deleted warning (`... until 'Tag' is imported here`) it names no
  remedy. For the dominant case the remedy is concrete: export `Tag` from `named`.
- **Recommendation:** Trim the redundant repetition and append the fix, e.g. `...; export 'Tag'
  from library 'named' so the generated code can reference it`. The spec only requires the
  companion, field, and type to be named, so this is free.
- **Fix:** Every reason now carries its own consequence or remedy and drops the repeated name. The
  not-exported case reads `... typed 'Tag' from a base declared in another module, and library
  'named' does not export 'Tag'; export it from 'named' so the generated code can reference it`
  (or, through an alias, `it resolves to 'X', which library 'named' does not export; export 'X'
  from 'named' ...`). Tests assert the new wording.
- **Verification:** Confirmed. The shared prefix at `model.rs:1050-1053` dropped its trailing
  boilerplate, and each reason at `model.rs:1078-1130` now ends in its own consequence or remedy;
  the not-exported case special-cases `item.name == name` so the type name appears twice, not three
  times. `typegen.rs:3032` and `:3050` assert `does not export 'Tag'` and `export it from 'named'`
  for both languages, and the spec's "naming the companion, the field, and the type" still holds.
  The aliased variant of the not-exported reason (`it resolves to 'X', which library '…' does not
  export`) has no test, but it differs only in which name it prints.

## New Findings Discovered During 2026-09-11 18:48 Verification

### ✅ Verified - RF8 The RF1 fix adds a third warning case that the spec delta does not describe
- **Severity:** Low
- **Evidence:** `specs/cli-code-generation/spec.md:15-19` enumerates exactly two warning cases:
  "When the resolution reaches a type the dependency does not export, or one typegen cannot
  reference across libraries". The peer-shadow warning added for RF1 is neither — the type *is*
  exported, it *is* referenceable, and it lives in the generating library rather than across a
  library boundary. There is also no scenario for it, while the other two warning-adjacent
  behaviors each have one (`spec.md:44`, `:51`, `:58`). Design D2 step 3 records the rule but
  design is not the spec, and `openspec validate --strict` passes either way, so nothing else will
  catch the drift before archive.
- **Recommendation:** Widen the requirement's warning sentence to cover a resolution the generating
  module cannot name unambiguously (import shadowing a peer included), and add a scenario mirroring
  `library_update_companion_warns_when_an_import_shadows_the_peer_an_inherited_field_names`: library
  `ui` declares `Tag` and a base typed by it, its `user.nx` imports a different `Tag` from `other`,
  and generation warns naming `User_update`, `tag`, and `Tag` while leaving the written name.
- **Fix:** The requirement's warning sentence now names the third case ("one the generating module
  cannot name unambiguously because an import of its own claims the same visible name"), and a
  new scenario "Inherited companion field whose peer type an import shadows still warns" mirrors
  the RF1 regression test. `openspec validate --strict` passes.
- **Verification:** Confirmed. `specs/cli-code-generation/spec.md:15-20` now reads "a type the
  dependency does not export, one typegen cannot reference across libraries, or one the generating
  module cannot name unambiguously because an import of its own claims the same visible name",
  which covers the peer-shadow case without loosening the other two. The new scenario at
  `spec.md:66-70` matches the RF1 regression test's shape (`ui` with `tag.nx`/`named.nx`, a
  `user.nx` importing a different `Tag` from `other`) and the requirement's "naming the companion,
  the field, and the type". `openspec validate --strict` passes and `cargo test -p nx-cli` is still
  182 green. One clause of the new scenario is not asserted anywhere — see RF9.

## New Findings Discovered During 2026-09-11 19:05 Verification

### ✅ Verified - RF9 The peer-shadow scenario's "carries the written name" clause is asserted nowhere
- **Severity:** Low
- **Evidence:** The scenario added for RF8 (`specs/cli-code-generation/spec.md:66-70`) has two
  **THEN**/**AND** clauses: the warning, and "the generated companion SHALL carry the written name
  `Tag` for that field". `library_update_companion_warns_when_an_import_shadows_the_peer_an_inherited_field_names`
  (`crates/nx-cli/src/typegen.rs:2887-2916`) asserts only the warning and never looks at the
  generated output. Its sibling `update_companion_warns_when_an_inherited_field_type_is_not_exported`
  does assert the matching clause for the not-exported scenario (`typegen.rs:3042` and `:3059`), so
  this is an inconsistency between two scenarios of the same shape, not a deliberate omission. The
  behavior is correct today — I confirmed by CLI that the shadow case emits `tag?: Tag;` — but
  nothing would catch a future change that, say, dropped the field or emitted a qualified name the
  emitters cannot resolve.
- **Recommendation:** The test already loops over both languages and has `output` in hand; add one
  assertion per language inside that loop — `tag?: Tag;` in the TypeScript file and
  `NxOptional<Tag>` on the C# `User_update` body via `csharp_companion_body` — mirroring
  `typegen.rs:3042`/`:3059`.
- **Fix:** The loop now asserts per language: `tag?: Tag;` in `user.ts`, and
  `public NxOptional<Tag> Tag { get; set; }` on the C# `User_update` body via
  `csharp_companion_body`. `cargo test -p nx-cli typegen` passes (112).
- **Verification:** Confirmed at `typegen.rs:2926-2939`. Both assertions are specific rather than
  incidental: `tag?: Tag;` cannot be satisfied by the `other?: Tag;` field the same fixture
  declares, and the C# assertion is scoped to the `User_update` body by `csharp_companion_body`,
  so it reads the `tag` field's property and not `Other`. That matches the scenario's second clause
  in both languages and mirrors how the sibling not-exported test asserts the same clause.
  `cargo test -p nx-cli typegen` is 112 green and `cargo fmt --all --check` is clean.

### ✅ Verified - RF10 `proposal.md` still describes two warning cases and three new scenarios
- **Severity:** Low
- **Evidence:** `proposal.md` "What Changes" says "The warning stays only for a type the declaring
  library does not export, or that typegen cannot generate a cross-library reference for", and
  "Modified Capabilities" says the requirement "gains scenarios for a field typed by a dependency
  export the generating module does not import, for a type reached through the declaring module's
  alias, and for the warning that remains when the dependency does not export the type". Both
  predate the RF1 fix and the RF8 spec update: there are now three warning cases and four new
  scenarios. This does not reach the main specs on archive — only the spec delta is promoted — but
  it leaves the change's own artifacts disagreeing with each other, which is what
  `openspec-verify-change` looks for before archiving.
- **Recommendation:** Add the peer-shadow case to the warning sentence and the fourth scenario to
  the Modified Capabilities list. Two small edits; no design or code implication.
- **Fix:** `proposal.md` "What Changes" now lists the peer-shadow case as the third warning, and
  "Modified Capabilities" names the fourth scenario.
- **Verification:** Confirmed. `proposal.md:21-24` now reads "a type the declaring library does not
  export, one typegen cannot generate a cross-library reference for, or a peer type the generating
  module cannot name unambiguously because an import of its own claims the same visible name",
  matching `spec.md:15-20` and the three `Err` families in `resolve_inherited_type_name`.
  `proposal.md:46-49` names all four new scenarios, which is the count in the spec delta
  (`spec.md:46`, `:53`, `:60`, `:66`). Proposal, design D2/D5, spec, and implementation now agree.

## Questions
- Design D2 step 1 resolves only under `PreparedNamespace::Type`, but the implementation
  (`model.rs:1081-1084`) also falls back to `PreparedNamespace::Element`. It looks like a
  deliberate improvement for component-typed fields; should design.md record it, and is there a
  scenario where the Element namespace could resolve a name the Type namespace deliberately does not?
  - **Answer:** Deliberate: `ModuleNamespace` reaches components only through the element
    namespace, and a component can type a property. Design D2 step 1 now records the fallback. The
    type namespace is consulted first, so a name both namespaces resolve (a record) takes the type
    answer; only a name the type namespace has no entry for falls through, and for a record-typed
    field that cannot happen.
- Task 5.1 says "the three bullets of item 5", but item 5 at `HEAD` had two bullets, and only the
  first (the checker test) matches a task in section 4 — tasks 4.2 and 4.3 close gaps recorded
  elsewhere. The work is done; the artifact wording is just off. Worth fixing before archive?
  - **Answer:** The working tree's copy of the document (uncommitted at the time the change was
    written) had three bullets under item 5, one per task 4.x; the task text matches that copy,
    which is the one this change edited. Left as is.
- `register_generating_library` (`model.rs:993-999`) propagates `build_cached_imported_library`'s
  error with `?`, so a generating library whose root directory name is not valid UTF-8 now fails
  library typegen outright instead of generating. Intentional, or should it degrade to "no peer
  resolution" with a warning?
  - **Answer:** Intentional. `from_library_with_warnings` already fails outright when a module is
    not under the library root, and the emitters derive the package and namespace from the same
    directory name, so a root that cannot be named has no usable output either way. Left as is.

## Summary
- The core design lands cleanly and the implementation is faithful to it: `declaring_module` on
  `ExportedRecordField`, a single model-level pass that synthesizes `ImportedType`s, retirement of
  `warn_about_unresolvable_inherited_fields`, and no emitter changes. I verified the headline
  scenarios end to end through the CLI, including the peer case and the visible-name-collision case
  that only had model-level tests; both produce correct TypeScript and C#. All specified test suites
  are green and `openspec validate --strict` passes.
- One real gap: the peer branch omits the taken-visible-name guard its dependency-branch sibling
  has, which lets an explicit import in the generating module silently retype an inherited field in
  TypeScript (RF1). Everything else is coverage and polish: a lost source-mode/wildcard test (RF2),
  two assertions that can never fail (RF3), model-only coverage of two resolution paths (RF4),
  duplicated test helpers (RF5), an undeclared transitive package dependency (RF6), and a warning
  that dropped its remedy (RF7).
- 7 findings opened, none blocking except RF1, which is narrow but silent when it bites.

### Verification pass — 2026-09-11 18:48
- All 7 findings addressed: RF1–RF5 and RF7 verified as fixed, RF6 resolved as a recorded
  follow-up. RF1's fix reproduces correctly against the original CLI repro; the rest were checked
  against the code and the new tests.
- Re-ran `cargo test -p nx-cli` (182 passed, up from 178), `cargo test --workspace` (green),
  `dotnet test` in `bindings/dotnet` (110 passed), `cargo fmt --all --check` (clean),
  `cargo clippy -p nx-cli --all-targets` (warning count unchanged against a stashed tree, so none
  introduced), and `openspec validate --strict` (valid).
- One new finding: RF8, the spec delta still describes only two warning cases and does not cover
  the peer-shadow warning RF1's fix introduced. Worth closing before archive, since archiving
  promotes this delta into the main spec.

### Verification pass — 2026-09-11 19:05
- RF8 verified: the requirement's warning sentence now names all three cases and a matching
  scenario was added, `openspec validate --strict` passes, `cargo test -p nx-cli` still 182 green,
  `cargo fmt --all --check` clean. All 10 findings so far are closed except the two below.
- Two new findings, both Low and both artifact-coherence rather than behavior: RF9 (the new
  scenario's second clause is asserted by no test, unlike its sibling scenario) and RF10
  (`proposal.md` still says two warning cases and three new scenarios). Worth closing before
  archive so the change's artifacts agree with each other, but neither affects generated output.

### Verification pass — 2026-09-11 19:19
- RF9 and RF10 verified; no findings reopened and none new. All 10 findings are now closed: 9
  verified as fixed, RF6 resolved as a recorded follow-up.
- Re-ran `cargo test --workspace` (green), `cargo test -p nx-cli typegen` (112 passed),
  `cargo fmt --all --check` (clean), `openspec validate --strict` (valid).
- Proposal, design, spec delta, implementation, and tests now agree on all three warning cases and
  all four new scenarios. Nothing outstanding blocks archiving.
