# Review: add-graphics-gradient-paints

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/graphics-library/spec.md, specs/graphics-preview/spec.md, specs/graphics-rendering/spec.md, tasks.md (18/18 checked)
**Reviewed code:** crates/nx-graphics/src/paint.rs, crates/nx-graphics/src/lib.rs (preparation, bounds, draw, tests), crates/nx-graphics-skia/src/lib.rs (paint/shader, destination, tests), crates/nx-graphics-skia/src/corpus_tests.rs, crates/nx-graphics-skia/fixtures/gradients/sampling.json, crates/nx-graphics-skia/fixtures/revisions/7/main.nx, crates/nx-api/src/std/graphics/graphics.nx, bindings/wasm/src/graphics.ts, sites/playground/src/graphics/{paint.ts, canvas.ts, hit-test.ts, prepare.ts, DrawingView.tsx, protocol.ts, graphics.test.mjs}, sites/playground/scripts/check-graphics-browser.mjs, scripts/{check-gradient-sampling.mjs, verify-graphics-fixtures.mjs, check-graphics-preview.py}, examples/graphics/{gradients.nx, runtime.nx}, sites/playground/src/examples/nx/gradients.nx, crates/nx-codegen/src/graphics_tests.rs, runtime/typescript/test/graphics.test.mjs, sites/website/.../graphics.md, evaluations/graphics-v1/tasks.json, Skia 0.153.3 gradient sources (SkLinearGradient.cpp, SkRadialGradient.cpp, SkGradientBaseShader.cpp/.h), /tmp/nx-gradient-review-diff.patch (file index)
**Verification pass (source inspected):**
- crates/nx-graphics/src/paint.rs (`MIN_GRADIENT_SPAN`, `validate_span`, tests)
- crates/nx-graphics/src/lib.rs:922-926 (text-normalized span check)
- Skia 0.153.3 `include/core/SkScalar.h:100-104` (`SkScalarNearlyZero`)
- crates/nx-graphics-skia/src/corpus_tests.rs:191-288 (shared scene test, tiny-span collapse probe)
- crates/nx-graphics-skia/fixtures/gradients/scenes.json (structure: names, fit, transforms, clipPath, dashArray, opacity)
- sampling.json (size only)
- sites/playground/scripts/check-graphics-browser.mjs:117-131
- sites/playground/src/graphics/paint.ts
- evaluations/graphics-v1/tasks.json
- scripts/verify-graphics-fixtures.mjs

**Not personally executed:** the coordinator reported these as passing; I did not run them and had no shell:
- neutral nx-graphics: 23 tests
- native nx-graphics-skia: 12 tests
- nx-codegen: 17 tests
- TypeScript IR suite
- WASM rebuild and playground type-check
- playground graphics: 18 tests
- full Chromium check
- native preview fixtures
- fixture verifier
- strict OpenSpec validation

## Findings

### ✅ Verified - RF1 Gradients that are too small for Skia pass validation, then render as a solid last-stop color, which breaks the sampling contract and makes native and browser output differ.
- **Severity:** Medium
- **Evidence:**
  - **Validation:** `crates/nx-graphics/src/paint.rs:140` only rejects linear endpoints that are equal at f32 precision. `paint.rs:155` only rejects a radius that is ≤ 0 at f32 precision.
  - **Skia behavior:** Skia 0.153.3 treats any linear vector length or radius below `kDegenerateThreshold = 1/32768` as degenerate:
    - `SkGradientBaseShader.h:72` defines the threshold.
    - `SkLinearGradient.cpp:97` and `SkRadialGradient.cpp:90` apply it.
    - With `TileMode::Clamp`, a degenerate gradient becomes `SkShaders::Color(colors.back())` (`SkGradientBaseShader.cpp:1120`).
  - **Native trigger:** a Drawing with `viewBox` width and height 0.001 at the default 640×480 gets a 480000× scale, which is within the 1e7 coefficient limit. Inside it, a Rect 0.001×0.001 with `radialGradient radius=0.00002`, or `linearGradient x1=0 x2=0.00002`, covers about 10 device pixels. The analytic contract requires a gradient there; native `paint()` (`nx-graphics-skia/src/lib.rs:299-312`) paints a solid last-stop color. Chromium's Canvas also uses Skia, so it collapses the same way.
  - **Native/browser divergence for text:** `canvas.ts:53` and `paint.ts:23-29` divide gradient coordinates and radius by `ratio = size/1000`. Native Skia does not (`lib.rs:438-439`). Text with `fontSize=2000` and a local radius or vector length of 0.00005 becomes 2.5e-5 < 3.05e-5 in the browser only. The browser paints a solid color while native paints a gradient.
  - **Spec:** this conflicts with "reject non-finite derived math and backend-unrepresentable values" and "adapter-specific defaults SHALL NOT excuse interpolation mismatches".
