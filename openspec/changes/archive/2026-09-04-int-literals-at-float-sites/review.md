# Review: int-literals-at-float-sites

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, and the deltas for
`contextual-numeric-literals`, `primitive-type-names`, and `unbraced-literal-forms`.  
**Reviewed code:** contextual typing and HIR rewrite in `crates/nx-types` and `crates/nx-hir`;
language service, formatter, type generation, code-generation, and evaluation coverage; the
DrawnUI corpus tooling; and the related examples and documentation.

## Findings

### ✅ Verified - RF1 Float-typed content bindings still reject integer literals

- **Severity:** Medium
- **Evidence:** The three element-content paths call `check_typed_binding` without an `ExprId`
  (`crates/nx-types/src/infer.rs:1560`, `1673`, and `1883`), so the conversion branch at
  `infer.rs:2484` cannot run. Reproduced with
  `let <collect content item: float64 />: float64 = { item }`:
  `<collect>{1}</collect>` reports “expects float64, found int”, while
  `<collect>{1.0}</collect>` succeeds. This is a declared component property binding and conflicts
  with the requirement that every declared floating-point binding site supplies the expectation
  (`specs/contextual-numeric-literals/spec.md:48-54`; task 1.4).
- **Recommendation:** Thread the content expression(s) and their element expectation into the
  contextual-literal check for record, union, and component element bodies. Add regression tests
  for scalar and list-valued float content, including an inexact integer.
- **Fix:** All three content paths now call a new `check_content_binding`
  (`crates/nx-types/src/infer.rs`) instead of `check_typed_binding` with no `ExprId`. A single
  content expression is checked as itself, so it reaches the rule exactly as a property binding
  does; several are checked as the elements of the declared list, through a new
  `convert_int_literals_in` extracted from the `Expr::Array` branch of `convert_int_literals` — body
  content is a *sequence* of expressions with no expression of its own, so it needs the list
  treatment rather than a single `ExprId`. `test_int_literal_binds_as_element_body_content`
  (`crates/nx-types/src/check.rs`) covers scalar float content, a `float64[]` content property with
  three literals, a record's content field, and an inexact literal, which is rejected with
  `float-literal-not-exact` naming the value and `float64`. End to end,
  `<collect>{1}</collect>` and `<collect>{1.0}</collect>` now both evaluate to `1.0`. The spec's
  site list and a new scenario name body content, and design D6 records why it was missed.
- **Verification:** Verified with `cargo test --workspace` and direct CLI evaluation: scalar and
  three-element `float64[]` body content written as integers evaluate as floats, while an inexact
  body literal reports `float-literal-not-exact`. The helper is used by record, union, and component
  content paths, and its new regression test covers scalar, list, record, and inexact cases.

### ✅ Verified - RF2 Converted literals at `float32` sites are recorded as `float64`

- **Severity:** Medium
- **Evidence:** `convert_int_literals` unconditionally records `Type::float64()` at
  `crates/nx-types/src/infer.rs:2547-2555`, even though it has already identified a `float32`
  target. The contextual-numeric-literals spec explicitly requires the literal at a `float32`
  property to have type `float32` (`specs/contextual-numeric-literals/spec.md:27-30`). The design's
  D4 instead says to mirror the current real-literal behavior, so the change artifacts disagree;
  the implementation follows the design rather than the normative requirement.
- **Recommendation:** Resolve the specification/design conflict. If the requirement is retained,
  record the resolved target type (and update the explicit-real-literal path as needed) and assert
  it through the type environment, IR, and language-service tests. Otherwise, amend the requirement
  and its scenario to state the deliberate `float64` representation.
