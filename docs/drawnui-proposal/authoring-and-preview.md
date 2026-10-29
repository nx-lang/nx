# NX graphics authoring and preview contracts

The initial graphics-core profile is implemented for Rust and TypeScript IR runtimes. See the
[production reference](../../sites/website/src/content/docs/reference/libraries/graphics.md) for the
supported declarations, preview command and limitations. This document also describes future
catalog goals; the illustrative `.nx` files retain the larger catalog. The legacy HIR interpreter
will not be updated for graphics and is not an acceptance path.

## Authoring conventions

The production library is unstable `@nx/graphics`. Expose helper functions/components as ordinary NX source in that library; they return primitives, never new host-only types. There is no dependency on `@nx/ui`, a model service, or Figma. Positions are local logical units, y increases downward, and angles are degrees clockwise from +x, before node transforms. Shape helpers accept explicit fill/stroke/transform props and preserve their authored call's identity in preview provenance where the existing source origin is available. They are not external components requiring renderer implementations.

| Helper | Essential parameters | Rules |
|---|---|---|
| `RegularPolygon` | center, radius, point count, rotation | Count >= 3; radius >= 0. Vertices are equally spaced; rotation locates the first vertex. Lowers to Polygon. |
| `Star` | center, outer/inner radius, point count, rotation | Count >= 3; 0 <= inner radius <= outer radius. Count is the number of outer tips; alternate inner/outer vertices. Lowers to Polygon. |
| `Arrow` | start/end points, shaft width, head width, head length | A filled straight arrow, distinct from an open stroked line's cap. Widths >= 0, head width >= shaft width. Require head length <= endpoint distance; coincident endpoints and negative dimensions are errors. Lowers to Polygon. |
| `Arc` | center, radius, start angle, sweep angle | Open stroked circular arc; signed sweep in [-360, 360], radius >= 0; default fill none. Zero sweep paints nothing; full sweep must lower to multiple SVG arc segments. |
| `RingSector` | center, outer/inner radius, start angle, sweep angle | 0 <= inner <= outer; signed sweep in [-360, 360]. Inner=0 gives a sector; abs(sweep)=360 gives a full ring. Lowers to a compound Path with explicit fill rule; zero sweep paints nothing. |

Defaults must be documented in the implementation reference and appear in generated catalog instructions. Every helper needs fixtures for its parameters, degenerate inputs, and requested revisions. Coordinate construction uses the delivered unstable `@nx/math` library, with explicit conversion from graphics degrees to math radians. [The math change](../../openspec/changes/archive/2026-10-28-add-math-standard-library/proposal.md) is implemented and archived at `e46b846`; see the [graphics integration baseline](../../openspec/changes/archive/2026-10-29-add-nx-graphics-core-and-direct-skia/math-readiness.md). Reuse it without private numerical approximations. Graphics still validates its own counts, radii, sweeps and coincident Arrow endpoints; math accepting atan2(0,0) does not make a zero-length Arrow valid. Do not quietly implement authoring helpers as opaque Skia callbacks. An initial preview may support a helper via NX-generated points/path data, but may not claim it works until that ordinary NX source compiles and evaluates.

Independent rectangle corner radii, routed connectors, image paints, and reusable geometry definitions are pre-stability follow-ups, not additional backend operations in this first slice. Text anchoring and a same-font measurement query come before automatic label routing or a constraint solver.

## Preview and measurement

Provide a local host executable/example and reusable APIs that an agent harness can wrap. Inputs are a source entry/root, viewport, device scale, host font registry, limits, and output destination. Do not start a model session or fetch assets implicitly. Use the existing compiler/runtime for evaluation; do not add graphics-specific IR instructions.

A successful response includes a PNG artifact path, document/scene revision, profile, viewport/device scale, font identities, render configuration, diagnostics, and an element table. Each entry has a scene-local ID, parent/paint order, declaration identity, provenance when available, local geometry bounds, transformed paint bounds, clipped visible bounds or a declared conservative estimate, and text metrics where applicable. Distinguish advance from ink bounds; a glyph can extend outside its advance box. Bounds are in documented coordinate spaces, with viewport bounds in logical units. Empty/fully clipped content must be distinguishable from an error. Never label an axis-aligned clip-bound intersection as exact visible geometry.

The first identity guarantee is deterministic IDs for unchanged evaluated structure and configuration, scoped to a revision. Structure changes may change generated IDs. The tool rejects a stale revision when inspecting a previous preview. Persistent authored IDs and source-edit addressing are a separate pre-stability decision; do not imply stable cross-edit identity from positional IDs. For helper expansions, provide many-to-one provenance where possible; absence of source origin is explicit, not fabricated.

Text requests use exactly the same host font selection, shaping, font size, and metrics as rendering. The first profile supports single-line runs with a supplied font, glyph shaping, anchors and alphabetic baseline; newline, missing font/glyph coverage, unsupported typography and unsupported baseline options are explicit errors. Missing glyphs must not silently become tofu. Broader fallback and multiline support come later. Prepared text retains the glyph positions and font resources used to measure; no reshaping with different defaults at paint time.

Preparation errors return diagnostics and no new successful frame. Painting errors restore host state and do not publish a partially successful preview. Bounds/clip warnings use element IDs; overlap warnings, if provided, are opt-in heuristics. Zero external calls are required for the deterministic fixture suite. A small SVG reference fixture set and versioned agent task corpus accompany the renderer.
