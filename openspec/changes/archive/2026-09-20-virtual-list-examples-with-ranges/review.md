# Review: virtual-list-examples-with-ranges

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `specs/playground/spec.md`,
`specs/fiddle-nx-language/spec.md`, `specs/range-expressions/spec.md`,
`specs/unbraced-literal-forms/spec.md`

**Reviewed code:**

- `nx:` (commits `d0b3b8e`, `c48de57`, `5d0130f`) — `crates/nx-types/src/infer.rs`,
  `crates/nx-types/tests/contextual_literals.rs`, `crates/nx-syntax/tests/parser_tests.rs`,
  `openspec/specs/range-expressions/spec.md`,
  `sites/playground/src/examples/nx/{cells,uneven-cells,scroll}.nx`
- `~/src/DrawnUi.FiddleEngine` (commits `5889b49`, `7fb1300`, `327d639`) —
  `nx/generate-catalog.mjs`, `nx/catalog/{drawnui.nx,catalog-meta.json,omitted.json}`,
  `nx/CATALOG.md`, `nx/src/{values.ts,draw.tsx,runtime.ts,templateCell.ts}`,
  `nx/test/{catalog,generator,runtime,values}.test.ts`, `nx/test/support.ts`,
  `src/DrawnUi.Fiddle/FiddlePresetsNx.cs`

**Verified by running:** `cargo test -p nx-types -p nx-syntax` (all green),
`pnpm run check-examples` in `sites/playground` (20/20 ok, the three rewritten examples still
`static (code-behind)`), `npm run test:nx -- --nx ../nx` in the fiddle (90/90 pass),
`openspec validate --strict virtual-list-examples-with-ranges` (valid), and the diagnostic
behaviors below reproduced from a fresh `cargo build -p nx-cli`.

## Findings

### ✅ Verified - RF1 The new bare-name suggestion proposes `{name}` without checking the binding's type fits the site, so it can still name a form the site rejects
- **Severity:** Medium
- **Evidence:** `crates/nx-types/src/infer.rs:4554` offers `{name}` whenever `self.env.lookup(name)`
  finds anything, with no check that the binding satisfies `expected`. Reproduced:
  - `let r = 5` + `let x:string = r` → *"expects string … to use the value bound to 'r' write {r}"*,
    while `let x:string = {r}` → *"expects string, found int"*. The quoted form, which that site
    does accept, is suppressed by the `else if`.
  - `let f() = 5` + `let x:int = f` → *"write {f}"*, a function value at an `int` site.

  This is the exact failure mode the change set out to remove, moved to a different site, and it
  contradicts two clauses of the requirement it implements
  (`specs/unbraced-literal-forms/spec.md`): *"The diagnostic SHALL NOT propose a form the site would
  reject"*, and the scenario *"The quoted form is offered only where a string fits"*, whose WHEN
  ("a bare name that resolves to nothing … at a site whose expected type is `string`") is satisfied
  by the first case above while its THEN is not.
- **Recommendation:** Gate the braced branch on the binding's type satisfying `expected`
  (`type_satisfies_expected_with_coercion` on the looked-up type, as the string branch already
  does), and fall through to the quoted branch otherwise. Add a test for `let r = 5` +
  `let x:string = r` asserting `"r"` and not `{r}`.
- **Fix:** As recommended. `crates/nx-types/src/infer.rs:4552` now computes `binding_fits` — the
  looked-up type run through `type_satisfies_expected_with_coercion` against `expected` — and only
  that branch proposes `{name}`; a binding that does not fit falls through to the quoted branch and
  then to the fallback of RF6. New test `a_visible_binding_of_the_wrong_type_is_not_offered` pins
  both reproducers: `let r = 5` + `let x:string = r` names `"r"` and never `{r}`, and
  `let f() = 5` + `let x:int = f` never names `{f}`. The delta's requirement text now reads "whose
  own type satisfies the site", with a scenario for it.
