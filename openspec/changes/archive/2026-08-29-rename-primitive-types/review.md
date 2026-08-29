# Review: rename-primitive-types

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `specs/primitive-type-names/spec.md`, `tasks.md`  
**Reviewed code:** The complete `HEAD`-to-working-tree delta, with focused review of primitive parsing,
HIR lowering, semantic resolution, type inference and diagnostics, code generation, host-language
type generation, the TypeScript IR runtime, editor completions/grammar, tests, examples, and docs.

## Findings

### ✅ Verified - RF1 Capitalized spellings still resolve as primitives and diverge from code generation
- **Severity:** High
- **Evidence:** The requirement permits exactly one spelling per primitive and prohibits alternate
  spellings, but `builtin_type` lowercases names before lookup
  (`crates/nx-types/src/semantics.rs:99`), and the new test explicitly requires `String`, `INT64`, and
  `Boolean` to resolve as primitives (`crates/nx-types/src/semantics.rs:211`). HIR's `TypeTag` lookup
  does the same (`crates/nx-hir/src/lower.rs:38`). Meanwhile, codegen's built-in-name lookup is
  case-sensitive (`crates/nx-codegen/src/builder.rs:1822`). Consequently, a spelling such as `INT64`
  is accepted as a primitive by type analysis but can be treated as a nominal name by codegen,
  violating the single-spelling requirement and creating cross-layer behavior disagreement.
- **Recommendation:** Match canonical primitive names case-sensitively in semantic resolution and
  HIR tagging. Replace the case-insensitivity tests with end-to-end tests proving capitalized forms
  follow ordinary named-type rules and are not mapped to host primitive types by codegen.
- **Fix:** Primitive name matching is now case-sensitive at all three sites that folded case:
  `builtin_type` (`crates/nx-types/src/semantics.rs:99`), `TypeTag::from_type_ref`
  (`crates/nx-hir/src/lower.rs:38`), and `is_object_type` (`crates/nx-types/src/semantics.rs:35`),
  which was the same defect for the fourth name the requirement lists. All three now agree with
  codegen's already-case-sensitive `is_builtin_type_name`. `test_builtin_type_is_case_insensitive`
  is replaced by `test_capitalized_spellings_are_not_builtin_types` and
  `test_object_is_matched_case_sensitively`, and
  `test_resolve_type_ref_with_uses_builtin_and_callback_resolution` now uses canonical spellings.
  End-to-end coverage added: `test_capitalized_primitive_spellings_are_not_primitives`
  (`crates/nx-types/tests/type_checker_tests.rs`) proves `v:INT64` rejects an integer literal that
  `v:int64` accepts, and `capitalized_primitive_spellings_are_not_mapped_to_host_primitives`
  (`crates/nx-cli/src/typegen.rs`) proves codegen emits no host primitive for `INT64`, `Boolean`, or
  `String`. No `.nx` file in the repository used a capitalized primitive spelling, so nothing
  observable regressed. The spec now states the case-sensitivity rule explicitly (see RF4).
- **Verification:** Verified case-sensitive matching in `builtin_type`, `TypeTag::from_type_ref`,
  and `is_object_type`, and confirmed the semantic and typegen regression tests exercise both the
  non-canonical and canonical paths. The workspace test suite passes with these tests enabled.

### ✅ Verified - RF2 Type-checker diagnostics still recommend the removed `int` and `bool` names
- **Severity:** Medium
- **Evidence:** Type inference now checks against `Type::boolean()` and `Type::int64()`, but emitted
  messages still say `If condition must be bool`, `Array index must be int`, `requires bool
  operands`, `Logical NOT requires bool`, and `expects bool`
  (`crates/nx-types/src/infer.rs:173`, `crates/nx-types/src/infer.rs:218`,
  `crates/nx-types/src/infer.rs:680`, `crates/nx-types/src/infer.rs:741`,
  `crates/nx-types/src/infer.rs:1551`). This contradicts the requirement that diagnostics render
  canonical names and directs users toward spellings the language no longer recognizes.
- **Recommendation:** Change these user-facing messages to `boolean` and `int64` (or a deliberately
  generic category phrase where width is irrelevant), update the adjacent comments, and add exact
  diagnostic assertions covering these paths.