- **Recommendation:**
  - In neutral validation, reject any linear vector length or radius below a documented backend threshold (≥ 1/32768 local units) with a contextual diagnostic.
  - For Canvas text, either reject paints whose length or radius divided by the reference-font ratio falls below the threshold, or construct the gradient so that it does not shrink: for example, create it in pre-scale coordinates and draw through a transform that keeps the gradient's local magnitudes.
  - Add tests for the tiny-radius, tiny-vector and large-font text cases.

- **Fix:** Neutral preparation rejects spans below Skia’s 1/32768 threshold and checks text-normalized spans before font lookup. Shared contract/design/reference now document the minimum; direct boundary tests cover linear, radial, and large-text normalization.
- **Verification:** Reopened because of an off-by-boundary error. The rest of the fix is sound:
  - **Correct parts:**
    - `MIN_GRADIENT_SPAN = 1/32768` and `validate_span` (`paint.rs:32-46`) run after geometry validation (`paint.rs:184`).
    - The text check divides by `max(1, size/1000)` before font lookup (`lib.rs:923-926`), which matches the Canvas reference ratio.
  - **Off-by-boundary:**
    - The guard rejects only `span / scale < MIN_GRADIENT_SPAN` (`paint.rs:42`).
    - Skia's `SkScalarNearlyZero` is inclusive: `return SkScalarAbs(x) <= tolerance;` (`include/core/SkScalar.h:103`).
    - So a span of exactly 1/32768, which is exactly representable in f32, passes validation but still collapses to the last stop in Skia and Chromium.
    - The new unit test codifies the wrong side of the boundary: `paint.rs:193-216` includes `1.0 / 32768.0` and expects `validate_span(1.0)` to succeed.
    - The same applies to text whose normalized span lands exactly on the threshold.
  - **Concrete trigger:** `radialGradient radius=0.000030517578125`, or `linearGradient x1=0 x2=0.000030517578125`, inside a magnifying viewBox (as in the original trigger). Preparation succeeds, but the shader is a solid color.
  - **Probe coverage:** the Skia collapse probe (`corpus_tests.rs:275-282`) only exercises 0.00002 and does not pin the boundary.
  - **Fix needed:**
    - Reject `span / scale <= MIN_GRADIENT_SPAN`, or require a small margin above it, since Skia computes the linear length in f32 from f32 points.
    - Flip the boundary expectation in the test.
    - Add the exact-threshold case to the native collapse probe.
  - **Suggested coordinator probe:** render `{"kind":"radialGradient","cx":0,"cy":0,"radius":0.000030517578125,…}` through the existing probe and confirm it is a solid last-stop color.

- **Fix (second):** The guard now rejects `span / scale <= threshold × (1 + 4 × f32::EPSILON)`, which adds a rounding margin. Tests reject native exact and normalized-text exact thresholds and accept 0.001. The native Skia direct probe now includes exact 1/32768 for linear and radial. Docs and spec now explicitly require spans strictly above the backend minimum plus a rounding margin.
- **Verification (second):** Verified from source. The boundary correction is correct and the requested tests are present.
  - **Guard:** `paint.rs:42-45` now rejects `span / scale <= MIN_GRADIENT_SPAN * (1.0 + 4.0 * f32::EPSILON as f64)`.
    - That is strictly inclusive of Skia's `SkScalarNearlyZero` test (`|x| <= 1/32768`) and adds a 4-ulp relative margin.
    - The margin covers Skia's f32 `length()` of a vector built from f32 points.
  - **Span computation:** `paint.rs:37-40` mirrors the backend inputs.
    - Linear spans use the f32-cast endpoint differences with `hypot`.
    - Radial spans use the radius cast to f32.
  - **Call sites:** `paint.rs:185` applies the guard to every gradient at scale 1. `lib.rs:923-926` applies it again to Text fills at `max(1, size/1000)`.
  - **Unit test:** `paint.rs:193-219` tests spans {0.00002, 0.00005, 1/32768, 2/32768, 0.001} for linear and radial paints at scales 1 and 2. I traced each case:
    - Exact 1/32768 at scale 1 is rejected.
    - 2/32768 at scale 2, the normalized exact threshold, is rejected.
    - 2/32768 at scale 1 and 0.001 at both scales are accepted.
    - The expectation `span > k × MIN` diverges from the margin-inclusive guard only for spans within 4 f32 ulps above the threshold. No tested value falls in that range, so the test is consistent.
  - **Native Skia probe:** `corpus_tests.rs:272-309` renders exact 1/32768 linear and radial gradients, plus 0.00002 ones, under a 480000× canvas scale. It asserts the pixel is the solid last stop `[0,0,255,255]`. That pins the upstream inclusive collapse the guard now excludes.
  - **Not inspected:** I did not re-read the docs/spec wording or run any tests. The pass counts (neutral 23, native 12) are the coordinator's report.
  - **Related gap:** this pass found a separate Canvas-text quantization issue with the same symptom. It is recorded below as RF7 instead of reopening RF1.

