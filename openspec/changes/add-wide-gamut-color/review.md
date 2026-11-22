# Review: add-wide-gamut-color

## Scope
**Reviewed artifacts:** proposal.md, design.md (including Decided Questions), tasks.md,
specs/color-library/spec.md, specs/color-literals/spec.md, specs/color-tooling/spec.md,
specs/graphics-library/spec.md  
**Reviewed code:** `git diff origin/main...HEAD` (commit d58cc146), excluding generated `.nxir`/`.nxir.txt`
files and `runtime/typescript/dist/`: `crates/nx-value/src/color.rs`, `color-cases.json`,
`color_corpus_tests.rs`, `runtime/typescript/src/color.ts` (also checked that the stripped emitted-JS
form parses), `runtime/typescript/src/index.ts`, `crates/nx-program/src/std/color/{color,operations}.nx`,
`crates/nx-hir/src/lower.rs`, `crates/nx-types/src/{infer,ty}.rs`, `crates/nx-codegen/src/{builder,ir}.rs`,
`crates/nx-ir/src/image.rs`, `crates/nx-ir-runtime/src/module.rs`, `crates/nx-graphics/src/{paint,lib}.rs`,
`crates/nx-graphics-skia/src/lib.rs`, `crates/nx-language-service/src/{source_tree,source_edit}.rs` and
tests, `crates/nx-api/src/nx_text.rs` and codegen tests, `bindings/wasm/src/{host,types}.ts`,
`packages/viewer/src/{source,edit,styles}.ts`, `packages/value-view/src/{render,types}.ts`,
`packages/language-protocol/src/index.ts`, docs `color.md`, `graphics.md`, `specs/future.md`.  
**Checks run:** `cargo test -p nx-value color` (8 pass), `cargo test -p nx-language-service color`
(12 pass), `npx tsc -p . && node test/color.test.mjs` in `runtime/typescript` (282 corpus cases pass),
`node --check` on `color.ts` after the `runtime.rs` annotation stripping (parses), and a scratch probe
against both runtimes for the findings below.

## Findings

### ✅ Verified - RF1 The Rust parser accepts bare `display-p3(…)`, `srgb(…)` and `srgb-linear(…)` as functions; TypeScript rejects them
- **Severity:** Medium
- **Confidence:** High (reproduced on both runtimes)
- **Evidence:** `crates/nx-value/src/color.rs:607-627`: the allow-list of function names, which is also
  checked when the text has no `color(` wrapper, now has `"display-p3"` beside the existing `"srgb"`
  and `"srgb-linear"`. A probe gives `parse("display-p3(1 0 0)")` → `Ok(Color.displayP3 1 0 0 1)`
  and `parse("srgb(1 0 0)")` → `Ok(Color.srgb …)` in Rust. In TypeScript
  (`runtime/typescript/src/color.ts`, regex `^(rgb|rgba|hsl|hsla|hwb|oklab|oklch|color)\(`) the same
  calls fail with `color-invalid-text`. This breaks the spec's requirement that parser acceptance not
  depend on the host, and it is not valid CSS: a browser will not take the `toCss`-style text a user
  copies. Swatch discovery in `nx-language-service` uses the Rust parser, so the editor shows a
  swatch for `c.parse("display-p3(1 0 0)")`, which then fails at runtime on the TypeScript IR.
  The `srgb`/`srgb-linear` part was already there; this change extends it to the new space.
- **Recommendation:** Accept `srgb`, `srgb-linear` and `display-p3` only as the color-space token
  inside `color(…)`. For example, after the `if name == "color"` block, reject those names when the
  outer function was not `color`. Add corpus error cases for `display-p3(1 0 0)` and `srgb(1 0 0)` so
  both runtimes are held to it.
- **Fix:** The Rust parser now rejects `srgb`, `srgb-linear` and `display-p3` outside `color()`, matching TypeScript (`color.rs` parse). Corpus cases for `display-p3(1 0 0)` and `srgb(1 0 0)` expect `color-invalid-text` on both runtimes.
- **Verification:** Verified. `parse` now refuses `srgb`, `srgb-linear` and `display-p3` before handling `color(` (`color.rs:601-604`), so `color(display-p3 …)` still parses. Both runtimes pass the new corpus error cases: `cargo test -p nx-value color` (9 tests) and the TypeScript corpus (294 cases).

