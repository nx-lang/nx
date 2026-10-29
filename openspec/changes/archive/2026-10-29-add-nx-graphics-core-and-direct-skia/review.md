# Review: add-nx-graphics-core-and-direct-skia

## Scope
**Reviewed artifacts:** proposal.md, design.md, math-readiness.md, tasks.md, specs/graphics-library/spec.md,
specs/graphics-rendering/spec.md, specs/graphics-preview/spec.md\
**Reviewed code:** crates/nx-api/src/std/graphics/graphics.nx, crates/nx-api/src/artifacts.rs,
crates/nx-api/src/standard_library_tests.rs, crates/nx-types/src/infer.rs, crates/nx-interpreter/src/interpreter.rs\

Implementation is partial: only the task 1.1 groundwork exists. Tasks 1.2-6.4 (Text profile rules,
neutral `nx-graphics`, Skia adapter, helpers, preview host, corpus, docs) are planned work and are
not reported as defects. The proposal sources under `docs/` hold the user's earlier edits and were
not reviewed as part of this change.

Another agent edited the working tree during this review (snapshot 2026-10-28 12:52). Between 12:45
and 12:52, `graphics.nx` comments changed and a candidate fix for `Record type not found: ViewBox`
appeared in `interpreter.rs`. The findings describe the 12:52 state.

Evidence was gathered in a scratch copy of the tree, outside the repository. A throwaway probe
evaluated the same sources through `eval_program_artifact` (HIR interpreter) and through
`emit_nx_ir` + `nx-ir-runtime`. `cargo test -p nx-types -p nx-api` at 12:52: 451 passed, 1 failed
(`graphics_standard_library_is_standalone_and_links_math`).

Probe results (current tree):

| Source | HIR interpreter | IR runtime |
| --- | --- | --- |
| `<g.Drawing viewBox=<g.ViewBox .../>><g.Rect .../></g.Drawing>` | `expected Element, got Rect` | ok, with defaults |
| Same, unaliased `import "@nx/graphics"` | `expected Element, got Rect` | ok |
| Local `external component <Box content children?: Element+ />` with `<Box><Shape/></Box>` or `<Box><div/></Box>` | `expected Element, got Shape` / `got div` | ok |
| `<g.Rect ... transform={<g.Transform.translate x=1 y=2/>}/>` | `expected Transform, got g.Transform.translate` | ok |
| Unaliased `<Rect ... transform={<Transform.translate .../>}/>` | ok | ok |
| `<g.ContentAlignment/>`, `g.Paint.none` | ok (after the in-tree record fix) | ok |
| Local `component <Rect .../>` decoy beside `<g.Rect width=2 height=3/>` | type-checks with the infer.rs fix; reverting it gives `g.Rect has no property 'height'` | ok |
| All-module emit for an aliased graphics consumer | — | images `main.nx`, `@nx/graphics/graphics.nx`, `@nx/math/math.nx` |

## Findings

### ✅ Verified - RF1 The HIR interpreter rejects every element value at an `Element`-typed prop, so no graphics Drawing or Group with children evaluates through `eval_program_artifact`.
- **Severity:** High
- **Evidence:** Once the `ViewBox` lookup is fixed, the new regression test fails at
  `crates/nx-api/src/standard_library_tests.rs:1114` with `Type mismatch in component prop
  initialization 'Drawing.children': expected Element, got Rect`. This predates the change: a local
  `external component <Box content children?: Element+ />` fails the same way for a component
  child (`got Shape`) and an intrinsic child (`got div`). `Element` resolves at runtime to a plain
  `Type::Named("Element")` (`interpreter.rs:4233-4262`). `record_value_matches_expected_type`
  (`interpreter.rs:4735`) then tries name equality, record subtyping and `is_component_subtype`
  (`interpreter.rs:4724`), and none of them knows `Element`. The checker special-cases it
  (`crates/nx-types/src/infer.rs:8319-8323` → `named_type_is_element_like`). The IR runtime accepts
  all of these cases. `eval_program_artifact`, `nxlang run` and the .NET, Node and WASM bindings still
  use this interpreter (`retire-hir-interpreter` is not done), so graphics drawings are unusable on
  those paths.
- **Recommendation:** This is a focused, reproduced blocker of the kind task 1.3 allows. In
  `record_value_matches_expected_type`, make an expected `Named("Element")` (in item position, so
  `Element+` works through the existing `Seq` arm) accept any interpreter-built `Value::Record`. The
  checker already proved the site element-like, as it does for other statically checked values.
  Keep union-case values on their existing path. Add an `nx-interpreter` or `nx-api` regression test
  covering a local external component, an intrinsic child, and an imported library component child.
- **Verification:** `record_value_matches_expected_type` (`interpreter.rs:4745-4748`) now accepts
  any interpreter-built record at a `Named("Element")` item, matching the checker's name-based rule
  (`infer.rs:8319-8323`). Union-case values keep their own path. The new
  `element_props_accept_local_components_and_intrinsic_children` covers a local external component
  and a `<div/>` child. The graphics test covers an imported `g.Rect` child. Both pass.

### ✅ Verified - RF2 An aliased imported record element (`<g.ViewBox .../>`) fails at runtime with `Record type not found`, and the candidate fix in the tree has no regression test and leaves a stale workaround note.
- **Severity:** High
- **Evidence:** This is the reported failure. Before the 12:52 edit, `eval_element_expr` resolved
  the record through the tag (`g.ViewBox`) but then called `effective_record_shape(module,
  &record_def.name)` with the caller module and the bare name. That name does not exist in
  `main.nx`, so the lookup fell back to the caller module and returned `None`. The working tree now
  resolves `(target_module, declared name)` inside `effective_record_shape`
  (`interpreter.rs:4648-4652`) and passes the tag (`interpreter.rs:3519-3520`). My probe confirms
  `<g.ViewBox/>` and `<g.ContentAlignment/>` (with union-case defaults) evaluate after it. Remaining
  problems:
  - The `_record_def` binding at `interpreter.rs:3519` is resolved and then thrown away, and the
    item is resolved a second time inside `effective_record_shape`.
  - The other `effective_record_shape(module, &record_def.name)` callers (`interpreter.rs:3354`,
    `4864`) still pass a bare name with the caller's module. `g.ViewBox(1, 2, 3, 4)` happens to
    work today only because the constructor path differs.
  - No interpreter-level test pins the fix.
  - `crates/nx-api/src/nx_text.rs:1413-1418` still says the interpreter cannot construct an aliased
    record.
- **Recommendation:** Keep the target-module resolution, but make the element branch read
  `if self.resolve_record_definition(module, tag_name).is_some()`. Better, have one helper return
  `(target_module, shape)` and use it at every caller. Add a focused regression test: aliased record
  element, aliased record with defaults, and a same-name local record decoy. Update the `nx_text.rs`
  comment, and convert that test to a source round-trip if it now works.
- **Verification:** The element branch now uses `.is_some()` (`interpreter.rs:3519`).
  `effective_record_shape` resolves the declaring module and declared name (`interpreter.rs:4648-4652`).
  The checker's record-invocation path now resolves `element.tag` (`infer.rs:4888`), which matches
  the record `owner` lookup next to it. `graphics_aliased_records_defaults_and_payload_cases_evaluate`
  constructs `<g.ViewBox/>` and `<g.ContentAlignment/>` (defaults) beside a local
  `type ViewBox = { unrelated:string }` decoy, and it passes. The `nx_text.rs` note no longer claims
  the interpreter cannot construct aliased records. Leaving the value-level test and the other
  `effective_record_shape` callers alone is acceptable: `g.ViewBox(1, 2, 3, 4)` already evaluated.
  Construction is fixed. Passing the constructed value back into an alias-typed site in the
  consumer module is a separate defect, tracked as RF7.

### ✅ Resolved - RF3 Aliased payload union cases keep the import alias in their runtime type name, so `<g.Transform.translate/>` is rejected at `Rect.transform`.
- **Severity:** Medium
- **Evidence:** `eval_element_expr` builds the case value with `Name::new(tag_name)`
  (`interpreter.rs:3438`), which stores the authored spelling `g.Transform.translate`. Prop
  coercion then runs in the graphics module (`normalize_component_props(target_module, …)`), where
  `union_case_value_matches_expected_type` (`interpreter.rs:4766`) re-resolves `g.Transform` and
  fails. The error is `expected Transform, got g.Transform.translate`. The unaliased spelling and
  aliased constant cases (`g.Paint.none`) work. The spec requires "Alias spelling SHALL NOT change
  component identity" (graphics-library, first requirement), but the alias currently leaks into
  evaluated values. The IR runtime emits `Transform.translate`.
- **Recommendation:** Build the value with the declaration's canonical name,
  `format!("{}.{}", union_def.name, case.name)`, as the constant-case path already does
  (`interpreter.rs:4593`). Add aliased `transform`, `fill=<g.Paint.solid …/>` and
  `stroke=<g.Stroke paint=…/>` cases to the regression test.
- **Verification:** Reopened: the fix regresses consumer-module sites. `interpreter.rs:3438` now
  stores `Transform.translate`, so the graphics-module props (`Rect.transform`) accept it and the
  new test passes. But the same value now fails at any site the *consumer* types with the alias.
  The checks resolve the bare union name in the consumer module, where only `g.Transform` exists
  (`union_case_value_matches_expected_type`, `interpreter.rs:4771`). Reproduced in a scratch copy;
  every case passes on `nx-ir-runtime`:
  - `let id(t: g.Transform): g.Transform = { t }` with `id(<g.Transform.translate x=1 y=2/>)` →
    `expected g.Transform, got Transform.translate`
  - `external component <Holder t: g.Transform />` → `'Holder.t': expected g.Transform`
  - `type Holder = { t: g.Transform }` → `record field 't': expected g.Transform`

  With line 3438 reverted to `Name::new(tag_name)`, all three passed (the value was
  `g.Transform.translate`). The old spelling is therefore accepted only in the consumer module and
  the new spelling only in the declaring module, so no bare name works at both kinds of site.
  Storing the canonical name is the right direction; the gap is in the matching side.
  `cargo test -p nx-codegen` (374 + 5 passed) and the supplied
  nx-types/nx-api/nx-interpreter log (1,823 passed, 0 failed) do not cover consumer-side alias-typed
  sites.
- **Recommendation (updated):** Keep the canonical name, and fix the matching side as RF7 describes
  so a canonical value matches an expected type spelled through an import alias. Extend the
  regression test with the three consumer-side cases above.

### ✅ Verified - RF4 The new tests don't verify what tasks 1.1 and 1.3 and the spec scenarios require, and the infer.rs fix has no test of its own.
- **Severity:** Medium
- **Evidence:**
  - `graphics_standard_library_is_standalone_and_links_math` (`standard_library_tests.rs:1096`)
    only asserts that `eval_program_artifact` is not `Err`. It checks no evaluated defaults
    (`x=0`, `fit=contain`, `clipRule=nonzero`), no inherited `opacity=0.5`, and no canonical
    identity, so it does not cover "Standalone drawing evaluates under an alias" or "Imported props
    and defaults work".
  - Task 1.3 requires IR emission and linking, with the all-module emit containing
    `@nx/math/math.nx` at its content version. Nothing exercises `emit_nx_ir` or `nx-ir-runtime`
    for graphics, although the probe shows it already works.
  - The one-line `infer.rs:4474` fix (`&component.name` → `&element.tag`) is necessary. Reverting it
    loses inherited props and defaults on aliased imported components (`g.Rect has no property
    'opacity'`, `requires property 'x'`). With a same-name local `Rect` present, `g.Rect` is
    silently checked against the local contract. Its only coverage is the nx-api test, which
    currently fails for unrelated runtime reasons.
- **Recommendation:**
  - Add an `nx-types` (or nx-api static-diagnostics) test for an aliased imported component that
    uses inherited props and omitted defaults, plus a same-name local component decoy.
  - Extend the nx-api test to assert the normalized fields.
  - Add an nx-codegen IR test (like `math_tests.rs`) that emits all modules, asserts the three
    image identities and the math image's `standard-math-v1` feature, links, and compares the
    output with the HIR result.
- **Verification:**
  - `graphics_standard_library_is_standalone_and_links_math` now asserts `$type`, `fit`, root
    `opacity`, and the child's `x`/`y` defaults, `clipRule` and inherited `opacity=0.5`.
  - `graphics_alias_checks_use_the_imported_contract_despite_local_decoy` type-checks
    `<g.Rect … opacity=0.5/>` beside a local `external component <Rect unrelated:string />` with no
    diagnostics. Earlier, reverting `infer.rs:4474` produced errors for exactly this case.
  - `graphics_tests.rs` builds every image and asserts `main.nx`, `@nx/graphics/graphics.nx` and
    `@nx/math/math.nx`, plus `standard-math-v1` on the math image. It then links and evaluates on
    `nx-ir-runtime` and asserts equality with the HIR result.
  - All pass. The remaining task 1.3 negative cases (missing math image, unsupported feature) are
    correctly left as unchecked planned work.

### ✅ Verified - RF5 `Drawing` inherits `transform`, `clipPath` and `clipRule` from `GraphicsCommon`, which goes beyond the root contract the design specifies.
- **Severity:** Low
- **Evidence:** `graphics.nx:56` declares `Drawing extends GraphicsCommon`. Design §1 defines
  Drawing's contract as "viewBox/fit/contentAlignment/background/opacity/accessibility/children".
  It says "Do not … expose ineffective … props", and §2 says "Set root scale/fit exactly once".
  A root `transform` or `clipPath` either competes with view-box fit or has undefined order relative
  to it. Agents will generate any prop the declaration accepts.
- **Recommendation:** Declare `Drawing` without a base, with `opacity` and `accessibility` only.
  Alternatively, specify where root transform and clip apply relative to view-box mapping and add
  that to the design and spec. Decide before 1.4 and 2.3 build on it.
- **Verification:** `Drawing` (`graphics.nx:59-67`) has no base and declares exactly `opacity`,
  `accessibility`, `viewBox`, `fit`, `contentAlignment`, `background` and `children`, matching
  design §1. The IR and HIR tests confirm the root still evaluates with `opacity=1` and
  `fit=contain`.

### ✅ Verified - RF6 The shipped library header and doc comments don't meet the stated instability and documentation conventions, and leave key props undocumented.
- **Severity:** Low
- **Evidence:**
  - The header is `// Unstable graphics-core profile. No UI dependency. Unsupported effects are not
    declared.` (`graphics.nx:1`). The math and visualizer libraries state "may change incompatibly
    in any NX release" and link a reference page, and design §1 requires the source header to
    identify compatibility status.
  - Hover docs still cite proposal section numbers and DrawnUI notes (`§6.3`, `§6.4` … `§6.10` at
    `graphics.nx:64,71,81,88,96,104,109,114`). Those point to a `docs/` file that is still being
    edited.
  - `clipPath?: string` (`graphics.nx:43`) does not say whether it is SVG path data or a reference.
  - `Text.fill` has no documented default (`graphics.nx:131`).
  - The comment `// An omitted ry uses rx during scene preparation.` (`graphics.nx:70`) duplicates
    the `ry` doc.
- **Recommendation:** Adopt the math/visualizer header form, with the profile name and an
  instability statement, and leave the reference link for task 6.3. Replace the `§` references with
  self-contained descriptions. Document `clipPath` as path data in local coordinates, using
  `clipRule`, and the Text fill default. Drop the duplicate comment.
- **Verification:** The header (`graphics.nx:1-2`) names the graphics-core profile and uses the
  math/visualizer instability wording. All `§` references and DrawnUI notes are gone. `clipPath` is
  documented as SVG path data in local coordinates using `clipRule` (`graphics.nx:45`), and the
  `Text.fill` default as solid black (`graphics.nx:134`). The duplicate `ry` comment is removed. The
  library artifact still has no diagnostics. One cosmetic leftover is not worth reopening for:
  Group's doc comment now begins in lowercase ("a transform/clip/opacity scope", `graphics.nx:69`).

## New Findings Discovered During 2026-10-28 12:57 Verification

### ✅ Resolved - RF7 HIR runtime type checks reject imported records and union cases at sites the consumer types through an import alias (`g.ViewBox`, `g.Paint`, `g.Transform`).
- **Severity:** Medium
- **Evidence:** Interpreter-built values carry the declaration's bare name (`ViewBox`, `Paint.none`,
  and since RF3 `Transform.translate`). An expected type written in the consumer module keeps the
  alias spelling, because `resolve_runtime_named_type` builds `Type::named(name)` /
  `Type::union_type(name, …)` from the written name (`interpreter.rs:4243-4261`).
  `record_value_matches_expected_type` (`interpreter.rs:4735`) and
  `union_case_value_matches_expected_type` (`interpreter.rs:4771`) then resolve the *actual* bare
  name in the consumer module, where it does not exist or names a local decoy. Reproduced in a
  scratch copy (every case passes on `nx-ir-runtime`):
  - `let id(v: g.ViewBox): g.ViewBox = { v }` with `id(<g.ViewBox x=1 y=2 width=3 height=4/>)` →
    `expected g.ViewBox, got ViewBox`, with or without a local `type ViewBox` decoy
  - `type Holder = { v: g.ViewBox }` with `<Holder v=<g.ViewBox …/> />` → the same mismatch
  - `let id(p: g.Paint): g.Paint = { p }` with `id(g.Paint.none)` →
    `expected g.Paint, got Paint.none` (this predates the change)
  - the three payload-case sites listed under RF3

  The checker accepts all of these. Consumers will hit this as soon as they write reusable helpers,
  styles or wrapper components over graphics values (task 4.5's "shared reusable
  dimensions/styles"). It also affects `nxlang run` and the SDK bindings.
- **Recommendation:**
  - In both matchers, canonicalize the *expected* side before comparing: when an expected
    `Named`/`Union` name resolves through `resolve_item(module, name)` to a `Record`, `TypeAlias` or
    `Union` declared in another module, compare against that declaration's own name in its
    declaring module. That means `record_type_satisfies_expected(target_module, actual,
    &record.name)` and, for unions, matching `actual_union_name == union_def.name` with the case in
    `union_def.cases`. Keep `Element`, `Seq` and the existing same-module paths unchanged.
  - Add HIR regression tests for aliased record, constant-case and payload-case values passed to a
    consumer function parameter, a local component prop and a local record field, each typed
    `g.…`, with a same-name local decoy. Extend `graphics_tests.rs` with one such program so the
    IR/HIR equality check covers it.

## Questions
- Should RF1 and RF3 be fixed in `nx-interpreter`, or should graphics tests and the preview host run
  on the IR path only until `retire-hir-interpreter` lands? The design says preview evaluates
  through nx-api, which today means the HIR interpreter. I recommend fixing them: both are small,
  reproduced, and on public `nxlang run` and SDK paths.
- `check_element_bindings_against_component` still keys the type-parameter owner and argument
  resolution by `component.name` (`infer.rs:4489-4491`), while `component_origins` is keyed by the
  visible binding name. An aliased or shadowed *generic* imported component may then get the wrong
  or no owner. Graphics has no generic components, so this is unreproduced here. Should it get the
  same `element.tag` treatment and a test?
- Planned-work risk for tasks 4.x and the Rust/TypeScript helper scenario: record constructor calls
  (`P(1.0, 2.0)`, `g.Point(1.0, 2.0)`) evaluate on the HIR interpreter but fail on `nx-ir-runtime`
  with `nx-ir-reference: Declaration 'P' cannot be used as a value`, even for local records.
  Helpers should construct `Point` values with element syntax, or that existing IR gap needs its own
  fix.

## Summary
- Registration matches the delivered standard-library pattern. `@nx/graphics` is unstable, has no
  runtime functions, imports `@nx/math`, and needs no UI. The library artifact has no diagnostics,
  and the emit includes the transitive math image. The `infer.rs` fix is correct and needed.
- The concrete fixes needed now are RF1 (`Element` acceptance in the HIR interpreter) and RF3
  (canonical union-case name). RF2's in-tree fix needs tightening and a test, and RF4 needs real
  assertions plus an IR test. Together these make the existing regression test pass on both Rust
  evaluators.
- RF5 and RF6 are contract and documentation tidy-ups, cheapest to fix before 1.2-2.x build on the
  declarations.
- Everything else is planned work, correctly left unchecked in tasks.md: the Text unsupported-feature
  rules, identity-aware preparation and spoof rejection, the Skia adapter, helpers, preview,
  corpus, docs and release notes.
- Verification (2026-10-28 12:57): RF1, RF2, RF4, RF5 and RF6 are verified. RF3 is reopened because
  canonical payload-case names now fail at consumer-module sites typed through the alias. RF7 is the
  shared matcher fix that RF3 also needs. The focused graphics and element tests pass,
  `cargo test -p nx-codegen` passes, and the supplied nx-types/nx-api/nx-interpreter log shows
  1,823 passed and 0 failed. Tasks 1.2 onward remain planned and unchecked.

## Fix pass
- **Fix RF1:** Accept Element item contracts in legacy record coercion; added local component and intrinsic child regression coverage.
- **Fix RF2:** Resolve aliased record shapes in the target module, remove unused binding, add default/decoy regressions and update stale comment. Record invocation checking also uses the visible tag after the decoy test reproduced the same static lookup issue.
- **Fix RF3:** Payload case construction stores the canonical union/case name; regression covers aliased transform, paint and stroke.
- **Fix RF4:** Add normalized default/opacity/type assertions, component decoy diagnostics and compiled IR transitive-image/capability plus differential evaluation test. Remaining task 1.3 negative linking cases are unchecked planned work.
- **Fix RF5:** Drawing now declares only root properties.
- **Fix RF6:** Clarify instability, self-contained shape descriptions, clip semantics and text fill default.

## Remaining design blocker

RF3/RF7 remain open. The proposed expected-side canonicalization compares actual values by bare
name in the expected declaration's module, while legacy Value::Record carries no declaration
origin. A consumer-local same-name value therefore needs explicit negative coverage before that
approach can be treated as a high-confidence identity fix. This change forbids generic value/IR
encoding redesign. Resolve the legacy identity strategy or explicitly select compiled IR for the
new graphics host, with the public legacy-path limitation recorded, before extending this patch.
No fix or verification is claimed for these two findings.

RF3/RF7: resolved by explicit user scope decision: no further legacy interpreter graphics updates; IR runtimes are the acceptance targets.

## New Findings Discovered During 2026-10-28 14:55 Review

### Scope of this pass
**Reviewer session:** `d57bacfc-93eb-452a-9f21-b9e12a19f47d` (Claude Code; the same session can verify fixes)\
**Reviewed artifacts:** proposal.md, design.md, math-readiness.md, tasks.md (all 31 checked),
implementation-notes.md, release-notes.md, the three delta specs\
**Reviewed code:** `crates/nx-graphics` (all of it), `crates/nx-graphics-skia` (lib and
`bin/preview.rs`), `crates/nx-api/src/std/graphics/graphics.nx`, `crates/nx-codegen/src/graphics_tests.rs`,
`runtime/typescript/test/graphics.test.mjs`, `scripts/check-graphics-preview.py`,
`scripts/verify-graphics-fixtures.mjs`, `schemas/graphics-preview.schema.json`, `evaluations/graphics-v1`,
`fixtures/svg-v1` and `fixtures/revisions`, `.github/workflows/graphics-native.yml`, the graphics
reference page and the registry/test diffs in nx-api. The earlier `docs/` proposal edits are out of scope.

This is a full-implementation pass under the IR-only runtime scope. The HIR interpreter is out of
scope, and RF3/RF7 are not reopened. The tree changed during the review:
`nx-graphics/src/lib.rs` and `nx-graphics-skia/src/lib.rs` were modified at 14:48. Text run handles
are now excluded from the revision (`#[serde(skip)]`), and a run drawn through another `Fonts`
instance is rejected. The findings below describe the tree after 14:48. All probes were re-run
against it.

**Validation re-run (current tree):** `cargo test -p nx-graphics -p nx-graphics-skia --features
nx-graphics-skia/native` (6 + 5 passed), `cargo test -p nx-codegen graphics_` (2 passed), preview
build, `python3 scripts/check-graphics-preview.py`, `node runtime/typescript/test/graphics.test.mjs`
and `node scripts/verify-graphics-fixtures.mjs` all pass. `cargo fmt --check` and `cargo clippy -D
warnings` are clean, including `nx-graphics-skia --features preview`. The supplied logs
(`/tmp/nx-graphics-{suite,ts-suite,native-tests}.log`) show only passes.

**Probes run in scratch crates outside the repository, with no repository edits.** These areas
were checked and are correct, so they produce no findings:
- **SVG paths:** S/T reflection (including after non-curve commands), relative commands after `Z`,
  an implicit subpath after `Z`, zero-radius arcs (drawn as lines), equal-endpoint arcs
  (omitted), radius scale-up, negative radii, the flag grammar, and the moveto-first rule.
- **Transforms and layout:** SVG transform order, and contain/cover/fill fit with start/center/end
  alignment. The viewport clip maps exactly to `(0,0,W,H)`.
- **Clips and opacity:** clips are applied in transformed local space, and group and child visible
  bounds are correct.
- **Identity:** spoofing was rejected in every probe. A local `Rect` decoy is rejected. The `@nx/`
  workspace identities are reserved by the compiler, so `./@nx/graphics/graphics.nx` cannot
  collide. Values returned through `let` constants, functions and `for` loops all resolve to the
  correct declarations.
- **Cross-runtime output:** Rust and TypeScript IR produce byte-identical helper path strings,
  exponent formatting included. Polygon points differ only in the last ulp. A 4096-gon gives
  4096 points on both runtimes.
- **Revisions and publication:** revisions are deterministic, including with a reused `Fonts`.
  `current.json` stays unchanged after a failure.

### ✅ Verified - RF8 Text is shaped with hinted, whole-unit glyph advances, so small text under view-box or transform magnification renders visibly mis-spaced and measures wrong.
- **Severity:** High
- **Evidence:**
  - `Fonts::shape` builds `Font::from_typeface(face, size)` with Skia's defaults
    (`crates/nx-graphics-skia/src/lib.rs:142`): normal hinting, no subpixel positioning and no
    linear metrics. Each glyph advance is therefore rounded to a whole *logical* unit, before the
    view-box fit or device scale is applied.
  - Measured "Hello" advances against DejaVu's typographic width (≈2.535 em):

    | Font size | Measured | Typographic | Error |
    | --- | --- | --- | --- |
    | 2 | 6 | ≈5.07 | +18% |
    | 4 | 9 | ≈10.14 | −11% |
    | 20 | 51 | — | — |
    | 200 | 507 | — | — |

  - Rendered probe: a `viewBox` of 0 0 40 10 previewed at 800×200, containing
    `<g.Text fontSize=2.5>Illuminating minimum</g.Text>`. The run measures 35 units. The same
    string at `fontSize=25` with `scale x=0.1` measures 27.4. In the PNG, the size-2.5 line is
    spread out letter by letter, while the transformed line is spaced correctly.
  - Small view boxes (24×24 icons, unit diagrams) are a primary use of this profile.
  - Measurement and painting still agree, because they use the same blob. That is why the tests
    pass. `supplied_font_shaping_and_coverage` only asserts `advance > 0`
    (`lib.rs:467-469`), and the preview script compares the measured advance against itself
    (`scripts/check-graphics-preview.py:35-36`).
- **Recommendation:**
  - Make shaping resolution-independent: `set_subpixel(true)`, `set_linear_metrics(true)`,
    `set_hinting(FontHinting::None)` and `set_baseline_snap(false)`. Record those settings in the
    renderer configuration, so they take part in the revision.
  - Add a test that advances scale linearly. For example, `advance(2) * 10` should be within a
    small tolerance of `advance(20)`, and both should match the font's unscaled advances.
  - Add a pixel or ink check of a small-size label under view-box magnification.
- **Fix:** Use subpixel positioning, unhinted linear metrics and no baseline snapping; settings participate in the renderer identity. Added typographic advance scaling and magnified ink tests.
- **Verification:**
  - `Fonts::shape` now sets subpixel positioning, linear metrics, no hinting and no baseline snap
    (`crates/nx-graphics-skia/src/lib.rs:181-185`). The preview renderer name records those
    settings, so they take part in the revision.
  - Re-probed "Hello": the advance is exactly 2.5347 em at sizes 2, 4, 20 and 200 (5.069, 10.139,
    50.693, 506.934). Before the fix it was 6, 9, 51 and 507.
  - Re-rendered the magnified probe (view box 40×10 at 800×200): the size-2.5 label and the
    size-25 label scaled by 0.1 both measure 27.46/274.6. Their painted bounds are identical
    (x 24.857..565.045), and the two lines look the same in the PNG.
  - `unhinted_advances_scale_and_ink_uses_glyph_bearings` asserts linear scaling and the
    typographic width. `small_text_ink_scales_through_the_viewbox_and_local_transform` checks
    magnified bounds; it checks bounds, not pixels, which is enough given the visual probe.

### ✅ Verified - RF9 Text "ink bounds" are font-bounding-box estimates, about 1 em too wide, and they feed every Text node's geometry, painted and visible bounds.
- **Severity:** Medium
- **Evidence:**
  - Ink comes from `TextBlob::bounds()` (`crates/nx-graphics-skia/src/lib.rs:180-189`), which
    unions each glyph position with the font's whole bounding box.
  - With the fixture font at 20 px, every string reports `x0 = -20.41` and `y0 = -24.65`, which
    is DejaVu's `xMin`/`yMax`. Examples: "Hello" (advance 51) reports ink `-20.41..74.87`; "fi"
    reports `-20.41..35.87`. Glyph ink actually starts near `x ≈ +1.6`.
  - The `revisions/7` manifest therefore places the label's bounds at x0 = 56.0 for text drawn
    from x ≈ 74.
  - The value is flagged `conservative`, but the spec asks for ink bounds "sufficient for
    deliberate placement", and agents use these bounds for overlap and alignment.
  - Task 3.5 ("verify advance/ink/baseline metrics") only checks `ink.rect.is_some()`.
- **Recommendation:**
  - Compute ink from the shaped glyphs: `Font::get_bounds` for the run's glyph IDs, offset by
    their positions and unioned. Keep a small antialiasing pad, and mark the result exact or
    conservative accordingly.
  - Assert tight values for a known glyph run. For example, the left ink edge of "H" should sit
    close to its left side bearing.
- **Fix:** Union actual shaped glyph outline bounds at their positioned origins, with glyph-bound fallback and a small conservative pad. Added a known H side-bearing assertion.
- **Verification:**
  - Ink is now the union of each glyph's tight outline bounds at its shaped position, falling
    back to glyph bounds. A 0.001×size pad is added and the result stays flagged conservative
    (`lib.rs:119-140,223-228`).
  - Re-probed at 20 px: "Hello" now reports ink 1.94..49.62 / −15.22..0.30 (previously
    −20.41..74.87). The combining accent raises the top to −16.02, and the Arabic run gives sane
    bounds.
  - In the magnified PNG, the painted right edge (565.0) matches the glyph ink.
  - The "H" left side bearing assertion (between 1 and 3 at 20 px) passes.

### ✅ Verified - RF10 Invalid helper input is reported as "malformed SVG path: unexpected data at position 1" from inside library source; the helper's reason and the author's call are lost.
- **Severity:** Medium
- **Evidence:**
  - Every helper's invalid branch returns `<Path data="invalid … "/>`
    (`graphics.nx:151,161,171,196,208`).
  - Preview probe: `<g.Arrow x1=0 y1=0 x2=10 y2=0 headLength=11/>` returns
    `{"code":"graphics-profile","element":"/children/1","message":"malformed SVG path:
    unexpected data at position 1","provenance":{"module":"@nx/graphics/graphics.nx",…}}`.
    The words Arrow and "head longer than distance" never appear. The provenance points at the
    library's `<Path>`, not the author's `<g.Arrow>`.
  - Coincident endpoints and a head that is too long share one message, and the fill/stroke
    props are dropped.
  - On TypeScript IR, or in any host other than this preparer, the invalid helper evaluates to
    an ordinary Path whose `data` is prose.
  - The reference page says "Invalid helper dimensions are diagnosed during scene preparation"
    (`graphics.md:58`). That is true, but the diagnostic does not say what is wrong, which
    defeats the agent revision loop the change targets.
  - There is no test for the head-too-long or invalid-polygon/star/arc/ring cases through the
    preview path.
- **Recommendation:**
  - Use one reserved sentinel prefix with a precise reason for each failed condition, for example
    `data="nx-graphics-invalid: Arrow headLength 11 exceeds endpoint distance 10"`.
  - Have `svg()` recognize that prefix and emit a dedicated code such as
    `graphics-helper-invalid`, with the reason as the message.
  - Document that the element path is the locator: origins only carry the constructing element,
    so provenance stays in library source.
  - Extend `check-graphics-preview.py` with one case per helper, asserting the code and reason.
- **Fix:** Reserve nx-graphics-invalid: Path data with detailed helper-specific reasons and dedicated graphics-helper-invalid diagnostics. Added preview cases for every helper and coincident/head-too-long Arrow inputs. Documented constructing-source provenance.
- **Verification:**
  - Each helper's invalid branch now emits `nx-graphics-invalid: <reason>`, and `svg()` maps that
    to `graphics-helper-invalid` (`crates/nx-graphics/src/lib.rs:324-328`).
  - Re-probed `headLength=11` on a length-10 Arrow: the result is
    `graphics-helper-invalid: Arrow headLength 11 exceeds endpoint distance 10`.
  - Rust and TypeScript IR emit byte-identical sentinels.
  - `check-graphics-preview.py` covers all five helpers plus coincident and too-long Arrow
    inputs, and passes.
  - Provenance still points at the library's constructing element. The reference page now
    documents this.
  - Remaining non-finite-input gaps are tracked separately as RF17. Any Path or clip string that
    starts with the sentinel prefix also reports this code, but it is still a failure, so that is
    harmless.

### ✅ Verified - RF11 Visible bounds are always flagged conservative, so exact and conservative visible bounds are never distinguished.
- **Severity:** Low
- **Evidence:**
  - `Bounds::intersect` sets `conservative: true` unconditionally (`crates/nx-graphics/src/lib.rs:104-114`).
    Every node then goes through it for the viewport intersection (`lib.rs:919`).
  - Probe: a plain `Rect` at 10..30, inside the viewport with no clip, has exact geometry and
    painted bounds but conservative visible bounds. Every visible bound in
    `fixtures/revisions/*/manifest.json` is conservative.
  - The spec says "Exact versus conservative bounds SHALL be distinguished".
  - The opposite case is a documented limitation: a shape lying wholly inside an evenodd clip
    hole reports a non-empty visible rect (40..60).
- **Recommendation:**
  - Keep the input's flag when intersecting with an axis-aligned rectangle (the viewport, or a
    rectangular clip) under an axis-aligned world transform.
  - Mark the result conservative only for non-rectangular clips or rotated/skewed mappings.
  - Add a fixture asserting an exact visible bound.