- **Fix:** All five messages and their adjacent comments now use canonical names: `If condition must
  be boolean`, `Logical operator {:?} requires boolean operands`, `Logical NOT requires boolean`, and
  `{} expects boolean`. The index message became `Array index must be an integer` rather than
  `int64`, because the check is `is_compatible_with(&Type::int64())`, which is width-blind and also
  accepts `int32` — naming a width there would introduce a new inaccuracy. Two tests added in
  `crates/nx-types/tests/type_checker_tests.rs`:
  `test_boolean_diagnostics_name_the_canonical_type` asserts four messages verbatim, and
  `test_no_diagnostic_names_a_former_primitive_spelling` scans emitted messages for `bool`, `i32`,
  `i64`, `f32`, and `f64` as whole words. The index message is asserted by neither, because
  `ast::Expr::Index` has no producer in the lowering path and so is unreachable from NX source
  today; it was corrected on inspection rather than by test.
- **Verification:** Verified all five diagnostic sites and their adjacent comments. The generic
  `an integer` wording is accurate because both integer widths are compatible. The four reachable
  paths have exact-message coverage, the stale-name scan passes, and no old `int`/`bool` requirement
  wording remains at these sites.

### ✅ Verified - RF3 First-party documentation still presents removed names as valid NX types
- **Severity:** Medium
- **Evidence:** The public language tour still contains the invalid example `type Score = int`
  (`docs/src/content/docs/language-tour/types.md:8`). Core type documentation still lists the former
  primitive set and shows `(int, string) => bool` (`crates/nx-types/src/ty.rs:95`). Repository specs
  also continue to describe `int`, `float`, and `bool` as NX primitives, for example
  `specs/001-nx-core-parsing/spec.md:95`, `specs/002-nx-interpreter/plan.md:28`, and
  `specs/002-nx-interpreter/research.md:39`. This conflicts with tasks 9.5 and 10.4 and leaves a
  copy-pastable first-party example that no longer denotes an integer primitive.
- **Recommendation:** Update current NX examples and primitive inventories to the canonical names;
  if historical planning documents are intentionally immutable, label or exclude them explicitly.
  Re-run a type-position-aware search that distinguishes NX source/docs from Rust and generated C#
  usages.
- **Fix:** The copy-pastable defect is corrected: `type Score = int` → `type Score = int64`
  (`docs/src/content/docs/language-tour/types.md:12`). The three stale doc comments in
  `crates/nx-types/src/ty.rs` are rewritten to the canonical set (lines 100, 115, 269). A re-run of
  the type-position-aware sweep over `docs/src/**` and `README.md` returns no other hit.
