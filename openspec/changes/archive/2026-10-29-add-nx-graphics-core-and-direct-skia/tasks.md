## 1. Production library and runtime boundary

- [x] 1.0 Record the delivered math/external-declaration baseline at `e46b846` and recheck the existing library, runtime/generator conformance and renderer-free geometry fixtures; see math-readiness.md. Graphics-specific standard-library import/linking remains part of tasks 1.1/1.3.

- [x] 1.1 Add unstable `@nx/graphics` Drawing/Group/basic-shape/shared-value declarations through the existing standard-library registry, independent of UI; import `@nx/math` from the graphics source and register no private numerical implementations; verify a consumer importing only graphics resolves the transitive math library and standard-library identity/version tests pass.
- [x] 1.2 Add the bounded single-line Text declaration/profile and explicit unsupported-feature rules; verify valid and rejected invocations against compiled fixtures and reference docs.
- [x] 1.3 Exercise imported defaults, inherited props, aliases, library IR emission/linking and external-value output; add focused regression fixes only for reproduced blockers and verify the all-module emit contains `@nx/math/math.nx`, its identity/content version links correctly, missing math images/unsupported `standard-math-v1` fail explicitly, and existing math-free/generic serialization fixtures remain unchanged.
- [x] 1.4 Add the neutral graphics package and identity-aware value-to-scene entry point; verify a spoofed same-name declaration is rejected and a genuine aliased declaration is accepted.

## 2. Geometry preparation and inspection data

- [x] 2.1 Implement validated geometry/paint/stroke/affine types and host limits; verify finite/range/dimension/depth/complexity boundary cases fail before any destination operation.
- [x] 2.2 Implement full SVG path parsing/normalization including arcs and open/closed subpaths; verify malformed suffixes, arc edge cases, fill closure and stroke closure fixtures.
- [x] 2.3 Implement standalone viewport fit/alignment, local transforms and clip-rule propagation; verify known-coordinate transform order and contain/cover/fill fixtures.
- [x] 2.4 Implement scene-local IDs, revision/config fingerprints and available provenance; verify unchanged structure is deterministic and reordered revisions cannot be inspected with stale IDs.
- [x] 2.5 Compute geometry/paint/visible bounds with exact/conservative classification; verify stroke expansion, transformed bounds, evenodd clips and empty/fully clipped results.
- [x] 2.6 Define and implement the frame-scoped backend-neutral destination/host-text contracts; verify mock-destination tests for paint order, balanced scopes and absence of mandatory recording.

## 3. Direct Skia adapter and shaped text

- [x] 3.1 Add the isolated Skia adapter package, select/pin a published binding that builds on the target platform, and wire opt-in native checks; verify compiler/neutral package dependency graphs do not link Skia and native adapter builds without sibling patches.
- [x] 3.2 Implement direct solid fills and specified stroke/dash/path operations against a supplied canvas; verify primitive raster fixtures, zero strokes and odd/all-zero dash patterns.
- [x] 3.3 Implement transform/clip/view-box and isolated opacity scopes with entry-state guards; verify overlapping-child opacity images and canvas restoration after success/injected failure.
- [x] 3.4 Add a licensed pinned fixture font and host font registry with content identities; verify font resolution errors, license artifact and zero implicit network/system-font dependence.
- [x] 3.5 Implement single-line shaping, reusable measured runs and glyph coverage checks; verify advance/ink/baseline metrics, a combining/ligature sequence, a supported non-Latin run and missing-glyph errors.
- [x] 3.6 Paint retained shaped runs with start/middle/end anchors and defined text-profile restrictions; verify measured-versus-painted placement and rejection of multiline/unsupported typography.

## 4. Ordinary NX authoring helpers