- **Fix:** Preserve exact bounds for unclipped contents and axis-aligned rectangular coverage/clips; retain conservative flags for nonlinear/rotated clips and propagate them to descendants. Added exact-visible-rectangle test.
- **Verification:** Reopened for one remaining soundness gap. The fix works for the intended
  cases:
  - A plain Rect inside the viewport, and a Rect fully inside a translated group's rectangular
    clip, are now exact.
  - Rotated clips, an evenodd hole and circles cut by the viewport stay conservative.
  - `exact_visible_rectangles_and_unclamped_radius_default` passes.

  The gap: `axis_rect` (`crates/nx-graphics/src/lib.rs:1413-1429`) accepts any closed four-point
  path whose edges are horizontal or vertical. That includes zero-area paths that fold back on
  themselves, such as `M0 0 L20 0 L20 20 L20 0 Z` and `M0 0 L20 0 L0 0 L0 20 Z`. With either
  path as the clip of a filled Rect at 5..15, the probe reports visible `5..15` with
  `conservative: false`. Skia paints nothing through that clip, so this is a false exact claim.
  Before the fix the result was flagged conservative, which was sound.
  - **Fix needed:** `axis_rect` should require a real rectangle: alternating horizontal and
    vertical edges, with a nonzero width and height. Add the folded-path case to the test.
- **Fix:** Follow-up: axis_rect now requires alternating horizontal/vertical sides with strictly nonzero width and height in either orientation. Added both folded paths and a zero-width path; none can claim an exact nonempty visible area.
- **Verification (follow-up):**
  - `axis_rect` (`crates/nx-graphics/src/lib.rs:1413-1429`) now requires opposite sides to match,
    in either winding orientation, with `a.x != b.x` and `a.y != d.y` (or the transposed
    conditions).
  - Re-probed a filled Rect at 5..15 under eight clips:

    | Clip | Visible bounds |
    | --- | --- |
    | Both folded paths from the report | conservative |
    | Triangle `M0 0 L20 0 L20 20 L0 0 Z` | conservative |
    | Skewed quad `M3 0 …` | conservative |
    | Zero-height `M0 0 L20 0 L20 0 L0 0 Z` | empty |
    | Genuine rectangles (clockwise, counterclockwise, starting from another corner) | exact `5..15` |

  - The earlier cases are unchanged: a plain Rect and a Rect in a translated rectangular group
    clip are still exact; the evenodd hole and rotated clips stay conservative.
  - `folded_and_degenerate_clips_never_claim_exact_nonempty_bounds` covers both folded paths
    plus a zero-width path, and passes.

### ✅ Verified - RF12 An omitted Rect `ry` takes the clamped `rx`, which diverges from SVG and from the reference page's wording.
- **Severity:** Low
- **Evidence:**
  - `lib.rs:710-711` clamps `rx` to `w/2` first, then defaults `ry` to that clamped value.
  - Probe: `width=10 height=100 rx=20` gives `ry = 5`. SVG takes the specified `rx` (20) as the
    automatic `ry` and only then clamps each radius, which gives `ry = 20` (elliptical corners).
  - `graphics.md:32` says "Rect's omitted ry uses rx; corner radii clamp to half the
    corresponding dimension". That reads as the SVG order.
  - The svg-v1 "differences" list does not mention this difference.
- **Recommendation:** Default `ry` from the unclamped `rx`, then clamp both. Add a fixture for
  this case.
- **Fix:** Default ry from specified rx before independently clamping both axes. Added the narrow/tall rx=20 case.
- **Verification:** The specified `rx` now becomes the default `ry` before each radius is clamped
  (`lib.rs:716-719`). Re-probed `width=10 height=100 rx=20`: the first corner ends at (10, 20), so
  `ry = 20`, matching SVG. The test asserts that case, and the reference page wording now matches
  the behavior.

### ✅ Verified - RF13 `Fonts` retains every shaped run forever, so a long-lived host leaks memory on every measure and every prepare.
- **Severity:** Low
- **Evidence:**
  - Each `shape` call appends a `RetainedRun` (blob, cloned request and font identity) to
    `Fonts.runs` (`crates/nx-graphics-skia/src/lib.rs:24,191`). There is no lookup, eviction or
    clear.
  - Re-preparing an unchanged scene allocates new runs. The new test asserts the handles differ.
  - This is harmless in the one-shot CLI. The design positions the adapter for a reusable host,
    such as a DrawnUI canvas repainting after each edit, where the vector grows without bound.
- **Recommendation:** Do one of the following:
  - Cache runs by `(request, font identity)` and reuse the handle.
  - Scope runs to the prepared `Scene`, for example by having the host service return an opaque
    `Arc` handle.
  - Add an explicit generation reset, plus a test that repeated `prepare` does not grow the store.
- **Fix:** Cache shaped requests by request and font content identity, and expose clear_runs after retiring prepared scenes. Repeated preparation reuses runs; tests check bounded repeated-request storage.
- **Verification:**
  - `shape` now caches by the request plus the font's content identity and returns the cached run
    (`lib.rs:176-180,244`). `clear_runs` releases both stores.
  - The new test shapes the same request 100 times with no growth, then clears. My probe
    confirms that preparing the same input twice with one `Fonts` gives the same handle and
    revision.
  - After `clear_runs`, a stale scene either draws an identical run or fails with "different
    text/font service" (request and identity are compared), so it never draws the wrong text.
  - Distinct requests still accumulate until the host calls `clear_runs`. That matches the
    recommended explicit-release option.

### ✅ Verified - RF14 The versioned SVG reference corpus is never rendered or compared, which overstates task 6.1.
- **Severity:** Low
- **Evidence:**
  - `scripts/verify-graphics-fixtures.mjs:10-11` only checks that each SVG file exists and that
    each assertion string is longer than 10 characters.
  - Nothing parses or renders `opacity.svg`, `transforms.svg` or `ring.svg`, or compares them
    with NX output.
  - The analytic transform and opacity checks exist as independent Rust tests. No raster check
    covers the RingSector helper's hole: the TypeScript test counts `" A "` substrings.
  - The `differences` list omits the Rect `ry` difference (RF12). It also lists "stroke width
    zero means no stroke", which is ordinary SVG behavior.
- **Recommendation:**
  - Add a native test that renders the NX equivalent of each corpus entry and asserts the stated
    pixels or coordinates. A good first case is the ring hole at its center, rendered from the
    `RingSector` helper.
  - Correct the `differences` list.
  - Or relabel the corpus as documentation-only and adjust the task 6.1 claim.
- **Fix:** Parse all three SVG references in native tests, prepare/render NX equivalents and assert their corpus samples/coordinates. Preview tests decode actual RingSector PNGs and assert the hole and annulus pixels. Corrected the differences list.
- **Verification:**
  - `corpus_tests.rs` parses the three corpus SVGs with roxmltree and converts them to NX graphics
    values. It prepares and renders them, then asserts each expectation sample: alpha 128 at the
    three opacity points, a painted pixel for the transform, and a transparent hole (50,50) with
    an opaque annulus (80,50) for the ring. It also checks the analytic (1,0)→(12,0).
  - `check-graphics-preview.py` decodes the real preview PNG and asserts white at the RingSector
    hole (70,160) and blue in the annulus (100,160), for both revisions.
  - The `differences` list no longer claims zero-width strokes as a difference. The Rect radius
    difference is moot, because the behavior now matches SVG.
  - Everything passes in the native run.

### ✅ Verified - RF15 The graphics-native CI lane does not run when the compiler or runtime code the preview depends on changes.
- **Severity:** Low
- **Evidence:**
  - `.github/workflows/graphics-native.yml:4-10` triggers only on `crates/nx-graphics*`,
    `crates/nx-api/src/std/graphics`, `examples/graphics` and `Cargo.lock`.
  - Spoof rejection, origin/span identity lookup, math diagnostic codes, publication safety and
    measured placement are tested only by `scripts/check-graphics-preview.py`, which only runs
    in this lane.
  - These behaviors depend on nx-api (`module_by_prepared_identity`, `elements_mut`, imports),
    nx-hir, nx-types, nx-codegen and nx-ir-runtime origins.
  - The lane also skips changes to the script itself, `schemas/`, `evaluations/` and the
    workflow file.
- **Recommendation:** Add those crate, script, schema and workflow paths to the filter, or run
  the lane on every pull request that touches `crates/**`.
