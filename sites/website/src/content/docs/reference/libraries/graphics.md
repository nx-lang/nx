---
title: '@nx/graphics'
description: 'Unstable standalone graphics authoring, direct Skia rendering and local preview.'
---

`@nx/graphics` is unstable: declarations may change incompatibly in any NX release. The
`graphics-core-v1` profile targets the Rust and TypeScript IR runtimes. The legacy HIR interpreter
will not be updated for graphics; `nxlang run` and SDK APIs that still use it are not graphics
acceptance paths until the separate interpreter-retirement change lands. Use the preview below
or emit/link IR directly. The TypeScript runtime evaluates authoring helpers; direct Skia painting
is provided by the optional Rust adapter.

```nx
import "@nx/graphics" as g
let root() = <g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>>
  <g.Star cx=50 cy=50 outerRadius=40 innerRadius=20 pointCount=5
          fill=<g.Paint.solid color="#2563eb"/>/>
</g.Drawing>
```

Drawing, Group, Rect, Circle, Ellipse, Line, Polyline, Polygon, Path and Text have no UI/layout
library dependency. The host supplies viewport dimensions and device scale. Drawing supports
viewBox, contain/cover/fill, start/center/end alignment, background, opacity, accessibility and
children. Shapes support solid/none fills, centered strokes with caps/joins/dashes, local affine
transforms, SVG path clips and independent nonzero/evenodd winding rules. Child order is paint
order. Opacity isolates the complete subtree. Zero-width strokes paint nothing; odd dash lists
repeat, and all-zero lists are solid. SVG path commands M/L/H/V/C/S/Q/T/A/Z accept absolute and
relative forms. Open paths close for filling and remain open for strokes.

Transforms concatenate in SVG order: translate(10,0), then scale(2,2) maps local (1,0) to (12,0).
Closed shapes and Text default to black; Line and Polyline default to no fill. This Polyline default
differs from SVG. Colors are CSS solid color strings. Rect's omitted ry uses rx; corner radii clamp
to half the corresponding dimension. Clip paths are path-data strings in the node's local space.

Text is a shaped single-line run at an alphabetic baseline, with an explicit supplied font family,
positive fontSize (default 16), exact host fontWeight/fontStyle selection (default 400/normal),
and start/middle/end textAnchor. Stroke text, line breaks, letter spacing, alternate baselines,
rich text, images, gradients, shadows and filters are unsupported and diagnosed. There is no
implicit system-font fallback. The fixture font covers Latin, combining marks and Greek; unsupported
glyphs fail explicitly. Host registrations select exact family/weight/style combinations.

## Shared graphics types

`GraphicsElement` is the abstract base for elements inside a drawing, with transform, opacity,
clipping and accessibility. `Shape` extends it with fill/stroke properties for geometric shapes.
`Group` and `Text` extend `GraphicsElement`; `Drawing` is the standalone viewport root.
`AxisAlignment = start | center | end` positions content along one viewport axis, and
`ContentAlignment` combines its `x` and `y` values. `TextAnchor` remains a separate text concept.
`pointCount` counts RegularPolygon vertices or Star outer tips; a five-point star emits ten vertices.

## Ordinary NX shape helpers