- [x] 4.1 Build on the delivered `@nx/math` and renderer-free geometry example; explicitly convert degree angles to radians, use float64 angular steps and hypot for distances, and document any atan2(y,x) use. Verify known trig/distance/rotation samples, periodic start/rotation handling without losing full sweeps, and separate graphics geometry tolerances in Rust/TypeScript, with no private approximations or new external helper implementations.
- [x] 4.2 Implement RegularPolygon and Star as ordinary NX helpers lowering to Polygon; verify point-count/radius/rotation fixtures and five-to-seven-tip revision behavior.
- [x] 4.3 Implement the straight filled Arrow helper lowering to Polygon; verify endpoint orientation, shared appearance and invalid/coincident/head-too-long diagnostics, including rejection before zero-length normalization despite math accepting atan2(0,0).
- [x] 4.4 Implement Arc and RingSector helpers lowering to SVG Path; verify signed/zero/full sweeps, inner-radius holes and full-circle multi-segment lowering.
- [x] 4.5 Compile/evaluate a scene using every helper and shared reusable dimensions/styles; verify the prepared scene contains only core primitive declarations and preserves available helper provenance; numerical failures retain math diagnostic categories/call origins rather than substitute zero coordinates.

## 5. Local preview and agent feedback

- [x] 5.1 Add the in-repository opt-in preview executable/example that compiles/evaluates NX and renders offscreen PNG; verify a documented command works offline with only this checkout and the pinned fonts, and supplies the transitive graphics/math images to evaluation.
- [x] 5.2 Return a structured manifest with revision/profile/config/fonts, element hierarchy/bounds/provenance and diagnostics; verify the manifest schema and representative success/error responses.
- [x] 5.3 Add standalone text measurement using the same host/run service; verify an example positions a label using returned advance/ink metrics and matches the rendered result.
- [x] 5.4 Implement failure-safe artifact publication and revision-checked inspection; verify evaluation errors (including math domain/non-finite/overflow), preparation failures and drawing failures never publish partial output or mislabel a previous image as current; verify linked library changes participate in the revision fingerprint.
- [x] 5.5 Add parameter-edit and measured-label-placement walkthroughs using the preview host; verify expected geometry/label placement after each revision and retain before/after artifacts as fixtures.

## 6. Evaluation and release readiness

- [x] 6.1 Add a small versioned SVG reference corpus and analytic-coordinate/raster checks for claimed shared semantics; verify deliberate NX differences are documented and tolerances are explicit.
- [x] 6.2 Add versioned icon/diagram/illustration/infographic generation and revision tasks, result schema and visual rubric; verify deterministic fixture/schema checks run without a model key, and optional trial instructions record model/config/budget/artifacts.
- [x] 6.3 Update graphics reference pages, standard-library instability/release notes and the proposal sources to match the delivered profile; verify all examples compile and public APIs distinguish planned from supported features.
- [x] 6.4 Run the focused compiler/runtime/neutral/native integration suites plus supported-target checks; verify end-to-end NX-to-PNG output, unsupported-feature diagnostics, no recorder requirement and no sibling-checkout dependency, recording commands/results in the change notes.

## Runtime scope clarification (2026-10-28)

Per user direction, this graphics change targets the Rust and TypeScript IR runtimes. The legacy
HIR interpreter will not be updated for graphics support; its import/type-matching limitations are
accepted and do not block this change. `retire-hir-interpreter` remains a separate later change.
The preview compiles with nx-api/nx-codegen and evaluates linked images with nx-ir-runtime directly.
Graphics runtime acceptance and negative tests use IR, with no HIR parity requirement.


## 7. Playground scope extension

- [x] 7.1 Expose artifact-scoped shared graphics preparation through the WASM SDK, resolving identities from linked compilation metadata; test genuine aliases, spoof rejection and error results without Skia or HIR evaluation.
- [x] 7.2 Add browser supplied-font loading, coverage validation and measurement with an explicit browser configuration; verify fonts, multiline/unsupported-face failures and finite metrics.
- [x] 7.3 Add a worker IR Drawing pipeline and stale-response-safe UI tab, preserving component Preview; verify failures clear the current scene and old responses cannot replace a new revision.
- [x] 7.4 Render prepared geometry with HTML Canvas 2D, without a WASM renderer, and expose node identity/bounds/metrics inspection; verify paths, transforms, independent clip rules, isolated opacity and safe text rendering.
- [x] 7.5 Add a discoverable graphics example and playground/reference documentation; run SDK, playground, browser integration and native regression checks, then the independent Claude review/fix/verify loop.