- **Verification:** Confirmed against a rebuilt `nx-cli`. `crates/nx-types/src/infer.rs:4554-4560`
  gates the braced branch on `binding_fits`. Both reproducers are gone: `let r = 5` +
  `let x:string = r` now answers *"for a string value write \"r\""* with no `{r}`, and
  `let f() = 5` + `let x:int = f` no longer names `{f}`. The cases that should still get the braced
  form do: int→int, string→string and record→record all name `{name}`. `cargo test -p nx-types
  -p nx-syntax` green (27 suites, 0 failures), including the new
  `a_visible_binding_of_the_wrong_type_is_not_offered`. The delta's requirement text and its new
  scenario match the behavior.

### ✅ Verified - RF2 The cell factory gets a new identity on every redraw, so any state change drops and rebuilds every realized cell
- **Severity:** Medium
- **Evidence:** `bindTemplate` is created inside `drawValue` (`nx/src/draw.tsx:170-178`) and calls
  `templateFactory`, which declares a fresh `class NxTemplateCell` and returns a fresh
  `() => new NxTemplateCell()` per call (`nx/src/templateCell.ts:170-219`). `drawValue` runs on
  every redraw — `runtime.ts:493` redraws the whole tree after each dispatch — so the `ItemTemplate`
  prop is a different function each time. DrawnUI compares it by identity and rebuilds on a change:
  `node_modules/drawnui-react/dist/controls/SkiaLayout.js:211-216` → `ApplyItemsSource()`, commented
  *"Drops realized cells and rebuilds the structure"*. `ItemsSource` is likewise a fresh array each
  redraw (`nx/src/values.ts:216`).

  Failure scenario: a fiddle snippet that binds `ItemsSource`/`ItemTemplate` and also has a button
  with `<Update …/>`. Tapping the button drops every realized cell and marks the structure dirty —
  scroll position and recycling state are lost on a tap that has nothing to do with the list. Both
  new presets are static, so no test or browser check exercises it. The
  `fiddle-nx-language` requirement asks that *"recycling and measuring behave as they do for a
  control the engine's own templates drive"*, which this does not.
- **Recommendation:** Memoize the factory per template on something that outlives a redraw (the
  `Session`/`InstanceTree`), keyed by the function record's `module` + `name` plus the parameter
  list, and return the same function across redraws. Consider caching the coerced `ItemsSource`
  array the same way. Add a runtime test that redraws after a dispatch and asserts the
  `ItemTemplate` prop is identical to the one before.
- **Fix:** As recommended, on the `Session`. `DrawContext` gains `templates: Map<string, () =>
  unknown>` (`nx/src/draw.tsx`), held on the session beside `reported` (`nx/src/runtime.ts`) so it
  outlives a redraw; `bindTemplate` keys it on `` `${record.module}.${record.name}(${params})` ``
  and returns the factory it already made. New test *"a redraw binds the same cell factory, so a
  dispatch elsewhere does not drop the realized cells"* mounts a page with a list and an unrelated
  `<Update/>` button, fires the tap and asserts the `ItemTemplate` prop is the same function; it
  fails on the pre-fix `draw.tsx` and passes after.
- **Status (`ItemsSource`):** not cached. The coercion is identity-preserving already
  (`Array.isArray(item) ? item : [item]`), so the array only changes when the value tree behind it
  does — and the engine's `ApplyReorderChange` handles a new reference over the same items without
  rebuilding. Caching it would freeze a list that is meant to change, which is a worse bug than the
  one it would fix.
