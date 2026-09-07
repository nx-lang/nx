# Review: improve-hover-content

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `measured-diagnostics.md`,
`specs/editor-language-service/spec.md`, `specs/source-analysis-pipeline/spec.md`

**Reviewed code:** working-tree diff plus untracked additions —
`crates/nx-types/src/infer.rs`, `crates/nx-types/tests/inference_inside_unresolved_elements.rs`,
`crates/nx-language-service/src/hover.rs` (new), `crates/nx-language-service/src/lib.rs`,
`crates/nx-language-service/src/positions.rs`, `crates/nx-lsp/src/lib.rs`,
`crates/nx-codegen/src/tests.rs`, `crates/nx-api/src/artifacts.rs`, `examples/nx/complex.nx`,
`src/vscode/CHANGELOG.md`, `src/vscode/TODO.md`

**Verification performed:** `cargo test -p nx-language-service -p nx-lsp -p nx-types` (all green:
96 + 4 + 7 + unit suites), `cargo clippy` on the changed crates (no new warnings in the new code),
`cargo fmt --all --check`, a re-run of the `examples/nx` corpus analysis to check
`measured-diagnostics.md`'s residual counts, and an out-of-tree probe binary driving the public
`WorkspaceSnapshot::hover` API and `nx_types::analyze_str` over positions the test suite does not
cover.

The corpus re-run matches the change's own record: `examples/nx/complex.nx` reports exactly the four
documented `Member access not yet implemented: .length` diagnostics plus its pre-existing syntax and
resolution errors, and nothing else new. The inference recursion, the new position contexts, and the
NX-fenced renderer all do what the specs ask for on the paths the tests cover. The findings below
are about positions and fallbacks the tests do not reach.

## Findings

### ✅ Verified - RF1 Hover on a qualified union case reports it as a property with a nonsense type

- **Severity:** Medium
- **Evidence:** `crates/nx-language-service/src/positions.rs:441` (`member_access_context`) claims
  the `admin` of `Role.admin`, and `crates/nx-language-service/src/lib.rs:485`
  (`member_access_hover`) renders every member-access as `hover::property(...)`. Probed against the
  public API:

  ```
  source: type Role = admin | guest
          let r: Role = {Role.ad⟨cursor⟩min}
  hover:  ```nx
          (property) admin: Role.admin
          ```
  ```

  A union case is not a property, and `Role.admin` is not a type — it is the same string as the
  member. The receiver `Role` is a type name with no recorded expression type, so the qualifier is
  dropped too. Before this change the position reported nothing, so this is a new wrong answer
  rather than a pre-existing one.

  The position is not incidental: `proposal.md`'s table of silent positions lists
  ``{Role.ad⟨cursor⟩min}`` — a qualified case — as one of the eight this change set out to answer.
  No scenario in `specs/editor-language-service/spec.md` covers it and no test asserts it, which is
  why the wrong rendering got through. It also makes three spellings for one concept: `(case)
  Role.admin` at the case's declaration, bare `Role.admin` at a property value or typed `let`, and
  `(property) admin: Role.admin` here.

- **Recommendation:** In `member_access_hover`, when the access's type is a union-case type (or the
  receiver resolves to a union declaration), render `hover::union_case(union, case)` so a qualified
  case reads `(case) Role.admin`, matching what its declaration site already reports. Add a scenario
  and a test for the position the proposal named.
- **Fix:** Added `hover::union_case_expression`, which reports `(case) Role.admin` when the
  expression *is* a case rather than merely having one as its type. The test is the expression node,
  not the type: `ContextualName`, `ResolvedUnionCase`, and `Member` are cases; an `Ident` bound to
  one is a use of it. Applied in both `member_access_hover` and `inferred_type_hover`, so all four
  positions that name the concept now agree — the case's declaration, a bare name at a typed site, a
  bare name in a value slot, and the qualified form. Pinned by
  `hover_over_a_qualified_union_case_reports_the_case`, which asserts the four are equal, and by
  `hover_over_a_value_typed_by_a_union_case_reports_the_type_not_the_case`, which pins the
  distinction. Added the scenario "Hover over a qualified union case reports the case" to
  `specs/editor-language-service/spec.md`, answering the first Question below.
- **Verification:** Confirmed. `member_access_hover` and `inferred_type_hover` both route through
  `hover::union_case_expression`, and probing the public API shows the four positions now agree
  exactly: `{Role.ad⟨cursor⟩min}`, `let r: Role = ad⟨cursor⟩min`, `<Card role=ad⟨cursor⟩min />` and
  the case's own declaration all report ```` ```nx\n(case) Role.admin\n``` ````. The guard does not
  over-claim: `u.na⟨cursor⟩me` on a record still reports `(property) U.name: string`, an optional
  field reports `string?`, a chained `a.b.c` reports `(property) B.c: int`, and a field *declared*
  with a union-case type reports `(property) U.role: Role.admin` rather than `(case)`. A value
  merely typed by the union reports `let chosen: Role`, pinned by the new test. The scenario
  "Hover over a qualified union case reports the case" is in the delta at
  `specs/editor-language-service/spec.md:123`. One position still answers nothing — the *unbraced*
  `let r: Role = Role.admin` — but that source is a syntax error (`analyze_str` reports
  `"Syntax error"` plus `'Role' is not a case of union 'Role'`), so declining is the conservative
  contract working, not a gap.

### ✅ Verified - RF2 `property_value_hover` duplicates `property_value_members`, is unreachable in practice, and would answer in a different spelling