### ✅ Verified - RF2 Required raster coverage is missing for gradient backgrounds, transformed/clipped radial gradients, dashed gradient strokes and viewBox placement, yet tasks 4.1, 4.2 and 6.1 are checked complete.
- **Severity:** Medium
- **Evidence:**
  - A repository-wide search finds no gradient `background` in any test, fixture or example; only the spec text mentions one. The spec scenario "Background retains drawing coordinates" (nonzero viewBox origin, contain letterboxing) is therefore unverified in every renderer.
  - **Native:** `nx-graphics-skia/src/corpus_tests.rs` has no gradient cases. Native gradient raster tests are limited to:
    - `gradient_sampling_matches_shared_analytic_corpus`, which draws `paint()` onto an untransformed rect and skips preparation, transforms, clips, opacity, strokes and backgrounds (`lib.rs:500-544`).
    - The text test (`lib.rs:545-625`).
  - **Browser:** `check-graphics-browser.mjs:63-117` covers:
    - the corpus on a plain rect
    - one hit test
    - one axis-aligned clip with opacity using a linear gradient
    - text sizes and anchors

    It does not cover dashed strokes, a radial gradient under nonuniform scale inside a clip (the "Gradient follows transformed geometry" scenario), nonzero viewBox origins, contain/cover/fill placement, or shared paints. All of these are listed in task 4.2.
  - The integrated page check at `check-graphics-browser.mjs:381-386` only compares the ordering of two pixels.
- **Recommendation:** Add shared native and browser raster fixtures, compared against the analytic oracle with the 2/255 interior tolerance and edge exclusions, for:
  - a gradient background with a nonzero viewBox origin under contain, cover and fill
  - a radial fill translated and nonuniformly scaled inside a clipped group
  - a dashed gradient stroke
  - a gradient under isolated opacity in native output
  - a paint shared by two elements

- **Fix:** Added seven shared authored/prepared raster scenes covering nonzero-origin background contain/cover/fill, clipped nonuniform radial transform, dashed stroke, isolated opacity, and shared paint. Native neutral preparation reproduces snapshots and both native/Chromium check the same analytic pixels within 2/255; suites pass.
- **Verification:** Verified from source.
  - **Fixture contents:** `fixtures/gradients/scenes.json` contains the gradient background scenes with `fit` variants (three cases), a `Paint.radialGradient` under a nonuniform `scale` with a `clipPath`, a `dashArray` stroke, an isolated-opacity scene and a shared-paint scene.
  - **Native:** `corpus_tests.rs:191-264` prepares each authored value through `nx_graphics::prepare`. It asserts the prepared snapshot equals the stored `prepared` value, renders it with Skia, and compares each sample's premultiplied RGBA with `expected` within 2/255.
  - **Browser:** `check-graphics-browser.mjs:117-131` renders the same scenes through `renderCanvas` and fails on any sample mismatch.
  - **Limits of this check:** I did not trace how each `expected` value was derived from the analytic oracle, and I did not check each sample point's distance from edges. The tests' passing results are the coordinator's report.

