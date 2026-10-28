# Review: add-math-standard-library

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, and all five change specs; public math and IR reference documentation and conformance provenance.
**Reviewed code:** compiler standard-library registry and descriptor versioning; StandardMath HIR/type/codegen traversal; IR writer/reader/feature validation; Rust numerical implementation and IR/legacy evaluators; TypeScript runtime and generated TS/JS helpers; binding smoke-test additions; numerical, malformed-image, function-value, budget and geometry fixtures. Unrelated graphics proposals and other active changes were excluded.

## Findings

No actionable findings identified. No implementation edits were made by the reviewer.

## Questions

None.

## Validation

- `node runtime/typescript/test/math.test.mjs`: all 87 independent reference cases pass through TypeScript IR and generated TypeScript/JavaScript, including diagnostics and signed-zero checks.
- `cargo test -q -p nx-codegen standard_math`: 9 tests pass, covering Rust IR/legacy references, identity, malformed images, feature gating, budgets, call origins and geometry.
- `cargo test -q -p nx-api standard_library`: 30 tests pass.
- `cargo test -q -p nx-ir-runtime`: all enabled tests pass (one pre-existing ignored test).
- `openspec validate add-math-standard-library --strict`: passes.
- Additional temporary authored-identifier probe uses `nxStandardMath`, `nxStandardMathNames` and `nxStandardMathArities` alongside imported math. Generated JavaScript files and program-module output parse and return the expected result, 16. For execution in plain Node, the temporary program-module probe maps its host-provided `nx:runtime` import to the generated runtime file.

## Summary

The reviewed implementation matches the change's core numerical, identity, capability-gating and runtime contracts. No high-confidence fixes are required based on this review. Binding and website additions were inspected; their full platform build suites were not rerun in this independent review.

## Independent verification pass

The same independent reviewer ran `openspecex-review-verify` after writing this report and reloaded OpenSpec status/apply context. There are no `Fixed` findings, so no fixes are claimed as verified. Findings verified: 0; reopened: 0; new findings: 0.

The TypeScript runtime test executable passes, including unknown math operation, wrong arity, invalid reference and absent feature cases. The temporary collision probe also executes successfully in program-module mode after supplying its expected runtime import. `git diff --check` passes. Legacy imported constant/function-value resolution was inspected for shadowing and isolated-context usage, and the Rust reference/function-value fixtures pass. No additional actionable issue was identified.

## New Findings Discovered During 2026-10-28 01:14 Review (Claude)

**Reviewer:** Claude (Opus 5.5). This is a fresh, independent pass over the same change.
**Reviewed artifacts:** proposal.md, design.md, tasks.md, validation.md, release-notes.md, and all five change specs (OpenSpec status/apply context reloaded).
**Reviewed code:** working-tree and untracked files: `crates/nx-value/src/math.rs` and `math-cases.json`, `specs/math-conformance/`, `crates/nx-api/src/artifacts.rs` (descriptor synthesis and versioning), HIR/type/scope/component traversal, `crates/nx-codegen` (builder, IR emitter, TS/JS emitter, runtime helper, tests), `crates/nx-ir` (model, image reader/writer validation, explain), `crates/nx-ir-runtime` (decoder, evaluation, call-origin mapping), legacy interpreter, `runtime/typescript` (src, tests, committed dist), Node/WASM/.NET binding tests, website pages, `docs/nx-ir-format.md`, IR conformance fixtures and `examples/math-graphics/`. Other active changes and the graphics proposal docs were excluded.

### ✅ Verified - RF1 `lerp` reports `nx-math-overflow` for extrapolations whose result can be represented
- **Severity:** Low
- **Evidence:** `crates/nx-value/src/math.rs:227-231` and `runtime/typescript/src/math.ts:57-58` choose the formula by endpoint sign, not by whether `t` is in `[0,1]`. With same-sign endpoints and `t > 1`, `t * (b - a)` can overflow even though the result fits. Reproduced in both runtimes: `lerp(1.7e308, 1e307, 1.2)` fails with `nx-math-overflow`, but the exact result is about `-2.2e307`, and `(1 - t) * a + t * b` evaluates to `-2.1999999999999994e307`. The design says only a non-finite *final* result is diagnosed, and the spec reserves `nx-math-overflow` for results that cannot be represented. The legacy interpreter shares the Rust code and generated TS/JS share `math.ts`, so every path reports this false error.
- **Recommendation:** If the chosen form gives a non-finite value, compute the other form (`(1 - t) * a + t * b` or `a + t * (b - a)`) before reporting overflow. Alternatively, choose the form by the range of `t` as well as the endpoint signs. Add a named corpus fixture in `generate.py` such as `lerp-extrapolation-no-intermediate-overflow`, so all six paths check it.