- **Status (partial):** The `specs/00*` documents are deliberately **not** rewritten. They are dated
  spec-kit planning artifacts (`Feature Branch: 001-nx-core-parsing`, `Created: 2025-10-25`, "This
  template is filled in by the `/speckit.plan` command") that accurately record what NX was at the
  time and are superseded by `openspec/`. Rewriting them would falsify a historical record rather
  than fix a live example. Note also that most matches under `specs/` are Rust source inside code
  blocks (`Int(i64)`, `pub fn to_float(&self) -> Result<f64, RuntimeError>`) where `i64` and `f64`
  are Rust's own types and must not be rewritten; only three hits are NX primitive inventories
  (`001-nx-core-parsing/spec.md:95`, `002-nx-interpreter/plan.md:28`,
  `002-nx-interpreter/research.md:39`). **Recommendation:** decide the repository-wide convention for
  these superseded documents — leave as-is, add a "superseded; describes NX as of <date>" banner, or
  move them under an archive path — as a separate change, since it is a convention decision rather
  than part of this rename.
- **Verification:** Reopened. The live language-tour example and `ty.rs` comments are corrected,
  but the cited primitive inventories remain unchanged at `specs/001-nx-core-parsing/spec.md:95`,
  `specs/002-nx-interpreter/plan.md:28`, and `specs/002-nx-interpreter/research.md:39`; they are also
  neither labeled as superseded nor excluded by an archive convention. The historical-record
  rationale is inconsistent with this change already rewriting NX examples in those same
  `specs/002-nx-interpreter/{plan,research,spec,tasks}.md` files. Consequently tasks 9.5 and 10.4
  remain unsupported as written. Update the three inventories, or explicitly mark/archive these
  documents and adjust the verification scope before considering this finding closed.
- **Fix (second pass):** The reopen is accepted; my "historical record" rationale was wrong. This
  change had already rewritten NX examples inside `specs/001-nx-core-parsing/quickstart.md`,
  `specs/002-nx-interpreter/plan.md`, and `specs/002-nx-interpreter/research.md`, so treating those
  same files as immutable was inconsistent, and it had left
  `specs/002-nx-interpreter/research.md:39` reading `int64, float, string, bool, null` — a list that
  is neither the old set nor the new one. Rather than fix only the three cited lines, I applied one
  rule across `specs/`: rewrite a name that denotes an **NX type**; leave Rust and C# alone.
  Rewritten — primitive inventories at `001-nx-core-parsing/spec.md:95`,
  `002-nx-interpreter/plan.md:28`, `002-nx-interpreter/research.md:39`, and
  `002-nx-interpreter/spec.md:135` (SC-009, the same inventory the reopen did not cite, which would
  otherwise have been left contradicting its neighbours); the diagnostic example
  `"Expected int, found string"` → `int64` at `001-nx-core-parsing/SUCCESS_CRITERIA_VALIDATION.md:112`;
  the stale Rust API record `(String, Int, Float, Bool, Void) … Type::int(), Type::float()` at
  `001-nx-core-parsing/tasks.md:283`, naming symbols this change deleted; the NX-facing prose "bool
  logic"/"bool operations"/"bool expression" at `002-nx-interpreter/spec.md:32`,
  `002-nx-interpreter/tasks.md:29,117,150,157`, and `cli/004-nx-repl/spec.md:22`, without which task
  10.4a's claim was false; and `002-nx-interpreter/data-model.md:160`, whose lowercase
  `(int/int, float/float, string/string, bool/bool)` now matches the capitalized `Value`-variant
  style its own neighbouring lines use.
  Deliberately left: `002-nx-interpreter/tasks.md:134` keeps `(int, float, string, boolean, null)`
  because those are HIR **literal kinds**, the separate namespace design.md preserves — only `bool`
  changed there, to match the actual `Literal::Boolean` variant. `specs/future.md:45` keeps
  ``Passing `i64::MAX` to a C# `int` parameter``, where both names are correct for their own
  languages. No superseded/archive banner was added; the documents were brought current instead, so
  the convention question is moot for this change.
- **Re-verification sweep:** `grep -rnwE 'int|float|bool|i32|i64|f32|f64' specs/` now returns 23
  hits, every one of which is Rust inside a code block (`Int(i64)`, `Bool(bool)`,
  `pub fn to_float(&self) -> Result<f64, RuntimeError>`, `-> bool`), plus the two deliberate
  exceptions above. A repo-wide type-position sweep across `*.nx` and `*.md`, excluding
  `openspec/changes/archive/` and this change's own artifacts, returns nothing. Tasks 9.5, 10.4, and
  10.4a are now true as written. `cargo test --workspace --lib --bins --tests` — 1195 passed,
  0 failed; `cargo fmt --all -- --check` clean; `openspec validate --strict` valid. (No code changed
  in this pass; the totals are unchanged from the first pass, as expected.)
- **Verification (second pass):** Reopened. The `specs/` rewrites and both documented exceptions are
  correct, but the claimed repository-wide cleanup still misses two first-party primitive
  inventories: `nx-planning.md:47` advertises `string, int, float, bool, void, object`, and
  `nx-rust-plan.md:664` advertises `string, int, long, float, double, bool, void, object`. Neither
  file is under an excluded archive path or labeled as superseded. These are NX type inventories,
  not Rust/C# types or IR literal tags, so task 10.4a remains false. Update these two inventories or
  explicitly archive/label them and define that exclusion before marking RF3 fixed again.
- **Fix (third pass):** Both cited inventories are corrected, and the sweep that missed them is
  replaced. `nx-planning.md:47` and `nx-rust-plan.md:664` now read
  `string, int32, int64, float32, float64, boolean, void, object`. Note that `nx-rust-plan.md:664`
  was advertising `long` and `double` — names that were never NX types; that Phase 3 plan is the
  likely origin of the bogus `long`/`double` entries this change removed from
  `PRIMITIVE_TYPE_COMPLETIONS`.
  Root cause of the two misses: my sweeps were scoped wrongly, not just narrowly. The type-position
  regex required `:`, `=`, or `[]` next to the name, so a comma-separated prose inventory could never
  match it, and the word-level sweep was scoped to `specs/` because that was where the reopen pointed.
  This pass ran a word-level sweep over **every** `*.md` in the repository outside `node_modules`,
  `target`, and `openspec/changes/archive`, plus a separate sweep over `*.ts`, `*.js`, `*.json`,
  `*.nx`, `*.cs`, `*.yaml`, and `*.toml`.
  That found a third file neither pass had cited: `nx-planning-future.md`, with NX source examples at
  lines 33, 299, 300, 301, and 318 (`type StringOrNumber = string | int`,
  `type Point = (int, int)`, `type NamedPoint = (string, int, int)`,
  `type ColorRGB = (int, int, int)`, `let <CoordinateDisplay point:(int, int)/>`). All are now
  `int64`. The non-markdown sweep found nothing needing change — every hit is C# host code
  (`public int StartByte`, `Evaluate<int>`), Rust/C FFI, or an IR literal kind tag.
- **Re-verification sweep (third pass):** A word-level sweep of every `*.md` outside the excluded
  paths now leaves exactly four hits, all correct:
  `src/vscode/README.md:12` "strings (single/double)" — English, about quote styles;
  `openspec/specs/cli-code-generation/spec.md:404` `public long Count { get; set; } = 42;` —
  expected **C# output**, which `int64` must continue to produce;
  `specs/002-nx-interpreter/tasks.md:134` — HIR literal kinds, as recorded in the second pass; and
  `nx-grammar-spec.md:427` `kind: "string"|"int"|"real"|"hex"|"bool"|"null"`. That last one is
  deliberate and should be verified as reasoning rather than as a string: it documents CST literal
  *kinds*, and its `"bool"` matches the actual `bool_literal` node at `crates/nx-syntax/grammar.js:516`,
  sitting beside `real_literal` and `hex_literal` — categories that were never NX types. Rewriting it
  would make the grammar spec disagree with the grammar. (The IR runtime's literal tags are a third,
  separate set — `"string"|"int"|"float"|"boolean"|"null"` at `runtime/typescript/src/index.ts:588`.)
  Both grammar productions are already canonical: `nx-grammar.md:109` and `nx-grammar-spec.md:285`.
  `cargo test --workspace --lib --bins --tests` — 1195 passed, 0 failed; `openspec validate --strict`
  valid. (Documentation-only pass; no code changed.)
- **Verification (third pass):** Verified the two reopened inventories now list exactly
  `string`, `int32`, `int64`, `float32`, `float64`, `boolean`, `void`, and `object` in
  `nx-planning.md` and `nx-rust-plan.md`. Also verified the additional NX examples in
  `nx-planning-future.md` use `int64` and `boolean`. An independent repository-wide sweep found no
  surviving former spelling used as an NX type: remaining matches are host-language/Rust types,
  English words, survey directives, or literal-category tags. In particular,
  `nx-grammar-spec.md`'s `"int"`/`"bool"` values correctly document CST literal kinds and agree with
  `grammar.js`, while both primitive-type grammar productions list only canonical names. Focused
  `git diff --check`, the workspace Rust test suite, the TypeScript runtime tests,
  `cargo fmt --all -- --check`, and `openspec validate rename-primitive-types --strict` all pass.

### ✅ Verified - RF4 The new requirement promises `object` primitive storage that the design excludes
- **Severity:** Medium
- **Evidence:** The delta spec says the primitive set includes `object` and that every primitive has
  a name the type model stores (`openspec/changes/rename-primitive-types/specs/primitive-type-names/spec.md:3`).
  The design simultaneously declares the `object` grammar/`Primitive` mismatch a non-goal
  (`openspec/changes/rename-primitive-types/design.md:41`). `Primitive` has no `Object` variant
  (`crates/nx-types/src/ty.rs:13`), `builtin_type` does not return an object primitive
  (`crates/nx-types/src/semantics.rs:99`), and object handling remains a case-insensitive named-type
  special case (`crates/nx-types/src/semantics.rs:35`). The implementation therefore cannot satisfy
  the requirement as written.
- **Recommendation:** Since the design deliberately leaves object semantics unchanged, narrow the
  requirement to canonical source spellings and explicitly document `object`'s existing named-type
  representation. Otherwise, bring `object` into `Primitive` and update the design/scope and all
  affected consumers.
- **Fix:** Narrowed, taking the first option, because the design already lists resolving the
  `object` grammar/`Primitive` mismatch as a non-goal. The requirement in
  `specs/primitive-type-names/spec.md` drops "the type model stores" and now says it "governs source
  spellings only", stating outright that `object` continues to be carried as a named type rather
  than as a `Primitive` variant, and pointing at the design non-goal. The same edit adds the
  case-sensitivity rule that RF1's code fix implements, plus a "A capitalized spelling is not a
  primitive" scenario covering `INT64`, `Boolean`, `String`, and `Object`.
  `openspec validate rename-primitive-types --strict` passes.
- **Verification:** Verified the requirement now explicitly governs source spelling, documents
  `object`'s named-type representation, and covers capitalization without contradicting the design
  non-goal. Strict OpenSpec validation passes, and the implementation matches the revised rule.

## Questions
- None.

## Summary
- All four findings are verified: RF1, RF2, RF3, and RF4.
- No findings remain open.
- No new findings were discovered during verification.
- `cargo test --workspace --lib --bins --tests`, `npm test` in `runtime/typescript`,
  `cargo fmt --all -- --check`, and `openspec validate rename-primitive-types --strict` all pass.

## Fix pass (implementer report before verification)

The implementation pass reported all four findings addressed. RF3 was fixed for first-party live
documentation while its `specs/00*` component was deliberately left. The verification above accepts
RF1, RF2, and RF4 and reopens RF3.

Re-verified after the fixes:
- `cargo test --workspace --lib --bins --tests` — **1195 passed, 0 failed** (up 5: three new tests,
  plus two replacing `test_builtin_type_is_case_insensitive`).
- `npm test` in `runtime/typescript` — 12 passed, 0 failed.
- `cargo fmt --all -- --check` — clean.
- `openspec validate rename-primitive-types --strict` — valid.

One judgement call worth the reviewer's attention: RF1's fix makes `Object` a nominal name rather
than the universal top type, which is slightly wider than the finding's literal wording ("semantic
resolution and HIR tagging"). It was included because `object` is one of the eight names the
requirement governs, and leaving `is_object_type` case-folding would have kept a live alternate
spelling contradicting the very clause being narrowed in RF4. No `.nx` file uses `Object`.

## Fix pass 2 (implementer report before verification)

RF3 only. The reopen was accepted rather than argued: the earlier "immutable historical record"
rationale did not survive the observation that this change had already edited those same files, and
it had left one inventory in a half-rewritten state. The three cited lines are fixed, along with
five further NX-type-name sites in `specs/` that the same rule reaches — including a second copy of
the identical inventory in `002-nx-interpreter/spec.md:135` that the reopen did not cite.

Two `specs/` sites are deliberately still not canonical, and the verifier should confirm the
reasoning rather than the string: `002-nx-interpreter/tasks.md:134` names HIR literal kinds, not
types, and `specs/future.md:45` names Rust's `i64` and C#'s `int`.

## Fix pass 3 (implementer report before verification)

RF3 again, and the reopen was correct again. The two cited inventories are fixed, plus a third file
neither review pass had found (`nx-planning-future.md`, five NX examples).

The substantive change this pass is to the sweep rather than the documents: the earlier
type-position regex structurally could not match a comma-separated prose inventory such as
`Primitive types: string, int, float, bool, void, object`, which is why two passes of "the sweep
returns nothing" were both wrong. This pass used a word-level sweep across all markdown in the
repository and a second across non-markdown sources, and the four surviving markdown hits are
enumerated with a reason each in the finding above — the one worth checking is
`nx-grammar-spec.md:427`, kept because its `"bool"` names the `bool_literal` CST node, not a type.

## Revision: `int` reintroduced as a distinct type

After this review, the change was revised: `int` returns as a **distinct primitive type** — exact
over ±(2^53−1) on every backend, and the default integer type — rather than as the
display-preserving alias for `i64` that the change removed. `float` does not return. See the
proposal's "Why", design.md's "`int` is defined by range, not by storage width", and tasks 11–12.

Effect on the findings above:

- **RF1** is unaffected. The capitalized-spelling behavior and its test are unchanged, except that
  the assertion now also covers `Int` and `INT`.
- **RF2** stands as fixed, and one of its fixes reads better than it did. `Array index must be an
  integer` was chosen as a width-neutral phrase because the check is width-blind; it remains correct
  now that the check is `is_compatible_with(&Type::int())`.
- **RF3's live-example fix is deliberately reversed.** `type Score = int64` in the language tour
  returns to `type Score = int`, which is valid again and is the right example to hand a reader,
  since `int` is the default integer type. The same reversal applies to the other first-party NX
  sources the fix passes rewrote. The `ty.rs` doc comments stay canonical and now illustrate `int`.
- **RF3's `specs/00*` status note does not match the tree, and did not before this revision.** The
  note records that the dated spec-kit planning artifacts were deliberately left alone as historical
  records, but fix pass 2 rewrote their primitive inventories, and fix pass 3 extended that to
  `nx-planning-future.md`. This revision follows what the change actually did rather than what the
  note says, and adds `int` to those inventories. The underlying question the note raises — whether
  superseded planning documents should be rewritten, bannered, or archived — is still open and still
  belongs to its own change.

## New Findings Discovered During 2026-08-21 21:12 Review

**Review scope:** The combined staged and unstaged implementation, with focused review of the new
distinct `int` primitive across the OpenSpec artifacts, type analysis, HIR lowering, interpreter,
C#/TypeScript generation, TypeScript IR runtime, tests, and formatting checks.

### ✅ Resolved - RF5 The defining portable `int` range and checked arithmetic are not implemented
- **Severity:** High
- **Evidence:** The spec requires `int` to be exact over ±(2^53−1) on every backend and requires
  checked, non-wrapping arithmetic
  (`specs/primitive-type-names/spec.md:140-162`), but then explicitly says neither guarantee is
  enforced (`specs/primitive-type-names/spec.md:150-153`). Literal lowering accepts every `i64` and
  tags it as `int` (`crates/nx-hir/src/lower.rs:826-857`), while the interpreter wraps addition,
  subtraction, and multiplication (`crates/nx-interpreter/src/eval/arithmetic.rs:36-43,59-64,77-82`).
  The TypeScript boundary accepts any JavaScript `number` for `int`, including fractions, infinities,
  and unsafe integers (`runtime/typescript/src/index.ts:846-860`). The change nevertheless marks all
  79 tasks complete. Consequently the new type's defining cross-backend guarantee is currently
  neither true nor testable, and the same program can behave differently across implementations.
- **Recommendation:** Either implement literal, host-boundary, and arithmetic range checks across
  the interpreter and generated runtimes now, with tests at ±(2^53−1) and immediately outside the
  range, or move the normative `SHALL` contract and its scenarios into the later bounds-check change.
  Do not present this change as implementing portable `int` semantics while enforcement is deferred.
- **Resolution:** The change owner confirmed that runtime bounds checks are intentionally deferred to
  a separate change. JavaScript `int64` support, potentially using a `bigint` representation, is also
  intentionally deferred; `int64` continues to use JavaScript `number` for now. This matches the
  explicit non-goals in `proposal.md:80-88`, `design.md:45-51`, and tasks 11–12. The ±(2^53−1) rule
  records the intended portable semantics that later runtime work must enforce; implementing it is
  not an acceptance criterion for this change. No implementation fix is required here.

### ✅ Verified - RF6 The revised artifacts still contradict the distinct-`int` design
- **Severity:** Medium
- **Evidence:** The proposal introduces the distinct type at `proposal.md:60-71`, but later says
  `v:int` names no type and claims there is "No semantic change," including to literal binding
  (`proposal.md:89-97`). The design likewise promises to preserve every observable semantic
  (`design.md:42-43`), says `Primitive::Int` is gone (`design.md:221-224`), and says there is no type
  called `int` (`design.md:250-260`). Those statements describe the earlier exact-width-only revision
  and are false for the implementation now being reviewed.
- **Recommendation:** Rewrite or remove the obsolete passages so all artifacts consistently state
  that the old alias is removed and a new, semantically distinct default `int` primitive is added.
  Reconcile the proposal's impact and non-goals with the actual literal, promotion, and range changes.
- **Fix:** All six cited passages are rewritten to describe the implementation as it now stands:
  - `proposal.md` — the "Deliberately not fixed here" bullet no longer claims `v:int` names no type;
    the unresolved-name gap is now illustrated with `v:long` and `v:decimal`, which are still
    accepted silently. The blanket "No semantic change" bullet is split into "What changes
    semantically" (integer literals infer `int` instead of the type formerly spelled `i64`; promotion
    gains the middle rank `int32` < `int` < `int64`; the range is not yet enforced, so no existing
    program's evaluation changes today) and "What does not change" (widths of the named types,
    boolean semantics, integer/float compatibility, and the TypeScript/C#/MessagePack mappings).
  - `design.md` — the goal at 42-43 no longer promises every observable semantic is preserved; it
    names literal binding as the one deliberate exception. The alias-deletion payoff no longer says
    `Primitive::Int` is gone: `Primitive::Float` is gone outright, while `Primitive::Int` survives as
    a variant with its own canonical spelling rather than a second name for `Primitive::Int64` — the
    reason `CanonicalPrimitive`, `canonical()`, and the hand-written impls still go away is that no
    two variants share a canonical value, which is stated directly. The IR section no longer claims
    there is no type called `int`; it records that the type name and the literal tag are once again
    the same string, accepted for the same reason as the `boolean` homonym, with the sweep going by
    field rather than by string (its type-shape example moved to `"name":"i64"`, which is a name that
    actually needs renaming). The boolean paragraph's back-reference to a near-homonym "the `int`
    removal just eliminated" is corrected, and the `bool`-versus-`int` argument at 123 now says
    "removing the `int` alias".
  - The proposal's Impact section was re-read and left as-is: it enumerates the files the change
    touches and its "currently infer" wording describes the pre-change state, which remains accurate.
  - `openspec validate rename-primitive-types --strict` passes.
- **Verification:** Verified every cited stale passage has been corrected. The proposal now
  distinguishes the semantic changes from preserved mappings and uses `long`/`decimal` for the
  unresolved-name gap. The design consistently treats `int` as a distinct surviving primitive,
  explains why alias machinery can still be removed, and distinguishes the IR type-name field from
  the literal-kind field. A targeted search found none of the obsolete claims, and strict OpenSpec
  validation passes.

### ✅ Resolved - RF7 The interpreter erases the `int`/`int64` distinction at runtime
- **Severity:** Medium
- **Evidence:** The spec requires `int` and `int64` to be unequal and each to render under its own
  name (`specs/primitive-type-names/spec.md:20-23`), with canonical names in diagnostics
  (`specs/primitive-type-names/spec.md:62-75`). Both types nevertheless use `Value::Int(i64)`
  (`crates/nx-interpreter/src/value.rs:32-41`); that carrier always reports `int`
  (`crates/nx-interpreter/src/value.rs:147-155`), and runtime reconstruction always produces
  `Type::int()` (`crates/nx-interpreter/src/interpreter.rs:2671-2675`). An `int64` parameter, result,
  or statically promoted `int + int64` value therefore becomes runtime `int`, so runtime diagnostics
  cannot preserve its declared canonical type and later range enforcement cannot select the correct
  bound from the value.
- **Recommendation:** Add a distinct `int64` runtime carrier, or retain semantic primitive metadata
  alongside integer values. Add interpreter tests proving that declared and promoted `int64` values
  remain `int64` through runtime checks and diagnostic rendering.
- **Resolution:** Deferred by the change owner, for the same reason as RF5 and as part of the same
  later change. The finding is accurate — both `int` and `int64` use `Value::Int(i64)`, and
  `runtime_type_of_value` always reconstructs `Type::int()` — but a distinct runtime carrier is only
  worth building alongside the bounds enforcement that needs it, and `int64`-as-`bigint` on
  JavaScript, which is deferred too.
  <br>The investigation is recorded in `specs/future.md` under "Bounds checks are specified but not
  enforced" so it is not rediscovered later. It documents two designs and their costs: **type-directed**
  (~1–1.5 days) reads the already-computed `TypeEnvironment` — which `build_resolved_program` in
  `crates/nx-api/src/artifacts.rs:1609` currently discards while keeping the adjacent
  `lowered_module`, the single reason the interpreter is width-blind — and makes *expressions*
  width-correct without making *values* so; **value-directed** (~3–4 days) adds the `Int64` carrier
  this finding recommends across the three parallel value enums (`Value`, `NxValue`,
  `SerializedValue`) and is the design that actually satisfies RF7. The recommendation recorded there
  is value-directed, done together with `int64`-as-`bigint`, since both solve the same FFI-boundary
  problem.
  <br>One unrelated live defect was found while scoping and is recorded there as well:
  `crates/nx-interpreter/src/interpreter.rs:1908` negates via `Value::Int32(-n)`, which panics in
  debug builds on `i32::MIN`. It is reachable only through an FFI-supplied `int32`, since evaluation
  never produces that variant from source, and it is independent of this change.

### ✅ Verified - RF8 The latest revision does not pass repository formatting checks
- **Severity:** Low
- **Evidence:** `cargo fmt --all -- --check` reports required formatting changes in
  `crates/nx-cli/src/typegen.rs`, `crates/nx-ffi/tests/ffi_smoke.rs`,
  `crates/nx-hir/src/lower.rs`, `crates/nx-syntax/tests/parser_tests.rs`, and
  `crates/nx-types/src/semantics.rs`. `git diff --check` also reports trailing whitespace in nine new
  `int`-rewritten test source strings in `crates/nx-interpreter/tests/conditionals.rs` and one in
  `crates/nx-interpreter/tests/operators.rs`.
- **Recommendation:** Run `cargo fmt --all`, remove the trailing spaces in embedded NX snippets, and
  require both `cargo fmt --all -- --check` and `git diff --check` to pass before archiving.
- **Fix:** `cargo fmt --all` was run; it reformatted exactly the five reported files
  (`crates/nx-cli/src/typegen.rs`, `crates/nx-ffi/tests/ffi_smoke.rs`, `crates/nx-hir/src/lower.rs`,
  `crates/nx-syntax/tests/parser_tests.rs`, `crates/nx-types/src/semantics.rs`) and nothing else.
  Trailing whitespace was stripped from `crates/nx-interpreter/tests/conditionals.rs` and
  `crates/nx-interpreter/tests/operators.rs`. Note that the trailing whitespace predates this change
  — 37 occurrences in `conditionals.rs` and 2 in `operators.rs` are present at `HEAD`, and
  `git diff --check` flagged only the subset this change happened to touch — so both files were
  cleaned in full rather than line-by-line, to stop the same lines being reported again. Every
  affected line is inside an embedded NX source snippet where trailing whitespace is insignificant,
  not inside an expected-output string.
  <br>`cargo fmt --all -- --check` and `git diff --check` now both pass. `nx-interpreter`, `nx-cli`,
  `nx-hir`, `nx-types`, `nx-syntax`, and `nx-ffi` tests were re-run afterwards: all pass, zero
  failures.
- **Verification:** Verified `cargo fmt --all -- --check` and `git diff --check` both complete with
  no output and exit successfully. Re-ran the six affected crate suites (`nx-interpreter`, `nx-cli`,
  `nx-hir`, `nx-types`, `nx-syntax`, and `nx-ffi`); all tests pass. The formatting fix is complete.

## Questions from the 2026-08-21 Review
- None. The change owner confirmed that runtime enforcement of the portable ±(2^53−1) range is
  intentionally deferred.

## 2026-08-21 Review Summary
- RF6 and RF8 are verified. RF5 and RF7 are resolved as intentional deferrals. No findings from this
  review pass remain open.
- The six affected Rust crate suites pass, as do strict OpenSpec validation,
  `cargo fmt --all -- --check`, and `git diff --check`.

## Fix pass 4 (implementer report before verification)
- **RF6 fixed.** The obsolete passages in `proposal.md` and `design.md` — the ones written for the
  earlier exact-width-only revision — are rewritten to match the distinct-`int` implementation.
- **RF7 resolved as deferred**, with the full investigation, both candidate designs and their costs,
  and the `build_resolved_program` discarded-`type_env` root cause recorded in `specs/future.md`.
- **RF8 fixed.** `cargo fmt --all -- --check` and `git diff --check` both pass; the six crates whose
  files were reformatted were re-tested and all pass.
- Nothing remains open. RF7 needs the reviewer's agreement that deferral is an acceptable
  disposition rather than a fix; RF6 and RF8 await verification.
