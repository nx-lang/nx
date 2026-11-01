# Review: add-geometry-standard-library

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, verification.md, specs/geometry-library/spec.md,
specs/graphics-library/spec.md, specs/graphics-preview/spec.md, specs/graphics-rendering/spec.md,
specs/value-constraints/spec.md
**Reviewed code:** `crates/nx-syntax/grammar.js`, `crates/nx-syntax/src/syntax_kind.rs` (generated
`parser.c`/`grammar.json`/`node-types.json` not read line by line), `crates/nx-hir/src/ast/types.rs`,
`crates/nx-hir/src/lower.rs`, `crates/nx-types/src/infer/constraints.rs`,
`crates/nx-types/tests/value_constraints.rs`, `crates/nx-syntax/tests/fixtures/valid/value-constraints.nx`,
`crates/nx-language-service/src/lib.rs`, `crates/nx-language-service/src/source_tree_tests.rs`,
`crates/nx-api/src/std/geometry/geometry.nx`, `crates/nx-api/src/std/graphics/graphics.nx`,
`crates/nx-api/src/artifacts.rs`, `crates/nx-api/src/standard_library_tests.rs`,
`crates/nx-api/src/schema_tests.rs`, `crates/nx-codegen/src/graphics_tests.rs`, `crates/nx-graphics/src/lib.rs`,
`crates/nx-graphics-skia/src/{lib.rs,corpus_tests.rs}` and fixtures, `specs/ir-conformance/{constraints,geometry-library}/**`,
`specs/ir-conformance/manifest.json`, `runtime/typescript/test/{geometry,graphics}.test.mjs`,
`runtime/typescript/package.json`, `bindings/wasm/test/graphics.test.ts`, `sites/playground/src/graphics/canvas.ts`,
playground tests/scripts/examples, `scripts/check-graphics-preview.py`, `scripts/verify-graphics-fixtures.mjs`,
`schemas/graphics-preview.schema.json`, `evaluations/graphics-v1/tasks.json`, `examples/graphics/*.nx`, website
reference docs (geometry, graphics, color, types, if, modules, style guide).
**Checks run:** `cargo test -p nx-api -p nx-codegen -p nx-types -p nx-syntax -p nx-language-service -p nx-graphics`
(all passed); `pnpm --dir runtime/typescript test` (passed); `cargo fmt --check` (clean); ad hoc probes with
`nxlang run`, `nxlang typegen`, and `nxlang codegen --target nx-ir` plus the TypeScript runtime (float32/float64
finite boundary and widening checks behave correctly: infinity -> `nx-ir-constraint`, out-of-range float32 ->
`nx-ir-boundary-*`, `float32::MAX` widened to `float64 where finite` accepted).

## Findings

### ✅ Verified - RF1 A union case named `finite` can no longer be matched by its bare name in an `is` arm.
- **Severity:** Medium
- **Evidence:** `finite_pattern` (`crates/nx-syntax/grammar.js:1147`) is a `prec(1)` alternative of
  `numeric_pattern`, which the `pattern` rule (`grammar.js:1122-1128`) offers alongside `qualified_name`. A bare
  `finite` in any arm now parses as the numeric term, not a name pattern. Repro:
  `type Extent = finite | infinite` with `if e is { finite => "finite" infinite => "infinite" }` reports "A numeric
  pattern needs a numeric scrutinee; `Extent` is not one, so `finite` cannot match it" and "Union match on
  'Extent' is missing cases: finite". The `infinite` arm still resolves, so the failure is specific to the new term.
  The value-constraints delta requires `finite` to be "a contextual pattern term rather than reserving the name"
  (specs/value-constraints/spec.md). The added tests only cover fields and functions named `finite`
  (`crates/nx-types/tests/value_constraints.rs` `finite_patterns_compose_and_preserve_identifiers`).
