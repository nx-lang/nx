# Review: update-fiddle-to-latest-nx

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md` (52/52 done),
`specs/fiddle-nx-language/spec.md`, `specs/editor-language-service/spec.md`;
`openspec validate --strict` passes.

**Reviewed code:**

- This repository (working tree, uncommitted): `crates/nx-syntax/src/syntax_node.rs`,
  `crates/nx-syntax/tests/fixtures/valid/function.nx`, `crates/nx-language-service/src/lib.rs`,
  `bindings/wasm/test/{exports,sdk-wasm,trap}.test.ts`, `specs/future.md`.
- `~/src/DrawnUi.FiddleEngine` (branch `nx-linkable-ir`, twelve commits `789bc82..14adb5f` over
  `02fa142`): `nx/generate-catalog.mjs`, `nx/catalog/*`, `nx/src/{instances,draw,values,runtime}.ts*`,
  `nx/test/*`, `dev/build-nx-runtime.mjs`, `nx/catalog-artifact.mjs`,
  `src/DrawnUi.Fiddle/FiddlePresetsNx.cs`, `README.md`, `AGENTS.md`, `nx/CATALOG.md`.

**Re-run here:** `cargo test -p nx-syntax -p nx-language-service` green;
`npm run test:nx -- --nx ../nx` in the fiddle green (73/73); the README's preset size table matches
what the suite prints today, to the tenth of a KB. `instances.ts` diffed against
`sites/playground/src/render/instances.ts`: the only differences are imports, quoting, `parentOption`
and two non-null assertions, as decision 1 requires. Not re-run here: the browser tasks (§7, 9.5,
10.4, 11.6) and `dotnet build`/`dotnet test`.

## Findings

### ✅ Verified - RF1 An event that arrives before the redraw commits is dropped silently, and nothing records it
- **Severity:** Medium
- **Evidence:** `nx/src/draw.tsx:133-137`: the callback captures `drawn = source.instance` and
  returns early when the instance has been replaced. A dispatch always replaces it
  (`nx/src/instances.ts:307`), so every event delivered between a dispatch and React's commit is
  swallowed with no dispatch and no report — the only case in the whole runtime where something a
  visitor did produces neither a drawing change nor a console line. This is not merely a
  sub-frame race: a radio group fires `Toggled` on both buttons within one gesture, so the second is
  *deterministically* lost, and the Controls preset works around it in prose
  (`src/DrawnUi.Fiddle/FiddlePresetsNx.cs:565-567`: "both events arrive in one gesture and the
  renderer runs only the first"). Neither the delta spec ("Authored handlers run in the editor and
  in the player") nor design.md's risk list mentions the loss, so the one place it is written down is
  a comment inside a preset.
- **Recommendation:** Decide and record the behavior. Cheapest: report the drop through `report`
  (it is exactly the kind of thing the console pane is for) and add it to the delta spec as a
  scenario, plus a line in design.md. Better, if it holds: check whether a re-initialized instance
  keeps its handler tokens (`source.instance.handlers.get(token)`) and, when it does, dispatch
  against the current instance instead of dropping — that makes the radio pair behave and lets the
  preset's comment go away. Whatever is chosen belongs in the playground's copy too
  (`sites/playground/src/render/DrawnTree.tsx:87-92`).
- **Fix:** Reported rather than dropped. `draw.tsx` gains `reportStale` on the draw context and
  calls it where the guard returns, and `runtime.ts` writes it to both channels every time it
  happens (not once per program: it is this tap that did nothing, as a failed dispatch is). The
  better option does not hold: `ir-runtime` numbers tokens per generation (`h<gen>-<n>`,
  `runtime/typescript/src/index.ts:2319`), so the instance that replaced this one holds no handler
  under the old token and re-dispatching would fail rather than run the second event. The delta spec
  says the event SHALL NOT run and SHALL be reported, with a scenario for the radio pair; design
  decision 3 records why it cannot be re-dispatched; the fiddle's README says which reports are said
  once and which each time; the Controls preset's comment now ends "saying in the console that the
  second was too late". The playground's copy reports it to its console — an event is not a property
  of a drawing, so it belongs in neither list its panel shows. Test: the stale-callback test asserts
  one warning naming `SkiaButton.onTapped`.
- **Verification:** Confirmed. `nx/src/draw.tsx:135-142` calls `reportStale` where the guard returns
  and `nx/src/runtime.ts:453-459` writes it to both channels on every occurrence; the renamed
  stale-callback test asserts exactly one warning naming `SkiaButton.onTapped`, and passes. The
  reason re-dispatching was rejected checks out at the source: `runtime/typescript/src/index.ts:2319`
  builds tokens as `h${generation}-${n}` into a handlers map rebuilt per generation, so an old
  token is genuinely absent from the instance that replaced it. Spec scenario "A second event in one
  gesture is reported, not dropped", design decision 3, the fiddle's README (which now separates
  what is said once per program from what is said per event), the Controls preset comment and the
  playground's `DrawnTree.tsx`/`useNxDrawing.ts` all carry it. The playground typechecks.

### ✅ Verified - RF2 `children()` skips comments but the other named-child accessors still return them
- **Severity:** Medium
- **Evidence:** `crates/nx-syntax/src/syntax_node.rs:46-58` filters comments out of `children()`,
  with a comment stating the invariant ("every caller here reads children as the constructs a node
  is made of"). `child(index)` (`:113`), `child_count()` (`:108`), `next_sibling()` (`:84`) and
  `prev_sibling()` (`:91`) go straight to tree-sitter's named-child API and are untouched, so the
  invariant holds for one of five accessors. `crates/nx-language-service/src/positions.rs:245`,
  `:269`, `:502` and `:583` read a property's name as `child(0)` — the same "first named child is
  the construct I want" assumption that produced the bug this change fixed. Nothing in `nx-hir`
  uses index or sibling access today, so no current defect is demonstrated; the risk is the next
  caller.
- **Recommendation:** Filter comments in `child`, `child_count`, `next_sibling` and `prev_sibling`
  too (`child(i)` then means the i-th non-comment child, which is what every caller means), or, if
  they are deliberately raw, say so in their doc comments and point at `children()` for the
  construct view. Either way add the fixture-style test for one index/sibling accessor next to
  `children_skips_comments_and_children_with_tokens_keeps_them`.
- **Fix:** `child`, `child_count`, `next_sibling` and `prev_sibling` read the view `children()`
  reads, so index `i` is the i-th construct and a comment never stands between two siblings; the
  doc comment on `children()` says the four share it. `index_and_sibling_access_skip_comments` in
  `syntax_node.rs` covers a commented list (it fails on the old accessors: `child_count()` counted
  4 where 2 are constructs). `cargo test --workspace` green, the four `positions.rs` callers
  included.
- **Verification:** Confirmed. `child` is `children().nth(index)`, `child_count` is
  `children().count()`, and both sibling walkers loop past comments
  (`crates/nx-syntax/src/syntax_node.rs:84-131`); the doc comment on `children()` states that the
  four share its view. `index_and_sibling_access_skip_comments` exercises a list commented before
  and between its elements and asserts the count, both indices and both directions.
  `cargo test --workspace` green.

### ✅ Verified - RF3 An unrelated fixture edit sits in the working tree that the change's close-out would commit
- **Severity:** Low
- **Evidence:** `crates/nx-syntax/tests/fixtures/valid/function.nx` drops `: Element` from
  `let <Button ... />: Element =`. No task in this change asks for it, and tasks.md's "What remains"
  enumerates exactly what the NX-side commit should carry (wasm SDK test expectations, the language
  service's handler properties, `children()` skipping comments, `specs/future.md`, the artifacts) —
  this is not on the list; it reads as spillover from `add-function-types` /
  `infer-unannotated-return-types`. It also passes silently: `parser_tests.rs:47-57` only asserts the
  fixture parses, and after the edit no fixture under `tests/fixtures/valid/` exercises `/>:` at all
  (only the inline source at `parser_tests.rs:309` does).
- **Recommendation:** Revert it before committing the NX side, or move it to the change that wants
  it. If the removal is intended, restore the return-type coverage in a fixture.
- **Fix:** Reverted; `cargo test -p nx-syntax` is green with the annotation restored, so nothing in
  this change wanted the edit.
- **Verification:** Confirmed. `git diff crates/nx-syntax/tests/fixtures/` is empty, so
  `/>: Element` is back, and the workspace suite is green with it.

### ✅ Verified - RF4 A program that fails to link or evaluate re-reports on every render
- **Severity:** Low
- **Evidence:** `nx/src/runtime.ts:450-484`: when the `try` block throws, `held.current` is never
  assigned, so the next render repeats prepare → link → evaluate → `report('error', ...)`. Every
  other notice is deliberately said once per prepared program (`:406-438`, and task 5.1's last
  bullet records that the triple inert notice was worth fixing), but the failure line is outside
  that mechanism, so a hot reload plus the run plus React's development double render prints it
  three times, and any later re-render prints it again. The existing test for this path
  ("a share from another catalog version draws when its names resolve") renders once, so nothing
  catches it.
- **Recommendation:** Route the failure through the same `reported` set (key on the message), and
  extend the version-mismatch test to render twice and assert one line.
- **Fix:** A failed image is now held where a session would be (`Failed`), so the render after it
  neither links again nor draws a new failure label, and the message is said once per image through
  a mount-level `failures` set — the notice cannot hang on a prepared program, because there is
  none. The version-mismatch test renders twice and mounts the same image again, and asserts one
  line; it fails on the old code with three.
- **Verification:** Confirmed. The failed image is held as `Failed` and returned on later renders
  (`nx/src/runtime.ts:505-512`), and the message is deduplicated in a mount-level `failures` set;
  the version-mismatch test now renders twice and mounts the image a third time, asserting one error
  line. Nit accepted, not reopened: the key uses the literal `(image)` for a `Uint8Array` mount, so
  two different byte images failing with the same message would report once — the editor and the
  player both mount the base64 string, so nothing reaches it today.

### ✅ Verified - RF5 A host effect names the component, not the instance the spec promises
- **Severity:** Low
- **Evidence:** `nx/src/instances.ts:312` builds `{ instance: node.component, ... }`, so
  `describeEffect` prints `from 'Card'`. The delta spec says "reported with the action and the
  instance that produced it" and its scenario says the report "SHALL name the action, its fields and
  the instance that produced it". With five `Card`s on screen (the Controls preset) every unhandled
  effect reads the same, and `InstanceNode.key` — the position path that would distinguish them — is
  right there.
- **Recommendation:** Either carry `node.key` alongside the component name in `HostEffect` and print
  it, or reword the spec scenario to say the component. Keep the playground's copy in step.
- **Fix:** The spec now says the component, in the requirement text and in the scenario, and design
  decision 3 says why: the instance key is a position path through a drawing (`root.1.0/body`),
  which tells a visitor reading the console less than the component's name does, and keeping it out
  of `HostEffect` keeps `instances.ts` diffable against the playground's copy as decision 1 asks.
- **Verification:** Confirmed as a spec fix, which was one of the two options offered. The
  requirement and both scenarios now say "the component that produced it"
  (`specs/fiddle-nx-language/spec.md`), and design decision 3 gives the reason. `HostEffect.instance`
  still holds a component name, which keeps `instances.ts` byte-comparable with the playground's
  copy as decision 1 asks; the field name reads oddly against the spec now, but no behavior or
  wording is wrong.

### ✅ Verified - RF6 The generator's event logic is only tested through its committed output
- **Severity:** Low
- **Evidence:** Task 2.1 asks for "a generator test"; `nx/test/catalog.test.ts` instead reads back
  `drawnui.nx`, `catalog-meta.json` and `omitted.json` from disk. That catches a stale commit but
  not a regression in `readEvent`/`eventSignature` until someone runs `npm run catalog`, and it
  leaves the one hard-failure path — "a dropped parameter precedes a kept one, which the metadata
  cannot express" (`nx/generate-catalog.mjs:217`) — with no coverage at all, although that is the
  branch that protects the by-position argument mapping the renderer depends on. Separately,
  `nx/test/runtime.test.ts:422-427` keys `stubArguments` by event name alone, parsed out of the
  catalog text with a regex, so two components declaring an event of the same name with different
  payloads would silently share the last one's stubs.
- **Recommendation:** Either accept the output-level test and say so in tasks.md, or add a small
  direct test over `readEvent` covering a dropped-then-kept parameter list. Key `stubArguments` by
  component and event.
- **Fix:** Both halves. `readEvent` is exported (with `main()` guarded so importing the generator
  does not run it) and driven directly by `nx/test/generator.test.ts` over stub types: a primitive
  parameter becomes a field, an engine object is dropped, and a kept parameter after a dropped one
  throws naming the event — the rule that has no output to read back. `nx/test/run.mjs` keeps the
  generator and `typescript` out of the bundle, as it already did the resolver. The stub arguments
  stay keyed by event name, and a new test asserts no two catalog events share a name and differ in
  payload, so the day one does is a failure that names both payloads instead of a silent
  substitution.
- **Verification:** Confirmed, both halves. `readEvent` is exported and driven directly by
  `nx/test/generator.test.ts` over stubbed checker/signature objects: primitive parameters become
  fields, the sender is skipped, an engine object is dropped, and a kept parameter after a dropped
  one throws naming `SkiaControl.Panned`. The collision guard asserts no two catalog events share a
  name and differ in payload. I re-ran `npm run catalog -- --nx ../nx` end to end: the `main()`
  guard does not break the script (43 emits, 72 KB image) and the committed catalog is regenerated
  byte-identical, so the export did not disturb the generator.

### ✅ Verified - RF7 The editor-language-service delta adds a requirement that overlaps two existing ones
- **Severity:** Low
- **Evidence:** `specs/editor-language-service/spec.md` adds "Language service offers and describes
  handler properties", whose scenarios restate behavior already required by "Language service
  exposes conservative completions" (`openspec/specs/editor-language-service/spec.md:286`, scenario
  "Component property position includes property completions": "SHALL include undeclared properties
  accepted by that component … SHALL NOT include properties already supplied") and by "Language
  service exposes hover information". Since the implementation makes `on<Emit>` an ordinary
  `PropertyDeclaration` (`crates/nx-language-service/src/lib.rs:2001-2017`), the new requirement is a
  clarification of the old ones, and the two can now drift about, say, filtering supplied properties.
- **Recommendation:** Fold the three scenarios into the existing completions and hover requirements
  as MODIFIED, or keep the requirement and open it with a sentence saying it refines them rather
  than adding a second rule.
- **Fix:** The requirement opens by saying it refines "Language service exposes conservative
  completions" and "Language service exposes hover information" for one kind of property, and
  defers to their terms ("on the terms those two requirements already set for a declared property")
  instead of restating the offer-and-filter rule. Its three scenarios stay, as the emit-specific
  coverage. Folding it into two MODIFIED requirements was the alternative; it would have meant
  restating both in full, and the hover requirement is some forty lines whose every clause would
  have to be copied exactly.
- **Verification:** Confirmed as a spec fix, the second option offered. The requirement opens by
  naming the two requirements it refines and defers to their terms rather than restating the
  offer-and-filter rule; the three emit-specific scenarios remain.
  `openspec validate update-fiddle-to-latest-nx --strict` passes.

## Questions

- `programs` (`nx/src/runtime.ts:365`) is keyed by the share string and never evicted, so a long
  editing session retains one prepared+linked program per distinct compile. Design decision 2 keeps
  this "as today" deliberately — is it worth a cap now that a program also anchors an instance tree's
  lifetime through the mounted session?
  - **Answer:** Left as it is. A cap needs an eviction that cannot drop a program a mounted session
    still dispatches through, which is a design question rather than a review fix. Noted here so it
    is decided before the fiddle's pull request. (The `failures` set added for RF4 grows the same
    way and by the same bound: one entry per distinct image.)
- Re-running an *unedited* snippet produces an identical `ir` string, so `held.current.ir !== ir` is
  false and the instances keep their state. The spec's "Running again resets the instances" is
  written about an edit. Is keeping state on an unchanged re-run the intended reading?
  - **Answer:** Yes. The image is the identity of the program, and a run that changes nothing is the
    program that is already mounted; discarding the instances would throw away a visitor's state for
    pressing Run. The spec's scenario says "the visitor edits the snippet and it runs again", which
    is the case that resets, and the test "two mounts of one share keep separate state, and another
    image at the same mount starts again" is written on the same terms.
- `catalog.test.ts` asserts `SkiaCheckbox` restates no emit because "a redeclared emit is an error in
  NX as a redeclared prop is". Is that checked anywhere in NX's own tests, or is the fiddle the only
  place that would notice if it stopped being true?
  - **Answer:** It was the only place. `DuplicateInheritedEmit` (`crates/nx-hir/src/components.rs:227`)
    had no test, where the prop, field and type-parameter redeclarations each have one.
    `redeclaring_an_inherited_emit_is_rejected` now covers it next to them.

## Summary

The change is in good shape and the claimed verification holds up where it can be re-run here: both
suites are green, the ported instance tree is a faithful copy of the playground's, the catalog
generator's event handling matches what the delta spec requires, and the README's numbers are
current. Coverage is unusually thorough for the interesting paths — dispatch, emit-to-parent, atomic
failure, the player path without a compiler, both console channels, and a generic per-preset dispatch
that will fail loudly if a preset loses its interaction.

Nothing found is a blocker. RF1 is the one with user-visible consequences: the dispatch model drops
an event that arrives before the redraw commits, deterministically so for a radio group, and the only
record of it is a comment inside a preset — that belongs in the spec and design, and should probably
be reported to the console at least. RF2 is a completeness question about the `nx-syntax` fix rather
than a defect. RF3 is working-tree hygiene to settle before the NX-side commit the close-out calls
for. The rest are small.

## Fixes

All seven findings were addressed; each is marked `🟡 Fixed` above with what was done, and
`tasks.md` §12 records the same list. The suites re-run after them:

- `nx:` `cargo test --workspace` green (with the two new tests: the index and sibling accessors, and
  the redeclared inherited emit), `pnpm -r test` green, `sites/playground` typechecks.
- Fiddle: `npm run test:nx -- --nx ../nx` green, 77 tests (73 before: the generator's three and the
  event-name collision guard are new).
- Not re-run: the browser tasks (§7, 9.5, 10.4, 11.6) and `dotnet build`/`dotnet test`. Two fixes
  change what a browser run would show — the console line for a stale event, and a failure reported
  once instead of on every render — so both are worth a look when the fiddle is next opened.
- RF5 was answered in the spec rather than in the code, and RF7 by reframing a requirement rather
  than folding it into two MODIFIED ones; both alternatives are recorded with the findings.

## Verification 2026-09-18 01:47

All seven findings verified as fixed; none reopened, no new findings. Each finding carries its own
`- **Verification:**` note above. Re-run independently for this pass:

- `nx:` `cargo test --workspace` green, including `index_and_sibling_access_skip_comments` and
  `redeclaring_an_inherited_emit_is_rejected`; `pnpm -r test` green (the playground's 20 examples
  included); `sites/playground` `npm run typecheck` clean, which is what holds its new `reportStale`
  to the same contract as the fiddle's; `openspec validate --strict` passes.
- Fiddle: `npm run test:nx -- --nx ../nx` green, 77 tests (73 at the first review, +3 generator, +1
  event-name collision guard); `npm run catalog -- --nx ../nx` re-runs as a script and reproduces
  the committed catalog byte for byte.
- Read at the source rather than taken from the fix note: the per-generation handler tokens that
  make RF1's "report, do not re-dispatch" the only option
  (`runtime/typescript/src/index.ts:2319`).
- Still not run here, as at the first review: the browser tasks (§7, 9.5, 10.4, 11.6) and
  `dotnet build`/`dotnet test`. RF1 and RF4 change what a browser run shows, so the note in "Fixes"
  about looking at the console when the fiddle is next opened still stands.

Two things to carry forward rather than findings: the fiddle's fixes are in its working tree and not
yet committed to `nx-linkable-ir` (the close-out expects local commits), and the `programs`/`failures`
growth question above is still open by decision.
