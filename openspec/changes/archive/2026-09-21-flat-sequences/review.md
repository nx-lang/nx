# Review: flat-sequences

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md` (32 tasks, all `- [x]`), `review-notes.md`, and all eight delta specs (`sequence-model`, `conditional-result-types`, `braced-value-sequences`, `type-reference-suffixes`, `record-type-parameters`, `component-type-parameters`, `cli-code-generation`, `primitive-type-names`).

**Reviewed code:** the whole uncommitted working tree for this change —
`crates/nx-syntax/src/validation.rs`, `crates/nx-syntax/tests/*`,
`crates/nx-hir/src/ast/{expr,types}.rs`, `crates/nx-hir/src/lower.rs`,
`crates/nx-types/src/{infer,ty,semantics,lib}.rs`, `crates/nx-types/tests/*` (incl. the new `flat_sequences.rs`),
`crates/nx-interpreter/src/interpreter.rs`, `crates/nx-interpreter/tests/arrays.rs`,
`crates/nx-codegen/src/{emit,tests}.rs`, `runtime/typescript/src/index.ts` + `dist` + `test/emitted-ir.test.mjs`,
`crates/nx-cli/src/{main,typegen}.rs`, `crates/nx-language-service/src/lib.rs`,
`nx-grammar.md`, `nx-grammar-spec.md`, `docs/src/content/docs/**`, `examples/nx/generic-records.nx`, `docs/drawnui-proposal/ui/ui.nx`.

**Verification actually run (not taken on trust):**
- `cargo test --workspace --locked` — pass.
- `npm test` in `runtime/typescript` — pass; `npm run build` leaves `dist` byte-identical, so the committed `dist` is in sync with `src`.
- `pnpm run check-examples` in `sites/playground` — pass.
- `npx astro build` in `docs` — pass (23 pages).
- `pnpm run test:grammar` in `src/vscode` — pass (262 tests).
- `openspec validate flat-sequences --strict` — valid.
- Every `nx` code block in the rewritten `sequences-and-objects.md` executed under `nxlang run` — all 8 succeed.
- Hand-built `.nx` probes run through the interpreter, `nxlang codegen --target javascript`, and the IR runtime (`prepareNxIrProgram` + `evaluateFunction`) to compare the engines directly.

**Re-run for Fix Pass 1 (2026-09-20), all independently, none taken on trust:** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (clean) and `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript` (including the new nested-conditional three-engine case) with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` in `sites/playground` (20/20); `npx astro build` in `docs` (23 pages) with all 8 `nx` blocks in `sequences-and-objects.md` still running; `pnpm run test:grammar` in `src/vscode` (262); `openspec validate flat-sequences --strict`. Plus a `HEAD` worktree built from scratch, used for the two-source corpus diff (39 files, byte-identical) and as the baseline for deciding which residual divergences predate the change. Roughly 40 hand-built `.nx` probes were run through all three engines by a small harness that compares interpreter, IR runtime and generated JavaScript output for one source.

**Re-run for Fix Pass 2 (2026-09-20), again all independently:** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (0 warnings) and `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript`, including the new empty-body case, with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` (20/20); `npx astro build` (23 pages) with all 8 `nx` blocks in `sequences-and-objects.md` still running; `pnpm run test:grammar` (262); `openspec validate flat-sequences --strict`; and the two-source corpus diff against a freshly rebuilt `HEAD` worktree (39 files each side, `diff -r` empty). Plus about 30 further probes aimed at the two pass-2 fixes: alias counts across declaration orders, cycles, type-parameter-scope targets and the quiet-resolution path for RF5, and every content-binding shape — no default, non-list content, no content field, union case, component, nesting, and the host `constructComponentDescriptor` entry point — for RF14.

**Re-run for Fix Pass 3 (2026-09-20):** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (0 warnings); `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript` (308 assertions) with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` (20/20); `npx astro build` (23 pages) plus all 8 `nx` blocks in `sequences-and-objects.md`; `pnpm run test:grammar` (262); `openspec validate flat-sequences --strict`; two-source corpus diff against a freshly rebuilt `HEAD` worktree (39 files each side, `diff -r` empty). New this pass: a **working multi-module harness** — `nxlang typegen <library directory>` loads and type-checks every module in a library, so the imported-alias paths that could not be reached before were measured directly rather than reasoned about.

**Re-run for Fix Pass 4 (2026-09-20):** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (0 warnings); `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript` (308) with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` (20/20); `npx astro build` (23 pages) plus all 8 `nx` blocks in `sequences-and-objects.md`; `pnpm run test:grammar` (262); `openspec validate flat-sequences --strict`; two-source corpus diff against a freshly rebuilt `HEAD` worktree (39 files each side, `diff -r` empty); `cargo test -p nx-cli a_type_alias_cycle_across_modules`. Plus the RF5 and RF16 suites re-run in full, since this pass touches the same two functions, and about twenty new probes on the `scoped` change, `seen` seeding, and two- and three-module cycles.

**Re-run for the rule change (2026-09-21):** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (0 warnings); `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript` (309) with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`; `openspec validate flat-sequences --strict`; the two-source corpus diff against a freshly rebuilt `HEAD` worktree (39 files each side, `diff -r` empty); and every `nx` block on `sequences-and-objects.md` (8/8) and on `if.md` (the 3 new ones run; the 6 pre-existing reference fragments do not, at `HEAD` either). Plus about 35 probes through all three engines: the collecting-position suite (nested, closed-over-open, empty arm, arity one, uncovered match, `for` filter, written null, widening) and the value-position cases — unannotated bindings, annotated `let`, declared returns, `for` over each — that produced RF18–RF20, each classified against a `HEAD` build and `HEAD`'s IR runtime.

**Re-run for Fix Pass 5 (2026-09-21):** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (0 warnings); `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript` (310) with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`; `openspec validate flat-sequences --strict`; the 8 blocks on `sequences-and-objects.md` and the 3 new ones on `if.md`; the two-source corpus diff against a freshly rebuilt `HEAD` (39 files, identical). Plus the RF18–RF20 repros re-run through all three engines, lift-plus-widening and null-branch probes, each classified against `HEAD`.

**Re-run for Fix Pass 6 (2026-09-21):** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (0 warnings); `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript` (311) with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`; `openspec validate flat-sequences --strict`; the 8 blocks on `sequences-and-objects.md` and the 3 new ones on `if.md`; the two-source corpus diff against a freshly rebuilt `HEAD` (39 files, identical). Plus about twenty probes attacking the null-literal exclusion through all three engines — typed nullables of every kind, a null through a variable, nested and `for`-body mixes — and inferred-type reveals for nullable-sequence items, classified against `HEAD`.

**Re-run for Fix Pass 7 (2026-09-21):** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (0 warnings); `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript` (311) with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`; `openspec validate flat-sequences --strict`; the 8 blocks on `sequences-and-objects.md` and the 3 new ones on `if.md`; the two-source corpus diff against a freshly rebuilt `HEAD` (39 files, identical). Plus a dozen probes through all three engines covering every contribution consumer, widening through a nullable contribution, already-nullable elements, the lone-brace case, and the runtime shape of a lone nullable-sequence child.

**Re-run for Fix Pass 8 (2026-09-21):** `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets` (0 warnings); `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript` (312) with `npm run build` leaving `dist` byte-identical; `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`; `openspec validate flat-sequences --strict`; the 8 blocks on `sequences-and-objects.md` and the 3 new ones on `if.md`; the two-source corpus diff against a freshly rebuilt `HEAD` (39 files, identical). Plus a nineteen-shape lone-child suite through all three engines, run on the working tree and on `HEAD`'s own CLI and TS runtime.

## Findings

### ✅ Verified - RF1 A nested `if` with no `else` still contributes a `null` item in the interpreter, while the IR runtime and generated JavaScript drop it
- **Severity:** High
- **Evidence:** `crates/nx-interpreter/src/interpreter.rs:3471` — `eval_item_into` tests only the *outer* item with `contributes_no_items` and then calls `eval_expr` on the whole item. When the outer conditional *is* taken and its branch is itself an else-less conditional that is not, `eval_if` returns `Value::Null` and that null is pushed as an item. Both other engines recurse through the branch instead: `runtime/typescript/src/index.ts:2176` (`evalItemInto` calls itself on `takenBranch`) and `crates/nx-codegen/src/emit.rs:3557` (`emit_item` recurses into `then_branch`). Reproduced with

  ```nx
  type A = { n:int = 1 }
  type Box = { content items:A?[] }
  let c = true
  let d = false
  let root(): Box = { <Box><A/>{if c { if d { <A/> } }}</Box> }
  ```

  - interpreter (`nxlang run --format json`): `{"$type":"Box","items":[{"$type":"A","n":1},null]}`
  - IR runtime: `{"$type":"Box","items":[{"$type":"A","n":1}]}`
  - generated JS (`m0_*.js`): `[A, (c ? (d ? A : []) : [])].flat()` → one item

  This violates `conditional-result-types` ("It SHALL NOT contribute a null item") and `sequence-model` ("The interpreter, the TypeScript IR runtime and every code generation target SHALL produce the same value for the same source"). It is the headline defect the change exists to remove, reappearing one level down.
- **Recommendation:** In `eval_item_into`, when the item is an `If`/`Match` with no `else`, resolve the taken branch (the existing `contributes_no_items` machinery already computes it) and recurse into `eval_item_into` on that branch rather than calling `eval_expr` on the whole item — matching `evalItemInto` in `index.ts`. Add an interpreter test and a three-engine case in `emitted-ir.test.mjs` for the nested form.
- **Fix:** `eval_item_into` (`crates/nx-interpreter/src/interpreter.rs`) now resolves the taken branch itself and recurses into `eval_item_into` on it, for `If` and `Match` alike, instead of testing the outer item and then calling `eval_expr` on the whole thing — the shape `evalItemInto` in `index.ts` already had. `contributes_no_items` is gone; a `Widen` item is handled by widening each contributed value, which `widened_value` already does element by element. The same gap was present in generated JavaScript one case over: `emit_item` recursed into a missing `else` but emitted a closed conditional whole, so `if c { if d { A } } else { B }` still emitted `null`; it now recurses into both branches. New tests: `a_conditional_nested_in_a_conditional_contributes_no_items`, `an_else_on_the_outer_conditional_does_not_resurrect_the_null` and `a_nested_conditional_that_is_taken_contributes_its_item` in `crates/nx-interpreter/tests/arrays.rs`, and a three-engine case in `emitted-ir.test.mjs` covering open-in-open, closed-over-open and both-taken.

- **Verification:** confirmed. `eval_item_into` resolves the taken branch and recurses, `contributes_no_items` is gone, and the condition/scrutinee/patterns are evaluated once. Re-probed all three engines from scratch on open-in-open (`c` true, `d` false), closed-over-open with the outer `else` taken, and both-taken: interpreter, IR runtime and generated JavaScript now agree item for item, with no `null` and no nested list. The extra step beyond the recommendation — `emit_item` recursing into *both* branches of a closed conditional — is right and introduces no new divergence: `if c { if d { A } } else { B }` emits `(c ? (d ? A : []) : B)` and all three engines produce one item in each of the three condition combinations. The arity-one content guard moving from `is_open_conditional` to `contributes_conditionally` was probed separately (closed conditional with plain branches, with sequence branches, open conditional with a sequence branch taken and not taken): the only arity-one content divergences left are pre-existing ones unrelated to conditionals, recorded as RF15.
- **Superseded mechanism (rule change):** the fix recorded above — `eval_item_into` and `emit_item` recursing into the taken branch — is deleted. The finding stays closed by a stronger means: a missing `else` now evaluates to the empty sequence in all four engines, so no engine can produce a null item from an untaken conditional at any depth, and `eval_item_into`/`evalItemInto` are back to the one splice rule with no conditional case. Re-verify against the current code rather than the note above. The nested-conditional tests and the three-engine case still pass unchanged.

- **Re-verification (rule change, 2026-09-21):** confirmed against the current code. `eval_if` and `eval_match` now return `Value::Array(vec![])` for a missing `else` or an uncovered path, `nodeKinds.if`/`ifIs` return `[]`, and the emitter writes `(c ? … : [])`; `eval_item_into` and `evalItemInto` are the bare splice with no conditional case. Re-probed through all three engines: open-in-open (outer taken, inner not) at `A?[]` and at `A[]`, closed-over-open with the outer `else` taken — no `null` item anywhere and all three agree. The finding stays closed.

### ✅ Verified - RF2 The checker widens the element type for a nested else-less conditional, so the flat-sequence form is rejected
- **Severity:** Medium
- **Evidence:** `crates/nx-types/src/infer.rs:585` records `open_conditional_joins[if] = then_ty`, the branch's *inferred* type, not the branch's *contribution*. When the branch is itself an else-less conditional, `then_ty` is already the nullable value-position type, so the outer conditional contributes `A?`. Observed:

  ```
  $ nxlang run n1.nx
  error n1.nx:5:16: Content for 'Box' binds to 'items' expects A[], found A?[]
        let root() = { <Box><A/>{if c { if d { <A/> } }}</Box> }
  ```

  `sequence-model` states a conditional with no `else` "SHALL NOT widen that type"; `conditional-result-types` lists "the branches of an `if`" as a collecting position. Same root cause as RF1, on the checker side.
- **Recommendation:** Record the branch's *contribution* — `self.item_contribution_of(then_branch)` / the join of each arm's contribution — in `open_conditional_joins`, so a nested open conditional contributes `A` rather than `A?`. Cover the nested form in `crates/nx-types/tests/flat_sequences.rs`.
- **Fix:** `open_conditional_joins` is now `open_conditional_contributions` and records the branch's *contribution* (`item_contribution_of(then_branch)`) rather than its inferred type, so `item_contribution_of` returns it as the item type it already is instead of stripping a sequence layer from it. The match arm joins arm contributions the same way. Tests: `a_conditional_nested_in_a_conditional_contributes_an_item_not_a_nullable_one` (two and three levels, plus the match form) and `a_nested_conditional_is_still_nullable_where_one_value_is_expected` in `crates/nx-types/tests/flat_sequences.rs`.

- **Verification:** confirmed. `{<A/>{if c { if d { <A/> } }}}` at a content property declared `A[]` now type-checks at two and three levels, and the `if … is` form with an uncovered path does too; the value position is unchanged (`let v = { if c { if d { 1 } } }` is `int?`, and at `int` the message still names `int?`, never `int??` or `void`). The whole-body form still binds the empty list. The match path joins arm *contributions*, and `uncovered` can only be true when there is no `else`, so every member joined there is an arm body as the comment claims.
- **Superseded mechanism (rule change):** `open_conditional_contributions` no longer exists. A conditional nested in a conditional is `T[]` because the inner one's type is already a sequence and the join does not nest it, not because a contribution is recorded. Re-verify against the current code; `a_nested_conditional_is_a_sequence_of_items_not_a_sequence_of_sequences` now asserts `int[]` at two and three levels.

- **Re-verification (rule change, 2026-09-21):** confirmed. `{<A/>{if c { if d { <A/> } }}}` at a content property declared `A[]` type-checks: the inner conditional is `join(A, never[]) = A[]`, the outer is `join(A[], never[]) = A[]` through the `(Array, Array)` arm, and a collecting site takes its element type `A`. No `?` is introduced at any depth.

### ✅ Verified - RF3 `docs/drawnui-proposal/ui/ui.nx` carries unrelated, apparently accidental edits that break the file further
- **Severity:** Medium
- **Evidence:** `git diff docs/drawnui-proposal/ui/ui.nx` replaces a valid declaration with commented-out scratch and invents a property modifier:

  ```
  -  content children: Element[]?
  +  //content children: Element
  +  //content children: {Element]        <- ui.nx:90, malformed
  ...
  -  columns: TrackSize[]
  +  columns: TrackSize list              <- ui.nx:98
  ```

  Neither edit is called for by any task (no task touches this file; 10.1 only asks for a *sweep*). It is a regression: at `HEAD` the file produces no diagnostic about line 98, and with the edit `nxlang run docs/drawnui-proposal/ui/ui.nx` reports a new first error, `Unsupported property modifier 'list'; only 'content' is allowed here`. `Element[]?` was already legal under the new rule (one `[]`, one `?`), so there was nothing here to fix.
- **Recommendation:** Revert `docs/drawnui-proposal/ui/ui.nx` to its `HEAD` content.
- **Fix:** reverted `docs/drawnui-proposal/ui/ui.nx` to its `HEAD` content. The pre-revert text is kept at `ui.nx.scratch-before-revert` in this session's scratchpad in case any of it was wanted.

- **Verification:** confirmed. `docs/drawnui-proposal/ui/ui.nx` is absent from `git status`, so it matches `HEAD` exactly; running it under the working-tree build reproduces `HEAD`'s diagnostics with no `Unsupported property modifier 'list'`.

### ✅ Verified - RF4 `review-notes.md` records claims that do not hold, including one created by the change's own edit
- **Severity:** Medium
- **Evidence:**
  1. "`docs/drawnui-proposal/{ui/ui.nx,graphics/graphics.nx}` are proposal sketches using syntax NX does not have (a `list` property modifier)" — the `list` modifier is not pre-existing; this change introduced it (RF3). The notes cite as pre-existing breakage a failure the change caused.
  2. "Every `if`, `if … is` and condition list in the corpus has an `else` arm" — false. `examples/nx/types.nx:80` is an `if loadState is { idle … loading … failed … loaded … }` with no `else` arm. (It is exhaustive, so behaviour is unchanged, but the sweep's stated basis is wrong.)
  3. "The captured stdout/stderr is **byte-identical for every file**" — the comparison was old toolchain vs new toolchain over the *same* (already-edited) sources. That design cannot detect a corpus *source* edit that broke a file, which is precisely what happened in RF3. The diff that would have caught it is `HEAD` sources under `HEAD` vs working-tree sources under the working tree.
- **Recommendation:** Re-run the sweep after reverting RF3, correct the `else`-arm claim (say "every conditional child" rather than "every `if`"), and state the corpus-diff methodology (same sources, two toolchains) plus a second pass comparing source versions.
- **Fix:** `review-notes.md` rewritten on all three points. The proposal sketches' actual failure is named (a missing `docs/drawnui-proposal/core` import, plus a union default written as a bare name in `ui.nx`) instead of a `list` modifier this change had introduced; the `else`-arm claim now says no conditional *child* is missing an `else` and names `examples/nx/types.nx:80` as an exhaustive value-position match that has none; and the corpus diff was re-run with the methodology the finding asks for — `HEAD` sources under a detached-worktree `HEAD` build against working-tree sources under this build — which is byte-identical for all 39 files, with the note now stating that design and why it differs from the earlier same-sources pass.

- **Verification:** confirmed, including the corpus diff, which I reproduced rather than read. I built `HEAD` in a detached worktree and ran each toolchain over **its own** `.nx` sources — 39 files each, `nxlang run <file> --format json`, stdout and stderr and exit code captured per file — and `diff -r` over the two captures is empty. I also re-checked the two prose corrections: `HEAD`'s `ui.nx` does fail on the missing `docs/drawnui-proposal/core` import and on `width: Length = auto` as claimed, and `examples/nx/types.nx:80` is the exhaustive no-`else` match the note now names.

### ✅ Verified - RF5 A nested sequence type is reported two or more times, and the extra reports point at the wrong span
- **Severity:** Medium
- **Evidence:** The new `validate_local_type_aliases` (`crates/nx-types/src/infer.rs:5558`) resolves every local alias eagerly, and the resolver at `infer.rs:3798` reports again at each use site. Measured:
  - `type Matrix = string[][]` → **2** errors: the validator's (`nx-syntax`, pointing at the second `[]`) and the resolver's (pointing at the whole alias). The change's own task 1.1 says "each report one error on the second `[]`".
  - ```nx
    type Names = string[]
    type Rows = Names[]
    let a:Rows = { "x" }
    let b:Rows = { "y" }
    ```
    → **3** errors: one at `let a` (3:1), one at `let b` (4:1), one at the alias (2:1). The two use-site diagnostics say "'Names' is already a sequence" while pointing at a line that mentions neither `Names` nor a `[]`, so the count grows with uses and the spans mislead.
  - Separately, `validate_local_type_aliases`'s doc comment claims it reports "a target that is not a type … where the alias was written", but `type Bad = Nonexistent` with a use still produces no diagnostic naming `Bad`, so only the nesting half of its stated purpose is met.
- **Recommendation:** Suppress the resolver's diagnostic for an alias whose declaration already reported (e.g. cache the alias's resolved `Type::Error` after the first report, or report only in `validate_local_type_aliases` and return `Type::Error` silently thereafter), and skip the alias pass for a target whose `[]` the parse validator already flagged. Add a test asserting the error count for `string[][]` and for an alias used twice.
- **Fix:** three changes. The resolver no longer repeats what post-parse validation already said: `report_nested_sequence_type` returns without reporting when the inner reference spells a `[]` of its own (through any number of `?`), so `string[][]` is one error, on the `[]`, from the validator. A `reported_nested_sequence_names` set makes the through-a-name form report once however many times the alias is resolved, and `type_from_type_ref_in_quietly` rolls that set back with the diagnostics it suppresses so a quiet resolution cannot swallow the only report. And `validate_local_type_aliases` now runs before the signature, binding and field passes, so the report lands at the alias rather than at whichever declaration reached it first. Measured after: `string[][]` → 1, `type Rows = Names[]` used twice → 1 at the alias, a field written `Names[]` → 1 at the field, an unused alias → 1 at its declaration. Test: `nesting_a_sequence_through_a_name_is_reported_once_at_the_name_that_nests_it`. The finding's third point — that the doc comment overstates what the alias pass reports — is covered by rewriting it to the one thing it does.

- **Verification:** reopened — the fix is right for the three cases the finding measured but the dedupe is keyed per message rather than per alias, so the same defect survives in three other spellings. Confirmed fixed: `type Matrix = string[][]` → 1 error (on the `[]`, from the validator), `type Rows = Names[]` used twice → 1 at the alias, `type Holder = { rows:Names[] }` → 1 at the field with a precise span, an unused alias → 1 at its declaration, and `let f(rows:Names[])` → 1 at the parameter. `type_from_type_ref_in_quietly`'s rollback is watertight: it clones the set *before* the quiet resolution and restores that clone, so it removes only what the quiet pass added and cannot swallow a report made earlier. Still broken, each measured through `nxlang run`:

  - `type Names = string[]` + `type X = Names?[]` with two uses → **3** errors (2:1, 3:1, 4:1). The inner reference is `Nullable(Name)`, which falls to the `_ =>` arm of `report_nested_sequence_type`, so it gets neither the alias's name nor the `reported_nested_sequence_names` dedupe.
  - `type Box = { T:type value:T }` + `type Bad = <Box T=int[]/>` with two uses → **3** errors. `reject_sequence_type_argument` has no dedupe at all.
  - `type Bare = Box` (a generic record named without its arguments) with one use → **2** errors, where the `HEAD` build reports **1**. This one is a regression the eager alias pass introduced, and it has nothing to do with sequences: the pass duplicates *any* diagnostic an alias's target produces, at the alias and again at each use.

  So the shape of the fix is the issue, not its correctness. Deduping per alias — resolve each local alias once, cache the resulting `Type` (including `Type::Error`), and have use sites read the cache instead of re-walking the target — would cover all four messages at once and would also undo the `type Bare = Box` regression, where three separate per-message sets would not.
- **Fix (pass 2):** replaced the per-message dedupe with the per-alias resolution cache the verification recommends. `resolved_type_aliases` caches what each local alias's target resolved to, `Type::Error` included, so the target is walked once however many times the alias is used; `validate_local_type_aliases` now writes its own resolution into that cache, so the eager pass and the first use are one resolution rather than two; and `type_from_type_ref_in_quietly` rolls the cache back with the diagnostics it discards, so a quiet resolution cannot leave an answer behind whose report was thrown away. This closes all four messages, not just the one the old dedupe knew how to key: `type X = Names?[]` used twice → 1, `type Bad = <Box T=int[]/>` used twice → 1, and `type Bare = Box` → 1, matching HEAD and undoing the regression the eager pass had introduced. Two declarations that each write the mistake still report twice, at their own spans, which is two mistakes rather than one repeated. The test in `flat_sequences.rs` was extended to all five counts.

- **Verification (pass 2):** reopened again, for a narrow residue. Everything the fix claims reproduces: `Rows` used twice → 1 at the alias, `Names?[]` used twice → 1, `<Box T=int[]/>` alias used twice → 1, `type Bare = Box` → 1 (against **1 at `HEAD`**, at the use site rather than the alias — the count regression is genuinely gone and the span moved on purpose), `string[][]` → 1, and two records each writing `x:Names[]` → 2 at their own spans. **I agree with that last judgement**: `Names[]` was written twice, so two reports at two spans is right, and the same holds for two use sites each writing `TItem=Names`, which I measured as 2. The `type_from_type_ref_in_quietly` rollback is correct for the cache as well as the diagnostics — it clones before and restores after, so it removes only what the quiet pass added — and it is additionally unreachable for a local alias: `register_type_definitions` makes no resolver call, `validate_local_type_aliases` runs next and both reports and caches every local alias, and all ten `type_from_type_ref_in_quietly` call sites are field/return-type lookups that run after it. So the worse failure mode you named cannot occur. An alias's target is resolved with `scoped = false`, so a target that could only resolve through a component's type-parameter scope resolves to nothing whoever asks — the cache cannot freeze a context-dependent answer — and `type Alias = <Range2 T=TValue/>` is now reported at the alias where `HEAD` said nothing at all.

  **But one alias is still resolved twice**, which is the question you asked me to check. `validate_local_type_aliases` resolves each alias's *target* directly through `type_from_type_ref_at`, which never consults `resolved_type_aliases`. So an alias the loop already resolved as a side effect of an earlier alias gets its target walked a second time, and the count depends on declaration order:

  ```nx
  type Names = string[]
  type Alias2 = Rows        // declared before Rows
  type Rows = Names[]
  ```
  → **2** errors, at 2:1 and 3:1, both "'Names' is already a sequence". The same three declarations in dependency order (`Rows` before `Alias2`) → **1**. `HEAD` reports 0 for both, since nothing uses them. The same doubling happens for the type-argument message (`type Alias2 = Bad` before `type Bad = <Box T=int[]/>` → 2) and through a three-alias chain, and the first of the two reports points at `Alias2`, a line that names neither `Names` nor a `[]` — verbatim the defect this finding is about. The new test pins all five counts but not declaration order, so it passes.
- **Recommendation (pass 2):** two small changes. In `validate_local_type_aliases`, skip an alias already present in `resolved_type_aliases` — that alone makes the count 1 and order-independent. Then, in `resolve_named_type`, set `type_ref_span` to `alias.span` (already in hand there for the cycle error) while walking that alias's target, so the one report lands on the alias that wrote the mistake rather than on whichever alias reached it first. Add the reordered form of the existing test as a case.
- **Fix (pass 3):** both changes the verification asks for, plus a third the second of them needed. `validate_local_type_aliases` skips an alias already in `resolved_type_aliases`, so an alias reached while resolving an earlier one is not walked a second time; `resolve_named_type` puts the alias's own span in `type_ref_span` for the length of the target walk, so the one report lands on the alias that wrote the mistake rather than on whichever reference reached it first. The third: that span override is gated on `local_type_alias_spans`, recorded from this module's own items, because `type_aliases` also holds **imported** aliases — under the name this module reaches them by, but with the span the *declaring* module wrote them at. Moving a diagnostic onto one of those would point it at an offset in a different file. Measured: `Alias2` before `Rows` → 1, at `Rows`; the same three in dependency order → 1, at `Rows`; a three-alias chain → 1, at the alias that writes the `[]`; `Alias2 = Bad` before `Bad` → 1. Tests: `where_a_nested_sequence_is_reported_does_not_depend_on_declaration_order` pins the count and the line for both orders, and `a_reported_alias_span_is_always_one_this_module_wrote` pins that every reported span falls inside the source under check.

- **Verification (pass 3):** confirmed, and the gate holds under attack. Locally, everything is now order-independent and correctly attributed: `type Alias2 = Rows` declared **before** `type Rows = Names[]` reports **1** error at `Rows`'s own line (3:1), the same three declarations in dependency order report **1** at `Rows` (2:1), a three-alias chain reports **1** at `Rows`, and `type Alias2 = Bad` before `type Bad = <Box T=int[]/>` reports **1** at `Bad` (3:1) where pass 2 gave 2. The counts that were already right stay right: `string[][]` → 1 from the validator, two records each writing `x:Names[]` → 2 at their own spans, an alias plus a separately written field mistake → 2.

  **I found a route into the imported path you could not**, and used it: `nxlang typegen <directory> --language typescript` loads a *library directory* and type-checks every module in it, reporting across files. (`import "./x"` resolves `x` as a **directory**, not a file — that is why the sibling-file layout failed. `examples/nx/core/` is the shape.) With that, all four of your attacks pass:
  - *Imported alias with a bad target, reached from a local alias.* Two modules in one library, `deep.nx` with `type Rows = Names[]` at line 32 and a **one-line** `shallow.nx` with `export type Local = Rows`. Reports: one at `deep.nx:32:1` and one at `shallow.nx:1:1`. The consumer's report lands on its own single line, not on line 32 of another file — the gate refuses the foreign span exactly as designed, and a 1-line file makes that unambiguous.
  - *Imported alias reached from record fields rather than an alias.* Two fields each written `r:Rows` in one module → **1** report in that module, at the first field's span (`use.nx:1:24`), plus the one in the declaring module. The cache dedupes within a module; the second field adds nothing.
  - *Local alias shadowing an imported one of the same name.* A local `type Shadowed = string` beside an imported bad `Shadowed` → one report, in the declaring module only. No misattribution onto the local span.
  - *Imported bad alias never referenced.* Reported once, in its own module.
- **Verification note:** `a_reported_alias_span_is_always_one_this_module_wrote` can only exercise local aliases, because `check_str` checks one module; the imported half of what its comment describes is covered by the library-directory probe above rather than by the test. Worth adding a multi-module case if a harness for one lands.

### ✅ Verified - RF6 A spliced sequence item is not widened to the joined numeric item type
- **Severity:** Medium
- **Evidence:** `crates/nx-types/src/infer.rs` `Expr::Array` inference joins *contributions* but passes each item's **own** type to `record_join_widenings` (`infer.rs:1495`), so `join_widening(int[], float64)` is `None` and no widening is recorded for the elements inside the spliced sequence:

  ```nx
  let ns:int[] = {1 2}
  let all = { ns 1.5 }     // infers float64[]
  let root() = { all }     // evaluates to: 1  2  1.5   (Int, Int, Float)
  ```
  versus the unspliced form `let root() = { 1 1.5 }` → `1.0  1.5`. The value's runtime representation disagrees with its static type; JSON output shows `[1, 2, 1.5]`. At an annotated site the later coercion repairs it (`<Box xs={all}/>` at `float64[]` yields `1.0 2.0 1.5`), so the divergence survives only where nothing re-coerces — exactly the unannotated inference this change newly made reachable (it previously inferred `object[]` with a nested array).
- **Recommendation:** Record widenings against the contribution: for a sequence-typed item whose element type widens to the joined item type, mark the item so the runtime widens elementwise (or record the widening on the elements). Add a test pinning `{ns 1.5}` to `float64` values.
- **Fix:** `Expr::Array` inference now measures each item against the joined type by its *contribution* rather than by its own type, so `join_widening(int, float64)` fires for a spliced `int[]` where `join_widening(int[], float64)` did not. The widening is still recorded against the item expression, because that is what a widening wraps, and `widened_value` already widens a sequence element by element. `{ns 1.5}` now evaluates to `1.0 2.0 1.5`. Test: `a_spliced_sequence_widens_to_the_joined_item_type` in `crates/nx-interpreter/tests/arrays.rs`.

- **Verification:** confirmed, and the runtime representation is what the finding asked for. `let ns:int[] = {1 2}` with `let all = { ns 1.5 }` now prints `1.0 2.0 1.5` where it printed `1 2 1.5`. Nothing that widened before stopped: `{1 1.5}` is still `1.0 1.5`, a `for` body item widens (`{for n in ns { n } 1.5}` → `1.0 2.0 1.5`), a conditional item widens (`{1.5 if c { 1 }}` → `1.5 1.0`), a pure-`int` `for` stays `int`, and a `string[]` item joined with a float still widens to nothing. All three engines agree on each.

### ✅ Verified - RF7 A one-item braced value holding an else-less conditional is a value position, contradicting the new `if` documentation
- **Severity:** Medium
- **Evidence:** `docs/src/content/docs/reference/syntax/if.md` (new "When there is no `else`" section) states the collecting positions as "an element body, a content property, **a braced sequence**, the body of a `for`". But at arity one the braced value is scalar, so no `Expr::Array` exists to route through `item_contribution`, and only the content path is special-cased (`infer.rs` `check_content_property`, the `open_conditional_joins.contains_key(&content[0])` branch). Result:

  ```
  let xs:int[] = { if c { 1 } }             -> error: expects int[], found int?
  <Box items={if c { 1 }} />  (items:int[]) -> error: expects int[], found int?
  <Box>{if c { <A/> }}</Box>  (content A[]) -> accepted, binds the empty list
  ```

  The same syntactic form therefore works as element body content and fails at a list-typed property or `let`. The two delta specs disagree too: `sequence-model` says "a **multi-item** braced value", `conditional-result-types` says "a braced value list".
- **Recommendation:** Pick one. Either extend the arity-one handling to any list-typed site (mirroring the content special case at the binding/property check) and keep the docs as written, or narrow the docs and `conditional-result-types` to "a braced sequence of more than one item" and add a scenario that pins the arity-one behaviour explicitly.
- **Status:** left open — this is the design question the finding names, not a defect with one right answer. At arity one the braces are grouping, not sequence construction (`let x:int = { 1 }` is an `int`), so reading `{ if c { 1 } }` as `int?` is as defensible as reading it as a collecting position; but arity two already collects, so the asymmetry is real. Extending the content special case to every list-typed binding, property, argument and return site is the wider of the two fixes and easy to leave incomplete. Needs a decision on which reading the specs and the docs should state, then one pass to make `sequence-model`, `conditional-result-types` and `if.md` agree.

- **Verification:** reasoning reviewed and sound; left open as a decision, not a fix. Re-measured: `let xs:int[] = { if c { 1 } }` and `<Box items={if c { 1 }} />` at `items:int[]` are still rejected with `found int?`, while `<Box>{if c { <A/> }}</Box>` at a content property declared `A[]` is accepted and binds the empty list, and arity two (`{ 7 if c { 9 } }`) collects. The asymmetry is exactly as described, and `sequence-model` ("a multi-item braced value") and `conditional-result-types` / `if.md` ("a braced value list", "a braced sequence") still disagree about which reading is the rule.
- **Fix (rule change):** dissolved rather than patched. Decided with the author: an `if` or match with no `else` is read as having an implicit `else { }`, so its type is the join of its branches with `never[]` — a sequence — and it evaluates to the empty sequence when no branch is taken. With the conditional already a sequence on its own account, the braces around it make no difference: `let xs:int[] = { if c { 1 } }`, `<Box items={if c { 1 }} />` and `{ if c { 1 } 2 }` all type-check by the one splice rule, and the content-only special case in `check_content_binding` is deleted rather than generalised. `sequence-model` now says a braced value (not a *multi-item* one) and that arity makes no difference; `conditional-result-types` and `if.md` agree. Test: `how_many_items_sit_beside_a_conditional_makes_no_difference`, and the `alone`/`beside` fields of the new three-engine case.

- **Verification (rule change, 2026-09-21):** confirmed for what the finding complained about. `let xs:int[] = { if c { 1 } }` and `<Box items={if c { 1 }} />` at `items:int[]` are now accepted and bind `[]` when `c` is false, so a one-item braced value and element body content no longer disagree. The spec disagreement is gone too: `sequence-model` no longer says "multi-item", and it no longer matters, because the conditional's *type* is a sequence in every position. One caveat that belongs elsewhere: when the branch **is** taken, the accepted program's value differs by engine — `let root(): int[] = { if c { 1 } }` returns `[1]` in the interpreter and a bare `1` in the IR runtime and generated JavaScript. That is not this finding; it is tracked as RF18 (the rule types a bare value as a sequence) and RF20/RF15 (the IR and generated JS do not lift a single item at an annotated `let` or return).

### ✅ Verified - RF8 A conditional *with* an `else` does not contribute per branch, so an empty arm still reports `object[]`
- **Severity:** Low
- **Evidence:** `sequence-model` lists "the branches of an `if` or the arms of a match in a sequence position" among collecting positions, but only the else-*less* form is routed through the contribution rule; a closed conditional is joined whole.

  ```nx
  type A = { n:int = 1 }
  type Box = { content items:A[] }
  let root() = { <Box><A/>{if c { <A n=2 /> } else { }}</Box> }
  // error: Content for 'Box' binds to 'items' expects A[], found object[]
  ```

  `common_supertype(Named(A), Array(Never))` falls to `object`. This is pre-existing behaviour, but this change is where the rule that covers it was written, and `braced-value-sequences` already promises "An empty arm takes its element type from the arm it is joined with".
- **Recommendation:** Route a closed conditional/match item in a collecting position through the per-branch contribution join as well, so the empty arm contributes nothing and the item type is `A`. Add the scenario to `flat_sequences.rs`.
- **Status:** the runtime half is fixed as a side effect of RF1 — all three engines now take a closed conditional's branch and apply the contribution rule to it, so an empty arm contributes nothing at run time. The checker half is left open. It is sound as it stands: the contribution of a closed conditional is `strip_array(join(branches))`, which is never narrower than the join of the branches' contributions, so the checker only ever rejects where the runtime would have coped — it never accepts something the runtime mishandles. Making it exact means recording contributions for closed conditionals too, which is the same machinery RF7's answer would decide the shape of, so the two are best settled together.

- **Verification:** the runtime half is confirmed fixed and the checker half confirmed still open, as the status note says. `{<A/>{if c { <A n=2 /> } else { }}}` at a content property declared `object[]` now yields one item in the interpreter, the IR runtime and generated JavaScript alike — the empty arm emits `[]` and is spliced away — while the same body at `A[]` is still rejected with `found object[]`. The soundness argument holds in the one new place I found it: at arity one, `<Box>{if c { if d { <A/> } } else { <A n=3 /> }}</Box>` at `A[]` is rejected with `found A?` in all three engines rather than mis-evaluated, because a *closed* outer conditional is not recorded in `open_conditional_contributions` and so does not reach the arity-one content special case. That is the same gap, and settling it with RF7 as the note proposes still looks right.
- **Fix (rule change):** the checker half is fixed by a new join rule, and the rule change makes it the same mechanism as a missing `else`. The join now lifts an item to a sequence — `join(T, U[])` is `join(T, U)[]` — when exactly one side is sequence-shaped (looking through `?`), because an item is a sequence of one. `{<A/>{if c { <A n=2 /> } else { }}}` at `A[]` therefore joins `A` with `never[]` to `A[]` and is accepted, and an empty arm written out and a missing one now infer the identical type, where before the written form joined to `object` and the missing one was nullable. The guard on exactly one sequence-shaped side is what keeps `string[]?` beside `string[]` from being lifted into `string[]?[]`. Tests: `a_missing_else_joins_exactly_as_an_empty_arm_would` and `lifting_never_builds_a_sequence_of_sequences`, and the `emptyArm`/`missingArm` fields of the three-engine case.

- **Verification (rule change, 2026-09-21):** confirmed on both halves. `{<A/>{if c { <A n=2 /> } else { }}}` at a content property declared `A[]` now type-checks — `join(A, never[])` is `A[]` by the new lifting arm rather than `object` — and binds one item in the interpreter, the IR runtime and generated JavaScript alike. The checker half that was left open by design is closed by the same join.

### ✅ Verified - RF9 `emit_item` handles only `If`, not `Match`, with nothing recording why that is safe
- **Severity:** Low
- **Evidence:** `crates/nx-codegen/src/emit.rs:3557` and `:3577` match `CodegenExpressionKind::If { else_branch: None, .. }` only. `CodegenExpressionKind::Match` (`model.rs:317`) also carries `else_branch: Option<..>`, and the interpreter and IR runtime both handle the match form. It is currently unreachable only because source codegen rejects every `Match` outright (`emit.rs:226`, "match expressions are not supported by executable source codegen yet") — verified: a non-union `if s is { "a" => … }` content item drops correctly in the interpreter and refuses in codegen. Nothing at `emit_item` records that coupling, so match support would silently start emitting `null` items.
- **Recommendation:** Add a comment at `emit_item`/`is_open_conditional` naming the `Match` rejection as the reason, or handle `Match { else_branch: None }` now so the two arrive together.
- **Fix:** took the first option. `emit_item` carries a `<para>` naming the `Match` arm of `collect_expression_source_codegen_diagnostics` as the reason no match arm is needed here, and saying that match support has to add one at the same time or a match item will start emitting the `null` the function exists to avoid. Emitting a match item properly is not possible today — the target refuses every match before emission — so handling it now would be dead code with no way to test it.

- **Verification:** confirmed. `emit_item` carries the `<para>` naming the `Match` arm of `collect_expression_source_codegen_diagnostics` and the obligation to add an arm with match support. Re-checked that the coupling still holds: a non-union `if s is { "a" => <A/> }` as a content item drops correctly in the interpreter and the IR runtime, and source codegen refuses it with `match expressions are not supported by executable source codegen yet` before emission.
- **Superseded mechanism (rule change):** `emit_item` is deleted, taking the comment with it. The coupling it recorded now lives at the `Match` arm of `emit_expression`, which says that match codegen must emit an uncovered path as `[]` exactly as the `If` arm emits a missing `else`. Re-verify that note is present and accurate.

- **Re-verification (rule change, 2026-09-21):** confirmed. `emit_item` and `contributes_conditionally` are gone; the coupling note now sits on the `Match` arm of `emit_expression` and says match codegen must emit an uncovered path as `[]` exactly as the `If` arm emits a missing `else`. The arm it depends on still holds: `collect_expression_source_codegen_diagnostics` refuses every `Match` before emission, and a non-union `if s is { "a" => … }` content item is refused by codegen while the interpreter and IR runtime drop its uncovered path.

### ✅ Verified - RF10 Generated JavaScript appends `.flat()` to every sequence, including constant scalar literals
- **Severity:** Low
- **Evidence:** `emit_sequence_items` (`emit.rs:3541`) is unconditional, so `crates/nx-codegen/src/tests.rs` now asserts `tags: ["admin", "editor"].flat()` and `return [1, 2, 3].flat();`. The emitter has the static item types available and every item here is a string or int literal, so the call can never do anything. The design's argument for unconditional splicing (`object` may hold a sequence) applies to the IR runtime, where the item types are not in hand at emit time.
- **Recommendation:** Skip `.flat()` when no item's static type is a sequence and none is `object` (`Type::Error`/unknown counting as "may be"), keeping it everywhere else. Adjust the emission assertions accordingly.
- **Fix:** `emit_sequence_items` appends `.flat()` only when some item can contribute something other than itself, which `item_can_splice` decides from the item's static type: a sequence, an `object` (opaque, and it may hold one), a type the checker left open, or a conditional that can contribute nothing and is emitted as `[]`. Everything else is exactly one item. `{"admin" "editor"}` is now `["admin", "editor"]` and `{1 2 3}` is `[1, 2, 3]`, while `{xs "c"}` keeps `[xs, "c"].flat()`. The two emission assertions in `crates/nx-codegen/src/tests.rs` were updated to match.

- **Verification:** confirmed, and probed hard because a wrong `false` from `item_can_splice` would be silent. `{"admin" "editor"}` and `{1 2 3}` now emit without `.flat()`; every item form that can contribute something other than itself still gets it, checked end to end against the interpreter and the IR runtime: an alias-typed sequence (`type Names = string[]`, item typed `Names` — `ty` is the resolved `Array`, so it splices), an `object`-typed item actually holding a sequence, a call returning `T[]`, a field access of a `T[]` field, a `for` as an item, a closed conditional whose branches are sequences, and an open conditional with a sequence branch taken and not taken. `type_can_hold_a_sequence` answers `true` for `Nullable(object)` and falls through to `true` for a type parameter, an error type and an unresolved variable, and `ty: None` is `true`, so every uncertain case is conservative. No divergence found.

### ✅ Verified - RF11 Stale `void` doc comment and an over-long doc-comment line in `ty.rs`
- **Severity:** Low
- **Evidence:** `crates/nx-types/src/ty.rs:193` still reads `/// Primitive type (int, int32, int64, float32, float64, string, boolean, void)` although `Primitive::Void` was removed by this change. `crates/nx-types/src/ty.rs:41` is 134 characters — the `<para>` on `Primitive::Never` was re-joined when the `Primitive::Void` cross-reference came out, past the repo's 120-character doc-comment wrap rule in `AGENTS.md`.
- **Recommendation:** Drop `void` from the `Type::Primitive` comment and re-wrap `ty.rs:41` at 120 columns.
- **Fix:** `Type::Primitive`'s doc comment lists `never` in place of `void`, and `Primitive::Never`'s `<para>` is re-wrapped inside 120 columns — its first sentence had also been left reading "an author receives it, they never write it" by the `Void` cross-reference's removal, so it is rewritten.

- **Verification:** confirmed. `Type::Primitive`'s doc comment reads `never` in place of `void`, `Primitive::Never`'s `<para>` is inside 120 columns, and no line in `ty.rs` exceeds 120. A workspace grep finds no `Primitive::Void` or `Type::void()`. The only line over 120 characters this change adds anywhere is a JavaScript `console.log` in `emitted-ir.test.mjs`, which the doc-comment rule does not cover.

### ✅ Verified - RF12 Two `- [x]` tasks record verification steps whose stated result does not hold
- **Severity:** Low
- **Evidence:** All 32 tasks are marked done and the substance of each is implemented; two of the *verification clauses* are not literally satisfied.
  - 9.2: "verify no page under `docs/src/content/docs` contains `[][]` … by grepping" — three pages still contain `[][]`: `reference/concepts/sequences-and-objects.md:108` and `:125`, `language-tour/types.md:117`. All three state the rule rather than offer an example, which is the right outcome, so the task text is what is wrong.
  - 1.1: "verify … `string[][]`, `string[]?[]`, `(string[])[]` and `<function />: string[][]` each report **one** error on the second `[]`" — at the parser level this holds (`test_nested_sequence_suffix_is_rejected` passes, and `((string[]))[]` is caught too), but end to end `type Matrix = string[][]` reports two (see RF5).
- **Recommendation:** Reword 9.2's verification to "contains no `[][]` example" and re-check 1.1 once RF5 is fixed.
- **Fix:** 9.2's verification clause now asks that no page offer a `[][]` *example* and says the three remaining occurrences are prose stating the rule. 1.1 needed no change: with RF5 fixed, `string[][]`, `string[]?[]`, `(string[])[]` and `<function />: string[][]` each report exactly one error end to end, which is what it claims.

- **Verification:** confirmed. 9.2's verification clause now asks that no page offer a `[][]` *example* and names the three prose occurrences; the grep for `[[`, a tuple type and `void` as an inferred type still comes back clean. 1.1 holds end to end after the RF5 work: `string[][]`, `string[]?[]`, `(string[])[]`, `<function />: string[][]` and `((string[]))[]` each report exactly one error, on the second `[]`, through `nxlang run`.

### ✅ Verified - RF13 Interpreter re-evaluates a conditional item's condition, and for a match its scrutinee and every pattern, twice; and one debug assertion misses `object?`
- **Severity:** Low
- **Evidence:** `contributes_no_items` (`crates/nx-interpreter/src/interpreter.rs:3495`) evaluates the condition, and for a `Match` runs `match_arm_body`, which evaluates the scrutinee *and* every pattern expression; `eval_item_into` then calls `eval_expr`, which does all of it again. The code comment only owns up to "one extra evaluation of a pure expression", which understates the match case, and this runs once per `for` iteration. Separately, `interpreter.rs:3650` asserts `!matches!(item, Value::Array(_)) || is_object_type(expected_item)`, and `is_object_type` (`nx-types/src/semantics.rs:66`) does not strip nullable, so an `object?[]` element holding a sequence would panic in a debug build where an `object[]` element would not.
- **Recommendation:** Have `contributes_no_items` return the taken branch (it already computes it) so the item is evaluated once — this is also the shape RF1's fix wants — and use `expected_item.strip_nullable()` in the assertion.
- **Fix:** both halves. The double evaluation is gone with RF1's restructure — `contributes_no_items` no longer exists, so the condition, the scrutinee and the patterns are evaluated once and the taken branch is handed straight to `eval_item_into`. The debug assertion now tests `is_object_type(expected_item.strip_nullable())`, so an `object?[]` element holding a sequence is the same exception an `object[]` one is.

- **Verification:** confirmed on both halves. `contributes_no_items` no longer exists, so a conditional item's condition — and a match item's scrutinee and patterns — are evaluated once; the taken branch is handed straight to `eval_item_into`. The debug assertion now reads `is_object_type(expected_item.strip_nullable())`, so an `object?[]` element holding a sequence is the same exception an `object[]` one is. All probes ran against a debug build, so the assertions were live throughout.
- **Superseded mechanism (rule change):** `contributes_no_items` and the per-item branch resolution are both gone, so there is no second evaluation of a condition, scrutinee or pattern anywhere — `eval_item_into` evaluates each item exactly once. The `strip_nullable()` half of the fix is unchanged.

- **Re-verification (rule change, 2026-09-21):** confirmed. `eval_item_into` is now `match self.eval_expr(item)? { Array(e) => extend, other => push }`, so every item — conditional or not — is evaluated exactly once. The debug assertion still reads `is_object_type(expected_item.strip_nullable())`.

## New Findings Discovered During 2026-09-20 21:05 Verification

### ✅ Verified - RF14 The IR runtime binds a content property's declared default when body content produces no values, where the interpreter and generated JavaScript bind the empty list
- **Severity:** Medium
- **Evidence:** `conditional-result-types` scenario "A conditional that is the whole body produces the empty list when not taken" says that with a body of exactly `{if c { <A/> }}` at a content property declared `A[]` **with a non-empty default**, and `c` false, "evaluation SHALL bind the empty list rather than the declared default". Measured across the three engines:

  ```nx
  type A = { n:int = 1 }
  type Box = { content items:A[] = { <A n=9 /> } }
  let c = false
  let root(): Box = { <Box>{if c { <A n=2 /> }}</Box> }
  ```

  - interpreter: `{"items":[]}`
  - IR runtime: `{"items":[{"$type":"A","n":9}]}`  ← the declared default
  - generated JavaScript: `{"items":[]}`

  The IR runtime falls back to the default whenever a content list comes out empty, not only for this form: `<Box>{}</Box>` and a zero-iteration `for` body behave the same way, which `braced-value-sequences` also pins ("Empty body content evaluates to an empty list", "Body content that produced no values binds the empty list … SHALL NOT fall back to the declared default"). I reproduced both of those against a `HEAD` build and `HEAD`'s `runtime/typescript/dist`, so the fallback **predates this change**. What is new is the untaken-conditional instance: before this change that body produced a `null` item, so the list was never empty and the fallback never fired, and `conditional-result-types` is a capability this change adds. The existing `emitted-ir.test.mjs` case for an untaken conditional child puts a sibling `<A/>` beside it, so the list is non-empty and the gap is not exercised.
- **Recommendation:** In `runtime/typescript/src/index.ts`, make a content property whose body was *written* bind the value `contentAt` produced even when it is empty, and apply the declared default only where the element has no body at all — the distinction `normalize_content_values` draws in the interpreter (`field.default >= 0` at `index.ts:3077` is where the fallback is taken). Add the non-empty-default case to `emitted-ir.test.mjs` for all three forms: `{}`, a zero-iteration `for`, and an untaken conditional as the whole body.
- **Fix (pass 2):** `applyContentBinding` in `runtime/typescript/src/index.ts` now takes `hasBody`, read from the element's written child count (`hasBodyAt`, which is `entry[at] !== 0`), and falls back to the declared default only when no body was written at all. A body that was written and produced nothing binds the empty list, as the interpreter and generated code already did. The binding itself changed from `content.length > 1` to `content.length !== 1` so that an empty written body binds `[]` rather than reading `content[0]` off the end. `constructComponentDescriptor` passes `false`: a host supplies content as an argument that defaults to `[]`, with no body to have been written, so nothing there can mean "a body that produced nothing". Verified across all three engines for an untaken conditional child, `<Box>{}</Box>` and a zero-iteration `for`, all now `[]`, with an absent body still taking the default. New three-engine case in `emitted-ir.test.mjs`, built so each element's body stands alone — the earlier test put a sibling beside the conditional, which is why the list was never empty and the gap was never exercised.

- **Verification:** confirmed, and it holds across every shape I could reach. All three engines now agree at `[]` for an untaken conditional as the whole body, `<Box>{}</Box>`, and a zero-iteration `for`, each with a non-empty declared default; an absent body still takes the default (the remaining generated-JS difference there is RF15, not this). Also checked: a content property with **no** default → `[]`; a union case with content → `[]`; a component with content → `[]`; an empty body nested inside a non-empty one → the inner binds `[]` and splices as one child. The two paths I worried about from reading the code — `applyContentBinding` now reaching `fail("… does not accept content")` for a record with no content field, and binding `[]` into a **non-list** content property, both newly reachable because the `content.length === 0` early return is gone — are unreachable from valid source: the checker rejects a body on a record with no content field and rejects `{}` at a non-list content property, in all three engines. `content.length !== 1` is right: a one-child body at a list-typed property already bound a list through `bindsList`, so the change only affects the zero case. `constructComponentDescriptor` passing `false` is behaviour-preserving — I called it directly with no content argument, with `[]`, and with one child, and got the same results as before the fix. The new `emitted-ir.test.mjs` case does give each element its own standalone body and covers all four forms, comparing IR against the interpreter wholesale and against generated JS per field, with an honest comment about the one excluded field.

### ✅ Resolved - RF15 Generated JavaScript never lifts a single item to a one-element list at a sequence-typed site
- **Severity:** Low
- **Evidence:** `sequence-model` requires that "the interpreter, the TypeScript IR runtime and every code generation target SHALL produce the same value for the same source" and that an item supplied where a sequence type is expected is treated as a one-element sequence. The emitted-JS backend applies no declared-type coercion at all, so wherever the lift is what makes the value a list, it emits the bare item:

  | source | interpreter / IR runtime | generated JS |
  | --- | --- | --- |
  | `<Box things={ "a" } />` at `things:object[]` | `{"things":["a"]}` | `{"things":"a"}` |
  | `<Box xs={ 3.0 } />` at `xs:float64[]` | `{"xs":[3]}` | `{"xs":3}` |
  | `let ys:A[] = { <A n=4 /> }` | one-element list | bare record |
  | `<Box>{if c { A2 } else { A3 }}</Box>` at content `A[]` | `{"items":[A3]}` | `{"items":A3}` |
  | `<Box />` with `items:A[] = { <A n=9 /> }` | `{"items":[A9]}` | `{"items":A9}` |

  Byte-for-byte the same output comes from a `HEAD` build for the `let ys` and property cases, so this **predates this change** and none of it is conditional-related — `emit_content_value`'s arity-one path and property emission behave as they did. It is recorded because `sequence-model`, which this change adds, is the first requirement that states the cross-engine guarantee it breaks, and because a consumer of generated JavaScript that iterates a `T[]` field will iterate a string's characters or fail on a record.
- **Recommendation:** Decide whether generated JavaScript owes the same value as the other two engines. If so, emit a one-element array at a sequence-typed property, field default and annotated binding when the value is statically a single item — the types are in hand at emit time, as RF10's `item_can_splice` shows — and add the cases above to the three-engine test in `emitted-ir.test.mjs`. If not, `sequence-model`'s cross-engine requirement needs a stated exception. Either way this is larger than flat-sequences and may belong in its own change.
- **Status:** left open — agreed, and agreed it is its own change. This is a missing coercion layer in one target, not a sequence-model defect: generated JavaScript binds a value to a sequence-typed site without ever consulting the declared type, so the single-item lift is absent everywhere, at properties, content, `let` annotations and field defaults alike. Adding it means driving emission from declared types at every binding site in `emit.rs`, which is a larger and independently testable piece of work than anything else in this change, and it is byte-identical at HEAD so nothing here caused or worsened it. The new `emitted-ir.test.mjs` case compares the three engines field by field for exactly this reason, with a comment saying so.

- **Verification:** reasoning reviewed and agreed; correctly left open. It is a missing coercion layer in one target rather than a sequence-model defect — generated JavaScript binds to a sequence-typed site without consulting the declared type anywhere, so the lift is absent at properties, content, `let` annotations and field defaults alike — and I re-confirmed byte-identical output from a fresh `HEAD` build for the `let ys:A[] = { <A/> }` and property cases, so nothing here caused or worsened it. The `absent` field is the only one the new RF14 test excludes from the generated-JS comparison, and the comment there says why, which is the right way to leave it.
- **Resolution:** split into its own change, decided with the author. It is a missing coercion layer in generated JavaScript — values are bound without consulting the declared type, so the single-item lift is absent at every binding site — not a sequence defect, and it is byte-identical at HEAD. The rule change briefly made it more reachable, since a taken `if c { 1 }` evaluated to a bare `1`; RF18's source lift took conditionals out of it again, so a taken conditional is `[1]` in every engine before any binding sees it. What the three-engine tests still leave out for this gap: `absent` in the empty-body case (a one-item declared default), and the RF24 case uses a two-item sequence so it does not meet this gap at a record field — each with a comment naming it. (Corrected per RF25: this note first said `taken` was compared on only two engines, which stopped being true with RF18.)

- **Verification (rule change, 2026-09-21):** the split is sound — it is a missing coercion layer in one target, byte-identical at `HEAD`, and not a flat-sequences defect. But the split-out change is scoped too narrowly. Re-probing found that the **IR runtime** has the same gap at annotated `let` bindings and declared function returns: `let root(): int[] = { 1 }` returns `1` in both the IR runtime and generated JS (interpreter: `[1]`), and `let v:int[] = { 1 }` then `for x in v` throws in the IR runtime and gives `[]` in generated JS. That is identical at `HEAD`, so it is pre-existing, but RF15 as written names only generated JavaScript. Recorded as RF20 so the split-out change covers both targets. The comment on the `taken` exclusion in the new `emitted-ir.test.mjs` case ("the two runtimes that lift a lone item to a one-item list") is true only at a record field, which is where the test happens to put it.
- **Scope widened (RF20):** the split-out change must cover the TypeScript IR runtime as well as generated JavaScript — `let root(): int[] = { 1 }` gives `1` in both. Conditionals are no longer part of it: RF18's source lift makes a taken conditional a one-item sequence in every engine before any binding sees it.

## New Findings Discovered During 2026-09-20 21:20 Verification

### ✅ Verified - RF16 A self-referential type alias reports a spurious "a sequence cannot contain sequences" on top of its cycle error
- **Severity:** Low
- **Evidence:** Cycle recovery in `resolve_named_type` (`crates/nx-types/src/infer.rs`) returns `Type::Error` for the name that closed the cycle, but the enclosing suffixes are still applied to it, so the alias caches a *partially built* type that later checks then read as real:

  ```nx
  type A = A[]
  ```
  ```
  error: Type alias 'A' forms a cycle
  error: A sequence cannot contain sequences; 'A' is already a sequence
  ```

  Walking `A`'s target `Array(Name(A))` resolves the inner `Name(A)`, which re-enters `resolve_named_type`, detects the cycle, and returns `Type::Error`; the inner `Array` arm wraps that into `Array(Error)` and caches `A → Array(Error)`; the outer `Array` arm then sees an `Array` element and reports nesting. `type A = A?[]` produces the same pair. `A` is not "already a sequence" — it is not a type at all — so the second message is wrong, and one mistake yields two diagnostics. `HEAD` reports neither, because it has no eager alias pass and nothing uses the alias, so this is surfaced (not caused) by the pass this change added.
- **Recommendation:** make cycle recovery poison the whole chain rather than one name: when `resolve_named_type` reports `type-alias-cycle`, cache `Type::Error` for the alias being walked as well as returning it, and have the `Array`/`Nullable` arms of `type_from_type_ref_walk` return `Type::Error` unchanged when their inner resolved to `Type::Error` instead of wrapping it. Either half alone fixes this case; the second is the more general guard, since any check reading a suffix-wrapped error type has the same problem. Add `type A = A[]` as a test asserting exactly one diagnostic, the cycle.
- **Fix (pass 3):** took the second of the two suggestions, and it needed both suffix arms rather than one. The `Array` arm returns `Type::Error` unchanged when its element is an error instead of building `Array(Error)`, so an enclosing `[]` cannot read the error as a sequence; the `Nullable` arm does the same, which is what `type A = A?[]` needs — `Nullable(Error)` would otherwise be wrapped into `Array(Nullable(Error))` and the outer `[]` would believe it just the same. Both spellings now report the cycle alone. Test: `a_self_referential_alias_reports_only_its_cycle`.

- **Verification:** confirmed, both spellings and both arms. `type A = A[]` and `type A = A?[]` each report the cycle alone; so do `type A = A`, `type A = A?`, and the two-alias forms `type A = B[]` / `type B = A[]` and `type A = B?` / `type B = A?`. I went looking for collateral from making the suffix arms propagate `Type::Error` and found none that is wrong. What it does change is that a follow-on mismatch naming an error-containing type is now suppressed: `type Wrapped = Rows[]` with `let u:Wrapped = { "x" }` used to add `expects string[][][], found string`, and `type Wrapped = Bad[]` used to add `expects <Box T=int[]/>[], found string` — both gone, which is the right outcome, since printing a type built out of an already-reported error helps nobody. Independent diagnostics still fire: in one file, the alias error, an unannotated-empty-list error and an unrelated `expects string, found int` all appear. An unknown name in a suffix position is unaffected, because it resolves to a `Named`, not to `Type::Error` (`type X = Nope[]` reports nothing, at `HEAD` too — a separate pre-existing gap). The one suppression I traced that hides a *distinct* mistake — a bad member access on a parameter whose alias type is an error — comes from the nested-sequence rejection returning `Type::Error`, which predates this pass, not from the propagation change.

## New Findings Discovered During 2026-09-20 21:30 Verification

### ✅ Verified - RF17 A type-alias cycle that crosses modules is reported at a span from the other file
- **Severity:** Medium
- **Evidence:** `resolve_named_type` reports `type-alias-cycle` at `alias.span` taken straight from `TypeAliasInfo`. That is the field RF5's pass-3 fix guards the *nesting* message against, because for an imported alias it holds the span the **declaring** module wrote — but the cycle arm a dozen lines above is ungated, and the eager alias pass made it reachable. One library, two modules:

  ```
  lib/a.nx   (27 lines)   line 26: export type A = B
                          line 27: export type UsesA = { x:A }
  lib/b.nx   (2 lines)    line  1: export type B = A
                          line  2: export type UsesB = { y:B }
  ```

  | | `HEAD` | working tree |
  | --- | --- | --- |
  | | `'A' forms a cycle` → **a.nx:26:1** | `'B' forms a cycle` → **a.nx:1:1** |
  | | `'B' forms a cycle` → **b.nx:1:1** | `'A' forms a cycle` → **b.nx:3:1** |

  `HEAD` puts each message on its own alias. The working tree puts `'B'`'s message on `a.nx:1:1`, a filler comment in the wrong file, and `'A'`'s message on `b.nx:3:1` — **past the end of a two-line file**, which is `a.nx`'s line-26 byte offset rendered against `b.nx`. This is a regression, not a pre-existing gap: at `HEAD` the cycle was first entered from a *use* site, so it closed on the local alias whose span was right; `validate_local_type_aliases` now enters from the local alias first, so it closes on the imported one. An out-of-range span is worse than an imprecise one — in an editor it lands on nothing, or on whatever text happens to occupy those bytes.
- **Recommendation:** gate the cycle span exactly as the nesting span is gated: report at `local_type_alias_spans.get(name)` when the name is one this module declared, and at the current `type_ref_span` otherwise, which is always a span this module wrote. Three lines, mirroring the override already in the same function. A test needs two modules, so it belongs wherever a multi-module harness lives — `nxlang typegen <library directory>` reproduces it from the CLI.
- **Fix (pass 4):** the three lines, and then the cause underneath them. The cycle arm now takes its span from `local_type_alias_spans` when the name is one this module declared and from the reference that reached it otherwise, which puts every span back inside the file its diagnostic is attributed to. That alone still left the message naming the wrong alias — `a.nx:26` underlining `export type A = B` while saying `'B' forms a cycle` — because `validate_local_type_aliases` entered the walk at the alias's *target*, leaving the alias itself out of `seen` until the loop came back round, so the cycle closed on whichever name completed it. A new `resolve_local_alias_target` seeds `seen` with the alias's own name and resolves the target unscoped, which is exactly what `resolve_named_type` does, so the eager pass and a use site now perform the same walk and cannot cache different answers. Output matches HEAD again: `'A' forms a cycle` at `a.nx:26:1`, `'B' forms a cycle` at `b.nx:1:1`. `TypeAliasInfo.span` had been this arm's only reader and is now removed — it was a foreign span with no safe use, and the type carries a note saying so. Test: `a_type_alias_cycle_across_modules_is_reported_where_each_alias_was_written` in `crates/nx-cli/src/typegen.rs`, which is where the library harness lives since `check_str` cannot link modules; it asserts each file's message and offset and that no label runs past the end of the file it underlines, and it fails on the pre-fix code.

- **Verification (pass 4):** confirmed, and the extra level was needed. On the exact case that exposed the finding — one library, `a.nx` 27 lines with `export type A = B` at line 26 and a 2-line `b.nx` with `export type B = A`, both used — the working tree now reports `'A' forms a cycle` at **a.nx:26:1** and `'B' forms a cycle` at **b.nx:1:1**, byte-for-byte what a freshly built `HEAD` reports, where pass 3 put `'B'` on a filler comment in the wrong file and `'A'` past the end of a 2-line file. Every span I measured is in range and names the alias on the line it underlines.

  The four attacks you asked for:
  - *The `scoped` change.* Unobservable, and I checked it rather than reasoning: `type_parameter_scope` is written only inside component, record-default and applied-type checking, and `validate_local_type_aliases` is the second call in `new()`, before `register_function_signatures`, `register_value_bindings` and both default passes. Behaviourally, an alias whose target spells a name that is *also* a component type parameter resolves to the module-level type and not to the parameter (`type TValue = string` beside `component <Slider TValue:type …/>`, with `type Alias = TValue`), and an alias target naming *only* a component's parameter still resolves to nothing — identical to `HEAD` in both cases. Substitution through an alias to an applied type is intact: `type IntRange = <Range2 T=int/>` with `r.start` at an `int` is clean and at a `string` reports `expects string, found int`, the same message `HEAD` gives, which is the proof the argument was substituted rather than skipped.
  - *`seen` seeding, non-cycle cases.* No false cycles. A diamond is clean in both declaration orders; an alias that names a record which names the alias back (`type R = { x:A? }`, `type A = R`) is clean; self-reference through an applied type inside a record (`type Node = { T:type value:T next:<Node T=T/>? }` with `type IntNode = <Node T=int/>`) is clean and usable. A genuine self-reference through an applied type (`type Self2 = <Box T=Self2/>`) reports the cycle once, which is right.
  - *`TypeAliasInfo.span` removed.* The struct now carries only `target`, the workspace builds and the suite passes, and the three remaining readers of the map all read `target`. The removed field's one reader was the cycle arm, which for a local alias now takes the same span out of `local_type_alias_spans`, so nothing is lost locally and the imported case is what improved.
  - *Longer and mixed cycles.* A three-module cycle reports each alias in its own file at its own declaration (`a.nx:11:1`, `b.nx:4:1`, `c.nx:1:1`). A cycle whose members are reached only from a record field in a third module reports `'A'` and `'B'` in their own modules and, in the third, at the **field's** span (`holder.nx:1:24`) — in-file, because the name is not local there and the reference that reached it is the fallback, exactly as the comment says.

  I confirmed the new test discriminates without rebuilding the pre-fix code: it asserts each file's message *and* start offset *and* that no label ends past the length of the file it underlines, and pass 3's measured output — `'B'` at a.nx offset 0 and `'A'` at a b.nx offset of roughly 250 bytes in a 46-byte file — violates all three. `cargo test -p nx-cli a_type_alias_cycle_across_modules` passes on the fixed code.

  One expected consequence, recorded rather than filed: a cross-module cycle is now reported once per module that reaches it, so the three-module case yields three diagnostics and the record-field case yields three. That is inherent to per-module checking and matches how the nesting message already behaves across modules; `HEAD` reported fewer only because it had no eager pass.

  I also re-ran the RF5 and RF16 suites after this pass, since it touches both. All sixteen local cases and all five library-directory cases are unchanged: counts, spans and messages identical to what I verified in pass 3. The only differences anywhere are which end of a purely local cycle closes first — `type A = B` / `type B = A` now names `'A'` at line 1 rather than `'B'` at line 2, and each is self-consistent.

## New Findings Discovered During 2026-09-21 01:50 Verification

### ✅ Verified - RF18 The lifting join types a conditional as a sequence while every engine still evaluates the taken branch to a bare item, so newly accepted programs crash or compute the wrong answer
- **Severity:** High
- **Evidence:** the rule makes `if c { 1 }` an `int[]`, and the new arm in `common_supertype` makes a *closed* `if c { 1 } else { xs }` an `int[]` too. But no engine changed what the taken branch evaluates to: `eval_if`, `nodeKinds.if` and the emitted `(c ? 1 : [])` all produce the bare `1`. The type says sequence, the value is an item, and nothing reconciles them except a downstream typed-site lift — which exists in the interpreter at annotated sites, in the IR runtime only at record fields, in generated JavaScript nowhere, and in no engine at an unannotated binding or a `for` iterable.

  ```nx
  let c = true
  let v = { if c { 1 } }
  let root(): int[] = { for x in v { x * 10 } }
  ```
  - `HEAD`: **rejected** — `For iterable must be an array, found void`.
  - working tree: **accepted**, then interpreter `Runtime error: Type mismatch in for loop iteration: expected array, got int`; IR runtime throws `For expression iterable must evaluate to an array.`; generated JavaScript returns `[]` silently (`Array.from(1)` is empty).

  `let v = { if c { 1 } else { xs } }` fails the same three ways. The documentation's own headline example for the rule, `let tags:string[] = { if c { "new" } }` from `if.md`, evaluates to `["new"]`, `"new"` and `"new"` in the three engines, and iterating it gives `["new!"]` in the interpreter, a throw in the IR runtime, and **`["n!","e!","w!"]`** in generated JavaScript, which iterates the string's characters. A program the checker newly accepts now crashes in two engines and silently gives the wrong answer in the third — the class of divergence this change exists to remove. The new three-engine case in `emitted-ir.test.mjs` does not catch it because it places the taken conditional only at a record field (`taken:int[]`), the one site where the IR runtime lifts.
- **Recommendation:** make the value match the type at its source rather than relying on each engine's downstream lift. When the join lifts an item branch to a sequence, record that per branch in the checker — exactly as `widened_joins` records a numeric widening — and lower it to a wrap node alongside `Expr::Widen`, so the interpreter, the IR runtime and generated JavaScript all produce `[1]` for the taken branch and `[]` for the untaken one. That also fixes numeric widening across a lifted join (`if c { 1 } else { floats }` currently leaves the taken `1` as an `Int`). Then extend the three-engine case with an unannotated binding, an annotated `let`, a declared function return and a `for` over each.
- **Fix:** took the recommendation — lift at the source, beside the widenings. `record_join_lifts` records each `if`/match branch whose type is not sequence-shaped while the join's is, in a new `lifted_joins` set next to `widened_joins`; after analysis, a new `nx_hir::apply_join_lifts` wraps each one as a one-element `Expr::Array` literal, and `check.rs` types the wrapper as a sequence of its branch. An array literal is something every engine already evaluates to a sequence and splices, so no engine needed a change: a taken `if c { 1 }` is `[1]` in the interpreter, the IR runtime and generated JavaScript, and a lifted `object` holding a sequence stays flat. The lifts run *before* `apply_join_widenings`, which visits array literals as parents too, so a branch that is both lifted and widened is widened as its wrapper's element; `join_widening` gained an arm so an item widens to a joined sequence's element type. Measured on all three engines: `let v = { if c { 1 } }` iterated gives `[10]`, the docs' `tags` example iterated gives `["new!"]` (JavaScript previously gave `["n!","e!","w!"]`), `if c { 1 } else { xs }` iterated gives `[1]`, and `if c { 1 } else { floats }` with no annotation to repair it gives `[1.0]`. Tests: a new three-engine case iterating all three; `a_taken_conditional_is_a_one_item_sequence_not_a_bare_item` and `a_lifted_branch_that_widens_widens_inside_its_sequence`, which run the *analyzed* module through a new `execute_analyzed` harness — the file's existing `execute_function` lowers without analysis and so skips every rewrite analysis writes back, which is why a first draft of the test failed while the CLI passed. The implicit `{}` needs no rewrite, since `eval_if` returns the empty sequence itself; only the lift does. The earlier three-engine case no longer excludes `taken` from the JavaScript comparison: with the lift at the source it agrees, so the case now compares the whole value.

- **Verification (Fix Pass 5):** confirmed. Every repro in the finding now agrees in all three engines with the value its type promises: an unannotated `if c { 1 }` iterated gives `[10]`, the closed `if c { 1 } else { xs }` iterated gives `[10]`, `if.md`'s `tags` example iterated gives `["new!"]` (not `["n!","e!","w!"]`), and a conditional at a declared `int[]` return iterated gives `[10]`. Lift and widening compose correctly: `if c { 1 } else { fs }` with `fs:float64[]` evaluates to `1.0` in the interpreter, so the widening lands on the lifted element and nothing is widened twice (a lifted branch is by construction not sequence-shaped, so it cannot also take the `(Array, Array)` widening arm). The new `(member, Array(joined))` arm in `join_widening` cannot misfire in `Expr::Array` inference, where the joined type is an item type and never an `Array` under the flat rule, and `apply_join_lifts` visits only `If`/`Match` parents, so no other join grows a wrapper. The collecting-position suite (nested, empty arm, RF6 widening) is unchanged. The new iteration case in `emitted-ir.test.mjs` compares the whole value across all three engines, and the earlier case no longer excludes `taken` — both pass. The direct-HIR asymmetry you raised is the existing contract, not a finding: widenings and literal conversions are post-analysis rewrites too, and unanalyzed execution already skips them; recorded under Questions only so the contract is written down. One defect in the *lift condition* is filed separately as RF24.

### ✅ Verified - RF19 The lifting join drops a nullable sequence's `?`, so `null` is accepted at a non-nullable sequence site
- **Severity:** High
- **Evidence:** the lifting arm takes `let Type::Array(element) = sequence.strip_nullable()` and returns `Type::array(join(element, item))`, discarding the sequence side's nullability. So `join(string[]?, string)` is `string[]`, not `string[]?`:

  ```nx
  let maybeNames:string[]? = null
  let c = true
  let v:string[] = { if c { maybeNames } else { "x" } }
  let root() = { v }
  ```
  - `HEAD`: **rejected** — `expects string[], found object`.
  - working tree: **accepted**, then the interpreter fails with `Type mismatch in initializer for 'v': expected string, got object?`, and the IR runtime and generated JavaScript both bind **`null`** to `v:string[]`. The unannotated form infers `string[]` and holds `null`.
- **Recommendation:** keep the nullability of the side that was sequence-shaped: build `Type::array(self.common_supertype(element, item))` and wrap it in `Type::nullable` when `sequence` was `Nullable`. Add the case above as a test that asserts rejection, next to `join(string[]?, string[])`.
- **Fix:** the lifting arm re-wraps its result in `nullable` when the sequence side was nullable, so `join(string[]?, string)` is `string[]?`. The reviewer's case, `let v:string[] = { if c { maybeNames } else { "x" } }`, is now rejected statically as `expects string[], found string[]?` — the null that every engine used to bind to a `string[]` can no longer reach one. The same program at a `string[]?` annotation is accepted and evaluates to `null` in all three engines. The guard on exactly one sequence-shaped side is unchanged, so `string[]?` beside `string[]` still takes the nullable arm and is not lifted. Test: `lifting_beside_a_nullable_sequence_keeps_its_question_mark`.

- **Verification (Fix Pass 5):** confirmed. `let v:string[] = { if c { maybeNames } else { "x" } }` is rejected statically as `expects string[], found string[]?`, so no engine can bind `null` to a `string[]` through that join; at a `string[]?` annotation it is accepted. The fix is exactly the one line recommended, and the exactly-one-side guard is unchanged.

### ✅ Resolved - RF20 The IR runtime does not lift a single item at an annotated `let` binding or a declared function return
- **Severity:** Medium
- **Evidence:** pre-existing, byte-identical at `HEAD`, and the IR half of RF15. `normalizeValue` lifts an item to a one-item list at record fields and component props, but a `let` binding's annotation and a function's declared return type are never normalized:

  | source | interpreter | IR runtime | generated JS |
  | --- | --- | --- | --- |
  | `let root(): int[] = { 1 }` | `[1]` | `1` | `1` |
  | `let v:int[] = { 1 }`, then `for x in v { x * 10 }` | `[10]` | throws | `[]` |
  | `let f(): int[] = { 1 }`, then `for x in f() { … }` | `[10]` | throws | `[]` |

  Before the rule change this took a scalar written at a list-typed site, which is rare. Now every `if` with no `else` at an annotated `let` or return depends on it, because that is the only thing that makes the taken branch a list.
- **Recommendation:** fold into the change RF15 was split into, so it covers the IR runtime as well as generated JavaScript: normalize a `let` binding against its annotation and a function result against its declared return type, as record fields already are. If RF18 is fixed by wrapping at the source, conditionals stop depending on this, but `let v:int[] = { 1 }` still diverges without it.
- **Resolution:** folded into RF15's split-out change, whose scope is widened to name the IR runtime as well as generated JavaScript. RF18's lift took this off the conditional path: `let root(): int[] = { if c { 1 } }` now gives `[1]` in all three engines, because the branch is lifted at the source before any binding sees it. What remains is the general case — `let root(): int[] = { 1 }` gives `1` in the IR runtime and generated JavaScript — which is pre-existing, identical at HEAD, and not about conditionals at all. It is the same missing single-item coercion as RF15, in a second engine.

- **Verification (Fix Pass 5):** resolution reasoning checked and sound. The conditional path is genuinely off it — `let f(): int[] = { if c { 1 } }` iterated gives `[10]` in all three engines, because the lift reaches the branch before any binding does — and what remains (`let root(): int[] = { 1 }` → `1` in the IR runtime and generated JS) is identical at `HEAD` and unrelated to conditionals. RF15 now carries a "Scope widened (RF20)" note naming the IR runtime.

### ✅ Verified - RF21 Two doc comments still describe the deleted "contribution" notion
- **Severity:** Low
- **Evidence:** `item_contribution`'s doc comment in `crates/nx-types/src/infer.rs` still says "a conditional with no `else` contributes what the branch it takes contributes — nothing, when it takes none", and the comment in `Expr::Array` inference still says an item contributes "the branch's contribution when it is a conditional with no `else`". Both describe `open_conditional_contributions`, which the rule change deleted; the code now strips one sequence layer from any item and nothing else, as `item_contribution_of`'s own inline comment correctly says.
- **Recommendation:** rewrite both to the one rule: a sequence-typed item contributes its elements, anything else contributes itself, and a conditional with no `else` needs no case because its type is already a sequence.
- **Fix:** both comments rewritten. `item_contribution`'s `<para>` now says a sequence-typed item contributes its elements and anything else itself, and that a conditional with no `else` is not a case of the rule because its type is already a sequence. The `Expr::Array` inference comment says the same in passing. A sweep for the deleted wording found no other occurrence.

- **Verification (Fix Pass 5):** confirmed — both comments now state the one splice rule, and no other occurrence of the deleted wording remains.

### ✅ Verified - RF22 Two claims in the rule-change evidence are stronger than what supports them
- **Severity:** Low
- **Evidence:**
  1. "The checker was temporarily instrumented to report every implicit `{}` it fills … it filled none, so no corpus value can have changed." The probe counts only implicit-`{}` fills, but the rule change also added a lifting arm that changes the type of any *closed* conditional or match that mixes an item arm with a sequence arm — with no fill at all. The conclusion still holds: I swept all 37 `if` lines and every multi-line arm in the 39 files and found no closed conditional mixing a scalar arm with a sequence arm, and the byte-identical diff covers the 14 files that evaluate. But for the 20 playground files neither the diff nor `check-examples` compares values, so it is that sweep, not the probe, that rules out a change there.
  2. "Every `nx` block on `if.md` … running." Six of the nine do not run standalone. They are the pre-existing reference fragments (undefined names, the older `let x = if …` form), unchanged by this change and failing identically at `HEAD`; the three blocks the change *adds* all run.
- **Recommendation:** state the probe's scope ("implicit `{}` fills") and add that no closed conditional in the corpus mixes an item arm with a sequence arm, which is what rules out a lifting-arm change; and say "the three blocks this change adds to `if.md` run".
- **Fix:** both claims corrected in place. For the docs, the Rule Change section, task 11.7 and the design now say all 8 blocks on `sequences-and-objects.md` and the 3 this change adds to `if.md` run — the other 6 on `if.md` are reference fragments failing identically at HEAD. For the probe, I did better than cite the sweep: with lifts now recorded, the checker was re-instrumented to report *every* lift, which covers closed conditionals mixing an item arm with a sequence arm as well as implicit fills. It reported none across the 39 files, and a sanity case confirmed it fires. The stated limit remains, and is now explicit: a file failing analysis is covered only as far as inference got, which includes the 20 playground examples under a bare `nxlang run`, so for those the verifier's sweep is the evidence.

- **Verification (Fix Pass 5):** confirmed, and the wording is as strong as the evidence and no stronger. The re-instrumented probe covers every lift, closed conditionals included, which answers the gap I raised; the stated limit — a file failing analysis is covered only as far as inference got, so for the 20 playground files under a bare `nxlang run` the sweep is the evidence — is accurate. The docs claim now matches what I measure: all 8 blocks on `sequences-and-objects.md` run, and on `if.md` the 3 blocks this change adds run while the 6 pre-existing fragments fail as they do at `HEAD`.

### ✅ Verified - RF23 The mismatch message a migrating author now sees reads `found list string[]`
- **Severity:** Low
- **Evidence:** `crates/nx-types/src/infer.rs:5006` formats a sequence-where-a-scalar-was-expected mismatch as `"{} expects {}, found list {}"`, so the word "list" doubles the `[]`: `Record field 'v' on 'R' expects string?, found list string[]`. It predates this change (`let v:string? = { xs }` prints it at `HEAD`), but the rule change makes it the diagnostic every author meets when `v={if c { "x" }}` at a nullable site needs `else { null }` — `HEAD` said `found void` there — and the rule decided deliberately against a tailored hint, so this message is the only guidance. `if.md` quotes it verbatim.
- **Recommendation:** drop "list" (`expects string?, found string[]`), and update the quotation in `if.md`.
- **Fix:** agreed with the verifier's call. The branch now formats `{} expects {}, found {}`, and carries a comment saying why it exists at all — it withholds the scalar-only hints (bare form, function signature, lossy numeric conversion) that do not apply to a sequence mismatch — and that the `[]` already says it is a sequence. Eight assertions across four test files and the `if.md` quotation were updated, and no other occurrence remains.

- **Verification (Fix Pass 5):** confirmed. The message is now `expects string?, found string[]`, the `if.md` quotation matches, and the branch's comment explains why it still exists.

## New Findings Discovered During 2026-09-21 02:05 Verification

### ✅ Verified - RF24 The lifting join treats a `null` branch as an item, so the `else { null }` idiom the rule change prescribes is rejected beside a sequence branch
- **Severity:** High
- **Evidence:** the null literal's type is `Nullable(Variable)`, which `is_sequence_shaped` correctly calls not sequence-shaped — so beside a sequence branch the lifting arm, which sits *before* the `Nullable` arm, lifts it: `join(null, string[])` becomes `string?[]`, and `record_join_lifts` wraps the null branch as `[null]`. At `HEAD` the same join took the nullable arm and gave `string[]?`.

  ```nx
  let xs:string[] = {"a" "b"}
  let c = false
  let v:string[]? = { if c { xs } else { null } }
  ```
  - `HEAD`: accepted, evaluates to `null`.
  - working tree: **rejected** — `expects string[]?, found string?[]`. Same for `if c { null } else { xs }` and for `if s is { "a" => xs else => null }`.
  - unannotated, `if c { null } else { xs }` evaluates to `[null]` — a sequence holding a null item, which is how RF1 began.

  This is the one migration path the rule change documents: "a nullable site needs the `else` written — `if c { "x" } else { null }`". It works for a scalar branch and fails for a sequence branch, so an author who wants a nullable *sequence* has no spelling that compiles except wrapping the sequence in something else. None of the tests has a sequence branch beside `null`.
- **Recommendation:** let the null literal defer to the nullable arm: in the lifting arm's guard, and in `record_join_lifts`, treat a side for which `is_null_literal_type` holds as not an item to lift, so `join(null, T[])` stays `T[]?` and the null branch evaluates to `null`. That keeps `join(int?, never[])` → `int?[]` (a nullable *value* is still an item), which the rule depends on. Add both branch orders and the match form as tests, at a `T[]?` annotation and unannotated.
- **Fix:** took the recommendation. The lifting arm's guard and `record_join_lifts` both exclude a side for which `is_null_literal_type` holds — `Nullable(Variable)`, the untyped null literal only — so `join(null, string[])` falls through to the nullable arm and is `string[]?`, and the null branch is not wrapped. A branch *typed* nullable is `Nullable(Primitive)` or similar, not a variable, so `if c { maybe }` with `maybe:int?` still lifts to `int?[]`, which the rule needs. Measured on all three engines: `let v:string[]? = { if c { xs } else { null } }` type-checks again and evaluates to `null` in both branch orders; the match form type-checks and evaluates to `null` in the interpreter and IR runtime (generated JavaScript refuses every match, as before). Unannotated, `if c { null } else { xs }` is `string[]?` and evaluates to `null`, not `[null]`. Tests: `a_written_null_beside_a_sequence_is_a_nullable_sequence_not_a_null_item` (both orders, the match form, and the `int?` control), `an_explicit_null_else_beside_a_sequence_is_a_nullable_sequence` on the analyzed interpreter, and a new three-engine case. That case first failed on its JavaScript comparison because `xs` held one item, which meets RF15 at a record field; it now uses two items, with a comment saying why, so it tests RF24 alone.

- **Verification (Fix Pass 6):** confirmed. `let v:string[]? = { if c { xs } else { null } }` is accepted again and evaluates to `null` untaken and `["a","b"]` taken, identically in all three engines; the reversed order gives `null`; the match form (`if s is { "a" => xs else => null }`) gives `null` in the interpreter and IR runtime (generated JS refuses matches, as ever). The exclusion is exactly the *untyped* null: a null reached through an unannotated variable (`let n = null`) is still the null-literal type and behaves the same, while every *typed* nullable value — `int?`, `string?`, `object?`, a nullable record, a nullable union — is an item and lifts to `T?[]`, giving `[null]` when that value is null. That is consistent with the rule (a nullable value is an item; a written null item in a `T?[]` is legal) and all three engines agree on each. No other path produces an *untaken-conditional* null item: nested conditionals mixing `null` and a sequence give `null`, not `[null]`, and a `for` body or a braced list holding the idiom produces a written null item only where `else { null }` was actually taken. The two-item `xs` in the new three-engine case is the right call and hides nothing about RF24 — the assertions that matter (`absent` and `reversed` are `null`, not `[null]`) do not depend on `xs`'s length; strictly, the gap a one-item `xs` would meet is at the `let xs:string[] = {"a"}` annotation rather than at the record field, but that is the same RF15 gap either way. One adjacent defect is filed as RF27.

### ✅ Verified - RF25 `record_join_lifts` carries `record_join_widenings`' doc comment, and RF15's resolution still describes the pre-lift behaviour
- **Severity:** Low
- **Evidence:** `record_join_lifts` was inserted between `record_join_widenings` and its doc comment, so it is documented as "Records each branch of a join whose numeric type is narrower than the join's", while `record_join_widenings` now has no doc comment at all; `record_join_lifts`' actual rule is in an inline comment. Separately, RF15's `Resolution` still says `let v:int[] = { if c { 1 } }` "yields a bare `1` in generated JavaScript when the branch is taken" and that the tests compare `taken` "on the interpreter and IR runtime only" — both untrue since RF18's lift, which the later `Scope widened (RF20)` note does not correct.
- **Recommendation:** move the widening doc comment back onto `record_join_widenings`, give `record_join_lifts` its own, and add one line to RF15 saying the conditional case is now `[1]` in all three engines and `taken` is compared across all three.
- **Fix:** both. `record_join_lifts` has its own doc comment and `record_join_widenings` has its original one back. RF15's resolution note now says RF18 took conditionals out of that gap, names what the three-engine tests still leave out and why, and records that it was corrected per RF25.

- **Verification (Fix Pass 6):** confirmed. `record_join_lifts` has its own doc comment and `record_join_widenings` has its original one back; RF15's `Resolution` now says a taken conditional is `[1]` in every engine and states exactly which fields the tests still leave out and why.

### ✅ Verified - RF26 RF6's regression test would pass without RF6's fix
- **Severity:** Low
- **Evidence:** `a_spliced_sequence_widens_to_the_joined_item_type` in `crates/nx-interpreter/tests/arrays.rs` runs through the unanalyzed `execute_function`, which applies no widening rewrite at all, and binds through `let root(): float64[] = { all }` — so the interpreter's return coercion turns `1 2 1.5` into `1.0 2.0 1.5` whatever `record_join_widenings` recorded. The case RF6 was about is the *unannotated* one, where nothing re-coerces.
- **Recommendation:** run it through the new `execute_analyzed` harness with an unannotated `root() = { all }`, and assert `Value::Float(1.0)` for the first element.
- **Fix:** moved to the analyzed `execute_analyzed` harness with an unannotated `root()`, so nothing after the join can repair the value. Confirmed it now discriminates: with RF6's contribution-based widening temporarily disabled the test fails, and restored it passes.

- **Verification (Fix Pass 6):** confirmed independently. The test now runs through `execute_analyzed` with an unannotated `root() = { all }` and asserts `Value::Float(1.0)` first. Without RF6's fix that same program evaluates to `1 2 1.5` with the first two as `Int` — which is exactly what I measured unannotated in the original review, before the fix existed — so the test would fail; with the fix it passes.

## New Findings Discovered During 2026-09-21 02:15 Verification

### ✅ Verified - RF27 A nullable-sequence item in a collecting position contributes itself, so inference builds a nested, unspellable `T[]?[]` for a value every engine produces flat
- **Severity:** Medium
- **Evidence:** `item_contribution_of` strips one layer only from a bare `Type::Array`; a `Nullable(Array(T))` item falls to `other => other` and contributes the whole `T[]?`. So any collecting position holding a nullable sequence infers a sequence *of* nullable sequences:

  | source | inferred (working tree) | `HEAD` |
  | --- | --- | --- |
  | `let maybeXs:string[]? = {xs}` then `{ maybeXs "c" }` | `string[]?[]` | `object[]` |
  | `{ "z" if c { xs } else { null } }` — the RF24 idiom in a braced list | `string[]?[]` | `object[]` |
  | `for n in ns { if (n == 1) { xs } else { null } }` | `string[]?[]` | `string[]?[]` |
  | `<Box>{if c { as2 } else { null }}</Box>` at content `A?[]` | rejected: `found A[]?[]` | — |

  That type is exactly what `sequence-model` forbids ("no type reference, alias chain, type-argument substitution or inference result SHALL produce a sequence whose element type is a sequence"), and it cannot be written — the validator rejects `[]?[]` — so no annotation can accept it: `let v:string?[] = { maybeXs "c" }` fails with `found string[]?[]`, and so does `string[]`. Meanwhile every engine produces a *flat* value, splicing the sequence when present and pushing a null item when absent: `["a","b","c"]` and `[null,"c"]` for the first row, `["z",null]` for the second, identical in the interpreter, the IR runtime and generated JavaScript. So the inferred type is nested, unannotatable, and disagrees with the value. RF24's fix makes this reachable from the documented `else { null }` idiom in any collecting position; for the braced forms the working tree newly produces the nested type where `HEAD` produced `object[]`.
- **Recommendation:** make `item_contribution_of` look through `?` the way the runtime does: a `T[]?` item contributes `T?` — its elements when present, a null item when absent — so `{ maybeXs "c" }` is a `string?[]` and matches `[null,"c"]`. One arm, `Type::Nullable(inner) if matches!(*inner, Type::Array(_)) => Type::nullable(element)`. Add the four rows above as tests, including that `let v:string?[] = { maybeXs "c" }` is accepted.
- **Fix:** took the recommendation. `item_contribution_of` gained one arm: a `T[]?` item contributes `T?` — the element type made nullable, left singly nullable if it already was — which is exactly what every engine does, splicing a present value and pushing an absent one as a null item. So no runtime value changed; only the static type now agrees with it. Measured: `{ maybeXs "c" }` and `{ "z" if c { xs } else { null } }` both infer `string?[]`, type-check at a `string?[]` annotation, and give `[null,"c"]` / `["z",null]` identically in all three engines; a `for` body yielding `maybeXs` infers `string?[]`; `maybe:string?[]?` beside an item stays `string?[]`, not `string??[]`. The content case now reports `found A[]?` rather than the unwritable `A[]?[]`, and its rejection at arity one is sound: a lone child binds as itself, so it could bind `null` to an `A?[]` that does not admit one. With a sibling beside it, the same child is accepted and gives `[A, null]` in all three engines. I also checked the adjacent lone-brace case, `{ maybeXs }`: arity-one braces group, so it is `null` in every engine and typed `string[]?` — the item's own type, which is what covers both a null and a present array — so there is no mismatch there. Test: `a_nullable_sequence_item_contributes_a_nullable_item_not_itself`. On the verification's third question I chose the recommended answer — an absent `T[]?` contributes a null item, since that is what the engines do — and recorded the alternative in `specs/future.md` under the null-removal entry, where it dissolves.

- **Verification (Fix Pass 7):** confirmed for the defect as filed: no inference result is a sequence of nullable sequences any more, and every value agrees across all three engines. All three contribution consumers now get `T?`:
  - **`Expr::Array`:** `{ maybeXs "c" }` is accepted at `string?[]`, giving `["a","b","c"]` present and `[null,"c"]` absent. The idiom in a braced list, `{ "z" if c { xs } else { null } }`, gives `["z",null]`.
  - **`for` body:** a body yielding `xs` or `null` is accepted at `string?[]` and gives `["a","b",null]`. Unannotated, it iterates.
  - **`normalized_sequence_type` (content with a sibling):** `<Box><A n=9 />{if c { as2 } else { null }}</Box>` at `A?[]` gives `[A, null]`.

  Widening through a nullable contribution works, and nothing downstream assumed contributions are non-nullable. `{ maybeInts 1.5 }` infers `float64?[]` and evaluates to `1.0 2.0 1.5` present and `null 1.5` absent. An already-nullable element stays singly nullable: `string?[]?` contributes `string?`, and `{ maybeAliases "c" }` is accepted at `string?[]`. I confirm the lone-brace claim: `{ maybeXs }` is grouping, typed `string[]?`, and gives `null` or `["a","b"]` in all three engines, so there is no mismatch.

  I disagree with the argument offered for the one row still rejected, the lone `<Box>{if c { as2 } else { null }}</Box>` at `A?[]`, now reporting `found A[]?`. Its premise is that a lone child binds as itself, so the value could be `null` at an `A?[]`. No engine does that at a list-typed content property. All three splice the lone child. The same body at `object?[]` binds `[null]` identically in the interpreter, the IR runtime and generated JavaScript, and a lone `A?` child at `A?[]` binds `[null]` in the interpreter and IR runtime. Binding as itself is only the *checker's* arity-one path. The rejection is conservative, not unsound, so it does not reopen RF27, but it is filed as RF28 because it contradicts a `SHALL`.

## New Findings Discovered During 2026-09-21 02:20 Verification

### ✅ Resolved - RF28 A lone nullable-sequence child is rejected at a list-typed content property that accepts the same child beside a sibling
- **Severity:** Low
- **Evidence:** `sequence-model` says element body content is a collecting position, and that "how many items are written alongside it SHALL make no difference to how one item is treated." The checker's arity-one content path compares the lone child's *own* type with the declared type, while the multi-child path compares its *contribution*. After the rule change and RF27, those two agree for every item type but one: a nullable sequence, whose own type is `T[]?` and whose contribution is `T?`. So:

  | body at `content items:A?[]` | checker | engines |
  | --- | --- | --- |
  | `<A n=9 />{if c { as2 } else { null }}` | accepted | `[A, null]` in all three |
  | `{if c { as2 } else { null }}` alone | **rejected**: `found A[]?` | (at `object?[]`: `[null]` in all three — *stale: measured before RF29; now `[null]` in the interpreter and IR runtime and `null` in generated JavaScript, the RF15 gap*) |

  The same child is accepted beside a sibling and rejected alone, although all three engines would bind `[null]` or `[A, A]`, both valid `A?[]` values. The rejection is conservative, so it never lets a wrong value through. But it is the exact arity asymmetry the `SHALL` rules out, and it hits the documented `else { null }` idiom when it is the whole body.
- **Recommendation:** at a list-typed content property, check the lone child the way the multi-child path does: `Type::array(item_contribution_of(child))` against the declared type. For every type except a nullable sequence this equals the lift already applied, so nothing else moves. Alternatively, if lone-child-binds-as-itself is intended for the checker, say so in `sequence-model` as an explicit exception to the arity sentence. Add the lone form at `A?[]` as a test either way.
- **Status:** left open for a decision; the fix is not high-confidence. Testing it showed that the premise we were both working from is only half right. The verifier is correct that a lone child at `object?[]` comes out as `[null]` in the interpreter and IR runtime, and I was wrong to say a lone child simply "binds as itself". (Its original "in all three engines" was measured before RF29; since RF29, generated JavaScript gives `null` there, the RF15 gap.) But the engines do not share one rule for a lone child:
  - **Interpreter:** binds it as itself, then coerces it to the property's type. That lifts `null` to `[null]` at `object?[]` and `A?[]`, and keeps it `null` at `A[]?`.
  - **IR runtime:** splices a lone child at any list-typed property. At `A[]?` it builds `[null]` and then throws `Expected Box.items[0] to be an object`.
  - **Generated JavaScript:** emits it as itself, as HEAD did.

  So at an `A[]?` property the three give `null`, a throw, and `null`. HEAD, rebuilt to check, gives exactly the same, so this predates the change.

  The recommended checker fix — checking a lone child as `Type::array(item_contribution_of(child))` — assumes the splicing reading. It would accept this finding's case at `A?[]`, where generated JavaScript would then bind `null` because it does not lift a single item (RF15). It would also newly reject `<Box>{maybeList}</Box>` at `A[]?`, where the interpreter and JavaScript agree.

  The question underneath RF28 is what a lone content child *means*, and the engines already disagree about it for nullable sequences. The question also disappears if `null` is removed and `?` becomes an occurrence indicator, since there is then no `T[]?` to be absent. Recommend deciding it together with that change, or with RF15, rather than here.

- **Verification of the status (Fix Pass 8):** the reasoning holds, and my recommendation was wrong. Re-measured after RF29: at `A[]?`, a lone absent nullable child gives `null` in the interpreter and generated JavaScript and throws in the IR runtime, identical at `HEAD`. My proposed check — the lone child as `Type::array(contribution)` — would compare `A?[]` against `A[]?` and newly reject `<Box>{maybeList}</Box>`, a program on which two engines agree and produce a valid value. At `object?[]`, RF28's own shape now gives `[null]` in the interpreter and IR runtime but `null` in generated JavaScript. The `[null]`-in-all-three measurement I based RF28 on was taken against the pre-RF29 JavaScript that spliced lone children, which is the defect RF29 removed. So RF28's premise was partly an artefact of RF29. The underlying observation stands: the checker's arity-one path and the multi-child path differ for a nullable sequence, and the engines share no single lone-child rule. But no one-line checker change fixes it, since any rule the checker picks disagrees with at least one engine. Correctly left open for the author, and better framed as "decide the lone-child binding rule for all engines" than as a checker bug. It sits next to RF15, since two of the three divergences run through the single-item lift.
- **Resolution:** deferred to the null-removal change, decided with the author. The inconsistency is real — a lone child is judged by its own type and several children by what each contributes, and the two differ only for a nullable sequence — but no checker-only change closes it, because the engines share no lone-child binding rule and did not at HEAD either (interpreter: bind as itself, then coerce; IR runtime: splice; generated JavaScript: bind as itself). Under occurrence semantics `T?[]` is gone and `T[]?` collapses to `T[]`, so no type is left whose own type and contribution differ and the checker's two paths stop disagreeing; the engines' lone-child rule then has to be chosen once and applied in all three. Recorded in `specs/future.md` under "Removing `null`: `?` as an occurrence indicator", as the third behaviour that change must settle, including the IR runtime's throw at an `A[]?` property and the instruction to cover it with a three-engine case that includes the IR runtime. The rejection it leaves in place is conservative: it never admits a value the engines compute wrongly.

## Fix Pass 1

Eleven findings fixed (RF1–RF6, RF9–RF13); RF7 and RF8 left open as design decisions, with the
reasoning under each. Gates re-run after the fixes, all passing:

- `cargo test --workspace --locked` — pass, including 7 new tests (3 in `flat_sequences.rs`,
  4 in `nx-interpreter/tests/arrays.rs`).
- `cargo clippy --workspace --all-targets` — clean; `cargo fmt --all` applied.
- `npm test` in `runtime/typescript` — 307 assertions pass, including the new nested-conditional
  three-engine case; `npm run build` still leaves `dist` byte-identical.
- `pnpm run check-examples` in `sites/playground` — 20/20 compile and evaluate.
- `npx astro build` in `docs` — 23 pages; all 8 `nx` blocks in `sequences-and-objects.md` still run.
- `pnpm run test:grammar` in `src/vscode` — 262 pass.
- `openspec validate flat-sequences --strict` — valid.
- Corpus diff re-run with the two-source methodology RF4 asks for: `HEAD` sources under a detached
  `HEAD` build against working-tree sources under this build, `nxlang run --format json` over all
  39 `.nx` files — byte-identical, 14 evaluating and 25 failing identically for reasons that
  predate the change.

## Fix Pass 2

RF5 reopened by verification and RF14 both fixed; RF15 left open, with the reasoning under it.
RF7 and RF8 unchanged. Gates re-run after this pass, all passing:

- `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets` (clean),
  `cargo fmt --all --check` (clean).
- `npm test` in `runtime/typescript` — 308 assertions, including the new empty-body case; `npm run
  build` leaves `dist` byte-identical.
- `pnpm run check-examples` (20/20), `npx astro build` (23 pages), `pnpm run test:grammar` (262),
  `openspec validate flat-sequences --strict`.
- Corpus diff re-run against the same `HEAD`-build baseline: byte-identical for all 39 files.

## Fix Pass 3

RF5 (reopened a second time) and RF16 fixed. RF7, RF8 and RF15 unchanged, still open by intent.
Gates re-run, all passing: `cargo test --workspace --locked`, `cargo clippy --workspace
--all-targets` (0 warnings), `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript`
(308); `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`
(23 pages); `openspec validate flat-sequences --strict`; and the corpus diff against the same
`HEAD` baseline, byte-identical for all 39 files.

On the verification's open question about imported aliases: it is now partly answered and partly
still unmeasured. The span half is answered and guarded — `local_type_alias_spans` exists exactly
so an imported alias's span is never moved onto a diagnostic in this file, with a test asserting
every reported span falls inside the source under check. The dedupe half is still unmeasured end to
end: `nxlang run` builds no workspace from a sibling directory, and `nx-types`'s `TypeCheckSession`
checks each file independently rather than linking them, so neither reaches an imported alias. The
behaviour should be the same improvement it is locally, since imported aliases sit in the same
`type_aliases` map and resolve through the same cache, but it is reasoning rather than measurement.

## Fix Pass 4

RF17 fixed, with a regression test on the multi-module route the verification found. RF7, RF8 and
RF15 unchanged, still open by intent — the three design decisions.

Gates re-run, all passing: `cargo test --workspace --locked`, `cargo clippy --workspace
--all-targets` (0 warnings), `cargo fmt --all --check` (clean); `npm test` in `runtime/typescript`
(308); `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`
(23 pages); `openspec validate flat-sequences --strict`; corpus diff against the same `HEAD`
baseline, byte-identical for all 39 files.

The verification's multi-module question is now closed on both halves. `nxlang typegen <library
directory>` links modules and reports across files, and `build_library_artifact_from_directory`
with `write_library` is the same route from a test, which is what the new test uses. A linking
`nx-types` harness would still be worth having, since `check_str` is single-module and every alias
test there is therefore local-only.

## Rule Change: An `if` With No `else` Carries An Implicit `else { }`

Decided with the author after Fix Pass 4, and implemented as task section 11. RF7 and RF8 were the
last open findings, and on inspection they, RF1 and RF2 shared one cause: a conditional with no
`else` had a *type* (`T?`) and, separately, a *contribution* (`T`, or nothing), and each engine
carried its own implementation of the second. Every one of those four findings was an engine
getting the contribution wrong.

The rule: the missing branch is `{}`. So the conditional's type is the join of its branches with
`never[]`, which the new lifting join makes `T[]`, and its value is the empty sequence when no
branch is taken. The splice rule then does every job with no conditional-specific code anywhere,
and nesting is free because a sequence never contains a sequence. Deleted as a result:
`open_conditional_contributions`, `open_conditional_type` and the arity-one case in
`check_content_binding` (checker); the conditional arms of `eval_item_into` (interpreter); the
`takenBranch` case in `evalItemInto` (IR runtime); `emit_item` and `contributes_conditionally`
(codegen).

What it costs, accepted deliberately: a nullable site needs the `else` written — `if c { "x" }` is
a `string[]`, so `v:string?` needs `if c { "x" } else { null }`. No tailored diagnostic, since the
author intends to remove `null` from NX and a hint would teach a spelling that is going away.

Considered and rejected: typing the conditional `T?` and adding an implicit `T? → T[]` conversion.
It fails unannotated — `let xs = { "a" if c { "b" } }` would infer `string?[]` and hold a null
item, which is RF1 again by design — because today's `T?` means "one value, possibly null", and
that collides with "zero or one items" at `T?[]`. It becomes the right answer once `?` is an
occurrence indicator, which is recorded in `specs/future.md` ("Removing `null`: `?` as an occurrence
indicator") together with a stricter rule for inferred `object` ("Inference invents `object` where
it should ask"). Under that future design `T[]` narrows to `T?` with no change in behaviour.

Gates, all passing: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets`
(0 warnings), `cargo fmt --all --check`; `npm test` (309, including a new three-engine case) with
`dist` byte-identical after `npm run build`; `pnpm run check-examples` (20/20); `pnpm run
test:grammar` (262); `npx astro build` (23 pages), all 8 `nx` blocks on `sequences-and-objects.md`
and the 3 blocks this change adds to `if.md` running and matching their prose — the page's other 6
are reference fragments (bare syntax forms, undeclared names) that fail identically at HEAD — and no
other docs block newly failing; `openspec validate flat-sequences --strict`; and the two-source
corpus diff, byte-identical for all 39 files.

To check that the diff is not vacuous, the checker was temporarily instrumented to report every
branch the join lifts — which covers both an implicit `{}` and a *closed* conditional mixing an item
arm with a sequence arm, the case an earlier probe of implicit fills alone could not see — and run
over the same 39 files. It reported none, and a sanity case (`if c { 1 } else { xs }`) confirmed it
fires. The only conditional without an `else` arm that any sweep has found is the exhaustive
`if loadState is { … }` at `examples/nx/types.nx:80`, which has no uncovered path and keeps its
scalar type (`an_exhaustive_union_match_is_not_made_a_sequence`). The limit on that evidence: a
file that fails before or during analysis is covered by the probe only as far as inference got.
That includes `types.nx`, whose line 80 is therefore pinned by the test, and the 20 playground
examples, which need the DrawnUI library a bare `nxlang run` does not supply; for those, the
verifier's sweep of every `if` line and multi-line arm is the evidence.

## Fix Pass 5 (after the Rule Change)

RF18, RF19, RF21, RF22 and RF23 fixed; RF20 resolved into RF15's split-out change, whose scope now
names the IR runtime too. Nothing open.

The two High findings shared one gap: the rule change made a conditional's *type* a sequence without
making its *value* one. RF18's fix closes it at the source rather than in each engine — a lifted
branch becomes a one-element array literal, which every engine already evaluates correctly — so the
four engines agree by construction instead of by four separate implementations, which is the lesson
the Rule Change itself drew from RF1, RF2, RF7 and RF8.

Gates, all passing: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets`
(0 warnings), `cargo fmt --all --check`; `npm test` (310, including a new three-engine case that
iterates a taken conditional) with `dist` byte-identical after `npm run build`; `pnpm run
check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build` (23 pages) with the 8 + 3
blocks the change owns running; `openspec validate flat-sequences --strict`; and the two-source
corpus diff, byte-identical for all 39 files.

## Fix Pass 6

RF24, RF25 and RF26 fixed. Nothing open. RF24 was the one blocker the verification named, and a
regression in the exact idiom the rule change tells authors to use; it is fixed by keeping the
untyped null literal out of the lift, so `else { null }` makes a nullable sequence beside a sequence
just as it makes a nullable scalar beside a scalar.

Gates, all passing: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets`
(0 warnings), `cargo fmt --all --check`; `npm test` (311) with `dist` byte-identical after
`npm run build`; `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`
with the 11 blocks the change owns running; `openspec validate flat-sequences --strict`; and the
two-source corpus diff, byte-identical for all 39 files.

## Fix Pass 7

RF27 fixed. Nothing open. A static-typing fix only — the engines already produced the flat value;
the checker now types it the way they compute it, so the change's own rule that no inference result
is a sequence of sequences holds for nullable sequences too. The verification's two new questions
(typed versus untyped null, and what an absent `T[]?` contributes) are both consequences of `?`
meaning "nullable", so both are recorded in `specs/future.md` beside the null-removal direction
rather than decided here.

Gates, all passing: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets`
(0 warnings), `cargo fmt --all --check`; `npm test` (311) with `dist` byte-identical after
`npm run build`; `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`
with the 11 blocks the change owns running; `openspec validate flat-sequences --strict`; and the
two-source corpus diff, byte-identical for all 39 files.

## New Findings Discovered During Fix Pass 8

### ✅ Verified - RF29 Generated JavaScript splices a lone content child, turning an absent nullable sequence into `[null]` where it used to agree with the interpreter on `null`
- **Severity:** Medium
- **Evidence:** Found while testing RF28, and introduced by this change. `emit_content_value` sent a lone content child through `emit_sequence_items` whenever `item_can_splice` said its type could hold a sequence — added so that a conditional's `(c ? item : [])` became a list. A nullable sequence can hold a sequence, so `<Box>{if c { as2 } else { null }}</Box>` at `items:A[]?`, untaken, emitted `[(c ? as2 : null)].flat()` = `[null]`. The interpreter binds `null`. HEAD, rebuilt to check, emitted the lone child as itself and gave `null` in both.
- **Recommendation:** emit a lone content child as itself again. RF18 makes the splice unnecessary: a conditional whose type is a sequence already evaluates to a list, because analysis lifts its item branch at the source.
- **Fix:** `emit_content_value` emits a lone child as itself, exactly as HEAD did. The `<Box>{}</Box>` peephole goes with the splice, since `{}` emits `[]` as itself. Measured:
  - The lone absent `A[]?` case is back to `null` in the interpreter and generated JavaScript.
  - Lone taken and untaken conditionals and `<Box>{}</Box>` at `A[]` still give `[A]`, `[]` and `[]` in all three engines.
  - The `items: []` codegen assertion still passes.

  A new `emitted-ir.test.mjs` case compares the interpreter and generated JavaScript, with a comment saying why the IR runtime is not compared: its lone-child throw predates this change and is described under RF28.

- **Verification (Fix Pass 8):** confirmed. `emit_content_value`'s lone-child path is back to `HEAD`'s exactly: `content.len() == 1` emits the expression as itself, and only the multi-child path differs, which is the intended splice. The three points you asked me to attack:
  1. **Can a lone child's *value* still be a bare item while its *type* is a sequence?** Not through anything this change introduced. Every way a lone child gets a sequence type now yields a list value at the source: a conditional, because RF18 lifts any branch the join made a sequence (taken open, nested open-in-open, both-taken, closed with an empty arm and closed item/sequence all give lists); a `for`, a braced value and `{}`, because they are array expressions. The only bare-item-typed-as-sequence values left are variables, fields or returns annotated `T[]` and bound to a single item — the RF15/RF20 gap, byte-identical at `HEAD` and resolved into its own change. Removing the guard therefore reopens nothing.
  2. **Engine agreement.** Nineteen lone-child shapes, all three engines. Taken, untaken, untaken with a non-empty default, nested, both-taken, closed with an empty arm either way, closed item/sequence, `{}`, `{}` with a default, `{xs}`, a `for`, a zero-iteration `for` and a multi-item brace all agree, with the values their types say (`[]` for every untaken or empty case, never the default and never `null`). Three shapes still diverge, and all three do so byte-identically at `HEAD`, rebuilt with its own CLI and TS runtime: a lone absent nullable sequence and the lone `else { null }` idiom at `A[]?` (interpreter and JS `null`, IR runtime throws `Expected Box.items[0] to be an object.`), and a lone single child element at `A[]` (JS binds the bare record, RF15). On the same suite `HEAD` fails more — the IR runtime throws or substitutes the default for `{}` and a zero-iteration `for` — which RF14 fixed.
  3. **Anything else the revert changed relative to `HEAD`?** No. The lone path is textually `HEAD`'s, and the removed `{}` peephole is unnecessary because `{}` emits `[]` through `emit_sequence_items` with no `.flat()`, which the probes confirm.

  The new test's exclusion is honest: it compares the interpreter and generated JavaScript for a lone absent nullable sequence at `A[]?`, and leaves out the IR runtime, whose throw I reproduced on `HEAD` for both the plain `{m}` form and the idiom.

## Fix Pass 8

RF28 is left open for a decision, with the reasoning under it. RF29 was found while testing RF28, and is fixed.
Gates, all passing: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets`
(0 warnings), `cargo fmt --all --check`; `npm test` (312) with `dist` byte-identical after
`npm run build`; `pnpm run check-examples` (20/20); `pnpm run test:grammar` (262); `npx astro build`
with the 11 blocks the change owns running; `openspec validate flat-sequences --strict`; and the
two-source corpus diff, byte-identical for all 39 files.

## Outcome

All 29 findings are closed: 26 verified, 3 resolved. RF15 (the single-item lift missing in
generated JavaScript and, per RF20, the IR runtime) and RF20 are split into their own change; RF28 is
deferred to the null-removal change and recorded in `specs/future.md`. Nothing is open and nothing
awaits verification, so the change is ready to archive.

Two directions this review turned up are recorded in `specs/future.md` rather than taken here:
"Removing `null`: `?` as an occurrence indicator", which now also carries RF28 and the two
nullable-sequence behaviours from RF24 and RF27, and "Inference invents `object` where it should
ask". Also left for later, as recorded under `## Questions`: documenting the post-analysis rewrite
contract for host embedders, and the fiddle catalogue sweep before the next release.

## Questions
- ~~Is the arity-one asymmetry in RF7 intended?~~ — **answered by the rule change:** there is no asymmetry left to intend, since the conditional is a sequence on its own account. Original question: `sequence-model` says "a multi-item braced value" while `conditional-result-types` and the new `if.md` say "a braced sequence". Recorded rather than asked, per the review instructions; the specs need one answer either way.
- ~~Task 10.1 explicitly scopes the fiddle catalogue into the sweep~~ — **answered:** the task is narrowed to this repository's corpus, and the catalogue is swept as a separate change before the next release that ships these rules. Original question: Task 10.1 explicitly scopes the fiddle catalogue into the sweep, and `review-notes.md` states it was **not** swept because it lives in `DrawnUi.FiddleEngine` and pins a published release. That is a reasonable answer, but the task is marked `- [x]` with part of its stated scope skipped — should the task text be narrowed, or the catalogue swept before the next NX release that ships this rule?
- ~~RF8 is pre-existing behaviour~~ — **answered:** fixed inside this change, by the lifting join. Original question: RF8 is pre-existing behaviour that the new `sequence-model` wording arguably now requires changing. Should it be fixed inside this change, or deferred with a note in the spec?
- ~~RF14 and RF15~~ — **answered:** RF14 fixed here; RF15 split into its own change. Original question: RF14 and RF15 are both pre-existing gaps that this change's own new requirements are the first to pin. RF14 has a `conditional-result-types` scenario that fails today, so it looks in scope; RF15 is a whole missing coercion layer in one target and may need its own change. Which of the two belongs here? (RF14 is now fixed and verified, so the open half of this is RF15.)
- `constructComponentDescriptor` now has no way for a host to say "a body that produced nothing": passing `[]` and passing nothing are the same, and both take the declared default. That matches its behaviour before the fix and the note argues it deliberately, so nothing regressed — but if a host embedder ever needs the empty-list binding, the API needs a way to express it. Recorded, not acted on.
- ~~Multi-module behaviour of the alias cache could not be exercised end to end~~ — **answered in Fix Pass 3 verification.** The route is `nxlang typegen <library directory> --language typescript`, which loads and type-checks every module in a library and reports across files. The layout is the catch: `import "./x"` resolves `x` as a **directory** of `.nx` files, not as a sibling file — `examples/nx/core/` is the shape, which is why a two-file `lib.nx` + `main.nx` layout fails with "Missing workspace module or loaded library". With that, all the imported-alias paths were measured, and the cache does report an imported bad alias once per consuming module instead of once per use. A `nx-types` harness that links modules would still be worth having, since `check_str` is single-module and the span test written for RF5 can only exercise local aliases.
- ~~RF17~~ — **answered:** fixed in this change, with an `nx-cli` library-harness test. Original question: RF17 is a cross-module diagnostic-span regression with no test that can reach it from `nx-types` today. Should it be fixed in this change (three lines, mirroring the gate RF5 already added) with a CLI-level or `nx-cli` test, or split out with the multi-module harness question above?

- ~~RF18's fix changes what a conditional evaluates to~~ — **answered: wrap at the source.** Restricting the lifting arm to collecting positions would reintroduce the position-dependent rule the author explicitly rejected, and the author's own statement of the rule was that `if c { 1 }` is an `int[]` of arity zero or one — which `[1]` is and a bare `1` is not. So this implements a decision already made rather than taking a new one. Original question: RF18's fix changes what a conditional *evaluates* to in value position (`[1]` rather than `1` when a branch is taken). That is what the type already promises and what the interpreter produces today wherever an annotation is present, but it is a runtime-visible change to every engine and a design decision in its own right. Wrap at the source, as recommended — or restrict the lifting arm to collecting positions, which reintroduces the position-dependent rule the author rejected?

- The post-analysis rewrites — literal conversions, widenings, string conversions, and now join lifts — are required for correct evaluation, and direct-HIR execution without analysis skips all of them (a taken `if c { 1 }` is a bare `1` there). That is the existing contract rather than a defect, and the interpreter test suite now has an `execute_analyzed` harness for it; is it written down anywhere a host embedder would read it?

- RF24's exclusion is keyed on the *type* of the null, so the same runtime value takes two shapes depending on an annotation: `let n = null` beside `xs:string[]` gives a nullable sequence that is `null`, while `let n:string? = null` beside it gives `[null]`. Both are sound and both follow the stated rule, but an author who annotates a variable will see a different result. Worth a sentence in `conditional-result-types`, or acceptable as is until `?` becomes an occurrence indicator?
- On documenting the post-analysis-rewrite contract for host embedders: agreed with recording rather than fixing it here — it predates this change and every rewrite before join lifts already depended on it. It belongs in the embedding documentation, not in flat-sequences.
- For RF27, the recommended contribution of an absent `T[]?` is a null item, because that is what all three engines already produce. The alternative — an absent sequence contributes nothing, as an empty one does — is arguably closer to "a sequence contributes its items", but it would change runtime values in every engine. Which should the spec say?

## Summary
The change is substantial, coherent and well covered: the validator, resolver, type-argument rule, `for`/braced-value/content splicing, the unit type's removal, the generators, the docs and the grammar prose all landed, and every verification gate I re-ran independently passes (workspace tests, TS runtime tests, playground `check-examples`, docs build, VS Code grammar tests, `openspec validate --strict`, and every code block on the rewritten sequences page).

The central claim — that the four engines agree on the contribution rule — holds for the one-level cases the tests pin, and the new three-engine test in `emitted-ir.test.mjs` is a genuinely good gate. It does **not** hold one level deeper: a conditional nested inside a conditional (RF1) still produces the `null` item the proposal set out to eliminate, in the interpreter alone, and the checker widens the same form's element type (RF2). Those two are the blocking findings. Beyond them, the diagnostics for a nested sequence fire two or more times with misleading spans (RF5), a spliced numeric sequence is not widened to its own inferred item type (RF6), the documented collecting positions do not match the implementation at arity one (RF7), and `docs/drawnui-proposal/ui/ui.nx` carries accidental scratch edits that the review notes then misreport as pre-existing (RF3, RF4). The remaining findings are polish.

### After Fix Pass 1 (verified 2026-09-20)

Ten of the eleven claimed fixes verify: RF1, RF2, RF3, RF4, RF6, RF9, RF10, RF11, RF12, RF13. The blocking pair is genuinely closed — a conditional nested in a conditional now contributes an item or nothing in the interpreter, the IR runtime and generated JavaScript alike, at two and three levels, with the outer `else` taken or not, and the checker no longer widens it — and the two fixes that went beyond their recommendations both hold up under targeted probing: `emit_item` recursing into a closed conditional's branches introduces no new divergence, and `item_can_splice` never wrongly answers "cannot splice" for an alias-typed sequence, an `object` item, a call, a field access, a `for`, a conditional, a type parameter or an untyped item.

RF5 is reopened. Its three measured cases are fixed and the `type_from_type_ref_in_quietly` rollback is watertight, but the dedupe is keyed per message rather than per alias, so `type X = Names?[]`, `type Bad = <Box T=int[]/>` and — as a regression against `HEAD` — `type Bare = Box` each still report once per use plus once at the declaration. One per-alias resolution cache would close all four messages at once.

RF7 and RF8 remain open by intent, and their reasoning stands up: RF7 is a genuine spec disagreement rather than a defect, and RF8's runtime half is now fixed with the checker half erring only toward rejection. Two new findings came out of the three-engine probing, both pre-existing but first pinned by requirements this change adds: the IR runtime substitutes a content property's declared default for an empty content list (RF14), which a `conditional-result-types` scenario explicitly forbids, and generated JavaScript never lifts a single item to a one-element list at a sequence-typed site (RF15).

### After Fix Pass 2 (verified 2026-09-20)

**RF14 verifies** and is the substantive close of this pass: all three engines now bind `[]` for an untaken conditional as the whole body, for `<Box>{}</Box>` and for a zero-iteration `for`, each against a non-empty declared default, while an absent body still takes the default. The two paths the `content.length === 0` early return used to guard — a record with no content field, and a non-list content property — are unreachable from valid source, the checker rejects both; `content.length !== 1` only changes the zero case; and `constructComponentDescriptor` is behaviour-preserving, measured directly. The new test gives each element a standalone body and covers all four forms.

**RF5 is reopened a second time, for a much narrower residue.** Every count the fix claims reproduces, the `type Bare = Box` regression against `HEAD` is gone, the quiet-resolution rollback is correct for the cache and in fact unreachable for a local alias, an alias target is resolved with `scoped = false` so the cache cannot freeze a context-dependent answer, and the two-mistakes-two-reports judgement is right. What remains is that `validate_local_type_aliases` resolves each alias's target through a path that never reads the cache, so an alias already resolved as a side effect of an earlier one in the loop is walked twice: the same three declarations report **2** errors or **1** depending only on the order they are written in. Skipping an already-cached alias in the loop, plus reporting at `alias.span` from inside `resolve_named_type`, closes it and makes the diagnostic order-independent.

One new finding, RF16 (Low): a self-referential alias (`type A = A[]`) reports a spurious "a sequence cannot contain sequences" beside its cycle error, because cycle recovery returns `Type::Error` for the inner name and the enclosing `[]` wraps it into an `Array(Error)` that the nesting check then believes. `HEAD` reports neither, so the eager alias pass surfaced rather than caused it. RF7, RF8 and RF15 stay open by intent, and RF15's "this is its own change" reasoning is sound: the missing lift is byte-identical at `HEAD` and absent at every binding site in that target, not only the ones sequences touch.

### After Fix Pass 3 (verified 2026-09-20)

**Both claimed fixes verify.** RF5's third attempt is right where the first two were not: the nested-sequence report is now one per mistake, on the alias that wrote it, and independent of declaration order — `Alias2` before `Rows` and `Rows` before `Alias2` both give one error on `Rows`. The `local_type_alias_spans` gate is the correct shape and it holds: using `nxlang typegen` over a library directory (the multi-module route that was missing from the last two rounds) I put a bad alias at line 32 of one module and consumed it from a **one-line** module, and the consumer's diagnostic lands on its own line 1, never on line 32 of the other file. Shadowing, record-field consumption and an unreferenced imported alias all behave. RF16 is closed for both spellings and both suffix arms, with no wrong collateral — the follow-on mismatches it suppresses were all cascades off an already-reported error, and independent diagnostics still fire.

**One new finding, RF17 (Medium).** Attacking the gate turned up the place it was not applied. `resolve_named_type`'s `type-alias-cycle` arm still reports at `TypeAliasInfo.span` unconditionally, which for an imported alias is the declaring module's byte offset. The eager alias pass changed which end of a cross-module cycle is entered first, so this is now reachable and is a regression against `HEAD`: `HEAD` puts each cycle message on its own alias, while the working tree puts one on a filler comment in the wrong file and the other **past the end of a two-line file**. The fix is the same three lines the nesting message already has.

RF7, RF8 and RF15 remain open by intent and unchanged. The change now stands at thirteen findings verified and four open: RF7, RF8 and RF15 are deliberate decisions awaiting an answer rather than defects, and RF17 is the one actionable defect left.

### After Fix Pass 4 (verified 2026-09-20)

RF17 verifies, and the extra level the fix needed was the right diagnosis: gating the span alone put every diagnostic back in the right file but left it naming the other module's alias, because the eager pass entered the walk at the target rather than at the alias. Seeding `seen` with the alias's own name makes the eager pass and a use site take the same walk, and the output on the case that exposed the finding is now byte-identical to `HEAD` — `'A'` on `a.nx:26:1`, `'B'` on `b.nx:1:1`. Three-module cycles, a cycle reached only from a record field, the `scoped` change and the `seen` seeding all hold up, and re-running the RF5 and RF16 suites in full found no movement in either.

**The change now stands at fourteen findings verified and three open, and none of the three is a defect.** RF7 is a spec disagreement about whether a one-item braced value is a collecting position, RF8 is a checker refinement that only ever rejects where the runtime would have coped, and RF15 is a missing coercion layer in generated JavaScript that is byte-identical at `HEAD` and belongs in its own change. Every gate passes independently, including a two-source corpus diff over 39 files against a freshly built `HEAD`, and the flat-sequence model now behaves identically in the checker, the interpreter, the IR runtime and generated JavaScript for every case I have been able to construct across four rounds of probing. Subject to an answer on RF7 and RF8, this is ready to archive.

### After the Rule Change (verified 2026-09-21)

The implicit `else { }` is the better rule and most of it verifies. RF7 and RF8 are closed by it; RF1, RF2, RF9 and RF13 re-verify against code that no longer contains any of their original fixes; RF6 widening still holds; and every collecting-position case agrees across the interpreter, the IR runtime and generated JavaScript with no conditional-specific code in any of them. The emitter's braced-literal exception is sound: it only fires on an `Array` literal, which splices its own elements internally, so a conditional cannot slip past it. RF15's split is right, but the change it became must also cover the IR runtime (RF20).

**The rule is not finished, though, and two new findings are High.** It changed the *type* of a conditional without changing its *value*: `if c { 1 }` is now an `int[]` but still evaluates to a bare `1` when taken in every engine. Where an annotation happens to lift it the programs work; where none does, programs that `HEAD` rejected are now accepted and then crash in the interpreter and the IR runtime or silently compute the wrong thing in generated JavaScript — the documentation's own `let tags:string[] = { if c { "new" } }`, iterated, yields `["n!","e!","w!"]` there (RF18). And the new lifting arm discards a nullable sequence's `?`, letting `null` through to a `string[]` site that `HEAD` rejected (RF19). Both come from the lifting arm in `common_supertype`, both have one-screen repros, and neither is caught by the new three-engine test because it exercises the taken conditional only at a record field.

Four more are smaller: RF20 (pre-existing IR gap, now on the main path), RF21 (stale comments), RF22 (evidence wording), RF23 (the `found list string[]` message). The change is **not** ready to archive until RF18 and RF19 are fixed; the recommended fix for RF18 — record a lifted branch as `widened_joins` records a widening, and wrap it in lowering beside `Expr::Widen` — keeps the author's type rule intact and makes value and type agree in every engine.

### After Fix Pass 5 (verified 2026-09-21)

Five fixes verify — RF18, RF19, RF21, RF22, RF23 — and RF20's resolution holds. RF18's design is right: wrapping a lifted branch at the source as a one-element array literal makes the four engines agree by construction, and every repro that crashed, threw or iterated a string's characters now gives the value its type promises. Lift and widening compose, and the tightened three-engine tests compare whole values.

**One new High finding, RF24, blocks archiving.** The lift condition treats a `null` branch as an item, so beside a sequence branch `else { null }` is lifted into `[null]` and the join becomes `T?[]` instead of `T[]?`. That rejects `let v:string[]? = { if c { xs } else { null } }`, which `HEAD` accepts — and it is the exact idiom the rule change tells authors to write for a nullable site. The fix is a guard on the null literal in the lifting arm and in `record_join_lifts`. RF25 and RF26 are small. With RF24 fixed, nothing I have found blocks archiving.

### After Fix Pass 6 (verified 2026-09-21)

RF24, RF25 and RF26 verify. The `else { null }` idiom now works beside a sequence in every engine, and the null-literal exclusion is exactly as narrow as intended: only an untyped null defers to the nullable arm, and every typed nullable value still lifts as an item. RF26's test now genuinely depends on RF6's fix.

**One new Medium finding, RF27.** Attacking the exclusion led one step further: a nullable-sequence item in a collecting position — including the `else { null }` idiom placed in a braced list, a content body or a `for` — contributes itself rather than its elements, so inference builds `string[]?[]`. That is a nested sequence type, which the change's own spec forbids, and it cannot be written, so no annotation accepts it — while every engine produces a flat value. The fix is one arm in `item_contribution_of`. It is a static-typing defect, not a runtime one, but it violates a `SHALL` the change introduces, so I would fix it before archiving. Beyond RF27 I have found nothing open; RF15 and RF20 remain resolved into their split-out change.

### After Fix Pass 7 (verified 2026-09-21)

RF27 verifies. A nullable-sequence item now contributes `T?` in every collecting position. The nested, unspellable `T[]?[]` is gone, the static type matches what all three engines compute, and widening and already-nullable elements behave. The questions about typed versus untyped null and the absent `T[]?` are recorded in `specs/future.md`, and the post-analysis-rewrite contract is recorded rather than fixed. Both are reasonable dispositions, so I have removed nothing from `## Questions`.

**One new Low finding, RF28.** The fix passes argued that the lone-child rejection at `A?[]` is sound because a lone child binds as itself. It is sound, but only the checker binds a lone child as itself: all three engines splice it. That leaves an arity asymmetry that `sequence-model` explicitly rules out. It never admits a wrong value, so I would not hold archiving on it if the author prefers to record it. The fix is one line in the arity-one content check. With RF28 either fixed or written into the spec as an intended exception, I have nothing else open.

### After Fix Pass 8 (verified 2026-09-21)

RF29 verifies. Generated JavaScript's lone-child path is `HEAD`'s again, and RF18's source lift genuinely makes the removed guard unnecessary: every lone child that can have a sequence type — conditionals of every shape, `for`, braced values, `{}` — now evaluates to a list, and all three engines agree on each. The three lone-child shapes that still diverge are byte-identical at `HEAD`.

RF28 stays open, and the fix pass's account of it is right where mine was not: the engines have no shared lone-child rule, so no checker-only change can close it. It is a pre-existing cross-engine decision that belongs with RF15. The change now stands at 26 verified, 2 resolved (RF15, RF20), and one open item that is a decision for the author rather than a defect this change introduced. From my side it is ready to archive.