- **Verification:** `DrawContext.templates` is a `Map` held on the `Session`
  (`nx/src/runtime.ts:275,446,483`), so it is created once per IR and outlives every redraw;
  `bindTemplate` returns the cached factory before making one, keyed by
  `` `${record.module}.${record.name}(${params})` `` (`nx/src/draw.tsx:184-200`). A new IR builds a
  new `Session` with a fresh `Map`, so an edited template still gets a new factory — the cache does
  not stale. The new test asserts the `ItemTemplate` prop is identical across a dispatch, and it is
  sensitive: pre-fix `bindTemplate` called `templateFactory` unconditionally, which returns a
  freshly created `() => new NxTemplateCell()` on every call, and the existing Cards preset test
  independently pins that a dispatch really does redraw. 93/93 fiddle tests pass. The
  `ItemsSource` reasoning holds — caching it would pin a list that is meant to change.
  Non-blocking nit: the new test would be sturdier if it also asserted the tap took effect (the
  button's `Text` going `Taps: 0` → `Taps: 1`), so it cannot pass vacuously if redraw-on-dispatch
  ever stops.

### ✅ Verified - RF3 The new playground requirement is set-wide, but `snapping.nx` still doubles a palette by hand to reach a count
- **Severity:** Medium
- **Evidence:** `specs/playground/spec.md` states the rule over the whole set — *"Where an example
  needs a collection of a given size whose items follow from their position, its source SHALL derive
  that collection from a range … rather than from a written-out seed list, a repeated element … and
  no example SHALL carry scaffolding whose only purpose is to reach a count."*
  `sites/playground/src/examples/nx/snapping.nx:40-45` writes the same six colours twice to get
  twelve slides, with the comment *"the original cycles six colors by index"*, and
  `snapping.nx:177` loops it only to number the slides — the identical shape `scroll.nx` just
  retired, and expressible as `for index in 0..12` with a `color(index % 6)` function.
- **Recommendation:** Either rewrite `snapping.nx`'s `loopColors` the way `scroll.nx` was, or narrow
  the requirement's scenarios to the examples the change actually claims. Leaving both as they are
  means the capability is untrue the day it is written.
- **Fix:** Rewrote `snapping.nx`, rather than narrowing the requirement: the rule is worth having
  over the whole set, and this is the last example that broke it. The doubled twelve-colour list is
  now `loopColor(index:int)` cycling six colours with `index % 6`, carrying the same
  no-list-indexing note `scroll.nx` carries, and the carousel counts with `for position in 0..12`.
  Verified `pnpm run check-examples` (20/20, Carousel & Drawer still `static (code-behind)`) and
  that the evaluated drawing is byte-identical to the one before the rewrite.
- **Verification:** `snapping.nx:41-52` is now `loopColor(index:int)` cycling six colours with
  `index % 6`, and `snapping.nx:185` counts with `for position in 0..12`. The rewrite is
  colour-for-colour identical: the old `loopColors` was the palette written twice, so
  `loopColors[position]` was exactly `palette[position % 6]` for 0..11, and `0..12` is half-open.
  `pnpm run check-examples` 20/20 with Carousel & Drawer still `static (code-behind)`. Swept the
  rest of the set: the eight remaining literal lists (`star`, `aspects`, `chipLabels`, `big`,
  `peek`, `samples`, `languages`, and the ranges' own loops) are all real content whose items do
  not follow from their position, so the set-wide requirement is now true of the whole set.

### ✅ Verified - RF4 The template-failure dedup key silently degrades if the message ever lacks `' at index '`
- **Severity:** Low
- **Evidence:** `nx/src/runtime.ts:457` computes the `once` key as
  `where.slice(0, where.indexOf(' at index '))`. When `indexOf` returns `-1`, `slice(0, -1)` yields
  the whole message minus its last character, so the key becomes per-message and the "once per
  template" guarantee in the spec's *"A failing template is reported, not fatal"* scenario quietly
  becomes once per failure. Only `templateCell.ts:210` calls it today, so this is latent, not live.
- **Recommendation:** Have `reportTemplateFailure` take the template name as its own argument
  (the caller already has `template.name`) rather than parsing it back out of the message.
- **Fix:** As recommended. `reportTemplateFailure` is now `(template: string, where: string)` in
  both `TemplateCellContext` and `DrawContext`; `templateCell.ts` passes `template.name`, and
  `runtime.ts` dedups on `` `template:${template}` `` with no string surgery.
- **Verification:** `reportTemplateFailure` is `(template, where)` in both `TemplateCellContext`
  (`nx/src/templateCell.ts:58`) and `DrawContext` (`nx/src/draw.tsx:66`); `templateCell.ts:211`
  passes `template.name`, and `nx/src/runtime.ts:459` dedups on `` `template:${template}` `` with
  no string surgery left. The existing "reported once with the index" test still passes.

### ✅ Verified - RF5 The index signature added to `DrawnUiNamespace` is unused and weakens the interface
- **Severity:** Low
- **Evidence:** `nx/src/values.ts:26-27` adds `readonly [name: string]: unknown` with the comment
  *"which is how a cell built outside React reaches one"* — but the cell path reaches classes
  through `DrawnUiCore`, cast locally in `templateCell.ts:73`, never through `drawnUi()`. The only
  uses of `drawnUi()` remain the five named constructors at `values.ts:49-66`. The signature's one
  effect is that any misspelled member of `DrawnUi` now type-checks as `unknown`.
- **Recommendation:** Delete the index signature and its comment.
- **Fix:** Deleted. `DrawnUiNamespace` is back to the five named constructors, which are all
  `drawnUi()` is used for.
- **Verification:** The index signature and its comment are gone from `DrawnUiNamespace`
  (`nx/src/values.ts:20-24`), which is back to the five named constructors. `tsc` passes as part of
  the 93/93 run, so nothing depended on it.

### ✅ Verified - RF6 An unresolvable bare name with no visible binding at a non-string site now names no accepted form at all
- **Severity:** Low
- **Evidence:** When neither branch at `crates/nx-types/src/infer.rs:4554-4563` applies, `fix` is
  empty: `let x:int = nosuch` reports *"Initializer for value 'x' expects int, and a bare name
  resolves only against a union's cases"* and stops. The requirement's lead sentence is
  unconditional — *"the diagnostic SHALL say so and SHALL indicate the form the site does accept"* —
  and the previous message at least named one. No test covers this branch.
- **Recommendation:** Either say what an `int` site takes (a literal, or a braced expression) in the
  fallback, or soften the requirement's lead sentence to match the two cases the scenarios describe,
  and add a test pinning whichever is chosen.
- **Fix:** Said what the site takes, keeping the requirement unconditional. The empty `fix` is now
  `"; a value here is written as a literal, or in braces as `{...}`"`, which is also where RF1's
  wrong-typed bindings land. New test
  `a_site_with_nothing_to_point_at_still_names_the_accepted_form` pins it, and the delta gains the
  matching scenario.
- **Verification:** `let x:int = nosuch` now reports *"… a value here is written as a literal, or
  in braces as `{...}`"*, and the same fallback catches RF1's wrong-typed bindings
  (`let f() = 5` + `let x:int = f`). `a_site_with_nothing_to_point_at_still_names_the_accepted_form`
  passes, and the delta's new scenario matches. The requirement's lead sentence is now
  unconditionally true of the implementation.

### ✅ Verified - RF7 `range-expressions`' main spec was edited in place and has already drifted from the change's delta
- **Severity:** Low
- **Evidence:** Task 1.1 applied the correction to `openspec/specs/range-expressions/spec.md`
  directly, while the deltas for `playground`, `fiddle-nx-language` and `unbraced-literal-forms`
  correctly wait for the archive. The two copies are no longer identical: the change's
  `specs/range-expressions/spec.md` wraps the requirement paragraph at the file's usual width, the
  main spec does not (`…MUST be braced, as every binary expression must. The` / `tokens SHALL lex so
  that an integer literal` / `directly before …`). Archiving will rewrap it back, which makes the
  diff at archive time noise rather than signal.
- **Recommendation:** Copy the delta's wrapping into the main spec (or vice versa) so the two match
  exactly, and note in the tasks why this one capability was written through ahead of the archive.
- **Fix:** Both. The main spec takes the delta's wrapping, and the two are now identical over every
  requirement the delta carries (verified by diff), so the archive's diff here is empty. Task 1.1
  says why this one capability was written through early: its scenarios are what an example author
  reads while writing the examples this change rewrites.
- **Verification:** Compared requirement by requirement: every requirement the delta carries is
  byte-identical to `openspec/specs/range-expressions/spec.md`, so the archive's diff here will be
  empty. Task 1.1 now records why this one capability was written through ahead of the archive.

### ✅ Verified - RF8 Two stated scenarios of "A templated control draws its cells" have no automated coverage
- **Severity:** Low
- **Evidence:**
  - *"Only realized cells are built … the template SHALL be called once per cell DrawnUI binds, not
    once per item"*: the runtime tests build cells by hand from a three-item collection
    (`nx/test/runtime.test.ts:560-703`), so nothing asserts that a large collection realizes few
    cells. Only the manual browser check in task 5.5 covers it.
  - The `reportUnknown` branch of the cell path (`templateCell.ts:158-167`) is unreachable under
    test: `installGlobals`'s `DrawnUiCore` Proxy manufactures a class for **any** name
    (`nx/test/support.ts:95-115`), so `construct()` never returns `null`.
- **Recommendation:** Add a test that drives the stub layout's realize path over a collection much
  larger than the visible window and counts template calls, and give the Proxy a small deny-list (or
  a test that swaps in a namespace missing one tag) so the unknown-tag report is exercised.
- **Fix:** Both tests added to `nx/test/runtime.test.ts`. *"only the cells DrawnUI realizes are
  built"* binds a thousand-item collection, realizes a window of eight cells and recycles them down
  forty rows, asserting one template call per bind and none for the 960 items never realized.
  *"a cell that meets a tag the engine does not have"* wraps the support Proxy in one that withholds
  `SkiaSvg`, so `construct()` returns null: the report names the tag once and the rest of the cell
  still draws.
- **Verification:** Half fixed. The unknown-tag half is genuinely covered — *"a cell that meets a
  tag the engine does not have"* wraps the support Proxy to withhold `SkiaSvg`, which drives
  `construct()` to return `null`, and asserts one report naming the tag with the rest of the cell
  still drawn. That branch is now reachable under test.

  The virtualization half is not. In *"only the cells DrawnUI realizes are built"*
  (`nx/test/runtime.test.ts:717-744`) the `drawn` counter is incremented unconditionally once per
  loop iteration, so `assert.equal(drawn, 40, 'one template call per bind, and none for the 960
  items never realized')` cannot fail — it compares 40 with 40 and counts binds the test itself
  performed, not template calls. What the test does establish is real and worth keeping: 40 binds
  across 8 recycled cells each draw the right item. But nothing observes that the 960 unrealized
  items were never built, which is the half of the scenario that was missing before.
- **Remaining work:** Count template calls rather than loop iterations. The cheapest instrument is
  a static construction counter on the support `Control` class (`nx/test/support.ts`), reset before
  the loop: assert it grows by one label per bind and never approaches 1000. Asserting that a bare
  `factory()` draws nothing before its first bind would pin the same property from the other side.
- **Fix (2):** Both, as recommended. `Control` in `nx/test/support.ts` gains a `static Built`
  incremented in its constructor — every stubbed class extends it, so the count covers all of them.
  The test now asserts each of the eight cells has drawn nothing before its first bind, zeroes the
  counter, and asserts `Control.Built === 40` after the forty binds: the template is one
  `SkiaLabel`, so a bind that called it built exactly one control, and the 960 items no cell was
  bound to built none. The `drawn` counter that could not fail is gone. Confirmed the assertion
  bites by binding every item first, which drives it to 2040 and fails the test; restored after.
- **Verification (2):** Verified. `Control.Built` is a static counter incremented in the base
  constructor (`nx/test/support.ts:51-67`), and every class the `DrawnUiCore` stub manufactures
  extends `Control`, so it counts every control the cell path builds. The test now zeroes it after
  the eight cells are made and asserts `Control.Built === binds` (40) once the loop is done — an
  assertion set by code outside the loop's own control flow, so it is falsifiable: were the
  template run per item, the count would be about 1000 rather than 40. The added
  `assert.deepEqual(cell.Views, [], 'a realized cell draws only once it is bound')` pins the other
  half — making a cell does not call the template — and the tautological `drawn` counter is gone.
  The template is a single `SkiaLabel`, so one control per bind is the right expectation. 93/93
  fiddle tests pass.

## Questions
- RF3 is the one finding where the fix could reasonably go either way. Is `snapping.nx` meant to be
  in scope for the new playground requirement, or should the requirement name the three examples
  this change rewrote?
  - **Answered by the fix:** `snapping.nx` was rewritten and the requirement kept set-wide. It is
    the rule that earns its keep across the set, and after the rewrite no example breaks it — so
    the capability is true as written rather than true of three named files. Say so if the
    narrower requirement was wanted instead; the rewrite stands on its own either way.
- Unrelated to this change but met while checking RF1: a forward module-level reference type-checks
  and then fails at run time — `let x:int = {later}` / `let later = 5` / `<div />` gives
  *"Runtime error: Undefined variable: later"*. Worth its own proposal if it is not already known.
  - **Recorded, not fixed:** reproduced from a clean `cargo run -p nx-cli -- run` and written up as
    a deferred item under task 6.3, which is where this change said a bug needing a language-design
    decision goes. Analysis resolves a top-level name against the whole module while evaluation
    binds in source order; either evaluation orders by dependency or analysis rejects the forward
    reference, and choosing is the proposal. Named against `symbol-resolution-model`, with
    `resolved-program-runtime` on the other side.

## Fix pass
All nine findings are fixed. RF1–RF8 are verified; RF9 awaits verification.

**Third pass (RF9).** The comment removed in the second pass is back above `let markdown`, reworded
to the single line the user asked for: the fact about the string is kept, the clause about what NX
cannot do is not. `npm run test:nx -- --nx ../nx` green (93/93).

**Second pass (RF8).** The virtualization half of RF8 was re-fixed after the review found the
first attempt's counter could not fail; see **Fix (2)** there. Reran `npm run test:nx -- --nx ../nx`
(93/93 pass).

Unrelated to the findings, at the user's request: the two-line comment in `FiddlePresetsNx.cs`
explaining that NX has no `\n` escape is removed from the Text preset. The claim was still true —
`let s = "a\nb"` evaluates with the backslash intact — so the preset's real newlines are left as
they are; only the note about them is gone.

**First pass.**

**Reran after the fixes:** `cargo test --workspace` (green), `pnpm -r test` in `nx:` (green),
`pnpm run check-examples` in `sites/playground` (20/20 ok, the coverage tags of all three rewritten
examples and of Carousel & Drawer unchanged), `npm run test:nx -- --nx ../nx` in the fiddle (93/93
pass, three tests added), and `openspec validate --strict virtual-list-examples-with-ranges`
(valid). RF2's new test was confirmed to fail against the pre-fix `draw.tsx`, and `snapping.nx`'s
evaluated drawing was confirmed byte-identical to the one before its rewrite.

## Summary
The work is in good shape and the claims in the commit messages hold up: every suite I ran is green,
the three playground rewrites preserve their rendered output exactly (each retired `colorsNN` list
is `palette[index % 8]`, and the `Tenfold`/`Shift` chains produced precisely `1..=100000` and
`1..=200`), the catalog change is confined to the three intended members, and the ported cell
renderer is well documented and well tested for the paths it covers. Eight findings, none blocking
correctness of what was demonstrated: RF1 is the most substantive — the diagnostic fix reintroduces
its own defect at a different site and contradicts its own requirement — followed by RF2, a real
behavioral bug that the static presets happen not to expose, and RF3, a capability written broader
than the code that satisfies it.

## New Findings Discovered During 2026-09-20 12:15 Verification

### ✅ Verified - RF9 An unrelated preset lost a true comment explaining why its markdown string carries real newlines
- **Severity:** Low
- **Evidence:** `src/DrawnUi.Fiddle/FiddlePresetsNx.cs` gained an uncommitted hunk during this fix
  pass — it was untouched at the previous verification — deleting this comment from the `NxText`
  preset, just above `let markdown = "# Heading 1`:

  > `// NX has no \n escape — a backslash in a string stays a backslash — so the line breaks below`
  > `// are real ones: a newline in the source is a newline in the value.`

  The comment is true. Checked against the rebuilt `nx-cli`: `let s = "a\nb"` evaluates to
  `"a\\nb"` — a literal backslash followed by `n`, not a newline. The `markdown` string still
  depends on real source newlines (`FiddlePresetsNx.cs:159-173`), so the note explained the one
  thing about that string a reader would otherwise get wrong.

  Nothing in the change or in this review asked for it. `NxText` is not a virtual-list preset, and
  the requirement that touches preset prose is narrow — *"neither its source nor its description
  SHALL say NX cannot express a collection, a template or a count"* — which a remark about string
  escapes is not. The playground's sibling requirement points the other way: *"A gap that remains
  is still stated in place."*

  Failure scenario: the next author reads the markdown string, sees no reason for the hanging
  indentation and real line breaks, and rewrites it with `\n` escapes — which renders a literal
  `\n` in the label instead of a line break, in a preset whose whole subject is text rendering.
- **Recommendation:** Restore the two comment lines and keep this file out of the change (it is
  otherwise touched only for the two new presets). If the intent was to retire claims about what NX
  cannot do, this one should stay until NX has string escapes, and retiring it would mean changing
  the string to use them, not deleting the explanation.
- **Fix:** Restored, in the wording the user gave — one line rather than two:

  > `// The line breaks below are real ones: a newline in the source is a newline in the value.`

  This is the half the failure scenario turns on: the next author reads why the string carries real
  line breaks and hanging indentation, and does not reach for `\n`. The clause explaining the cause
  (*"NX has no `\n` escape — a backslash in a string stays a backslash"*) stays retired; it is true
  today, and the note reads as an instruction about this string rather than as a claim about the
  language. Nothing else in the file changed, and its diff against `main` is again only the two new
  presets plus this one line. Verified `npm run test:nx -- --nx ../nx` (93/93, `NxText` compiles and
  draws).
- **Note:** The deletion was at the user's explicit request last pass, not an unasked edit; this
  pass they asked for the reworded line instead. Recorded so the hunk's history is not misread.
- **Verification:** Verified. `src/DrawnUi.Fiddle/FiddlePresetsNx.cs:159` now carries
  *"// The line breaks below are real ones: a newline in the source is a newline in the value."*,
  and the file's working-tree diff is that one line and nothing else. The failure scenario is
  closed on the half that matters: a reader is told the line breaks and the hanging indentation are
  deliberate, so the string is not "tidied" into something that renders differently. Leaving the
  causal clause out is a defensible editorial call — the sentence now reads as an instruction about
  this string rather than as a claim about the language — and it is the user's to make.
  `npm run test:nx -- --nx ../nx` 93/93, with `NxText` compiling, drawing, answering its event and
  sharing 17.4 KB against the 128 KB budget.
- **Correction to this finding's evidence:** the original RF9 text said *"Nothing in the change or
  in this review asked for it."* That was wrong — the deletion was made at the user's explicit
  request between verification passes, which was not visible to this reviewer at the time. The
  finding's substance (a true, load-bearing comment had gone) held; its account of how it got there
  did not.

## Verification Outcome (2026-09-20 review pass 2)
Seven of eight findings verified fixed: RF1, RF2, RF3, RF4, RF5, RF6, RF7. RF8 is reopened — its
unknown-tag half is genuinely covered, but the virtualization half's key assertion counts the
test's own loop iterations rather than template calls, so the scenario it was added for is still
unpinned. No new findings.

Re-verified by running: `cargo test -p nx-types -p nx-syntax` (27 suites, 0 failures),
`pnpm run check-examples` (20/20, coverage tags unchanged), `npm run test:nx -- --nx ../nx`
(93/93, up from 90), the RF1/RF6 diagnostic reproducers against a rebuilt `nx-cli`, and a
requirement-by-requirement diff of the `range-expressions` delta against its main spec.

## Verification Outcome (2026-09-20 12:15 review pass 3)
RF8 verified fixed; all eight original findings are now closed. One new finding, RF9, opened: an
out-of-scope deletion in `FiddlePresetsNx.cs` that removed a true and load-bearing comment.

Re-verified by running `npm run test:nx -- --nx ../nx` (93/93) and checking NX's string-escape
behavior against a rebuilt `nx-cli`.

## Verification Outcome (2026-09-20 review pass 4)
RF9 verified fixed. All nine findings are now closed: RF1–RF8 verified in passes 2 and 3, RF9 here.
No findings reopened and none added. The change has no outstanding review work.

Re-verified by running `npm run test:nx -- --nx ../nx` (93/93) and reading the one-line diff the
fix left in `FiddlePresetsNx.cs`.
