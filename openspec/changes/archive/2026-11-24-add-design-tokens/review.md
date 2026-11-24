# Review: add-design-tokens

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/design-tokens/spec.md, tasks.md  
**Reviewed code:** `crates/nx-program/src/std/design/{types,palette,tokens}.nx`,
`crates/nx-program/src/artifacts.rs`, `crates/nx-program/src/standard_library_tests.rs`,
`crates/nx-api/tests/program_standard_library_tests_evaluation.rs`,
`crates/nx-api/tests/codegen/ir_corpus_tests.rs`, `crates/nx-cli/src/typegen/model.rs`,
`crates/nx-cli/src/main.rs` (new typegen test), `specs/ir-conformance/design-tokens/`,
`specs/ir-conformance/design-tokens-theme/` (sources, `program.json`, `results.json`,
`operations.json`), `specs/ir-conformance/manifest.json`, and the docs
`reference/libraries/design.md`, `reference/style-guide.md`, `reference/syntax/modules.md`,
`tutorials/working-with-design-tokens.md`.

**Checks run:** `cargo test -p nx-api --test codegen_parity ir_corpus` (10 passed, without
`NX_UPDATE_CORPUS`), `cargo test -p nx-program design` (4 passed),
`cargo test -p nx-api --test program_standard_library_tests_evaluation design` (3 passed),
`cargo test -p nx-cli typegen_design` (1 passed), and the website code-block check on all four
docs (0 failures). Spot-checked palette values against Tailwind v4 (zinc 50/100, blue/red/green/amber
500): they match.

## Findings

### ✅ Verified - RF1 The spec and reference page say a theme bundle can be applied to a subtree, but the language has no way to do that
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `specs/design-tokens/spec.md:117-118` says a theme is "applied app-wide with
  `provide theme` in the main module or around a subtree", and
  `reference/libraries/design.md:251-252` says "Apply it app-wide with `provide` at the top of the
  main module, or to one section with `env.` attributes". `env.` attributes set single values, not a
  bundle, and `provide brand { … }` around an expression is a syntax error: a check of
  `let root() = { provide brand { env.radius.md } }` fails with `syntax-error: Invalid function
  definition`. `reference/syntax/environment.md` (Bundles) only documents `provide bundle` at the
  top of the main module. The tutorial's heading "4) Apply it app-wide and to one section"
  (`tutorials/working-with-design-tokens.md:105`) has the same implication, though its body only
  changes `env.colorMode` on the section.
- **Recommendation:** Say what is true: a bundle applies app-wide through a top-level `provide` in
  the main module; one section gets individual `env.` overrides (such as `env.colorMode=dark`). Fix
  the spec requirement text, the reference page sentence and the tutorial heading. Or, if applying a
  bundle to a subtree is wanted, record it in `specs/future.md` as later work rather than claim it.
- **Fix:** Spec, reference page and tutorial now say a bundle applies app-wide through the main module's `provide`, and a section changes single values with `env.` attributes or `provide env.x=… { … }`; a bundle for one section is recorded in `specs/future.md`.
- **Verification:** Spec (`spec.md:117-119`), reference page (`design.md:251-253`), tutorial heading and step 4, and task 3.3 now say a bundle applies only app-wide and a section changes single values; `specs/future.md` records a bundle for one section as later work. Correct and complete.

### ✅ Verified - RF2 The tutorial's "whole file" does not compile: it defines `root` twice
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `tutorials/working-with-design-tokens.md:41` says "Each block from here on continues
  the same file", and step 5 (`:130`) tells the reader to paste the whole file into the playground.
  Step 1's block defines `let root()` (`:25-29`) and step 4's fragment defines it again (`:113-120`).
  Concatenating the tutorial's four blocks and running the code-block check gives
  `duplicate-root: Duplicate definition of 'root'` and `duplicate-declaration`. With step 1's `root`
  removed the file passes, so this is the only problem. The per-block check misses it because steps
  2–4 are `fragment` blocks. Lower confidence secondary point: step 5 says the playground shows
  "each button's colors", but the corpus result for the same program
  (`design-tokens-theme/expected/results.json`, `main.nx::root`) shows each `Button` as an
  unexpanded element with only `text`, so the colors are not visible in `root`'s value.
