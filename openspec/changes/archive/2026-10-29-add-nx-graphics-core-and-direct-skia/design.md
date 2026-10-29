## Context

See proposal.md for motivation and the three delta specs for behavioral contracts. The math prerequisite is delivered at `e46b846`; both math and source-authored external declarations are implemented and archived. See [math-readiness.md](math-readiness.md) for the verified baseline and remaining integration checks. The existing compiler carries ordinary NX standard-library sources, external components evaluate to typed values, and the Rust IR runtime is already used by the sibling DrawnUi.FiddleEngine Rust player. The proposal's consolidated declarations are illustrative: they import UI sizing and contain historical syntax-gap notes; they are not production-ready source to copy verbatim.

Reference checkouts inspected: `~/src/DrawnUi.FiddleEngine/nx/rust/src/player.rs` and `nx/rust/Cargo.toml` show the host/runtime boundary; `src/DrawnUi.Fiddle/FiddlePresets.cs` demonstrates direct `WhenPaint` access to `SKCanvas`; `~/src/DrawnUi.Rust/drawnui/src/paint.rs`, `fonts.rs`, `testing.rs`, and workspace Cargo.toml show direct Rust Skia painting, font handling and offscreen tests. These are references, not build dependencies. The supplied spelling `~/src/DrawnUI.FiddleEngine` resolves locally as `~/src/DrawnUi.FiddleEngine`.

## Goals / Non-Goals

**Goals:** compile ordinary NX authoring, prepare a bounded semantic scene, paint directly into a supplied Skia canvas, and expose a useful local preview/measurement loop. Keep independent layers small enough to test and replace.

**Non-Goals:** do not redesign generic values/IR, introduce an interpreted drawing language, require the UI layout engine, implement a remote protocol, or claim the full proposed catalog. No sibling repository changes or releases are required.

## Decisions

### 1. Standard library and implementation profile

Carry unstable `@nx/graphics` through the existing source registry. Use ordinary exported external components for Drawing, Group, Rect, Circle, Ellipse, Line, Polyline, Polygon, Path, and Text. Production Drawing has its own viewBox/fit/contentAlignment/background/opacity/accessibility/children contract; the host viewport determines presentation size. Do not inherit UiCommon or expose ineffective margin/grid/grow props. Shared values can be local/re-exported now; extracting a common stable library is deferred.

The `graphics-core` profile supports none/solid paint, specified centered strokes/dashes, affine transforms, nonzero/evenodd fills and clips, isolated opacity, and single-line positioned text. Full SVG path grammar includes arcs. Reject unsupported gradients, shadows, images, multiline text, non-alphabetic baselines and nonzero letter spacing instead of ignoring them. In this profile Text has font family/size/weight/style, start/middle/end anchor, fill, transform/clip/opacity and literal content; stroke text and effects are deferred. Profile/version is included in tool output. An unstable source header, reference page and release note must identify the compatibility status.

Alternative: copy the full proposed catalog and allow backends to ignore fields. Rejected because agents would generate valid-looking but incorrectly rendered content.

### 2. Neutral semantic preparation and direct destination

Add `nx-graphics` independent of Skia and compiler internals. Accept evaluated module-aware NX values plus a declaration resolver/table from the linked program. Whitelist known graphics identities, not display strings or import aliases. Parse native value shapes intentionally; do not change generic runtime serializers. Prepare semantic nodes, local transforms, paints, parsed path geometry and text layout through host services. Enforce limits and reject errors before submitting paint operations.

Expose a frame-scoped drawing destination with balanced transform/clip/layer scopes and explicit fill/stroke/text operations; public neutral data owns no Skia objects. Drawing consumes prepared nodes and calls destination operations directly in paint order. Preparation retains high-level structure and cached geometry, not a flattened standard NX display list. No recorder is required. Preserve accessible names and original text in inspection data alongside visual geometry.

Use f64 for neutral coordinates; check conversions to backend scalar ranges and transformed bounds before drawing. Set root scale/fit exactly once. Record conservative bounds where tight bounds are unavailable and mark their kind. An inverse matrix is not required to accept a finite singular transform; collapsed geometry is empty/conservative as appropriate, not an unchecked inverse failure.

Alternative: mandatory command recording or mapping through all DrawnUI controls. Both obscure direct-path costs and inherit unnecessary UI/layout dependencies.

### 3. Separate Rust Skia adapter and real local host