- **Recommendation:** Pick the meaning of a lone `finite` arm from the scrutinee type. For example, parse a bare
  `finite` arm as an ambiguous name and resolve it in lowering or the checker: a numeric scrutinee gets the finite
  term, and a union or enum scrutinee gets the case. Keep `finite && ...` and `where finite` as they are. Add a
  regression test with a union case named `finite` matched by its bare name, and a constant named `finite` if that
  is a supported pattern.
- **Fix:** Resolve lone finite against union scrutinees using contextual case resolution before HIR emission; add checking and shared IR evaluation regressions.
- **Verification:** Lone `finite` arms on union scrutinees now resolve as contextual case names (`crates/nx-types/src/infer.rs`, rewrite in `crates/nx-hir/src/components.rs`), while numeric scrutinees keep the finite term. Probed `type Extent = finite | infinite` directly, through an alias, as an optional parameter, and as a payload case: legacy evaluation and TypeScript IR evaluation return the expected arms. Numeric `finite` arms still route Infinity/NaN to `else`. A union without the case reports "'finite' is not a case of union". `finite_remains_a_contextual_union_case_pattern` passes.

### ✅ Verified - RF2 The private `DashLength` alias leaks into the generated TypeScript and C# bindings for `@nx/graphics`, where it is never declared.
- **Severity:** Medium
- **Evidence:** `crates/nx-api/src/std/graphics/graphics.nx:108` declares the non-exported
  `type DashLength = float64 where finite && >= 0.0`, and the public `Stroke.dashArray` uses it
  (`graphics.nx:123`). `nxlang typegen --language typescript @nx/graphics` emits `dashArray?: DashLength[]` and
  `DashLength[] | null` (generated `graphics.ts` lines 206 and 611) with no `DashLength` declaration. `tsc`
  reports `TS2304: Cannot find name 'DashLength'`. The C# output has the same problem (`public DashLength[]?
  DashArray`, `NxProperty<Stroke, DashLength[]?>`). Before this change the field was `float64+`, which generates
  `number[]`/`double[]`. Graphics declares the `@nx-lang/graphics` / `NxLang.Graphics` packages
  (`crates/nx-api/src/artifacts.rs:1383-1384`), so these bindings are meant to compile.
- **Recommendation:** Make typegen expand non-exported aliases to their target (here `number[]`/`double[]`), or
  emit them as non-exported helper declarations. Exporting `DashLength` would conflict with design §7 ("Do not add
  new public numeric aliases"). Add a typegen test that generates every standard library and type-checks the
  TypeScript output (or at least asserts there are no unresolved names).
- **Fix:** Expand private aliases recursively in typegen public fields and companions. Added alias-chain tests, TypeScript compilation of all standard libraries, and C# numeric-array assertions; 140 typegen tests passed.
- **Verification:** `expand_private_aliases` (`crates/nx-cli/src/typegen/model.rs`) replaces non-exported aliases recursively in public fields, union cases, state, and update companions. Regenerated TS and C# for all six standard libraries: `dashArray` is now `number[]` / `double[]?`, no `DashLength` remains, and an independent `tsc --strict` over every generated TS library passes. `standard_library_typescript_bindings_type_check_without_private_aliases` and `private_alias_chains_expand_in_public_fields_and_companions` pass.

### ✅ Verified - RF3 Several raw-scene defenses that tasks 2.4 and 2.6 claim are verified have no tests, and the source migration removed the tests that used to reach some of them.
- **Severity:** Low
- **Evidence:** In `crates/nx-graphics/src/lib.rs`, preparation still rejects `w <= 0.0 || h <= 0.0` viewBoxes
  (around line 791), non-matching module identities (line 634), fractional `fontWeight` (line 919), and
  non-finite raw values via `prepare_value` (`graphics-non-finite`, lines 1864-1874). Only some of these are
  tested:
  - Zero viewBox dimensions: tested through NX source (`routing.test.mjs`).
  - Raw out-of-range opacity: tested (`limits_numbers_strokes_and_clips`).
  - No test supplies a negative or non-finite raw viewBox.
  - No test supplies a child whose identity is the geometry `Rect` (`@nx/geometry/geometry.nx`). The spoof test
    only uses a generic wrong identity.
  - No test anywhere asserts "invalid font weight" or a 400.5 weight.
  - No test still asserts `graphics-non-finite`. The one check that did
    (`scripts/check-graphics-preview.py:152`, `<g.Rectangle width={1.0e309 - 1.0e309} ...>`) now stops at
    `constraint-violation` before preparation runs.

  The graphics-library spec requires these raw-value rejections in the scenarios "A viewport retains stricter
  dimension rules", "A geometry rectangle is not a drawable element", and "Cross-field validation remains
  necessary".
- **Recommendation:** Add nx-graphics unit tests that use raw JSON or `NxValue`:
  - viewBox width `-1`, plus a non-finite width through `prepare_value`
  - a resolver returning `{module: "@nx/geometry/geometry.nx", declaration: "Rect"}` for a child
  - `fontWeight: 400.5`
  - a non-finite numeric field that still yields `graphics-non-finite`
- **Fix:** Add raw viewport negative/nonfinite, geometry Rect child and fractional fontWeight rejection tests; 28 neutral graphics tests passed.
- **Verification:** New nx-graphics tests cover raw negative viewBox width and height, non-finite viewBox fields through `prepare_value` (asserting `graphics-non-finite` and the element path), a child whose identity is `@nx/geometry/geometry.nx` `Rect`, and `fontWeight: 400.5` ("invalid font weight", reached before text shaping). Confirmed the test config version is `test`, matching the resolvers, so the geometry-child rejection really fails on its module, not on a version mismatch. All pass.

### ✅ Verified - RF4 No test shows that a non-v4 scene is rejected, and the rendering delta spec still names v2 as the old profile.
- **Severity:** Low
- **Evidence:** The gates exist: `sites/playground/src/graphics/canvas.ts:10` and `Scene::draw` in
  `crates/nx-graphics/src/lib.rs:236`. But no test, browser check, or script feeds either one a scene with another
  profile. Searching for "Unsupported graphics profile" or for scenes built with non-v4 profiles finds only the
  implementations. Task 3.1 ("verify ... unsupported revisions fail explicitly") and the graphics-preview scenario
  "Browser profile mismatch is explicit" have no supporting test. Separately,
  `specs/graphics-rendering/spec.md:28` still says "a host accepting only graphics-core-v2 SHALL reject the changed
  payload". With this revision, the previous host is v3.
- **Recommendation:** Add tests:
  - `renderCanvas({...scene, profile: "graphics-core-v3"})` throws (playground unit test or browser check).
  - A prepared `Scene` whose `profile` is changed to v3 fails `draw`.

  Update the scenario to say graphics-core-v3, or "a host accepting only an earlier profile".
- **Fix:** Add native and browser rejection tests for v3 and unknown profiles, and correct the previous-profile delta scenario to v3.
- **Verification:** `earlier_and_unknown_profiles_fail_before_destination_calls` checks that `Scene::draw` rejects v3 and v999 before any destination call. The playground test "Canvas rejects earlier and unknown graphics profiles before creating a frame" covers `renderCanvas` (156/156 playground tests pass). The delta scenario now names graphics-core-v3, and `openspec validate --strict` passes.

### ✅ Verified - RF5 The browser check meant to show that the viewport Rect is not a drawable element can never fail.
- **Severity:** Low
- **Evidence:** `sites/playground/scripts/check-graphics-browser.mjs:906` asserts
  `doesNotMatch(gradientElements, /ViewBox/)`. `ViewBox` no longer exists anywhere in the migrated sources, so the
  assertion is vacuous. The graphics-preview scenario "Inspection identifies the drawable declaration" requires
  that the shared Rect viewport value "SHALL NOT appear as a drawable child".
- **Recommendation:** Assert `doesNotMatch(gradientElements, /\bRect\b/)`. `\bRect\b` does not match `Rectangle`,
  so it catches a Rect viewport leaking into the element list.
- **Fix:** Assert that the whole word Rect does not appear in the browser drawable list, allowing Rectangle.
- **Verification:** Still effectively vacuous. Each option renders as `{declaration} · {id}` (`sites/playground/src/graphics/DrawingView.tsx:116-118`) and node ids are `element`, `element/children/N` (`crates/nx-graphics/src/lib.rs:728`). The combobox `textContent` concatenates options with no separator, so a leaked label is always glued to a word character, as in `element/children/0Rect · ...`, and `/\bRect\b/` cannot match it. Checked: `/\bRect\b/.test("Drawing · elementRectangle · element/children/0Rect · element/children/1")` is `false`. Read the options individually, for example `select[aria-label="Drawing element"] option` with `allTextContents()`, and assert that no label's declaration part (the text before ` · `) equals `Rect`.

- **Fix (second pass):** Read option labels individually with allTextContents, extract each declaration before the separator, and assert exact Rectangle inclusion and Rect exclusion.
- **Verification (second pass):** `sites/playground/scripts/check-graphics-browser.mjs:904-908` now reads each `option` of the "Drawing element" combobox with `allTextContents()`, splits each label on the same ` · ` separator that `DrawingView.tsx` renders (byte-identical U+00B7), and asserts exact `Rectangle` inclusion and exact `Rect` exclusion. The inclusion assertion also guards against a separator mismatch or an empty list. A mutation check of the same logic passes the clean list and rejects both a leaked `Rect · element/children/1` label and a list with no Rectangle. `pnpm --dir sites/playground test:graphics-browser` passed in Chromium (exit 0), including "The gradients playground example renders with shared Rect data and Rectangle elements."

### ✅ Verified - RF6 The graphics.nx constraint annotations dropped part of the `dashArray` API docs and broke the style guide's comment alignment.
- **Severity:** Low
- **Evidence:** In `crates/nx-api/src/std/graphics/graphics.nx:123-125`, the aligned `///` continuation "Odd-length
  lists repeat; empty or all-zero lists give a solid stroke." became a `//` comment (line 124). That behavior note
  is no longer in hover or generated docs: the generated TS doc for `dashArray` has only the first sentence. The
  `/// Offset into ...` block on line 125 also follows a trailing `///` comment without the blank line the style
  guide requires. Trailing doc comments are no longer aligned within their groups:
  - `viewBox: Rect` (line 190)
  - `pointCount` (lines 322 and 367)
  - the coordinate fields of Rectangle, Circle, Ellipse, Line, and the helpers
  - Rect `width`/`height` in `crates/nx-api/src/std/geometry/geometry.nx:23-24`, which sit one column off from
    `x`/`y`

  The style guide (style-guide.md, "Comments and documentation") requires aligned trailing comments within a
  record, and AGENTS.md requires following it.