- **Severity:** Medium
- **Evidence:** `crates/nx-language-service/src/lib.rs:1213` reimplements
  `property_value_members` (`lib.rs:1945`) statement for statement — same `scope.element(tag)`, same
  `base_type` extraction, same `scope.type_in_module`, same union check — differing only in what it
  returns. Design D4 says hover "resolves it through the same `property_value_members` path
  completions use"; it copies that path instead, so the two can drift.

  It is also dead on the scenario it was written for. Probing
  `<Card role=ad⟨cursor⟩min />` returns ```` ```nx\nRole.admin\n``` ````, which is
  `hover::inferred_type` — the `self.inferred_type_hover(...)` arm at `lib.rs:441` answers first.
  `property_value_hover` would have produced `(case) Role.admin`. So the fallback is untested, and
  on the rare path where it does fire it contradicts the spec sentence the test
  `hover_over_a_bare_property_value_reports_the_case_it_resolves_to` exists to pin: "the result
  SHALL be the one reported for the same bare name written at a declaration of type `Role`."

- **Recommendation:** Extract the shared resolution — something like
  `property_value_union(tag, property, scope) -> Option<&Declaration>` — and have both
  `property_value_members` and the hover fallback call it. Then make the fallback emit the same
  fragment the inferred path emits (or make both emit `(case) Role.admin`, which is the more
  informative and satisfies the "state the kind once" requirement for a position whose kind is not
  evident). Add a test that forces the fallback branch.
- **Fix:** Extracted `property_value_union(tag, property, scope) -> Option<&Declaration>`; both
  `property_value_members` and `property_value_hover` now call it, so the resolution exists once.
  The spelling divergence is gone as a consequence of RF1 — the primary path now emits
  `(case) Role.admin` too, so the fallback firing is not observable as a different answer. The
  fallback branch still has no test, because with `ProgramBuildContext::empty()` every visible
  declaration's module is analyzed and the inferred path always answers first; it is kept as the
  defensive path for a property whose union is visible when the type environment is not.
- **Verification:** Confirmed. `property_value_union` (`lib.rs:1954`) holds the resolution once,
  and both `property_value_members` (`lib.rs:1939`) and `property_value_hover` (`lib.rs:1213`) are
  now three-line callers of it, so the duplication is gone and D4's "ask it once" is met literally.
  The spelling divergence is closed by RF1: the primary path and the fallback both produce
  `(case) Role.admin`, so a fallback firing is genuinely unobservable. The acknowledgement that the
  branch still has no test is accurate and acceptable — it is now defensive code that cannot answer
  differently from the path in front of it, which is a much weaker reason to demand coverage than
  when it could.

### ✅ Verified - RF3 `declaration_fallback` is a second renderer that spells some declarations as something they are not

- **Severity:** Medium
- **Evidence:** `crates/nx-language-service/src/hover.rs:240` renders a declaration whose module has
  no lowered item, duplicating the record and union rendering that `record_signature`
  (`hover.rs:135`) and `union_signature` (`hover.rs:171`) already do, and diverging from them:
  - A single-case union renders `type Wrapper = only`. Per `AGENTS.md`, that spelling *is a type
    alias*, not a union — the bar is syntax there, not style. `union_signature` gets this right
    (probed: `type Wrapper =\n  | only`); the fallback does not.
  - A union with payload cases renders only the payloadless `Declaration::members`, silently
    dropping cases — the exact filtering problem design D2 cites as its reason for reading HIR.
  - A record loses `abstract`, `extends`, and the `action` keyword; an element-style function
    renders `component <X />` where the analyzed path renders `let <X />`.

  The ADDED requirement says the fragment shown "is the declaration as an author would write it".
  The fallback breaks that, and no test exercises it (every multi-document hover fixture analyzes
  all its documents, so `DocumentScope::item` always succeeds).

- **Recommendation:** Have the fallback share the case/field formatting with `union_signature` and
  `record_signature` — at minimum apply the same one-line-vs-bar-per-case rule so a single-case
  union never renders as an alias. Add a hover test over a declaration reached from a prebuilt
  library artifact so the path has coverage.
- **Fix:** Removed `declaration_fallback` rather than sharing formatters with it, and made
  `declaration_hover` return `Option<String>` so a declaration with no lowered item reports nothing.
  Sharing would have fixed the single-case-union bug but not the other two defects the finding
  names: `Declaration::members` is filtered to payloadless cases, so the fallback cannot list a
  union's payload cases at all, and `Declaration` carries no way to tell `let <X />` from
  `component <X />`. A path that structurally cannot spell "the declaration as an author would write
  it" should not answer — which is what the conservative contract already says: return no hover
  rather than fabricate incomplete semantic data. This also removed the now-unused
  `DocumentSymbolKind::display_name`. **Reviewer:** this is a deliberate departure from the
  recommendation; if you would rather keep a lower-fidelity fallback than go silent, say so and I
  will restore it with the shared formatters instead.
- **Verification:** Confirmed, and the departure from the recommendation is the better call — no
  need to restore a fallback. `declaration_fallback` and `DocumentSymbolKind::display_name` are gone
  from the tree (the only remaining `display_name` hits are an unrelated local in
  `nx-syntax/src/validation.rs`). Checking why this is safe rather than merely defensible: every
  `Declaration` in `scope.visible` is derived from `WorkspaceDeclarations::by_origin`, which is
  built from the analyzed artifacts themselves — the language service references no
  `LibraryRegistry` or interface at all — so `DocumentScope::item` can only miss for a module with
  no lowered item, i.e. one that failed to parse, which
  `hover_inside_a_declaration_with_a_syntax_error_returns_no_result` already requires to report
  nothing. The fallback was unreachable in practice, so deleting it removes a wrong answer and
  costs no real one. Declarations still spell correctly from HIR: `type Wrapper =\n  | only` keeps
  its bar, a payload union lists `failed { message:string }`, an element-style function reads
  `let <Panel title:string />`, and `abstract type Shape = { … }` keeps its modifier.