- **Fix:** Trigger native CI for crates/** plus graphics scripts, schemas, evaluations, workflow, examples, TypeScript runtime and Cargo metadata.
- **Verification:** The trigger paths now include `crates/**`, `scripts/*graphics*`,
  `schemas/**`, `evaluations/**`, `examples/graphics/**`, `runtime/typescript/**`, the workflow
  file, `Cargo.toml` and `Cargo.lock` (`graphics-native.yml:4-14`). That covers every dependency
  of the preview checks named in the finding.

### ✅ Verified - RF16 Preview host robustness: a partial `--limits` file is rejected, and different failed revisions can share one `requestRevision`.
- **Severity:** Low
- **Evidence:**
  - `Limits` has no serde defaults (`crates/nx-graphics/src/lib.rs:46-54`, read at
    `preview.rs:181`). `{"nodes":5}` fails with `missing field depth`, although the reference page
    says the file "overrides" the limits.
  - The failure ID hashes only the arguments and the entry file bytes (`preview.rs:229-232`).
    Probe: two different errors in an imported `lib.nx` both returned `requestRevision
    643dfd0e…`. Fonts and library content are also excluded.
  - The spec requires a failure response to identify the failed revision.
- **Recommendation:**
  - Add `#[serde(default)]` to `Limits`.
  - Derive `requestRevision` from all workspace source bytes and font identities. Use the program
    fingerprint once compilation succeeds.
  - Test that two failing edits to an imported module produce different IDs.
- **Fix:** Add serde defaults for partial limits. Failed request identities include workspace source bytes, library versions, supplied/default font content, renderer settings and the compiled program fingerprint when available. Added partial-limit and imported-failing-edit tests.
- **Verification:**
  - `Limits` now has `#[serde(default)]`. Re-probed `{"nodes":5}`: it is accepted and enforced
    ("node or depth limit exceeded" at `/children/2/children/1`), and the other limits keep their
    defaults.
  - The failure ID now covers:
    - the arguments
    - the program fingerprint, when compilation got that far
    - the standard-library versions
    - every `.nx` file under the source root (the same tree `NxWorkspace::from_directory` loads)
    - the renderer settings
    - the font bytes
    - the limits file
  - Re-probed two different failing edits to an imported `lib.nx`: they give `9ca1fff4…` and
    `f69de0f8…`, and repeating the same failure reproduces its ID.
  - The script's imported-module case passes.

### Questions (this pass)
- Nested union values are recognized by their `$type` suffix alone (`lib.rs:293` for paints,
  `lib.rs:387` for transforms). The neutral `prepare` accepts `{"$type":"Evil.solid"}` and
  `{"$type":"MyUnion.translate"}`. Through compiler-checked IR this cannot happen, because the
  props are typed, so I have not filed it as a defect. The reference page tells hosts not to trust
  `$type` strings, though. Should nested Paint, Stroke and Transform values also be resolved
  through origins, or should the API state that only node identity is verified?
- Helper nodes report provenance inside `@nx/graphics/graphics.nx`, not at the author's call. Is
  that acceptable as the "available helper provenance" task 4.5 requires, given that origins carry
  only the constructing element?

### Summary (this pass)
- The core pipeline is sound. Path parsing and normalization, transforms, fit/alignment, clips,
  isolated opacity, identity checks, Rust/TypeScript helper parity, deterministic revisions and
  failure-safe publication all hold up under the probes. All suites pass.
- RF8 is the substantive correctness defect: hinted, whole-unit text advances misplace glyphs
  under view-box magnification. RF9 (font-box ink bounds) and RF10 (opaque helper diagnostics)
  undermine the measurement and revision loop agents rely on.
- RF11-RF16 are lower-risk fidelity, resource, test-coverage and CI-coverage gaps.
- The task checkboxes are broadly supported by code and tests. The exceptions are weak
  verification behind 3.5 and 5.3 (RF8, RF9) and the SVG corpus claim in 6.1 (RF14).

## New Findings Discovered During 2026-10-28 15:18 Verification

**Verifier session:** `d57bacfc-93eb-452a-9f21-b9e12a19f47d`, the same session that reported
RF8-RF16. The scope is IR-only and the HIR interpreter was not touched.

**Re-validation:** run against the 15:14 tree. These suites pass:
- `cargo test -p nx-graphics -p nx-graphics-skia --features nx-graphics-skia/native` (7 + 8)
- `cargo test -p nx-codegen graphics_` (2)
- `cargo test -p nx-api graphics` (3)
- `python3 scripts/check-graphics-preview.py`
- `node runtime/typescript/test/graphics.test.mjs`
- `node scripts/verify-graphics-fixtures.mjs`
- `cargo fmt --all --check`
- `cargo clippy -D warnings` on `nx-graphics` and `nx-graphics-skia --features preview`

The supplied fix logs agree. CI's workspace-wide `cargo clippy --workspace --all-targets -D
warnings` fails with 4 lints, but they are in `crates/nx-codegen/src/math_tests.rs:159,168,205`
and `crates/nx-api/src/standard_library_tests.rs:1059`. Both are code committed in `e46b846`, not
code this change touches, so they are not counted here.

**Self-fixes:**
- The `prepare_value` raw-value preflight reports `graphics-non-finite` with the
  authored-element provenance. For example, a Rect `x` computed as `1e309 - 1e309` reports at
  `/children/0/x`.
- Large-angle remainders match on both IR runtimes. A rotation of 1e20 reduces to 280°, and
  start angles of −1e17 and 7.5e15 also agree. Rust and TypeScript output is byte-identical for
  path strings and within 1.8e-15 for polygon points.
- Transient handles stay out of revisions, and a run requested from the wrong service is
  rejected.

### ✅ Verified - RF17 Non-finite helper inputs still produce misattributed or opaque diagnostics: NaN radii are blamed on `sweepAngle`, and infinite Arc/RingSector inputs become "malformed SVG path".
- **Severity:** Low
- **Evidence:** NX produces NaN from `1.0e309 - 1.0e309` (`0.0 / 0.0` is a division-by-zero
  error instead). Preview probes:

  | Input | Diagnostic |
  | --- | --- |
  | `<g.Arc cx=0 cy=0 radius={nan()}/>` | `graphics-helper-invalid: Arc sweepAngle must be in -360..360`; the sweep is the default 90 |
  | `<g.RingSector cx=0 cy=0 outerRadius={nan()}/>` | the same `sweepAngle` message |
  | Star with NaN `outerRadius` | `Star innerRadius exceeds outerRadius` |
  | `<g.Arc cx=1.0e309 cy=0 radius=1/>` | `graphics-profile: malformed SVG path: invalid number at position 3`, with provenance in `@nx/graphics/graphics.nx` |
  | `<g.RingSector cx=0 cy=0 outerRadius=1.0e309/>` | the same malformed-path diagnostic |

  - The helpers' else branches choose a reason through negated comparisons (`radius < 0`,
    `outerRadius < innerRadius`). Those are false for NaN, so control falls through to the last
    message (`graphics.nx` Arc and RingSector branches).
  - Infinite coordinates in Arc and RingSector exist only inside the formatted path string
    (`"Infinity"`). Neither the helper checks nor the new `prepare_value` non-finite preflight
    can see them.
  - The polygon helpers are not affected: RegularPolygon with `radius=1.0e309` correctly
    reports `graphics-non-finite`.
  - Spec context: graphics-library says helper "finite … checks SHALL remain independent of the
    math library's numerical domain checks". RF10's fix covers finite invalid ranges only.
- **Recommendation:**
  - Check finiteness first in each helper, for the scalars that reach geometry (center, radii,
    widths, angles), for example with `x - x == 0.0`, which is false for NaN and ±∞.
  - Emit `nx-graphics-invalid: <Helper> <param> must be finite`.
  - Add preview cases for a NaN Arc radius and an infinite RingSector radius or center, asserting
    the code and the parameter name.
- **Fix:** Check each helper geometry scalar for finiteness before range checks, math calls or path formatting, returning the dedicated helper/parameter reason. Arc/RingSector also check derived center +/- radius coordinates. Added NaN and infinity cases in preview and both Rust/TypeScript IR fixtures; raw primitive non-finite values remain covered by prepare_value.
- **Verification:**
  - Every exported helper now checks each geometry scalar with `finite(x) = (x - x == 0.0)`
    before range checks, math calls or formatting (`graphics.nx:144-146` and each helper's guard
    block). Arc and RingSector also check center ± radius.
  - Preview re-probes of the reported cases now give the precise reason:
    - NaN Arc radius: `graphics-helper-invalid: Arc radius must be finite`
    - NaN RingSector outer radius: `RingSector outerRadius must be finite`
    - NaN Star outer radius: `Star outerRadius must be finite`
    - `RingSector outerRadius=1.0e309`: `RingSector outerRadius must be finite`
    - `Arc cx=1.0e309`: `Arc cx must be finite`
  - New cases:
    - `Arc cx=1.7e308 radius=1.7e308` and a RingSector with `cy=-1.7e308`:
      `… center +/- radius coordinates must be finite`
    - NaN Arc `startAngle`, infinite Star `rotation` and NaN Arrow `headLength`: each names its
      parameter.
  - Derived overflow in the other helpers still fails explicitly, with no fabricated geometry:
    - `Arrow x1=-1.7e308 x2=1.7e308` keeps math's `nx-math-non-finite-input` (hypot), as the spec
      requires.
    - `RegularPolygon cx=1.7e308 radius=1.7e308` reports `graphics-non-finite` at
      `/children/0/points/0/x`.
  - Valid helpers still render: a −360° Arc and a seven-tip Star.
  - Rust and TypeScript IR emit byte-identical sentinels for NaN Arc, infinite RingSector `cx`,
    center overflow and NaN Arrow `y1`. All earlier helper outputs stay byte-identical or within
    1.8e-15.
  - `graphics_tests.rs`, `graphics.test.mjs` and `check-graphics-preview.py` cover these cases,
    and all pass.

### Summary (2026-10-28 15:18 verification)
- **Verified (8):** RF8, RF9, RF10, RF12, RF13, RF14, RF15, RF16. Text advances are now linear and
  unhinted, ink bounds are tight, helper diagnostics are specific, and Rect corner radii match
  SVG. Runs are cached with an explicit release. The SVG corpus and the RingSector PNG pixels are
  rendered and asserted. The CI trigger is expanded, partial limits work, and failure IDs are
  distinct.
- **Reopened (1):** RF11. The new rectangle detection treats folded, zero-area four-point clip
  paths as rectangles and reports an exact non-empty visible bound where nothing is painted.
- **New (1):** RF17, non-finite helper inputs (Low).

## Verification Update 2026-10-28 16:26

The same reviewer session (`d57bacfc-93eb-452a-9f21-b9e12a19f47d`) ran this follow-up. The HIR
interpreter was not touched.

**Validation:** re-run against the 16:23 tree. These pass:
- `cargo test -p nx-graphics -p nx-graphics-skia --features nx-graphics-skia/native` (8 + 8)
- `cargo test -p nx-codegen graphics_` (2)
- `cargo test -p nx-api graphics` (3)
- `python3 scripts/check-graphics-preview.py`
- `node runtime/typescript/test/graphics.test.mjs`
- `node scripts/verify-graphics-fixtures.mjs`
- `cargo fmt --all --check`
- `cargo clippy -D warnings` on `nx-graphics` and `nx-graphics-skia --features preview`

The `/tmp/nx-graphics-followup-*.log` logs agree.

- **Verified (2):** RF11 and RF17.
- **Reopened:** none. All earlier verified findings remain verified.
- **New findings:** none proven.

All findings RF1-RF17 are now Verified or Resolved. The workspace-wide clippy failures noted at
15:18 are in code committed before this change and remain out of scope.


## Post-verification validation amendment

The workspace-wide Clippy failures mentioned in the verifier's historical notes were four
pre-existing test-only lints. They have subsequently been corrected in standard_library_tests.rs
and math_tests.rs with no semantic changes. `cargo clippy --workspace --all-targets -- -D warnings`
and `cargo test --workspace --locked` now both pass. No further HIR interpreter changes were made.

## Playground review attempt (2026-10-28)

The Canvas 2D playground extension has not yet received an independent review. The requested
Claude CLI `/openspecex-review` invocation returned its weekly-limit error before reading/reviewing
the change (`resets 6pm (America/New_York)`, session
`c3d78d75-ab67-4d69-ad6c-d2b95c82d0a3`). No new findings or verified fixes are recorded by that
attempt. RF1–RF17 above apply to the earlier native implementation and are preserved unchanged.
Resume the independent review of the playground/SDK extension when Claude is available, then
use `/openspecex-review-fix` and the same reviewer's `/openspecex-review-verify` as requested.

## New Findings Discovered During 2026-10-28 18:16 Review

### Scope of this pass

This is the independent review of the playground/WASM SDK extension (tasks 7.1-7.7). It is a
fresh Claude Code session (`c1fb5c65-7d20-45f8-a425-8eeb7188a9d8`); RF1-RF17 above are unchanged.
The implementation was still being finalized while this pass ran (`App.tsx` and
`preview/workbench.ts` changed during the review), so the references are to the 18:16 tree.

**Reviewed artifacts:** proposal.md (including the playground and shared-selection extensions),
design.md §7-8, tasks.md 7.1-7.7, the graphics-preview spec (playground and selection
requirements), the graphics-library and graphics-rendering specs, and implementation-notes.md.
**Reviewed code:** `bindings/wasm/native/src/graphics.rs`, `lib.rs`/`Cargo.toml` and the ABI 11
`abi.ts`/`host.ts` changes; `sites/playground/src/graphics/*`; `preview/selection.ts` and its
test; `preview/PreviewView.tsx`, `workbench.ts`, `protocol.ts` and the workbench test; `App.tsx`,
`OutputPane.tsx`, `worker/session.ts` and `worker/protocol.ts`; the example metadata and
`graphics.nx`; `scripts/check-graphics-browser.mjs` and `check-examples.mjs`; the
`runtime/typescript` `includeCallSites` change and README; `packages/value-view` `selectNodes` and
its styles; `deploy-playground.yml`; the playground README and graphics reference page. Read for
context: `crates/nx-graphics/src/lib.rs` (layer, text and Drawing preparation) and the native
`preview.rs` resolver.

**Commands run (read-only; the probes were temporary test files, deleted after each run):**
- `node --test src/graphics/graphics.test.mjs src/preview/selection.test.mjs src/preview/workbench.test.mjs`
  (in `sites/playground`): 34 passed.
- Probes calling `prepareDrawing` with the WASM host and a stub measurement, used for the
  reproductions below. They also confirmed that spoofed `Rect`/`Text` are rejected by identity,
  aliases resolve, call-site provenance is recorded for repeated helper calls, and 1,900 helper
  nodes prepare in about 0.6 s.

Verified correct, so no finding is opened for these:
- Stale-response handling: `reduceDrawing` checks both the generation and the source, and
  `DrawingView` hides the scene while a request is pending or the source has moved on.
- The paint lifecycle: the offscreen frame is published only after a complete paint, and layers
  and frames are released on every path.
- Isolated opacity: the subtree is painted to its own layer and composited under the
  parent/local clip.
- Clip and fill rules are applied independently.
- The root viewport clip is in view-box-local coordinates.
- The bridge resolves identity from the construction site, not from call sites, so call-site
  ancestry is never used to resolve a declaration.

### ✅ Verified - RF18 In the browser, helper validation errors and preparation errors on helper output have no source location, although the authored call site is in their provenance.
- **Severity:** Medium
- **Evidence:** `prepare.ts:39` builds a source span only when `provenance.module` is
  `playground.nx`. `prepare.ts:49` passes only `provenance.module/start/end`, which for helper
  output is the `<Polygon>`/`<Path>` construction inside `@nx/graphics/graphics.nx`. The
  `provenance.callSites` that `graphics.rs:73` forwards are ignored. Reproduced against the WASM
  host: each of these yields `origin: "program", span: null`, so the editor shows no marker and
  the Diagnostics entry cannot locate the authored call:
  - `<g.Star … innerRadius=50 outerRadius=10/>` → "Star innerRadius exceeds outerRadius (/children/0)"
  - `<g.RegularPolygon … count=2/>` → "RegularPolygon count must be in 3..4096; received 2"
  - `<g.Arrow x1=5 y1=5 x2=5 y2=5/>` → "Arrow endpoints coincide"
  - `<g.Star … fill=<g.Paint.solid color="nope"/>/>` → "invalid solid color"

  For comparison, the same invalid color on a directly authored `<g.Rect>` is marked at the
  `<g.Rect …/>` span. RF10 fixed this attribution for native helper diagnostics. The new
  call-site provenance carries the location, but the browser does not use it.
- **Recommendation:** Map a preparation diagnostic's span through
  `authoredSites(d.provenance, FILE_NAME)[0]`, the same rule `selection.ts` uses for navigation.
  Add a `graphics.test.mjs` case asserting that an invalid Star/RegularPolygon/Arrow diagnostic
  has `origin: "source"` and a span covering the authored `<g.Star …/>` element.
- **Fix:** Preparation diagnostics now use authoredSites on provenance, matching navigation. Four helper regression cases assert exact authored spans, including invalid paint.
- **Verification:** Verified on the final WASM build. `prepare.ts:118-126` maps every preparation diagnostic through `authoredSites(d.provenance, FILE_NAME)[0]`. Re-running the four original reproductions (Star innerRadius > outerRadius, RegularPolygon `count=2`, coincident Arrow, Star with an invalid paint) now gives `origin: "source"`, with spans starting at the authored `<g.Star`/`<g.RegularPolygon`/`<g.Arrow` element.

### ✅ Verified - RF19 The browser rejects any drawing with more than 54 translucent elements, because every node with opacity < 1 is charged a full 640×480 layer cumulatively.
- **Severity:** Medium
- **Evidence:** `prepare.ts:48` sets `layer_pixels: 16_777_216` for a 640×480 viewport.
  `nx-graphics/src/lib.rs:635-640` adds `width*height*scale²` for every node with
  `opacity < 1.0`, including `opacity=0` nodes, which `canvas.ts:46` never paints. The total is
  cumulative across the whole scene. The Canvas adapter, however, allocates one layer per
  translucent node and frees it right away (`canvas.ts:59-72`), so at most nesting-depth layers
  exist at once. Reproduced: `{for i in 0..55 { <g.Circle cx={i * 1.0} cy=50 r=3 opacity=0.5/> }}`
  fails with "layer area limit exceeded (/children/54)", while 54 circles render. Translucent
  dots, bars and overlays are common in the infographic/illustration tasks the evaluation corpus
  targets, so the playground rejects ordinary content. The limit is not documented in the
  reference page or the README.
- **Recommendation:** Bound what the destination actually holds: the peak concurrent layer area
  along the current nesting stack, and/or a separate cap on layer count. Do not charge
  `opacity == 0` subtrees, since they are skipped. Otherwise size the limit to a realistic count
  and document it. Add a neutral test showing that many sibling translucent nodes pass, while
  deep nesting or excessive concurrent area still fails.
- **Fix:** The neutral preparer charges peak concurrent physical layer area and skips layer charges under zero-opacity subtrees. Direct drawing skips those subtrees too. Tests accept 1,000 siblings, reject nested concurrent area, and accept hidden nesting; browser regression accepts 100 translucent circles.
- **Verification:** Verified. `nx-graphics/src/lib.rs:659-668` charges a layer only while painting and only for 0 < opacity < 1, and releases it after the subtree (`:985`). `draw_node` skips opacity-0 nodes (`:232`). `layer_budget_tests` passes (1,000 siblings accepted, 4-deep nesting rejected, hidden nesting accepted), and `cargo test -p nx-graphics` passes 9. Probes: 100 translucent circles and three nested translucent groups prepare cleanly. A malformed path inside an `opacity=0` group is still rejected with its authored span, so hidden content is still validated.

### ✅ Verified - RF20 In Source mode, the drawing and structured-Preview highlight does not follow the editor's selection, so it shows the last output click after the cursor moves.
- **Severity:** Low
- **Evidence:** In Source mode, `outputSourceSelection` is `outputSelection` (`App.tsx:353-356`).
  That value is written only by output navigation (`App.tsx:374`) and by the Visual→Source switch
  (`App.tsx:309`). Editor cursor and selection changes never update or clear it. Scenario: in
  Source mode, click the Star on the canvas. The Star is highlighted and the editor selects
  `<g.Star …/>`. Now select the `<g.Arrow …/>` text, or type elsewhere without changing the
  structure. The Star stays highlighted, and `DrawingView` shows it as the selected element,
  although it no longer corresponds to the source selection. The spec only requires Visual-driven
  multi-selection. Still, the user asked for two-way selection that preserves the Visual/Source
  mode, and `check-graphics-browser.mjs` exercises source→output only in Visual.
- **Recommendation:** Do one of the following:
  - Feed Monaco selection changes in Source mode into `outputSelection`, converting UTF-16 to
    UTF-8, so Source mode is also two-way.
  - At minimum, clear `outputSelection` when the editor selection moves away from it.

  Then add a browser step that selects an element's text in Source and checks
  `data-selected-ids`.
- **Fix:** Monaco reports source snapshots and UTF-16 cursor/selection offsets. App converts them to byte spans and drives the shared selection model in Source mode. The production Chromium test moves the cursor from Star to Arrow and checks the rendered selection.
- **Verification:** Verified. `NxEditor` reports `onDidChangeCursorSelection` with the model snapshot. In Source mode, App converts the UTF-16 offsets to UTF-8 bytes and updates `outputSelection` (`App.tsx:700-716`). Snapshots that don't match the current source are ignored, so mid-edit events cannot install a stale selection. The final-tree Chromium run (`pnpm test:graphics-browser`) passed: the cursor moves from the Star into the Arrow line and the test asserts that canvas `data-selected-ids` includes `element/children/2`. It also passed the Visual, loop and Question/Choice steps and component Preview.

### ✅ Verified - RF21 Browser text measurement runs before identity and limit validation, and it picks Text nodes by display name.
- **Severity:** Low
- **Evidence:** `textRequests` (`prepare.ts:23-36`) selects any record whose `$type` ends in
  `Text`, then measures each one (`prepare.ts:47`) before `prepareGraphics` resolves identities or
  applies `limits.nodes`. As a result:
  - A user `external component <Text fontFamily:string text:string/>` with `fontFamily="Other"`
    is reported as "The browser preview supplies Fixture at weight 400…". It should report the
    "unrecognized graphics declaration identity" that the stub-measurement probe produces. That
    is the display-name trust the design rules out.
  - Every distinct Text is measured with a new `OffscreenCanvas` (`browser-font.ts:25`) before
    the 2,000-node limit applies. A loop such as `for i in 0..100000 { <g.Text … text="{i}"/> }`
    is within `PREVIEW_LIMITS.maxRangeLength`, so it can spend the worker's 10 s deadline
    measuring. The user would then see a worker timeout rather than "node or depth limit
    exceeded". This is likely but not run in a browser.
- **Recommendation:** Collect text requests only from nodes whose identity the bridge resolves to
  the graphics `Text`. For example:
  - a two-phase bridge call that returns validated text requests and limit errors before any
    measurement; or
  - at least, cap the number of requests at `limits.nodes` before measuring.

  Also reuse one measurement context per call.
- **Fix:** A bridge preflight validates identities, geometry and all scene limits before returning deduplicated Text requests; it publishes no scene and needs no font service. Actual measurement uses one reusable realm-local context, then full preparation with measured runs. Spoofed Text and oversized Text scenes are tested to make zero measurement calls.
- **Verification:** Verified. The display-name collector is gone. `prepare.ts:128-139` runs a bridge preflight first: `graphics.rs:109-119` validates identities, geometry and limits with a collecting `TextService`, then returns deduplicated `textRequests`. Measurement reuses one realm-local context (`browser-font.ts:84`). Probes: a spoofed `<Text fontFamily="Other">` makes zero measurement calls and reports `unrecognized graphics declaration identity`. A 5,000-Text loop makes zero calls and reports `node or depth limit exceeded`, with a source span. Two identical Texts and one distinct Text make 2 calls.

### ✅ Verified - RF22 The new SDK and runtime surfaces have no tests in their own packages, and the SDK method is untyped.
- **Severity:** Low
- **Evidence:**
  - `NxProgramArtifact.prepareGraphics(request: unknown): unknown` (`host.ts:229-232`) is public
    API of the published `@nx-lang/sdk-wasm`. It has no test under `bindings/wasm`: the 128 SDK
    tests do not exercise it. `graphics.rs` has no Rust test either.
  - Its only coverage is the playground `graphics.test.mjs`, which casts the result
    (`prepare.ts:48`).
  - `includeCallSites` is documented in `runtime/typescript/README.md` with these guarantees:
    default reports unchanged, innermost-first ordering, a non-boolean rejected, operation counts
    and values unchanged. `runtime/typescript/test` has no test for any of them. They are
    exercised only indirectly through playground selection tests.
  - Task 7.1 asks for alias, spoof and error-result tests of the SDK operation itself.
- **Recommendation:** Add SDK tests for `prepareGraphics`: a genuine alias, a local spoof, a
  malformed request, a missing measurement and a disposed artifact. Give it typed
  request/response interfaces, even if marked unstable. Add `runtime/typescript` tests covering
  each `includeCallSites` guarantee.
- **Fix:** Added public unstable typed graphics request/response/config/scene interfaces, SDK tests for alias/spoof/preflight/missing measurements/malformed input/disposal/non-finite input, native bridge collector tests and runtime-owned includeCallSites tests. Default origins, ordering, costs, values and invalid options are covered in the runtime package.
- **Verification:** Verified. Typed `NxGraphics*` DTOs are exported, and `prepareGraphics` rejects non-finite numbers before JSON encoding (`host.ts:716-728`). `bindings/wasm/test/graphics.test.ts` covers alias/spoof, preflight, missing measurement, malformed request, non-finite config and use after disposal. The SDK vitest run passes 130. `graphics.rs` has a collector/parse unit test; the native bridge passes 20. `runtime/typescript/test/graphics.test.mjs` asserts that default origins have no `callSites`, that values and usage are unchanged, innermost-first order, and rejection of a non-boolean option; it passes.

### ✅ Verified - RF23 Declaration-identity resolution is duplicated in the WASM bridge and the native preview, and both clone the whole lowered module for every origin.
- **Severity:** Low
- **Evidence:** `bindings/wasm/native/src/graphics.rs:70-113` repeats
  `nx-graphics-skia/src/bin/preview.rs:125-160` almost line for line. Both:
  - look up the module by prepared identity;
  - run `(*module.lowered_module).clone()` only to call `elements_mut()`, because the HIR has no
    immutable iterator (`nx-hir/src/lib.rs:1607`);
  - match the span exactly, resolve the tag locally or as an import, and build the `Identity`.

  This logic is the spoof/alias security boundary for graphics. Two copies can drift: the WASM
  copy already differs in carrying `callSites`. The per-origin clone also makes resolution
  O(nodes × module size).
- **Recommendation:** Move the lookup into one shared helper. For example, a `nx-api`
  `ProgramArtifact` method that returns the constructing declaration's identity for a
  (module, span) pair. Back it with an immutable element iterator or a span index built once per
  call. Have both hosts call it, and test spoof/alias resolution there once.
- **Fix:** ProgramArtifact.construction_declarations builds one immutable span/declaration index for both hosts. LoweredModule.elements provides immutable access, eliminating module clones. A shared nx-api test distinguishes aliased genuine Rect from a local spoof.
- **Verification:** Verified. `ProgramArtifact::construction_declarations` (`nx-api/src/artifacts.rs:133-172`) builds one immutable (module, span) → declaration index over `LoweredModule::elements()`. Both `graphics.rs:92-108` and `nx-graphics-skia/src/bin/preview.rs:126` use it. No lowered-module clone remains, and the `nx-api` alias/spoof test passes.

### ✅ Verified - RF24 Much of the new playground TypeScript is written as dense one-liners, unlike the surrounding code, and one helper detaches an existing doc comment.
- **Severity:** Low
- **Evidence:** These files pack whole functions onto single lines, with brace-less `if` bodies
  and no spacing:
  - `prepare.ts`, `browser-font.ts`, `cmap.ts`, `model.ts`, `useDrawing.ts`, `protocol.ts`;
  - the added `App.tsx` lines (`useEffect(()=>{if(...)setView(...)})`, the `marked`/`Diagnostics`
    ternaries);
  - `check-examples.mjs:124-131`, `graphics.test.mjs` and `selection.test.mjs`.

  For example, `cmap.ts:12-14` is the whole format 4/12 lookup in three lines, and
  `useDrawing.ts:18` is the whole request/answer/error flow on one line. The repository
  instructions require matching the surrounding style and always using braces. `canvas.ts`,
  `DrawingView.tsx`, `selection.ts` and the rest of the playground follow it. Separately,
  `evaluateGraphicsExample` was inserted between `checkDocs`'s doc comment
  (`check-examples.mjs:120-123`) and `checkDocs` (`:133`), so that comment now documents the wrong
  function. These are the files most likely to need follow-up fixes, and they are the hardest to
  review.
- **Recommendation:** Reformat the new files to the playground's prevailing style: multi-line
  bodies, braces, spacing and short doc comments where the surrounding modules have them. Move
  `evaluateGraphicsExample` above the `checkDocs` doc comment, with its own comment.
- **Fix:** Reformatted the new graphics/selection files and changed App blocks with expanded bodies, braces and consistent spacing. Moved evaluateGraphicsExample ahead of checkDocs documentation and gave it its own comment.
- **Verification:** Verified. The graphics, selection, App and test files now use expanded multi-line bodies with braces and consistent spacing. `evaluateGraphicsExample` (`check-examples.mjs:141`) has its own doc comment, and `checkDocs`'s comment (`:165-170`) is attached to `checkDocs` again.

### Questions (this pass)
- RF20: should Source mode be fully two-way (editor selection drives the output highlight), or is
  output→source enough there? The spec's multi-selection scenario names only Visual.
- Tasks 7.5-7.7 are still unchecked while the selection work is being finalized. Re-run the
  browser integration and SDK suites on the final tree before closing them; this pass did not
  re-run the Chromium check.

### Summary (this pass)
- The Canvas 2D/WASM-bridge design is sound. Identity comes from construction sites resolved
  against the artifact's metadata, stale answers cannot replace the current revision, and
  publication is atomic, with correct isolated opacity, clips and font handling.
- Two Medium issues:
  - RF18: helper errors are not located in the browser even though call sites are available.
  - RF19: the cumulative layer budget caps a playground drawing at 54 translucent elements.
- Low: RF20 (stale Source-mode highlight), RF21 (text measurement runs before identity and
  limits), RF22 (no tests for the SDK/runtime surfaces), RF23 (duplicated resolver with
  per-origin module clones) and RF24 (style).

## New Findings Discovered During 2026-10-28 18:33 Verification

The same reviewer session (`c1fb5c65-7d20-45f8-a425-8eeb7188a9d8`) ran this verification against
the final tree, with `nx.wasm` rebuilt at 18:29.

Re-run here, all passing:
- `cargo test -p nx-graphics` (9).
- `cargo test -p nx-api construction` (1).
- `cargo test -p nx-sdk-wasm-native` (20).
- `pnpm --dir sites/playground test` (142 tests and 25 examples).
- `bindings/wasm` vitest (130).
- `runtime/typescript` `node test/graphics.test.mjs`.
- `pnpm --dir sites/playground test:graphics-browser` (Chromium, including the Source-cursor and
  structured-selection steps).
- Temporary `prepareDrawing` probes for RF18, RF19 and RF21, deleted after the run.

`bindings/wasm/dist/nx-debug-trap.wasm` is still from 17:19. It predates the resolver change, so
rebuild it before relying on any test that loads it.

### ✅ Verified - RF25 A browser text-measurement failure has no element or source location, unlike every other drawing diagnostic.
- **Severity:** Low
- **Evidence:** Preflight returns bare deduplicated `TextRequest`s (`graphics.rs:60-75, 117`),
  with no element path or provenance. When `measureBrowserText` throws (`prepare.ts:136-139`),
  the generic catch (`prepare.ts:183-192`) turns the error into `graphics-browser` with
  `origin: "program", span: null`. Examples are a missing glyph, a non-Fixture family, an
  unsupported weight/style, or an oversized/multiline run. Probe:
  `<g.Rect …/><g.Text fontFamily="Fixture" text="x"/>` with a measurement that throws "Fixture is
  missing glyph U+6F22." produces exactly that span-less diagnostic. The editor cannot mark which
  Text failed, while RF18's preparation errors and native missing-glyph errors (which set
  `e.element`) do identify it. The spec asks that preparation "identify the text/font failure".
- **Recommendation:** Have preflight return, with each deduplicated request, the first element
  path and provenance that requested it, or all of them. When a measurement fails, map the error
  through `authoredSites` to a `source` diagnostic, as for preparation diagnostics, with the
  element in the message. Extend the browser missing-glyph step, or a `graphics.test.mjs` case,
  to assert the span covers the `<g.Text …>` element.

- **Fix:** Preflight now returns each deduplicated request with the first requesting element ID and construction/call provenance. Measurement failures map through authoredSites into source diagnostics. SDK assertions and a duplicate-Text regression verify the exact first Text span and a single measurement call.
- **Verification:** Verified on the final tree (release and debug-trap WASM rebuilt 18:35). Preflight returns `{request, element, provenance}` per deduplicated request, from the first requesting node in scene order (`graphics.rs:77-91, 133-141`). `prepareDrawing` catches a failure at that request and maps it through the shared `authoredSites` diagnostic mapper, re-throwing `NxHostCrashedError` (`prepare.ts:136-157`). Probes with a throwing measurer:
  - The second of two identical `bad` Texts makes one call and reports `origin: "source"` at the first `<g.Text … text="bad"/>`.
  - A Text built by a local function is located at its editable construction, consistent with navigation.
  - A Text under `opacity=0` is still validated and located.
  - A host crash propagates.

  Re-run, all passing: playground 143 tests and 25 examples, SDK vitest 130, native bridge 20. `/tmp/nx-review-rf25-browser.log` (18:37) shows the Chromium check passing on the rebuilt assets. Measurement errors name the element by scene ID (`element/children/2`), while preparation errors use the value path (`/children/2`). This is cosmetic; both identify the element, and the span is exact.

### Summary (2026-10-28 18:33 verification)
- **Verified (7):** RF18, RF19, RF20, RF21, RF22, RF23 and RF24.
- **Reopened:** none.
- **New (1):** RF25 (Low): browser measurement failures are not located.
- RF1-RF17 statuses are unchanged.

## Verification Update 2026-10-28 18:37

The same reviewer session (`c1fb5c65-7d20-45f8-a425-8eeb7188a9d8`) ran this update.

- **Verified (1):** RF25.
- **Reopened:** none.
- **New findings:** none.

RF1-RF25 are all now Verified or Resolved. RF18-RF24 and earlier statuses are unchanged.

## New Findings Discovered During 2026-10-28 23:33 Review

### Scope of this pass

An independent Claude CLI reviewer (session `9714ae84-0863-4ac2-a891-6b15035c1dff`) reviewed the
generic playground preview routing in tasks section 8 (8.1-8.5). It covered the "Generic playground
preview routing" requirement and design section, and the implementation notes for 2026-10-28.

**Reviewed code:** `sites/playground/src/preview/{workbench.ts,registry.ts,model.ts,usePreview.ts,
protocol.ts,PreviewView.tsx,routing.test.mjs}`, `src/output/{OutputPane.tsx,evaluation.ts}`,
`src/App.tsx`, `src/graphics/{prepare.ts,DrawingView.tsx,useDrawing.ts,model.ts}`,
`src/worker/{session.ts,nx.worker.ts}`, `runtime/typescript/src/index.ts` (`declarationAtOrigin`),
the playground README and the example comments.

The probes were temporary `node --test` files that drove `createNxSession` the way
`routing.test.mjs` does. They were deleted after the run. No implementation code was changed. RF1-RF25
were not re-reviewed. Confirmed as working:
- Import-alias and same-name-spoof routing.
- A Drawing returned through a `let` function.
- Retention of the raw value and the Preview tab when graphics preparation fails.
- Compile and evaluation failures during edits to a static drawing.
- The stale-answer guard in `reducePreview`.
- Hiding the drawing while an answer is pending.

### ✅ Verified - RF26 When a component fails to start, the workbench stays pinned to it: Preview disappears, and Output keeps showing that failure after the source is fixed.
- **Severity:** High
- **Evidence:** `#start` records `this.#component = chosen` and `#fromRoot` before
  `createPreviewSession` (`workbench.ts:376-386`). When the session cannot start, `#session` stays
  undefined, so `#state()` reports `renderer: null` (`workbench.ts:523-526`). `App.tsx` then
  hides the switcher and forces Output, along with Preview's component picker and its missing-prop
  guidance. Every later reload runs with no session. It calls
  `#start(source, this.#component ?? undefined)` (`workbench.ts:401-402`), which ignores
  `#fromRoot`, so it starts the failed component again rather than the one `root` now names.
  Output is derived from this workbench (`output/evaluation.ts`), so it shows the start failure as
  "Evaluating root failed". Probed with `<A/>` (starts) and `<B/>` (`state { n:int = {1 / 0} }`):
  - Editing root from `<A/>` to `<B/>` gives `renderer=null, problem=startFailed` ("Division by
    zero").
  - Restoring `<A/>` still gives `renderer=null, component=B, startFailed`, so Output shows the
    division error for a correct document.
  - Picking B from the Preview picker (`start {component:"B"}`) gives `renderer=null`, so the
    picker the user just used disappears. The next edit stays on B.
  - A document with no root, `component <Greeting name:string/>` followed by
    `component <Hello/>`, gives `renderer=null, startFailed (Missing required … 'name')`. Hello can
    never be previewed, and Output reports "Evaluating root failed" for a document that has no
    root.

  This breaks the "Temporary errors" scenario: the tab is not kept during the failure. It also
  breaks the requirement that Output reflect the document's current value. Before this change,
  Output used its own evaluation and Preview kept the picker and the "would not start" message.
- **Recommendation:**
  - Report the component previewer whenever component preview applies: root returns a component
    element, or there is no root and components exist. Do this even when the start failed, so
    PreviewView keeps showing the picker, the `startFailed` message and the missing-prop note.
  - In `#reload` with no session, restart with
    `this.#fromRoot ? undefined : (this.#component ?? undefined)`, or record `#component` only
    after a successful start, so a fixed root recovers.
  - For a document with no root, Output should show the no-root message, not the component's
    start failure.
  - Add routing tests for:
    - a root component that fails to start, then recovers;
    - picking a failing component, after which the picker remains;
    - a no-root document whose first component needs props.

- **Fix:** Component capability is retained even when start fails; no-session reloads follow root when appropriate. Output distinguishes a missing root from a component inspection failure. Added regression tests for failed root recovery, failed picker choice, and component-only missing-prop guidance.
- **Verification:** Verified. `#state()` now reports `component` while `#component` is one of the
  current components, even with no session (`workbench.ts:543-546`). A no-session `#reload`
  restarts with `#fromRoot ? undefined : #component` (`workbench.ts:416-420`). Output uses
  `hasRoot` to show the no-root message (`output/evaluation.ts`). Probed through
  `createNxSession` with the page's `callFor`/`reducePreview`:
  - root `<A/>` -> `<B/>` (division by zero): `renderer=component, startFailed`, so Preview and the
    picker remain.
  - Restoring `<A/>`: the session runs A again, and Output shows its value.
  - Picking B: Preview remains. B is kept across edits, and it runs once B is fixed.
  - With no root, where `Greeting` needs a prop: `renderer=component, startFailed`, and Output says
    noRoot (`hasRoot=false`). Picking `Hello` runs it.
  - Changing the root to a scalar clears the component (`renderer=null`). Breaking the scalar
    document keeps it Output-only.

### ✅ Verified - RF27 Opening another document that does not compile keeps the previous document's Preview tab, value and running session.
- **Severity:** Medium
- **Evidence:** `App.tsx` resets only the tab preference on open (`setPreferredView(null);
  preview.reset(...)`, `App.tsx:128-129, 171-172`). `reset` sends an ordinary `start`. When the new
  source does not compile, `#start` returns at `programFor` (`workbench.ts:313-316`) before it
  clears `#structured`/`#session`. The answer therefore still carries the old document's
  `renderer`, value and session. Probed by starting the graphics drawing, then starting
  `let root() = { 1 + }`: the answer had `renderer=graphics`, `state.source` was the drawing, and
  Output showed the drawing's value (stale). After the counter, the answer had
  `renderer=component` and a live session. Preview's action buttons and timeline still dispatch to
  the previous document's program. The "preserve availability across temporary failures" rule is
  meant for edits to the same document. The spec says opening another document resets. In practice
  this comes from a shared link to broken code, or Back to an edited draft.
- **Recommendation:** Make a document reset explicit in the protocol, for example `start` with
  `reset: true` or an `open` op. Before compiling, it clears `#structured`, `#session`, `#left`,
  `#component`, `#fromRoot` and `#rootProps`, so a failed open reports no renderer and no value.
  Add a routing test: open a drawing, then open a non-compiling document, and expect
  `renderer=null` with no structured value or session.

- **Fix:** Added explicit start.reset protocol flag for opening another document. It clears the old value/session/picker/props/timeline and owned artifacts before compilation; usePreview.reset sends it. Added graphics/component-to-broken-document regressions.
- **Verification:** Verified. `start.reset` clears the value, session, timeline, component
  choice, props and owned builds before compiling (`workbench.ts:174-186`). `usePreview.reset` sends
  it, and both App open paths call it (`App.tsx:129, 171`). Probed for a drawing and for a running
  component: opening `let root() = { 1 + }` gives `renderer=null`, no session and no structured
  value. The next edit to `let root() = 1` evaluates normally. A render still in flight when reset
  disposes its artifact gets `NxDisposedResourceError`, which `prepareDrawingValue` catches. The
  reducer drops that answer as superseded.

### ✅ Verified - RF28 The routing change leaves an unused page hook, a fake drawing model and a dead structured branch in Preview.
- **Severity:** Low
- **Evidence:**
  - `graphics/useDrawing.ts` (new in this change) is imported by nothing. `graphics/model.ts`'s
    `reduceDrawing` and `generation` are used only by `graphics.test.mjs`. App builds
    `DrawingModel`-shaped data with a constant `generation: 0` (`App.tsx:158-165`) to satisfy
    `DrawingView`'s type.
  - `src/useEvaluation.ts` and `evaluateInBrowser` are no longer used by the page.
  - `PreviewView` still renders `state.structured` when there is no session (`PreviewView.tsx`,
    `(session ?? state?.structured) != null`). Preview is shown only for `renderer` "component"
    (always a session) or "graphics" (DrawingView), so this branch is unreachable. It contradicts
    "no duplicated static Preview".

  The legacy worker endpoints are intentionally kept, but these page-side modules are not
  endpoints.
- **Recommendation:**
  - Delete `useDrawing.ts` and `useEvaluation.ts`. Delete `reduceDrawing`/`initialDrawing` and
    their tests, or keep only what the graphics endpoint tests need.
  - Narrow `DrawingView`'s prop to `DrawingResult & { pending; retry }`.
  - Drop the `state.structured` fallback from `PreviewView`.
  - Keep `evaluateInBrowser` only if a test or the legacy endpoint still needs it.

- **Fix:** Removed obsolete useDrawing/useEvaluation hooks and drawing reducer/tests, narrowed DrawingView to its actual presentation props, removed fake generation and unreachable static Preview branch. Removed evaluateInBrowser; the shared source path now reuses the generic retry helper for builds only, never actions.
- **Verification:** Verified. `useEvaluation.ts`, `graphics/useDrawing.ts` and
  `graphics/model.ts` are gone, and nothing references `evaluateInBrowser`, `reduceDrawing` or a
  drawing `generation`. App passes `DrawingView` only source, scene, diagnostics, pending and
  retry. `PreviewView` no longer renders `state.structured`. The new `retrying` wrapper in
  `usePreview` introduced a separate message bug, recorded as RF30.

### ✅ Verified - RF29 The counter example and the README architecture section still describe the removed tab model and the old evaluation path.
- **Severity:** Low
- **Evidence:**
  - `src/examples/nx/counter.nx:1` says "Switch the pane from Output to Preview to run it", but
    Preview is now selected by default for a component.
  - The README diagram (`sites/playground/README.md:52-58`) still shows Output as
    "@nx-lang/sdk-wasm ──▶ root's value as annotated NX text ──▶ @nx-lang/value-view". Output now
    comes from the IR workbench, and `sdk-wasm` only compiles and formats.
  - `usePreview`'s doc comment and parameter still say `compiled` is "the latest source that
    compiled". App now passes the editor text (`usePreview(true, source)`), and the workbench
    compiles it.
- **Recommendation:**
  - Reword the counter comment to say Preview runs it, with Output showing the rendered value.
  - Update the diagram to show the IR workbench feeding Output and Preview.
  - Rename the `usePreview` parameter (for example `source`) and update its doc.

- **Fix:** Updated the counter comment, README architecture diagram and runtime wording, and usePreview editor-source parameter/documentation. The spec now explicitly says import alias; type-alias constructor semantics are outside this routing extension.
- **Verification:** Verified. The counter comment now says Preview runs it automatically, and
  that Output shows its rendered value. The README diagram shows the sdk-wasm compiler feeding the
  TypeScript IR workbench, which feeds structured Output and registered Preview. `usePreview`
  takes `source`, documented as editor text. The spec scenario now says "import alias".

### Questions (this pass)
- `type Pic = g.Drawing` followed by `<Pic …>` evaluates to a value spelled `<Pic content=…>`, and
  it is not routed to Preview. The legacy graphics endpoint likewise reports "unrecognized
  graphics declaration identity". Routing is consistent with preparation, so this is not raised
  as a routing finding. Does "including through an alias" in the spec mean only import aliases
  (`as g`)? If a type alias of a record should construct the aliased record, that is a compiler
  issue outside this change.

### Summary (this pass)
- **New:** 4 findings, RF26-RF29.
  - High (RF26): a component start failure removes Preview, and Output stays pinned to the failed
    component after the source is fixed.
  - Medium (RF27): opening a non-compiling document keeps the previous document's preview, value
    and session.
  - Low: RF28 (dead page hook, model and Preview branch) and RF29 (stale docs and comments).
- Graphics routing by declaration identity, alias and spoof handling, retention of the raw value
  on renderer failure, and stale-scene guards behaved as specified in probes.

## New Findings Discovered During 2026-10-28 23:40 Verification

The same reviewer session (`9714ae84-0863-4ac2-a891-6b15035c1dff`) ran this verification. Re-run
here, all passing:
- `pnpm --dir sites/playground typecheck`.
- `pnpm --dir sites/playground test`: 158 tests and all 25 examples
  (`/tmp/nx-verify-tests.log`).

The implementer's logs at 23:28-23:39 also pass:
- Development and production Chromium (`/tmp/nx-generic-preview-{browser,production-browser}.log`).
- SDK vitest, 130 tests.
- The full TypeScript runtime.

The probes were temporary `node --test` files driving `createNxSession`, `callFor` and
`reducePreview`. They were deleted after the run.

### ✅ Verified - RF30 Start and reload failures that reach the page are labelled "The compiler failed:" twice, or attached to the wrong cause.
- **Severity:** Low
- **Evidence:** `sendInBrowser` now wraps start/reload in `retrying` (`usePreview.ts:18-24`).
  `retrying` already converts every failure into `new Error(compileFailureMessage(error))`
  (`compile/worker.ts:20-36`). `usePreview`'s rejection handler applies `compileFailureMessage`
  again. The wrapped error is a plain `Error`, so the default branch prefixes it once more. Probe
  (`compileFailureMessage` of what `retrying` throws):
  - `NxStackOverflowError` -> "The compiler failed: The program recursed deeper than the browser
    can run. …"
  - `TimeoutError` -> "The compiler failed: The compiler took too long. It has been restarted …"
  - `NxHostCrashedError` -> "The compiler failed: The compiler crashed on this source. …"
  - A generic error -> "The compiler failed: The compiler failed: boom".

  Runaway recursion is a program error, not a compiler failure, and the old `useEvaluation` path
  showed these messages without the extra prefix. They appear in `preview.failure`, which
  Preview and Output both display.
- **Recommendation:** Format the message once. Either let `retrying` re-throw the original error
  and leave classification to the caller, or have `usePreview` use an already formatted message
  as is. Add a `usePreview`/`sendInBrowser`-level test (or a `retrying` + `compileFailureMessage`
  composition test) asserting the stack-overflow and timeout texts are unchanged.

- **Fix:** retrying now rethrows the original error (including its name) and only usePreview formats it. The composition regression covers stack overflow, timeout, host crash, module loading, and generic failure, asserting one classification and the correct retry count. Existing retry tests now explicitly check original error identity/classification.
- **Verification:** Verified. `retrying` re-throws non-recoverable errors unchanged, and returns
  or propagates the second attempt for recoverable ones (`compile/worker.ts:19-32`). Its only
  caller, `sendInBrowser`, uses it for start/reload only. `compileFailureMessage` is applied once,
  in `usePreview`'s rejection handler. Re-running the earlier probe now gives:
  - `NxStackOverflowError`: "The program recursed deeper than the browser can run. Look for …"
  - `TimeoutError`: "The compiler took too long. It has been restarted …"
  - `NxHostCrashedError`: "The compiler crashed on this source. …"
  - A generic error: "The compiler failed: boom".

  The new `worker.test.mjs` case checks the identity of the error received, the retry counts
  (2 for timeout and crash, otherwise 1) and the one-pass wording. Re-run here: the
  `worker.test.mjs` cases (10) pass, and `pnpm --dir sites/playground typecheck` passes. The
  implementer's `/tmp/nx-generic-preview-tests.log` (23:41) shows 159 tests and 25 examples
  passing.

### Summary (2026-10-28 23:40 verification)
- **Verified (4):** RF26, RF27, RF28 and RF29.
- **Reopened:** none.
- **New (1):** RF30 (Low): failure messages are prefixed twice.
- RF1-RF25 statuses are unchanged.

## Verification Update 2026-10-28 23:42

The same reviewer session (`9714ae84-0863-4ac2-a891-6b15035c1dff`) ran this update.

- **Verified (1):** RF30.
- **Reopened:** none.
- **New findings:** none.

RF26-RF30 are all Verified. RF1-RF25 statuses are unchanged.

## New Findings Discovered During 2026-10-29 01:11 Review

### Scope of this pass

An independent Claude CLI reviewer (session `2cc6e84a-afc7-48ea-a8d9-d182bdb2c0bd`) reviewed the
library-authored preview metadata and graphics icons in tasks section 9 (9.1-9.4). It covered the
proposal and design sections on library-authored preview metadata, and the 2026-10-29 implementation
and release notes. It also covered the delta specs `visualizer-library`, `visualizer-declarations`,
`source-tree`, `graphics-library` ("Graphics declarations carry presentation icons") and
`graphics-preview` ("Libraries declare preview associations in NX"), checked against the canonical
specs.

**Reviewed code:** `crates/nx-api/src/std/{visualizer/visualizer.nx,graphics/graphics.nx}`,
`crates/nx-api/src/program_visualizers.rs`, `crates/nx-hir/src/{visualizers.rs,lib.rs}`,
the visualizer-declaration and `preview-renderer-literal` parts of `crates/nx-types/src/infer.rs`,
`crates/nx-language-service/src/{visualizers.rs,source_tree.rs,source_tree_tests.rs}`,
`packages/language-protocol/src/index.ts`, `bindings/wasm/test/preview-metadata.test.ts`,
`sites/playground/src/preview/{registry.ts,workbench.ts,routing.test.mjs}`,
`src/worker/session.ts`, `scripts/check-graphics-browser.mjs`, the new tests in
`visualizer_declarations_tests.rs`, the website pages `reference/libraries/visualizer.md`,
`reference/concepts/visualizer-declarations.md` and `reference/libraries/graphics.md`, and the
playground README.

The probes were temporary `node --test` and vitest files that drove the WASM SDK, the Node SDK and
`createNxSession`. They were deleted after the run. No implementation code was changed. RF1-RF30 were
not re-reviewed. Confirmed as working:
- All 15 graphics icon names exist in the bundled `lucide@1.57.0`.
- Empty, whitespace-only, computed, missing and `{p}`-descriptor renderers are diagnosed.
- `v.ElementVisualizer`/`v.PreviewVisualizer` resolve through an import alias.
- `ValueVisualizer` and `FunctionVisualizer` reject `preview`, and `FunctionVisualizer` rejects
  content.
- Element functions accept only `FunctionVisualizer`. Records reject it, and paren functions remain
  unsupported.
- The document can override a library helper's icon (`visualizer g.Star = <FunctionVisualizer
  icon="sparkles"/>`) and Drawing's renderer.
- Base-to-derived inheritance resolves for a directly referenced kind.
- Node and WASM answer `includeUnreferencedDeclarations` source trees identically.
- Unknown renderer IDs stay Output-only.
- `nx-graphics-preview` and `scripts/check-graphics-preview.py` still pass with `@nx/graphics` now
  importing `@nx/visualizer`.
- The opt-in snapshot and source-tree query cost about 2.3 ms for the graphics example.

### ✅ Verified - RF31 `includeUnreferencedDeclarations` omits a derived kind that only inherits its preview, so a value of it built inside an imported helper gets no Preview.
- **Severity:** Medium
- **Evidence:** `ProgramVisualizers::targets` (`crates/nx-language-service/src/visualizers.rs:107`)
  returns only declarations that have a visualizer declaration of their own. `source_tree.rs:470`
  adds those, and `entry` adds their bases, but never their derived kinds. A probe used a library
  `shapes.nx` with `abstract type Shape` (`visualizer Shape = <ElementVisualizer
  preview=<PreviewVisualizer renderer="app.shapes"/>/>`), `type Square extends Shape` with no
  visualizer, and `export let makeSquare():Square = { <Square size=2/> }`. With the document
  `let root() = { makeSquare() }`, the opt-in table holds only `root`, `makeSquare` and `Shape`.
  `previewerFor` (`sites/playground/src/preview/registry.ts:32-37`) finds no entry for the evaluated
  `Square`, so the value is Output-only. Writing `<Square/>` in the document gives `Square` the
  inherited `app.shapes`. The source-tree delta requires the opt-in table to carry "the same resolved
  inheritance/override metadata". The task asks to match nominal identities "using existing
  precedence/inheritance", and the canonical "A kind with no visualizer inherits its base's" scenario
  applies. The single-document playground with today's standard libraries cannot reach this case.
  Multi-module hosts can, and so can any future library type that inherits a preview.
- **Recommendation:** With the option, also list every loaded declaration whose base chain reaches
  a target, for example by walking the program modules' records and components and calling `entry`
  for those with a target in `base_chain`. Alternatively, define the option as "every loaded
  element declaration with a resolved visualizer". Update the source-tree scenario to name derived
  kinds. Add a language-service or SDK test for the `shapes.nx` case above.

- **Fix:** The opt-in target list now includes loaded program declarations whose base chains reach a visualizer target, not only direct visualizer owners. Added a two-module Shape/Square/makeSquare regression and clarified the source-tree scenario.
- **Verification:** Verified. `ProgramVisualizers` records the program's loaded module identities.
  With the option, `targets` (`crates/nx-language-service/src/visualizers.rs`) also lists every
  declaration in a loaded module whose base chain reaches a declaration with a visualizer. A
  re-run of the probe gave the opt-in table `Shape`, `Square` and a further derived `Tiny`, each
  with the inherited `app.shapes`. The default query still lists only `root` and `makeSquare`, and
  a workspace module the program does not load contributes nothing. A host visualizer module
  (`visualizer B = …renderer="host.r"`) on a library base is inherited by the unreferenced `D`
  returned from `mk()`. The two-document regression is in `source_tree_tests.rs`, and the
  source-tree scenario now names inherited-only kinds. The opt-in query takes about 3.5 ms for the
  graphics example.

### ✅ Verified - RF32 A failure in the preview-metadata query fails the whole preview answer, including Output.
- **Severity:** Low
- **Evidence:** `#state()` computes the metadata lazily
  (`sites/playground/src/preview/workbench.ts:562`, `structured.build.visualizers ??=
  this.#visualizers?.(...)`). `answer()` calls `#state()` after its `try/catch`
  (`workbench.ts:225`). Any error from `createLanguageSnapshot`/`sourceTree` that is not a crash
  therefore rejects the preview request. That includes `start`, `dispatch` and `goTo` while a
  structured value is current. The page then loses the raw value that Output shows, and the
  failure is not cached, so it repeats on every call. Elsewhere this change treats preview
  availability as optional: unknown IDs stay Output-only, and RF26-RF30 keep raw output on
  renderer failures.
- **Recommendation:** Treat the metadata as best-effort. Catch errors in the lookup, re-throw
  `NxHostCrashedError` so the session still replaces the host, and otherwise cache `[]` for that
  build so `renderer` is `null`. Add a workbench test with a throwing `visualizers` callback,
  asserting that Output keeps its value.

- **Fix:** Workbench metadata lookup is best-effort and caches [] on non-crash errors; NxHostCrashedError still propagates to recovery. Tests retain Output across start/dispatch/goTo/restart and verify that failures are queried once.
- **Verification:** Verified. `#presentationMetadata` (`sites/playground/src/preview/workbench.ts:543`)
  calls the lookup once per build. It re-throws `NxHostCrashedError` and caches `[]` for any other
  error, so `renderer` is `null` and Output keeps the value. The RF32 test in `routing.test.mjs`
  throws from the callback across `start`, `dispatch`, `goTo` and a re-`start`. It asserts one
  attempt, an intact `<Drawing` Output and no problem, and it checks that a host crash still
  propagates.

### ✅ Verified - RF33 Some preview and function-target diagnostics are doubled or now misleading.
- **Severity:** Low
- **Evidence:**
  - The `preview-renderer-literal` check (`crates/nx-types/src/infer.rs:1642-1657`) fires
    whenever the renderer is not a nonempty literal. It does this even when another diagnostic has
    already reported the same mistake. Probes report `lowering-error` plus
    `preview-renderer-literal` for `renderer={r}` and for `preview={p}`, and `missing-property`
    plus `preview-renderer-literal` for `<PreviewVisualizer/>`.
  - A paren-function target is still rejected with "'badge' cannot have a visualizer: it is a
    function" (`crates/nx-hir/src/visualizers.rs:195`). Element functions can now have a
    visualizer, so the reason is no longer accurate and does not point to the supported form.
- **Recommendation:** Report `preview-renderer-literal` only where nothing else does: a string
  literal that is empty or blank, or a renderer value that is not a literal and that the
  constant-value rule did not already report. Change the paren-function reason to something like
  "it is a paren-style function; only an element-style function (`let <Name …/>`) can have a
  `FunctionVisualizer`". Assert single diagnostics in `preview_descriptors_are_typed_literal_metadata`.

- **Fix:** Literal validation now avoids adding an error when lowering/type checking already diagnosed the preview span, and leaves missing-renderer reporting to the required-property check. Tests assert single diagnostics. Paren-function rejection now names paren style and the supported element form.
- **Verification:** Verified. Each probe now reports exactly one diagnostic:
  - empty and blank renderers: `preview-renderer-literal`;
  - `renderer={r}` and `preview={p}`: `lowering-error`;
  - `renderer=foo`: `contextual-name-requires-nominal-type`;
  - `<PreviewVisualizer/>`: `missing-property`;
  - `preview="x"`: `record-field-type-mismatch`.

  The paren-function rejection now reads "it is a paren-style function; only an element-style
  function (`let <Name …/>`) can have a FunctionVisualizer". `visualizer_declarations_tests.rs`
  asserts a single error per case. The suppression is keyed on any error inside the preview span.
  For `renderer="" bogus="1"`, only the unknown field is reported until it is fixed. The
  declaration is rejected either way, so this is acceptable.

### ✅ Verified - RF34 The visualizer reference page is incomplete for the new records, and the graphics page omits the library's preview and icons.
- **Severity:** Low
- **Evidence:**
  - In `sites/website/src/content/docs/reference/libraries/visualizer.md`, the new "Preview
    descriptors" and "Element-function icons" sections (lines 298 and 321) come after "See also"
    (line 293).
  - On the same page, the "Elements" table (line 62) has no rows for `PreviewVisualizer` or
    `FunctionVisualizer`.
  - "The library source" (line 181) is meant to reproduce `visualizer.nx`, but it lacks
    `ElementVisualizer.preview`, `PreviewVisualizer` and `FunctionVisualizer`.
  - `reference/libraries/graphics.md` does not mention that `Drawing` declares
    `preview=<PreviewVisualizer renderer="nx.graphics"/>`, or the icons and helper
    `FunctionVisualizer`s the library ships. The graphics-library delta makes those a requirement.
  - In `visualizer.nx:28`, the new fields use trailing `///` comments, while the rest of the file
    uses leading blocks. The `preview` comment says less than the type's own documentation (inherited
    or overridden, host-supplied, Output fallback).
- **Recommendation:**
  - Move both sections above "See also", and add `PreviewVisualizer` and `FunctionVisualizer`
    rows to the types table.
  - Regenerate "The library source" from `visualizer.nx`.
  - Add a short "Presentation" note to the graphics reference with the `nx.graphics` association
    and the icon list.
  - Use leading `///` blocks for the new fields to match the file.

- **Fix:** Moved the new sections before the source/See also, added both records to the table, and regenerated the embedded library source verbatim. The graphics reference has its Presentation metadata section, renderer example and icon conventions/list. New field descriptions explain inheritance, overrides and fallback. Retained aligned trailing comments deliberately: the human explicitly prefers short trailing property docs with aligned continuations, which takes precedence over the requested leading-comment style.
- **Verification:** Verified.
  - In `reference/libraries/visualizer.md`, "Preview descriptors" and "Element-function icons"
    now come before "The library source" and "See also".
  - The types table has `PreviewVisualizer` and `FunctionVisualizer` rows.
  - The embedded library source matches `crates/nx-api/src/std/visualizer/visualizer.nx`
    byte-for-byte (checked with `diff`).
  - `reference/libraries/graphics.md` has a "Presentation metadata" section with the `nx.graphics`
    association, the shape icons and the helper `FunctionVisualizer` icons. This review's grep
    missed that section.
  - The `preview` field comment now covers inheritance, overrides and the Output fallback.
  - Per the user's stated preference, the aligned trailing `///` style is accepted and not
    reopened.

### ✅ Verified - RF35 The playground test copies the production metadata lookup instead of exercising it, and nothing checks its caching or snapshot release.
- **Severity:** Low
- **Evidence:**
  - `routing.test.mjs:150-180` re-implements the `createLanguageSnapshot` → `sourceTree` →
    `dispose` closure from `src/worker/session.ts:50-62`, so that test does not cover the
    production lookup.
  - `session.ts` repeats the literal `"nx://playground/playground.nx"` twice instead of using
    `MODEL_URI` (`src/editor/NxEditor.tsx:19`) or a shared constant.
  - No test asserts that the lookup runs once per build, or that its snapshot is disposed. The
    implementation notes name both as properties ("caches this metadata per build").
  - The WASM/Node parity test (`bindings/wasm/test/parity.test.ts:235`) does not include `preview`
    or `includeUnreferencedDeclarations`. A probe in this pass found the two bindings identical.
- **Recommendation:**
  - Export the lookup from `session.ts` (for example `previewDeclarations(host, source)`, using a
    shared URI constant), and use it in the test.
  - Add a counting `visualizers` callback test across `start`, `dispatch` and a re-`start` of the
    same source, and assert one snapshot per build that is disposed.
  - Add `preview` and the opt-in flag to the parity corpus.

- **Fix:** Extracted the production previewDeclarations lookup into preview/metadata.ts and shared MODEL_URI with the editor. Tests call that helper, count one lookup per compiled build across repeated operations, and assert disposal on success and failure. The SDK preview-metadata test now compares native and WASM opt-in metadata including preview descriptors.
- **Verification:** Verified.
  - `previewDeclarations` (`sites/playground/src/preview/metadata.ts`) is the single production
    lookup, and the worker session and the routing tests call it.
  - `MODEL_URI` now lives in `compile/evaluate.ts`, and the editor re-exports it. This adds no
    worker-only code to the main thread, because `App.tsx` already imported that module.
  - The RF35 test counts one snapshot, query and release across `start`, `dispatch`, `goTo` and a
    re-`start`, and one more for a new build. It also checks release when the query throws.
  - `bindings/wasm/test/preview-metadata.test.ts` compares the native and WASM opt-in trees.
  - A new imported-helper-to-`Drawing` test routes through the real lookup.
  - Re-run here: `routing.test.mjs` and `workbench.test.mjs` (43 pass), and
    `preview-metadata.test.ts` with `parity.test.ts` (36 pass).

### ✅ Verified - RF36 Element-function targets carry a placeholder `target_type`, and the alias branch maps a preview that cannot exist.
- **Severity:** Low
- **Evidence:**
  - `classify` gives an element function `target_type: Name::new("object")`
    (`crates/nx-hir/src/visualizers.rs:192`). `infer.rs:1572-1575` then resolves `object` as the
    bound `Target` type, although `FunctionVisualizer` has no `Target`. A reader has to work out
    that the value is a placeholder.
  - `attach_visualizers` copies `resolved.preview` into alias entries
    (`crates/nx-language-service/src/source_tree.rs:2002`). `ValueVisualizer` rejects `preview`,
    and the implementation notes say aliases keep only formats and editors, so that code never runs.
- **Recommendation:**
  - Make `target_type` an `Option<Name>`, `None` for element functions, and have the checker skip
    `Target` binding when it is absent.
  - Drop `preview` from the alias branch, so the protocol carries `preview` only for element kinds
    as the source-tree delta lists them.

- **Fix:** VisualizerTarget.target_type is now Option<Name>, None for FunctionVisualizer; inference skips Target binding for that case. Removed the impossible alias-preview mapping.
- **Verification:** Verified. `VisualizerTarget.target_type` is `Option<Name>`, and it is `None`
  only for element functions. `infer.rs` does not bind `Target` when it is absent. A probe gives a
  clean `FunctionVisualizer`, and `Target=Badge` is rejected as an unknown field. Unknown targets
  still get only `unknown-visualizer-target`. The alias branch of `attach_visualizers` no longer
  sets `preview`, and `ValueVisualizer` aliases resolve as before. Re-run here: `nx-language-service`
  (305 pass) and the `nx-api` visualizer tests (43 pass).

### Questions (this pass)
- A document type or host override can opt into a bundled renderer ID. Examples are `type Pic`
  with `renderer="nx.graphics"`, or `visualizer g.Rect = <ElementVisualizer
  preview=<PreviewVisualizer renderer="nx.graphics"/>/>`. Preview then appears and shows a
  `graphics-profile` error ("unrecognized … identity" or "Drawing must be the sole root"), with
  Output intact. This is consistent with "hosts SHALL activate only bundled supported renderer IDs"
  and with preparation's identity check, so it is not raised as a finding. Is a visible failing
  Preview the intended result of an explicit opt-in, rather than Output-only?
- Element functions that a library exposes only through its interface, without raw source, cannot
  be resolved as visualizer targets, because `interface_item` has no function case. The standard
  libraries are unaffected. Is this acceptable until packaged libraries need it?

### Summary (this pass)
- **New:** 6 findings, RF31-RF36.
  - Medium (RF31): the opt-in table omits derived kinds that only inherit a preview, so a value of
    one built in an imported helper is not routed.
  - Low: RF32 (a metadata failure fails the whole preview answer), RF33 (doubled or misleading
    diagnostics), RF34 (reference docs incomplete), RF35 (the test duplicates the lookup, and
    caching is untested), RF36 (placeholder `target_type` and an unreachable alias preview).
- The typed literal descriptor, static reading with qualified-tag normalization, precedence
  across declaring, other and host modules, the `FunctionVisualizer` target rules, the library-owned
  Drawing association, the Lucide icons and Output-only fallback for unknown IDs all behaved as
  specified in probes. No renderer code is loaded from metadata, and no HIR, runtime or IR encoding
  changed for this section.
- RF1-RF30 statuses are unchanged.

### Scope clarifications for this pass
Explicit requests for a bundled renderer remain visible Preview requests even when that renderer
rejects the value's profile; this preserves renderer diagnostics and raw Output as agreed. Renderer
IDs do not bypass nominal identity validation. Function visualizers currently target source-bearing
element functions; interface-only function metadata is deferred with packaged plugin/library work.

## Verification Update 2026-10-29 01:31

The same reviewer session (`2cc6e84a-afc7-48ea-a8d9-d182bdb2c0bd`) ran this update. The probes were
temporary `node --test` files that drove the rebuilt WASM SDK (`dist` 01:23) and `createNxSession`.
They were deleted afterwards, and no implementation code was changed.

- **Verified (6):** RF31, RF32, RF33, RF34, RF35 and RF36.
- **Reopened:** none.
- **New findings:** none.
- **Questions from the review pass:** both are answered in the docs and notes. An explicit opt-in
  to a bundled renderer shows that renderer's profile errors and keeps the raw Output. Interface-only
  function targets are future packaged-library work.

RF31-RF36 are all Verified. RF1-RF30 statuses are unchanged.

## New Findings Discovered During 2026-10-29 02:45 Review

### Scope of this pass

This is an independent, full review of the whole change by a Claude reviewer (session
`2e3fd5ea-4d4d-4afd-ac2d-833e53b95385`). It is not an incremental section 9 pass. Code that earlier
passes reviewed was back in scope, and current behaviour was reproduced instead of trusting earlier
Verified statuses. The reviewer read `openspec status`/`instructions apply`, proposal.md, design.md,
tasks.md, math-readiness.md, implementation-notes.md, release-notes.md and all six delta specs.
`openspec validate --strict` passes. Five parallel sub-reviews covered these areas, and their
findings were spot-checked before being recorded:

- **Neutral core:** `crates/nx-api/src/std/graphics/graphics.nx`, `crates/nx-graphics`, the
  construction-declaration index in `artifacts.rs`, and the codegen and standard-library tests.
- **Native and CI:** `crates/nx-graphics-skia` (adapter, fonts, preview binary, corpus), fixtures,
  `schemas/`, `scripts/*graphics*`, `evaluations/graphics-v1`, `examples/graphics`, and
  `.github/workflows/{graphics-native,deploy-playground}.yml`.
- **Browser pipeline:** the WASM bridge and SDK (`bindings/wasm/{native/src,src,test}`), the
  TypeScript runtime's call-site origins, and `sites/playground/src/graphics/*`.
- **Playground application:** the workbench, session, model, App, Output/Preview, registry and
  metadata, `packages/value-view` selection, examples and scripts, and the README.
- **Visualizer metadata:** `visualizer.nx`, `program_visualizers.rs`, `nx-hir`/`infer.rs`
  visualizer checks, language-service source tree, language protocol, and the website
  reference pages.

Probes were temporary scratch crates, NX sources, Node scripts and Playwright runs under
`/tmp/nxrev-*`. Any temporary file placed in the repository was deleted. No implementation file was
changed. These all pass on the current tree:

- `cargo test -p nx-graphics` (9), `-p nx-graphics-skia --features native` (8),
  `-p nx-codegen graphics` (2) and `-p nx-language-service` (305);
- the WASM SDK (132), the playground (164 tests and 25 examples) and `test:graphics-browser`;
- the TypeScript runtime suites.

Fresh `tsc` builds of `runtime/typescript/dist` and the SDK `dist` are byte-identical to the
checked-in output. No RF1-RF36 fix regressed. The human decisions listed in the request were applied
and are not reported as defects:

- IR-only runtimes;
- Canvas 2D in the browser;
- declarative renderer IDs;
- presentation-only FunctionVisualizer;
- no survey renderer;
- the trailing `///` property style;
- Preview selection that preserves the current mode.

Confirmed as working in this pass:

- **Path parsing:** implicit and relative commands after M and Z, S/T reflection, arc radius
  correction and flags.
- **Transforms and view box:** transform order, and view-box contain/cover/fill with every
  alignment.
- **Strokes and opacity:** dash normalization, zero-width strokes, isolated group opacity
  (single 128 alpha, nested 64), and evenodd clip holes in both Skia and Canvas.
- **Native painting:** canvas-state restoration on failure, atomic publication and stale
  inspection, and device scale in the native preview.
- **Fonts and selection:**
  - the cmap parser against an independent parse;
  - call-site origins for nested helpers inside loops;
  - alias and spoof identity checks;
  - tab-preference persistence and reset;
  - multi-byte Source-mode selection.
- **Visualizer metadata:**
  - visualizer precedence and merging across the declaring, other and host modules;
  - RF31 inherited-only targets;
  - Node/WASM metadata parity;
  - all 15 Lucide icon names;
  - no Skia in the compiler or SDK dependency graphs.

### ✅ Verified - RF37 The `nx` block in the graphics reference does not compile, so the website's code-block check fails `pnpm -r test` in CI.
- **Severity:** High
- **Evidence:**
  - `sites/website/src/content/docs/reference/libraries/graphics.md:173-178` is a plain
    ```` ```nx ```` block that declares `visualizer Drawing = …` without an import, so `Drawing` is
    not in scope.
  - `node --no-warnings sites/website/scripts/check-code-blocks.mjs` reports
    `graphics.md:174:12: unknown-visualizer-target: Unknown name 'Drawing' … 223 nx blocks in 30
    files, 1 failures`. This is the only failure on the site.
  - The website `test` script runs that checker (`sites/website/package.json:11`), and
    `.github/workflows/build.yml:273` runs `pnpm -r test`.
- **Recommendation:** Mark the block `nx fragment`, or make it compile with
  `import "@nx/graphics" as g` and `visualizer g.Drawing = …`. Run the website `test` script as
  part of this change's validation.

- **Fix:** Marked the library-local visualizer example as an NX fragment. Website tests pass, including all 223 NX blocks.
- **Verification:** Verified. The block is now `nx fragment`, and `node --no-warnings sites/website/scripts/check-code-blocks.mjs` reports `223 nx blocks in 30 files, 0 failures`.

### ✅ Verified - RF38 A worker failure (crash, timeout or runaway recursion) drops the shared Output value, hides Preview and resets the component session, and "Start again" cannot be reached.
- **Severity:** Medium
- **Evidence:**
  - **Where the value is dropped:**
    - On `failed`, `reducePreview` sets `state: null` (`sites/playground/src/preview/model.ts:82-90`).
    - `App.tsx:155-156` then computes Preview availability from `preview.state?.renderer`, so the
      switcher disappears and Output is forced.
    - `output/evaluation.ts` receives a null outcome, so the last value is gone instead of being
      marked stale.
    - "Start again" (`PreviewView.tsx:116`) renders only in the Preview slot, which is hidden in
      this state.
    - The next call is `start`, not `reload` (`model.ts:107`), so the session the workbench kept is
      discarded.
  - **Reproduced in the browser:**
    - On `/play/graphics`, nesting 1,000 `<Group>` elements removes both tab buttons and leaves 0
      `nx-value` nodes; only the recursion message remains.
    - On `/play/counter`, click to Count: 4, then append a deeply nested graphics value and restore
      the source. The counter comes back at "Count: 3" with a one-step timeline. An ordinary compile
      error keeps "Count: 4".
  - **Requirements and tests:**
    - The graphics-preview "Temporary errors" scenario requires the tab to stay available with
      stale values marked.
    - RF30 classed runaway recursion as a program error.
    - `src/evaluation.test.mjs:60-67` still asserts "keeping the last value stale", but against
      `reduceEvaluation`, which the page no longer uses. The test therefore passes without covering
      the shipped path.
- **Recommendation:**
  - Keep `state` on `failed`, recording the failure separately, and show the message over the
    stale Output and Preview.
  - Send `reload` when a prior state exists.
  - Make "Start again" reachable from Output, or keep Preview visible.
  - Delete `reduceEvaluation`/`initialEvaluation` and their tests, or retarget them at
    `reducePreview`/`outputEvaluation`.
  - Add a model test and a browser check for a crash after a successful run.

- **Fix:** The page retains its last PreviewState after worker failure and reloads it after edits. Output and Drawing expose retry controls; failed graphics hide old scenes. Removed the obsolete evaluation reducer and retargeted its tests at the shipped preview/output model. Chromium reproduces deep-input failure and verifies Count: 4 survives recovery with its timeline.
- **Verification:** Reopened (Medium). Edits to the same document are fixed:
  - With 1,000 nested Groups, both tabs stay and Output keeps the last `<Drawing …` value marked stale. "Start again" (Output) and "Retry drawing" (Preview) are reachable, no old scene is drawn, and there is no call loop (2 → 2 preview posts over 4 s).
  - On `/play/counter`, Count: 4 with its timeline survives a deep failing edit and the source restore.
  - A failing first load from a `#code=` link shows the failure with "Start again" and does not loop.

  What remains is the RF27 analogue for a failed open. At Count: 4 on `/play/counter`, navigate (pushState + popstate) to a `#code=` link for a deep graphics document. The editor shows the graphics source, but Preview still shows the counter's Count: 4, its 3-step timeline and its component picker, and Output shows the counter's value as stale. The worker cleared its session for `start{reset:true}` (`workbench.ts:182-193`) and then failed. The page's `failed` branch (`sites/playground/src/preview/model.ts:82-90`) keeps `state` for every call, including a reset. Choosing another example from the menu after a failure is clean.

  Fix: on `failed` for a reset `start`, clear `state`/`notice`, either by recording the call in the `failed` event or by clearing `state` in `asked` when `reset` is set. Add a model test and a browser case for opening a failing document over a running session.

- **Fix (second pass):** On a reset start, the page clears the old state/notice/failure before the worker runs. Added model coverage and a real #code= navigation over a running counter that fails compilation and shows no previous value, picker or Preview tab.
- **Verification (second pass):** Verified (second pass). At Count: 4 on `/play/counter`, navigating to a failing deep-graphics `#code=` link now shows only the recursion failure and "Start again". No value, tabs, timeline or picker remain, and no calls are sent while idle (3 → 3). Back gives a fresh Count: 3, and Forward/Back repeat cleanly with only alternating reset starts (6 → 6 idle). A failing first load followed by choosing the counter has no carry-over. A same-document failure still keeps Count: 4 stale, and restoring it keeps steps Start, Tapped, Edit. A late action-answer race found here is RF72.

### ✅ Verified - RF39 `check-examples` passes an example with no root, while printing "not ok".
- **Severity:** Medium
- **Evidence:**
  - With the workbench `noComponent` problem, `evaluateExample` returns `outcome: null` with no
    diagnostics.
  - `sites/playground/scripts/check-examples.mjs:77-79` then prints `not ok` and `continue`s
    without recording a failure. The `noRoot` branch at line 83 can no longer be reached.
  - Reproduced with a copy of the script and an extra example `let x = 1`: the output was `not ok -
    No root probe`, then "All 26 examples match their expected output.", and the exit code was 0.
    At HEAD this reported "it has no root".
  - The header comment (line 5) still names `evaluateSource`. A blank line at 166-167 detaches
    `checkDocs`'s doc comment.
- **Recommendation:** Map `noComponent`/`hasRoot === false` to `{kind: "noRoot"}`, and record a
  failure whenever `outcome` is null. Fix the header, and reattach the doc comment.

- **Fix:** Missing roots map to noRoot; a null outcome always adds a failure. Updated the checker description and doc-comment placement.
- **Verification:** Verified. A copy of the script with an extra `let x = 1` example prints `not ok - No root probe` and `- noroot-probe: it has no root`, and exits 1 (`check-examples.mjs:77-81`). The header is updated, and the `checkDocs` doc comment is reattached (166-170).

### ✅ Verified - RF40 Browser text is measured and painted at the authored local font size, so rounded Chromium metrics yield "conservative" ink smaller than the painted glyphs, tiny sizes misplace text, and sizes above 10000 are silently clamped.
- **Severity:** Medium
- **Evidence:**
  - **Code:** `sites/playground/src/graphics/browser-font.ts:92-109` measures `${size}px` text and
    reports `actualBoundingBox*` as ink with `conservative: true`. `canvas.ts:37-45` paints at the
    same local size under the view-box transform. This is the browser equivalent of RF8/RF9.
  - **Chromium, "Hello world":**
    - The advance is 5.606 em at 1-16 px, but drifts below about 0.5 px and becomes 0 below
      0.05 px.
    - Ink is rounded to whole local units.
    - At 16 px, ascent and descent are 12/0, against true values of 12.25/0.25 measured at 1000 px.
      The rounding goes to the nearest unit, not outward.
    - At 20000 px the advance equals the 10000 px value.
  - **End to end:**
    - With `viewBox` 0..24 and `fontSize=7`, painted pixels start at device y=73 while
      `visible.y0` is 80.
    - With `viewBox` 0..1 and `fontSize=0.04`, the text paints 82 px wide instead of the 103 px of
      the equivalent 0..100/`fontSize=4` drawing. The inflated ink box then covers x 278..640 and
      captures clicks.
    - `fontSize=20000` renders at half size with no diagnostic.
- **Recommendation:**
  - Measure and paint at a fixed reference size below the browser clamp, for example 1000 px.
    Scale the metrics by `size/ref` and paint under `ctx.scale(size/ref)`.
  - Round ink outward, with a pad.
  - Reject sizes the browser would clamp.
  - Add Chromium tests that compare scaled and unscaled layouts and check painted pixels against
    the visible bounds.

- **Fix:** Browser measurement and painting use a fixed 1000px reference font with metrics and drawing scaled to authored units. Ink rounds outward with a reference-pixel pad. Chromium verifies consistent advances and ink from 0.01 through 20000 units.
- **Verification:** Reopened (Low). The reported causes are fixed:
  - The advance is 5.60596 em for "Hello world" at every size from 0.01 to 500000.
  - 20000 is no longer clamped.
  - Ink is independent of size and rounded outward.
  - Worker (OffscreenCanvas) and page measurements are bit-identical.
  - `viewBox` 0..1 with `fontSize=0.04` paints exactly like 0..100 with `fontSize=4` (106 px).
  - `viewBox` 0..24 with `fontSize=7` paints inside its visible bounds.

  What remains is that painted text still leaves its "conservative" ink. With `viewBox` 0..100 and `fontSize=4`, 103 pixels with alpha up to 137 lie wholly outside the visible rectangle at DPR 1 (row 224 has alpha 59 while `y0` is 225.27). At DPR 2 it is 361 pixels with alpha up to 247. Glyphs are hinted and antialiased at the device size they are drawn, but the pad (`browser-font.ts:96-99`) is one 1000 px reference pixel, 0.001 em, far below one device pixel.

  Fix: express the pad in device pixels, for example at least 1.5 device pixels converted to local units at preparation scale. Add a Chromium assertion that painted pixels stay inside the visible bounds.

- **Fix (second pass):** Added a host-configured text_ink_padding (default 0) to painted text bounds after the world transform; browser uses 1.5 physical pixels converted by device scale before clipping. Intrinsic font metrics remain separate. Chromium checks every black text pixel against painted bounds at DPR1/2/4.
- **Verification (second pass):** Verified (second pass). The painted text pad is `text_ink_padding / scale`, applied after world mapping and before clipping (`crates/nx-graphics/src/lib.rs:1035-1040`). It is part of the hashed config. Twelve Chromium cases at DPR 1, 2 and 4 found 0 black text pixels outside the visible bounds:
  - sizes 0.04 to 20000;
  - middle and end anchors;
  - text rotated 30° and skewed 25°;
  - a clip on the Text, a clip on its parent, and viewport clipping.

  Before the second pass, DPR 2 had 361 pixels outside. Hit-testing uses the same bounds. Native manifests record `text_ink_padding: 0.0`, and the fixture walkthrough passes.

### ✅ Verified - RF41 SVG arcs with any radius at or below 1e-5 become straight chords, so small-unit Path arcs, Arcs and RingSectors render wrongly.
- **Severity:** Medium
- **Evidence:**
  - **Code:** `crates/nx-graphics/src/lib.rs:1415-1430` falls back to `line_to` when
    `kurbo::Arc::from_svg_arc` returns `None`. In kurbo 0.11.3, `is_straight_line` returns `None`
    for `radii <= 1e-5` (`svg.rs:417`). SVG treats only zero radii as lines; nonzero radii are
    scaled up to reach the endpoint.
  - **Prepared paths:**
    - `M0 50 A0.000001 0.000001 0 0 1 10 50` prepares as `M0,50 L10,50`, where SVG draws a
      radius-5 semicircle.
    - `A0.000001 10 …` collapses the same way.
  - **Preview with `viewBox` 0.0001² at 200×200:**
    - `Circle r=0.000008` draws a ring.
    - `<g.Arc radius=0.000008 sweepAngle=360/>` draws a horizontal line.
    - The matching `RingSector` paints nothing.
- **Recommendation:** Treat only exactly-zero radii as a line. Normalize the arc before calling
  kurbo: translate to the start point, scale by `1/max(|rx|,|ry|,chord)`, then map the cubics back.
  Add tiny-radius Path and helper tests.

- **Fix:** Nonzero SVG radii are normalized to units where the smallest radius is one, avoiding kurbo's absolute epsilon; only exactly-zero radii become lines. Added tiny/corrected/anisotropic arc regressions.
- **Verification:** Verified. Only exactly-zero radii become lines (`crates/nx-graphics/src/lib.rs:1517-1553`), and nonzero arcs are scaled before kurbo. Probes:
  - `M0 50 A0.000001 0.000001 0 0 1 10 50` gives 6 cubics with bounding box y 45..50, a radius-5 semicircle. Rotated and large-arc variants are also correct.
  - `A0 5` and `A0 0` give a line.
  - Anisotropic vertical chords give thin half-ellipses. Corrections beyond the 1e7 cap are rejected as out of host range.
  - The smallest accepted radius (1e-44) normalizes cleanly. No NaN appears.
  - In a 0.0001² viewBox at 200×200, the tiny Arc draws a ring and the tiny RingSector keeps its hole.

### ✅ Verified - RF42 Arc expansion is checked against the path-command limit only after allocation, with a tolerance derived from the uncorrected radii.
- **Severity:** Medium
- **Evidence:**
  - **Code:**
    - `svg()` counts the authored commands (`crates/nx-graphics/src/lib.rs:336-373`), normalizes
      the path, and only then checks `p.elements().len()` (`:374-377`).
    - The flattening tolerance is `1e-6 * max(|rx|,|ry|,1)` from the authored radii (`:1425`).
      After radius correction enlarges tiny radii, kurbo emits about 66 cubics per semicircle.
  - **Measurements:**
    - `M0 0` plus 99,999 `A 2e-5 2e-5 0 0 1 ±9e6 0` commands (about 3 MB of text) is rejected with
      "normalized path command limit exceeded" after 0.4 s, with a 368 MB peak RSS.
    - A single such arc becomes 67 elements.
  - This breaks the design's "preflight … limits" before resources are committed, and it applies
    in the browser preparer too.
- **Recommendation:**
  - Use a tolerance relative to the corrected radii.
  - Before allocating, compute each arc's segment count from its sweep and tolerance, and charge
    it to a scene-wide remaining budget that is passed into `svg()`.
  - Add a limit test for this input.

- **Fix:** Arc tolerance derives from corrected radii. Expanded segments are counted against the remaining scene budget before appending; authored path and polygon counts are bounded too. Added budget regressions.
- **Verification:** Verified. Tolerance comes from the corrected radii (`lib.rs:1544`), and each arc's segment count is checked against the remaining budget before appending (`:1545-1548`).
  - 99,999 `A 2e-5 …` commands are rejected in 84 ms at 18.9 MB peak RSS, against 368 MB before.
  - 99,000 arcs are rejected on expansion in 115 ms at 24.6 MB.
  - The budget is scene-wide: two 61-element paths fail at limit 126 and pass at 127, and a Group clip plus a child Path fail at 122 and pass at 128.
  - Polygon and Polyline points are pre-counted (`:837-844`).

### ✅ Verified - RF43 Group painted and visible bounds include content that a child's own clip removes or that opacity 0 hides, and they are still reported as exact.
- **Severity:** Medium
- **Evidence:**
  - **Code:** a parent unions `child.painted`, computed before the child's clip and regardless of
    its opacity (`crates/nx-graphics/src/lib.rs:945`). `visible` then derives from that union
    (`:959-969`) with no conservative flag.
  - **Clipped child:** in a Group holding a Rect 0..50 with `clipPath="M0 0H10V10H0Z"`, the child is
    correctly visible over 0..10 (exact), but the Group reports 0..50 with `conservative: false`.
  - **Hidden child:** in a Group holding Rects at 0..10 and 50..60, where the second has
    `opacity: 0`, the Group reports a visible area of 0..60 with `conservative: false`.
  - **Impact:** the manifest, the inspector and canvas hit-testing (RF49) all use these bounds.
- **Recommendation:** Union each child's post-clip extent (its painted bounds intersected with its
  own clip box in world space), and skip children with opacity 0. At minimum, mark the bounds
  conservative when a descendant clips. Add both cases as tests.

- **Fix:** Container painted bounds union child post-clip contributions and skip opacity-zero children. Rectangular intersections keep their accurate classification; other clipped intersections remain conservative. Added clipped/hidden descendant tests.
- **Verification:** Verified (`lib.rs:1020-1047`).
  - A child clipped to 0..10 beside an opacity-0 sibling gives a Group with painted and visible 0..10, exact.
  - Nested clipped and hidden groups give 0..10, conservative. An only-hidden subtree is empty.
  - A rotated clip, a circle under a rectangular clip, a stroked rect and an evenodd clip with a hole are all conservative. A clip outside the content gives empty.
  - Translated nesting stays exact and correct.

### ✅ Verified - RF44 A zero-length closed subpath paints a cap dot that bounds don't report, and a full-sweep RingSector with the default `innerRadius=0` emits one.
- **Severity:** Medium
- **Evidence:**
  - **Helper source:** for a full sweep, `ringData` emits `outer + " Z " + inner + " Z"`
    (`crates/nx-api/src/std/graphics/graphics.nx:414-417,453`). With radius 0 the inner arcs
    collapse to `M cx cy Z`, which Skia strokes as a cap dot.
    `<g.RingSector cx=50 cy=50 outerRadius=40 sweepAngle=360 fill=none stroke=<… width=6
    lineCap=round/>/>` renders the circle plus a dot at (50,50).
  - **Bounds:** `bounds()` returns empty when kurbo yields no segments (`crates/nx-graphics/src/lib.rs:428-437`).
    - `Path data="M30 50 Z"` with a round cap of width 10 reports empty geometry, painted and
      visible bounds, but paints a dot at (30,50).
    - `M70 50 L70 50` correctly gets 50..90.
- **Recommendation:** Emit only the outer subpath when `innerRadius == 0`. Include the points of
  zero-length closed subpaths in the stroke bounds for round and square caps. Add pixel and bounds
  tests.

- **Fix:** Full-sweep solid RingSector omits the zero-radius inner subpath. Bounds include isolated closed-moveto points even in mixed paths, so cap strokes cannot escape inspection bounds. Added helper and bounds regressions.
- **Verification:** Verified for the reported cases. A full-sweep solid RingSector emits only its outer subpath (`graphics.nx:548`).
  - Stroked with a round cap of width 6 at ±360°, it has a white centre and no dark pixels within 30 px. `innerRadius=20` keeps its hole.
  - `M30 50 Z` with a round cap gives painted 10..50 × 30..70, and mixed absolute and relative paths include the dot.
  - The bounds change over-reports fill-only paths; that is RF64.

### ✅ Verified - RF45 The committed walkthrough manifests no longer match what the current code produces, and nothing compares them.
- **Severity:** Medium
- **Evidence:**
  - **Age:** `crates/nx-graphics-skia/fixtures/revisions/{5,7}` were written on 2026-10-28 at
    16:23. `graphics.nx` last changed on 2026-10-29 at 01:01.
  - **Regenerated from the same `main.nx`:**
    - The PNGs are byte-identical.
    - `revision` changed (`7b38a969…` → `ec9dee7e…`, `58849e0b…` → `4a0b843d…`).
    - `config.graphics_version`, every `identity.version` and `program_revision` changed.
    - Every helper node's provenance span changed (for example 6203..6443 → 15397..15637), so the
      committed spans point at the wrong text in today's `graphics.nx`.
  - **Why nothing catches it:** `scripts/check-graphics-preview.py` only overwrites the fixtures,
    with `--write-fixtures`, and never compares against them.
  - **Docs:** `implementation-notes.md` and `graphics.md` present these as current regenerated
    artifacts.
- **Recommendation:** Regenerate the fixtures. Add a compare mode, run in CI, that checks the PNG
  bytes and the manifest structure and fails with a "regenerate" message when versions or spans
  drift.

- **Fix:** Regenerated both walkthrough revisions against the current library. The default checker now compares source, PNG bytes and parsed manifest with a regeneration message on drift; --write-fixtures is explicit.
- **Verification:** Verified. Regenerating with `--write-fixtures` into a copy gives output byte-identical to the committed `revisions/{5,7}` (PNG, manifest and `main.nx`). Provenance spans slice to the right `<Polygon …` text in today's `graphics.nx`. In two separate copies, shifting a span by 1 or appending a byte to a PNG makes the default run of `check-graphics-preview.py` fail (`:91-92`) with the regeneration message.

### ✅ Verified - RF46 CI does not run several of the deterministic graphics checks the change relies on.
- **Severity:** Medium
- **Evidence:**
  - **Fixtures:** `scripts/verify-graphics-fixtures.mjs` appears in no workflow (`grep -rn
    verify-graphics-fixtures .github/` finds nothing), yet the graphics-preview spec requires "CI
    SHALL validate deterministic fixtures".
  - **Schema:** `graphics-native.yml` runs `check-graphics-preview.py`, which only checks that keys
    are present (`:50-56`). Nothing validates success or error output against
    `schemas/graphics-preview.schema.json`. A probe with `jsonschema` found the current outputs
    valid.
  - **Font hash:** the fixture font's `sha256` is checked only by a playground test.
  - **Browser check:** the Chromium graphics check (`deploy-playground.yml:68-69`) runs only on push
    to main or manual dispatch, not on pull requests.
  - **Clippy:** `build.yml`'s workspace Clippy builds `nx-graphics-skia` without `native`, so the
    adapter and the preview binary are never linted.
- **Recommendation:** In `graphics-native.yml`:
  - run `node scripts/verify-graphics-fixtures.mjs`;
  - validate success and error JSON against the schema;
  - check the font hash;
  - run `cargo clippy -p nx-graphics-skia --features preview -- -D warnings` and `cargo fmt --check`.

  Also consider running the browser check on pull requests that touch the graphics paths.

- **Fix:** Native PR CI validates deterministic tasks, supplied font hashes and success/error JSON schemas, runs native all-target Clippy and rustfmt, and compares walkthrough fixtures. Playground browser validation now runs on matching PRs; the deployment job explicitly excludes PRs.
- **Verification:** Verified. All workflows parse with PyYAML.
  - `graphics-native.yml` runs on `pull_request` and runs the native tests, `verify-graphics-fixtures.mjs` (including font sha256), `cargo fmt --check`, native all-target Clippy and the schema-validating, fixture-comparing checker.
  - `deploy-playground.yml` adds a PR trigger with the Chromium check, and the deploy job is gated by `if: github.event_name != 'pull_request'`.
  - Gaps in the path filters, concurrency and preview tests are RF66 and RF68.

### ✅ Verified - RF47 The preview command silently ignores misspelled options, unknown limit keys and mistyped font fields, then renders with defaults.
- **Severity:** Medium
- **Evidence:**
  - **Option parsing:** `arg()` (`crates/nx-graphics-skia/src/bin/preview.rs:24-29`) matches exact
    flag names and ignores everything else.
    - `--widht 100` succeeds at width 640.
    - `--scale=2 --width=50` succeeds with the default revision.
  - **Limit keys:** `Limits` is `#[serde(default)]` without `deny_unknown_fields`, and its keys are
    snake_case, which `graphics.md` doesn't document. `{"pathCommands":1,"layerPixels":1}`
    succeeds with the defaults, while `{"path_commands":1}` correctly fails.
  - **Font fields:** `preview.rs:47-48` turns `"weight":"700"` or `"italic":"true"` into 400/normal.
    A bold face is then silently registered as regular. "invalid font bytes" doesn't name the file.
  - **Impact:** a host that thinks it tightened a safety limit is still running on the defaults.
- **Recommendation:**
  - Reject unknown flags, and either reject `--x=y` or accept it explicitly.
  - Add `#[serde(deny_unknown_fields)]` to `Limits`, and document its keys or rename them to
    camelCase.
  - Make wrongly typed `weight`/`italic` errors that name the face entry, and include the path in
    font-byte errors.

- **Fix:** CLI options reject unknown/missing/duplicate flags and support explicit --name=value. Limits deny unknown keys; typed font manifests reject malformed fields and report paths for font-byte failures. Documented snake_case limits and added negative tests.
- **Verification:** Verified.
  - Unknown options, missing values and duplicates are rejected: `--widht` gives "unknown option", and `--width 1 --width=2` gives "duplicate option".
  - `--scale=2 --width=50` takes effect and produces a 100×960 PNG.
  - Limits `{"pathCommands":1}` and `{"node":1}` are rejected as unknown fields, naming the snake_case keys. `{"path_commands":1}` still fails the scene.
  - String `weight`/`italic` font fields are rejected with the manifest path and position.
  - Bad font bytes and sha256 mismatches name the file.
  - Array-form JSON still bypasses the named-key checks; that is RF67.

### ✅ Verified - RF48 `--measure-text` does not apply the same validation and diagnostics as scene preparation.
- **Severity:** Medium
- **Evidence:**
  - The graphics-preview spec requires measurement diagnostics "consistent with scene preparation".
  - **Unsupported fields accepted:** a request with `"letterSpacing":5,"dominantBaseline":"middle"`
    returns metrics as if both were absent. Preparation rejects both as unsupported.
  - **Generic messages:** `size:0`, `weight:0` and a newline all return the generic "invalid
    single-line text request". Preparation reports `font size must be positive`, `invalid font
    weight` and `multiline text is unsupported`.
  - **Cause:** `TextRequest` (`crates/nx-graphics/src/lib.rs:130-137`) has no `deny_unknown_fields`,
    and `Fonts::shape` (`crates/nx-graphics-skia/src/lib.rs:153-163`) uses one combined check.
  - Design §4 also asks for the same settings in measurement and preview responses, but the
    measurement response omits the renderer configuration.
- **Recommendation:** Deny unknown fields, and route measurement through the Text preparation
  validator. Include the renderer configuration in the response. Add one negative test per
  diagnostic.

- **Fix:** TextRequest denies unknown fields and both scene/native measurement use the shared text-profile validator. Standalone responses include the renderer configuration ID and fonts. Added negative measurement checks.
- **Verification:** Verified. Both paths call `validate_text_request` (`nx-graphics/src/lib.rs:156,890`, `nx-graphics-skia/src/lib.rs:153`).
  - `letterSpacing` and `dominantBaseline` are rejected as unknown fields.
  - Size 0, weight 0 and a newline give the same strings as `g.Text` preparation.
  - The response includes `renderer`, `fonts` and `profile`.

### ✅ Verified - RF49 Clicking the canvas selects the topmost node whose visible bounding box contains the point, not the node painted there.
- **Severity:** Medium
- **Evidence:**
  - **Code:** `sites/playground/src/graphics/DrawingView.tsx:235-246` tests only `node.visible.rect`.
  - **Reproduced:** with a blue Rect at (70,5,20,20) under a diagonal Line from (0,100) to (100,0),
    clicking the Rect's centre reads a blue pixel but selects `element/children/1` (the Line). The
    editor then navigates to the Line.
  - **Why it matters:** the selection requirement says that selecting a rendered item selects its
    authored expression. The reference page says hit-testing uses conservative bounds, but not that
    a covering shape's box wins over the shape actually painted.
  - RF40 and RF43 enlarge the boxes involved.
- **Recommendation:** Hit-test with `isPointInPath`/`isPointInStroke` under each node's world
  transform, respecting clips and fill rules. Fall back to bounds only for text. Add a browser test
  with overlapping shapes.

- **Fix:** Canvas selection traverses reverse paint order with actual fill/stroke paths, transforms, alpha and ancestor clips; only text uses conservative ink bounds. Chromium verifies the Rect beneath a diagonal Line is selected.
- **Verification:** Verified. `hit-test.ts:5-42` follows the paint order and clip handling in `canvas.ts`. 24 Chromium cases pass at both DPR 1 and DPR 2:
  - Paint order: the Rect under the diagonal Line, the Line's stroke, and the empty part of the Line's box.
  - Fill rules and clips: evenodd holes, nonzero centres, a child clip hole, and outside an ancestor clip.
  - Invisible content: opacity-0 nodes and groups, and transparent fills.
  - Transforms: rotated and scaled shapes, and scaled strokes.
  - Text and dashes: text ink, and a dash versus its gap.

### ✅ Verified - RF50 With raised limits, Skia silently drops geometry that preparation accepted and reports as exactly visible, and destination errors carry no element.
- **Severity:** Low
- **Evidence:**
  - **Dropped geometry:** with `--limits {"coordinate":3.0e38}`, a Rect from -2e38 with size 2.5e38
    renders all-transparent. The manifest still reports exact visible bounds of 0..100. The same
    Rect at 1e30 paints.
  - **Dash failure:** four dashes of 1e38 pass preparation, then fail while painting with
    `"element":"","message":"invalid backend dash pattern","provenance":null`.
  - **Cause:** `prepare` only requires a finite, positive coordinate limit, and `draw_node` does not
    attach the node's ID or provenance to destination errors.
- **Recommendation:**
  - Cap `limits.coordinate` at a backend-safe maximum, or let the destination report its own cap.
  - Check that dash intervals sum to a finite value during preparation.
  - Attach the node ID and provenance in `draw_node`.

- **Fix:** Host coordinate limits may tighten the supported 1e7 cap but cannot raise it into unsafe Skia ranges. Dash totals are checked for backend representability. Destination errors are annotated with the active node ID/provenance. Documented and tested the cap.
- **Verification:** Verified.
  - `{"coordinate":3e38}` is now an error. `1e7` produces the default revision.
  - Four `1e38` dashes are rejected during preparation with `element/children/0` and provenance.
  - Within the cap, ±5e6 at `--scale 4000` paints every pixel, and tiny-viewBox magnification is rejected.
  - An external failing `Destination` returns errors annotated with the node ID and provenance (`lib.rs:255-288`).
  - The browser uses the same 1e7 cap.
  - The misleading limit-error wording is RF69.

### ✅ Verified - RF51 Circle with `r=0` and Ellipse with `rx=0` or `ry=0` still paint when stroked, unlike SVG and unlike Rect.
- **Severity:** Low
- **Evidence:**
  - **Code:** `crates/nx-graphics/src/lib.rs:750-763` always builds a `kurbo::Ellipse`, while
    `rounded_rect` returns an empty path for zero sizes (`:1025-1027`). SVG disables rendering for
    a zero radius, and the library documents both elements as corresponding to SVG.
  - **With an 8-wide round-cap stroke:**
    - `Ellipse rx=0 ry=30` paints a bar, with painted bounds 42..58 × 22..78.
    - `Circle r=0` paints a dot.
    - `Rect width=0` paints nothing.
- **Recommendation:** Produce an empty path when any radius is zero, and add a test.

- **Fix:** Circle/Ellipse generate empty paths when any radius is zero. Regression tests verify no visible bounds or stroked geometry.
- **Verification:** Verified. Zero radii produce empty paths (`lib.rs:819-823`). Stroked `Circle r=0`, `Ellipse rx=0` and `Ellipse ry=0` paint nothing and have no visible bounds. The regression test is `closed_moveto_caps_have_bounds_and_zero_radii_do_not_paint`.

### ✅ Verified - RF52 Arrow, RegularPolygon and Star lack the overflow pre-checks that Arc and RingSector gained in RF17, so finite but overflowing input is blamed on library source.
- **Severity:** Low
- **Evidence:**
  - **Arrow:** `Arrow x1=-1e308 x2=1e308` gives `nx-math-non-finite-input "hypot: arguments must be
    finite"`, located in `@nx/graphics/graphics.nx` bytes 21095..21118.
  - **RegularPolygon:** `RegularPolygon cx=1e308 radius=1e308` gives `graphics-non-finite` at
    `/children/0/points/0/x`, with provenance in `graphics.nx`.
  - **Contrast:** Arc and RingSector check `c ± radius` (`graphics.nx:406,446`) and name the
    helper.
- **Recommendation:** Add the same helper-named checks:
  - Arrow: `x2-x1`, `y2-y1`, and the endpoints ± the head and shaft extents.
  - RegularPolygon: `cx/cy ± radius`.
  - Star: `cx/cy ± outerRadius`.

- **Fix:** RegularPolygon/Star precheck center +/- radius; Arrow prechecks endpoint deltas, distance and outline extents before normalization. Overflow diagnostics name the helper. Added Rust IR regressions.
- **Verification:** Verified. Each probe gives `graphics-helper-invalid` naming the helper:
  - `Arrow x1=-1e308 x2=1e308`: "Arrow endpoint differences must be finite";
  - `RegularPolygon cx=radius=1e308`: "RegularPolygon center +/- radius coordinates must be finite";
  - `Star cx=outerRadius=1e308`: the same Star check;
  - `Arrow … headWidth=1e308`: "Arrow outline coordinates must be finite".

### ✅ Verified - RF53 Tests on the riskiest geometry paths are shallow.
- **Severity:** Low
- **Evidence:**
  - **Neutral tests:** the arc test in `crates/nx-graphics/src/lib.rs` only checks that a `CurveTo`
    and a `ClosePath` exist. None cover:
    - rotate/skew or rotation about a centre;
    - the alignment translation (only the scale is checked);
    - relative commands after `Z`, or S/T reflection;
    - arc flags or radius correction;
    - tiny radii (RF41), group visible bounds (RF43) or zero-length subpaths (RF44).
  - **SVG corpus:** `fixtures/svg-v1` has three samples.
  - **Helper tests:** `crates/nx-codegen/src/graphics_tests.rs` checks Arc and RingSector by counting
    `" A "` occurrences. Endpoints, clockwise direction, the 1e-5 deviation bound and the Arrow
    vertices other than its tip are untested.
- **Recommendation:** Add point-level assertions for each item, including one helper case that
  checks deviation against the stated tolerance.

- **Fix:** Added point-level tests for rotate-about-centre/skew, alignment translations for all fit modes, SVG close/relative/reflected controls, arc flags/correction/tiny radii and radial deviation, helper signed arc endpoints and every Arrow vertex. Bounds/limit regressions accompany their fixes.
- **Verification:** Verified. The new tests assert actual points:
  - tiny-arc radial deviation (21 samples per segment, within 5e-5);
  - `l` after `z`, S/T reflected controls, and the sweep and large-arc flags;
  - rotation about a centre and skewX;
  - end-alignment translation for each fit mode;
  - all 7 Arrow vertices, and signed Arc start, midpoint and end points.

  Remaining small gaps, not reopened: the helper sweep-flag token is not asserted, the Arrow zip does not assert the point count, the overflow test does not check the helper name, and the SVG corpus is still three samples.

### ✅ Verified - RF54 Preview artifact paths depend on the working directory, the committed font manifest cannot be used with `--fonts`, and failure IDs hash the raw arguments.
- **Severity:** Low
- **Evidence:**
  - **Image paths:** `preview.rs:172,188,199` record `--output` as given.
    - A revision first written from `/tmp/nxrev-b` with `--output cwd/out` keeps
      `"image":"cwd/out/<rev>/image.png"`.
    - When a later run from `/` reuses that revision folder, the path no longer resolves.
    - The committed fixtures rewrite this to `image.png`, so they don't show the real format.
  - **Font manifest:** `fixtures/fonts/manifest.json` is a single object with `file`. `--fonts`
    needs an array of `{family, weight, italic, path}`, so passing it fails with "font manifest
    must be an array".
  - **Failure IDs:** `preview.rs:208-210` hashes the raw argument list, including `--output` and
    the entry spelling, so the same failure gets different IDs from different directories.
- **Recommendation:**
  - Write paths relative to the output directory.
  - Use one font-manifest format, keeping `sha256` and `license` as extra fields.
  - Derive failure IDs from the normalized options.

- **Fix:** Manifests use image.png relative to their revision directory; current.json uses revision-relative paths; CLI responses use absolute paths. Unified supplied font manifests with the bundled array format and hash checks. Failure hashing sorts/normalizes options, includes defaults and source contents, and excludes destination/entry spelling.
- **Verification:** Reopened (Low). Fixed:
  - Manifests use `image.png`, `current.json` holds `<rev>/manifest.json`, and CLI responses are absolute.
  - A revision written from one directory resolves from another.
  - `--fonts crates/nx-graphics-skia/fixtures/fonts/manifest.json` works.
  - Failure IDs no longer change with the output directory, `main.nx` vs `./main.nx`, `--entry=` or number spelling, and they differ when source content differs.

  What remains is that `hash_sources` (`crates/nx-graphics-skia/src/bin/preview.rs:317-328,380-399`) hashes the `--source-root` path as given and each file's full path. The same failing source gets four IDs: `8ee78bf9…` with `--source-root .`, `10a6f96b…` with an absolute root or with none, `ca6facd1…` with `../b`, and `4e758f9f…` copied to another directory. Success revisions are already location-independent.

  Fix: canonicalize the source root and hash file paths relative to it.

- **Fix (second pass):** Canonicalized source roots and hash source names relative to the root, including the relative entry identity. Integration checks equal failure hashes for source-root spellings and copied source directories.
- **Verification (second pass):** Verified (second pass). One failing source gets the same ID (`78dfe603…`) with no root, `.`, `./`, an absolute root, `../b`, a symlinked root, an entry through a symlinked directory, and a copy in another directory. Changed content gives a different ID. Success revisions are unchanged across directories and spellings. A non-existent root fails with an OS error that doesn't name the path; this is pre-existing wording.

### ✅ Verified - RF55 The browser preview ignores devicePixelRatio and `scene.config.scale`.
- **Severity:** Low
- **Evidence:**
  - **Code:** `sites/playground/src/graphics/prepare.ts:172` hard-codes `scale: 1`.
    `canvas.ts:10` and `DrawingView.tsx:223` size the backing store at the logical width.
  - **Probe at DPR 2:** the backing store is 640×480 at a 640×480 CSS size, so the preview is
    upscaled and blurry on HiDPI displays. A scene with `scale ≠ 1` would paint at the wrong size.
- **Recommendation:** Prepare with `scale: devicePixelRatio`, size the backing store at
  `width*scale` with a root `ctx.scale`, and keep the logical CSS size. The layer limit already
  counts physical pixels.

- **Fix:** The page supplies devicePixelRatio to worker preparation. Frames, opacity layers and visible backing stores use physical pixels; transforms and CSS retain logical coordinates. Chromium DPR2 coverage asserts 1280x960 backing dimensions.
- **Verification:** Reopened (Medium). Up to DPR 3.5 this works:
  - Backing stores are 1280×960 at DPR 2 and 2240×1680 at DPR 3.5, with a 640 px CSS width.
  - Opacity layers are at physical size.
  - Scale flows through `usePreview.ts:20` → `session.ts` → `workbench.ts` → `prepare.ts`, and is part of the hashed config.

  What remains:
  - At DPR 4 and 5 (for example 200% zoom on a 2× display), `/play/graphics` shows "Drawing could not be rendered — surface limit exceeded". The scale is the uncapped `devicePixelRatio`, while `surface_pixels` is 4,194,304 (`sites/playground/src/graphics/prepare.ts:98`); 640×480×16 is 4,915,200.
  - The translucent-nesting budget also depends on DPR. 14 nested `opacity=0.9` groups render at DPR 1 and fail with "concurrent layer area limit exceeded" at DPR 2, and DPR 3 allows only 6.

  Fix: clamp the preparation scale to `min(devicePixelRatio, sqrt(surface_pixels/(width*height)))`, or scale the pixel limits with it. Add a DPR 4 browser case.

- **Fix (second pass):** Clamp device raster scale to the surface budget and lower it further, down to scale1, when concurrent opacity layers exceed their physical budget. Logical geometry is unchanged. Chromium now checks DPR4 and 14 nested opacity groups.
- **Verification (second pass):** Verified (second pass). `/play/graphics` renders at DPR 4 and 5. The scale is clamped to 3.694 (a 2364×1773 backing store), and the CSS width stays 640 at every DPR. The opacity preflight lowers the scale to fit: 14 nested opacity groups render at DPR 2 (1.5), DPR 3 (1.6875) and DPR 4 (1.559), and 47 groups render at scale 1. Logical geometry and hit-testing are unchanged at reduced scales. The final scale is in the revision (47 groups give the same revision at DPR 2, 3 and 4). Integer scales stay crisp; reduced fractional scales are smoothly upscaled, which is documented. Minor, not reopened: the scale shrinks in ×0.75 steps rather than to the largest scale that fits (1.5 where about 1.97 would fit).

### ✅ Verified - RF56 The SDK's typed graphics DTOs and diagnostics disagree with the payload.
- **Severity:** Low
- **Evidence:**
  - **Accessibility type:** `bindings/wasm/src/graphics.ts:95` types `accessibility` as
    `{label?}`. The payload carries `{"$type":"Accessibility","description":…,"label":…}`, and
    the canvas exposes only `aria-label`, so the description is lost.
  - **Element names:** diagnostic text names elements by JSON pointer (`… (/children/1)`), while
    node IDs, text requests and the inspector use `element/children/1`.
- **Recommendation:** Type, or project, `{label, description}`, and expose the description, for
  example as `aria-description`. Use one element-ID form in diagnostics.

- **Fix:** SDK accessibility includes description, exposed through aria-description on Canvas. Graphics diagnostics use public element-prefixed scene IDs while provenance lookup retains internal JSON-pointer paths.
- **Verification:** Verified. The canvas carries `aria-label` and `aria-description`, and the SDK type is `{label?, description?}` (`bindings/wasm/src/graphics.ts:95`). Opacity, glyph, font and path diagnostics all name `element/…` IDs that match the inspector's options (`crates/nx-graphics/src/lib.rs:24-35,568-573`).

### ✅ Verified - RF57 Unused code paths remain from earlier preview designs.
- **Severity:** Low
- **Evidence:**
  - **Worker request:** nothing in `src/` sends the `"graphics"` request kind
    (`sites/playground/src/worker/protocol.ts:20`, `worker/session.ts:77`).
    `prepareDrawing`, which compiles and evaluates a second time, is reached only through that
    branch and `graphics.test.mjs`. The production path is `PreviewWorkbench.render` →
    `prepareDrawingValue`.
  - **Output fallback:** the `session.selectionTargets ?? session.built.flatMap(...)` fallback in
    `PreviewView.tsx` `SessionOutput` is never used, because the workbench always sends
    `selectionTargets`.
  - This is the same kind of leftover that RF28 removed.
- **Recommendation:** Remove the request kind and its branch, and point the tests at
  `prepareDrawingValue`. Make `selectionTargets` required and drop the fallback.

- **Fix:** Removed the unused graphics worker request and duplicate production compile/evaluate preparer. Tests exercise prepareDrawingValue with IR setup. Selection targets are required; SessionOutput's dead fallback is removed.
- **Verification:** Verified. The worker protocol has only `evaluate`, `language` and `preview` (`worker/protocol.ts:18-22`), and the session has no graphics branch. `prepareDrawing` survives only as a test helper over `prepareDrawingValue`. `selectionTargets` is required (`preview/protocol.ts:151,168`), and `SessionOutput` reads it directly (`PreviewView.tsx:383`).

### ✅ Verified - RF58 Every dispatch, goTo and compiling edit briefly shows the "runs the last program that compiled" notice, greys values out, disables actions and re-reveals the selection.
- **Severity:** Low
- **Evidence:**
  - **Code:** `sites/playground/src/output/evaluation.ts:31-34` sets `stale` whenever a request is
    pending or a debounce is outstanding. `App.tsx:197` passes that to Preview
    (`PreviewView.tsx:161`). While stale, `SessionOutput` clears the highlight, then re-applies it
    with `reveal=true` on every answer.
  - **Observed on `/play/counter`:** a MutationObserver records the notice and `stale: true` for each
    "Add one" click and for typing a valid `// ok` line. At HEAD, stale was shown only when an edit
    failed to compile.
- **Recommendation:** Derive staleness only from a source mismatch or a failure, and show pending
  work as a separate quiet indicator. Don't reveal again when only the session value changed.

- **Fix:** Pending requests are reported separately from staleness; only mismatched sources and failures mark retained output stale. Session value updates retain selection without revealing again unless the authored selection changes.
- **Verification:** Reopened (Low). Dispatch and goTo are fixed: in a MutationObserver probe, "Add one" twice and "Back a step" never show the notice or `stale`. `selectNodes` reveals only when the authored selection moves. A compile error still ends stale with the notice.

  What remains is that a compiling edit still flickers. One space keystroke shows the notice, greys all 3 `nx-value`s and disables the action buttons from +32 ms to +297 ms. `App.tsx:198-201` (`source !== preview.state?.source`) and `output/evaluation.ts:31-34` (`state?.source !== source`) treat a debounced or pending edit as stale, and `src/evaluation.test.mjs:31` asserts that behaviour.

  Fix: mark stale only when the answer for the asked source came back without updating `state.source`, or on failure. Show pending work separately, and change the test.

- **Fix (second pass):** Stale styling now waits for an answered failed edit or an explicit failure; debounce/pending work has a quiet updating indicator. Source-snapshot navigation guards remain separate. Added a MutationObserver regression proving a valid edit does not flash stale.
- **Verification (second pass):** Reopened (Low, second pass). The valid-edit flash is fixed: a space, `// ok`, Add one and Back a step show no stale notice, no greyed values and no disabled actions, only a quiet "updating…". A compile error still ends stale. Selection reveals only on authored changes, and a navigate during the debounce after a span-shifting edit selects nothing.

  What remains is the inverse case. Once the document is already broken, each further keystroke clears the stale styling until its answer returns: at +15 ms the notice disappears, stale values go 3 → 0 and disabled actions go 3 → 0, and all return at +283 ms. For about 270 ms per keystroke, actions are enabled on a session running an older program. The cause is `stale: (!evaluating && state?.source !== source) || …` in `sites/playground/src/output/evaluation.ts:36-39`, and no test covers this case.

  Fix: base staleness on the last answered source rather than on pending work, so that a broken document stays stale while its next edit is pending. Add a model test for broken → keystroke.

- **Fix (third pass):** Staleness now follows the last answered diagnostics/source pair and explicit failures, independently of pending work. An already-failed edit remains stale while typing; a successful document stays live during debounce. Added model coverage and Chromium MutationObserver regressions for both directions.
- **Verification (third pass):** Verified (third pass). Staleness follows the last answered `diagnosticsSource` against `state.source`, or an explicit failure, and not pending work (`sites/playground/src/output/evaluation.ts`). Timed MutationObserver probes:
  - Valid edits (a space, `// ok`, and a valid edit after a fix) show only the quiet "updating…" note, with no notice, greyed values or disabled actions.
  - A broken document (`let = `) turns stale when its answer arrives. It then stays stale through further typing for the whole pending window, with the notice, 3 greyed values and 3 disabled actions. The inverse flicker is gone.
  - Removing the error goes live on its answer.
  - Component actions stay live, and selection reveals only on authored changes. A navigate during the debounce after a span-shifting edit selects nothing.
  - Runtime-notice and root-props failures stay stale while pending.
  - The RF26 and RF30 start-failure behaviour and labels are unchanged.

### ✅ Verified - RF59 A value that evaluates but cannot be formatted fails the whole answer, losing Preview, and the WASM JSON depth is below the graphics depth limit.
- **Severity:** Low
- **Evidence:**
  - **Code:** `sites/playground/src/preview/workbench.ts:370` calls `formatValue` outside any `try`.
    `#view` (`:624-644`) already turns the same failure into an "output has no NX spelling"
    diagnostic.
  - **Probe:** a Drawing with 63 nested `<g.Group>` elements, through `createNxSession`, gives
    `renderer=null`, `problem=programFailed` and "Format request is not valid: recursion limit
    exceeded". The value itself evaluated, and the graphics node-depth limit (64) is never reached.
- **Recommendation:**
  - Catch format failures in `#start` and keep the structured value and renderer, with the
    formatting diagnostic in Output, so graphics preparation can report its own limit.
  - Align the WASM JSON nesting limit with the graphics depth limit, or lower the browser's graphics
    depth to one that fits.

- **Fix:** Static formatting failures retain the evaluated value and selected renderer, surfacing a raw-Output diagnostic. Browser depth is explicitly 48, with a pre-bridge JSON nesting guard, within the bounded SDK transport. Added formatter-failure and 63-group regressions.
- **Verification:** Reopened (Low). The workbench part is fixed: through `createNxSession`, Drawings with 63, 64 and 200 nested Groups keep `renderer=graphics` and `problem=null`. Output carries the formatting diagnostic, and Preview keeps its own diagnostic. With 48-53 Groups, Preview reports "node or depth limit exceeded (element/children/…)".

  What remains, for the finding's own 63-Group case:
  - Preview shows the generic "The drawing value exceeds the browser preview limits.", with no element or source span. This comes from the pre-bridge guard `depth > 110` (`sites/playground/src/graphics/prepare.ts:29`), because each Group adds two JSON levels. The graphics depth diagnostic is never reached for 54 or more Groups. `routing.test.mjs:502` matches `/limits|depth/` and so accepts the generic message.
  - Output labels the formatting failure "Evaluating root failed" (`OutputPane.tsx:120-129`, no code), though evaluation succeeded. Its position, "line 1 column 3980", is inside the internal format request. `SessionOutput` labels the same failure "The output has no NX spelling".

  Fix: when the JSON guard trips, report the graphics depth diagnostic with the first over-deep element, or check scene depth before the guard; then make the test match depth. Give format failures their own code and heading, and drop the internal request position.

- **Fix (second pass):** An iterative scene-depth/node guard runs before JSON serialization, returning the failing public element ID and authored provenance. Tightened the 63-group test to require a depth error and source span. Formatting errors have nx-output-format and their own display heading; serde recursion wording becomes a clear display-depth message.
- **Verification (second pass):** Reopened (Low, second pass). The depth part and the Output label are fixed:
  - 47 nested Groups render. 48, 49, 54, 63 and 200 give "node or depth limit exceeded" naming the first node at depth 49, with a source span, from `sceneLimit` (`prepare.ts:55-100`), which runs before the JSON guard.
  - The node limit fails at `element/children/1999` with a span.
  - Output for 63 and 200 Groups shows `nx-output-format` under "The value could not be displayed" ("too deeply nested to display as NX text"), with no internal "line 1 column N".

  What remains: for a component whose output nests 300 `<Row>`s, Preview shows the heading "The output has no NX spelling" while Output shows "The value could not be displayed" for the same failure. `PreviewView.tsx:398-403` uses that heading for every diagnostic and ignores `nx-output-format`.

  Fix: choose the heading by code in `SessionOutput`, as `OutputPane` does.

- **Fix (third pass):** Raw Output and SessionOutput now call the same outputErrorHeading classifier. Formatting failures consistently say The value could not be displayed, including component sessions with 300 nested Rows. Added the shared classifier test and a two-tab Chromium regression.
- **Verification (third pass):** Verified (third pass). `outputErrorHeading` (`evaluation.ts:47-56`) is shared by `OutputPane.tsx:134` and `PreviewView.tsx:402`. In Chromium, a component whose output nests 300 Rows shows "The value could not be displayed" ("too deeply nested to display as NX text") in both tabs. The static graphics cases are unchanged:
  - 47 Groups render.
  - 48 Groups give the depth error with span 2:494.
  - 63 and 200 Groups keep the graphics renderer with that depth diagnostic, and Output shows `nx-output-format` without an internal position.

### ✅ Verified - RF60 An `ElementVisualizer` on an element function also reports a misleading "write Target=<type>" error.
- **Severity:** Low
- **Evidence:**
  - **Cause:** since RF36, `target_type` is `None` for element functions, so `infer.rs` (about lines
    1579-1590) binds nothing, even when the value is an `ElementVisualizer`.
  - **Probe:** `let <Badge text:string/> = { <B/> }` with `visualizer Badge = <ElementVisualizer
    icon="x"/>` reports `type-parameter-not-specified … write Target=<type>` in addition to
    `visualizer-record-mismatch`.
  - **Contrast:** `FunctionVisualizer` on a record gives one error.
  - **Test:** `visualizer_declarations_tests.rs:798-802` uses `assert_error`, so it doesn't check
    that only one error is reported.
- **Recommendation:** Bind `Target` to the error type for function targets, as unknown targets do,
  so that only the mismatch is reported. Assert a single diagnostic.

- **Fix:** Element functions carry an error Target binding when inferring a mismatched visualizer, suppressing the secondary missing-Target error. The regression asserts exactly one record-mismatch diagnostic.
- **Verification:** Verified. Element-function targets bind `Target` to the error type when the value is mismatched (`crates/nx-types/src/infer.rs`), so `let <Star count:int/> = <Panel/>` with `visualizer Star = <ElementVisualizer icon="star"/>` reports exactly one `visualizer-record-mismatch` (`visualizer_declarations_tests.rs:798-805`). `cargo test -p nx-api visualizer` passes (43).

### ✅ Verified - RF61 With `includeUnreferencedDeclarations`, a host visualizer module adds targets from modules the program never loads, without the library's own metadata.
- **Severity:** Low
- **Evidence:**
  - **Code:** `ProgramVisualizers::targets` (`crates/nx-language-service/src/visualizers.rs:114-121`)
    adds `self.host.keys()` without the `loaded_modules` filter that the RF31 derived-kind branch
    applies.
  - **Probe:** `main.nx` does not import graphics, and the host module imports it and overrides
    `g.Drawing` and `g.Rect`. The opt-in table then lists:
    - `@nx/graphics/graphics.nx::Drawing {"icon":"host-frame"}`, without the library's
      `nx.graphics` preview;
    - `Rect` with `host.rect`;
    - Rect's bases, and `ViewBox`, `Paint` and `Stroke`.
  - The source-tree delta promises "loaded visualizer targets … with the same resolved
    inheritance/override metadata".
- **Recommendation:** Filter every target by `loaded_modules`, including host keys. Extend the RF31
  regression test with this case.

- **Fix:** All opt-in visualizer targets, including host overrides, are filtered by loaded program modules. Added an unloaded graphics host-module regression.
- **Verification:** Verified. `ProgramVisualizers::targets` filters program and host keys by `loaded_modules` (`crates/nx-language-service/src/visualizers.rs:114-121`). `unreferenced_host_visualizers_do_not_add_unloaded_library_targets` asserts that no `@nx/graphics/` declaration appears when only the host module imports graphics. `cargo test -p nx-language-service` passes (306).

### ✅ Verified - RF62 Several docs and change artifacts are stale or conflict with a parallel change.
- **Severity:** Low
- **Evidence:**
  - **Release notes:** `release-notes.md` says "The existing component Preview and legacy HIR
    Output paths remain available". The playground now has no HIR path, and the README says so.
  - **tasks.md:** line 71 still says "All 38 tasks are complete" (there are 47).
  - **Language protocol README:** `packages/language-protocol/README.md:197-201` documents neither
    `preview` nor `includeUnreferencedDeclarations`.
  - **Concepts page:** the description of
    `sites/website/src/content/docs/reference/concepts/visualizer-declarations.md` omits element
    functions, and its "What a reading shows" section omits `preview`.
  - **Graphics reference:** "provenance identifies the library's constructing element, since the
    current origin report does not retain the author's complete call chain" is now true only of the
    native Rust preview. The browser uses TypeScript call-site origins (task 7.6, RF18).
  - **Source-tree delta overlap:** this change and `add-visualizer-icon-expressions` both MODIFY
    "The answer carries resolved visualizer declarations when asked". The other copy still says an
    element function "a visualizer cannot target" and lacks `preview` and the opt-in flag.
    Archiving either one after the other silently drops text. Strict validation cannot catch this.
- **Recommendation:**
  - Correct the release note, tasks.md, the protocol README, the concepts page and the graphics
    reference sentence.
  - Record the delta overlap in implementation-notes.md, so that the icon-expressions delta is
    rebased onto the archived text.

- **Fix:** Updated release notes, task checkpoint wording, protocol/concepts/graphics docs and browser provenance description. Recorded the overlapping icon-expressions delta and required archive-time rebase without editing that separate change.
- **Verification:** Verified.
  - The release note no longer mentions an HIR Output path.
  - tasks.md describes the 38-task checkpoint as historical and states that all 47 tasks are complete.
  - The protocol README documents `preview` and `includeUnreferencedDeclarations`.
  - The concepts page's description includes element-style functions, and its reading section mentions the preview descriptor.
  - The graphics reference distinguishes native provenance from browser call sites.
  - implementation-notes.md records the icon-expressions delta overlap and the archive-time rebase.

### ✅ Verified - RF63 Dense formatting is back in new tests and the shipped helper source.
- **Severity:** Low
- **Evidence:**
  - **Tests:** `sites/playground/src/preview/workbench.test.mjs:340-357` (the two survey tests) are
    single-line statement blocks, unlike the rest of the file. This repeats what RF24 flagged.
  - **Helper source:** in `crates/nx-api/src/std/graphics/graphics.nx`, the helper bodies put the
    validated branch at the wrong indentation inside `else => {` (lines 242-246, 274-281, 314-316,
    407-411 and 447-465). Lines 244, 276, 280, 315, 338 and 453 run to roughly 150-330 characters.
  - **Why it matters:** this is public standard-library source that readers reach through
    go-to-definition and the docs.
- **Recommendation:** Reformat the survey tests to match the file. Indent the helper bodies
  consistently, and wrap the long `Polygon`/`Path` invocations one property per line, as the
  declarations above them already are.

- **Fix:** Expanded the two survey tests and reformatted helper bodies, nested branches and element property lists; retained the user-preferred aligned trailing property docs and periods.
- **Verification:** Verified. The survey tests in `workbench.test.mjs:340-365` are multi-line. The `graphics.nx` helper bodies are consistently indented, list Polygon/Path properties one per line, and have no non-doc line over 112 characters (previously up to 613). The aligned trailing `///` docs are kept. Minor, not reopened: `else` placement is mixed in places. New dense code from the fixes is RF71.

### Questions (this pass)
- RF49: should canvas selection follow painted geometry, as recommended? If bounding-box selection
  is intended, the reference should say that a covering shape's box wins.
- RF55: is HiDPI-sharp rendering expected in this profile? If not, record it as deferred.

### Summary (this pass)
- **New:** 27 findings, RF37-RF63.
  - **High (1):** RF37, the website code-block check fails CI.
  - **Medium (12):** RF38 (a worker failure drops Output and the session), RF39 (the example gate
    passes a missing root), RF40 (browser text metrics), RF41 (tiny arc radii), RF42 (arc
    expansion after the limit check), RF43 (group bounds marked exact), RF44 (cap dot from
    zero-length subpaths), RF45 (stale walkthrough fixtures), RF46 (CI gaps), RF47 (preview options
    ignored silently), RF48 (measurement diagnostics), RF49 (bounding-box hit-testing).
  - **Low (14):** RF50-RF63.
- No RF1-RF36 fix regressed, and the statuses of RF1-RF36 are unchanged.
- The core architecture held up in probes:
  - identity-checked preparation;
  - the IR-only runtimes;
  - the shared value for Output and Preview;
  - library-owned renderer metadata;
  - Canvas and Skia compositing.

  The defects are at the edges: numeric extremes, bounds, failure paths, CI coverage and the CLI
  contract.


## New Findings Discovered During 2026-10-29 12:12 Verification

The same reviewer session (`2e3fd5ea-4d4d-4afd-ac2d-833e53b95385`) found these while verifying
RF37-RF63. RF38, RF40, RF54, RF55, RF58 and RF59 were reopened above and are not repeated here.

### ✅ Verified - RF64 A fill-only path with an isolated `M…Z` reports painted and visible bounds it does not paint, marked exact.
- **Severity:** Low
- **Evidence:**
  - **Cause:** the RF44 change adds every closed-moveto point to the geometry bounds
    (`crates/nx-graphics/src/lib.rs:471-482`), and for an unstroked shape the painted bounds equal
    the geometry bounds.
  - **Probe:** `<g.Path data="M0 0 H10 V10 H0 Z M90 90 Z" fill=black/>` in a 100-unit viewBox at
    200 px. The manifest reports painted and visible 0..180 × 0..180 with `conservative: false`.
    Skia paints only device pixels 0..19 (400 dark pixels).
  - **Impact:** a wrapping Group inherits the same exact over-report, and so does painted-geometry
    selection fallback for text-free scenes.
- **Recommendation:** Add cap-dot points only to stroke-painted bounds, and only for round or square
  caps. Otherwise mark the result conservative. Add the fill-only case as a test.

- **Fix (second pass):** Intrinsic/fill bounds exclude isolated closed movetos; only round/square stroke bounds include cap-dot points. Added the exact fill-only mixed-path regression while retaining RF44 stroke coverage.
- **Verification (second pass):** Verified (second pass). In the preview, `M0 0 H10 V10 H0 Z M90 90 Z` with fill gives painted and visible 0..20 × 0..20 (exact). That is only the square; the isolated `M90 90 Z` is excluded. A round-cap `M30 50 Z` still includes its dot (conservative), and a butt-cap `M60 50 Z` is empty. `bounds()` no longer adds movetos, `stroke_bounds` adds them only for round and square caps (`crates/nx-graphics/src/lib.rs:462-487`), and `isolated_closed_movetos_only_contribute_to_cap_strokes` covers a group around the fill-only case.

### ✅ Verified - RF65 Path budget failures are reported as "malformed SVG path".
- **Severity:** Low
- **Evidence:**
  - **Cause:** the RF42 fix raises limit errors inside `normalize_svg` (`lib.rs:1547,1564`).
    `svg()` prefixes every `normalize_svg` error with "malformed SVG path:" (`:407-408`), so the
    unprefixed "normalized path command limit exceeded" at `:410` can no longer be reached.
  - **Effect:** a valid path that only exceeds the budget is reported as "malformed SVG path:
    normalized path command limit exceeded", which tells an author to fix the syntax.
- **Recommendation:** Return a typed error from `normalize_svg`, or pass limit errors through
  without the prefix. Assert the message in the budget regression test.

- **Fix (second pass):** SVG normalization returns typed syntax/limit failures; resource limits no longer acquire a malformed-path prefix. The regression asserts the exact budget message.
- **Verification (second pass):** Verified (second pass). `normalize_svg` returns `SvgError::{Syntax, Limit}` (`lib.rs:1465-1480`). Syntax errors keep "malformed SVG path: …" (for example `M0 0 Lx` gives "malformed SVG path: invalid number at position 7"). A valid arc over budget gives exactly "normalized path command limit exceeded" (preview probe with `path_commands=10`), and the regression asserts that message.

### ✅ Verified - RF66 The preview CLI's option tests never run in CI.
- **Severity:** Low
- **Evidence:**
  - `options_are_strict_and_support_inline_values` lives in the `preview`-gated binary.
  - Neither CI test step compiles it:
    - `.github/workflows/graphics-native.yml:26` tests with only `nx-graphics-skia/native`.
    - `build.yml` tests without features.
  - Clippy at `:37` builds it but does not run it.
  - The Python checker covers only an unknown option, a missing value and `--unknown`. Duplicates
    and `--name=value` taking effect are unchecked.
- **Recommendation:** Add `--features preview` (or `--bins`) to the native test step, or move the
  option parser into a library module that the native tests cover.

- **Fix (second pass):** Native CI tests with the preview feature enabled, executing the CLI unit tests as well as native renderer tests.
- **Verification (second pass):** Verified (second pass). `graphics-native.yml:27` runs `cargo test --locked -p nx-graphics -p nx-graphics-skia --features nx-graphics-skia/preview`, which compiles and runs the binary's option test (1 passed) alongside 17 neutral and 8 native tests.

### ✅ Verified - RF67 Array-form JSON bypasses the strict named-key checks for limits, font faces and measurement requests.
- **Severity:** Low
- **Evidence:** serde's derived struct deserializers also accept a sequence, and
  `deny_unknown_fields` does not apply to that form. The following are all accepted:
  - `--limits` with `[1]` sets `nodes=1`, and `[]` keeps all defaults.
  - Font faces given as positional arrays.
  - `--measure-text '["a","Fixture",18,400,false]'`.

  RF47 and RF48 made these inputs strict precisely so that a mistyped request cannot silently mean
  something else.
- **Recommendation:** Require JSON objects, for example by deserializing through `serde_json::Map`
  first or with a custom `visit_seq` that errors. Add negative tests.

- **Fix (second pass):** CLI named inputs are checked as JSON objects before typed deserialization; font faces are likewise checked individually. Added unit/integration negatives for positional limits, measurements and faces.
- **Verification (second pass):** Verified (second pass). `named_object` (`preview.rs:82-90`) rejects non-object input. Limits given as `[]`, `[1]`, `"x"` or `null`, a positional `--measure-text` array, and array or mixed font faces are all rejected with messages naming the input or face index. Nested positional fields such as `{"nodes":[5]}` and `"weight":[400]` are type errors. The WASM `Request` still uses plain serde, but its config is built by the playground rather than by users; this is outside the CLI scope of the finding and is not reopened.

### ✅ Verified - RF68 The new CI triggers miss build inputs, and deploys are no longer serialized across branches.
- **Severity:** Low
- **Evidence:**
  - **Browser check filter:** the new `pull_request` path filter in
    `.github/workflows/deploy-playground.yml:31-40` leaves out several files the push filter
    includes:
    - `Cargo.toml`, `Cargo.lock` and `rust-toolchain.toml`;
    - `package.json` and `pnpm-workspace.yaml`;
    - `scripts/fetch-wasi-sysroot.mjs` and `.github/actions/setup-rust-node/**`.

    A lockfile or toolchain bump therefore skips the Chromium check on its PR.
  - **Native filter:** `graphics-native.yml` leaves out `rust-toolchain.toml`, although it runs
    Clippy with `-D warnings`.
  - **Concurrency:** the group changed from `deploy-playground` to
    `deploy-playground-${{ github.event.pull_request.number || github.ref }}` (`:47`). A
    `workflow_dispatch` from a non-main branch still deploys, but now runs alongside a main deploy
    instead of queueing behind it.
- **Recommendation:**
  - Make the PR filter match the push filter, and add `rust-toolchain.toml` to the native lane.
  - Keep one fixed concurrency group for the deploy job, for example a job-level
    `concurrency: deploy-playground`. Use the per-PR group only for validation.

- **Fix (second pass):** PR build filters match push inputs, native checks include rust-toolchain.toml, and all non-PR deployment runs share one production concurrency group across refs.
- **Verification (second pass):** Verified (second pass). The `pull_request` and `push` path lists in `deploy-playground.yml` are identical, and `graphics-native.yml` includes `rust-toolchain.toml`. Concurrency is `deploy-playground-${{ github.event_name == 'pull_request' && github.event.pull_request.number || 'production' }}`, so every non-PR run (main or a manual dispatch) shares one production group, and PRs validate in their own groups. The deploy job remains PR-excluded. Both files parse with PyYAML.

### ✅ Verified - RF69 Host-limit errors are misleading.
- **Severity:** Low
- **Evidence:**
  - **Wrong error:** the viewport check (`crates/nx-graphics/src/lib.rs:526-532`) runs before limit
    validation, so `{"coordinate":0}`, `-1` or `100` report "number outside host range" instead of
    an invalid-limits error.
  - **Vague error:** `{"coordinate":3e38}` gets a generic "invalid host limits" that names neither
    the field nor the supported 1e7 maximum.
  - **Docs:** they say raising the cap "is unsupported", not that it is an error.
  - **Dead check:** the dash-total check (`:929-932`) can no longer trigger under the 1e7 cap.
- **Recommendation:** Validate limits first, and name the field and its allowed range. Document
  that values above 1e7 are rejected. Drop or comment the unreachable dash-total check.

- **Fix (second pass):** Host limits validate before viewport numbers, with field-specific finite/range messages and an explicit coordinate cap. Viewport conflicts name the coordinate constraint. Documented rejection above1e7 and explained the defensive future-profile dash check.
- **Verification (second pass):** Verified (second pass). Limits are validated before viewport numbers (`crates/nx-graphics/src/lib.rs:531-561`). `coordinate` values 0, -1, 3e38 and 10000000.000001 give "invalid host limits: coordinate must be finite, positive and at most 10000000", and `surface_pixels`/`layer_pixels` errors name their field. A viewport that exceeds a tight coordinate limit says so. The docs state that values above 1e7 are rejected, and the dash-total guard is commented. Minor, not reopened: integer limits such as `{"nodes":0}` are not range-checked and fail later as "node or depth limit exceeded".

### ✅ Verified - RF70 After the worker is replaced, the visitor's picked component is lost on the next edit.
- **Severity:** Low
- **Evidence:**
  - **Before RF38:** `failed` nulled `state`, so the next call was `start` with
    `component: model.picked`.
  - **Now:** `state` is kept, so `callFor` sends `{op:"reload"}`
    (`sites/playground/src/preview/model.ts:107`). A replaced worker after a TimeoutError or
    WorkerStoppedError has a fresh workbench with no component, so the reload starts the default
    root component while `model.picked` still names the visitor's choice.
  - **Probe:** with components A and B and root `<A/>`, pick B, force a worker replacement, then
    edit. The worker runs A.
- **Recommendation:** Send the picked component with `reload` as a fallback for a worker with no
  session. Alternatively, when a reload is answered without a session, retry as `start` with
  `picked`. Add a model test.

- **Fix (second pass):** Reload calls carry the picked component as a fallback. A fresh workbench with no session honors that fallback; an existing session keeps its state. Added model and replacement-workbench regressions.
- **Verification (second pass):** Verified (second pass). After picking B, replacing the worker and editing, the call is `{op:"reload", component:"B"}` and B runs. When a live session already runs A while `picked` still names B, later reloads (even after B is re-added) keep A's session and steps (2 → 3 → 4).

### ✅ Verified - RF71 The fix code and tests reintroduce dense, brace-less formatting.
- **Severity:** Low
- **Evidence:**
  - **workbench.ts:**
    - `sites/playground/src/preview/workbench.ts:377` is a brace-less `if (…) throw …`. It is
      unreachable, because `valueOutcome` always returns a value.
    - Lines 379 and 387-388 pack arguments and fields onto one line.
    - Line 681 is another brace-less `if`.
  - **Tests:** `routing.test.mjs:478-501` and `src/evaluation.test.mjs:8-11,24-28` use one-line
    blocks, for example `} finally { workbench.dispose(); host.dispose(); }`.
  - This is the pattern RF24 and RF63 fixed. The repository conventions require braces and the
    surrounding expanded style.
- **Recommendation:** Expand these blocks to match the surrounding files, and remove the
  unreachable throw.

- **Fix (second pass):** Expanded workbench branches and fields, removed unreachable formatting throws by narrowing valueOutcome's return type, and formatted new TypeScript/tests to the surrounding expanded style. Braces are present on the new conditional blocks.
- **Verification (second pass):** Verified (second pass). `workbench.ts:376-395` and `684-686` are expanded with braces, and the unreachable throw is removed (`valueOutcome` returns `Extract<Outcome, {kind:"value"}>`, `compile/evaluate.ts:83`). The second-pass blocks in `model.ts`, `OutputPane.tsx`, `evaluation.test.mjs`, `routing.test.mjs:478-510` and the browser observer are multi-line. Minor leftovers, not reopened: `workbench.ts:711` keeps an always-true `formatted.kind === "value"` check, and the browser check's server loop (`check-graphics-browser.mjs:43,45`) has older brace-less lines.

### Summary (2026-10-29 12:12 verification)
- **Verified (21):** RF37, RF39, RF41, RF42, RF43, RF44, RF45, RF46, RF47, RF48, RF49, RF50, RF51,
  RF52, RF53, RF56, RF57, RF60, RF61, RF62, RF63.
- **Reopened (6):**
  - RF38 (Medium): a failing newly opened document keeps the previous document's state.
  - RF40 (Low): the ink pad is smaller than device antialiasing.
  - RF54 (Low): failure IDs depend on the source-root spelling.
  - RF55 (Medium): DPR 4 or more exceeds the surface limit.
  - RF58 (Low): compiling edits still flash stale.
  - RF59 (Low): the generic depth-guard message, and the "Evaluating root failed" label.
- **New (8):** RF64-RF71, all Low.
- **Validation re-run in this pass:**
  - Rust: `cargo test -p nx-graphics` (16), `-p nx-codegen graphics` (4),
    `-p nx-graphics-skia --features native` (8) and the preview binary test (1); `-p nx-api
    visualizer` (43) and `-p nx-language-service` (306).
  - Checks and lint: `check-graphics-preview.py` with fixture comparison and schema validation,
    `verify-graphics-fixtures.mjs`, native Clippy with `-D warnings`, and `cargo fmt --check`.
  - Playground and site: typecheck, 162 tests and 25 examples, `test:graphics-browser`, the WASM
    graphics tests, and the website code blocks (223 blocks, 0 failures).
- Probes were temporary scratch crates, NX sources, Node and Playwright scripts under `/tmp/nxver-*`,
  each run with an explicit timeout. No implementation file or existing test was changed, and no
  temporary file was left in the repository.
- RF1-RF36 statuses are unchanged.


## New Findings Discovered During 2026-10-29 12:34 Verification

The same reviewer session (`2e3fd5ea-4d4d-4afd-ac2d-833e53b95385`) ran this second verification of
RF38, RF40, RF54, RF55, RF58, RF59 and RF64-RF71.

### ✅ Verified - RF72 A component-action answer that arrives after a reset start can restore the previous document's state.
- **Severity:** Low
- **Evidence:**
  - **Cause:** the `answered` branch of `reducePreview`
    (`sites/playground/src/preview/model.ts:66-83`) drops outdated answers only for `start` and
    `reload`. A `dispatch` or `goTo` answer that lands after a reset `start` was asked replaces the
    state that the RF38 second-pass fix just cleared. It also clears `pending`.
  - **Model sequence:**
    1. An action is asked.
    2. A reset start is asked; the state is now null.
    3. The action is answered; the state is back to the counter at Count: 5, with `pending` cleared.
    4. The reset fails.

    Output then shows the counter's value for the new document, which is the RF27/RF38 symptom.
  - **Browser:** three Chromium attempts did not trigger it, because decoding the `#code=` link
    delays the reset until the action has answered. The window is real in the model and depends on
    timing.
- **Recommendation:** While a reset `start` is outstanding, drop `dispatch` and `goTo` answers,
  for example when `model.asked !== call.source` or a reset is pending. Add a model test for this
  sequence.

- **Fix (third pass):** Document-reset epochs are recorded in the page model and in asynchronous request closures. Replies/failures from prior epochs are ignored, including after the reset fails or succeeds; stale action replies cannot trigger sessionLost recovery. Added the full action/reset/late-reply/failure sequence test.
- **Verification (third pass):** Verified (third pass). `PreviewModel.epoch` and the synchronous ref in `usePreview.ts:53-92` increment only on a reset `start`. Success closures from an earlier epoch return before `answered` and the `sessionLost` restart. Failure events carry their epoch, and the reducer drops mismatches (`model.ts:61-73`).
  - **Model sequence:** action asked → reset asked → late action answered is dropped, and the reset stays pending. Both a failed and a successful reset then show only the new document, and a late epoch-1 failure is dropped. Actions and timeline moves in the new epoch are accepted.
  - **Through the real hook in Chromium:** the worker's "Add one" answer was delayed 2.5 s while navigating to a failing `#code=` link, and again to `/play/graphics`. Both show no counter state, and later actions are accepted.
  - **Other flows:** Back/Forward, RF38 and RF70 re-pass.
  - **Coverage limits, not reopened:** `sessionLost` was checked by reading the code only, and `usePreview` has no unit test of its own; the model regression covers the reducer.

### ✅ Verified - RF73 The browser's pre-bridge value budget rejects very large point lists without an element or source location.
- **Severity:** Low
- **Evidence:**
  - **Cause:** `checkGraphicsValue` (`sites/playground/src/graphics/prepare.ts:23-28`) counts
    1,000,000 JSON values before graphics preparation, and on overflow throws a generic error.
  - **Contrast:**
    - One Polyline with 60,000 points reaches preparation and gets "scene command limit exceeded
      (element/children/0)" with span 4:75.
    - Two Polygons with 99,999 points each instead get "The drawing value exceeds the browser
      preview limits.", with no element and no span, although the path-command limit would also
      reject them.
  - RF59's second pass moved the depth/node limits ahead of this guard, but not the value-count
    budget.
- **Recommendation:** Count points in the same pre-JSON `sceneLimit` walk against
  `path_commands`, and report the first exceeding element and its origin. Otherwise, include the
  failing path and the nearest node's origin in the guard's diagnostic. Add a routing or browser
  test.

- **Fix (third pass):** The pre-JSON scene walk counts point lists against the scene path-command limit, reporting the first exceeding element and origin. Remaining generic value-budget failures carry their path and nearest graphics node source. Added both large-point and oversized-payload regressions.
- **Verification (third pass):** Verified (third pass). The pre-JSON scene guard counts points against the path-command limit, and generic guard errors map to the nearest public scene element and its source.
  - Two Polygons with 99,999 points each, and one Polyline with 60,000, report "scene path command limit exceeded (element/children/0)" with span 4:75, in Node and in Chromium.
  - The browser guard never rejects a scene that native would accept. Native's full command count (five root commands plus a Polygon's close) always trips first, at 49,996 Polyline points, and both report an element and a span.
  - Spread across shapes, with clips and mixed Path data, and nested in groups, the failure names the exceeding element (for example `element/children/5` or `element/children/0/children/1/children/0`) with its span.
  - Eleven 100,000-character Texts or Path strings, including ones produced by a helper, report the generic budget message at `element/children/9` with that element's span.
  - A non-finite nested Point reports its JSON path plus the element and span.
  - DPR 1, 2 and 4 rendering and the RF59 depth and node limits are unchanged.
  - Minor wording difference, not reopened: "scene path command limit exceeded" (browser) versus "scene command limit exceeded" (native).

### Summary (2026-10-29 12:34 verification)
- **Verified (12):** RF38, RF40, RF54, RF55, RF64, RF65, RF66, RF67, RF68, RF69, RF70, RF71.
- **Reopened (2):**
  - RF58 (Low): a broken document briefly loses its stale styling and re-enables actions on each
    keystroke.
  - RF59 (Low): Preview's `SessionOutput` heading ignores `nx-output-format`.
- **New (2):**
  - RF72 (Low): a late action answer can restore state after a reset start.
  - RF73 (Low): the value-count guard gives a diagnostic with no location.
- **Minor observations, not opened:**
  - the opacity preflight shrinks the scale in ×0.75 steps rather than to the largest scale that
    fits;
  - integer host limits such as `nodes: 0` are not range-checked;
  - an always-true check at `workbench.ts:711`;
  - older brace-less lines in `check-graphics-browser.mjs:43,45`;
  - the WASM graphics `Request` accepts positional serde config, which only the playground builds.
- **Validation re-run in this pass:**
  - Rust: `cargo test -p nx-graphics` (17), and CI's `-p nx-graphics -p nx-graphics-skia
    --features nx-graphics-skia/preview` (17, 8 and 1).
  - Checks and lint: `check-graphics-preview.py`, `verify-graphics-fixtures.mjs`, Clippy with
    `-D warnings`, and `cargo fmt --check`.
  - Playground: typecheck, 165 tests and 25 examples, and `test:graphics-browser`.
  - Chromium probes:
    - DPR 1, 2, 4 and 5;
    - text pixel containment (36 runs, 0 outside);
    - nested opacity;
    - depth 47-200 and node limits;
    - failing `#code=` navigation, Back/Forward and picked-component recovery.
- Probes were temporary scratch NX sources and Node/Playwright scripts under `/tmp/nxver-*2`, each
  with an explicit timeout. No implementation file or test was changed, and no temporary file was
  left in the repository.
- Earlier statuses are unchanged: RF1-RF37, RF39, RF41-RF53, RF56, RF57 and RF60-RF63 remain
  Verified or Resolved.


## Verification Update 2026-10-29 12:44

The same reviewer session (`2e3fd5ea-4d4d-4afd-ac2d-833e53b95385`) ran this third verification.

- **Verified (4):** RF58, RF59, RF72, RF73.
- **Reopened:** none.
- **New findings:** none.
- **Validation re-run:**
  - Playground typecheck, 169 tests and 25 examples, and `test:graphics-browser`.
  - Chromium probes:
    - timed MutationObserver checks of valid, broken and broken-then-fixed edits;
    - the 300-Row heading in both tabs;
    - a delayed component-action answer across reset navigations;
    - huge point lists and oversized strings;
    - DPR 1, 2 and 4.
  - Node `createNxSession` probes for runtime notices and limits.
- Probes ran under `/tmp/nxver-*3` with explicit timeouts. No implementation file or test was
  changed, and no temporary file was left in the repository.
- **Coverage notes, not findings:** `usePreview`'s epoch handling has no hook-level unit test (the
  reducer regression covers the model), and `sessionLost` was checked by reading the code.

RF1-RF73 are now all Verified or Resolved; none are Open or Fixed.

## New Findings Discovered During 2026-10-29 13:15 Naming Review

**Scope:** The graphics naming refinement only (task group 10), compared against the pre-rename snapshot in
`/tmp/nx-naming-before/` and the naming-only diff. Reviewed `crates/nx-api/src/std/graphics/graphics.nx`, the new
contract test in `crates/nx-api/src/standard_library_tests.rs`, `crates/nx-codegen/src/graphics_tests.rs`,
`examples/graphics/{main,runtime}.nx`, the playground example, the graphics tests and browser check,
`scripts/check-graphics-preview.py`, the website graphics reference, the active DrawnUI proposal and its
`core.nx`/`graphics.nx` declarations, the OpenSpec proposal/design/spec/tasks/release and implementation notes,
and both regenerated walkthrough fixtures. RF1-RF73 were not re-audited.

**Evidence gathered:**
- A repository-wide search, excluding archives, review records, `target`, `node_modules` and `dist`, finds no
  remaining `GraphicsCommon`, `ShapeCommon` or `Anchor` in active graphics source or consumers, and no
  `RegularPolygon`/`Star` call that still passes `count=`. What remains is intentional: the vscode hover-syntax
  fixtures and `openspec/specs/editor-syntax-highlighting`; the dated findings record
  `docs/drawn-ui-proposal-nx-enhancements.md`; `docs/...-original.md`; the unrelated `Popup Anchor` IR conformance
  catalog; and the local `Stars count`/`Star count` functions in the range, parser and visualizer tests. No
  unrelated `count` was renamed.
- Probes run with the rebuilt `nxlang` (timestamped after the `graphics.nx` edit), then deleted:
  - `count=5` on `g.Star` reports both "has no property 'count'" and "requires property 'pointCount'".
  - `g.ShapeCommon` and `g.GraphicsCommon` fail with "could not be resolved", and `g.Anchor` is "not a visible
    type".
  - A selective import of `Shape`, `GraphicsElement`, `AxisAlignment`, `Drawing`, `ViewBox`, `ContentAlignment`
    and `Star` compiles. A local component that extends `Shape` picks up the inherited `fillRule` and `clipRule`
    defaults, and `ContentAlignment x={axis}` evaluates to `x=end y=center`.
  - With a wildcard `import "@nx/graphics"`, a local `type Shape` shadows the imported one without error.
  - `RegularPolygon pointCount=2` produces "RegularPolygon pointCount must be in 3..4096; received 2".
- `cmp` confirms that both walkthrough PNGs are byte-identical to the snapshot. In both manifests, normalizing the
  revision, graphics_version, program_revision and identity-version hashes and the provenance offsets leaves no
  difference. No stale old hash (`3b3b7b47…`, `5389c6d3…`, `8cbba614…`, `142f52f0…`) remains anywhere in the tree.
- The private helper renames match the user's mapping. `normalizeDegrees` keeps `degrees % 360.0` (signed
  remainder), and every call site was migrated.
- `openspec validate add-nx-graphics-core-and-direct-skia --strict` passes.
- The logs confirm that `graphics_domain_names_and_point_count_are_the_public_authoring_contract` passed in both
  the focused and the workspace runs. Fixture compare, TypeScript IR helpers, the 25 playground examples, 223
  website NX blocks, typecheck and the browser check all report success.

### ✅ Verified - RF74 The RegularPolygon doc comment still describes its vertex count as `count`.
- **Severity:** Low
- **Evidence:** `crates/nx-api/src/std/graphics/graphics.nx:224` reads "A regular polygon with count equally spaced
  vertices around (cx, cy)". The rename left this line unchanged. Hover and generated reference text will name a
  property that no longer exists, while the property line four lines below says `pointCount`.
- **Recommendation:** Change it to "A regular polygon with pointCount equally spaced vertices around (cx, cy);
  produces a Polygon.", then regenerate the version-bearing walkthrough manifests, since the library source hash
  changes. Optional while editing: the `ContentAlignment` trailing docs (lines 47-48) kept their old padding and
  now start at column 41. Every other single-word-gap block starts at column 34. If that is not deliberate,
  realign the two lines together.

- **Fix:** Updated RegularPolygon's summary to pointCount and realigned the two AxisAlignment property comments at the existing column34 convention. Rebuilt the linked library artifacts and regenerated/compared the version-bearing walkthrough manifests.
- **Verification:** Verified. `graphics.nx:224` now reads "with pointCount equally spaced vertices". The
  `ContentAlignment` property docs (lines 47-48) now start at column 34. Diffing `graphics.nx` against the
  pre-rename snapshot shows exactly the earlier naming hunks plus these three changed lines. Both manifests were
  regenerated after the 13:18 source edit, and fixture compare passed afterwards. `graphics_version` and all 10
  identity versions agree on `cec0428e64be5468`, and no older version hash remains in the tree. Both PNGs are
  still byte-identical to the snapshot.

### ✅ Verified - RF75 The active DrawnUI proposal's mechanical rename now says nodes "accept" a type instead of its properties.
- **Severity:** Low
- **Evidence:** `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md:494` ("Every node inside a `Drawing` accepts
  `GraphicsElement`") and `:518` ("Every filled/stroked geometry accepts `Shape`…`Shape` is expressed as a link in
  a chain") swapped bundle names (`GraphicsCommonProps`, `ShapeProps`) for base-type names without adjusting the
  sentences around them. The property tables at `:572`, `:580`, `:594`, `:605`, `:617`, `:629`, `:636`, `:645`,
  `:656` and `:678` now read "all `GraphicsElement`, `Shape`", next to the unchanged "all `UiCommonProps`" rows at
  `:548` and `:775`. The text now suggests a node accepts a `GraphicsElement` value rather than inheriting its
  properties.
- **Recommendation:** Change the wording to "accepts the `GraphicsElement` properties" and "inherits the `Shape`
  properties…`Shape` extends `GraphicsElement` rather than forming a second mixin". In the tables, write "all
  `GraphicsElement` properties" and "all `GraphicsElement` and `Shape` properties". Leave the code blocks as they
  are.

- **Fix:** Reworded proposal inheritance prose and property-table rows to say GraphicsElement/Shape properties, with Shape extending GraphicsElement through one base-type chain. Code declarations retain the approved domain names.
- **Verification:** Verified. Line 494 now says "inherits the `GraphicsElement` properties", and line 518 says
  "inherits the `Shape` properties… `Shape` extends `GraphicsElement`". All ten table rows (`:572`-`:678`) say
  "all `GraphicsElement` [and `Shape`] properties", and the code blocks are unchanged. Not a finding: the
  rewrite dropped the main proposal's only "NX allows only one base per component" rationale, which still
  appears in the companion `docs/drawnui-proposal/graphics/graphics.nx:73-76`. Restore it only if the main
  document should stand on its own.

### ✅ Verified - RF76 The new contract test verifies only the removal of `count`, not the "no aliases" rule for the renamed types.
- **Severity:** Low
- **Evidence:** The spec scenario at `specs/graphics-library/spec.md:92` requires that the library "SHALL NOT
  retain the old public names or count helper properties as aliases". The test
  `crates/nx-api/src/standard_library_tests.rs:1199` checks the new names positively. On the negative side it only
  checks that some diagnostic contains `"count"` (`:1221`). That check is loose, and nothing fails if
  `GraphicsCommon`, `ShapeCommon` or `Anchor` is later reintroduced as an alias. The probes above show the current
  behavior is correct; it is just not covered by a test.
- **Recommendation:** Add negative cases to the same test:
  - `extends g.GraphicsCommon` and `extends g.ShapeCommon` must report "could not be resolved".
  - `let a:g.Anchor = center` must report "not a visible type".
  - Tighten the `count` assertion to match "has no property 'count'" (or the stable diagnostic code), so an
    unrelated diagnostic that happens to contain "count" cannot satisfy it.

- **Fix:** Tightened legacy count checks to the unknown-property message and added negative cases for g.GraphicsCommon, g.ShapeCommon and g.Anchor, matching the normal unresolved-type diagnostics.
- **Verification:** Verified. `standard_library_tests.rs:1199` now:
  - requires "has no property 'count'" for both helpers;
  - requires "could not be resolved" for `extends g.GraphicsCommon` and `extends g.ShapeCommon`;
  - requires "not a visible type" for `g.Anchor`.

  The expected messages match the diagnostics observed in the earlier CLI probes.
  `/tmp/nx-naming-api-fix.log` shows the four `graphics_` tests passing, including this one.

### Questions (naming review)
- None. Leaving `docs/drawn-ui-proposal-nx-enhancements.md:29,492` (a dated findings record) and the
  hover-syntax `ShapeCommon` fixtures unchanged matches the stated scope.

### Summary (2026-10-29 13:15 naming review)
- The rename is complete and consistent across source, consumers, tests, examples, docs and OpenSpec artifacts.
  Behavior is unchanged: the PNGs are byte-identical and the manifests differ only in hashes and offsets. The
  version-bearing artifacts were regenerated, no aliases remain, and no unrelated `count` was touched.
- Three Low findings were opened (RF74-RF76): one stale doc word, proposal prose that a mechanical substitution
  left awkward, and a negative-coverage gap. None affects behavior. Tasks 10.2 and 10.3 are still unchecked,
  correctly.

## Verification Update 2026-10-29 13:24 (naming review)

- Verified: RF74, RF75, RF76. Reopened: none. New findings: none.
- All naming-review findings (RF74-RF76) are now Verified, and RF1-RF76 contain no Open or Fixed entries.
  Task 10.3 can be checked once the separately running final SDK and browser checks pass.
- No implementation or test file was changed, and no probe file was left behind.

## New Findings Discovered During 2026-10-29 13:55 Scope-Cleanup Review

**Scope:** only the 2026-10-29 user-requested cleanup (tasks.md section 11, implementation-notes.md
"Scope cleanup after user review"). Unrelated worktree edits and RF1-RF76 were not re-reviewed.\
**Reviewed code:** `crates/nx-interpreter/src/interpreter.rs` and `crates/nx-api/src/nx_text.rs` (against HEAD),
`crates/nx-codegen/src/graphics_tests.rs`, `crates/nx-codegen/src/lib.rs`, `crates/nx-api/src/standard_library_tests.rs`,
`crates/nx-graphics-skia/Cargo.toml`, `Cargo.lock`, `.gitignore`, `.github/workflows/graphics-native.yml`,
`scripts/check-graphics-preview.py`, `sites/playground/src/compile/{policy.ts,policy.test.mjs,types.ts}`,
`sites/playground/src/worker/{protocol.ts,session.ts,session.test.mjs}`, `sites/playground/src/preview/workbench.ts`,
`sites/playground/src/output/evaluation.ts`, `sites/playground/README.md`, `sites/playground/package.json`.

**Confirmed clean:**
- `git diff HEAD` is empty for `crates/nx-interpreter` and `nx_text.rs`. No added nx-api or nx-codegen line
  mentions the interpreter or HIR evaluation.
- `nx-graphics-skia` no longer lists `nx-hir` in `Cargo.toml` (dependency or `preview` feature) or in its
  `Cargo.lock` entry, and no source in either graphics crate references it. The `preview` feature, the
  `nx-graphics-preview` bin and the CI/Python callers are intact.
- `git check-ignore` no longer matches `crates/nx-graphics-skia/src/bin/preview.rs`, and `git status` lists it
  as untracked. The exception is anchored to that one directory, and no other `src/bin` exists under `crates/`.
- No playground source, test, script or package.json still mentions `evaluateSource`, `EvaluateResult`,
  `Evaluate`, `compile/evaluate`, `useEvaluation` or a `kind: "evaluate"` request. `WorkerCall` and the session's
  switch have only `preview`/`language`. `policy.ts` keeps every non-HIR export that `evaluate.ts` had at HEAD.
  `MAX_SOURCE_BYTES` is still enforced (`workbench.ts:267`), and `valueOutcome` still truncates IR output
  (`workbench.ts:375,625,707`). `#keepBuild` is private.
- The session-test expectation change for `declaration.startLine` (1 to 2) reflects the IR's built-record
  construction site (`App.tsx:386-398` `built` path), not a cleanup regression.
- A throwaway probe through the playground's `createNxSession` (TypeScript IR, now deleted) produced canonical
  `Transform.translate`, `Paint.solid` and `Stroke` values with defaults. It also gave `<ContentAlignment
  x=center y=center />` and `<ViewBox …/>` beside a local `ViewBox` decoy, and accepted a consumer
  `id(t: g.Transform)`. The behavior is correct; RF77 is about what the tests pin.
- `nxlang run examples/graphics/main.nx` now fails with `Record type not found: Stroke`. That is the accepted HIR
  limitation (proposal.md:59-62), and `sites/website/.../libraries/graphics.md:8` already documents it.
- Logs: `/tmp/nx-cleanup-ir.log` (6 graphics IR tests), `api` (37), `playground` (169/169), `workspace`
  (86 result lines, none failed), `cli`, `native`, `browser` and `sdk` all pass. When I checked,
  `test:graphics-browser` was still running.

### ✅ Verified - RF77 The migrated aliased-records IR test only checks that evaluation succeeds, so it no longer pins defaults, canonical case names or decoy isolation.
- **Severity:** Low
- **Evidence:** `graphics_aliased_records_defaults_and_payload_cases_evaluate_on_ir`
  (`crates/nx-codegen/src/graphics_tests.rs:222-233`) calls `.unwrap()` on each result and asserts nothing. The
  cleanup says this test covers "imported records, defaults, union payloads" (implementation-notes.md, Scope cleanup).
  But a regression that left `<g.ContentAlignment/>` without its `x`/`y` defaults would still pass. So would one that
  spelled a case `g.Transform.translate` (the RF3 failure mode) or picked up the local `ViewBox` decoy's shape.
  RF7's consumer-side sites (`let id(t: g.Transform)`, a local record field typed `g.ViewBox`) have no IR test at
  all. Those checks were HIR-only, and the cleanup retired them.
- **Recommendation:** Serialize each value and assert `$type` and the key fields: `ViewBox` with `x=1`;
  `ContentAlignment` with `x`/`y` `"center"`; `Rect.transform.$type == "Transform.translate"`;
  `fill.$type == "Paint.solid"`; `stroke.paint == "none"`. Add one consumer-typed case, such as
  `let id(t: g.Transform): g.Transform = { t }` used in `transform={id(...)}`, so the IR path pins what RF7 covered.

- **Fix:** Assert ViewBox and ContentAlignment canonical values/defaults, Rect dimensions/opacity, canonical Transform.translate/Paint.solid/Stroke and none paint, plus add a consumer-side g.Transform parameter and g.ViewBox record field with a local ViewBox decoy.
- **Verification:** `graphics_aliased_records_defaults_and_payload_cases_evaluate_on_ir`
  (`graphics_tests.rs:222-264`) now compares the whole `ViewBox` and `ContentAlignment` values (`x`/`y` `"center"`),
  so a leaked decoy field (`unrelated`) or a missing default would fail. It also pins `Rect` width, height and
  opacity, a canonical `Transform.translate` (accepting a sequence of one), `Paint.solid` and `Stroke` with
  `paint == "none"` and `width == 1.0`. The new `graphics_consumers_accept_aliased_shared_types_on_ir`
  (`graphics_tests.rs:266-288`) puts a local `ViewBox` decoy beside a `Holder` field typed `g.ViewBox` and an
  `id(t:g.Transform):g.Transform`, and compares both values whole. My own rerun of
  `cargo test -p nx-codegen graphics_tests` passed all 7 tests.

### ✅ Verified - RF78 The playground README still lists "evaluate" tests among those `pnpm test` runs, though that suite was removed.
- **Severity:** Low
- **Evidence:** `sites/playground/README.md:124` still reads "Worker, route, share, evaluate, worker and dev-shell
  tests". `src/compile/evaluate.test.mjs` is deleted, and its pure tests are now `src/compile/policy.test.mjs`. Task
  11.2 and request item 5 ask for the README update. Line 188 was updated, but this line was not.
- **Recommendation:** Replace "evaluate" with "policy" (or "IR session"), and drop the duplicated "worker".

- **Fix:** Updated the pnpm test description to policy/IR session coverage and removed the duplicate worker entry.
- **Verification:** `sites/playground/README.md:124` now reads "Worker, route, share, policy, IR session and
  dev-shell tests". This matches `package.json`'s `test` script, which globs `src/**/*.test.mjs` (including
  `policy.test.mjs` and `worker/session.test.mjs`).

### Questions (scope-cleanup review)
- None.

### Summary (2026-10-29 13:55 scope-cleanup review)
- The cleanup matches the request. The HIR interpreter files are back at HEAD, the playground's HIR endpoint and
  public types are gone with no dangling references, and the shared policies moved without losing source-size or
  output-truncation enforcement. `nx-hir` is out of the Skia adapter and its lockfile entry, and the preview CLI
  source is visible to Git through a narrow exception (untracked, not staged).
- Two Low findings were opened: RF77, IR test assertions too weak for the coverage the cleanup claims, and RF78, a
  stale README word. Neither affects behavior. Task 11.4 is correctly still unchecked.

## Verification Update 2026-10-29 14:10 (scope cleanup)

- Verified: RF77, RF78. Reopened: none. New findings: RF79 (Low).
- I also checked the requested refinement that made PreviewWorkbench's compile callback required. The constructor
  takes `compile: PreviewCompile` and stores it as non-optional `readonly #compile` (`workbench.ts:100,134-139`).
  `#buildOf` always compiles when no kept build matches (`workbench.ts:266-282`). Every `new PreviewWorkbench` call
  site passes a compile callback: `worker/session.ts:47`, `scripts/check-examples.mjs:139`, `routing.test.mjs`
  (7 sites) and `workbench.test.mjs:177`. `pnpm run typecheck` is clean, and my rerun of the workbench, routing,
  session and policy tests passed 61/61. There is no behavioral regression. Only comments are left over (RF79).