### 🟡 Fixed - RF2 Wide colors don't open a read-only picker; their edit target is dropped instead
- **Severity:** Medium
- **Confidence:** Medium
- **Evidence:** `packages/viewer/src/edit.ts:312-314` returns `undefined` for a `call`/`element`
  color whose swatch text doesn't start with `#`, so `editTargets` has no color target for
  `displayP3(…)`, `oklch(…)`, `parse("color(display-p3 …)")` or `<Color.oklch …/>`. The
  color-tooling spec says "A recognized color in another space SHALL open read-only, showing its CSS
  text", and design.md says "Wide colors open read-only". As built, nothing opens. The "SHALL NOT
  offer an edit" half is met, but there is no read-only view of the color, and no viewer test covers
  the picker for a wide color. The only new viewer test checks the swatch mark.
- **Recommendation:** Return a color target marked read-only, for example
  `{ kind: "color", value: node.color, readOnly: true }`, and have the popover show the CSS text with
  its inputs disabled. Add a viewer test that opens the picker on `displayP3(1, 0, 0)` and asserts it
  is read-only and sends no edit. If dropping the target is the intended UX, change the spec and
  design text to "is not editable" instead.
- **Fix:** Kept the no-picker behavior (a wide color is shown with its swatch and CSS text, never offered to the sRGB picker) and reworded the color-tooling requirement, its scenario and design.md to say so. New viewer test in `editing.test.ts` checks that only the sRGB color is a picker target while the P3 color keeps its swatch.
- **Verification:** Partly fixed. The behavior is consistent and tested: `editing.test.ts` shows only the sRGB color is a picker target, and the P3 color keeps its swatch. The reworded scenario is coherent. However, the requirement sentence now says a wide color "SHALL be shown read-only, with its swatch **and its CSS text**", and nothing shows the CSS text. Source view shows the authored text (`displayP3(1, 0, 0)`, `<c.Color.oklch …/>`), and the swatch `title` is only set to the outside-sRGB note, and only for colors outside sRGB (`packages/viewer/src/source.ts:329`). An in-gamut `oklch(0.7, 0.1, 200)` shows no CSS anywhere. design.md's line ("shown read-only, with no picker") also now runs past the 100-character wrap. To close: drop "and its CSS text" from the requirement, or set the swatch's `title` (and the outside-sRGB label) to the color's CSS text and add an assertion for it.
- **Fix (2):** Dropped "and its CSS text" from the requirement, which now says a wide color is shown read-only with its swatch and is not offered to the sRGB picker. Rewrapped that paragraph and the design.md line to 100 columns.

### ✅ Verified - RF3 An invalid SDK color fails with `nx-ir-boundary-constraint`, but the spec scenario requires `color-invalid-channel`
- **Severity:** Medium
- **Confidence:** Medium-High
- **Evidence:** `crates/nx-api/tests/codegen/color_transport_tests.rs:389` was changed to expect
  `nx-ir-boundary-constraint` when a host passes `{"$type":"Color.srgb","r":2,…}` to a function that
  takes a `Color`, because the new `Unit` constraints are checked at the boundary before `validate`
  runs. The color-library spec's "Invalid SDK record" scenario (`specs/color-library/spec.md:26-28`)
  says `validate` or a formatter receiving such a record "SHALL report color-invalid-channel for `c`".
  An SDK caller of the documented `validate` entry point gets a different error code from the one
  the spec and docs promise. The `color-invalid-channel` path still applies to raw records reaching
  the externals and graphics preparation, which bypass the boundary.
- **Recommendation:** Align the spec with the layering: a typed boundary rejects out-of-range
  channels with `nx-ir-boundary-constraint`, and `color-invalid-channel` covers records that reach
  the library without crossing a typed boundary, such as graphics preparation and raw externals.
  Alternatively, map the constraint failure to `color-invalid-channel` for Color fields. Either way,
  add a test for the scenario's actual case: `Color.oklch` with `c = -1` and with a non-finite `c`.
- **Fix:** The typed boundary checking constraints first is the intended design (constraints on the fields). Rewrote the "Invalid SDK record" scenario: a typed boundary fails with `nx-ir-boundary-constraint`; a value read without one (graphics preparation) fails with `color-invalid-channel`. Also reworded the `validate` sentence in the requirement.
- **Verification:** Verified as a spec change. The scenario is now coherent with the layering: a typed boundary rejects constrained channels with `nx-ir-boundary-constraint`, and paths without a typed boundary (graphics preparation, raw externals) use `color-invalid-channel`, as `paint.rs` and the graphics tests do. The `validate` sentence still makes sense, because unconstrained fields (Oklab `a`/`b` NaN or infinity via MessagePack) and raw externals can still reach it. Test coverage for the reworded scenario is tracked separately as RF8.