### ✅ Verified - RF3 The analytic corpus never exercises the Canvas zero-alpha substitution paths or unequal-alpha hard stops.
- **Severity:** Low
- **Evidence:**
  - `fixtures/gradients/sampling.json` has only four cases: a transparent end stop, unequal alpha, an opaque hard stop with missing endpoints, and an opaque radial.
  - Two `paint.ts` branches are never exercised:
    - Lines 37-39: a transparent stop followed by a visible stop.
    - Lines 56-61, combined with the next stop's substitution: a mid-gradient transparent stop, which produces two zero-alpha stops at the same offset.
  - No case covers a hard stop where alpha changes, or a transparent hard stop.
  - The spec scenario explicitly requires "colored fully transparent endpoints", and task 1.1 requires "premultiplied encoded-sRGB alpha" coverage.
- **Recommendation:** Add these corpus cases and sample them in both renderers:
  - transparent-green@0 → opaque-red@1
  - opaque-red@0 → transparent-blue@0.5 → opaque-green@1
  - an unequal-alpha hard stop, sampled on both sides
  - duplicate stops at offset 0, sampled at t<0 and t≥0

- **Fix:** Expanded the shared native/browser analytic corpus with transparent start, transparent midpoint, unequal-alpha hard stop, and duplicate-zero-offset cases. Oracle and native raster corpus pass.
- **Verification:** Verified from source. `sampling.json` now has eight cases. For each linear case I worked out the expected values by hand with the oracle's pixel-center convention, `t = (x + 0.5 − x1) / (x2 − x1)`:
  - **`transparent-start`** (`sampling.json:257-318`): green with α=0 at offset 0, opaque red at offset 1.
    - Samples at x=25, 50 and 75 expect `(0.25,0,0,0.25)`, `(0.5,0,0,0.5)` and `(0.75,0,0,0.75)`.
    - These match premultiplied interpolation, and the zero green channel proves the transparent stop's RGB does not leak in.
    - This exercises the Canvas transparent→visible substitution.
  - **`transparent-midpoint`** (`sampling.json:319-389`): opaque red at 0, transparent blue at 0.5, opaque green at 1.
    - x=25 expects `(0.5,0,0,0.5)`, x=50 expects `(0,0,0,0)` and x=75 expects `(0,0.5,0,0.5)`. All are correct.
    - This exercises both the visible→transparent end insertion and the next stop's substitution, which together put two zero-alpha stops at offset 0.5.
  - **`unequal-alpha-hard-stop`** (`sampling.json:390-459`): red with α=0.25 on [0,0.5], green with α=0.75 on [0.5,1].
    - Samples at x=45 (t=0.45) and x=55 (t=0.55) expect `(0.25,0,0,0.25)` and `(0,0.75,0,0.75)`.
    - That covers both sides of the hard stop, outside the two-pixel exclusion band.
  - **`duplicate-start`** (`sampling.json:460-530`): x1=10.5, red then blue at offset 0, green at offset 1.
    - x=0 (t=−0.1) expects red, which is the first coincident stop.
    - x=10 (t=0) expects blue, which is the last coincident stop.
    - x=60 (t=0.5) expects `(0,0.5,0.5,1)`.

  All values agree with the contract and with the oracle in `scripts/check-gradient-sampling.mjs`.
  - **Fragility note, not a reopen:** the duplicate-start x=10 sample sits exactly on the hard stop's pixel center. That is inside the design's two-pixel raster exclusion band, so it relies on both backends producing t=0 exactly.
  - The coordinator reports it passes natively and in Chromium.
  - If it ever flakes, move it to x=12 and leave the exact-offset check analytic only.

### ✅ Verified - RF4 The IR and example coverage for reusable gradients is narrower than task 2.2 and the spec scenarios, and one playground test's title overstates what it checks.
- **Severity:** Low
- **Evidence:**
  - Rust and TypeScript IR tests (`graphics_tests.rs:426-438`, `runtime/typescript/test/graphics.test.mjs:20`) evaluate only `gradientShape`: a Star with a linear gradient fill and stroke (`examples/graphics/runtime.nx:24-45`).
  - Task 2.2 and the spec require coverage of a Rect fill, a Line stroke, text and background, plus a radial gradient. The scenario "Reuse across fill and stroke" names a Rect fill, a Line stroke and a Star helper.
  - The playground test at `graphics.test.mjs:405` is titled "shared fill/stroke/text/background paints". But `gradients.nx` uses gradients only for fills: its background (line 32), Star stroke (line 56), Text fill (line 63) and chart stroke (line 72) are all solid.