- **Recommendation:** Put the odd-length/all-zero sentence back into the `dashArray` documentation, for example as
  a leading `///` block after a blank line. Realign the trailing comments in each touched record or component, or
  move the constrained fields' docs into leading blocks throughout a record.
- **Fix:** Restore full dashArray docs and use leading property documentation in graphics and geometry. Refresh source-version-sensitive native preview and gradient fixtures.
- **Verification:** `dashArray` again documents the odd-length and all-zero behavior as a leading `///` block; the regenerated TS JSDoc includes both sentences. Graphics and geometry fields now use leading documentation, so no misaligned trailing `///` comments remain. The geometry and graphics libraries analyze cleanly in the Rust suites. Native preview and gradient fixtures were refreshed, and `scripts/check-graphics-preview.py` passes.

### ✅ Verified - RF7 The geometry IR conformance fixture skips the spec's sequence-of-Rect round trip and never runs its NaN case.
- **Severity:** Low
- **Evidence:** The geometry-library scenario "Geometry values round-trip across a function boundary" calls for "a
  record containing Point, Size, and a sequence of Rect values". `specs/ir-conformance/geometry-library/main.nx:6`
  has only a single `bounds: geometry.Rect`. `main.nx:29` defines `nan(x)`, which builds a NaN Point coordinate,
  but `program.json` has no entrypoint for it. So the shared Rust/TypeScript fixture does not check that NaN
  computed inside NX fails a geometry binding. The Rust unit test and `geometry.test.mjs` cover NaN only as a host
  argument.