- **Fix:** Resolved in favour of design D4; the spec was amended, the implementation was not. The
  deciding fact is that `infer_literal` (`crates/nx-types/src/infer.rs:638`) gives *every* float
  literal `float64`, `float32` sites included — so a written `24.0` at a `float32` property is
  already recorded `float64`. Recording `float32` for a converted `24` would therefore make it more
  precisely typed than the `24.0` it is required by this same capability to be indistinguishable
  from, which is a contradiction inside the spec rather than a choice between two defensible
  readings. The scenario now requires the literal to take "the type a written `1.0` takes at that
  same site", and new prose in the requirement separates the type a value is *bound at* (the
  declared one, `float32`, which is still what decides exactness — `16777217` is still rejected)
  from the type *recorded for the literal*. Whether a real literal should narrow at a `float32`
  site is left open, as it is the same question for both spellings.
- **Verification:** Reopened. Although `contextual-numeric-literals` now describes the intended
  recorded type, `specs/primitive-type-names/spec.md:73-76` still requires
  `let x: float32 = 42` to infer `float32`, while the implementation records `float64` and the
  contextual-numeric delta now requires it to match an explicit real literal (also `float64`). The
  proposal likewise still says an expected `float32` literal is typed as that float type. Resolve
  those remaining artifact contradictions, or change the implementation, before this finding can
  be verified.
- **Fix (2):** Both remaining contradictions resolved, and a measurement settles them rather than a
  choice: **`let x: float32 = 42` gives `x` the type `float32` today** — and so does
  `let x: float32 = 42.0`. The two spellings already agree. The artifacts were conflating the type
  of the *binding* (the declared width, at every site) with the type recorded for the *literal
  expression* (whatever an explicit real literal takes, today `float64` at every width because a
  literal node carries no width). `primitive-type-names` was therefore not wrong about behavior,
  only about which of the two it was naming: its scenario now asserts `the type of x`, adds
  `- **AND** the result SHALL be the same as for let x: float32 = 42.0`, and the requirement states
  the distinction. `proposal.md` says "bound at that float type", spells out the `float32` example
  with both spellings, and notes what the literal expression records. The distinction is now pinned
  by a test rather than by prose:
  `test_an_expected_float_type_binds_the_same_for_both_literal_spellings`
  (`crates/nx-types/src/check.rs`) asserts all four of
  `float32`/`float64` × `42`/`42.0` bind `x` at the declared width. No implementation change.
- **Verification:** The proposal and both numeric deltas now consistently distinguish the declared
  binding type from the type recorded for a literal expression. The new type-checker regression test
  confirms that both spellings bind `x` at `float32` and `float64` respectively, and the codegen
  regression test confirms that `24` and `24.0` at a `float32` site emit the same NX IR once source
  provenance is excluded. `cargo test --workspace` passes.

### ✅ Verified - RF3 The language service does not provide the required literal quick-info

- **Severity:** Medium
- **Evidence:** Task 4.2 is checked off but requires a quick-info test for `<B v=1 />` at a
  `float64` site (`tasks.md:62-64`). The only language-service change tests diagnostics
  (`crates/nx-language-service/src/lib.rs:1858-1885`). Its `hover` implementation only searches
  top-level document symbols (`lib.rs:315-337`), and the existing tests explicitly cover only
  declaration hover (`lib.rs:2032-2056`); it cannot report a literal's contextual type.
- **Recommendation:** Add semantic hover/quick-info for expression spans backed by the analysis
  type environment, then test that the `1` in `<B v=1 />` reports `float64` (and settle the
  expected `float32` behavior alongside RF2).
- **Fix:** The inconsistency is fixed by correcting task 4.2, not by building the feature — please
  verify that judgment rather than the code. Task 4.2 contradicted this change's own design, whose
  Open Questions already says surfacing the contextual type in hover "changes no spec requirement
  and no task here". No requirement in any of the three deltas asks for it, and it is not a small
  addition: `hover` is documented as "conservative", matches a position against top-level document
  symbols only, and the service exposes no expression-level type API at all — so it would mean
  mapping a byte offset to an `ExprId` and reading the analysis type environment, infrastructure
  that serves every expression rather than this rule. Task 4.2 now describes what was actually
  verified (the diagnostics test, which does confirm the language service reports the new
  acceptance and still rejects an inexact literal) and states the exclusion; design's Open Question
  now names the missing infrastructure. **Recommended follow-up:** expression-level quick-info as
  its own change — it is the mitigation design's Risks section already leans on for "an author now
  cannot tell from the source whether a property is `int` or `float`".