Add `nx-graphics-skia` with a compatible published rust-skia binding pinned in the implementation lockfile after verifying native build availability. Use upstream bindings unless a concrete required feature justifies the DrawnUI fork. Do not depend on sibling source paths or their local Cargo patches. The neutral trait is not tied to either package's Canvas type. Adapter constructors accept a host-supplied canvas, and offscreen helpers create a raster surface and encode PNG. Match stroke, path, layer and antialias configuration explicitly; use save-count guards on every return/error path. A backend failure cannot generally undo pixels painted to a borrowed canvas: restore state, return failure, and let the host decide presentation. The preview host paints offscreen and atomically publishes only completed frames.

Add an opt-in `nx-graphics-preview` executable in the adapter package, with compiler access confined to its binary target, or an equivalent example target. It evaluates source through nx-api and links emitted standard-library modules via existing APIs. Inputs: entry/root, viewport, scale, font manifest, limits and output path. Output: PNG plus JSON manifest. Runtime evaluation supplies the transitive math image and preserves numerical diagnostics; include linked library versions through the existing program fingerprint when deriving preview revisions. Reuse Rust/TypeScript runtime implementations; .NET hosts continue to call Rust through the existing binding. Provide `--measure-text` using the same font/run configuration; no network, model key, window system or running fiddle required. Default compiler/SDK consumers do not acquire a Skia link dependency; include adapter checks explicitly in CI.

This is a real host and the initial agent tool surface. A DrawnUI application can later pass its canvas to the adapter; an integration in the sibling repository is not an acceptance dependency.

### 4. Minimal shaped text now

A host registry maps logical font family/weight/style to supplied font bytes with content identities. Prepare single-line text using Skia's shaping stack, retaining glyph positions and font handles behind the neutral host service. Require font coverage; missing selection/coverage is a diagnostic rather than implicit system font fallback. Use a redistributable pinned fixture font, preserve its license, and test at least a ligature/combining sequence plus a supported non-Latin run. Letter spacing, paragraph layout, text strokes and baseline variants are deferred explicitly.

Measurement returns advance, ink bounds, ascent/descent and baseline in local logical units; placement uses advance for textAnchor and painting uses the measured shaped run. Same settings/font identity are included in measurement and preview responses. Reject newline and unsupported props. This bounded capability is more useful than guessed metrics and avoids pulling a full UI text layout engine into the first change.

### 5. Pure authoring helpers instead of renderer types

Implement RegularPolygon, Star, Arrow, Arc and RingSector as ordinary NX component/function bodies that produce Polygon/Path nodes. Parameters and edge cases follow docs/drawnui-proposal/authoring-and-preview.md. The renderer never switches on helper names. Import the delivered `@nx/math` library for sin/cos, hypot/sqrt, atan2 where needed, angle conversion, and numeric helpers. Graphics source modules register no runtime math implementations of their own; their helpers remain ordinary `let` functions or component bodies, not new `external` functions. Public graphics angles remain clockwise degrees; convert explicitly to math radians. Do not copy private polynomial/Newton approximations into graphics or expose host-only geometry callbacks. Keep generated point deviation bounded by 1e-5 * max(1, characteristic length), including path/coordinate conversion error, and run the same helper fixtures in Rust and TypeScript. The existing math module owns its `standard-math-v1` bodies; graphics modules retain ordinary calls/references and declare only features their own nodes need. Emit and link the transitive `@nx/math/math.nx` image using existing library identity/version APIs. A client importing only `@nx/graphics` must not need to import or install math explicitly. Missing images and unsupported runtime features must fail normally before any paint operation; do not duplicate math bodies into graphics.

Use floating-point arithmetic before division for angular steps, for example `360.0 * index / count`; NX's exact `int`-to-`float64` widening already supports bounded point counts and loop indices. Positive sine increases y in the graphics y-down coordinate space, so converting degrees once gives the specified clockwise convention without another sign flip. Reduce periodic rotation/start angles in degree space where needed before conversion, while preserving the original signed sweep so full circles do not become zero sweeps. Prefer `hypot(dx,dy)` over squaring coordinates. Preserve math's finite/domain/overflow diagnostics and source origins; graphics-specific checks remain additional requirements. In particular, `atan2(0,0)` is valid math and does not validate an Arrow: reject coincident endpoints before normalization. Math also does not validate polygon counts, radius ordering, sweep limits, or Skia scalar conversions.

Reuse the independent numerical corpus and renderer-free [geometry example](../../../../examples/math-graphics/main.nx), then add full helper fixtures; the example is a numerical baseline, not an implementation of the graphics helpers. Keep the mathematical reference tolerance (`1e-12 * max(1, abs(reference))`) separate from the graphics construction/conversion tolerance above. No new numerical intrinsics or graphics IR opcodes are introduced. Native scene preparation and the Skia adapter may use native numerical facilities behind the neutral contracts; neither needs an NX VM merely to paint already-evaluated geometry.