- **Fix:** Rust and TS retry non-finite extrapolation with the weighted form, then a scaled difference form when both unscaled forms overflow. Added independent positive and negative extrapolation regression fixtures; all runtime/generated paths share the fixes.
- **Verification:**
  - Code: `crates/nx-value/src/math.rs` (same-sign branch) and `runtime/typescript/src/math.ts:59-61` retry with `(1 - t) * a + t * b`, then with the half-scaled difference form, before reporting overflow.
  - Fixtures: `lerp(1.7e308, 1e307, 1.2)` exercises the weighted fallback. `lerp(1e307, 1.7e308, -1.15)` overflows both unscaled forms and so exercises the scaled path. Both expectations come from mpmath, and `lerp-overflow` still fails as it should.
  - The Rust fallback is limited to the same-sign branch, while TS also retries after the opposite-sign form. They behave the same: in a differential probe of 200,000 random `(a, b, t)` with endpoints near ±1.79e308 and `t` in `[-4, 4]`, Rust and TS returned 0 different results and 0 false overflows against a scaled reference.
  - All six paths pass.

### ✅ Verified - RF2 The shared corpus misses several scenarios the spec names, and non-finite input is tested only for `sin`'s single argument
- **Severity:** Low
- **Evidence:** `specs/math-conformance/generate.py` and `crates/nx-value/src/math-cases.json` (87 cases) have:
  - no case for the "Inverse trigonometric domains" scenario: `asin(1.0)` about pi/2, `acos(1.0)` exactly `+0`, `atan(0.0)` exactly `+0`. The corpus has only the `-0` variants and `0.5`, and a repo-wide search finds these calls nowhere else.
  - `nx-math-non-finite-input` cases only for `sin` with one argument. Nothing checks a non-finite second or third argument, such as an infinite `clamp` bound, a NaN `atan2` x, or an infinite `lerp` t, although the requirement covers "every public math function".
  - no `lerp` extrapolation with `t < 0`, which task 3.3 asks for as "out-of-range t behavior".
  - no inexact interior `lerp` case. The only interior case, `(-1e308, 1e308, 0.5)`, is an exact `0.0`.
  - no `clamp` case with equal or inclusive bounds.

  Today the Rust and TS code is uniform enough to pass these. The corpus is the only cross-path guard, though, so a future change to one path would go unnoticed.
- **Recommendation:** Add these cases to `generate.py` and regenerate `math-cases.json`: the exact inverse-trig scenario values, non-finite values in each argument position of `atan2`, `hypot`, `min`, `clamp` and `lerp`, `lerp(0,10,-0.5)`, an inexact interior `lerp`, and `clamp(5,2,2)`. All Rust, TS IR and generated paths read this file, so they pick the cases up automatically.

- **Fix:** Expanded the generated corpus from 87 to 184 cases: named inverse-trig endpoints, every operation/argument position with NaN and both infinities, negative and inexact interpolation, inclusive/equal clamp bounds, and overflow regression cases.
- **Verification:**
  - `generate.py` now adds: `asin-upper-bound`; exact `acos-upper-bound = +0` and `atan-positive-zero = +0`; NaN, +Infinity and -Infinity in every argument position of all 22 operations (90 cases); `lerp-negative-extrapolation`; `lerp-interior-inexact`; and the `clamp` equal and inclusive bounds.
  - I re-ran the generator outside the repo with mpmath 1.3.0, writing to a temporary path. The output is byte-identical to the checked-in 184-case `math-cases.json`, and all names are unique.
  - The Rust primitive, Rust IR and legacy paths, TS IR, and generated TS/JS all pass the corpus (`math: 184 reference cases pass`).

