# Implementation and validation

Runtime scope: Rust and TypeScript IR. Per explicit user direction, the legacy HIR interpreter
will not receive further graphics updates. Its retirement remains separate. Earlier partial HIR
fixes were removed in the scope cleanup below; the legacy interpreter remains unchanged and
graphics acceptance uses IR only.

Delivered: unstable standard-library declarations and pure math-backed helpers; neutral declaration-
aware preparation, SVG normalization, bounds, scoped destination and text contracts; opt-in Linux
Skia destination with explicit supplied fonts/shaped retained runs; direct IR preview and measurement,
atomic revision publication and stale inspection; SVG/reference/edit fixtures, authoring-trial schema,
reference/release documentation and native CI lane. No recording or sibling checkout is required.

Validation performed on Linux x86_64:
- `cargo test -p nx-api -p nx-codegen -p nx-graphics -p nx-ir-runtime`: passed.
- `pnpm --dir runtime/typescript test`: passed, including graphics helpers and damaged-image checks.
- `cargo test -p nx-graphics-skia --features native`: passed, with shaped Latin/combining/Greek runs,
  missing-font/glyph diagnostics, isolated-overlap pixels, independent evenodd clip holes and state
  restoration after success/injected failure.
- `cargo check -p nx-graphics --target wasm32-wasip1`: passed (neutral package only).
- `cargo build -p nx-graphics-skia --features preview --bin nx-graphics-preview`: passed.
- `python3 scripts/check-graphics-preview.py --write-fixtures`: passed; actual PNG output, measured
  placement, five-to-seven-tip revisions, manifest fields, stale inspection, spoof rejection,
  numerical domain/non-finite-input/overflow diagnostics and unchanged publication after failure.
- `node scripts/verify-graphics-fixtures.mjs`: passed without model calls.
- Compiler and neutral dependency trees contain no Skia/DrawnUI package.

Pin: upstream skia-safe 0.153.3. 0.90.0 builds but lacks an explicit custom empty font loader;
0.156.0 encountered an upstream bindgen/syn dependency error. 0.153.3 builds with the selected
textlayout features, without local/sibling patches. The binary cache was unavailable here; source
build used clang and ninja. Runtime preview requires no network or system font discovery. Font
bytes, SHA-256 metadata and redistribution license are checked in.

Neutral coordinates and bounds are f64. Backend scalar conversions and host limits are checked.
Curved/SVG arc construction uses explicit graphics tolerance. Visible clip intersections and text
ink bounds are conservatively classified; exact clipped-hole geometry is not claimed. Native support
is validated on Linux; other backends/targets use their own destination/text implementations.
The versioned agent trial corpus has not been run against models and makes no quality claim.

Independent review: see review.md. A fresh Claude CLI pass reviews the full implementation after
the earlier partial-library pass, under the clarified IR-only runtime scope.

Additional self-checks before independent verification: text-run handles are transient and omitted
from serialization/revision identity; repeated preparation using one text service keeps the same
revision. Painting rejects a retained handle whose font/content/layout request does not match the
prepared text. Font sizes, transforms and device scales reject f32 underflow as well as overflow.

Full review pass RF8-RF16: fixed and awaiting same-reviewer verification. New focused tests cover
unhinted scaling, actual glyph ink, precise helper diagnostics, exact rectangular visible bounds,
independent radius clamping, run-cache reuse/release, all SVG-corpus samples, actual helper-ring
PNG pixels, partial host-limit overrides and imported failure revision identities. Additional raw
NxValue preparation rejects non-finite coordinates before JSON serialization can turn them into
null. Main shape helpers retain period reduction via the existing floating remainder operator,
with large finite rotation/full-sweep fixtures passing on Rust and TypeScript IR.

## Final verification

All 31 implementation tasks are complete. Same-session independent Claude verification
(`d57bacfc-93eb-452a-9f21-b9e12a19f47d`) verified RF11 and RF17 after the second fix pass and
reported no new proven issues. RF1-RF17 are Verified or explicitly Resolved; none remain open.
The follow-up finite-input checks remain ordinary NX source and run equivalently on both IR
runtimes. Folded/zero-area clip paths cannot claim exact nonempty visible bounds.