- No implementation or test file was changed, and no probe file was left behind.

## New Findings Discovered During 2026-10-29 14:10 Verification

### ✅ Verified - RF79 PreviewWorkbench comments still describe compile as an optional fallback for the removed evaluate-first flow.
- **Severity:** Low
- **Evidence:** These comments are unchanged from HEAD, where `compile` was optional and the `evaluate` endpoint
  built and kept the latest artifact (HEAD `workbench.ts:68-101,169-172`):
  - `#other` (`workbench.ts:96-99`) says it may hold "one a fresh worker was asked to run". A fresh worker's build
    now goes through `#keepBuild` into `#latest`, like every other build.
  - The `@param compile` doc (`workbench.ts:131-133`) limits it to "one a fresh worker is asked to run, or the one an
    older step of the session ran". It is now the only way any source is built, including each edit.
  - `#buildOf` (`workbench.ts:265`) says "compiled now when the workbench can". That was the HEAD
    `this.#compile === undefined` guard, which is gone.
  - `workbench.test.mjs:176` says "asked to start before any evaluation, it builds the source itself". There is no
    separate evaluation step any more.
- **Recommendation:** Describe `compile` as how the workbench builds any source it holds no build of. Drop "a fresh
  worker was asked to run" from `#other`, so it is just the build the session's current step still runs after an
  edit. Change `#buildOf` to "kept, or compiled now". Reword the test comment to describe the crash-then-rebuild
  path it actually exercises.