- **Recommendation:** Add a `regions: geometry.Rect+` field (with at least two values, including a zero-area one)
  to `Document`. Add a `nan` entrypoint marked `"fails": true` with an `nx-ir-constraint` expectation, then
  regenerate the expected images and results.
- **Fix:** Add a sequence of Rect values including zero area, and execute an internally computed NaN case with pinned nx-ir-constraint diagnostics. Regenerate corpus fixtures; all 10 corpus checks passed.
- **Verification:** `Document` now has `regions: geometry.Rect+` with a normal Rect and a zero-area Rect. `results.json` pins both values. The `nan#computed` entrypoint (`x * x - x * x` with 1e308) is pinned to `nx-ir-constraint` in `diagnostics.json`. Rust corpus tests and the TypeScript runtime corpus and geometry tests pass on the regenerated images.

## Questions
- None.

## Verification Pass 2026-10-31
**Checks run:**
- `cargo test -p nx-api -p nx-codegen -p nx-types -p nx-syntax -p nx-hir -p nx-language-service -p nx-graphics -p nx-cli -p nx-interpreter`: 79 result blocks, no failures.
- `cargo test -p nx-graphics-skia --features preview`: 13 passed.
- `pnpm --dir runtime/typescript test`: passed.
- `pnpm --dir sites/playground test`: 156/156 tests passed, and all 26 examples matched.
- `python3 scripts/check-graphics-preview.py`: passed.
- `node scripts/verify-graphics-fixtures.mjs`: passed.
- `openspec validate add-geometry-standard-library --strict`: passed.
- `cargo fmt --check` and `git diff --check`: clean.
- Ad hoc RF1 probes through `nxlang run` and TypeScript IR evaluation.
- Ad hoc RF2 regeneration and `tsc` over all standard-library TypeScript bindings.

