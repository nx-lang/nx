# Review: report-unresolved-type-names

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/symbol-resolution-model/spec.md,
specs/primitive-type-names/spec.md, tasks.md, release-notes.md  
**Reviewed code:** crates/nx-types/src/infer.rs, crates/nx-types/src/check.rs,
crates/nx-types/tests/unresolved_type_names.rs, crates/nx-types/tests/update_records.rs,
crates/nx-types/tests/void_is_not_a_primitive.rs, crates/nx-api/src/library_source_tests.rs,
sites/website/src/content/docs/reference/syntax/types.md,
sites/website/src/content/docs/reference/syntax/functions.md  

**Checks run:** `cargo test --workspace --no-fail-fast` (all pass), `pnpm -C sites/website test`
(174 blocks, 0 failures), `pnpm -C sites/playground test` (17/17 examples), `openspec validate
--strict` (valid), rustfmt on changed files (clean), clippy on nx-types (clean). Behavior was probed
with `nxlang run` / `nxlang codegen` built from the working tree and, for comparison, from `HEAD` in
a scratch worktree.

Each finding carries a **Confidence** line: *High* means the behavior was reproduced and the
expected outcome is unambiguous from the spec/design; *Medium* means the behavior is reproduced but
the right fix is a judgment call.

## Findings