- **Recommendation:**
  - Extend `runtime.nx` with exports for a Rect fill, a Line stroke, a Text fill, a Drawing background and a radial gradient. Assert equal values in both IR runtimes.
  - Make the gradient example (or a test-only variant) use a gradient text fill, background and stroke, or rename the playground test to match what it checks.

- **Fix:** Added Rect, Line, Text, Drawing background and radial exports/assertions to Rust and TypeScript IR suites; corrected misleading playground title. Added large-text gradient rejection and Canvas stop-count regressions. 17 codegen graphics tests, TS runtime suite, 18 playground graphics tests and full Chromium integration pass.
- **Verification:** Verified from source.
  - **New exports:** `examples/graphics/runtime.nx:48-62` adds:
    - `gradientRect` (fill)
    - `gradientLine` (stroke)
    - `gradientText` (Text fill)
    - `radialPaint`, a `Paint.radialGradient` function
    - `gradientBackground`, a Drawing `background` using `radialPaint()`

    The existing Star helper `gradientShape` reuses the same named `gradientPaint()` for fill and stroke.
  - **Rust IR:** `graphics_gradients_cover_every_paint_location_on_ir` (`crates/nx-codegen/src/graphics_tests.rs:440-461`) asserts:
    - Rect fill, Line stroke paint and Text fill each equal the evaluated `gradientPaint`
    - the background equals `radialPaint`
    - `radialPaint` has `$type` `Paint.radialGradient`
  - **TypeScript IR:** `runtime/typescript/test/graphics.test.mjs:20-26` makes the same deep-equality assertions, plus the existing Star fill/stroke check. Together these cover the "Reuse across fill and stroke" scenario and the text/background scenario in both IR runtimes.
  - **Playground title:** the test is now titled "gradient example prepares shared illustration and chart fills through TypeScript IR" (`sites/playground/src/graphics/graphics.test.mjs:405`). Its assertions match the title: a linear fill, a radial fill, and a fill shared between the Rect and the Star.
  - **Additional regressions present:** an equal-RGB Canvas fade stop-count test (`:415-426`) and a large-text gradient rejection test (`:428-434`).
  - The playground test does not assert the chart's `chartFade` fill specifically. That is minor, since the title no longer claims stroke, text or background coverage.

### ✅ Verified - RF5 `evaluations/graphics-v1/tasks.json` still declares `graphics-core-v2`.
- **Severity:** Low
- **Evidence:**
  - `evaluations/graphics-v1/tasks.json:3` reads `"profile": "graphics-core-v2"`. The file is not part of the diff.
  - The design calls for updating "every producer/consumer, manifest/schema, expected fixture and reference together". `scripts/verify-graphics-fixtures.mjs:6-11` reads this file but does not check the profile.
- **Recommendation:** Update it to `graphics-core-v3` and assert `tasks.profile === "graphics-core-v3"` in `verify-graphics-fixtures.mjs`.

- **Fix:** Updated evaluation task metadata to graphics-core-v3 and added a profile assertion to the deterministic fixture verifier; verifier passes.
- **Verification:** Verified from source. `evaluations/graphics-v1/tasks.json:3` now reads `"profile": "graphics-core-v3"`, and `scripts/verify-graphics-fixtures.mjs:8` asserts `tasks.profile === "graphics-core-v3"`.

### ✅ Verified - RF6 Canvas splits every unequal-alpha interval into 64 sub-stops, even where the RGB values match and plain interpolation is already exact, and the cost repeats on every selection change.
- **Severity:** Low
- **Evidence:**
  - `sites/playground/src/graphics/paint.ts:41-55` subdivides whenever alphas differ.
  - When both ends have the same RGB, which includes intervals rewritten by the zero-alpha substitution, unpremultiplied interpolation already equals premultiplied interpolation. Common fade-outs such as `#38bdf8c0 → #38bdf800` in `gradients.nx:24-25` therefore pay 63 extra CSS-parsed stops for nothing.
  - `renderCanvas` reruns on every selection change (`DrawingView.tsx:184`). The browser uses the full 100000-stop budget (`prepare.ts:193`), so a worst-case scene costs about 6.4M `addColorStop` calls per paint per render on the main thread.
- **Recommendation:**
  - Skip subdivision when the effective RGB at both ends is equal (after substitution).
  - Consider a lower browser `gradient_stops` limit, or adaptive subdivision based on RGB and alpha deltas, so the 1/256 error bound holds with fewer stops.