The helper catalog is inspired by [Figma's polygon, star, arrow, and ellipse arc tools](https://help.figma.com/hc/en-us/articles/360040450133-Shape-tools).
This is an authoring influence, not Figma API compatibility or import/export support. Helpers produce
NX polygons and paths, without retaining Figma parametric node types. NX uses explicit radii in local
units and clockwise angles in degrees; [Figma's arc data](https://developers.figma.com/docs/plugins/api/ArcData/)
uses a normalized inner-radius ratio and starting/ending angles in radians.

All helpers evaluate to core primitives; no renderer callbacks or graphics IR opcodes are involved.
They import `@nx/math` transitively. Coordinates use y-down logical units and clockwise degrees
from +x. Periodic start/rotation angles are reduced separately from signed sweeps. Use element syntax
for Point values (`<g.Point x=1 y=2/>`).

| Helper | Parameters and defaults | Rules |
| --- | --- | --- |
| RegularPolygon | cx, cy, radius, pointCount; rotation=0 | pointCount 3..4096; radius >= 0 |
| Star | cx, cy, outerRadius, innerRadius, pointCount; rotation=0 | pointCount is outer tips, 3..4096; 0 <= inner <= outer |
| Arrow | x1, y1, x2, y2; shaftWidth=2, headWidth=6, headLength=8 | Distinct endpoints; widths >= 0, headWidth >= shaftWidth, headLength <= distance |
| Arc | cx, cy, radius; startAngle=0, sweepAngle=90 | Open, fill none; radius >= 0, signed sweep -360..360 |
| RingSector | cx, cy, outerRadius; innerRadius=0, startAngle=0, sweepAngle=90 | 0 <= inner <= outer; signed sweep -360..360; evenodd hole |

Helpers accept stroke/transform/opacity/clipPath/clipRule; all except Arc accept fill. Zero sweeps
are empty and full circles use multiple SVG arc segments. Invalid helper dimensions are diagnosed
during scene preparation. Numerical math failures retain their runtime diagnostic codes and origins.
Geometry tests use `1e-5 * max(1, characteristic length)`, separately from math's reference tolerance.

## Local preview and measurement

On Linux with FreeType, install the Rust toolchain plus clang and ninja if Skia's published binary
cache is unavailable. The lockfile pins upstream `skia-safe 0.153.3`. Build once:

```sh
cargo build --locked -p nx-graphics-skia --features preview --bin nx-graphics-preview
```

After building, preview needs no network, system fonts, sibling checkout, model key or window system:

```sh
target/debug/nx-graphics-preview --entry examples/graphics/main.nx --output /tmp/nx-preview \
  --width 400 --height 240 --scale 1
target/debug/nx-graphics-preview --measure-text \
  '{"text":"NX graphics","family":"Fixture","size":18,"weight":400,"italic":false}'
python3 -m pip install jsonschema==4.25.1
python3 scripts/check-graphics-preview.py
```

Use `--source-root directory` for imports outside the entry's directory and `--root name` for another
entry function. `--fonts fonts.json` accepts an array of `{family, weight, italic, path}` registrations;
paths resolve relative to that manifest. Without it, the checked-in DejaVu Sans fixture is registered
as Fixture/400/normal. Its bytes and license are included. `--limits limits.json` overrides the host's
node, depth, path-command, coordinate, surface-pixel and layer-pixel limits. Keys use snake_case:
`nodes`, `depth`, `path_commands`, `coordinate`, `surface_pixels` and `layer_pixels`. Unknown keys
are rejected. `coordinate` may tighten the supported maximum of 10,000,000; values above it are rejected.
Limit validation precedes viewport validation; diagnostics name the invalid field or viewport constraint.
The CLI rejects unknown/duplicate options and accepts both `--width 640` and `--width=640`.
Font manifests are arrays of `{family, weight, italic, path, sha256?, license?, upstream?}`; paths
are relative to the manifest, and supplied hashes are verified. The bundled font manifest uses this format.
The manifest's image is relative to its revision directory; current.json points relative to the output
directory. CLI response paths are absolute so they remain usable from any working directory.

Measurement returns font content identity, shaped advance, conservative ink bounds, ascent, descent
and baseline in local logical units. Center a start-anchored label at x with `x - advance / 2`, or use
textAnchor=middle. The prepared run is reused for painting. The walkthrough script demonstrates a
five-to-seven-tip edit and measured label replacement; retained before/after NX, PNG and manifests
are under `crates/nx-graphics-skia/fixtures/revisions`.

A successful preview writes immutable `<revision>/image.png` and `manifest.json`, then atomically
replaces `current.json`. Errors return a JSON diagnostics response and leave the previous pointer
unchanged. The manifest includes profile/config/fonts, program revision, hierarchy, declaration
identities, source provenance when available, and geometry/painted/visible bounds. Geometry bounds
are local; painted and visible bounds are in logical viewport coordinates. Bounds carry explicit
empty and conservative flags; non-rectangular clip intersections use conservative bounding boxes, not exact hole
geometry. IDs are positional and revision-scoped. Inspection requires both:

```sh
target/debug/nx-graphics-preview --inspect /path/to/manifest.json --revision REV --id element/children/0
```

`nx-graphics` has no compiler or Skia dependency. Hosts implement DeclarationResolver using linked
program metadata rather than trusting `$type` strings, supply a TextService, call prepare, and paint
through Destination. `nx-graphics-skia` provides supplied-canvas painting, retained shaped fonts/runs,
and offscreen PNG output. Borrowed-canvas state is restored on all exits; pixels cannot be rolled
back after a destination failure. Native support is currently validated on Linux. Other targets can
use neutral preparation, subject to their own supplied text/destination implementations.

## Playground

Open [Graphics: shapes and text](/play/graphics). **Preview** is selected automatically for Drawing
values and paints with the browser's built-in **Canvas 2D** API. **Output** shows the same evaluated
value as a structured record. Both use the TypeScript IR runtime. No CanvasKit or Rust/Skia browser
renderer is involved; the existing WASM compiler also performs shared neutral graphics preparation.
Values without a registered previewer show Output only. Choosing Output persists across edits.

Change the Star's `pointCount=5` to `pointCount=7` to see a new revision. Click the canvas or use **Inspect
element** to select an element; expand **Bounds and text metrics** for its identity, local geometry,
viewport painted/visible bounds and measured text. Canvas selection tests fill/stroke geometry and
ancestor clips, with conservative ink bounds for text; the dropdown can select every element.
Selecting a rendering selects its authored expression while keeping Visual or Source active.
Selecting an expression in Visual highlights every matching rendering, including repeated loop
output. Helpers navigate to their authored invocation, while retaining their primitive identity
for validation. This selection contract is also used by the generic structured Output. Invalid source, paths, dimensions and fonts clear the
current image and show diagnostics.

The playground bundles `Fixture` (DejaVu Sans, weight 400, normal) and its license. Missing glyphs
or other face selections produce errors; no system font fallback is intentional. Text measurement
and painting use the same supplied bytes and Canvas typography settings. Browser text metrics and
rasterization can differ from the native Skia preview; `browser-canvas2d-v1` identifies this
configuration. Native Skia remains the PNG/export reference.

## Verification and optional authoring trials

```sh
cargo test -p nx-graphics
cargo test -p nx-codegen graphics_
cargo test -p nx-graphics-skia --features native
pnpm --dir runtime/typescript build
node runtime/typescript/test/graphics.test.mjs
node scripts/verify-graphics-fixtures.mjs
```

The versioned SVG corpus records analytic-coordinate and raster-interior expectations. The
`evaluations/graphics-v1` corpus defines icon, diagram, illustration and infographic generation/edit
trials and their result schema. Deterministic tests need no model key. Quality claims require
recorded model trials; none have been run. The larger DrawnUI catalog is a future proposal, not this
profile's supported API.

Shaping uses unhinted linear metrics with subpixel positioning and baseline snapping disabled,
so font size and transforms scale consistently. Ink bounds union actual shaped-glyph bounds with
a small conservative pad. Fonts caches identical requests; reusable hosts can call `clear_runs`
after retiring prepared scenes. Helper argument failures use a reserved `nx-graphics-invalid: `
Path-data marker, diagnosed as `graphics-helper-invalid` during preparation. In hosts that only
evaluate IR, invalid helper values remain that invalid Path and must not be painted. Diagnostic
element paths locate the expanded helper; provenance identifies the library's constructing
element in native Rust previews. Browser previews also retain TypeScript IR helper call sites,
so selection navigates to the authored invocation.
Node declaration identity is verified by the preparer. Nested shared values (Paint, Stroke,
Transform) rely on the compiler's type-checked IR contract, and are not independently authenticated.

## Presentation metadata

The library declares its preview association in NX:

```nx fragment
visualizer Drawing = <ElementVisualizer
  icon="frame"
  preview=<PreviewVisualizer renderer="nx.graphics"/>
/>
```

The playground bundles the `nx.graphics` renderer. Other hosts may supply their own implementation
or show the structured value when that renderer ID is unavailable. No renderer code is embedded in
the descriptor.

Graphics declarations also supply Lucide outline icons, following familiar
[design-tool layer conventions](https://help.figma.com/hc/en-us/articles/360040449873-Select-layers-and-objects):
frame for Drawing, group for Group, geometric outlines for shapes, a spline for Path, and a T for
Text. Element-style helpers supply their invocation icons through FunctionVisualizer: hexagon, star,
arrow, radius arc, and donut. Their resulting polygons/paths retain their own nominal identities.

Browser raster resolution follows device scale within the physical surface and opacity-layer budgets.
At high zoom or with deeply nested opacity, resolution is reduced while preserving the logical
drawing. Browser painted text bounds include 1.5 physical pixels of raster uncertainty, configured
through `text_ink_padding`; the neutral/native default is 0. Intrinsic font ink metrics remain in
local units and are separate from this host raster pad.
