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
