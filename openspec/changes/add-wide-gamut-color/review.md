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

### 🟡 Fixed - RF1 The Rust parser accepts bare `display-p3(…)`, `srgb(…)` and `srgb-linear(…)` as functions; TypeScript rejects them
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

### 🟡 Fixed - RF3 An invalid SDK color fails with `nx-ir-boundary-constraint`, but the spec scenario requires `color-invalid-channel`
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

### 🟡 Fixed - RF4 An inspector swatch for a reference to a wide color has no outside-sRGB mark
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

### 🟡 Fixed - RF5 Rust keeps a negative-zero Oklch hue; TypeScript normalizes it to 0
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

### 🟡 Fixed - RF6 Tests miss several of the new rules
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

### 🟡 Fixed - RF7 Small documentation and comment leftovers
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

## Questions
- RF2: is "no picker at all" for wide colors the intended UX, or should the popover open read-only as
  the spec says? The fix depends on which one is intended.
- RF3: should `validate` keep `color-invalid-channel` as its public contract for SDK callers, or should
  the spec accept the boundary-constraint code that typed entry points now produce?

## Summary
- The value model, both runtimes, gamut mapping, hex lowering, alias-union resolution, graphics
  preparation, swatch discovery and transport follow the design and Decided Questions: OKLCH for
  `lighten` and its kin on every input, Oklab kept as a stored space, P3 drawing deferred, and the
  profile staying `graphics-core-v4`.
- Both runtimes pass the shared corpus, and the TypeScript source still produces valid emitted
  JavaScript.
- The main issues:
  - a Rust/TypeScript parser divergence that the new space extends (RF1);
  - the picker not opening read-only for wide colors as the spec says (RF2);
  - a spec mismatch on the error code for invalid SDK colors (RF3);
  - smaller gaps: a missing mark on referenced swatches, signed-zero hue, test coverage, and docs
    (RF4–RF7).