**Not run in the first pass:** the Playwright browser check. RF5 was reopened from code inspection and a regex check against the rendered option format.

## Verification Pass 2026-10-31 (RF5 second pass)
**Checks run:** `pnpm --dir sites/playground test:graphics-browser` (Chromium, passed); a Node mutation check of the new option-label assertion; a byte comparison of the ` · ` separator in `DrawingView.tsx` and the check script.

## Summary
- Final state: all seven findings (RF1-RF7) are ✅ Verified, and none are open.
- The two functional regressions are fixed. RF1: a bare `finite` arm now resolves as a contextual union case for
  union scrutinees and as the finite term for numeric ones. RF2: typegen expands private aliases, so the generated
  standard-library TypeScript type-checks and the C# uses `double[]`.
- The coverage, assertion, and documentation findings (RF3-RF7) are addressed with tests that pass: raw-scene
  defenses, profile rejection on native and browser paths, a non-vacuous browser inspection check, restored
  `dashArray` docs with leading field documentation, and the sequence-of-Rect and computed-NaN conformance cases.
- The affected Rust, TypeScript runtime, Skia, playground, native preview, fixture, and real-browser checks pass, as
  do `openspec validate --strict`, `cargo fmt --check`, and `git diff --check`. The original review's summary, which
  listed RF1 and RF2 as needing fixes, is superseded by this one.