- **Fix:** Canvas skips subdivision when effective RGB endpoints match after zero-alpha substitution, retaining exact alpha interpolation and bounded stop count for common fades.
- **Verification:** Verified from source.
  - **Mechanism:** `paint.ts:44-51` computes the effective end color, using the start RGB with zero alpha when the next stop is transparent. When the effective RGB values match, it emits one end stop and skips the 64-way split.
  - **Trace:**
    - transparent→visible: the start is substituted with the next stop's RGB, so the ends match and the interval is exact.
    - visible→transparent: the end is substituted with the start's RGB, so the ends match and the interval is exact.
    - Subdivision now happens only when both alphas are positive and the RGB values differ, which is the only case where unpremultiplied interpolation errs.
  - **Duplicate stops:** the extra stops this emits at the same offset are either identical or zero-alpha, so they cannot be seen.
  - The trailing zero-alpha insertion at `paint.ts:64-69` can no longer be reached, which is harmless.
  - The browser budget itself is unchanged, which is acceptable under the design.

## Questions
- `Prepare::node` validates and counts a raw `fill` on a Drawing node against `gradient_stops` (`lib.rs:761`), then replaces it with `background` (`lib.rs:818`). Should a Drawing `fill` property be rejected as unsupported rather than silently discarded? The same question applies to Group, which has no geometry.
- A fully transparent `Paint.color` is dropped to `None` during preparation (`paint.rs:88`), so the inspector shows `fill: null`, while a fully transparent gradient is kept and shown. Is that asymmetry intended for the requirement "element dropdown SHALL still permit inspecting its … data"?
- These were verified as not issues: Skia's duplicate-stop-at-offset-0 handling under Clamp matches the oracle; the Canvas hard-stop and transparent-stop insertion logic matches the contract in my trace; the renderer identifier was bumped to `browser-canvas2d-v2`.

## New Findings Discovered During 2026-10-30 Verification

### ✅ Verified - RF7 Canvas text gradients can still collapse in the browser only, because the shifted text-local endpoints are rounded to float32 at a larger magnitude than neutral validation models.
- **Severity:** Low
- **Evidence:**
  - **How Canvas builds the gradient:** for text, `paint.ts:23-26` passes `(x - text.x) / ratio` endpoints to `createLinearGradient`. Blink stores CanvasGradient points as float (`gfx::PointF`) before building the Skia shader, so each endpoint is rounded to f32 at the magnitude `|x - text.x| / ratio`.
  - **What validation models:** neutral validation (`paint.rs:38`, via `lib.rs:925`) measures the span from f32 values of the local endpoints, at magnitude `|x|`, divided by the ratio. It does not account for the origin shift.
  - **Native path:** native Skia draws text with the shader in unshifted local coordinates (`nx-graphics-skia/src/lib.rs:438-439`), so native matches validation.
  - **Concrete trigger:** a Text with `x=-1500`, `fontSize=100` (ratio 0.1) and a string long enough for its glyphs to straddle local x=0, filled with `linearGradient x1=0 y1=0 x2=0.000032 y2=0`, red→blue.
    - **Validation:** 3.2e-5 > 3.0518e-5 × (1 + 4ε), so it accepts.
    - **Native:** renders red glyphs left of x=0 and blue to the right. The tiny vector acts as a hard edge between clamped half-planes, so no magnification is needed for it to be visible.
    - **Chrome:** the endpoints become 15000 and 15000.00032. The f32 spacing at 15000 is 2⁻¹⁰ ≈ 9.77e-4, so both round to 15000.
    - That zero-length vector is degenerate, and Chrome paints the whole run solid blue.
  - **Also affected:** any `|x − text.x|` larger than roughly 512–1024 units can hit this, for any font size, whenever the span is near the minimum.
  - **Not affected:** radial radius is unaffected, because `radius / ratio` has no origin term.
- **Recommendation:**
  - Move the text-paint check after shaping, where `text.x` is known.
  - Have it emulate the Canvas inputs exactly: f32-round `(x1 − tx) / ratio`, `(x2 − tx) / ratio` and the corresponding y values, then require their f32 length to exceed the same threshold-with-margin.
  - Add a regression for the trigger above. The coordinator can run it in Chromium: compare a solid-red reference against the gradient render at pixels on both sides of local x=0.

