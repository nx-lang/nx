## Why

NX agents need to produce and revise visible graphics, but the portable graphics catalog currently exists only as a proposal. Delivering direct Skia rendering with a preview/measurement loop and compact authoring helpers will test whether NX improves design generation before the object model becomes a compatibility commitment.

## Dependencies

The math foundation is delivered in commit `e46b846` (2026-10-28). Both
[add-math-standard-library](../2026-10-28-add-math-standard-library/proposal.md) and
[add-external-function-declarations](../2026-10-28-add-external-function-declarations/proposal.md)
are implemented, verified and archived. Reuse the unstable `@nx/math` API, its existing Rust and
TypeScript runtime support, and documented `external` declarations. No math implementation or
language extension is a prerequisite left to build in this change.

[Math readiness and integration notes](math-readiness.md) record the baseline, tests and remaining
graphics-specific work. In particular, `@nx/graphics` must import math as an ordinary transitive
standard-library dependency, and preview evaluation must link its emitted image. This change adds
no private numerical approximations, math operation registrations or graphics IR opcodes.

## What Changes

- Carry an unstable `@nx/graphics` standard library: standalone drawings, groups, basic geometry, paths, and positioned text, with no dependency on the Level 2 UI catalog.
- Add backend-neutral preparation, validation, semantic identity/bounds inspection, and a frame-scoped direct drawing contract; preserve existing generic NX value/IR serialization.
- Add a separate Rust Skia adapter and an in-repository preview host that evaluates NX, renders PNG, and returns structured diagnostics and measurements. Skia remains optional for compiler/SDK consumers.
- Add ordinary NX authoring helpers for regular polygons, stars, arrows, and arc/ring sectors using the delivered `@nx/math`; helpers lower to existing geometry rather than add backend opcodes.
- Add a bounded first text profile with host-supplied fonts, shaped single-line text, and measurement from the exact layout painted.
- Add deterministic fixture tests and a repeatable agent authoring/revision evaluation corpus. Declare unsupported effects explicitly rather than silently degrading them.

## Capabilities

### New Capabilities

- `graphics-library`: Unstable core declarations, ordinary NX authoring helpers, and standalone graphics evaluation.
- `graphics-rendering`: Prepared graphics scenes, validation, drawing-context contract, and direct Skia rendering.
- `graphics-preview`: Preview host, bounds/text measurements, diagnostics, and agent-facing evaluation fixtures.

### Modified Capabilities

- `visualizer-library`: Add typed PreviewVisualizer and presentation-only FunctionVisualizer metadata.
- `visualizer-declarations`: Accept element-style functions as FunctionVisualizer targets.
- `source-tree`: Carry resolved preview descriptors and optionally list otherwise unreferenced loaded visualizer targets.

Existing runtime value and IR encodings are reused; visualizer declarations remain non-executable metadata.

## Impact

- `crates/nx-api` standard-library sources/registry and focused compiler/runtime tests.
- New `crates/nx-graphics` and `crates/nx-graphics-skia` packages, an opt-in preview executable/example, and fixture assets; workspace and packaging configuration as needed.
- Skia and shaping dependencies isolated in the adapter; no Skia types/dependencies in compiler, IR, generic bindings, or the neutral API.
- Graphics reference documentation, examples, proposal/future design notes, and development-only agent evaluation tooling.
- The sibling DrawnUi.FiddleEngine and DrawnUi.Rust checkouts are implementation references only; the deliverable builds and runs within this repository.

## Non-goals

Level 2 layout, UI/A2UI adapters, animation/input/state, image loading, gradients/shadows, multiline/rich text, display-list recording or transport, Figma import/export, and publishing a stable graphics standard are outside this first change.

## Runtime scope clarification (2026-10-28)

Per user direction, this graphics change targets the Rust and TypeScript IR runtimes. The legacy
HIR interpreter will not be updated for graphics support; its import/type-matching limitations are
accepted and do not block this change. `retire-hir-interpreter` remains a separate later change.
The preview compiles with nx-api/nx-codegen and evaluates linked images with nx-ir-runtime directly.
Graphics runtime acceptance and negative tests use IR, with no HIR parity requirement.

## Playground scope extension

The user explicitly expanded this change to include browser playground rendering, and selected
HTML Canvas 2D with TypeScript IR evaluation. Reuse the neutral Rust preparer through the existing
WASM SDK and paint the validated scene through the browser's built-in Canvas 2D API. No CanvasKit,
Rust/Skia browser player, additional renderer WASM, server or GPU setup is required. The existing
compiler and shared preparer remain WASM; the rendering adapter is TypeScript.

The generic Preview tab renders supported values; Output shows their structured TypeScript IR value. Bundle the pinned font,
provide a discoverable example, node bounds/metrics inspection and clear preparation/font/evaluation
errors without showing a stale scene as current. Native Skia remains the PNG/export reference.
Browser text uses supplied-font browser shaping and its own renderer config; native and browser
pixel/metric identity is not promised. The HIR interpreter remains outside this change's scope.

## Shared preview selection extension

Per user direction, source/output selection is a renderer-independent facility. A current output
associates renderer-local IDs with authored source spans; one source expression may produce many
rendered items. The source snapshot scopes selections. Helpers retain their construction identity
for validation and separately report optional call-site ancestry for navigation. Preview selection
preserves the current Visual/Source mode, and Visual selection highlights all matching renderings.

Canvas previews and structured Output use the same contract. Output evaluates structured roots
through TypeScript IR, displays their records and reveals the selected Question/Choice (or any
other record) without a graphics-specific model. Generic structured values are displayed in Output, not duplicated in Preview; a survey-specific form previewer is not included.

## Generic playground preview routing extension

The right pane has universal structured Output and a conditional Preview for registered renderers.
Graphics uses Preview instead of a separate Drawing tab. Preview is initially selected when available;
user choices persist across edits. Unsupported types have Output only. Browser document evaluation
uses TypeScript IR for both presentations, sharing values and source provenance; the legacy HIR
interpreter itself remains unchanged. Existing stateful component inspection/actions are preserved
as a registered component preview, and Output reflects the current rendered component value.

## Library-authored preview and icon metadata

NX visualizer declarations own type-to-renderer associations. A typed PreviewVisualizer descriptor
names a logical renderer ID; the host supplies a bundled implementation. Unknown IDs leave Output
available without loading code. Dynamic installation, executable NX renderer code and module-wide
preview defaults remain future work. Graphics declares its Drawing preview and familiar outline
icons for major primitives and element-style helpers in its own source.

## Naming refinement

Public shared supertypes become GraphicsElement and Shape, the one-axis alignment type becomes
AxisAlignment, and polygon/star helpers expose pointCount. Private helper names describe their
finite checks, degree normalization, point construction and SVG path-data generation. Update
consumers, proposal/reference docs, compiled library images and versioned manifests together.