- **Fix:** Reworded the compiler callback, latest/previous build cache and build lookup comments for workbench-owned compilation, and updated the test comment to describe rebuilding after host loss. No behavior changed.
- **Verification:** All four comments now describe workbench-owned compilation:
  - `#other` (`workbench.ts:95-98`): "The previous build the session's current step still runs after an edit."
  - `@param compile` (`workbench.ts:131-132`): "builds any source the workbench holds no build of, including edits
    and sources needed again by an older session step."
  - `#buildOf` (`workbench.ts:265`): "kept, or compiled now."
  - `workbench.test.mjs:176`: the comment now covers compiling the initial source and rebuilding it after the host
    loses its builds.

  A grep of `workbench.ts` and `workbench.test.mjs` finds no "fresh worker was asked", "when the workbench can" or
  "before any evaluation". The test title "starts in a fresh worker" still fits, because the test builds a new
  workbench.

### Summary (2026-10-29 14:10 verification)
- RF77 and RF78 are Verified. The required-compile refinement is behaviorally sound and fully adopted at every
  call site. One Low comment-accuracy finding, RF79, is open. Task 11.4 can be checked once RF79 is fixed and
  verified, or accepted as is.

## Verification Update 2026-10-29 14:20 (scope cleanup)

- Verified: RF79. Reopened: none. New findings: none.
- RF77-RF79 are all Verified, and RF1-RF79 contain no Open or Fixed entries. Task 11.4 can be checked.
- No implementation or test file was changed.