- **Verification:** Reopened. Deferring expression quick-info is a reasonable scope decision, but
  it conflicts with the unchanged proposal impact (`proposal.md:82-84`) and risk mitigation
  (`design.md:157-160`), both of which say the language service should show the contextual type.
  Further, the revised task asks for diagnostics over `<B v=1 />`, but the only added test covers
  `let width: float64 = 24` (`crates/nx-language-service/src/lib.rs:1858-1885`). Update the
  proposal and risk text to make the deferral unambiguous and add the promised property-binding
  diagnostics test; alternatively, implement expression-level quick-info.
- **Fix (2):** Both gaps closed; the deferral stands. The two places that still read the other way
  now state it outright. `proposal.md`'s Impact said quick-info "should report the float type it
  took"; it now says the language service needs no change beyond agreeing with the checker, that
  quick-info would be good to have but is **out of scope here** because the service has no
  expression-level type API, and points at design's Open Questions. `design.md`'s Risks claimed the
  notation loss was "Mitigated by the language service, which knows the resolved type and can show
  it on hover" — which was doubly wrong, since the service neither knows it nor can show it; that
  risk is now recorded as **accepted unmitigated**, with the reason and the pointer to the Open
  Question. The promised test exists: the diagnostics test now checks
  `external component <B v:float64 /> / <B v=1 />` as its own case, alongside the annotated `let`,
  with a comment saying why the property binding does not ride on the `let`.
- **Verification:** The proposal, design risk, and task now consistently state that expression-level
  quick-info is out of scope, and no delta requirement demands it. The language-service regression
  test covers the annotated `let`, the actual `<B v=1 />` property binding, and an inexact literal;
  it passes along with `cargo test --workspace`.

### ✅ Verified - RF4 Required documentation corpus cleanup is incomplete and leaves obsolete guidance

- **Severity:** Medium
- **Evidence:** Task 6.1 is checked off but explicitly includes the `.nx` files under
  `docs/drawnui-proposal/` and `docs/displaylist-proposal/` (`tasks.md:84-91`); none of those files
  is in the implementation diff. They still contain redundant float defaults, for example
  `docs/drawnui-proposal/core/core.nx:103-113` and
  `docs/displaylist-proposal/displaylist/displaylist.nx:135-154`. In addition, the unmodified
  `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md:1481` and
  `docs/drawn-ui-proposal-nx-enhancements.md:539-570` state that `float64` properties must use
  `.0`, which is now false.
- **Recommendation:** Convert the in-scope documentation `.nx` examples and update or clearly
  historical-scope the prose that describes the old restriction. Add the stated compile/build
  verification before marking tasks 6.1 and 6.2 complete.
- **Fix:** Both halves done. All 54 `: float64 = N.0` defaults across the four proposal libraries
  (`core`, `ui`, `graphics`, `displaylist`) are now written `= N`, with trailing comment columns
  realigned. The verification gap is closed with a real gate rather than an assertion: `nxlang
  typegen` over a library directory type checks the whole library (confirmed by planting a
  deliberate type error and seeing it reported), and all four libraries produce **no diagnostic and
  byte-identical generated TypeScript** before and after; `displaylist` additionally emits NX IR
  that is **identical apart from source provenance**, the same D7 gate the fiddle corpus used. On
  the prose: the MVP proposal's "Integer literals still do not widen" sentence is replaced, and its
  Appendix A fences (63 occurrences) are converted to match the sentence's new claim and the `.nx`
  files they mirror; NXE8 in `drawn-ui-proposal-nx-enhancements.md` — its summary-table row, status
  line, worked example, impact, and "Possible enhancement" — now records this change as the
  resolution of the widening half, and states the two boundaries (an `int`-typed *expression* is
  still rejected, an inexact literal is rejected rather than rounded). Tasks 6.1 and 6.2 now name
  the verification actually performed. `docs/scratch-highlighting.nx` is untracked scratch for
  syntax highlighting, where a real literal is the point, and was left alone.