Arc output uses SVG arc commands; full circles use multiple segments. Ring holes explicitly choose fillRule. Arrow constructs a filled straight polygon with no route solving. Helpers accept shared appearance props and tests demonstrate edits to parameters. They do not promise preservation of Figma parametric object types.

### 6. Revision-linked inspection and evaluation

Generate deterministic scene-local IDs from evaluated child paths, scoped to a content/config revision. Preserve source/value origin when available; expanded helper nodes can share provenance. Do not promise IDs survive reorder/insertion. Tool inspection requires the corresponding revision; stale requests fail. Geometry, painted and clipped bounds are distinct. Report viewport intersection as conservative when clipping is not geometrically resolved. Overlap warnings are heuristic opt-in; invalid numbers and unsupported effects are errors.

Keep deterministic render/geometry fixtures in CI and a versioned prompt/revision corpus for model trials. Agent trials record model/version, budget, seed where available, font/renderer config, artifacts, metrics and rubric; they are optional and never a network requirement for CI. No quality improvement is claimed until those trials are run.

## Risks / Trade-offs

- Native Skia builds and platform font variation → optional dependency boundary, pinned fonts/toolchain, raster CI lane, fail clearly on unavailable native dependencies.
- Narrow text coverage → strict font manifest and supported profile, explicit missing-glyph errors, later fallback expansion.
- Declarative proposal and production syntax drift → compiled examples, declaration checks, normalized-value fixtures and updated docs; do not preserve known-stale language-gap prose as normative.
- Complex geometry/large opacity layers → preflight finite/range/depth/area/command limits and backend state guards.
- Cross-runtime numerical differences and approximate bounds → use @nx/math conformance plus geometry tolerances and conservative flags; do not present estimates as exact.
- Broad first slice → limit paints/text as above; gradients, images, shadows, UI and recording remain follow-ups.

## Migration Plan

Add unstable libraries/packages without altering existing generic APIs. Build and test neutral/compiler paths separately from optional native Skia. Publish an executable/example and docs with no changes required to existing users. Rollback removes the opt-in graphics packages/imports; no persisted stable graphics wire format is introduced. A later revision must version the implementation profile when observable semantics change.

## Runtime scope clarification (2026-10-28)

Per user direction, this graphics change targets the Rust and TypeScript IR runtimes. The legacy
HIR interpreter will not be updated for graphics support; its import/type-matching limitations are
accepted and do not block this change. `retire-hir-interpreter` remains a separate later change.
The preview compiles with nx-api/nx-codegen and evaluates linked images with nx-ir-runtime directly.
Graphics runtime acceptance and negative tests use IR, with no HIR parity requirement.

## 7. Browser playground rendering

An artifact-scoped WASM operation accepts the TypeScript IR value, construction origins, bounded
browser text measurements and viewport configuration. A preflight validates identities and scene
limits before collecting text requests; only a second measured prepare can return a scene. It resolves node declarations against that
artifact's preserved module/declaration table and runs nx-graphics preparation. The browser checks
non-finite values before JSON encoding. No generic IR/value encoding changes or HIR evaluation
are needed.

The graphics previewer in the Preview tab uses the built-in Canvas 2D API, with no CanvasKit or Rust/Skia browser player.
Compilation and shared neutral preparation retain the existing compiler WASM module; rendering
adds no WASM dependency. Paths, transforms, clipping, view-box placement and bounds use the shared
Rust core. Each translucent subtree paints to an isolated viewport-sized canvas, then composites
once under the parent clip. Preparation bounds peak concurrently allocated layer area. Paint a complete offscreen
frame before publishing it; current errors and cancelled or outdated requests clear/ignore output.

Bundle the pinned face and license. The worker and page explicitly load the same font bytes before
measurement/painting, use identical Canvas 2D typography settings and reject unsupported selections
or missing glyphs. The renderer config is browser-canvas2d-v1. Browser shaping/raster differences
are explicit; native Skia remains the PNG reference. Inspection exposes revision-scoped element
identity, source provenance, geometry/paint/visible bounds and text metrics. Text ink bounds come
from browser actualBoundingBox metrics and are conservatively classified.

## 8. Shared source/output selection

`preview/selection.ts` defines source sites (module + UTF-8 span), source selections scoped to the
exact source snapshot, and renderer-local target IDs associated with one or more sites. Selection
matches every target inside the authored span; selecting a property falls back to the smallest
enclosing authored site. Rendering adapters own highlighting, revealing and their local IDs.
Graphics nodes use scene-local IDs; structured values use current formatted record-node indices.
Selection mappings never use display type names as identity and never change the source mode.