## New Findings Discovered During 2026-10-29 14:47 Coverage-Gap Review

**Scope:** incremental coverage only. This pass covers material that no earlier pass explicitly reviewed and the
seams between passes. It is not a re-review of RF1-RF79, which the full (RF1-RF73), naming (RF74-RF76) and
cleanup (RF77-RF79) passes reviewed and verified.\
**Reviewed artifacts:** final `tasks.md` section 11 and its completion line; `implementation-notes.md` (the
"Scope cleanup after user review" section and the file header); `proposal.md`, `design.md`, `release-notes.md`
and the delta specs, checked for HIR, evaluate and native-CLI claims.\
**Reviewed code (not explicitly listed by the cleanup pass):** `sites/playground/src/App.tsx`,
`src/graphics/{prepare,selection}.ts`, `src/editor/{NxEditor.tsx,positions.test.mjs}`,
`src/preview/{metadata.ts,PreviewView.tsx,usePreview.ts}`, `src/worker/channel.test.mjs`,
`src/compile/{index,types,worker}.ts` and `worker.test.mjs`. Also checked: `crates/nx-graphics-skia/{Cargo.toml,
src/bin/preview.rs}`, the `Cargo.lock` entries for `nx-graphics`/`nx-graphics-skia`, `.gitignore`, and the website
graphics reference and playground README for HIR/CLI claims.