Final broader validation:
- `cargo test --workspace --locked`: passed, retaining existing ignored tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed. Four pre-existing math test
  lints noted by the reviewer were subsequently cleaned up without changing test semantics.
- Focused native graphics tests: 8 passed; neutral tests: 8 passed (including folded clips).
- Rust IR helper tests and TypeScript IR helper tests: passed with finite and non-finite cases.
- Preview walkthrough with regenerated before/after fixtures: passed, including actual PNG ring
  pixels, precise helper errors, imported failing-source revision IDs and partial limits.
- Neutral WASI compilation, graphics Clippy/fmt, fixture checks and OpenSpec strict validation:
  passed. The graphics reference includes build/run/measurement/inspection commands.

The change remains unarchived for user review. No commits or publication were performed.

## Playground extension: Canvas 2D (2026-10-28)

Per the user's final renderer choice, Drawing uses TypeScript IR evaluation and HTML Canvas 2D.
No CanvasKit or Rust/Skia browser player was added. Existing compiler WASM now exposes shared
neutral graphics preparation with an artifact-scoped resolver; the ABI is 11. Rendering adds no
WASM dependency. HIR remains unchanged by the playground extension.

The graphics example opens Drawing directly at `/play/graphics`. Geometry, clips, transforms,
limits and inspection come from `nx-graphics`. The Canvas adapter isolates translucent subtrees,
paints a complete offscreen frame before publishing, and rejects outdated source responses.
The supplied Fixture font is bundled with its license; measurement in the worker and painting in
the page use the same bytes and typography settings, with missing-face/glyph errors. The config
is `browser-canvas2d-v1`; browser and native text/raster identity is not claimed.

Validation:
- WASM SDK build/TypeScript typecheck and 128 SDK tests pass. Rebuilt the debug-trap WASM and
  native Node SDK before the parity suite; their older generated binaries initially caused ABI
  and keyword-completion mismatches, resolved by rebuilding matching artifacts.
- Playground typecheck, production Vite build, all 131 tests and all 25 example snapshots pass.
- Real Chromium integration passes revision edits, supplied fonts, isolated opacity pixels,
  ring/clip holes, malformed paths/missing glyph clearing, safe literal text, element inspection
  and component Preview availability. `pnpm --dir sites/playground test:graphics-browser` runs it;
  `CHROME_PATH` selects an installed browser. The deployment validation job now runs this check.
- Native neutral 8/Skia 8 regression tests and strict OpenSpec validation pass.

Independent review is pending. The Claude CLI invocation for `/openspecex-review` returned
`You've hit your weekly limit · resets 6pm (America/New_York)` before reviewing any code.
Session `c3d78d75-ab67-4d69-ad6c-d2b95c82d0a3`; output `/tmp/nx-graphics-playground-review.json`.
The earlier RF1–RF17 review remains intact. Task 7.5 stays open until an independent Claude review
and any necessary review-fix/review-verify iterations complete; no review success is claimed.

## Shared selection follow-up (2026-10-28)

The selection model is shared by graphics and structured output adapters, with source snapshots,
UTF-8 spans and current-output IDs. TypeScript IR can opt into helper call-site ancestry without
changing default Rust/TypeScript origin reports, values, costs or construction identity. Graphics
selection navigates to authored helper invocations. Both Visual and Source keep their current mode;
Visual selects all associated renderings, including repeated loop output. A pending selection can
wait for a fresh Visual tree without matching an old source snapshot.

Preview now supports static structured roots when no component can run. It formats the IR result
with source-linked record annotations, then uses the same selection contract to reveal/highlight
Question/Choice records. Stateful component output uses the same adapter contract. This supplies a
generic structured value view; a dedicated survey form renderer is not included.