### ✅ Verified - RF4 Duplicate property bindings go unreported on an unresolved tag

- **Severity:** Medium
- **Evidence:** The new fallthrough (`crates/nx-types/src/infer.rs:1465`) calls
  `property_paths_for_entries` but not `report_duplicate_property_paths`, which every resolved path
  calls right after it (`infer.rs:1573`, `:1685`, `:1892`). Probed:

  ```
  <div class=1 class=2 />          → []
  <W a=1 a=2 />  (W declared)      → ["Property 'a' on 'W' can be supplied more than once …"]
  ```

  Duplicate detection reads only the property paths; it needs no binding contract. The
  `source-analysis-pipeline` delta states "The absent tag SHALL be the only thing left unchecked
  about such an element", so this is a gap against the requirement this change added, not a
  pre-existing one.

- **Recommendation:** Call `report_duplicate_property_paths` on the returned paths in the
  unresolved fallthrough, and add a scenario/test for a duplicated property on an intrinsic tag. If
  it is deliberately deferred, say so in the delta so the requirement does not over-promise.
- **Fix:** The unresolved fallthrough now binds the paths `property_paths_for_entries` returns and
  passes them to `report_duplicate_property_paths`, as every resolved path does. Added the scenario
  "A property supplied twice on an unresolved element is reported" to the
  `source-analysis-pipeline` delta and the test
  `a_property_supplied_twice_on_an_unresolved_tag_is_reported`. Re-ran the corpus measurement: this
  adds **zero** diagnostics across `examples/nx`, the DrawnUI catalog, and the 12 fiddle examples.
- **Verification:** Confirmed. `infer.rs:1463` binds the paths and passes them to
  `report_duplicate_property_paths`, with the comment giving the reason. Probed:
  `<div class=1 class=2 />` now reports `Property 'class' on 'div' can be supplied more than once on
  the same path` — the same message shape a resolved `<W a=1 a=2 />` produces — while
  `<div class=1 id=2 />` and `<div>{"ok"}</div>` stay silent, so no false positive was introduced.
  The scenario is in the delta at `specs/source-analysis-pipeline/spec.md:37` and the test
  `a_property_supplied_twice_on_an_unresolved_tag_is_reported` passes.

### ✅ Verified - RF5 Task 7.3 records a clean `cargo clippy --workspace --all-targets`; it does not pass

- **Severity:** Low
- **Evidence:** `cargo clippy --workspace --all-targets` fails today: `approximate value of
  f{32,64}::consts::PI` errors in `nx-hir` and `nx-value` lib tests, and `unresolved import
  criterion` for the `nx-syntax` `parse_benchmark` bench. All are in crates this change does not
  touch, and `cargo clippy -p nx-language-service -p nx-lsp -p nx-types --all-targets` is clean of
  any warning in the new code (the three `nx-language-service` warnings are at `lib.rs:2009`,
  `:2093`, `:2098`, outside every hunk). The change is fine; the task record is not.
- **Recommendation:** Correct the task note to say which command was actually verified, so the next
  reader is not misled about the repo's baseline.
- **Fix:** Rewrote task 7.3 to name both baselines and the command that was actually verified clean
  (`cargo clippy -p nx-language-service -p nx-lsp -p nx-types -p nx-api -p nx-codegen --all-targets`,
  plus `rustfmt --check` over every changed file), and to record that the two `nx-codegen` test
  failures reproduce with the change stashed. **Update:** those two `nx-codegen` tests now pass, so
  `cargo test --workspace --no-fail-fast` is green at 52 binaries; the task note records that too.
- **Verification:** Confirmed. Task 7.3 now names both baselines and the exact command verified
  clean, and each claim in it holds: `cargo clippy --workspace --all-targets` does fail on the PI
  approximations and the missing `criterion`, all in untouched crates, and the narrower per-package
  invocation reports nothing inside a hunk this change adds. The record now matches reality.

### ✅ Verified - RF6 The new code is not rustfmt-formatted, including a stray double blank line

- **Severity:** Low
- **Evidence:** `cargo fmt --all -- --check` reports diffs in `hover.rs:44`, `hover.rs:244`,
  `lib.rs:444`, `lib.rs:516`, `lib.rs:2934`, `lib.rs:3038`, `lib.rs:3203` and in
  `crates/nx-lsp/src/lib.rs` — every one inside a hunk this change added. `lib.rs:518-519` is an
  unambiguous accident: two consecutive blank lines between `member_access_hover` and
  `inferred_type_hover`. (The repo baseline is not fmt-clean either — `nx-codegen/src/builder.rs`,
  `nx-hir/src/scope.rs`, and three test files are untouched by this change and also differ — so a
  rustfmt version difference may be in play for the rest.)
- **Recommendation:** Run `cargo fmt` over the changed files; at minimum drop the double blank line.
- **Fix:** Ran `rustfmt --edition 2021` over the eight files this change touches, and confirmed
  `rustfmt --check` is clean on all of them. Formatted per file rather than per package on purpose:
  the five baseline offenders (`nx-codegen/src/builder.rs`, `nx-hir/src/scope.rs`,
  `nx-syntax/tests/parser_tests.rs`, and two `nx-types` test files) are all files this change does
  not touch, so `cargo fmt --all` would have mixed unrelated reformatting into the diff.
- **Verification:** Confirmed. `rustfmt --edition 2021 --check` is clean on all eight files this
  change touches, and the double blank line between `member_access_hover` and `inferred_type_hover`
  is gone. Formatting per file rather than `cargo fmt --all` was the right call: the five baseline
  offenders are all files this change does not touch, and folding them in would have buried the
  diff.