### ✅ Verified - RF3 The IR reference and the standard-library reference are not updated where readers look for node kinds, features and libraries
- **Severity:** Low
- **Evidence:**
  - In `docs/nx-ir-format.md`, the node-kind table ends at kind 25 (`:155`), and the "Required features" section (`:501-518`) lists every feature except `standard-math-v1`. The new material is a separate section appended after the unrelated reducer discussion (`:1443`), so a reader using the table or the feature list does not find kind 26 or the new feature.
  - In `sites/website/src/content/docs/reference/syntax/modules.md`, the standard-library table (`:108-110`) still lists only `@nx/agent` and `@nx/visualizer`. The definition at `:85` ("A standard library is NX source the compiler carries") no longer covers runtime-provided declarations.
  - Task 5.2 asks for links from the relevant reference/import guidance. The language-tour pages link to the new page, but the reference index of standard libraries does not.
- **Recommendation:** Add a kind-26 row to the node table that points to the standard-math section, and add "one that contains a `standardMath` node lists `standard-math-v1`" to the Required features paragraph. Add `@nx/math` (unstable, float64 mathematics) to the modules.md table, and reword the definition to cover compiler-owned runtime-provided declarations.

- **Fix:** Added node kind 26 and standard-math-v1 to the main IR tables, updated the modules reference definition/table with @nx/math, and linked its math reference.
- **Verification:**
  - `docs/nx-ir-format.md:156` adds a kind-26 `standardMath` row that links to the operation and arity section. `:514-515` adds `standard-math-v1` to the Required features paragraph.
  - `reference/syntax/modules.md:85-86` now defines standard libraries as compiler-owned declarations "expressed in NX source or provided by the runtime". `:110` lists `@nx/math` with a link.
  - `check-code-blocks.mjs` on the four touched pages: 15 NX blocks, 0 failures.

### ✅ Verified - RF4 The trusted descriptor path is wired through hard-coded `@nx/math` strings and duplicated operation tables, not one registry entry
- **Severity:** Low
- **Evidence:**
  - Three separate string checks identify the runtime-provided library: `crates/nx-api/src/artifacts.rs:1242` (`self.root == "@nx/math"` in the version hash), `:1327` (`library.root == "@nx/math"` in declaration synthesis) and `crates/nx-codegen/src/ir.rs:1012` (`module_identity(...) == "@nx/math/math.nx"` to suppress empty spans). `MathOp::descriptor` also hard-codes the owner module (`crates/nx-value/src/math.rs:117`).
  - Design decision 2 calls for a typed registry in which each entry records its owner module, identity/signature, operation and revision.
  - Operation arity is restated as magic numbers in `crates/nx-ir/src/image.rs:1342-1346` (`6 | 8 | 11 | 12 => 2, 13 | 21 => 3, _ => 1`). `standard_math_operation_tables_agree` (`crates/nx-codegen/src/math_tests.rs:257`) checks names and codes but not arity. A future operation would therefore silently default to arity 1 in the format reader while the runtime decoder uses `MathOp::arity`.
  - The call-origin `map_err` closure is copied verbatim at `crates/nx-ir-runtime/src/eval.rs:1023` and `:1058`.
- **Recommendation:**
  - Add a runtime-functions field to `StandardLibrary`, for example `runtime_functions: &'static [MathOp]`, set only on the math entry. Derive the hash input, the declaration synthesis and the emitter's synthetic-span policy from it, and remove the string comparisons.
  - Put arity next to the names in `nx_ir::kinds::standard_math` (for example `(code, name, arity)`), use it in `image.rs`, and extend the agreement test to compare it with `MathOp::arity` and `nxStandardMathArities`.
  - Move the origin-mapping closure into one helper.