**Confirmed clean:**
- No concurrent edits. Every path in `/tmp/nx-recent-review-before.json` still matches its hash, apart from the
  three files deleted before the snapshot (`compile/evaluate{.ts,.test.mjs}`, `useEvaluation.ts`). The only
  snapshot files modified after the last cleanup code edit (`workbench.ts`/`workbench.test.mjs`, 13:59, RF79
  comments) are `review.md`, `tasks.md` and `implementation-notes.md`.
- The import moves are mechanical. Every `compile/policy.ts` import (`App.tsx:33`, `prepare.ts:11`,
  `selection.ts:1`, `PreviewView.tsx:14`, `metadata.ts:3`, `NxEditor.tsx:1,20`, `session.ts:17`,
  `workbench.ts:36-40`) names an existing export. `compile/index.ts` now re-exports only the types, and nothing
  imports the removed `evaluateInBrowser`, `Evaluate` or `EvaluateResult`. The generic `retrying` has one
  production caller (`usePreview.ts:26-28`), and it is limited to `start`/`reload`, so action dispatches are never
  retried. The `MODEL_URI` re-export through `NxEditor.tsx` was reviewed and verified earlier (the
  previewDeclarations/`MODEL_URI` finding, review.md ~1490-1507). Only its module name changed.
- Test migrations: `channel.test.mjs` swaps only the request shape (`preview`/`start`). `positions.test.mjs`
  evaluates through `programFromImages(generateNxIr({debug:true, modules:[]}))` + `evaluateFunction` +
  `formatValue`, the same IR path as `workbench.ts:303-305,375`. The only remaining `evaluateNx` references in
  playground tests are traps that throw if HIR runs (`graphics.test.mjs:55`, `routing.test.mjs:160`).