TypeScript origins optionally capture call-site ancestry through `includeCallSites`, leaving
existing default reports and Rust/TypeScript origin conformance unchanged. Construction sites
continue to resolve primitive declarations; source navigation uses the first authored site, and
selection can match outer invocations as well. No IR or generic value encoding changes are made.

Static structured roots use TypeScript IR in Output. Their records are
formatted with existing SDK value annotations, enriched with construction/call provenance, and
shown with `nx-value`. The adapter maps selections to record indices, reveals folded ancestors
and highlights all matches. Stateful component output uses the same mapping. No domain renderer
is required for Survey Question/Choice navigation; dedicated form/step rendering remains future
adapter work. Stale source snapshots cannot navigate or highlight a replacement output.

Review refinements: opacity limits count peak concurrent physical layer area; zero-opacity subtrees
are validated but never allocate rendering layers. Both hosts share an immutable constructing-
declaration span index from nx-api. Source-mode editor selections also drive the common selection
mapping. Public SDK preparation DTOs are typed and unstable, with SDK/runtime/native package tests.

### Generic preview registry and shared browser evaluation

The playground runs one document/session evaluation in the TypeScript IR worker. Output formats its
current value; Preview passes that same value and construction provenance to a registered adapter.
Graphics matches the resolved @nx/graphics Drawing declaration, never a display-name-only heuristic;
component inspection is a separate adapter for a running component. Generic records and primitives
are Output-only unless an adapter explicitly supports their type. Shared source selection works in
Output as well as specialized previews. The first registry ships graphics and component adapters;
no survey form renderer is implied.

Preview is the initial view for supported results, with Output always accessible. A document-level
user preference survives edits, while opening another document resets the preference. When Preview
is unavailable only the Output heading is shown. Compilation/evaluation failures preserve the last
known preview availability and mark old values stale; graphics errors clear the scene while keeping
Preview and the raw value available. Each answer, source span, scene, and selection is scoped to its
source snapshot. Stateful interactions and timeline movement update the value shared by both tabs.

### NX-authored preview metadata and graphics icons

ElementVisualizer accepts an optional PreviewVisualizer with a literal renderer
ID. Resolve it through existing program visualizer precedence and inheritance, alongside title/icon
metadata. An opt-in source-tree query includes otherwise unreferenced program visualizer targets,
so hosts can inspect an evaluated result produced inside an imported helper. The playground caches
this metadata per build, matches the result's nominal declaration identity, and activates only
renderer IDs the host bundles. NX metadata never supplies executable code or network locations.

Element-style helper functions may declare a FunctionVisualizer for their invocation icon; paren
functions remain outside that facility. Function preview behavior comes from the returned type.
Use existing Lucide outline symbols for graphics, guided by Figma's layer/shape conventions.

### Full-review validation and failure refinements

The supported neutral coordinate cap is 1e7; hosts may tighten it. SVG arc normalization uses
corrected radii and charges expansion against the remaining scene budget before appending cubics.
Fill bounds exclude isolated closed movetos, while round/square stroke bounds account for cap dots.
Hosts can configure `text_ink_padding` in physical pixels (default 0); browser Canvas uses 1.5 after
world mapping and before clipping to cover raster hinting. Browser raster scale adapts to the
physical surface/layer budgets without changing logical coordinates, and scene node/depth/point
limits carry element/source diagnostics before the bounded JSON bridge.

Document resets advance a page request epoch, preventing late answers or failures from restoring a
previous document. Reload carries the visitor's component choice as a fallback for a fresh worker.
Staleness follows the last answered source/diagnostic pair; pending work has a quiet indicator, while
navigation still guards exact source snapshots. Formatting errors preserve evaluated values and
preview capability and use a shared display-error category across both tabs.

### Graphics naming refinement

Use domain names GraphicsElement and Shape for the public abstract external component hierarchy.
Drawing remains a separate viewport root. AxisAlignment is the one-dimensional start/center/end
choice; ContentAlignment combines two axes. RegularPolygon/Star use pointCount (star outer tips).
Private helpers use isFinite, normalizeDegrees, toPeriodicRadians, pointOnCircle, arcPathData and
ringSectorPathData. These are source/API renames without geometry or renderer semantics changes.
No compatibility aliases are retained for this unstable library. Rebuild linked SDK/library images
and regenerate provenance/version-bearing walkthrough manifests after the source edits.