- **Fix:** Each StandardLibraryModule now owns its runtime_functions list. Declaration synthesis, descriptor hashing/owner identity, synthetic-span policy and corpus sizing derive from registry metadata. Added explicit IR arities and cross-table Rust/TS agreement checks, extracted shared call-origin mapping, and tested a non-math fixture registry entry.
- **Verification:**
  - Registry: `StandardLibraryModule::runtime_functions` (`crates/nx-api/src/artifacts.rs:1179`) drives the version hash, using a descriptor whose owner is `<root>/<identity>`. It also drives declaration synthesis, the emitter's synthetic-span policy (`crates/nx-codegen/src/ir.rs:1010-1027`) and corpus size accounting. No non-test Rust code compares against `"@nx/math"` any more; the root appears only in the registry entry.
  - Unchanged hashes: the descriptor text is unchanged for math, and source-only libraries add nothing to their hash. The pinned-hash tests still pass.
  - Fixture test: `standard_library_runtime_descriptors_follow_module_registry_metadata` builds a non-math `@nx/fixture` library and checks the version difference, determinism, the synthesized `cos` export and that the library has no diagnostics.
  - Arities: `nx_ir::kinds::standard_math::ARITIES` replaces the magic numbers in `image.rs:1342-1348`, and an unknown operation now gets an explicit error. `standard_math_operation_tables_agree` compares the arities with `MathOp::arity` and with the parsed TS `nxStandardMathArities`.
  - Call origin: a single `math_call_origin` helper (`crates/nx-ir-runtime/src/eval.rs:1049`) serves both call paths.

### ✅ Verified - RF5 The generated JavaScript math helper comes from the TypeScript source by unchecked string replacement
- **Severity:** Low
- **Evidence:** `crates/nx-codegen/src/runtime.rs:1315-1331` builds the generated helper from `runtime/typescript/src/math.ts` with eight literal `.replace` calls. The first one swaps the plain `Error` for `NxRuntimeError`, and the others strip TS annotations for JavaScript. `str::replace` does nothing when its pattern is missing. Reformatting `math.ts` or renaming a local can therefore produce generated JS with TS syntax left in, or generated TS/JS that throws a plain `Error`, which loses `nxDiagnosticsFromError` categories. Only `runtime/typescript/test/math.test.mjs` would notice, and it runs through cargo and `nx-cli` from the TS package. The Rust test `standard_math_codegen_keeps_authored_same_named_functions` runs only a success path, so it does not catch a lost `NxRuntimeError`.
- **Recommendation:** Route each substitution through a helper that asserts the pattern was found, for example by panicking in a unit test or comparing match counts. Add a Rust test that the JS helper contains no `as const`, `!`-assertions or type annotations and that its domain failure produces an `NxRuntimeError` with code `nx-math-domain`. A simpler option is one JS-compatible source, such as JSDoc-typed JS plus a `.d.ts`, that both outputs include unchanged.

- **Fix:** All helper substitutions assert their expected match counts. Rust tests check failed substitutions, JavaScript parsing, removal of TS syntax, and NxRuntimeError plus nxDiagnosticsFromError domain categories.
- **Verification:**
  - `replace_math_fragment` (`crates/nx-codegen/src/runtime.rs:1315`) asserts the exact match count for the `NxRuntimeError` swap and for all seven JS-stripping substitutions.
  - `standard_math_helper_rejects_changed_source_fragments` shows that a missing fragment panics.
  - `standard_math_javascript_helper_parses_and_preserves_error_categories` runs the full JS helper in Node. It checks that `sqrt(-1)` throws an `NxRuntimeError` whose code is `nx-math-domain`, both directly and through `nxDiagnosticsFromError`. It also checks that no TS-only fragments remain and that the TS variant uses `NxRuntimeError`.
  - Because the source comes from `include_str!`, any drift makes every codegen test fail, not only these two.

## Questions (2026-10-28 Claude review)

- A JavaScript program module that uses math now imports `nxStandardMath` from the host's `nx:runtime` but still declares `runtimeAbi: "nx-js-runtime-v1"` (reproduced with `nxlang codegen --format program-module`). A host that compares ABIs as the README says and supplies a runtime built before this change will pass the check, then fail at ESM link time with a missing export. `nxApplyUpdate` set a precedent of adding helpers without an ABI bump. Is that the intended policy, or should the ABI be bumped, or the requirement documented as "upgrade compiler and JS runtime together" in the README and the math reference?

## Validation (2026-10-28 Claude review)