- No production HIR path remains. `nx-graphics`, `nx-graphics-skia` and `bindings/wasm/native/src/graphics.rs`
  have no `nx_hir`/`nx_interpreter` reference. The Skia adapter's lock entry has no `nx-hir`.
  `preview.rs` evaluates through `nx_ir_runtime` (`preview.rs:4,171-181`), and its `preview` feature pulls in
  Skia (`native`) only as an option. `git check-ignore` no longer matches `preview.rs`. The website
  (`graphics.md:6-10`), the playground README (`:229`), `proposal.md:59-62,95-96`, `design.md:79-93` and
  `release-notes.md` all say HIR is not updated and the browser uses TypeScript IR with Canvas 2D.
- Reran on the current tree: `pnpm test` in `sites/playground` passed 169/169 tests and 25/25 example snapshots,
  and `pnpm typecheck` is clean. This confirms the notes' "169 passed after requiring the compiler callback".
  The earlier playground log (13:56) predates the RF79 comment edits (13:59). Rust IR tests were not rerun:
  `graphics_tests.rs` is unchanged since the 13:57 run of 7 passing tests (`/tmp/nx-cleanup-ir-fix.log`).

### ✅ Verified - RF80 The implementation-notes header still says the earlier HIR fixes "are preserved as-is", which the scope cleanup reversed.
- **Severity:** Low
- **Evidence:** `implementation-notes.md:3-5` says: "Earlier partial HIR fixes are preserved as-is; graphics
  acceptance does not rely on them." The same file's cleanup section (`implementation-notes.md:307-308`) says those
  fixes were removed, so `interpreter.rs` and `nx_text.rs` match HEAD. `git diff HEAD --stat --
  crates/nx-interpreter crates/nx-api/src/nx_text.rs` is empty, which confirms the removal. The cleanup pass
  checked the code but not this header. A reader of the summary, or an archiver, would wrongly conclude that the
  change ships interpreter edits. No other artifact has this claim.
- **Recommendation:** Replace the sentence with something like "Earlier partial HIR fixes were removed in the
  scope cleanup (below); `nx-interpreter` matches HEAD and graphics acceptance uses IR only."

- **Fix:** Corrected the implementation-notes header to state that the partial HIR fixes were removed and graphics acceptance uses IR only, matching the cleanup section and unchanged interpreter source.
- **Verification:** `implementation-notes.md:3-6` now says the earlier partial HIR fixes "were removed in the
  scope cleanup below; the legacy interpreter remains unchanged and graphics acceptance uses IR only." This agrees
  with the cleanup section (`implementation-notes.md:308-309`) and with the source: `git diff HEAD --stat --
  crates/nx-interpreter crates/nx-api/src/nx_text.rs` is still empty. No "preserved as-is" or "partial HIR" claim
  remains in the change directory. Apart from `review.md`, `implementation-notes.md` is the only file that differs
  from the pre-review hash snapshot, so no code or test changed.

### Notes (not findings)
- The Git index splits the change. `crates/nx-graphics-skia/src/bin/preview.rs` is staged (`A`), but its
  `.gitignore` exception is unstaged (` M`). A commit of only the staged set still tracks `preview.rs`, but it leaves
  out the exception, so new files under that `src/bin` would be ignored again. Staging is the human's decision;
  this is recorded only so nothing is missed.
- The "14:10" and "14:20" headings above were written by 13:59 (the `review.md` mtime and
  `/tmp/nx-cleanup-claude-verify2.json` are both 13:59). Their order is correct, but the times are not. They are
  left unchanged so the report is preserved.

### Questions (coverage-gap review)
- None.

### Summary (2026-10-29 14:47 coverage-gap review)
- The coverage gaps were the cleanup's import moves and test migrations that no pass listed, and the final
  tasks/notes edits. They hold no behavioral regression, no dangling HIR/evaluate reference, and no production HIR
  path. The native CLI and the browser path match the preserved decisions.
- One new Low documentation finding (RF80) is open: a stale header sentence in `implementation-notes.md`. No
  implementation or test file was changed by this pass, and no temporary file was left behind.

## Verification Update 2026-10-29 14:49 (coverage-gap review)

- Verified: RF80. Reopened: none. New findings: none.
- RF1-RF80 contain no Open or Fixed entries. Only documentation changed, so I reran no tests.