### ✅ Verified - RF4 An inspector swatch for a reference to a wide color has no outside-sRGB mark
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `packages/viewer/src/source.ts:858-868` paints a swatch for a reference from
  `referencedColor(value)`, which returns the referenced declaration's `color`. It never adds the
  `outside-srgb` class, title or `aria-label`. The mark is only applied in `swatch()`
  (`source.ts:341`), and only from the node's own `colorOutsideSrgb`. So `let accent = c.oklch(0.7,
  0.3, 145)` used as `fill={accent}` shows an unmarked swatch, which goes against the color-tooling
  requirement that a swatch for a color outside sRGB carries the mark.
- **Recommendation:** Have `referencedColor` return the color together with the held node's
  `colorOutsideSrgb` flag, and share one helper with `swatch()` that applies the class, title and
  accessible label. Extend the `wideColors` viewer fixture with a reference.
- **Fix:** `referencedColor` now returns the node that holds the color, and the inspector's reference swatch goes through the shared `markOutsideSrgb` helper. New inspector test for a reference to an Oklch color.
- **Verification:** Verified. `referencedColor` returns `{ color, at }` from the node that holds the color, and both swatch paths share `markOutsideSrgb`. The new inspector test (`marks a reference's swatch when the color it names is outside sRGB`) passes, as do the earlier swatch tests (viewer built with `tsc`, filtered `node --test` run).

### ✅ Verified - RF5 Rust keeps a negative-zero Oklch hue; TypeScript normalizes it to 0
- **Severity:** Low
- **Confidence:** High (reproduced)
- **Evidence:** `crates/nx-value/src/color.rs:191-198` (`hue`) returns `h.rem_euclid(360.0)`
  unnarrowed, and `make_in` (`color.rs:221-227`) uses `hue(x)` instead of `narrow(x)` for `h`. So
  `oklch(0.5, 0.1, -0.0)` and `oklch(0.5, 0.1, -360)` store `h = Float32(-0.0)` in Rust. TypeScript's
  `colorMakeIn` passes the wrapped hue through `colorNarrow` (`Math.fround(x) || 0`) and stores `0`.
  The spec says constructors "SHALL normalize signed zero". The difference can show in IR or JSON
  output and in hashing or equality on the bits.
- **Recommendation:** In `make_in`, write `narrow(hue(x))` for the hue, or have `hue` return
  `narrow`-normalized zero. Add a corpus case with `oklch(0.5, 0.1, -360)` and an exact check.
- **Fix:** `hue()` adds 0.0 after `rem_euclid`, so -360 stores +0.0. Unit test `a_wrapped_hue_is_never_negative_zero` checks the bits (fails without the fix); a corpus case covers both runtimes.
- **Verification:** Verified. `hue()` and `wrapped_hue()` add `0.0` after `rem_euclid`. `a_wrapped_hue_is_never_negative_zero` checks the bits, and the corpus case `oklch(0.5, 0.1, -360)` passes on both runtimes.

### ✅ Verified - RF6 Tests miss several of the new rules
- **Severity:** Low
- **Confidence:** High
- **Evidence:** In `crates/nx-value/src/color-cases.json` and the unit tests:
  - No `toGamut` case for Oklch lightness ≤ 0 mapping to black. Task 2.4 names "lightness at 0 and 1",
    and only L=1 is present.
  - No `mix` or `composite` case where the inputs are `oklab` and `oklch`, so the first input's space
    winning is untested.
  - No `composite` case with a wide input.
  - No transformation on an `oklab` or `oklch` input (`lighten`, `saturate`, `grayscale`) asserting
    that the result stays in that space unmapped.
  - No case for `mix` at weight 0 or 1 with inputs from different spaces (the endpoint keeps its own
    space).
  - On the tooling side, no test covers the picker for a wide color (see RF2).
- **Recommendation:** Add these corpus cases, which both runtimes share, so Rust and TypeScript are
  held to agreement on them, plus the viewer test from RF2.