- `cargo test -q -p nx-value -p nx-ir -p nx-ir-runtime -p nx-codegen -p nx-interpreter -p nx-api`: all suites pass (0 failures; pre-existing ignored tests only).
- `node runtime/typescript/test/math.test.mjs`: 87 reference cases pass in TypeScript IR and generated TS/JS. `node runtime/typescript/dist/test/runtime.test.js` passes, including the standard-math validation test.
- The TS runtime, rebuilt into a temporary directory, matches the committed `runtime/typescript/dist` exactly.
- `node sites/website/scripts/check-code-blocks.mjs` on the three touched docs pages: 11 NX blocks, 0 failures.
- `cargo package -p nx-value --list` includes `src/math.rs` and `src/math-cases.json`, so the published crate's corpus test is self-contained.
- Temporary probes outside the repo:
  - RF1 reproduced in both the Rust (`nx_value::math::evaluate`) and TS (`nxStandardMath`) primitives.
  - Host non-number and bigint arguments are rejected at the TS boundary (`nx-ir-boundary-type`).
  - The compiler rejects int literals that float64 cannot represent exactly.
  - Unaliased and selective imports evaluate correctly.
  - Authored same-named exports take precedence over math in the generated index.
  - Rounding ties, negative halves, `-0` results and the near-pole tangent agree between Rust and TS.
- Not rerun: Node native, WASM and .NET binding suites (inspected only).

## Summary (2026-10-28 Claude review)

The core contracts are sound and well tested: numerics, signed zeros, both rounding modes, capability gating, malformed-image rejection, function-value identity, budgets and call origins. Five low-severity findings remain:
- one numerical correctness edge in `lerp` extrapolation (RF1)
- corpus gaps against named spec scenarios (RF2)
- documentation placement gaps (RF3)
- two maintainability risks in the trusted-descriptor wiring and the derived JS helper (RF4, RF5)

There is one open question about the JS runtime ABI policy.

## Implementation response to Claude review

All five findings are fixed pending independent Claude verification. The JS ABI question is
addressed in the root README and math reference: program modules must use the helper bundle from
the same compiler release. `NX_JS_RUNTIME_ABI` describes the calling contract rather than a
minimum helper-export release; a matching ABI alone does not guarantee `nxStandardMath` exists.
The generated `files` layout already includes its matching helper.

Validation: the expanded TypeScript suite passes (184 numerical cases through IR/generated TS/JS),
the new registry-metadata test passes, changed documentation examples compile, the website build
and internal links pass, and strict OpenSpec validation, formatting and diff checks pass. The broad
Rust regression run is recorded in validation.md after completion.

## Claude verification pass (2026-10-28 01:25 America/New_York)

The same Claude reviewer reloaded the OpenSpec status/apply context, re-read review.md and inspected the current working-tree files afresh.

**Results:**
- Verified: 5 (RF1, RF2, RF3, RF4, RF5)
- Reopened: 0
- New findings: 0

**ABI question:** answered and closed. The root README and the math reference now say that a JavaScript `program-module` must be deployed with the helper bundle from the same compiler release, and that a matching `NX_JS_RUNTIME_ABI` does not guarantee the `nxStandardMath` export. The generated `files` layout includes its matching helper. This documents the existing additive-helper policy, which `nxApplyUpdate` set as a precedent. No ABI bump is required.

**Validation:**
- `cargo test -q -p nx-value -p nx-ir -p nx-ir-runtime -p nx-codegen -p nx-interpreter -p nx-api`: all suites pass with 0 failures; only pre-existing tests are ignored. This includes the new registry-metadata, helper-substitution and JS-helper tests.
- `cargo test -q -p nx-language-service hover_describes_standard_math_signature` passes.
- `cargo fmt --all -- --check` and `git diff --check` pass.
- TypeScript runtime: `node test/math.test.mjs` passes 184 reference cases in TS IR and generated TS/JS. `dist/test/runtime.test.js`, `test/corpus.test.mjs`, `test/emitted-ir.test.mjs` and `bench/core.test.mjs` all pass.
- The TS runtime, rebuilt into a temporary directory, matches the committed `dist` exactly.
- The corpus generator, re-run with mpmath 1.3.0 into a temporary path, reproduces `math-cases.json` byte for byte.
- A differential `lerp` probe over 200,000 cases found no difference between Rust and TS and no false overflow.
- `check-code-blocks.mjs` on four docs pages: 15 blocks, 0 failures.
- `openspec validate add-math-standard-library --strict` passes.
- Not rerun: the Node native, WASM and .NET binding suites. Their sources did not change in this fix round.

**Remaining questions:** none.