Validation: all 139 playground tests and all 25 examples, complete TypeScript runtime suite (including
Rust origin conformance),38 value-view tests, typecheck and production build pass. Real Chromium
checks helper preview-to-Visual selection, preserving Source mode, Visual-to-canvas mapping, all
three loop renderings and two-way Question/Choice selection. The shipped Survey IR preview is also
covered by a worker integration test. Independent Claude review has been re-invoked after reset;
its review/fix/verify results will be recorded below.

## Independent playground review fixes (2026-10-28)

Claude reviewer session `c1fb5c65-7d20-45f8-a425-8eeb7188a9d8` opened RF18–RF24. All seven were
addressed with `/openspecex-review-fix` and sent to the same session for `/openspecex-review-verify`:
source-attributed helper diagnostics; peak concurrent layer limits and invisible-subtree skipping;
Source cursor-driven selection; identity/limit preflight before text measurement; typed SDK DTOs
and package-owned coverage; one immutable construction-declaration index shared by both hosts;
and expanded/braced frontend formatting with corrected helper documentation.

Post-fix validation: 142 playground tests and 25 snapshots; 130 SDK tests; 38 value-view tests;
48 previewer tests; full TypeScript runtime suite with optional/default origin contracts;
9 neutral/8 Skia tests; 20 native WASM-bridge tests; a shared API resolver test; 3466 Rust workspace
tests passing with 17 existing ignored; workspace clippy with all targets and warnings denied;
strict OpenSpec validation. The production Chromium integration includes two-way selection in
Visual and Source, helper attribution, repeated output, generic Question/Choice and the shipped
Survey/counter previews. The native PNG/revision/font walkthrough also passes after resolver and
layer-accounting changes. Reviewer verification will determine final finding statuses.

## Final playground verification (2026-10-28 18:38 America/New_York)

The same independent Claude reviewer verified RF18–RF24, discovered RF25 (font measurement
errors lacked source spans), and verified RF25 after its fix. Preflight requests now carry the
first requesting element's ID and provenance; failures map to exact authored Text spans. Repeated
requests measure once. Host crashes still propagate for worker recovery. No findings were reopened
and no new findings remain. RF1–RF25 are Verified or Resolved; task 7.5 and all 38 tasks are complete.

Final post-fix checks: 143 playground tests, all 25 snapshots, 130 SDK tests, release/debug-trap WASM
rebuilt together, both typechecks, native bridge 20 tests and production Chromium integration.
Earlier post-review checks covered the Rust workspace, neutral/native renderer, immutable resolver,
TypeScript runtime, previewer, value-view and strict clippy. Production browser checks include
helper/source navigation, both source modes, all loop renderings and generic Question/Choice.

The playground dev server remains at http://localhost:5173/play/graphics; /play/survey opens the
structured Preview. Reload an existing page to load the rebuilt compiler WASM. No deployment or
archive was performed.

## Generic playground preview routing (2026-10-28)

The right pane now has conditional Preview and universal Output. The browser uses one TypeScript
IR workbench for static values and component sessions, with root results cached per compiled
revision. Graphics preparation consumes the evaluated value/origins directly rather than compiling
and evaluating it again. A nominal-identity registry selects the graphics adapter; the component
inspector retains its actions, state, and timeline. Generic records and scalars are Output-only.
Both presentations share source selection; component interactions update raw Output as well.
Tab preferences survive edits, reset when another document opens, and retain Preview availability
during temporary failures. A failed graphics prepare keeps its raw value and diagnostics accessible.

All 25 example snapshots now use the same IR workbench; three component snapshots intentionally
show their rendered values instead of their root component descriptors. Focused routing tests cover
resolved alias/spoof identity, application-defined registry entries, errors and stale-source guards.
Chromium covers conditional tabs, persistent choices, selection, graphics pixels, component actions
and timeline movement. Independent review and final build results are recorded below when complete.