### ✅ Verified - RF1 Lowering's error-recovery placeholder names now surface as spurious `unresolved-type` errors after a syntax or validation error
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `lower_type` ([lower.rs:1905-2000](crates/nx-hir/src/lower.rs#L1905-L2000)) substitutes
  `TypeRef::name("error")`, `TypeRef::name("unknown")` or an empty name for a type it could not
  lower. Before this change those were silently origin-less `Type::Named`; the new walk check
  ([infer.rs:4503](crates/nx-types/src/infer.rs#L4503)) reports them. Reproduced:
  - `type A = { n: }` → `Expected identifier here` **and** `` `` is not a visible type; did you mean `A`? ``
  - `let f(x: ) = { 1 }`, `type A = ( )`, `type A = { n:<function x: />: int }` → same empty-name error
  - fixture `crates/nx-syntax/tests/fixtures/invalid/union-empty-case-list.nx` (`type LoadState =`)
    → `` `` is not a visible type ``
  - fixture `invalid/function-type-type-parameter.nx` → the intended "Type parameter 'T' is not
    supported in a function type" plus `` `unknown` is not a visible type `` plus
    `` `T` is not a visible type `` (three errors for one mistake)
  - fixture `invalid/component-invalid-emits.nx` → `` `value` is not a visible type ``

  In the editor this fires on every half-typed annotation, with an empty name and a nonsense
  suggestion. The design goal is "Report each reference once" and the diagnostic "names the
  unresolved type"; a recovery placeholder is not a written reference.
- **Recommendation:** Make recovery produce something the checker maps to `Type::Error` without a
  report — e.g. a dedicated `TypeRef` error form, or treat the placeholder names (`error`, `unknown`,
  empty) as `Type::Error` in `type_from_type_name` — and make a function-type parameter already
  rejected by validation (`T:type`) resolve quietly. Add tests: a syntax error in a field type, a
  parameter type and an alias target each produce exactly the syntax diagnostic.
- **Fix:** Lowering now stands in for a type it could not lower (error node, missing type or result,
  a construct validation rejected, a missing applied-type tag or argument) with `TypeRef::recovery()`
  — an empty name no source can spell ([types.rs](crates/nx-hir/src/ast/types.rs),
  [lower.rs `lower_type`](crates/nx-hir/src/lower.rs)); an empty name from a missing identifier is the
  same thing. The checker resolves it to `Type::Error` without a report in `type_from_type_name` and
  `applied_type`. `type A = { n: }`, `let f(x: ) = …`, `type A = ( )`, `<function x: />`,
  `type LoadState =` and `<function TItem:type />` now each give exactly the syntax/validation error
  (test `a_type_that_did_not_parse_is_reported_only_as_a_syntax_error`). A site whose type is already
  an error also takes a bare name silently now instead of "expects <error>, and a bare name resolves
  only against a union's cases" (this surfaced in the RF4 cascade). Not changed:
  `invalid/component-invalid-emits.nx` still reports `` `value` is not a visible type `` — the parser
  recovers `value` as a real `emit_reference` node beside the error nodes, so it is a written name the
  component's contract then emits; suppressing it would need an error flag on `ComponentEmit`.
- **Verification:** Verified. Every listed reproduction (`type A = { n: }`, `let f(x: ) = …`, `type A = ( )`, `<function x: />`, `type LoadState =`, `<Box T= />`) now gives only the syntax/validation error, and of the `invalid/` fixtures only `component-invalid-emits.nx` still reports an `unresolved-type` (`value`); I accept the reason given for it. The `TypeRef::recovery()` change in nx-hir lowering is small and self-contained. Typegen special-cases the old `"unknown"`/`"error"` names, but it stops on syntax errors before reaching them (checked with `nxlang typegen` on a broken file: same output as HEAD). Correction to my original evidence: the extra `` `T` is not a visible type `` came from my probe `item:T`, not from the fixture. That case is still reported and is tracked separately as RF9.

### ✅ Verified - RF2 Skipping inline emit payloads rejected for a type parameter hides unrelated unresolved names
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `validate_local_record_defaults` ([infer.rs:6398-6429](crates/nx-types/src/infer.rs#L6398-L6429))
  collects *every* inline emit of a component whose contract failed with
  `TypeParameterInEmitPayload`, and skips each payload record wholesale. Reproduced:
  - `component <List T:type emits { Picked { item:T at:Instant } } />` → only the type-parameter
    error; `Instant` is never reported.
  - `component <List T:type emits { Picked { item:T } Other { at:Instant } } />` → `Other` is a
    separate payload that does not mention `T`, yet `Instant` is not reported.

  The spec requires a report for "every type reference written in a module" that names no visible
  type; design D4 only exempts the payload *field* already rejected. The user sees the second error
  only after fixing the first.
- **Recommendation:** Narrow the skip to the fields whose written type names one of the component's
  type parameters (e.g. resolve the payload with those parameter names bound to `Type::Error` in the
  scope, or skip per field rather than per record), and do not skip other inline payloads of the
  same component. Add tests for both reproductions above.
- **Fix:** The per-record skip is gone. For a component whose contract failed with
  `TypeParameterInEmitPayload`, its inline payloads are checked with the component's declared type
  parameters (plus the rejected, possibly inherited, one) bound to `Type::Error` in the scope, so only
  fields typed by a parameter are quiet. Both reproductions now report `Instant` alongside the
  type-parameter error (test `an_inline_payload_rejected_for_a_type_parameter_still_reports_other_names`).
- **Verification:** Verified. Both reproductions now report `Instant` next to the type-parameter error. A payload of another component (`<Other emits { Picked { at:Instant } }>`) and an extra unknown name in the rejected payload itself (`other:U`) are reported too. An inherited rejection (`abstract component <Base T:type emits { Picked { item:T } } />` extended by `List`) gives just the one resolution error. Test `an_inline_payload_rejected_for_a_type_parameter_still_reports_other_names` passes.

### ✅ Verified - RF3 The diagnostic labels the enclosing declaration, not the written reference
- **Severity:** Medium
- **Confidence:** High (behavior); Medium (fix choice)
- **Evidence:** `report_unresolved_type` ([infer.rs:4592](crates/nx-types/src/infer.rs#L4592)) uses
  `self.type_ref_span`, which is the owner's span. Reproduced:
  - `let f(first:string, x: MissingParam): MissingRet = { x }` → `MissingRet` underlines the whole
    function including its body (for a multi-line function, every line of it).
  - `let people: Contatc+ = {}` and `type Ids = Id+` → the whole declaration.
  - `type H = { onPick?:<function item:Item />: string }` → the whole `onPick` property.
  - Fields, props and state underline `name:Type` (closest to correct).

  Spec: "The diagnostic SHALL label the reference"; design D3: "primary label on the reference".
  The only test of the label, `an_unresolved_field_type_is_rejected`
  ([unresolved_type_names.rs:50-67](crates/nx-types/tests/unresolved_type_names.rs#L50-L67)), asserts
  only that the label is non-empty, so it would pass with any span.
- **Recommendation:** Either record type-reference spans in HIR (at least for return types, `let`
  annotations, alias targets and function-type parameters) and label the name itself, or amend D3
  and the spec to say the label is the declaration or property that wrote the reference. Strengthen
  the test to assert the label's text range (e.g. that it covers `MissingType` and, for a return
  type, does not extend into the body).
- **Fix:** Lowering records each type name with its span (`LoweredModule::type_name_span_within`) and
  each function return type / value annotation span (`LoweredModule::annotation_span`). Return types
  and `let` annotations are now resolved at the annotation's span (which also tightens other
  type-reference diagnostics there), and `unresolved-type` and `unresolved-type-argument` narrow their
  label to the written name inside the enclosing span, falling back to it for hand-built HIR. All of the
  listed cases now underline just the name (`MissingRet`, `Contatc`, `Id`, `Item`, …). Tests assert the
  label text per position (`the_label_is_the_written_name_in_every_position`,
  `a_name_written_as_a_parameter_and_a_return_type_is_reported_at_each`,
  `a_type_argument_is_labelled_at_the_argument`); design D3 describes the mechanism, and the spec gains
  a return-type label scenario.
- **Verification:** Verified. Each position now underlines only the written name: field, union-case field, parameter, return type (`MissingRet` only, not the body), `let` annotation, alias target, function-type parameter, prop, state, shared emit, and type argument (including a nested `<Box T=<Box T=Deep/>/>`). The label tests assert the exact text. One gap remains: a name written twice inside one enclosing span is labelled at its first occurrence for both reports. That is new finding RF10.

### ✅ Verified - RF4 A failed import now cascades into one `unresolved-type` per reference to every name it would have bound
- **Severity:** Medium
- **Confidence:** High (behavior); Medium (desired UX)
- **Evidence:** Reproduced with `nxlang codegen` on a workspace:
  - `import { Contact } from "../nope/types.nx"` + three uses of `Contact` → the missing-module
    error **and** three `` `Contact` is not a visible type ``. The import names `Contact` explicitly,
    so "not a visible type" is misleading; HEAD reported only the missing module.
  - `import "../shared" as S` (unresolvable) → every `S.Contact`, `S.Names`, `S.Card`, `S.Tapped`,
    `S.Contact.Property` reported.
  - `docs/drawnui-proposal` (entry `ui/ui.nx`, two unresolvable imports) → 42 new
    `unresolved-type` errors on top of the import errors.

  For hosts that compile tenant NX against libraries (the motivating ReachMe case), a
  misconfigured or missing library turns one actionable error into dozens.
- **Recommendation:** Do not report a name the module imported from a failed import: skip names
  bound by a selective import whose source is unresolved, and names qualified by the alias of an
  unresolved namespace import. For an unresolved whole-module import, consider suppressing
  `unresolved-type` in that module (or at least not suggesting a fix). Add a test for the
  selective-import case.
- **Fix:** `report_unresolved_type` (and the type-argument and applied-type-tag paths) skip a name an
  import that failed would have bound: a selective import's visible names and their `.Property`/`.Update`
  when nothing is bound under them (covers a missing module and "does not export"), `Alias.*` names of
  a namespace import that bound nothing, and — since its names are unknowable — any name when an
  unaliased wildcard import carries a diagnostic at its span. Each reproduction now gives only the
  import error; `docs/drawnui-proposal` goes from 42 `unresolved-type` (plus several "expects <error>")
  to just its import/base errors. Tests `a_name_an_unresolved_import_would_have_bound_is_not_reported_again`
  and `a_resolved_import_does_not_hide_a_misspelled_name` in `nx-api`; design D5, spec scenario and
  release note added. Trade-off (documented in design Risks): a failed unaliased wildcard import hides
  every `unresolved-type` in its module until it is fixed.
- **Verification:** Verified. A missing selective import, a missing namespace import (`S.*`), a missing wildcard import and `docs/drawnui-proposal` now give only the import errors. A selective import that resolved with one name unexported still reports an unrelated local `Typo`, and a resolved namespace import still reports `S.Contat` with the suggestion `S.Contact`. Name collisions between wildcard imports and local declarations raise no import diagnostic, so they do not trigger the suppression. However, the wildcard test is "any diagnostic at the import's span", not "the import failed". An import that resolved but is flagged as a duplicate therefore also hides every `unresolved-type` in the module. That is new finding RF11 (Low).

### ✅ Verified - RF5 Design D2 says a qualified union case used as a type resolves through its union, but it is reported as unresolved
- **Severity:** Low
- **Confidence:** High
- **Evidence:** D2: "A qualified union case used as a type is resolved through its union."
  `let f(s: Shape.circle): int = { 1 }` (with `type Shape = | circle { r:int } | dot`) now reports
  `` `Shape.circle` is not a visible type `` with no suggestion. On HEAD it was never usable as a
  type either (`Argument 0 expects Shape.circle, found Shape.circle`), so reporting it is arguably
  an improvement — but it contradicts the design, and no test covers it either way.
- **Recommendation:** Decide which is intended. If a case is not a type, correct D2 and consider a
  targeted message ("`Shape.circle` is a case of `Shape`; a case is not a type, use `Shape`"). If it
  should be a type, implement it. Add a test pinning the choice.
- **Fix:** A case is not a type. D2 is corrected, and the message is now targeted: "`Shape.circle` is
  not a visible type; it is a case of the union `Shape`, which is the type to write". Test
  `a_union_case_is_not_a_type` and a spec scenario pin it.
- **Verification:** Verified. `Shape.circle` and `Shape.dot` get the targeted "case of the union `Shape`" message, and `Shape.square` (not a case) gets the plain message. D2 is corrected, and the spec scenario and test `a_union_case_is_not_a_type` are present.

### 🔴 Open - RF6 A foreign alias's target is still resolved in the consumer's namespace; the new depth counter only hides the bare-name symptom
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `resolve_named_type` walks a foreign alias's target with `declaring_module = None`
  ([infer.rs:6983-6985](crates/nx-types/src/infer.rs#L6983-L6985)); `foreign_alias_depth` suppresses
  only the bare-name report. With a peer `types.nx` that does `import { Contact, Page } from
  "./base.nx"` and `export type Ps = <Page T=Contact/>`, a consumer importing only `Ps` reports
  `'Page' is not a generic record, so it cannot be applied` — a library reference reported by the
  consumer, contrary to D4 ("a consumer walking it reports nothing"). A bare target such as
  `export type Cs = Contact+` is now silent but still resolves to an origin-less `Contact` in the
  consumer. Pre-existing (HEAD behaves the same), so not a regression.
- **Recommendation:** Resolve foreign alias targets in their declaring module, as
  `foreign_nominal_type` already does for `Item::TypeAlias`, which would make
  `foreign_alias_depth`/`TypeAliasInfo.foreign` unnecessary; or narrow D4's wording and track the
  applied-target case as a follow-up.
- **Status:** Left open (pre-existing, not a regression). Resolving a foreign alias target in its
  declaring module touches alias caching, cycle detection and the quiet-resolution rollback, which is
  more than this change should take on. D4 is narrowed to what holds: a consumer walking a foreign
  alias reports no `unresolved-type`, and the applied-target case is named as pre-existing behavior.
  Recommend a follow-up change that carries the declaring module on `TypeAliasInfo` and walks the
  target through it (which would retire `foreign_alias_depth`).
- **Verification:** Still open by agreement. D4 now states the pre-existing behavior accurately ("an applied target the consumer cannot resolve still yields that applied type's own error"), and I confirmed it is unchanged from HEAD. Leaving it open as a follow-up, not a blocker for this change.
- **Deferred:** Recorded in `specs/future.md` as "An imported alias's target is resolved in the
  importing module", with the fix and the tests it needs.

### ✅ Verified - RF7 The did-you-mean suggestion is noisy for short names now that it fires everywhere
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** Reproduced suggestions: `Txt` → `int`, `Paint` → `int`, `Point` → `int`,
  `Zed` → `Rec`, `T` → `A`/`H`, empty name → `A`. `closest_candidate` was tuned for
  `unresolved-type-argument`; this change applies it to every type position, so the weak matches
  are far more visible (the proposal's motivation is authors "least able to diagnose the result").
- **Recommendation:** Tighten the threshold relative to name length (e.g. require distance ≤ ⌊len/3⌋
  and the candidate to share a first letter or case-insensitive prefix), or skip primitive
  candidates for capitalized names. Low priority.
- **Fix:** Type-name suggestions use a stricter `closest_type_name`: letter case is not an edit (ties
  go to the better case match), edits allowed are a third of the shorter name (at least one), and must
  be fewer than the written name's length. Now `Txt`→`Text` (when declared) and `Strin`→`string`, but
  `Txt`/`Paint`/`Point`/`Zed`/`T` get no suggestion and an empty name is never reported (RF1). The
  general `closest_candidate` used for cases and props is unchanged. Test
  `a_suggestion_is_offered_only_for_a_likely_misspelling`.
- **Verification:** Verified. `Txt`, `Paint`, `Point`, `Zed` and `T` get no suggestion. `Txt` gets `Text` when `Text` is declared. `Strin` and `Stirng` get `string`, `Contatc`, `Cnotact` and `contact` get `Contact`, and `Int`/`INT` get `int`. Those are reasonable suggestions.

### ✅ Verified - RF8 The suggestion formatting is duplicated between `unresolved-type` and `unresolved-type-argument`
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `report_unresolved_type` ([infer.rs:4592-4603](crates/nx-types/src/infer.rs#L4592-L4603))
  and the type-argument branch in `applied_type` ([infer.rs:4792-4797](crates/nx-types/src/infer.rs#L4792-L4797))
  each build `visible_type_names()`, call `closest_candidate` and format `"; did you mean `{}`?"`.
- **Recommendation:** Extract a `did_you_mean_suffix(&self, name) -> String` helper and use it in
  both (RF7's tuning then lands in one place).
- **Fix:** Extracted `did_you_mean_type` (and `type_name_span` for the label); both
  `report_unresolved_type` and the `unresolved-type-argument` branch use them.
- **Verification:** Verified. `did_you_mean_type` and `type_name_span` are shared by `report_unresolved_type` and the `unresolved-type-argument` branch. No duplicated formatting remains.

## New Findings Discovered During 2026-09-28 Verification

### ✅ Verified - RF9 A function type's rejected `T:type` parameter is still reported as an unresolved type where it is used
- **Severity:** Low
- **Confidence:** High (behavior); Medium (fix choice)
- **Evidence:** `type H = { f?:<function T:type item:T />: string }` reports the validation error
  "Type parameter 'T' is not supported in a function type" and also
  `` `T` is not a visible type `` at `item:T`. That is two errors for one mistake: the same cascade
  RF2 removed for inline emit payloads. Design D2 also still lists "function-type parameters" among
  the visible type parameters, but validation rejects them and nothing binds them.
- **Recommendation:** When lowering drops a function type's `T:type` parameter, bind its name to
  `Type::Error` while walking that function type (as RF2 does for rejected payloads), or have
  lowering replace references to it with `TypeRef::recovery()`. Remove "function-type parameters"
  from D2's list. Add a test that this source gives exactly the validation error.
- **Fix:** Lowering tracks the `T:type` parameters of each function type it lowers (validation rejects
  them) and lowers a use of that name inside the same function type to `TypeRef::recovery()`
  ([lower.rs](crates/nx-hir/src/lower.rs) `rejected_type_parameters`, `lower_type_name`). So
  `type H = { f?:<function T:type item:T />: T }` gives exactly the validation error, while a `T`
  outside that function type is still reported. D2 no longer lists function-type parameters as
  visible (it says a function type declares none), and D5 names the case. Test
  `a_rejected_function_type_parameter_is_reported_only_by_validation`.
- **Verification:** Verified. `<function T:type item:T />: T` and the nested `<function T:type item:<function x:T />: T />` each give only the validation error. A `T` written outside the function type (`g:T` next to it) is still reported. A module-level `type T` or an enclosing record `T:type` does not change the result. D2 and D5 are updated, and `a_rejected_function_type_parameter_is_reported_only_by_validation` passes.

### ✅ Verified - RF10 A name written twice inside one enclosing span is labelled at its first occurrence for both reports
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `type_name_span_within` returns the first recorded span of the name inside the
  enclosing span. So `type Twice = <function a:Dup />: Dup` gives two identical diagnostics, both at
  `a:Dup` (13:26), and the return-type `Dup` is never underlined. The same applies to any
  construct whose enclosing span holds the name twice, e.g. a field `m:<Pair A=Dup B=Dup/>`.
  Separate fields and parameters are unaffected, as is a parameter plus a return type, since each
  has its own span (`let g(x: Dup2): Dup2` labels both correctly).
- **Recommendation:** Carry the occurrence into the lookup, e.g. keep a per-walk cursor of names
  already reported inside the current `type_ref_span` and return the next unused occurrence. At
  least avoid emitting two diagnostics with the same code, message and span. Add a test for the
  alias case above.
- **Fix:** The lookup now takes an occurrence: `LoweredModule::type_name_span_within(name, within,
  ordinal)` returns the n-th recorded span of the name inside `within` (sorted, deduplicated, since
  some declarations are lowered twice). The checker counts each name as the walk visits it, in written
  order (`next_type_name_ordinal`), starting afresh for each reference of its own (an annotation
  entered through `type_from_type_ref_in`, a local alias target, any alias walk). Applied-type tags and
  bare type arguments are counted where `applied_type` reads them, and parts it skips (an early
  return, an unknown or duplicate argument) are counted through `skip_type_name_ordinals`, so later
  occurrences keep their place. `type Twice = <function a:Dup />: Dup` now labels `a:Dup`'s `Dup` and
  the return-type `Dup`; `<Pair A=Dup B=Dup/>` labels each argument. Test
  `a_name_written_twice_in_one_reference_is_labelled_at_each_occurrence`; D3 describes it.
- **Verification:** Verified. I probed the ordinal bookkeeping in each case where it could slip, and every label landed on the right occurrence: a name repeated in a function type's parameter and result (in a record field, union-case field, action field, abstract base, `let` annotation and alias target); both arguments of `<Pair A=X B=X/>` in a prop, an inline-emit payload and component state (state is lowered twice, which the dedup handles); a nested applied argument ahead of a bare one (`<Pair A=<Box T=D3/> B=D3/>`); arguments after an unknown argument, bare or composite (`Z=D4`, `Z=<Box T=D5/>`), where the skip keeps later ones in place; an early return on a non-generic or missing tag, followed by the same name in a later field; a local alias reached in the middle of a reference (`a:Ali b:D7` gives `D7` at the right place in both the alias and the reference); `?`/`+`/`*` suffixes on repeated names; and a parameter and return type repeating the same applied type (four labels, each correct). Quiet resolutions go through `type_from_type_ref_in`, which saves and restores the counts, so they cannot shift a later label.

### ✅ Verified - RF11 A wildcard import that resolved but carries any diagnostic suppresses every `unresolved-type` in its module
- **Severity:** Low
- **Confidence:** High (behavior); Medium (severity)
- **Evidence:** `is_name_of_failed_import` treats an unaliased wildcard import as failed when
  `module.diagnostics()` holds any diagnostic whose span equals the import's span. The prepare step
  also puts non-failure diagnostics at that span. With `import "../shared/types.nx"` written twice,
  the only error is "imported more than once", and the local typo `t:Typo` is not reported, even
  though the import resolved and bound its names. This goes beyond the documented trade-off ("an
  unaliased wildcard import that fails").
- **Recommendation:** Treat the import as failed only when it bound nothing. For an unaliased
  wildcard, that means checking that no binding in the module has that import as its origin/span,
  or checking the failure diagnostics specifically (missing module, did not parse, ambiguous).
  Add a test: a duplicated wildcard import still lets a local typo be reported.
- **Fix:** Preparation now records each import that did not resolve:
  `PreparedModule::mark_import_unresolved`, called at the 12 failure sites in
  [artifacts.rs](crates/nx-api/src/artifacts.rs) (missing, ambiguous, target did not parse,
  unsupported git/HTTP, invalid path, not a directory), but not at the two "imported more than once"
  sites. The unaliased-wildcard test is `is_import_unresolved(import.span)` instead of "any diagnostic
  at the import". A duplicated `import "../shared/types.nx"` now reports the local `t:Typo` next to
  the duplicate-import error. Tests `a_repeated_import_that_resolved_does_not_hide_an_unresolved_type`
  and `a_library_whose_wildcard_import_does_not_resolve_reports_only_the_import`; D5 and the
  proposal's Impact updated.
- **Verification:** Verified for the reported case. With a duplicated wildcard import, the local `Typo` is now reported next to the duplicate-import error. A missing wildcard import, alone or duplicated, still hides the names it would have provided, and `docs/drawnui-proposal` has 0 `unresolved-type`. The failure sites in `apply_graph_imports`, `add_workspace_import_bindings` and `apply_build_context_imports` all call `mark_import_unresolved`, and the two duplicate-import sites correctly do not. Two edge cases where an import supplies no names but is not marked are recorded as RF12.

## New Findings Discovered During 2026-09-28 Round-2 Verification

### ✅ Verified - RF12 Two ways an import can supply none of its names still cascade into `unresolved-type`
- **Severity:** Low
- **Confidence:** High (behavior); Medium (whether (b) should be suppressed)
- **Evidence:**
  - (a) `apply_build_context_imports` ([artifacts.rs:2558-2573](crates/nx-api/src/artifacts.rs#L2558-L2573))
    returns early when the importing file's path cannot be canonicalized. It adds one diagnostic
    across the whole file ("Local library import resolution was skipped …") but calls no
    `mark_import_unresolved`. Every unaliased local wildcard import in that module is then treated
    as resolved, so each name it would have provided is reported. This path is only reached for a
    library source file on disk whose path disappears or cannot be resolved, so it is rare. The fix
    is one line: mark each local import's span before returning.
  - (b) An import whose target module has a syntax error resolves, because the module is kept in
    part. The broken declaration binds nothing, though, so every use in an importer is reported. In
    a workspace with `shared/bad.nx` = `export type Contact = {` (unclosed), `import
    "../shared/bad.nx"` plus `c:Contact` reports `` `Contact` is not a visible type `` in
    `app/main.nx` next to the `Unclosed brace` in `bad.nx`. The selective form reports "does not
    export 'Contact'" instead. HEAD reported only the syntax error. In an editor, every importer
    lights up while a shared module is being edited. Libraries are unaffected, since a library with
    a syntax error does not load.
- **Recommendation:** For (a), mark every local import unresolved on the early return. For (b),
  either count an import whose target module has syntax errors as unresolved for the names it would
  have provided (for example, mark it when the target's parse diagnostics are non-empty), or accept
  the cascade and say so in design D5 / Risks. Add a test for whichever is chosen.
- **Fix:** The marker is renamed `PreparedModule::mark_import_incomplete` / `is_import_incomplete`
  ("did not bind every name its author meant it to"). It still changes nothing about what an import
  binds.
  - (a) The early return in `apply_build_context_imports` now marks every import of the module
    before returning. Test: the existing
    `apply_build_context_imports_reports_unresolved_source_path_for_local_imports` now also asserts
    the import is marked.
  - (b) Suppressed. `add_workspace_import_bindings` binds what the target provides as before, then
    marks the import incomplete when the target's parse/validation diagnostics include an error. The
    checker honors the mark for unaliased wildcards (every name), namespace imports (`Alias.*`) and
    selective imports (their listed names), in addition to the "nothing bound" test. So
    `import "../shared/bad.nx"` (or `as B`) with an unclosed brace in `bad.nx` reports only the
    `Unclosed brace`. Libraries are unaffected, since one with a syntax error does not load. Test:
    `an_import_of_a_module_with_a_syntax_error_reports_only_that_error`.
  - Artifacts: design D5 (the new cases) and Risks (a typo in an importer stays hidden while the
    imported module has a syntax error), a spec scenario, and a release-note sentence.
- **Verification:** Verified. (a) The early return now marks every import before returning, and the existing test asserts it. (b) With `shared/t.nx` = `export type Contact = {`, an unaliased wildcard reports only `Unclosed brace`. A namespace import (`as B`) suppresses `B.Contact` and `B.Nope` but still reports a local `Typo`. A selective import reports "does not export 'Contact'" plus the local `Typo`. Errors the target produces only at type-check time (a bad default, an unresolved type in the target) do not mark the import, so the importer's `Typo` is still reported next to them. The rename is complete: nothing in crates, bindings, sites or the change artifacts still refers to `mark_import_unresolved`, `is_import_unresolved` or `unresolved_imports`. `cargo test --workspace` passes (2596, 0 fail) and `openspec validate --strict` is valid. The unaliased-wildcard case is broader than it needs to be; see RF13.

## New Findings Discovered During 2026-09-28 Round-3 Verification

### ✅ Verified - RF13 Any syntax or validation error in an unaliased-wildcard target hides every `unresolved-type` in the importer, even when no declaration was lost
- **Severity:** Low
- **Confidence:** High (behavior); Medium (whether it is worth narrowing)
- **Evidence:** `add_workspace_import_bindings` marks the import incomplete when the target's
  parse diagnostics contain any error. This includes post-parse validation errors, which do not drop
  declarations. For an unaliased wildcard, `is_name_of_failed_import` then claims every name. With
  `import "../shared/t.nx"` and a local `t:Typo` in the importer, `Typo` is not reported when
  `t.nx` is any of the following:
  - `export type Contact = { name:string tags:string[] }`: a validation error, and `Contact` still
    lowers, as `string*`.
  - `export type Contact = { name:string }` plus an unrelated `type H = { f?:<function T:type />: string }`:
    a validation error in a declaration the importer never uses.
  - `export type Contact = { name:string }` plus an unrelated `let f() = { 1 + }`: a syntax error
    outside every imported declaration.

  In each case every name the importer uses was bound, so nothing needed suppressing, yet the
  importer's own typos stay hidden until the target is fixed. Design D5 does say "syntax or
  validation error", so the behavior is as designed. But the spec requirement and scenario, and the
  Risks entry, say only "syntax error". That understates what is hidden and leaves the artifacts
  inconsistent.
- **Recommendation:** Make the mark specific to lost declarations. For example, mark the import
  only when a top-level item of the target failed to lower, or when the error's span lies inside an
  exported declaration the import would bind. Or, at the least, ignore post-parse validation errors,
  which keep their declarations. If the broad rule is kept deliberately, align the spec text and the
  Risks entry with D5 ("syntax or validation error, anywhere in the target"). Add a test pinning
  whichever rule is chosen.
- **Fix:** Narrowed to lost declarations. `add_workspace_import_bindings` now marks the import incomplete
  only when `lost_a_declaration` holds ([artifacts.rs](crates/nx-api/src/artifacts.rs)): an
  error-severity diagnostic in the target whose primary span lies outside every top-level item that
  lowered. Errors inside a declaration that lowered leave its name bound, so they hide nothing. That
  covers a validation error (`tags:string[]`), an unrelated `<function T:type />`, a syntax error in a
  function body (`let f() = { 1 + }`) and a broken field type (`{ name: }`): all four now report the
  importer's `Typo`. An unclosed `export type Contact = {` and a removed `enum` declaration still
  hide it, since both lost a declaration. Test
  `an_error_that_leaves_every_declaration_in_place_hides_nothing_in_an_importer` pins the four
  non-hiding cases; `an_import_of_a_module_with_a_syntax_error_reports_only_that_error` still pins
  the hiding one. Design D5 and Risks, the spec requirement (with a new scenario, "An error that
  leaves every declaration in place hides nothing"), the release note and the doc comments now all
  say "lost a declaration".
- **Verification:** Verified. None of the three RF13 reproductions (`tags:string[]`, an unrelated rejected `<function T:type />`, an unclosed `let` body) hides the importer's `Typo` any more, and neither does a broken field type (`{ name: }`). An unclosed `export type Contact = {` followed by a newline, and a removed `export enum Contact`, still suppress for an unaliased wildcard import. With `as B` or a selective import, the local `Typo` is still reported. Errors in the target that are only warnings are filtered out. The edge cases you asked about: an unclosed record as the last declaration, an unclosed record at end of file, and a nameless `export type = {…}` all behave correctly. Stray tokens at the start of the file or between declarations (`@@@`) suppress everything, which is the conservative direction. A partially lowered declaration (`export type Contact = {` with no trailing newline lowers `Contact`) correctly counts as not lost. One boundary case is still missed: a declaration swallowed by an unclosed one before it. That is RF14. `cargo test --workspace` passes (2597, 0 fail).

## New Findings Discovered During 2026-09-28 Round-4 Verification

### ✅ Verified - RF14 A declaration swallowed by an unclosed declaration before it is not detected as lost, so importers report its name
- **Severity:** Low
- **Confidence:** High (behavior); Medium (fix choice)
- **Evidence:** `lost_a_declaration` ([artifacts.rs:2278-2296](crates/nx-api/src/artifacts.rs#L2278-L2296))
  counts a declaration as lost only when an error lies outside every declaration that lowered. When
  an unclosed declaration runs on over the next one, the error lies inside the unclosed declaration,
  which did lower in part, so the swallowed declaration goes unnoticed. With
  `import "../shared/t.nx"` and `type Uses = { c:Contact b:B t:Typo }`:
  - `t.nx` = `export type Contact = { name:string` then `export type B = { y:int }` → the only
    error in the target is "Invalid record definition" at 2:1, inside the partly lowered `Contact`.
    The importer reports `` `B` is not a visible type ``.
  - `t.nx` = `export type Contact = { name:string }`, then `export let f() = { 1 +`, then
    `export type B = { y:int }` → the unclosed body swallows `B`, and the importer again reports
    `` `B` is not a visible type ``.

  This is the cascade RF12(b) removed, now limited to a declaration that follows an unclosed one.
  HEAD and round 3 reported only the target's syntax error.
- **Recommendation:** Treat a lowered declaration as possibly lost when an error lies inside it and
  its syntax node spans a later top-level declaration keyword (`type`, `let`, `component`, `action`,
  `export`, …) at the start of a line. Alternatively, compare the CST's top-level declaration starts
  with the lowered items, or simply count any error whose span reaches past the end of the first
  line of its enclosing declaration as a lost declaration. Add a test for the swallowed-record case.
  Also note in Risks that stray tokens between declarations suppress everything, which is the
  conservative direction.
- **Fix:** `lost_a_declaration` ([artifacts.rs](crates/nx-api/src/artifacts.rs)) still applies only to
  a target with an error. It now also counts a declaration as lost when a line begins a top-level
  declaration where no lowered item begins. `declaration_line_starts` scans first-column lines for
  an optional `export`/`private`/`abstract`/`external` followed by `type`, `action`, `let`,
  `component` or `enum`. It scans the text because a swallowed declaration is no longer a node of its
  own. The existing "error outside every lowered declaration" condition stays. Both reproductions
  (`… { name:string` followed by `export type B`, and an unclosed `export let f() = { 1 +` before
  `export type B`) now give only the target's errors. Every RF13 non-hiding case still reports the
  importer's `Typo`, including a target that uses every declaration form (abstract/external
  components, action, private let, union), since each form lowers from its line start. Tests:
  `a_declaration_swallowed_by_an_unclosed_one_before_it_is_lost_too`, plus that all-forms case added
  to `an_error_that_leaves_every_declaration_in_place_hides_nothing_in_an_importer`. Design D5
  describes the rule. Risks carries your note that stray tokens between declarations, or a
  declaration-looking first-column line inside a broken module, hide everything (erring safe). The
  spec has a new scenario ("A declaration swallowed by an unclosed one is lost too"), and the
  release note mentions the case.
- **Verification:** Verified. In both swallowed cases (`export type Contact = { name:string` followed by `export type B = …`, and an unclosed `export let f() = { 1 +` before `export type B`), the importer now reports only the target's errors. The following targets each add an unrelated validation error, and none of them is falsely flagged as having lost a declaration, so the importer's `Typo` is still reported: `///` and `//` comments before a declaration, a `/* block */` prefix, every declaration form (`export abstract type`, `extends`, `external component`, `export component`, element-style `export let <Z/>`, `export action`), CRLF line endings, a UTF-8 BOM, declarations split across lines (`type X =` with the body on the next line, a multi-line union), and a double space after `export`. This confirms that lowered item spans start at the modifier, as the offset comparison needs. Lookalike lines at the first column only err toward hiding, and D5 and Risks document that. `cargo test --workspace` passes (2598, 0 fail). No new issues.

## Questions
- Task 2.2 (ReachMe built-in libraries, templates and seeds validated through the wasm SDK) and the
  .NET part of task 2.1 were not independently re-run in this review; is there a recorded command or
  output for them?
- A bare `Update` in a component's state (`state { u?:Update }`) is now reported as
  `` `Update` is not a visible type `` (HEAD accepted it silently). The spec lists only
  `<Name>.Update` as visible, so this appears intended — confirm.

### Answers (from the implementer)
- Task 2.2 was verified by building the wasm SDK (`pnpm -C bindings/wasm build`) and running, in
  ReachMe (linked to this NX checkout via `pnpm nx:link`): `pnpm --filter api test` (793 pass,
  including compiling every built-in library, template and seed with zero diagnostics) and
  `pnpm --filter @reachme/flow-runtime test` (30 pass). The .NET part of 2.1 was
  `dotnet test bindings/dotnet/NxLang.sln` (169 pass). Those runs predate this fix round.
- Yes, intended: only `<Name>.Update` is a visible type name, so a bare `Update` in state is
  reported. Pinned by test `a_bare_update_is_not_a_type_name`.

## Summary
- The core check is well placed (in the type-ref walk, reusing the type-argument visibility test),
  the quiet paths correctly keep other modules' references unreported, `extends` and type arguments
  keep their single diagnostics, and all spec scenarios have passing tests. All Rust, website and
  playground suites pass.
- The main gaps are noise and precision: recovery placeholders and failed imports now cascade into
  `unresolved-type` (RF1, RF4), the inline-payload skip is broader than D4 intends and hides real
  errors (RF2), and the label covers the declaration rather than the reference the spec requires
  (RF3). RF5–RF8 are low-severity design-text, pre-existing, and cleanup items.
- Verification (2026-09-28): RF1, RF2, RF3, RF4, RF5, RF7 and RF8 are verified. RF6 stays open as an
  agreed pre-existing follow-up. Three new Low findings (RF9–RF11) are edge cases of the fixes. Checks
  re-run: `cargo test --workspace` (2591 pass, 0 fail), website code blocks (174, 0 failures),
  `openspec validate --strict` (valid), rustfmt and clippy on nx-hir and nx-types (clean). I did not
  re-run .NET or ReachMe.
- Round-2 verification (2026-09-28): RF9, RF10 and RF11 are verified, and RF6 stays open as agreed.
  One new Low finding, RF12, covers import edge cases. Checks re-run: `cargo test --workspace` (2595
  pass, 0 fail), website code blocks (174, 0 failures), `openspec validate --strict` (valid), rustfmt
  (clean), and clippy (no warnings in nx-hir, nx-types or nx-api). I did not re-run .NET or ReachMe.
- Round-3 verification (2026-09-28): RF12 is verified, and RF6 stays open as agreed. One new Low
  finding, RF13, covers over-suppression when an unaliased-wildcard target has an error unrelated
  to the imported names. Checks re-run: `cargo test --workspace` (2596 pass, 0 fail) and
  `openspec validate --strict` (valid). I did not re-run .NET or ReachMe.
- Round-4 verification (2026-09-28): RF13 is verified, and RF6 stays open as agreed. One new Low
  finding, RF14, is a boundary case: a declaration swallowed by an unclosed one before it still
  cascades. Checks re-run: `cargo test --workspace` (2597 pass, 0 fail). I did not re-run .NET or
  ReachMe.
- Round-5 verification (2026-09-28): RF14 is verified, and no new findings came up. Every finding is
  now verified except RF6, the agreed pre-existing follow-up. Checks re-run: `cargo test --workspace`
  (2598 pass, 0 fail). I did not re-run .NET or ReachMe.