### ✅ Verified - RF7 Record hover drops field defaults, and `field_signature`'s doc comment claims otherwise

- **Severity:** Low
- **Evidence:** `crates/nx-language-service/src/hover.rs:161` documents "One record field as
  written: `name: string`, with its default where it has one", but `field_signature` at `:162`
  emits only `{name}: {type}`. `examples/nx/complex.nx:3-7` — the record this change introduced —
  declares `completed: boolean = false`, which hovers as `completed: boolean`. A default is part of
  how the author wrote the declaration, which is the standard the ADDED requirement sets.
- **Recommendation:** Either render the default (`RecordField` carries it) or correct the doc
  comment. The same question applies to parameter defaults in `function_signature` and
  `tag_signature`.
- **Fix:** Corrected the doc comment, and said why the default is omitted rather than only that it
  is: `RecordField::default` is an `ExprId`, and turning one back into the NX an author wrote needs
  an expression renderer this crate does not have. Rendering only the literal defaults would make
  the hover's fidelity depend on what the default happens to be, so the omission is uniform.
  `function_signature` and `tag_signature` claim nothing about defaults and are unchanged — and
  `Param` has no default to render in any case (see RF8).
- **Verification:** Confirmed. The doc comment at `hover.rs:189` now states the omission and its
  cause, and the cause checks out — `RecordField::default` is an `ExprId` with no NX renderer behind
  it, and the "uniform omission beats fidelity that varies with the default" argument is sound.
  `function_signature` and `tag_signature` claim nothing about defaults, and `nx_hir::Param`
  (`crates/nx-hir/src/lib.rs:199`) has only `name`, `ty`, `is_content` and `span`, so there is
  indeed no parameter default to render.

### ✅ Verified - RF8 A bare union case written as a default still hovers nothing

- **Severity:** Low
- **Evidence:** Probed: `let <Card role:Role = ad⟨cursor⟩min /> = <div />` returns `None`. Design
  D4's stated principle is "one rule for bare names: `<Card role=admin />` and `let r: Role = admin`
  are the same question asked in two places" — a parameter or field default is a third place that
  asks it, and `local_declaration_context` (`positions.rs:459`) reaches the position but
  `local_declaration_hover` finds no parameter named `admin` and returns `None`. Out of the letter
  of the specs, but inside the rule the design states.
- **Recommendation:** Treat a default-value position as a property-value position (the declared type
  is right there on the same `PROPERTY_DEFINITION`), or record it in the specs as deliberately out
  of scope.
- **Fix:** The cause was `local_declaration_context` over-claiming: it claimed any identifier under
  a `PROPERTY_DEFINITION`, including one written as the default. It now claims only the definition's
  own name node, the way `declaration_context` does, so a default falls through to the rules that
  answer for a value. A record field's default and a component property's default now both report
  `(case) Role.admin` — pinned by `hover_over_a_bare_default_value_reports_the_case_it_resolves_to`
  — and the spec sentence on property values now names a declared default as one of those positions.

  **Residual, left open deliberately:** an *element-style function* parameter's default
  (`let <Fn role:Role = admin /> = <div />`) still reports nothing, and no hover change can fix it.
  `nx_hir::Param` has no default field and `lower.rs:1646` drops the expression outright — "Default
  values are part of property_definition grammar but we don't store them in Param yet". There is no
  lowered expression to type or to hover. That is a lowering gap in `nx-hir`, worth its own change.
- **Verification:** Confirmed, including the residual. `local_declaration_context` now claims a
  `PROPERTY_DEFINITION` only when the cursor is on its own `child(0)` name node. Probed: a record
  field's default and a component property's default both report `(case) Role.admin`, matching every
  other position that names a case. The narrowing did not cost any declaration site — parameter,
  element-style parameter, component property, record field (with and without a default), and union
  payload field all still report as before, and a parameter name written ahead of a default
  (`let <Panel wid⟨cursor⟩th:int = 42 />`) still reports `(parameter) width: int`. The residual is
  real and correctly diagnosed: `nx_hir::Param` carries no default, so an element-style function
  parameter's default has no lowered expression to hover, and the position reports nothing. Worth
  its own change against `nx-hir`, as the note says.

### ✅ Verified - RF9 The unresolved-tag path builds and discards a cartesian product of property paths

- **Severity:** Low
- **Evidence:** `crates/nx-types/src/infer.rs:1465` calls `property_paths_for_entries` purely for
  its side effects and drops the `Vec<PropertyPath>`. That function
  (`infer.rs:1948`) multiplies alternatives across entries, so an element with *n* conditional
  property entries allocates on the order of 2^n paths that nothing reads. Reuse is the right
  instinct — the conditions do need `check_boolean_condition` — but the result is waste on the one
  path that never consumes it.
- **Recommendation:** If RF4 is fixed by passing the paths to `report_duplicate_property_paths`, the
  result stops being dead and this resolves itself. Otherwise, infer property values and check
  conditions without accumulating the path product.
- **Fix:** Resolved by the RF4 fix, as anticipated. The paths are now consumed by
  `report_duplicate_property_paths` on the same terms as every resolved path, so nothing is
  allocated that nothing reads.
- **Verification:** Confirmed. The paths `property_paths_for_entries` returns are consumed by
  `report_duplicate_property_paths` on the same terms as every resolved path, so nothing is
  allocated that nothing reads. Resolved as a consequence of RF4, exactly as the finding
  anticipated.

### ✅ Resolved - RF10 Task 7.4 is not done, so the change is not yet archivable