- **Recommendation:** Tell the reader to replace step 1's `root` in step 4 (or name step 1's
  function differently, e.g. `card()`), and reword step 5 to say what the reader actually sees.
- **Fix:** Step 1's function is now `greeting()`, so the whole file has one `root` (checked by concatenating the blocks and running it); step 5 no longer promises button colors.
- **Verification:** Step 1's function is `greeting()`; I concatenated the tutorial's four blocks and the code-block check passes with 0 failures. Step 5 no longer promises button colors.

### ✅ Resolved - RF3 Every token read re-evaluates large defaults, so one component token read costs about 230k operations
- **Severity:** Medium
- **Confidence:** Medium (the counts are recorded facts; whether they matter for hosts is a judgment)
- **Evidence:** `design-tokens/expected/operations.json`: one `env.color` read (`colors`) costs 18,508
  operations in light mode and 26,734 in dark; `contrast` (20 `env.color` reads) costs 369,096,
  about 20 times one read, so nothing is shared between reads. `design-tokens-theme/expected/operations.json`:
  `buttonRadius`, a single `env.button.radius` read whose value the bundle sets outright, costs
  231,969, and `root` costs 60,150 without expanding its buttons. The cause is the library's shape:
  `standardPalette` (`palette.nx:135-191`) is a 55-field record of `c.oklch` calls, and the runtime
  has no caching of top-level `let`s, so each of the ~12 `standardPalette.…` reads in the
  `env.color` default (`tokens.nx:49-92`) rebuilds all 55 colors; `env.text`'s default reads
  `env.font` 21 times (`tokens.nx:193-215`); and a component token like `ui.nx`'s `button`
  evaluates `env.color` twice plus all of `env.text` even when only `radius` is read. A UI that reads
  a few tokens per component will multiply this per element.
- **Recommendation:** At minimum, record the cost and the cause in `specs/future.md` (caching of
  top-level `let` values and/or lazy per-field defaults in the environment mechanism) and mention in
  the design's Risks section. Cheap mitigations inside this change are possible too, for example
  binding the mode's palette colors once per default (a local `let p = standardPalette`) so each
  `env.color` read builds the palette once instead of ~12 times.
- **Fix:** Recorded the costs and their three causes in `specs/future.md` (new "Design Tokens" section) and as a Risk in design.md. Caching top-level values and memoizing defaults change the cost model, so they're left to their own change.
- **Verification:** `specs/future.md` ("Design Tokens: What `add-design-tokens` Left For Later") records the measured costs and the three causes, cross-referencing the existing "Memoizing defaults" entry, and design.md has a matching Risk. Accepted as recorded rather than fixed, per the coordinator's answer.