Validation after routing review fixes:
- Playground typecheck; 159 tests and all 25 IR example snapshots.
- Full TypeScript runtime suite, including construction-declaration metadata coverage.
- WASM SDK: 130 tests; release and debug-trap compiler images rebuilt.
- Production Vite build and development/production Chromium integrations.
- Strict OpenSpec validation and git diff whitespace checks.
- Independent Claude reviewer session: `9714ae84-0863-4ac2-a891-6b15035c1dff`;
  RF26-RF30 are Verified by that reviewer, with none reopened and no new findings on the final pass.

Final checks after RF30: production Vite build and Chromium integration pass, strict OpenSpec
validation and whitespace checks pass. All 43 tasks are complete. Review reports are preserved in
review.md; final reviewer response is /tmp/nx-generic-preview-rf30-verify.json. No commit, deployment
or archive was performed. The local development playground remains available on port 5173.

## Library-authored presentation metadata (2026-10-29)

ElementVisualizer.preview carries a typed PreviewVisualizer with a nonempty literal renderer ID.
The existing visualizer reader resolves it statically with ordinary inheritance/overrides. Source
tree metadata has an opt-in includeUnreferencedDeclarations query for loaded visualizer targets;
the playground caches this metadata per build and matches nominal construction identity. Its host
registry now lists implementations by logical renderer ID only. Unknown IDs stay Output-only.
This first profile attaches previews to nominal element types; primitive aliases retain ordinary
ValueVisualizer formats/editors. There is no executable-code descriptor or dynamic plugin loader.

Graphics declares nx.graphics and Lucide frame/group/geometric/path/text icons in its own source.
FunctionVisualizer allows presentation-only icons on element-style helper invocations, without
new props records or changes to evaluation. Paren functions remain unsupported targets. The helper
icons use hexagon, star, move-right, radius and donut; returned primitive identities are unchanged.

Initial validation: 43 visualizer API tests, 3 graphics API tests, 304 language service tests,
161 playground tests, 25 example snapshots and 132 SDK tests pass. Node and WASM were rebuilt for
metadata parity; browser checks confirm actual SVG icons and preserve drawing/selection/session
behavior. Broad checks and independent review results follow below when complete.

Final section 9 verification:
- Independent Claude session `2cc6e84a-afc7-48ea-a8d9-d182bdb2c0bd` verified RF31-RF36,
  with none reopened and no new findings. RF1-RF36 are Verified or Resolved.
- Inherited-only visualizer targets from loaded helper modules are included by the opt-in query.
- Metadata-query failures cache an Output-only fallback while host crashes still reach recovery.
- The shared production lookup uses MODEL_URI, is cached per build, and disposes snapshots on
  success and failure. Direct tests cover these behaviors and native/WASM metadata parity.
- Final validation: 164 playground tests and 25 snapshots, 132 WASM SDK tests, 38 Node SDK tests,
  43 visualizer API tests, 305 language-service tests, full Rust workspace/all-targets, full
  TypeScript runtime, Clippy with warnings denied, release/debug WASM and native Node builds,
  production Vite build, production Chromium icons/rendering/selection/session checks, strict
  OpenSpec validation, and whitespace checks.
- The user-preferred aligned trailing property documentation remains in place; the visualizer
  reference's embedded library source matches the actual standard-library file.
- All 47 tasks are complete. No commit, deployment or archive was performed. Reload the local
  playground to load the updated compiler WASM and see the library-owned preview and icons.

## Full-review scope and delta coordination (2026-10-29)

The source-tree delta overlaps `add-visualizer-icon-expressions` on “The answer carries resolved
visualizer declarations when asked”. When either change is archived, rebase the other delta onto
the merged text, preserving element-function targets, `preview` and the opt-in loaded declaration
table. Do not copy the other change's older restriction that functions cannot have visualizers.
The full-review fixes also tighten browser depth to 48 for the bounded SDK JSON bridge and cap the
neutral coordinate limit at its supported default 1e7. Hosts may tighten these limits. Browser
hit-testing now follows painted geometry, with bounds as the portable fallback for text only.
Walkthrough manifests and PNGs are compared by default; use `--write-fixtures` deliberately after
source/library changes. The native workflow validates manifest/error schemas, font hashes and
deterministic task fixtures, and runs native Clippy. Playground browser validation runs on PRs
without enabling the deployment job.