- **Severity:** Low
- **Evidence:** `tasks.md` 7.4 is unchecked, with an honest note that it needs a person at an
  editor. The substitute verification described there (content checked through
  `WorkspaceSnapshot::hover`, fence grammar registered as an injection into `text.html.markdown`)
  is real, but nothing has confirmed VS Code actually highlights a ```` ```nx ```` fence inside a
  hover popup — hover markdown rendering does not always honor injection grammars the same way a
  markdown document does, which is the specific risk the task exists to retire.
- **Recommendation:** Keep it open and do the manual pass before archiving. If VS Code turns out not
  to apply the injection in hover, the design's stated fallback ("unhighlighted monospace, still
  better than proportional prose") holds, but that should be an observed result rather than an
  assumption.
- **Status:** Left open, but the central risk is retired. A partial manual pass has now happened,
  and VS Code hover popups **do** apply the fence injection — the specific thing this finding said
  had not been confirmed. The pass also found a defect the substitute verification could not: a
  fragment carrying a parenthesized kind, such as `(property) ShapeCommon.shadows: Shadow[]?`, was
  barely highlighted. The grammar's scoping is gated behind a declaration keyword, so a line that
  starts with `(` reaches none of it: the names went unscoped, and worse, the annotation colon and
  the nullable `?` were scoped as a ternary's while the word `type` inside `(primitive type)` was
  scoped as the `type` declaration keyword. Fixed by a `hover-annotation` rule in
  `src/vscode/syntaxes/nx.tmLanguage.json` that is anchored on a literal kind at line start so it
  cannot claim NX source, covered by eight tests in
  `src/vscode/test/grammar/hover-annotation.test.ts` (247 grammar tests pass, up from 239). The
  remaining work is a look at the fixed rendering and at the other positions 7.4 lists.
- **Resolution:** Closed at archive time on the user's call. The pass happened and did its job: it
  retired this finding's central risk by confirming VS Code applies the injection inside a hover
  popup, and it found a defect no amount of API-level verification would have — the grammar could
  not highlight the very fragments this change introduced. That defect is fixed, pinned by tests on
  both the bare and the fenced path, and now required by an `editor-syntax-highlighting` delta. Task
  7.4 is checked with that evidence recorded in it.
- **Verification:** Still open, correctly. `tasks.md` 7.4 remains unchecked and nothing in this
  pass touches the risk it exists to retire: whether VS Code applies the markdown-fence injection
  grammar inside a hover popup. Needs a person at an editor before archive.

## Questions

- ~~Should the qualified-case position (RF1) get its own scenario in
  `specs/editor-language-service/spec.md`?~~ **Answered.** The scenario "Hover over a qualified union
  case reports the case" was added to the delta and the implementation now matches it.
- Four untracked files sit alongside this change and are not obviously part of it:
  `docs/scratch-highlighting.nx` (self-described as "not meant to be kept"),
  `examples/nx/template-candidate.nx` (excluded from scope in `measured-diagnostics.md`, but it now
  contributes 10 of the 28 diagnostics an `examples/nx` scan reports, which will confuse the next
  person to measure), `drawnup-startup-fixes-review.md`, and a `review.md` at the repo root. Are any
  of these meant to land with this change, and should `template-candidate.nx` move out of
  `examples/nx/` until it checks clean?
- `measured-diagnostics.md` records that `examples/nx/complex.nx` keeps four
  `Member access not yet implemented: .length` diagnostics because NX has no list length member.
  Is a follow-up change tracked for that, or is the example expected to stay red?

## Summary

The core of the change is sound and well evidenced. The inference recursion is a genuine bug fix
whose blast radius was measured before it was built on, and the IR-division finding recorded in
`measured-diagnostics.md` (markup content silently dividing as float) is a real defect caught and
pinned by a regression test. The new position contexts are cleanly separated, the resolver's
ordering keeps type annotations and declaration names classified as before, the NX renderer reads
the declaration from HIR exactly as design D2 argued, and the conservative contract holds — all
three crates' suites are green and nothing new is warned about by clippy.

The findings cluster in the places the tests do not reach. One is a new wrong answer at a position
the proposal itself named (RF1). Three are fallback paths that duplicate logic they were designed to
share and that would answer differently from the path that actually runs (RF2, RF3), or that leave a
requirement this change wrote only partly met (RF4). The rest are hygiene and record-keeping. None
blocks the approach; RF1 and RF4 are worth fixing before archive, and RF10 still needs a human at an
editor.

## Verification pass — 2026-09-06

All nine findings marked fixed (RF1–RF9) are **✅ Verified**; RF10 stays **🔴 Open** and needs a
person at an editor. No finding was reopened and no new finding was discovered.

Verified by re-reading every changed hunk, then re-running: `cargo test -p nx-language-service -p
nx-lsp -p nx-types -p nx-api` (all green — 19 binaries, 0 failures, 1 pre-existing ignore),
`rustfmt --edition 2021 --check` over each of the eight files this change touches (all clean), and
an out-of-tree probe against the public `WorkspaceSnapshot::hover` API and `nx_types::analyze_str`
covering every position each finding named plus regression probes around them — the ordinary
member-access renderings, the six declaration sites `local_declaration_context` still has to claim,
the declaration spellings that `declaration_fallback` used to shadow, and the conservative
contract's five silent positions.

Two of the fixes are better than the recommendations they answer. RF3 deleted the diverging
renderer instead of sharing formatters with it, and that is the right call for a reason the fix note
understates: every `Declaration` derives from an analyzed artifact, so the fallback could only fire
for a module with no lowered item — a case the conservative contract already requires to report
nothing. It was unreachable dead code emitting wrong NX. RF8's fix corrects the actual cause
(`local_declaration_context` claiming any identifier under a `PROPERTY_DEFINITION`) rather than
special-casing the symptom, and its residual — an element-style function parameter's default, which
`nx_hir::Param` drops during lowering — is diagnosed accurately and belongs in its own change.

The change is ready to archive once task 7.4's manual editor check is done.

## New Findings Discovered During 2026-09-06 17:42 Review

**Reviewed code:** the unstaged working-tree changes only — `src/vscode/syntaxes/nx.tmLanguage.json`
(the new `hover-annotation` rule), `src/vscode/CHANGELOG.md`, `src/vscode/TODO.md`, and the
untracked `src/vscode/test/grammar/hover-annotation.test.ts`. Verified by running
`pnpm run test:grammar` in `src/vscode` (247 passing) and by probing the grammar directly — through
`source.nx` alone and through the `text.html.markdown` → `source.nx.embedded.markdown` injection
that hover content actually travels.

The rule works. Every fragment `hover.rs` emits is scoped, and it is scoped identically inside a
markdown fence, so the defect the partial manual pass found is genuinely fixed. What follows is
what the probe found around it.

### ✅ Verified - RF11 New TextMate scopes land with no `editor-syntax-highlighting` spec delta
- **Severity:** Medium
- **Evidence:** The change adds a top-level grammar rule and two scopes users and themes can key
  off — `meta.annotation.hover.nx` and `meta.annotation.hover.kind.nx`
  (`src/vscode/syntaxes/nx.tmLanguage.json:2472-2545`) — plus it repurposes five existing scopes for
  a line shape that is not NX. `openspec/specs/editor-syntax-highlighting/spec.md` is the capability
  that exists to pin exactly this ("Defines the TextMate scopes the NX grammar assigns to NX
  source", line 5), and this change carries deltas only for `editor-language-service` and
  `source-analysis-pipeline`. The behavior is documented in `src/vscode/CHANGELOG.md` and pinned by
  tests, but not by a requirement, so nothing stops a later change from renaming or dropping it.
- **Recommendation:** Add
  `openspec/changes/improve-hover-content/specs/editor-syntax-highlighting/spec.md` with a
  `## MODIFIED Requirements` or `## ADDED Requirements` entry — e.g. "Hover annotation lines are
  scoped as declarations" — carrying the scenarios `hover-annotation.test.ts` already asserts. The
  capability's purpose statement says "NX source"; the delta should widen it, since this rule
  deliberately scopes a line that NX cannot contain.
- **Fix:** Added `specs/editor-syntax-highlighting/spec.md` with an ADDED requirement, "Hover
  annotation lines are scoped as declarations", carrying seven scenarios — one per assertion the
  tests make, including the negative ones for the ternary and `type`-keyword mis-scopes, the
  container scope on all three shapes, the non-kind-word boundary, and survival through the markdown
  fence. The requirement text states outright that this is the one construct the grammar scopes that
  NX source cannot contain, which is the widening the finding asks for, and listed the capability in
  `proposal.md`.
- **Verification:** Confirmed. `openspec/changes/improve-hover-content/specs/editor-syntax-highlighting/spec.md`
  carries one ADDED requirement with seven scenarios, `openspec validate improve-hover-content`
  passes, and `proposal.md:127` lists the capability. Each scenario matches an assertion the tests
  make, and the kinds the requirement pins ("exactly those hover writes") are the five `hover.rs`
  emits. The widening the finding asked for is explicit: "This is the one construct the grammar
  scopes that NX source cannot contain."

### ✅ Verified - RF12 `meta.annotation.hover.nx` is on one of the three shapes, not all three
- **Severity:** Low
- **Evidence:** Only the first pattern carries `"name": "meta.annotation.hover.nx"`
  (`nx.tmLanguage.json:2477`). Probing the other two:
  `(case) Role.admin` and `(built-in type) Element` produce `meta.annotation.hover.kind.nx` on the
  kind and the type scopes on the rest, with no `meta.annotation.hover.nx` anywhere on the line.
  `src/vscode/CHANGELOG.md` documents the scope as "one line of hover content in that shape", which
  reads as covering every such line, and a theme keying off it — to tint hover content, or to
  suppress a rule inside it — would treat two of the three shapes as ordinary NX. No test asserts
  the meta scope on any line, so nothing catches the gap.
- **Recommendation:** Give the `(case)` match and the bare-type `begin`/`end` the same
  `meta.annotation.hover.nx` name (on a `match` rule, `"name"` scopes the whole match), and assert
  it in `hover-annotation.test.ts` for all three shapes.
- **Fix:** Both were given the name, and `marks every shape as a hover annotation line` asserts it
  on all three.
- **Verification:** Confirmed by probing all three shapes directly. `(property)
  ShapeCommon.shadows: Shadow[]?`, `(case) Role.admin`, and `(built-in type) Element` now carry
  `meta.annotation.hover.nx` on every token, including the kind and the punctuation. `"name"` on the
  `(case)` match scopes the whole match as expected.

### ✅ Verified - RF13 The rule does claim NX source, and the test that appears to prove otherwise tests a different shape
- **Severity:** Low
- **Evidence:** The rule's comment asserts "Requiring a literal kind word at line start keeps this
  out of NX source, where no line begins that way" (`nx.tmLanguage.json:2473`). It does not.
  Tokenizing `<div>` / `(parameter) rights: reserved` / `</div>` scopes the middle line as
  `meta.annotation.hover.nx` with `rights` as `variable.other.property.nx` and `reserved` as
  `entity.name.type.nx`; the same happens for a content line beginning `(case) All.rights`. Element
  content is prose, so a line may begin with any word. The guarding test — `leaves a parenthesized
  expression in source alone`, `hover-annotation.test.ts:112` — uses `let value = { (count) }`,
  which has no `(` at line start and no kind word, so it passes against either anchor and pins
  nothing.
- **Recommendation:** The trade is probably still right — the alternative is no highlighting on
  every hover fragment — but say so honestly rather than claiming it cannot happen: reword the
  comment to bound the cost (one prose line beginning with one of five kind words is mis-scoped),
  and replace the test with one that pins the real boundary, e.g. `(counted) items: many` at column
  zero staying unscoped because `counted` is not a kind word.
- **Fix:** The comment no longer claims it cannot happen. It now says a line-based grammar cannot
  tell hover content from element text that reads the same way, so the cost is bounded rather than
  avoided, and names the trade. Added `leaves a line whose first word is not a kind alone`
  (`(counted) items: many`), which pins the real boundary, and
  `does claim element text that reads like a hover annotation` (`(parameter) rights: reserved`),
  which pins the accepted cost so it is visible rather than discovered. The weak test is kept as a
  third case — it pins something narrower than its name suggested, but it is not wrong. The
  requirement added for RF11 states the bound too.
- **Verification:** Confirmed. The comment no longer claims the shape cannot occur in source; it
  names the trade. `(counted) items: many` is left alone (its `:` is still a ternary's, which is
  right — it is not hover content), and `(parameter) rights: reserved` is claimed, both now pinned
  by tests. Keeping the original test as a third case is the right call: it is narrower than its
  name suggested but not wrong. One residual inaccuracy in the reworded comment is filed as RF18.

### ✅ Verified - RF14 Nothing tests the rule on the path it exists for
- **Severity:** Low
- **Evidence:** `hover-annotation.test.ts` tokenizes bare lines against `source.nx` directly. Hover
  content reaches the grammar only through the fence injection —
  `text.html.markdown` → `source.nx.embedded.markdown` → `source.nx` — and that path wraps the rule
  in a `while` rule (`nx.markdown.codeblock.tmLanguage.json`), which is where an `^`-anchored
  `begin` is most likely to behave differently. I probed it and it works today
  (`meta.embedded.block.nx | meta.annotation.hover.nx | …` on every token), so this is coverage, not
  a defect.
- **Recommendation:** Add one case to `markdown-codeblock.test.ts`, whose fixture registry and
  injection wiring already exist: a ```` ```nx ```` fence containing
  `(property) ShapeCommon.shadows: Shadow[]?`, asserting the property and type scopes survive the
  embedding.
- **Fix:** Added `highlights a hover annotation inside a fenced code block` to
  `markdown-codeblock.test.ts`, using exactly that fence and asserting the property, the type, and
  the annotation colon.
- **Verification:** Confirmed. `highlights a hover annotation inside a fenced code block` runs
  through the real `text.html.markdown` → `source.nx.embedded.markdown` → `source.nx` path and
  asserts the property, the type, and the annotation colon. I also probed the `(case)` shape through
  the same path — `meta.embedded.block.nx | meta.annotation.hover.nx | entity.name.type.union.nx` —
  so the `^` anchor holds inside the `while` rule for every shape, not only the tested one.

### ✅ Resolved - RF15 A function type in a hover annotation is left entirely unscoped
- **Severity:** Low
- **Evidence:** `type_ref_display` renders `TypeRef::Function` as `(int) => bool`
  (`crates/nx-language-service/src/lib.rs:1921-1931`), so hover can emit
  `(parameter) onClick: (int) => bool`. `#type-annotation` has no alternative for a function type,
  so after the annotation colon the whole of ` (int) => bool` comes back with only
  `meta.annotation.hover.nx` — no arrow, no return type. Source does better at the same shape: in
  `let onClick: (int) => bool = handler` the `=>` is `keyword.operator.arrow.nx` and `bool` is
  scoped, because `#value-definition` contributes patterns that this rule's `#type-annotation`
  include does not.
- **Recommendation:** Either include the arrow and type patterns the value-definition context uses
  alongside `#type-annotation` in the hover rule, or extend `#type-annotation` itself with a
  function-type alternative — the second fixes both sites, since a nested annotation in source has
  the same hole.
- **Status:** Not fixed — the premise does not hold, so there is nothing to scope. **NX has no
  function-type syntax.** `crates/nx-syntax/grammar.js:226` defines `type` as
  `choice($.primitive_type, $.user_defined_type)` with only `?` and `[]` suffixes, and both
  `let onClick: (int) => boolean = 1` and the same annotation in a signature analyze to
  `["Syntax error"]`. `TypeRef::Function` is never constructed in `crates/nx-hir/src/lower.rs` — every
  reference to it is a consumer (typegen, codegen, `semantics.rs`) — so no NX source lowers to one
  and hover cannot emit `(parameter) onClick: (int) => bool`.

  I did implement the recommendation's second option first, and it worked: with a function-type
  alternative in `#type-annotation`, the parameter list and arrow scoped correctly in the hover
  line, the value definition, and the declaration signature. I reverted it on finding the above.
  Teaching the grammar a form the parser rejects would highlight invalid source as if it were valid,
  which is worse than the gap — and the "source does better at the same shape" comparison in the
  evidence is itself only reachable in a file that does not parse. Worth noting for whoever adds
  function types to the language: at that point `#type-annotation` needs the alternative, the commit
  is in this conversation's history, and the signature site also mis-scopes the parameter type as
  `variable.other.enummember.nx` today.
- **Verification:** The resolution is correct and my finding's premise was wrong.
  `crates/nx-syntax/grammar.js:226` defines `type` as `choice($.primitive_type, $.user_defined_type)`
  with only `?` and `[]` suffixes, so no type can begin with `(`; and `TypeRef::Function` is
  constructed nowhere outside a `nx-hir` unit test (`crates/nx-hir/src/ast/types.rs:110`) — every
  other mention is a consumer, and `lower.rs` builds only `name`, `nullable`, and `array`. Hover
  cannot emit a function type, so there is nothing to scope. Reverting the working implementation
  rather than shipping it is the right call for the reason given: it would highlight source the
  parser rejects.

### ✅ Verified - RF16 Three unrelated grammar comments are re-escaped to `\uXXXX`
- **Severity:** Low
- **Evidence:** `git diff src/vscode/syntaxes/nx.tmLanguage.json` rewrites the comments on
  `rhs-expression`, `loop-header`, and `loop-header-split-binder` with no change but their em dashes
  and ellipses rewritten as `\u2014` and `\u2026` escapes — incidental churn from a JSON
  re-serializer, in three rules this change has no reason to touch. These are long prose comments meant to be read while editing the grammar, and
  escapes make them harder to read.
- **Recommendation:** Restore the literal characters on those three lines, and in the new
  `hover-annotation` comments, so the diff is purely additive and the file keeps one convention.
- **Fix:** Re-serialized with `ensure_ascii=False`. No `\uXXXX` escape remains anywhere in the file,
  the three unrelated comments are untouched, and
  `git diff --stat src/vscode/syntaxes/nx.tmLanguage.json` is now purely additive.
- **Verification:** Confirmed. Zero `\uXXXX` escapes remain in the file, the three unrelated rule
  comments are absent from the diff, and `git diff src/vscode/syntaxes/nx.tmLanguage.json` is now
  purely additive — the top-level include plus the new rule.

### ✅ Verified - RF17 Three of the third pattern's five kind alternatives are unreachable
- **Severity:** Low
- **Evidence:** The bare-type pattern begins on
  `^\((parameter|property|case|primitive type|built-in type)\)` (`nx.tmLanguage.json:2519`), but
  `hover::parameter` and `hover::property` always emit `name: type`, which the first pattern claims,
  and `hover::union_case` always emits `Union.case`, which the second claims
  (`crates/nx-language-service/src/hover.rs:243-263`). `parameter`, `property`, and `case` there can
  only fire on a fragment hover cannot produce, and no test covers them, so the extra alternatives
  read as uncertainty about the shapes rather than as intent.
- **Recommendation:** Reduce the alternation to `primitive type|built-in type` and note in the
  comment that the other kinds are always claimed above; or, if it is meant as a deliberate fallback
  for a fragment shape yet to exist, say so and pin one with a test.
- **Fix:** Reduced to `primitive type|built-in type`. The pattern's comment now says why: a
  parameter and a property always carry `name: type` and a case always carries `Union.case`, so the
  two patterns above claim those first. There was no intent behind the extra alternatives — they
  were uncertainty, as the finding read them.
- **Verification:** Confirmed. The alternation is `primitive type|built-in type` and the comment
  says why. Nothing hover emits lost its scoping: `(case) Role` with no dot is now unclaimed, but
  `hover::union_case` always writes `Union.case`. I also checked the kebab risk the reduction could
  have introduced — a qualifier is a type or case name, and `crates/nx-syntax/grammar.js:961` gives
  `identifier` no hyphen, so the first pattern's qualifier group never has to match one.


## New Findings Discovered During 2026-09-06 18:08 Verification

### ✅ Verified - RF18 The reworded rule comment bounds the cost by the wrong count
- **Severity:** Low
- **Evidence:** The `hover-annotation` comment now reads "a line of prose beginning with one of the
  two kind words below, in the shape these patterns match, is scoped as a hover annotation"
  (`src/vscode/syntaxes/nx.tmLanguage.json:2474`). There are five kind words across the rule's three
  patterns, not two — `property`, `parameter`, `case`, `primitive type`, and `built-in type` — and
  all five claim prose: I probed `(case) All.rights` written as element text and it is scoped
  `meta.annotation.hover.nx | entity.name.type.union.nx | …`, exactly as `(parameter) rights:
  reserved` is. "Two" reads like a leftover from the RF17 edit, which reduced the third pattern's
  alternation to two kinds. This is the same class of defect RF13 opened — a comment stating a bound
  that the rule does not hold to — so it is worth correcting rather than leaving.
- **Recommendation:** Say "one of the five kind words these patterns recognize". The delta added for
  RF11 states the bound correctly and needs no change.
- **Fix:** The comment now reads "one of the five kind words these patterns recognize —
  `property`, `parameter`, `case`, `primitive type`, `built-in type` — in the shape the pattern for
  that kind matches". Naming them beats counting them, since the count is what went stale. It was
  indeed a leftover from the RF17 edit. Also widened the test that pins the accepted cost: it
  covered only `(parameter) rights: reserved`, so it would not have caught a claim that held for the
  annotated shape but not the dotted one; it now also asserts `(case) All.rights`, the second case
  the finding probed. The RF11 delta is unchanged, as recommended.
- **Verification:** Confirmed. The comment names all five kinds, and they match the three patterns'
  alternations exactly — `property|parameter`, `case`, `primitive type|built-in type` — so the bound
  is now stated by the same words the regexes use and cannot go stale against a count. Naming rather
  than counting is the better fix. The widened test is a real improvement over what I recommended:
  it pins both shapes the finding probed, and the dotted one is the case a claim holding only for
  the annotated shape would have slipped past. `pnpm run test:grammar` is green at 251 passing, and
  `git diff --numstat` on the grammar is 79 added, 0 removed — still purely additive.