- **Fix:** Check the exact shifted/scaled float32 Canvas gradient inputs after shaping and anchoring, while retaining the native/pre-shaping checks. Reject non-finite normalized coordinates. Added long-text x=-1500/fontSize=100 regression with span0.000032.
- **Verification:** Verified from source.
  - **Emulation of Canvas inputs:** `Paint::validate_text_span` (`crates/nx-graphics/src/paint.rs:37-67`) reproduces the browser's gradient inputs.
    - **Formula:** each coordinate is computed as `((value - origin) / scale) as f32`. That is the same f64 expression as Canvas's `(value - originX) / scale` (`sites/playground/src/graphics/paint.ts:23-24`), followed by the same round-to-nearest float conversion Blink applies.
    - **Linear spans:** the f32 endpoint differences are combined with `hypot` and compared against the same inclusive threshold-with-margin RF1 now uses. `validate_span` delegates with origin (0,0) (`paint.rs:34-36`), so both paths share one guard.
    - **Radial spans:** the radius is normalized without an origin, matching `paint.radius / scale` at `paint.ts:29`. Both centers are normalized too.
    - **Finiteness:** any normalized coordinate that overflows f32 is rejected with a contextual diagnostic (`paint.rs:45-55`).
  - **Call order:** in `lib.rs`, the existing pre-shaping check at `max(1, size/1000)` (`lib.rs:923-926`) and the native check (`paint.rs:212`) are retained. The new check runs after shaping and anchoring (`lib.rs:940-942`), using the anchor-adjusted `x`, the `y`, and the exact `size / 1000.0`.
  - **Match with Canvas:** these are the same values that become `Text.x`, `Text.y` and the ratio that `canvas.ts:42,53` passes to `canvasPaint` (`size / TEXT_REFERENCE_SIZE`, where `TEXT_REFERENCE_SIZE = 1000`). Browser preparation runs this same neutral code with browser measurements, so the validated origin equals the origin Canvas uses.
  - **Unit regression:** `translated_text_endpoints_are_checked_at_backend_precision` (`paint.rs:219-233`). I traced each assertion:
    - The unshifted span 3.2e-5 passes `validate_span(1.0)`.
    - At origin −1500 and scale 0.1, the endpoints become 15000 and 15000.00032, both of which round to f32 15000. The span is 0, so it is rejected.
    - At origin 0 and scale 0.1, the span is 3.2e-4, so it is accepted.
  - **Playground regression:** `sites/playground/src/graphics/graphics.test.mjs:436-442` prepares the exact trigger: x=−1500, fontSize=100, `"Gradient"` repeated 40 times so the glyphs straddle x=0, and the 3.2e-5 span. It asserts no scene is published and a "gradient span" diagnostic is returned.
  - **Further defects:** none found in the touched code. The f32 subtraction-then-`hypot` remains inside RF1's 4-ulp margin relative to Skia's f32 `length()`.
  - **Test status:** I did not run any tests. The coordinator reported neutral 24 passing; at the time of this verification they had not provided final results for the WASM and browser rebuild and test runs.

## Summary
- **Final status:**
  - RF1–RF7 are all ✅ Verified by source inspection across the verification passes.
  - RF7 was verified in this pass: post-shaping validation now exactly emulates the f32 Canvas text gradient inputs.
  - No findings were reopened, and no new findings were added.
- **Counts:** 7 Verified, 0 Open, 0 Reopened, 0 new.
- **Test evidence:** all test evidence comes from the coordinator; I executed no tests or probes. Coordinator-reported results across passes:
  - neutral nx-graphics 24 (latest)
  - native nx-graphics-skia 12
  - nx-codegen 17
  - TypeScript IR suite
  - playground graphics 18
  - full Chromium integration
  - native preview fixtures
  - fixture verifier
  - strict OpenSpec validation
- **Pending:** the WASM and browser rebuild and test run after the RF7 fix was still in progress, with no final result supplied. Confirm it passes before archiving.
- **Questions:** the two original questions (Drawing/Group `fill` handling and transparent `Paint.color` inspection) remain informational and are not blocking.

## Coordinator Validation After RF7 Verification

The coordinator subsequently reported that the final WASM rebuild and all 19 playground graphics tests pass, including the long-text-origin regression. Neutral 24 and native 12 tests, regular native preview checks, and strict OpenSpec validation also pass. The final Chromium integration run also passed, including the shared gradient corpus, prepared scenes, compositing, selection, failure recovery and device-scale checks. These are coordinator-run results, not tests executed by Claude.
