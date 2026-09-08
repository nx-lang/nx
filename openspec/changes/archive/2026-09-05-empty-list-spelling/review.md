# Review: empty-list-spelling

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md` (sections 1–10),
`specs/braced-value-sequences/spec.md`, `specs/primitive-type-names/spec.md`,
`specs/unbraced-literal-forms/spec.md`. (`obsolete-review.md` read for context only.)

**Reviewed code:** the working tree against `HEAD` (0fc0463) —
`crates/nx-syntax/grammar.js` and the regenerated `src/{grammar.json,node-types.json,parser.c}`,
`crates/nx-hir/src/lower.rs`, `crates/nx-types/src/{ty.rs,semantics.rs,infer.rs}`,
`crates/nx-interpreter/src/interpreter.rs`, `crates/nx-cli/src/format.rs`,
`crates/nx-cli/src/typegen.rs` and `typegen/languages/{csharp,typescript}.rs`,
`crates/nx-codegen/src/{emit.rs,ir.rs}`, `crates/nx-language-service/src/lib.rs`,
`src/vscode/syntaxes/nx.tmLanguage.json`, `README.md`, `nx-grammar.md`, `nx-grammar-spec.md`,
and the tests in `crates/nx-types/tests/{empty_lists,never_is_not_a_primitive,void_is_not_a_primitive}.rs`,
`crates/nx-syntax/tests/{parser_tests,tree_helpers}.rs`,
`crates/nx-interpreter/tests/{arrays,edge_cases}.rs`.

**Verification performed independently of the tasks:**

- `cargo test --workspace` — all green, 0 failures.
- `openspec validate empty-list-spelling --strict` — valid.
- Regenerated the parser from `grammar.js` with `tree-sitter 0.25.10`: the checked-in
  `parser.c`, `grammar.json` and `node-types.json` are byte-identical to a fresh generate, so the
  generated artifacts are in sync with the source grammar.
- Compared the tree-sitter conflict set from `HEAD`'s `grammar.js` against the working tree's,
  order-insensitively: **identical (25 entries each)**. Design D1's and D8's "costs no new
  ambiguity" claim holds as stated.
- Ran all 111 `.nx` files in the repository through `nxlang run` from a `HEAD` worktree and from
  the working tree and diffed the full output (see RF4).
- Exercised ~35 additional type-checking cases through `check_str` beyond those in the test suite
  (scalar sites, nullable sites, `object` sites, call arguments at every arity, record-constructor
  calls, `for`/`if`/arm bodies, content splicing, state declarations, nested annotations).
- Generated TypeScript for `<Box>{}</Box>` versus `<Box />` at a content property with a non-empty
  default (see RF6).

**Overall:** the core design is sound and the section-10 rewrite is a clear improvement — replacing
the `pending_empty_lists` apparatus with a real bottom type removed the whole class of
site-by-site plumbing failures the previous review found. Every behavioural claim I spot-checked in
the type checker, interpreter, formatter and code generator held. The findings below are one real
requirement gap, one false verification claim, and a cluster of documentation/test-strength issues
left behind by the supersession of section 9.

## Findings

### ✅ Verified - RF1 Three first-party listings of the primitive names still name `void`
- **Severity:** Medium
- **Evidence:** The `primitive-type-names` delta requires *"When first-party documentation lists the
  primitive type names — the list SHALL be exactly the primitive set — AND SHALL NOT include
  `void`"*, and task 8.10 claims *"Verify no first-party listing of the primitive names still names
  it."* Three do:
  - [src/vscode/README.md:11](src/vscode/README.md#L11) — `Primitive types: string, int, int32, int64, float32, float64, boolean, void, object`. This one is doubly wrong now: it is describing the coverage of the TextMate grammar in [src/vscode/syntaxes/nx.tmLanguage.json](src/vscode/syntaxes/nx.tmLanguage.json), which this change just stopped highlighting `void` in.
  - [nx-planning.md:47](nx-planning.md#L47) — `Primitive types: string, int, int32, int64, float32, float64, boolean, void, object`
  - [nx-rust-plan.md:664](nx-rust-plan.md#L664) — same list.

  The last two are not incidental misses: the archived `rename-primitive-types` review reopened a
  finding *twice* specifically over these two files, established that they are in-scope first-party
  NX type inventories rather than host-language lists, and updated them. This change's sweep put
  them back out of step. A fourth, lower-priority hit is
  [docs/drawn-ui-proposal-nx-enhancements.md:337](docs/drawn-ui-proposal-nx-enhancements.md#L337),
  which quotes the set inline as part of an argument.
- **Recommendation:** Drop `void` from all three inventories (`src/vscode/README.md`,
  `nx-planning.md`, `nx-rust-plan.md`). Decide explicitly whether the DrawnUI proposal doc counts —
  if it is treated as a historical artifact, say so in the tasks rather than leaving it unmentioned.
  Re-run the same word-level `*.md` sweep the prior change's third pass documented, since that is
  the sweep this change needed and did not repeat.

- **Fix:** All three cited inventories corrected, plus two the sweep also reaches:
  `src/vscode/README.md:11`, `nx-planning.md:47`, `nx-rust-plan.md:664`,
  `specs/001-nx-core-parsing/spec.md:95` (not cited here — it is the exact line the
  `rename-primitive-types` change updated, so leaving it would have re-broken that change a second
  time; it was also missing `object`, so it now reads as the eight canonical names), and
  `docs/drawn-ui-proposal-nx-enhancements.md:337`. On the open question: the DrawnUI proposal is
  **in scope**, by the boundary that change settled — every `*.md` outside `node_modules`, `target`,
  and `openspec/changes/archive` — and dropping `void` there does not weaken its argument, since a
  union case named `void` is exactly what the freed name now permits. Task 8.10 records the sweep
  and its deliberate exclusions.
- **Note on the remaining `void` hits:** the word-level sweep also finds
  `docs/src/content/docs/reference/syntax/types.md:13`,
  `docs/src/content/docs/overview/design-goals.md:39`, and
  `docs/src/content/docs/tutorials/building-your-first-component.md:13`, which write `void` as a
  function-type *return* — live docs, not planning artifacts. These are deliberately **not**
  rewritten. NX has no function-type syntax at all: `type EventHandler = (string) => string` and
  `onClick:() => object` are syntax errors on the working-tree build exactly as the `void` forms
  are, so those examples were already unparseable before this change, and rewriting `void` there
  would swap one unparseable spelling for another while implying a return type the language cannot
  express. Recorded as task 10.9. Also left: `nx-rust-plan.md:509` (`const void* tree`, C) and
  `nx-grammar-spec.md:267-269` (prose that already states the new rule).
- **Verification:** Confirmed. All five inventories now read as the eight canonical names
  ([src/vscode/README.md:11](src/vscode/README.md#L11), [nx-planning.md:47](nx-planning.md#L47),
  [nx-rust-plan.md:664](nx-rust-plan.md#L664),
  [specs/001-nx-core-parsing/spec.md:95](specs/001-nx-core-parsing/spec.md#L95),
  [docs/drawn-ui-proposal-nx-enhancements.md:337](docs/drawn-ui-proposal-nx-enhancements.md#L337));
  the last also gained the `object` it was missing. I re-ran the word-level `*.md` sweep at the
  boundary task 8.10 now states, and the residue is exactly what task 10.9 records — the three
  live-doc function-type returns, `nx-rust-plan.md:509` (C), `nx-grammar-spec.md:267-269` (prose
  stating the new rule), and the speculative uses in `nx-planning-future.md`. That last file is
  named in task 10.9 but not in this Fix note, which enumerates the residue as though it were
  complete; the task is the authoritative record, so this is a mismatch between two notes rather
  than a missed file. `openspec/specs/primitive-type-names/spec.md:10,215` still names `void`,
  correctly — that is the pre-archive spec, and this change's `MODIFIED Requirements` delta replaces
  the set when it lands. The in-scope call on the DrawnUI proposal matches the boundary the prior
  change was reopened to establish, and dropping `void` there does not weaken its argument.
### ✅ Verified - RF2 The "byte-identical output before and after" verification does not hold
- **Severity:** Medium
- **Evidence:** Tasks 6.3, 8.11 and 10.8 each claim the repository's `.nx` corpus produces
  identical output before and after, and design's Migration Plan repeats it (*"their output is
  byte-identical before and after"*). Running all 111 files through `nxlang run` from a `HEAD`
  worktree and from the working tree, one file differs:
  [src/vscode/samples/tally-survey.nx](src/vscode/samples/tally-survey.nx). Before: 2 diagnostics.
  After: 16. The file is already invalid before and after — it uses an unsupported positional
  attribute form, `<Option "Yes, borrowed"/>` — so no valid program changed meaning, and the
  substantive inertness claim survives. But the recovery cascade got materially worse, and the new
  lead diagnostic is actively misleading:

  ```
  error src/vscode/samples/tally-survey.nx:31:22: Unclosed brace
     31 |         if allowBoth {
        |                      ^ unexpected syntax here
  note: Add a closing '}' to match the opening brace
  ```

  That brace *is* closed two lines later. Admitting `{}` means `{` followed by `}` is now a valid
  parse, so recovery takes a different path through malformed input and reports an unclosed brace
  where none exists. Total corpus diagnostics went from 225 to 239, all of the increase in this one
  file.
- **Recommendation:** Correct the three task claims and the Migration Plan to say what is actually
  true — *no valid program changes output; one already-invalid sample's error recovery cascades
  differently* — and name the file. If the misleading "Unclosed brace" on a balanced brace is
  considered a diagnostic-quality regression worth fixing, that is a separate change; either way it
  should be recorded rather than asserted away.

- **Fix:** Confirmed independently and corrected, with different numbers than reported. Built
  `nxlang` in a detached `HEAD` (0fc0463) worktree, ran all 111 `.nx` files through both binaries,
  and diffed per file: exactly one file differs, `src/vscode/samples/tally-survey.nx`, as reported.
  The counts are **21 -> 35** diagnostics in that file, not 2 -> 16 — which is what accounts for the
  corpus-wide 225 -> 239 (+14) you also measured. The misleading `Unclosed brace` at line 31 on a
  brace closed two lines later is real and reproduced. Tasks 6.3, 8.11 and 10.8 and the design's
  Migration Plan now say "no valid program changes output" rather than "byte-identical", name the
  file, carry the corrected counts, and record the recovery regression explicitly as a follow-up
  candidate rather than asserting it away.
- **Verification:** Confirmed — and **the correction is right; my counts were wrong, not yours.**
  I rebuilt `nxlang` in a detached `0fc0463` worktree and re-ran all 111 `.nx` files through both
  binaries, diffing full output per file: exactly one file differs,
  `src/vscode/samples/tally-survey.nx`, at **21 → 35** diagnostics, with the corpus total
  **225 → 239**. The `2 → 16` in this finding's evidence was a miscount; the corpus figure was
  right. The misleading `Unclosed brace` at line 31 on a brace closed two lines later reproduces.
  Tasks 6.3, 8.11 and 10.8 and the design's Migration Plan now all say "no valid program changes
  output" rather than "byte-identical", name the file, and carry the corrected counts. Running the
  corpus after the RF6 recovery change also confirms that change introduced no new differing file:
  the differing set is still this one file alone.
### ✅ Verified - RF3 Comments describing the section-9 machinery survived its deletion
- **Severity:** Low
- **Evidence:** Section 10 deleted `pending_empty_lists`, the end-of-analysis sweep,
  `discharge_pending_empty_lists`, `resolve_empty_lists_in` and `settle_empty_lists`. Several
  comments still describe them as if they exist, which will send the next reader looking for code
  that is not there:
  - [crates/nx-types/src/infer.rs:1011-1014](crates/nx-types/src/infer.rs#L1011-L1014) — *"Dropping
    the pending entries keeps the end-of-analysis sweep from following an unusable call with
    'annotate the binding'"*, sitting above a bare `if func_ty.is_error() { return Type::Error; }`
    that drops nothing. Nothing in that function discharges anything any more, and nothing needs
    to.
  - [crates/nx-types/tests/empty_lists.rs](crates/nx-types/tests/empty_lists.rs) — the doc on
    `a_mismatch_on_an_empty_list_does_not_name_the_element_variable` (*"The element type of an
    unresolved empty list is an inference variable"*), on
    `an_empty_list_contributes_no_items_to_the_sequence_it_sits_in` (*"Joining the empty list's
    element variable"*), on `an_unresolved_empty_list_does_not_escape_into_an_inferred_signature`
    (*"A list whose element is still a variable"*), on
    `an_unannotated_function_whose_arms_are_all_empty_is_reported` (*"settling the arms … against a
    live variable … The demand moves to the `if`"*), and the section banner *"The demand moves
    outward rather than being dropped"*. There is no variable, no settling, and no demand that
    moves; the type is `never[]` from the moment it is inferred.
  - [crates/nx-types/src/ty.rs:242](crates/nx-types/src/ty.rs#L242) — the doc comment *"Creates a
    primitive void type."* is now attached to `Type::never()`, and `Type::void()` was left with
    none. `is_compatible_with`'s doc comment ([ty.rs:336-341](crates/nx-types/src/ty.rs#L336-L341))
    enumerates the compatibility rules and was not extended with the bottom-type rule the change
    added directly below it.
- **Recommendation:** Rewrite the `infer_call` comment to state the actual reason for the early
  return (an uncheckable call has already reported; there is nothing to add). Re-word the five test
  docs in terms of `never` and the join, not in terms of variables and demands. Give `Type::never()`
  its own doc and restore `Type::void()`'s; add the bottom-type bullet to `is_compatible_with`'s
  list.

- **Fix:** All four sites corrected. The `infer_call` comment now states the actual reason for the
  early return (an uncheckable call has already reported; a second diagnostic would point downstream
  of the real fix). `Type::never()` gets its own doc and `Type::void()`'s is restored — the doc had
  indeed been left attached to the wrong function. `is_compatible_with`'s rule list gains the
  bottom-type bullet. The five test docs in `empty_lists.rs` are rewritten in terms of `never` and
  the join; the section banner is now "A join of empty lists is still empty, and the binding it
  fixes is still named", and two tests whose *names* carried the stale model were renamed
  (`a_mismatch_on_an_empty_list_does_not_name_the_element_variable` ->
  `..._does_not_name_the_element_type`, and see RF6 for the other). The two "a type variable must
  not reach a diagnostic" assertions were also strengthened to reject `never`, which is the name
  that could actually leak now — both pass.
- **Verification:** Every site this finding cited is fixed. `Type::never()` and `Type::void()` each
  carry their own doc ([ty.rs:242-249](crates/nx-types/src/ty.rs#L242-L249)),
  `is_compatible_with`'s rule list gained the bottom-type bullet
  ([ty.rs:337-345](crates/nx-types/src/ty.rs#L337-L345)), the `infer_call` early return now states
  the real reason ([infer.rs:1016-1018](crates/nx-types/src/infer.rs#L1016-L1018)), and all five
  test docs plus the section banner are rewritten in terms of `never` and the join. The two renames
  are improvements, and strengthening the "must not reach a diagnostic" assertions to reject
  `never` goes beyond what was asked. **Two comments of the same class survive at sites this
  finding did not cite** — see RF8. They do not reopen this finding, but the class is not fully
  cleared.
### ✅ Verified - RF4 The record-constructor argument test is vacuous
- **Severity:** Low
- **Evidence:** `empty_list_is_accepted_as_a_record_constructor_argument` in
  [crates/nx-types/tests/empty_lists.rs](crates/nx-types/tests/empty_lists.rs) asserts
  `type Row = { cells:string[] }\nlet value = {Row({})}` is clean, backing task 7.4's *"and that a
  record constructor call accepts one"*. Record-constructor call arguments are not type checked at
  all, so the assertion cannot fail. All of these also report nothing:
  `Row("x")` at `type Row = { n:int }`, `Row({})` at `n:int`, `Row({"a" "b"})` at `n:int`, and
  `Row(1, 2)` at a one-field record. Contrast `let f(s:string): int = 1; f({})`, which correctly
  reports `Argument 0 expects string, found {}`. (The gap is pre-existing and not caused by this
  change — but it means the test verifies nothing about the empty list.)
- **Recommendation:** Either assert something the empty list can actually fail — e.g. that
  `Row({})` at a *non*-list field is rejected, which would currently fail and expose the real gap —
  or drop the case and note in task 7.4 that record-constructor arguments are unchecked, with a
  pointer to the pre-existing gap. Leaving a green test that would stay green under the opposite
  implementation is the worst of the three.

- **Fix:** Confirmed — all four of your probes report nothing on the working-tree build, and
  `f({})` at a `string` parameter correctly reports one. Took the second option rather than adding a
  knowingly-failing test: the case is now
  `record_constructor_arguments_are_unchecked_including_the_empty_list`, which asserts all four
  shapes are accepted and documents that this pins the *gap*, not the empty list. It can no longer
  stay green under an implementation that rejected the empty list everywhere, and it is expected to
  fail when record-constructor arguments do get checked — at which point it becomes the real
  assertions. Task 7.4 records the gap and points at the two tests that do exercise `{}` at an
  argument position against real functions.
- **Verification:** Confirmed, and this is the right one of the three options. The replacement test
  asserts all four shapes clean and its doc says it pins the *gap*; it can no longer stay green
  under an implementation that rejected `{}` everywhere, because `Row("x")` at an `int` field and
  `Row(1, 2)` at a one-field record would still have to pass. Task 7.4 records the gap and names the
  two tests that exercise `{}` at an argument position against real functions. All 35 tests in
  `empty_lists.rs` pass.
### ✅ Verified - RF5 Task 8.2's interpreter/codegen agreement has no test
- **Severity:** Low
- **Evidence:** Task 8.2 is checked: *"Verify the interpreter and the code generator agree: the same
  source that evaluates to an empty list also generates TypeScript binding `[]`."* `crates/nx-codegen`
  has no `tests/` directory, and no test anywhere in the repository covers it — `grep` for
  `<Box>{}` finds only the parser fixture and the interpreter tests. The two implementations of the
  written-body-versus-absent-body rule are genuinely independent:
  [interpreter.rs:2693-2698](crates/nx-interpreter/src/interpreter.rs#L2693-L2698) keys on
  `content_exprs.is_empty()`, while codegen keys on `!content.is_empty()` in three separate places
  ([emit.rs:2924](crates/nx-codegen/src/emit.rs#L2924),
  [emit.rs:2991](crates/nx-codegen/src/emit.rs#L2991),
  [builder.rs:1611](crates/nx-codegen/src/builder.rs#L1611)). I confirmed manually that they agree
  today — `<Box>{}</Box>` generates `({ $type: "Box", items: [] })` and `<Box />` generates the
  declared default — but nothing pins it, and this is exactly the rule design's Migration Plan
  calls the one shape whose meaning changed.
- **Recommendation:** Add the codegen assertion the task claims, or restate 8.2 as a manual
  verification with the observed output recorded. Given that the rule is duplicated across four
  call sites in two crates, a test is the cheaper of the two.

- **Fix:** Test added. `crates/nx-codegen` has no `tests/` directory, as you note, but it does have
  an in-crate `src/tests.rs` with both `generated_file` and `eval_program_artifact` helpers, so both
  implementations can be driven from one source in one test:
  `an_empty_written_body_generates_an_empty_list_and_an_absent_one_takes_the_default` asserts
  `<Box>{}</Box>` generates `items: []` and evaluates to `{"$type":"Box","items":[]}`, and that
  `<Box />` generates neither and evaluates to the declared default. Verified non-vacuous by dumping
  the generated TypeScript for both: `items: []` appears only in the written-body case, and the
  absent-body case emits the default-binding closure instead. Task 8.2 updated.
- **Verification:** Confirmed, and independently checked for non-vacuity. The test passes, and
  generating TypeScript for the two sources by hand shows they differ exactly as asserted: the
  written body emits `return ({ $type: "Box", items: [] });`, the absent body emits
  `return (() => { const __nx_field_0 = ({ $type: "A", n: 9 }); return { $type: "Box", items: __nx_field_0 }; })();`.
  Driving both implementations from one source in one test is a better answer than the two separate
  assertions the recommendation suggested, since it is their *agreement* that was unpinned.
### ✅ Verified - RF6 An unannotated value binding poisons to `Type::Error`; an unannotated function return does not
- **Severity:** Low
- **Evidence:** The two halves of D3's diagnostic recover differently, and nothing says why.
  [infer.rs:2966-2983](crates/nx-types/src/infer.rs#L2966-L2983) reports the value binding and
  returns `Type::Error`, so `let a = {}` gives `a` the error type. The function-return arm at
  [infer.rs:526-541](crates/nx-types/src/infer.rs#L526-L541) reports and then returns
  `body_ty.clone()`, so `let f() = {}` keeps the type `() => never[]`. The consequence is that after
  its single diagnostic, `f` remains usable at every list type at once:

  ```nx
  let f() = {}
  let a:string[] = {f()}   // accepted
  let b:int[]    = {f()}   // accepted
  ```

  That is defensible — design's Risks section explicitly blesses one `{}` inhabiting `string[]` and
  `int[]` — but then the value-binding side poisoning to `Error` is the odd one out, and the
  divergence is undocumented in both the code and D3. It also weakens
  `an_unresolved_empty_list_does_not_escape_into_an_inferred_signature`, whose doc comment says
  *"one value must not inhabit both `string[]` and `int[]` unreported"*: that test passes only on
  the `f`-naming diagnostic, not on the two binding sites, which are both accepted. Under the
  `never` design its stated rationale is no longer the design's position.
- **Recommendation:** Pick one recovery and say so in D3 — either both report-and-keep the
  `never`-bearing type, or both poison. Then rewrite that test's doc comment to match what the
  design now says (the annotation is required for legibility; inhabiting both list types is
  correct) and to assert the thing the test actually pins.

- **Fix:** Made both sites report-and-keep; the value binding no longer returns `Type::Error`. That
  is the direction the code itself argued for — the comment at the value site already said the
  annotation is required "for legibility, not because the system cannot type it", and then poisoned,
  which is what made the divergence undocumented in both places at once. `let a = {}` now reports
  once and `a` goes on satisfying `string[]` and `int[]`, exactly as `f` did. It is also the better
  recovery: under the old poisoning a genuine later mismatch was suppressed, whereas
  `let a = {}` followed by `let s:string = {a}` now reports the element-type diagnostic *and*
  `expects string, found {}`. D3 gains a **Recovery** paragraph stating the rule and why it follows
  from the rule being about legibility; task 10.10 records it. The whole suite passes unchanged, so
  nothing depended on the poisoning. The test you flagged is renamed
  `an_empty_list_in_an_inferred_signature_is_reported_at_the_function` and its doc now says what it
  actually pins — the diagnostic, not the two uses, which are correct.
- **Verification:** The code change is confirmed at both sites — the value binding returns `actual`
  rather than `Type::Error` ([infer.rs:2985](crates/nx-types/src/infer.rs#L2985)), matching the
  function-return arm ([infer.rs:542](crates/nx-types/src/infer.rs#L542)) — and D3 gained the
  **Recovery** paragraph. Behaviour checked directly: `let a = {}` reports exactly once; followed by
  `let b:string[] = {a}` and `let c:int[] = {a}` it still reports only that one diagnostic; and
  followed by `let s:string = {a}` it reports *both* the element-type diagnostic and
  `expects string, found {}`, which the old poisoning suppressed. A strict improvement, and the
  whole suite passes unchanged. **One correction to my own finding:** its evidence claimed
  `let f() = {}` with `let a:string[] = {f()}` and `let b:int[] = {f()}` were accepted. They are
  not, and never were — both report `expects string[], found T0`. That is a pre-existing defect
  unrelated to this change (it reproduces on `0fc0463`, and for any unannotated function, not only
  empty ones), but it does mean D3's new Recovery paragraph asserts a symmetry that does not hold at
  a call site. See RF9.
### 🔴 Open - RF7 Unrelated edits are bundled in the working tree under review
- **Severity:** Low
- **Evidence:** [examples/nx/types.nx](examples/nx/types.nx) is modified with content that belongs
  to the previous change, not this one — a `type Inset` with `float64 = 0` defaults, `let count = 8`,
  and a comment about an integer literal taking its type from a float site. That is commit
  0fc0463's subject ("Type an integer literal by the float site it is written at"), and nothing in
  this change's tasks touches that file. Also untracked and unrelated:
  `docs/scratch-highlighting.nx`, `drawnup-startup-fixes-review.md`,
  `examples/nx/template-candidate.nx`, and `openspec/changes/resolve-editor-positions/`.
- **Recommendation:** Commit or drop the `types.nx` hunk separately so this change's diff is only
  this change. Nothing here is wrong; it just makes the change harder to review and to revert as a
  unit, which the Migration Plan's "rollback is reverting it" depends on.

- **Status:** Not addressed — this is the repository owner's call, not a code fix, and acting on it
  unilaterally would mean committing or discarding work belonging to other changes. The observation
  is correct and has been raised repeatedly: `examples/nx/types.nx` carries commit 0fc0463's
  subject, and `docs/scratch-highlighting.nx`, `drawnup-startup-fixes-review.md`,
  `examples/nx/template-candidate.nx` and `openspec/changes/resolve-editor-positions/` are untracked
  and unrelated. Recommendation unchanged: commit or stash the `types.nx` hunk separately before
  archiving, so the Migration Plan's "rollback is reverting it" holds for a diff that is only this
  change.
## Questions

- Is `docs/drawn-ui-proposal-nx-enhancements.md:337` in scope for the primitive-set sweep, or is a
  landed proposal document treated as a historical artifact? The prior change defined the exclusion
  boundary as "not under an excluded archive path or labeled as superseded", which would put it in
  scope; this change should either follow that or restate the boundary.
- The misleading "Unclosed brace" that RF2 surfaced comes from recovery, not from the new rule
  itself. Is a diagnostic-recovery follow-up worth filing, or is it acceptable that malformed
  sources recover differently now that `{}` is grammatical?
- D5 keeps `void` as the rendering of `Type::Void` and D10 renders the bottom type as `never`, both
  names an author cannot write. Diagnostics spell an author-written `{}` as `{}` — but a hover or a
  completion detail in `nx-language-service` renders the inferred type directly. Should editor
  surfaces get the same `{}` treatment diagnostics got, or is `never[]` acceptable there?

## Fix pass

**Fixed: 6** — RF1, RF2, RF3, RF4, RF5, RF6. **Open: 1** — RF7 (owner's call; see its Status note).

Verification after the pass: `cargo test --workspace` 1447 passed / 0 failed (was 1446; the RF5
test is new, and RF4 replaced one test with another). `cargo fmt --all -- --check` reports exactly
the pre-existing five files and none of the four this pass edited. `openspec validate
empty-list-spelling --strict` valid. The corpus was re-run against the `HEAD` worktree after the
RF6 recovery change: output is unchanged from before it, and the differing-file set is still the
single already-invalid `tally-survey.nx` recorded in task 10.8.

### Answers to the Questions

- **Is `docs/drawn-ui-proposal-nx-enhancements.md:337` in scope?** Yes, and it is fixed. The
  `rename-primitive-types` change settled the boundary as every `*.md` outside `node_modules`,
  `target`, and `openspec/changes/archive`, with no exemption for landed proposals — it explicitly
  rejected its own "historical record" rationale after being reopened. Task 8.10 now states that
  boundary rather than leaving it to be re-derived.

- **Is the misleading `Unclosed brace` worth a follow-up?** Recorded, not fixed. It is a recovery
  artifact reachable only from source that already fails to parse, so no valid program is affected
  and it does not block this change; but it is a genuine diagnostic-quality regression (a brace that
  *is* closed reported as unclosed), so task 10.8 names it as a follow-up candidate rather than
  dismissing it. Fixing it means touching error recovery, which is out of scope here.

- **Should editor surfaces spell `never[]` as `{}`?** ~~Left as `never[]`, deliberately…~~
  **Superseded — the premise was wrong, and the answer's reasoning does not survive it.**

  There is no editor surface that renders an inferred type. `Snapshot::hover`
  ([lib.rs:316-338](crates/nx-language-service/src/lib.rs#L316-L338)) returns
  `format!("{} `{}`", symbol.kind.display_name(), symbol.name)` — "value `a`", "function `f`" — and
  `DocumentSymbol` ([lib.rs:745-754](crates/nx-language-service/src/lib.rs#L745-L754)) has no type
  field at all. Completion `detail` is a category label ("primitive type", "built-in type"), and the
  one place a type reaches a completion is `base_type_name`, which reads the *syntactic* `TypeRef`
  the author wrote. Nothing in `nx-language-service` or `nx-lsp` touches `expr_types`, `type_env`,
  or `Display for Type`. So neither `never[]` nor `void` can appear in a hover today; the claim that
  "a hover already shows `void`" was false.

  The taste call the answer agonized over is therefore not live, and where it *is* live —
  diagnostics — the change already decided it the right way and implemented it thoroughly.
  `empty_list_display` renders `{}` at every depth and shape: `found {}`, `found {}?`,
  `found list {}[]`, `found list {}[][]`. `never` cannot reach a message. The principle the answer
  hedged on — a diagnostic should name a form the author can actually write — is the correct one,
  and it should be the standing rule for any future surface that renders inferred types, hover
  included. Nothing to change in code.

### Two corrections to the report

- **RF2's counts.** The direction and the file are right; the numbers are not. `tally-survey.nx`
  goes 21 -> 35, not 2 -> 16. The corpus total 225 -> 239 you also report is the +14 that follows
  from 21 -> 35, so the two figures in the finding were inconsistent with each other.

- **RF1 undercounts by two.** `specs/001-nx-core-parsing/spec.md:95` and
  `docs/drawn-ui-proposal-nx-enhancements.md:337` are the same class of first-party inventory, and
  the former is the exact line the prior change was reopened over. Both are now fixed. Separately,
  the sweep surfaces three *live documentation* `void` uses that are not inventories and are
  deliberately left — see RF1's second note and task 10.9; they are pre-existing breakage, not
  caused by this change.

## New Findings Discovered During 2026-09-05 11:52 Verification

### ✅ Verified - RF8 Two section-9 comments survive at sites RF3 did not cite
- **Severity:** Low
- **Evidence:** RF3 is stated as a class — comments describing the deleted section-9 machinery — and
  every site it enumerated is fixed. Two more of the same class remain, both in `infer.rs`:
  - [crates/nx-types/src/infer.rs:2558-2559](crates/nx-types/src/infer.rs#L2558-L2559) — *"The
    element variable of an empty list is an inference artifact, not something the author wrote;
    naming it `T0[]` describes the implementation rather than the source."* This sits above the call
    to `empty_list_display`. Under section 10 the empty list has no element variable and would never
    render as `T0[]`; it is `never[]`. Both nouns in the comment describe the deleted model, and the
    real reason for the rendering — `never` has no source spelling — is the one the rewritten test
    doc on `a_mismatch_on_an_empty_list_does_not_name_the_element_type` already gives.
  - [crates/nx-types/src/infer.rs:3336-3340](crates/nx-types/src/infer.rs#L3336-L3340) — the arm for
    `(Type::Primitive(Primitive::Never), _) => true` carries two stacked rationales. The new one
    ("The bottom type satisfies every expectation…") was added *below* the old one, which still
    reads *"An empty list satisfies any list type… so a `T[]` site is satisfied without the element
    variable having to match `T`."* The old three lines describe the arm that used to be there, and
    the placement claim they make ("Placed before the nullable and array cases") is now attached to
    a different arm than the one it was written for.
- **Recommendation:** Fold each pair into one comment in `never` terms. At 2558, say that `never`
  has no source spelling so rendering the type directly would put an unwritable name in front of
  someone who wrote `{}`. At 3336, keep the two new lines and delete the three old ones, or restate
  the placement reason for the `Never` arm as it now stands.
- **Fix:** Both rewritten in `never` terms. At `infer.rs:2557` the comment now says `never` has no
  source spelling, so rendering the type directly would put an unwritable name in front of someone
  who wrote `{}`. At the `Never` arm the three old lines are deleted and the two new ones kept; the
  placement claim went with them, because it is no longer load-bearing — the arm matches on the
  actual type alone, and the only arm above it is guarded by `is_null_literal_type`, which `never`
  is not.
- **Verification:** Both confirmed. [infer.rs:2558-2559](crates/nx-types/src/infer.rs#L2558-L2559)
  now reads *"`never` has no source spelling, so rendering an empty list's type directly would put a
  name the author cannot write in front of someone who wrote `{}`"*, and the `Never` arm
  ([infer.rs:3337-3339](crates/nx-types/src/infer.rs#L3337-L3339)) carries only the two bottom-type
  lines. The reasoning for dropping the placement claim holds: `is_null_literal_type` matches
  `Nullable(Variable)` ([infer.rs:3369](crates/nx-types/src/infer.rs#L3369)), which `never` is not,
  so the one arm above it cannot intercept. Re-running the stale-vocabulary sweep across
  `crates/nx-types/{src,tests}` for `element variable`, `inference variable`, `T0[]`,
  `pending_empty_lists`, `end-of-analysis` and `demand moves` now returns **no hits at all** — the
  class RF3 opened is fully cleared.

### ✅ Verified - RF9 D3's new Recovery paragraph asserts a symmetry that does not hold at a call site
- **Severity:** Low
- **Evidence:** The **Recovery** paragraph added to D3 for RF6 says *"an unannotated `{}` binding
  goes on inhabiting every list type after it is reported — `let a = {}` satisfies both `string[]`
  and `int[]`, exactly as `let f() = {}` does"*. The first half is true and I verified it. The
  second half is not observable: an unannotated function's return type reaches its call sites as a
  bare type variable, so

  ```nx
  let f() = {}
  let b:string[] = {f()}   // error: Initializer for value 'b' expects string[], found T0
  ```

  reports at every call site, and `T0` — a name no author can write — reaches the diagnostic. This
  is **pre-existing and not caused by this change**: it reproduces on `0fc0463`, and for any
  unannotated function, e.g. `let f() = {"a"}` used at a `string[]` site fails the same way. But the
  design now leans on `f`'s behaviour to justify the value binding's, and `f` does not behave that
  way. (This also corrects RF6's own evidence, which asserted those two lines were accepted; see its
  Verification note.)
- **Recommendation:** Reword the Recovery paragraph to state the rule for the value binding without
  claiming the function side is observably symmetric — the two *sites* recover the same way, which
  is what 10.10 actually established, even though an unannotated return is unusable at a call site
  for an unrelated reason. Separately, the `T0` leak is worth its own change: it is a type variable
  escaping into a user-facing diagnostic, which is the same class of leak the empty-list rendering
  work was done to prevent.
- **Fix:** Confirmed the finding first — `let f() = {}` at a `string[]` site does report `found T0`,
  and `let f() = {"a"}` fails identically, so the leak is pre-existing and not about empty lists.
  D3's Recovery paragraph now states the rule for the value binding on its own and drops the
  "exactly as `let f() = {}` does" clause; a following paragraph says the two sites recover the same
  way while only the value binding's recovery is observable downstream, names the type-variable leak
  as a separate pre-existing defect of the class `empty_list_display` exists to prevent, and assigns
  it to its own change. Task 10.10's matching parenthetical ("as `f` already did") was corrected the
  same way. That change is now filed as `infer-unannotated-return-types`: the leak's root cause is
  that an unannotated return resolves to a placeholder and value bindings are inferred before every
  function body, so a value binding calling any local unannotated function is *always* falsely
  rejected — `let f() = 1` / `let b:int = {f()}` fails, with no empty list anywhere in it.
- **Verification:** Confirmed, and the fix went usefully further than the finding asked for. D3's
  Recovery paragraph now states the value binding's rule alone; the paragraph after it names the
  limit, attributes the leak to its own change, and no longer claims the function side is observably
  symmetric. Task 10.10's parenthetical is corrected to match. I re-derived the escalated claim
  rather than taking it on faith: `let f() = 1` with `let b:int = {f()}` reports
  `expects int, found T0` — no empty list anywhere in it — while the same pair with an annotated `f`
  is clean, and the same call from a markup attribute rather than a value binding is *also* clean,
  which is exactly the value-binding-versus-other-sites split the follow-up describes. Both of its
  stated orderings reproduce: `let g():int = {f()}` above `let f() = 1` fails, the reverse order is
  clean. Its cited root cause holds too — the placeholder is `fresh_var` at
  [infer.rs:2891](crates/nx-types/src/infer.rs#L2891), and `type_satisfies_expected` has no
  `Type::Variable` arm. `openspec validate infer-unannotated-return-types --strict` is valid.

## Verification pass — 2026-09-05 11:52

**Verified: 6** — RF1, RF2, RF3, RF4, RF5, RF6. **Reopened: 0.** **Still open: 1** — RF7 (unchanged;
the owner's call). **New: 2** — RF8, RF9, both Low.

Checks run for this pass: `cargo test --workspace` — **1447 passed, 0 failed**, matching the fix
pass's count. `cargo fmt --all -- --check` — 13 diffs across five files, byte-for-byte the same set
as on `0fc0463` (only shifted line numbers in `parser_tests.rs`), so the pass introduced none.
`openspec validate empty-list-spelling --strict` — valid. Full 111-file corpus re-run against a
freshly built `0fc0463` worktree — one differing file, 21 → 35, corpus 225 → 239. Direct behavioural
probes of both D3 recovery sites, the `{}`-at-`string[]`/`int[]` pair, and the suppressed-mismatch
case. Hand-generated TypeScript for the RF5 test's two sources to confirm it is not vacuous. A
repeated word-level `*.md` `void` sweep at the boundary task 8.10 states.


## Verification pass 2 — 2026-09-05 12:15

**Verified: 2** — RF8, RF9. **Reopened: 0.** **New: 0.** **Still open: 1** — RF7, unchanged.

Both fixes are prose-only and both are correct. The stale section-9 vocabulary that RF3 and RF8
tracked is now entirely gone from `crates/nx-types`. RF9's fix went further than the finding asked
for — rather than only softening D3's claim, it diagnosed the underlying leak and filed
`infer-unannotated-return-types` — and every load-bearing claim in that diagnosis was re-derived
here independently rather than accepted: the no-empty-list reproduction, the annotated control, the
markup-site control, both declaration orderings, and the two cited code sites.

Checks run for this pass: `cargo test --workspace` — **1447 passed, 0 failed**, unchanged, so
nothing behavioural moved. `cargo fmt --all -- --check` — the same 13 diffs across the same five
files at the same lines as the previous pass, so this pass introduced none.
`openspec validate --strict` — valid for both `empty-list-spelling` and the new
`infer-unannotated-return-types`.

One note against RF7, which stays open: the working tree now also carries the untracked
`openspec/changes/infer-unannotated-return-types/`. That directory is a legitimate product of this
review rather than stray work, but it is one more thing to separate when the `examples/nx/types.nx`
hunk and the other untracked files are sorted out before archiving.


## New Findings Discovered During 2026-09-05 12:40 Review Of The Open Questions

### ✅ Resolved - RF10 Freeing the name `void` makes `expects void, found void` reachable
- **Severity:** Low (corrected from Medium; see Resolution)
- **Evidence:** D5 removes `void` from the source surface but keeps `Type::Void` rendering as
  `void` in diagnostics ([ty.rs:62](crates/nx-types/src/ty.rs#L62)). The `primitive-type-names`
  delta separately guarantees that a user may now declare `type void = { … }` and have it resolve to
  their record. Those two decisions collide, and the collision is reachable in three lines:

  ```nx
  type void = { n:int }
  let c = true
  let a:void = {if { c => void(1) }}
  ```
  ```
  error: Initializer for value 'a' expects void, found void
  ```

  Both names are printed identically and neither is qualified — `display_type_pair` disambiguates
  two *nominal* types by declaring module, but here one side is `Type::Named("void")` and the other
  is `Type::Primitive(Primitive::Void)`, so it does not engage. The message is unreadable, and it is
  created by this change: before it, `void` in type position was the primitive, so the user type
  could not exist.

  This is the same defect class the change spent D10 and `empty_list_display` eliminating for `{}` —
  a diagnostic naming a type the author cannot write — reintroduced at the one internal type D5
  deliberately left rendering under a now-freed name.
- **Where `void` reaches a message at all:** `Type::Void` is constructed at exactly three sites:
  an `if`/`if … is` with no `else` ([infer.rs:317](crates/nx-types/src/infer.rs#L317)), a block
  expression with no trailing expression ([infer.rs:429](crates/nx-types/src/infer.rs#L429)), and
  the uncovered arms of a non-exhaustive union match
  ([infer.rs:741,744](crates/nx-types/src/infer.rs#L741-L744)). Only the first is observable as the
  word `void`: `let a:string = {if { c => "x" }}` reports `expects string, found void`. The match
  case never prints it, because `common_supertype` joins the arm's type with `void` and climbs to
  `object`, so the message reads `expects string[], found object` — which names neither the missing
  `else` nor the missing case, and is its own (pre-existing, lower-priority) legibility problem.
- **Recommendation:** Stop rendering the internal unit type under a name an author can now bind.
  Two options, in increasing cost and quality:
  1. **Rename the rendering** (small, contained). Change `Primitive::Void`'s `name()` from `"void"`
     to a form that is not a legal identifier, so no future declaration can collide with it the way
     `type void` now does. `no value` reads as prose in the message it lands in —
     `Initializer for value 'a' expects string, found no value` — and, because it contains a space,
     it can never be a declarable name. A bare word like `nothing` would only move the collision.
     This is a one-line change plus the two assertions at
     [ty.rs:760](crates/nx-types/src/ty.rs#L760) and
     [semantics.rs:252](crates/nx-types/src/semantics.rs#L252). Note it is *not* the same question as
     the code generator's `Primitive::Void => "void"` in
     [emit.rs:2720](crates/nx-codegen/src/emit.rs#L2720) and
     [ir.rs:1827](crates/nx-codegen/src/ir.rs#L1827), which emit a *host* language's `void` and
     should stay.
  2. **Name the construct instead of the type** (better, larger). The author did not write a type;
     they wrote an `if` with no `else`. `Initializer for value 'a' expects string, but this 'if' has
     no 'else' branch, so it produces no value` needs no type name at all, and points at the edit.
     This is the same move `empty_list_display` makes for `{}` and is the more consistent answer.

  Option 1 alone closes the collision and is worth doing inside this change, since this change is
  what opened it. Option 2 is follow-up material.
- **Resolution:** **Deferred to `type-conditionals-without-else`, and deliberately not fixed here.**
  That change types an `if` with no `else` by position — no items in a sequence, `T?` in a value
  position — which retires the last reachable producer of `Type::Void` and the collision with it.
  Investigating it surfaced two pre-existing defects that make the case stronger than RF10 alone: a
  false conditional child splices a **`null` item** into the content list in both the interpreter and
  generated TypeScript, and `let v:string? = {if { c => "x" }}` — the one annotation matching what
  the runtime actually produces — is *rejected*.

  **Severity corrected to Low.** The behaviour is right: the program is still rejected, and only the
  message is ambiguous. Reachability requires an author to declare `type void` — a name freed by this
  change, which no `.nx` source in the repository uses, and which nothing outside it can be using
  either, since until this change it did not parse as a declaration.

  **Why not take the stopgap anyway.** Cost is not the objection — it is verified small:
  `Primitive::as_str` ([ty.rs:62](crates/nx-types/src/ty.rs#L62)) feeds only `Display for Primitive`,
  nothing else in the workspace calls it, and the code generators keep their own separate mappings
  ([emit.rs:2720](crates/nx-codegen/src/emit.rs#L2720),
  [ir.rs:1827](crates/nx-codegen/src/ir.rs#L1827)), so it is one line plus two assertions with no
  codegen impact. The objection is churn in user-visible diagnostic text: renaming to `no value` now
  and deleting `Type::Void` shortly after would put three spellings of the same thing in front of
  authors inside one release window, to close a hole nothing can currently reach.

  **What makes deferring safe:** the follow-up's spec pins it as a scenario — *"A user-declared type
  named `void` is unambiguous in a message"* — so it cannot be silently dropped when that change is
  designed and implemented.

  **Revisit if** `type-conditionals-without-else` is descoped, abandoned, or still unstarted when
  anything first declares `type void`. In any of those cases, take Option 1 then; it stays a one-line
  change.


## Summary

The implementation matches the artifacts on every behaviour I could test, and the section-10 rewrite
is the right call — I independently confirmed the two claims it rests on (identical tree-sitter
conflict sets before and after; generated parser artifacts in sync with `grammar.js`), and the
`never`-based typing holds up across call arguments, content splicing, arm joins, `for` bodies,
nullable sites, `object` sites and state declarations. The formatter's round-trip and nested-list
guard, the interpreter's written-body-versus-absent-body rule, and the `void` removal across
grammar, semantics, completions, highlighting and both typegen backends are all correct and well
tested.

Two findings need action before archiving: **RF1**, a genuine requirement gap — three first-party
primitive inventories still advertise `void`, two of which a prior change had already been reopened
twice to fix — and **RF2**, a verification claim stated three times in the tasks and once in the
design that the corpus does not actually support. The rest (**RF3**–**RF7**) are documentation,
test-strength and hygiene issues concentrated where section 9 was superseded: comments and test
rationales still describe an inference variable and a migrating "demand" that no longer exist, one
test cannot fail, one claimed verification has no test, and one recovery asymmetry is undocumented.
None of them indicate a defect in the shipped behaviour.


## Fix pass — RF8, RF9

**Fixed: 2** — RF8, RF9. **Still open: 1** — RF7 (unchanged; the owner's call).

Both findings were correct as written and both are prose-only: two comments in `infer.rs`, D3's
Recovery paragraph, and task 10.10's parenthetical. No behaviour changed, and RF9's claim was
re-derived by direct probe rather than taken on faith — including the control (`let f() = {"a"}`)
that shows the `T0` leak is unrelated to this change.

The `T0`-at-a-call-site leak RF9 surfaces is recorded, not fixed: it reproduces on `0fc0463` for any
unannotated function, so it is outside this change. It is worth its own change.


## Status — 2026-09-05 13:05

**RF1–RF6, RF8, RF9: ✅ Verified.** **RF10: ✅ Resolved** (deferred to
`type-conditionals-without-else`, severity corrected to Low). **RF7: 🔴 Open** — the only item left,
and it is a working-tree hygiene call for the repository owner, not a code fix: `examples/nx/types.nx`
carries commit 0fc0463's content, and `docs/scratch-highlighting.nx`,
`drawnup-startup-fixes-review.md`, `examples/nx/template-candidate.nx`,
`openspec/changes/resolve-editor-positions/` and the two changes this review produced
(`infer-unannotated-return-types`, `type-conditionals-without-else`) are untracked and belong to
other work.

Nothing open blocks archiving on correctness grounds. Separating the `types.nx` hunk before
archiving is what makes the Migration Plan's "rollback is reverting it" true of a diff that is only
this change.

Two follow-up changes came out of this review and are filed and validating:
`infer-unannotated-return-types` (the `T0` leak, RF9) and `type-conditionals-without-else` (RF10 plus
the `null`-splice and rejected-nullable defects it surfaced). One item went to `specs/future.md`:
"Brace Recovery Reports A Closed Brace As Unclosed" (RF2's recovery regression).