- **Fix:** Added 12 corpus cases run by both runtimes: lightness 0 and 1 mapping to black and white, Oklab/Oklch first-input-wins both ways, `mix` endpoints across spaces at weights 0 and 1, `composite` with a P3 input, `lighten` on Oklab and `grayscale` on Oklch keeping their space, the hue signed zero, and the bare-function parse errors. Picker coverage is the RF2 test.
- **Verification:** Verified. The corpus now has L=0 → black, L=1 → white for P3, the Oklab/Oklch first-input rule in both orders, `mix` endpoints across spaces, `composite` with a P3 input, `lighten` on Oklab and `grayscale` on Oklch. Both runtimes pass all 294 cases. One small note: the P3 `composite` case uses an opaque foreground, so it checks the result space but not the blend itself. A translucent one would be stronger, but this is not blocking.

### ✅ Verified - RF7 Small documentation and comment leftovers
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - `crates/nx-program/src/std/color/color.nx:54` has `// The channel records are unchanged.`, a note
    about the change carried over from the draft `api/color.nx`. It describes nothing in the shipped
    library.
  - Task 5.1 asks for "the table of what maps into sRGB" in `reference/libraries/color.md`. The page
    covers it in prose only, and its "match on the space" advice has no `match` example; the only
    example converts with `toRgb`.
  - `specs/future.md` "Wide-Gamut Color" ends with a bullet set apart by a blank line, which renders
    as a second list.
- **Recommendation:** Remove the leftover comment. Add a short function → result-space table to
  `color.md`, matching the design's table, and a small `match` example on `Color.srgb` and
  `Color.oklch`, then run the code-block check. Join the last bullet to the list.
- **Fix:** Removed the draft comment from `color.nx`, added the table of what each function does with a wide color and an `if value is { … }` example to `color.md` (code-block check passes), and joined the split list in `specs/future.md`.
- **Verification:** Verified. The draft comment is gone, `color.md` has the function → result table and an `if value is { … }` example, and `specs/future.md` is one list. `node scripts/check-code-blocks.mjs` reports 265 nx blocks, 0 failures.

## New Findings Discovered During 2026-11-22 20:15 Verification

### 🟡 Fixed - RF8 The reworded "Invalid SDK record" scenario is not tested for its own case
- **Severity:** Low
- **Confidence:** High
- **Evidence:** The scenario in `specs/color-library/spec.md` now names a `Color.oklch` with `c = -1`
  at a typed boundary failing with `nx-ir-boundary-constraint`. The only boundary test,
  `color_transport_default_alpha_and_invalid_channels`
  (`crates/nx-api/tests/codegen/color_transport_tests.rs:389`), uses `Color.srgb` with `r = 2`.
  `c`'s constraint is written inline (`float32 where >= 0.0`) instead of through the `Unit` alias,
  so it takes a different path and should get its own check.
- **Recommendation:** Add the scenario's own case to that test: `{"$type":"Color.oklch","l":0.5,"c":-1,"h":0}`
  passed to `validateBrand` should give `nx-ir-boundary-constraint`. If practical, also add an Oklab
  record with a non-finite `a` through MessagePack, which should pass the boundary and fail in
  `validate` with `color-invalid-channel`.
- **Note:** The reworded lines in `specs/color-library/spec.md` (the `validate` sentence) and
  `specs/color-tooling/spec.md` (the read-only sentence) now run past the file's 100-character
  wrap. Rewrap them when the spec is next touched.
- **Fix:** `color_transport_default_alpha_and_invalid_channels` now also passes a `Color.oklch` with `c = -1` through the typed `validateBrand` boundary and expects `nx-ir-boundary-constraint`. Rewrapped the reworded requirement paragraph in the color-library spec to 100 columns.

## Questions
- RF2 and RF3 were answered by the coordinator: no picker for wide colors, and typed boundaries fail
  with `nx-ir-boundary-constraint`. The specs were reworded to match.
- No other open questions.

## Summary
- First pass: the implementation follows the design and its Decided Questions. Both runtimes pass
  the shared corpus, and the TypeScript source still produces valid emitted JavaScript.
- Verification (2026-11-22): RF1, RF3, RF4, RF5, RF6 and RF7 are verified.
- RF2 is reopened only for one phrase in the requirement, "and its CSS text", which nothing
  implements.
- RF8 is new and Low: the reworded SDK-boundary scenario has no test for its Oklch case.