- [x] 7.6 Preserve optional helper call-site provenance separately from primitive construction identities; navigate preview selections in the current source mode and select all renderings of a Visual source expression, including loops and Unicode source offsets.
- [x] 7.7 Share the selection contract across renderer adapters; provide structured IR root previews and Question/Choice two-way selection, including reveal/highlight behavior, stale-source guards and focused tests.

The 38 tasks through section 7 were complete at that review checkpoint. The independent Claude reviewer verified RF18–RF24, then reported RF25;
that fix was also verified with no reopened or new findings. RF1–RF25 are all Verified or Resolved.
Final playground validation: 143 tests, 25 example snapshots, production Chromium integration,
SDK 130 tests and matching release/debug WASM builds. See implementation-notes.md and review.md.

## 8. Generic playground preview routing

- [x] 8.1 Replace Drawing/Preview/Output tabs with conditional Preview and universal Output; select Preview by default when supported, preserve user choice during edits, and hide the switcher when only Output exists.
- [x] 8.2 Evaluate the playground document with TypeScript IR once per revision, share its current value and provenance across Output and registered previewers, and retain stateful component actions/timeline.
- [x] 8.3 Route graphics by resolved declaration identity through a generic previewer registry; move generic structured fallback and bidirectional selection into Output, keeping unsupported values Output-only.
- [x] 8.4 Preserve source/revision guards and preview availability across temporary failures; report renderer failures without losing raw output or publishing stale scenes.
- [x] 8.5 Update examples/docs and verify conditional routing, alias/spoof cases, selection, stateful output, errors, and browser behavior; complete independent Claude review/fix/verify.

## 9. Library-authored presentation metadata

- [x] 9.1 Add typed NX preview renderer descriptors to visualizers, expose resolved metadata through the existing language/SDK contract, and preserve precedence and nominal identities.
- [x] 9.2 Replace the playground's hardcoded type-to-renderer association with library metadata plus a bundled host renderer allowlist; unknown renderer IDs fall back to Output and metadata evaluation executes no renderer code.
- [x] 9.3 Declare the graphics previewer and familiar Lucide graphics icons in the library, including presentation-only icons for element-style helper functions.
- [x] 9.4 Add compiler, protocol, registry and browser coverage; update reference/design/release documentation, rebuild WASM and complete the independent review/fix/verify loop.

The original 47 implementation tasks are complete; the naming refinement adds the tasks below.

## 10. Graphics naming refinement

- [x] 10.1 Rename the abstract bases to GraphicsElement/Shape, Anchor to AxisAlignment, helper count properties to pointCount, and private numerical/path helpers to the agreed descriptive names; update active consumers and docs.
- [x] 10.2 Verify the renamed API on Rust/TypeScript IR, rebuild SDKs, regenerate/compare versioned fixtures, and check playground/browser and OpenSpec artifacts.
- [x] 10.3 Complete an independent Claude CLI review and fix/verify loop for this naming change.

All 50 tasks are complete. Claude independently reviewed the naming refinement and verified RF74-RF76; no naming findings remain open.


## 11. Scope cleanup after user review

- [x] 11.1 Remove the legacy HIR interpreter edits and their associated test-comment change; move graphics runtime acceptance tests to Rust IR while retaining compiler/library registration checks.
- [x] 11.2 Remove the obsolete playground HIR evaluation endpoint, evaluator and public types; keep shared policies in compile/policy.ts and migrate worker/editor/channel tests to the existing IR flow.
- [x] 11.3 Remove the unused direct nx-hir dependency from the native adapter and ensure the native preview CLI source is visible to Git with a narrowly scoped ignore exception.
- [x] 11.4 Rebuild the WASM compiler, verify playground/browser and native Skia/CLI behavior, run workspace regressions, and complete independent Claude review/fix/verify for the cleanup.

All 54 tasks are complete. Claude independently reviewed the scope cleanup and verified RF77-RF79; no findings remain Open or Fixed.