- **Verification:** Verified within the updated task scope. The four proposal-library `.nx` files
  no longer contain whole-number `.0` defaults, the stale restriction prose was updated, and
  `npm run build` in `docs/` succeeds. Remaining real-literal spellings in historical prose and
  examples are outside the task's targeted proposal-library/default cleanup.

## Questions

- ~~The recorded type at a `float32` site remains unresolved across the artifacts.~~ **Resolved
  (RF2 fix 2), and the disagreement was smaller than it looked.** The two deltas were naming
  different things: the binding is `float32` (measured — `let x: float32 = 42` and
  `let x: float32 = 42.0` both give `x` type `float32`), while the literal expression records what
  an explicit real literal records. All three artifacts now say which one they mean, and a test
  pins it.

## Summary

- Opened 4 findings: two behavioral gaps, one language-service task mismatch, and one incomplete
  documentation/corpus task.
- `cargo test --workspace`, the DrawnUI app's `npm run typecheck` and `npm test`, and
  `openspec validate int-literals-at-float-sites --strict` all passed.
- Verification: all four findings are verified. RF2's fix aligns the artifacts with the existing
  binding and literal-expression behavior; RF3's fix makes the intentional quick-info deferral
  explicit and tests the property-binding diagnostics the task calls for.

## Fix pass report — 2026-09-04

The implementation pass reported all four findings addressed: RF1 in the compiler, RF4 in the
corpus and prose, RF2 and RF3 by correcting artifacts that were wrong rather than code that was.
Verification subsequently confirmed RF1 and RF4 and reopened RF2 and RF3 because their artifact
updates remain incomplete.

- **RF2** amends a normative scenario to match the design instead of changing the implementation.
- **RF3** narrows a task instead of building expression-level quick-info, and recommends that
  feature as a separate change.

Verification after the fixes:

- `cargo test --workspace` — 1372 passed, 0 failed (up one: the new content-binding test).
- `cargo fmt --check` — clean for every file this change touches. The remaining `Diff in` reports
  are pre-existing drift in files this change does not modify, confirmed against a clean tree.
- Fiddle: `npm test` 12/12 examples, `npm run typecheck` clean, and the emitted IR for all 12
  examples still **byte-identical** to the pre-edit baseline.
- Docs proposal libraries: typegen output byte-identical for all four, `displaylist` NX IR identical
  apart from source provenance.
- Docs site: `npm run build` — 23 pages.
- `openspec validate int-literals-at-float-sites --strict` — valid.

## Fix pass 2 — 2026-09-04

Addressed both reopened findings. Neither needed an implementation change; both were artifacts
describing the implementation inaccurately, and in RF2's case the accusation of a spec/behavior
conflict dissolved once the binding type was actually measured rather than inferred from the code.

- **RF2** — `let x: float32 = 42` binds `x` at `float32`, exactly as `= 42.0` does. The deltas were
  conflating binding type with literal-expression type; all three artifacts now distinguish them,
  and `test_an_expected_float_type_binds_the_same_for_both_literal_spellings` pins the behavior.
- **RF3** — the deferral is now stated in the proposal's Impact and design's Risks, not only in the
  Open Questions; the risk is recorded as accepted unmitigated rather than falsely mitigated. The
  missing `<B v=1 />` property-binding diagnostics test was added.

Verification after this pass:

- `cargo test --workspace` — **1373 passed, 0 failed** (up two: the `float32` binding test and the
  property-binding diagnostics case).
- Fiddle: `npm test` 12/12, `npm run typecheck` clean, emitted IR for all 12 examples still
  byte-identical to the pre-edit baseline.
- `cargo fmt --check` clean for every file this change touches.
- `openspec validate int-literals-at-float-sites --strict` — valid.