Full-review fix validation: Rust workspace/all-targets 3,456 passed, 0 failed, 10 ignored;
playground 162 tests plus 25 example snapshots (obsolete reducer tests were replaced with tests of
the shipped model); WASM SDK 132, Node SDK 38; website 13 tests plus 223 NX code blocks. Development
and production Chromium checks include deep-input recovery retaining Count: 4, actual-path picking,
DPR2 backing stores, and normal graphics/component/selection behavior. Development Chromium also
checks reference-size font scaling. Release/debug-trap WASM and Node native SDK were rebuilt.
Workspace and native-preview all-target Clippy, rustfmt, strict OpenSpec validation, fixture/task/font
validation, schema-validated walkthrough generation and a separate fixture comparison passed.
Logs are retained locally under /tmp/nx-full-review-*.log; independent verification is in review.md.

The second verification pass found narrower residual cases. The follow-up separates intrinsic text
ink from a configurable physical painted-bounds pad (`text_ink_padding`, default 0, browser 1.5),
keeps fill-only bounds free of isolated cap points, normalizes source-root hashing, and rejects
positional JSON in CLI object inputs. Browser resolution adapts to physical surface/layer limits;
scene-depth failures retain node/source provenance before serialization. Opening a different document
clears the old model immediately; reloads carry the visitor's component as a fallback for a fresh
worker. Pending successful edits use a quiet indicator, and formatting errors have their own category.

Latest validation after the follow-up fixes: playground 169 tests plus 25 examples; refreshed Rust
workspace 3,457 passed, 0 failed, 10 ignored; WASM SDK 132; website 13 plus 223 NX blocks. Native
preview tests, schema/fixture/hash checks, workspace/native Clippy and typecheck pass. Browser checks
now also cover typing while already stale and matching component format-error headings; reset-epoch
and source-attributed large-payload regressions run in the unit suite. See review.md for independent
verification statuses and /tmp/nx-full-review-*-third.log for the latest local browser/build evidence.

The fresh full-change Claude CLI review is closed after three fix/verification passes in reviewer
session 2e3fd5ea-4d4d-4afd-ac2d-833e53b95385. All 37 new findings RF37-RF73 are fixed and independently
verified; earlier findings retain their Verified/Resolved statuses. Final verification reported zero
reopened findings and zero new findings. Development and production Chromium, 169 playground tests,
25 examples, strict OpenSpec validation and diff checks pass after the final TypeScript-only fixes.
No commit, archive or deployment was performed.

## Naming refinement

GraphicsCommon -> GraphicsElement, ShapeCommon -> Shape, Anchor -> AxisAlignment; RegularPolygon
and Star use pointCount. Private helpers now read isFinite, normalizeDegrees, toPeriodicRadians,
pointOnCircle, arcPathData and ringSectorPathData. Signed degree normalization and generated
geometry are unchanged. No compatibility aliases are retained. Updated active proposal declarations,
reference docs, playground/native examples and helper tests. Historic review reports, archived specs
and unrelated illustrative editor fixtures retain their original wording.

Rebuilt WASM/Node linked library images and regenerated walkthrough manifests/provenance. Both
walkthrough PNGs compare byte-for-byte equal with the pre-rename files. Native checker comparison,
Rust/TypeScript IR helper tests, graphics contract tests, playground examples and website code
blocks pass. The new public names and rejection of legacy count arguments have compiler coverage.

Naming validation is green: Rust workspace/all-targets 3,458 passed (10 ignored), graphics API4,
Rust IR helpers4, TypeScript IR helper/construction-origin suite, playground169 +25examples,
WASM SDK132, Node SDK38, website13 +223NXblocks, native renderer/CLI tests, and development/
production Chromium. Release/debug-trap WASM and Node native builds, typecheck, workspace/native
Clippy, rustfmt, strict OpenSpec, fixture regeneration/comparison and diff checks pass.
Independent naming-review results are appended to review.md; logs are /tmp/nx-naming-*.log.