### ✅ Verified - RF4 The stability contract contradicts the reason given for `.Update` themes
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `spec.md:136` and `reference/libraries/design.md:307` list "adding a role, step or
  field that has a default" as compatible. `reference/libraries/design.md:279-280`, `design.md:165-166`
  and `proposal.md:47-48` say adding a role "breaks code that constructs the whole record". If the
  added field has a record default (as `Shadow`'s fields do), constructing the whole record without
  it still compiles, so the claimed break does not happen for a compatible addition; if it has no
  default, the contract already counts it as breaking. Either way the two statements don't line up,
  and "has a default" is ambiguous between a record field default and the environment value's
  default.
- **Recommendation:** Decide which is meant and state it once: e.g. "adding a role is compatible;
  new roles get a field default so whole-record construction keeps compiling, and themes should still
  use `.Update` so they don't restate every role", or "adding a role is compatible for readers and
  `.Update` users only", and adjust the contract bullet to match.
- **Fix:** The rationale now matches the contract: a role added later gets a field default, so whole-record construction keeps compiling; themes use `.Update` so they name only what they change. Updated design.md, proposal.md, the reference page and the style guide.
- **Verification:** design.md, proposal.md, the reference page and the style guide now give one consistent story: a later role gets a field default, so whole-record construction keeps compiling, and themes use `.Update` to name only what they change. This matches the contract's "adding … that has a default" bullet.

### ✅ Verified - RF5 The reference page describes shadow rendering that no NX renderer does
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `reference/libraries/design.md:291-293` (and `design.md:196-198`) say "Skia's and
  Canvas's blur sigma is half the CSS blur, which the renderer converts". The graphics preparation in
  `crates/nx-graphics/src/lib.rs:717-727` rejects `shadows` as an unsupported feature, and nothing
  in the graphics crates converts a `Shadow`. The statement reads as current behavior.
- **Recommendation:** Phrase it as guidance for renderers ("a Skia or Canvas renderer uses half the
  CSS blur as its sigma"), or drop the sentence until shadows are rendered.
- **Fix:** Phrased as guidance for renderers, with a note that no NX renderer draws shadows yet (reference page and design.md).
- **Verification:** Reference page and design.md now phrase the sigma conversion as renderer guidance and say no NX renderer draws shadows yet. Correct (see RF8 for the line wrapping this edit introduced).

### ✅ Verified - RF6 The palette step-role guidance contradicts the default theme and itself
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `reference/libraries/design.md:191` says "400 to 950: text, depending on the mode
  (700 to 950 on light backgrounds, 400 to 50 on dark)". The range 400–950 excludes the 50 that the
  parenthetical (and the default dark `text.normal`, gray 50) uses, and the default light
  `text.muted` is gray 500 (`tokens.nx:57`), outside 700–950. The same wording is in `design.md:84`.
- **Recommendation:** Write it as "text: 500 to 950 on light backgrounds, 50 to 400 on dark", or
  otherwise make the ranges cover the defaults the page lists two sections earlier.
- **Fix:** Now "500 to 950 for text on light backgrounds, and 50 to 400 on dark" in the reference page, design.md and task 3.1.
- **Verification:** Reference page, design.md and task 3.1 now say "500 to 950 for text on light backgrounds, and 50 to 400 on dark", which covers every default text color (light gray 500/900, dark gray 50/400).

### ✅ Verified - RF7 The theme corpus does not check that the brand reaches the component token or sets `focus`
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `design.md:30-31` states that with the brand applied "the button is green 700 …
  in light mode and green 500 … inside `env.colorMode=dark`, with the brand's pill radius".
  `design-tokens-theme/main.nx` checks `env.color.accent` and `env.button.radius`, but no entrypoint
  reads `env.button.fill` under the theme, and `root`'s result does not expand `Button`, so the
  "theme that sets only core roles restyles every library" claim is only covered by
  `componentFollowsCore` with a `provide`, not through a bundle. Nothing checks the brand's `focus`
  override, although the docs make "set `focus` with `accent`" a rule. Also, `shadowOrdered`
  (`design-tokens/main.nx:74`) compares only `blur`, not offsets.
- **Recommendation:** Add entrypoints `buttonFill` (light and `rootDark`) and `focus` to
  `design-tokens-theme`, and optionally compare `offsetY` in `shadowOrdered`.
- **Fix:** Added `buttonFill`, `buttonText` and `focus` entrypoints, each in light and `rootDark`, to `design-tokens-theme`; `shadowOrdered` also compares `offsetY`.
- **Verification:** `design-tokens-theme` now has `buttonFill`, `buttonText` and `focus`, each with a `rootDark` case. The recorded results are green 700 / green 500 fill, white / gray 950 text and green 600 / green 400 focus, as the brand sets. `shadowOrdered` compares `offsetY` too. `ir_corpus` (10 passed) and the `damage` sweep (2 passed, with the two new programs added to its left-out list because they emit `@nx/color`'s 44 KB image) pass. Minor: the new block sits between `background()` and `darkAccent()`, so `darkAccent` now falls under the button comment rather than "The theme varies by mode" (`design-tokens-theme/main.nx:24-32`); cosmetic, not reopening.

## New Findings Discovered During 2026-11-23 Verification

### ✅ Verified - RF8 The fix edits left prose lines unwrapped, and the palette generator has two names
- **Severity:** Low
- **Confidence:** High
- **Evidence:** The edited sentences were not reflowed to the ~100-character prose width the rest of
  each file keeps: `openspec/changes/add-design-tokens/design.md:165` is 139 characters, followed by
  short ragged lines; the shadow bullet at `design.md:197-199` breaks after "Toolkits without
  spread,"; `reference/libraries/design.md:293` is 119 characters; and
  `tutorials/working-with-design-tokens.md:10` is 107. Separately, the palette generator follow-up
  is `palette(hue)` in `design.md:253` but `paletteFromHue(…)` in the new `specs/future.md` section.
- **Recommendation:** Reflow those paragraphs to the file's width, and use one name for the
  generator in both places.
- **Fix:** Reflowed the stability paragraph and the shadow bullet in design.md, the shadow bullet in the reference page and the tutorial's intro; the generator is `paletteFromHue(…)` in both places. Also moved `darkAccent` back under "The theme varies by mode" and regenerated the corpus.
- **Verification:** `design.md:160-167` and `:197-199`, `reference/libraries/design.md:292-294` and the tutorial intro (`:10-12`) are reflowed, and the generator is `paletteFromHue(…)` in both design.md and `specs/future.md`. `darkAccent` is back under "The theme varies by mode", and `ir_corpus` passes (10 tests) on the regenerated images. One nit is left, not worth reopening: the rename made `design.md:252` 106 characters, a little over the file's other long lines (101-102).

## New Findings Discovered During 2026-11-23 16:30 Review

**Checks run:** `cargo test -p nx-api --test codegen_parity ir_corpus -- --nocapture` (10 passed;
its output lists `design-tokens` and `design-tokens-theme` as skipped by generated JavaScript),
`cargo test -p nx-program design` (4 passed),
`cargo test -p nx-api --test program_standard_library_tests_evaluation design` (3 passed),
`cargo test -p nx-ir-runtime --test damage` (2 passed), `pnpm test` in `runtime/typescript` (the
design programs pass in both images), the website code-block check on the reference page, style
guide and tutorial (0 failures), and scratch programs through `nxlang run`, `nxlang codegen` and
`nxlang typegen @nx/design` (TypeScript and C#).

### ✅ Verified - RF9 Generated JavaScript and TypeScript refuse every program that imports `@nx/design`, and nothing records it
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** The `env.color` and `env.shadow` defaults are match expressions
  (`crates/nx-program/src/std/design/tokens.nx:50` and `:230`, `if env.colorMode is { … }`), which
  executable source codegen refuses. So the whole program is refused, even one that reads only
  `env.space.md`: `nxlang codegen` on `import "@nx/design"` + `let root() = { env.space.md }` with
  `--target javascript` fails with "match expressions are not supported by executable source
  codegen yet" at both lines. `generated_javascript_agrees_with_the_recorded_results` skips both
  corpus programs whole. Task 2.2 says "Check that generated JavaScript gives the same results, or
  record what it lacks in `specs/future.md`" and is checked, but the new Design Tokens section of
  `specs/future.md` (`:2409-2428`), design.md and the reference page say nothing about generated
  code. The theme pattern the spec and docs teach (`color={ if env.colorMode is { … } }`) hits the
  same refusal in a user's own bundle.
- **Recommendation:** Rewrite the two library defaults without a match expression, which codegen
  already supports: `if env.colorMode == ColorMode.dark { <…dark…/> } else { <…light…/> }` (a
  scratch program in this form generates JavaScript that returns the right color under
  `provide env.colorMode=dark`). Then at least programs that use the default theme, and
  `design-tokens` itself, would run as generated JavaScript, and the corpus would check it. Whether
  or not that is done, record in `specs/future.md` what is still out of reach (mode-varying themes
  written with `is`), and say it on the reference page.
- **Fix:** Took a further step than recommended: executable source codegen now emits a match whose patterns are all union cases (`nx-codegen/src/emit.rs`; TypeScript types an exhaustive match's fallthrough as `never`), so the library defaults and a user theme's `if env.colorMode is { … }` both generate. `generated_javascript_agrees_with_the_recorded_results` now runs both design programs; `design-tokens-theme`'s `darkAccent` is listed in `KNOWN_GENERATED_DIVERGENCES` because generated code evaluates the main-module bundle at load. New tests `generated_code_supports_a_match_on_union_cases` (JS and TS) and `generated_javascript_reads_design_tokens_in_both_modes`; `assert_json_values_eq` compares numbers by value. Generated TypeScript for design programs still doesn't type-check (it names `@nx/color`'s `Color` without importing it, a general gap for types from unimported modules). Both gaps are in the reference page's new "Generated code" section and `specs/future.md`'s Design Tokens section.
- **Verification:** `codegen_parity` passes (331), and `generated_javascript_agrees_with_the_recorded_results` now compares 140 entrypoints (was 108), with both design programs run and only `darkAccent` skipped; `environment` is no longer skipped either. The emitter change (`emit.rs:272-293`, `:3772-3777`, `:5706-5708`) accepts a match only when every pattern is a union case (or, as before, every arm is `$bindings`); payload cases already compare by `$type`; the fallthrough is `nxEmpty` (JS) / `(nxEmpty as never)` (TS) and the helper is collected. `nx-codegen` tests (130) pass. The two remaining gaps are on the reference page ("Generated code") and in `specs/future.md`. Correct and complete; the silent dark-section divergence that this now exposes is RF13. Nit: the skipped list prints `main.nx::darkAccent` without its program name, and the edited doc comment at `ir_corpus_tests.rs:1341` is 125 characters, unwrapped.

### ✅ Verified - RF10 A theme can't set one text color through `.Update` records as the docs teach, and the form that works is undocumented
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `text` is the only nested group in `SemanticColors` (`tokens.nx:35`), and
  `SemanticColors.Update`'s `text` field takes a whole `TextColors`:
  `<env.Update color=<SemanticColors.Update text=<TextColors.Update muted=#ff0000 /> /> />` fails
  with "Property 'text' on 'SemanticColors.Update' expects TextColors, found TextColors.Update". A
  theme following `reference/libraries/design.md:251` and `:280-281` ("sets roles through their
  `.Update` records … names only what it changes, and picks up the default of a role the core adds
  later") would therefore restate all three text colors, in each mode, and would get the field
  default, not the mode's default, of a text role added later. Path fields do work (checked):
  `<env.Update color.text.muted=… />`, and in a mode-varying theme
  `color.text={ if env.colorMode is { light => <TextColors.Update muted=… /> … } }` beside the
  `color={ … }` field. Neither the reference page, the tutorial nor the style guide shows them, and
  no corpus entrypoint covers a text role set by a bundle.
- **Recommendation:** Add a sentence and a short example to the reference page's Themes section
  (and the style guide's theme rule) that a nested group is set with a path field such as
  `color.text.muted=` or `color.text=<TextColors.Update … />`, and add an entrypoint reading
  `env.color.text` under such a bundle to `design-tokens-theme`. If `SemanticColors.Update text=`
  should accept a `TextColors.Update`, record that in `specs/future.md` instead.
- **Fix:** Reference Themes section and the style guide's theme rule now say `text` is set with a path field, with examples (`color.text={ if env.colorMode is { … <TextColors.Update muted=…/> } }` and `color.text.muted=…`). `design-tokens-theme`'s brand sets `muted` per mode that way, and new entrypoints `text` and `text#rootDark` check the set role and the kept defaults.
- **Verification:** Reference Themes section and style guide now show the path field, and both pages pass the code-block check (7 and 15 blocks, 0 failures). The brand's `color.text` works: `text` gives muted gray 600 with default normal/disabled in light, and muted gray 300 with the dark defaults under `rootDark`, as recorded in `results.json`.

### ✅ Verified - RF11 The corpus README gives the wrong reason the design programs leave the damage sweep
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `specs/ir-conformance/README.md:41-42` says the three programs are left out because
  they "emit `@nx/color`'s operations image", but `@nx__color__operations.nx.nxir` is 16,356 bytes,
  under the 16,384 limit. What puts them over is `@nx/color/color.nx` (44,096 bytes), and in the two
  design programs also `@nx/design/tokens.nx` (26,492) and `palette.nx` (17,416). The edited
  paragraph also ends on a ragged break ("Every image of every program" / "is still cut …",
  `:44-45`).
- **Recommendation:** Say "the three that emit `@nx/color`'s and `@nx/design`'s images, which are
  above the limit" (or name `color.nx`), and reflow the paragraph.
- **Fix:** README now names `@nx/color`'s `color.nx` and `@nx/design`'s `tokens.nx` and `palette.nx` as the images above the limit, and the paragraph is reflowed.
- **Verification:** README `:41-46` now names `color.nx`, `tokens.nx` and `palette.nx` as the images over the limit, and the paragraph is reflowed. The damage sweep still passes.

### ✅ Verified - RF12 Two small inaccuracies in the library source: the "stable core" header and `Shadow.blur` typed as `Spacing`
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `tokens.nx:3` says "This set is the stable core" directly under the line that says
  the library is unstable; the reference page and design call it the core meant to stay stable.
  `types.nx:50` types `Shadow.blur` as `Spacing`, which the library documents as "Gaps, padding and
  margins" (`types.nx:11`) and whose stated purpose is to let tools "tell spacing from radius"
  (`types.nx:4`); a value-suggestion or export tool keyed on the alias would offer `env.space`
  steps for a blur radius.
- **Recommendation:** Reword the header ("This set is the core, meant to stay stable once the
  library is"), and type `blur` as `float64 where finite && >= 0.0` (as `offsetX` and `spread` are
  plain), or add a dimension alias for it.
- **Fix:** Header reworded to "the core, meant to stay stable once the library is"; `Shadow.blur` is `float64 where finite && >= 0.0`. Corpus regenerated.
- **Verification:** Header reworded; `Shadow.blur` is `float64 where finite && >= 0.0` with aligned comments; corpus images regenerated and `ir_corpus` passes; `nx-program design` (4) and the evaluation `design` tests (3) pass.

## New Findings Discovered During 2026-11-23 16:51 Verification

### ✅ Verified - RF13 Generated JavaScript now gives silently wrong theme colors in a dark section, where before it refused the program
- **Severity:** Low
- **Confidence:** High
- **Evidence:** With RF9's emitter change, the corpus brand and the tutorial's step 4 (a mode-varying
  `provide brand` with an `env.colorMode=dark` section) now generate. But generated code evaluates
  the main-module bundle once at load, in light mode, so the dark section gets green 700 rather
  than green 500: `darkAccent` is the only design entrypoint left in `KNOWN_GENERATED_DIVERGENCES`
  (`ir_corpus_tests.rs:1282-1286`). The limit is old and recorded ("Generated values are evaluated
  at load", `specs/future.md:2379`, now also on the reference page's "Generated code"), but a
  program that used to fail to generate now runs and draws the wrong colors without a diagnostic.
  The README of the corpus (`:139-151`) still describes only refused programs and entrypoints as
  skipped, not a known-divergence list.
- **Recommendation:** Either make source codegen refuse (with a "not supported" diagnostic) a
  main-module `provide` whose value reads `env.` — so the case fails loudly as before — or accept
  it as documented and add a sentence about `KNOWN_GENERATED_DIVERGENCES` to the corpus README's
  generated-JavaScript paragraph.
- **Fix:** Took the second option. Generated code already evaluates main-module provides lazily (`nxEntryProvides` takes thunks); what makes the result wrong is the general "Generated values are evaluated at load" gap for the top-level `theme` value, and refusing every `provide` whose value reads `env.` would again refuse the default-theme programs RF9 opened up. The behaviour is stated on the reference page ("Generated code") and in `specs/future.md`, and the corpus README now describes `KNOWN_GENERATED_DIVERGENCES` and why `darkAccent` is in it (and says "a match on patterns other than union cases"). Nits from the RF9 verification also fixed: the skipped list names the program, and the doc comment is rewrapped.
- **Verification:** Accepting it is reasonable: refusing every `provide` that reads `env.` would again refuse the default-theme programs RF9 opened up, and the cause is the recorded load-time gap. The corpus README (`:139-151`) now describes `KNOWN_GENERATED_DIVERGENCES` and why `darkAccent` is in it, and says "a match on patterns other than union cases"; the reference page and `specs/future.md` state the behaviour. RF9's nits are fixed: the skipped list prints `design-tokens-theme: main.nx::darkAccent`, and the doc comment in `ir_corpus_tests.rs` is rewrapped. `generated_javascript_agrees_with_the_recorded_results` passes (140 compared) and `cargo fmt --check -p nx-api` is clean. Nit, not reopening: the edited README paragraph has two unwrapped lines, `:148` (122 characters, ending "…when it loads. A function value in generated JavaScript is the JavaScript function, which") and `:154` (108).

## Questions
- ~~Should applying an `env.Update` bundle to a subtree (RF1) be part of this change's promise?~~
  Answered: deferred, recorded in `specs/future.md`.
- ~~Is the token-read cost (RF3) acceptable?~~ Answered: recorded as a Risk and in `specs/future.md`,
  left to a change that caches top-level values and memoizes defaults.

## Summary
- The library, registration, typegen fix, tests and corpus programs are sound: all targeted tests
  and the corpus parity suite pass, the palette values match Tailwind v4, the contrast targets hold
  in both modes, and every new doc block passes the code-block check. The findings are mainly
  documentation accuracy (RF1, RF2, RF4–RF6), a notable evaluation cost worth recording (RF3), and
  small coverage gaps (RF7). Task 4.1 is still open, as expected before the review loop.
- Verification (2026-11-23): RF1, RF2 and RF4–RF7 verified, RF3 resolved by recording it as later work; RF8 opened (Low, wrapping and naming only), then verified in a second pass.
- Review pass (2026-11-23 16:30): four new findings. RF9 (Medium) is the one that matters: no
  program importing `@nx/design` can be generated as JavaScript or TypeScript, and task 2.2's
  "record what it lacks" was not done; rewriting the two mode defaults as `if … == … else` would
  lift it for the default theme. RF10–RF12 are Low: an undocumented way to set nested text colors
  from a theme, a wrong reason in the corpus README, and two small source inaccuracies.
- Fix pass (2026-11-23): RF9–RF12 fixed. Codegen, corpus parity, nx-codegen, nx-program, damage,
  standard-library evaluation and TypeScript runtime tests pass; clippy and fmt clean; the two
  edited doc pages pass the code-block check.
- Verification (2026-11-23 16:51): RF9–RF12 verified. RF13 opened (Low): the generated-JavaScript
  divergence for mode-varying themes is documented but silent, and the corpus README doesn't
  mention the known-divergence list.
- Fix pass (2026-11-23): RF13 fixed by documenting the divergence (README, reference page, future.md); RF9 nits fixed.
- Verification (2026-11-23, RF13 pass): RF13 verified (accepted and documented); RF9's nits fixed.
  Every finding RF1–RF13 is now closed.