Naming review closed in independent Claude CLI session eef90bab-c5ba-4698-8603-b7d5e3a20204.
RF74-RF76 (doc wording, inheritance prose and negative-coverage precision) were fixed and verified
by that same reviewer, with no reopened or new findings. All final SDK and development/production
browser checks passed after the final rebuild; all 50 tasks are complete.


## Scope cleanup after user review (2026-10-29)

Removed the generic HIR interpreter fixes retained from the early implementation. Both
nx-interpreter/src/interpreter.rs and the related nx-api/src/nx_text.rs comment now match HEAD.
Graphics runtime acceptance for imported records, defaults, payloads and Element children lives in
nx-codegen's Rust IR tests; nx-api retains compilation/registration/identity checks.

Removed the unused playground `evaluate` worker request and HIR evaluateSource function, along
with their obsolete public result/function types. Shared limits/diagnostic/formatting helpers live
in compile/policy.ts. Worker/channel/editor tests now exercise the TypeScript IR flow, and the
workbench's artifact-retention helper is private and its compiler callback is now required. The warning regression is covered through the
same IR Output projection as the page; action-handler values use their IR structured spelling.

Removed the unused direct nx-hir dependency from nx-graphics-skia (compiler internals may still
use HIR transitively). Kept the native CLI: graphics-native CI builds it and the Python end-to-end
walkthrough invokes it to test PNGs, font measurement, manifests, inspection and publication.
A narrow .gitignore exception exposes its src/bin directory so a normal add includes the Rust
source; it is not generated output. The CLI evaluates with nx-ir-runtime and renders through the
optional native Skia adapter; the playground continues to use Canvas 2D.

Validation after rebuilding the WASM compiler:
- Rust workspace/all-targets: 3,458 passed, 0 failed, 10 existing ignored tests.
- Playground: 169 passed; all 25 example snapshots matched; typecheck and production build passed
  (the existing bundle-size warning remains).
- Production Chromium integration passed Canvas pixels, icons, revisions, failures, bidirectional
  selection, component actions/timeline, recovery, DPR text bounds and resource-limit checks.
- Focused WASM graphics/metadata SDK tests: 4 passed against the rebuilt module.
- Native-feature suite: 17 neutral tests, 8 Skia tests and 1 preview CLI unit test passed.
- Rebuilt preview CLI and Python end-to-end walkthrough passed, including unchanged PNG/manifests,
  text measurement, stale inspection, spoof rejection and failure-safe publication.
- Native-feature Clippy, workspace fmt, diff whitespace, deterministic fixture/schema checks and
  strict OpenSpec validation passed.

Independent Claude CLI review (session ea87c530-f528-4214-80a5-d5bff53410a6) opened RF77/RF78:
strengthen migrated IR value/default/canonical-name assertions and correct a stale README test label.
Both were fixed and verified; the verifier also reported RF79 for stale compilation-flow comments,
which was fixed and verified in a second round. RF77-RF79 are all Verified; none remain Open or Fixed.
The new consumer-typed regression also covers a g.Transform parameter/return and g.ViewBox record
field beside a local decoy. All 7 focused Rust IR tests pass after these additions; the reviewer
independently reran those tests plus 61 playground workbench/routing/session/policy tests and typecheck.
All 169 playground tests and 25 snapshots also passed after requiring the compiler callback.
All 54 implementation tasks are complete; the change remains unarchived and uncommitted.


## Archive completion (2026-10-29)

All 54 tasks are complete and RF1-RF80 are Verified or Resolved, with no open findings.
Synced graphics-library, graphics-preview and graphics-rendering into new main specs, and merged
source-tree, visualizer-declarations and visualizer-library updates while preserving unrelated
requirements and scenarios. All 90 main specs passed strict validation before archiving.
Archived this change as 2026-10-29-add-nx-graphics-core-and-direct-skia, preserving its schema
metadata and review history. Updated relative links for the archive location. No implementation
code, Git staging, commits or deployment changed as part of archival.
