## Context

See proposal.md for motivation. `@nx/graphics` already exports Paint.none/solid and imports canonical Color. Neutral preparation currently reduces fills and strokes to RGBA arrays, removes fully transparent solids, and exposes color-only Destination methods. Skia consumes those arrays directly; the playground consumes the serialized neutral Scene through WASM SDK types. Canvas text uses reference-size typography plus a translation/scale, while Drawing backgrounds are viewport-covering geometry in viewBox coordinates. Those details must remain correct when paint acquires geometry.

Graphics acceptance targets Rust and TypeScript IR runtimes, not the legacy HIR interpreter. There is no need for source compatibility shims for this unstable library.

## Goals / Non-Goals

**Goals:** a single validated paint model, consistent geometry/interpolation across native and browser hosts, ordinary NX reuse, bounded work and inspectable output.

**Non-Goals:** change generic value/IR encodings, build a paint-server registry, add CSS inheritance, add a dedicated gradient editor, or change text layout. The remaining feature exclusions are listed in proposal.md.

## Decisions

### 1. Add ordinary Paint variants and a stop record

The public contract is:

```nx
export type GradientStop = {
  offset: float64
  color: Color
}

export type Paint =
  | none
  | color { color: Color }
  | linearGradient { x1: float64, y1: float64, x2: float64, y2: float64, stops: GradientStop+ }
  | radialGradient { cx: float64, cy: float64, radius: float64, stops: GradientStop+ }
```

All coordinates are explicit local user units. Do not add an interpolation or spread property in this release. Inline values and named Paint-returning functions use existing NX syntax and evaluation. The final public case names are Paint.color, Paint.linearGradient and Paint.radialGradient, with no aliases for the earlier solid/linear/radial spellings. Prepared paint tags use the same names. There is no ID/reference indirection: a reusable gradient is an ordinary value. Existing shape helpers already forward Paint and need coverage rather than new geometry implementations.

Require at least two stops; offsets are finite, nondecreasing and in [0,1]. Preserve authored order at duplicate offsets. Missing endpoint stops extend the first/last color to 0/1 and beyond, without rewriting authored data. At a duplicate offset, use the last stop at that offset; approach from below uses the first stop there. Thus hard stops remain deterministic. Gradient vector length and radial radius must be strictly greater than 1/32768 local units, including a float32 rounding margin at backend precision, because Skia silently collapses smaller gradients to a solid. For text, the shared minimum scales by max(1, fontSize/1000) to cover Canvas reference-font normalization. Zero-length linear vectors and nonpositive radial radius are errors; one-stop gradients are errors rather than implicit solids. These explicit constraints avoid backend-dependent degeneracy rules.

Alternatives: bounding-box units are convenient for reuse but complicate text, strokes and degenerate bounds; add later if demonstrated necessary. SVG-style named definitions introduce unnecessary identity/lifetime behavior for values NX can already share.

### 2. Pin sampling to premultiplied encoded sRGB

For a linear paint, t is the projection of the local point onto the endpoint vector divided by the vector's squared length. For a radial paint, t is local distance from center divided by radius. Clamp t to the stop range. Between neighboring distinct offsets, linearly interpolate alpha and encoded-sRGB channels multiplied by alpha, then unpremultiply if alpha is positive. Zero resulting alpha has canonical zero RGB. At a hard stop choose the last coincident stop as above. Input/inspection colors retain canonical unpremultiplied float32 Color values; sampling intermediates use sufficient precision before backend conversion.

This is deliberately one rendering contract, distinct from `@nx/color.mix`'s default Oklab space and `composite`'s linear-sRGB contract. No extra transfer-function conversion occurs while interpolating gradients. Reuse color validation, not a per-pixel runtime call to color.mix. Additional spaces are deferred because exposing choices that one host cannot honor would weaken portability.

First implement a small Skia/Canvas probe with opaque and unequal-alpha stops, colored fully transparent endpoints, duplicate offsets and transformed text. Check premultiplication, explicit Skia interpolation settings and browser behavior against the sampling oracle. The Chromium probe returns purple for opaque red to transparent blue, demonstrating unpremultiplied interpolation. Canvas therefore subdivides unequal-alpha intervals into 64 pieces and inserts limiting RGB values at transparent endpoints. Each segment has premultiplied channel error at most 1/256 before backend quantization; intermediate stops stream directly into CanvasGradient without raster allocation. At most 64 intermediate subdivisions per supplied interval bound extra work by the scene stop budget. Skia uses explicit premultiplied SRGB interpolation through the current gradient API. If adapter behavior differs, configure or implement compatible sampling; do not silently change the contract. Any auxiliary raster path must remain under the existing physical surface/layer budgets, include temporary allocations in peak accounting, and fail explicitly when unavailable. Renderer consistency is a release gate, not a deferred quality task.

### 3. Validate and transport a backend-neutral prepared paint

Replace color-only Node fills, Stroke colors and Destination path/text color parameters with a tagged prepared paint carrying solid channels or validated gradient geometry/stops. Keep absent paint distinct from a transparent gradient. Centralize potential-visibility checks: any nonzero stop alpha means potentially visible; an all-zero-alpha gradient is nonpainting after complete validation. This permits existing bounds and paint skipping to remain conservative without analyzing individual transparent intervals.

Add `gradient_stops` to Limits as a scene-wide total of supplied gradient stops, default 100000. Charge each paint occurrence, including shared values, once per consuming paint field; validate and count even invisible nodes and paints. Validate the limit as a positive integer before preparing content. Stop sequences must be checked against the remaining budget before cloning/building backend resources. Existing coordinate limits apply to endpoints, centers and radius; reject non-finite derived math and backend-unrepresentable values. Reuse canonical Color channel validation, preserving color diagnostic codes and adding element/paint-property/stop-index context.

Update Rust serialization, `bindings/wasm/src/graphics.ts`, native WASM bindings, playground protocol aliases and preview schema together. No Skia/platform handles enter the neutral model. Consumers reject an unsupported profile rather than interpreting old RGBA arrays as new paints. The prepared representation is an unstable host contract, not a new NX display list.

### 4. Keep paint coordinates attached to element geometry

Gradient coordinates undergo the same local-to-viewport mapping as their painted geometry. There is no additional gradient transform. Strokes sample in shape-local space, not distance along the path. Nonuniform element transforms naturally make radial circles elliptical. Drawing backgrounds cover the viewport as today and interpret gradient coordinates in Drawing viewBox units, including contain letterboxing; they do not switch to device pixels.

Skia creates shaders from validated local geometry and applies them to path and retained glyph-run paints. Canvas creates gradients in the element coordinate system and uses them for fillStyle/strokeStyle. For text, compensate explicitly for the internal reference-font translation/scale: subtract the shaped text origin from each gradient coordinate and divide coordinates/radius by the reference-font scale before creating the gradient used by fillText. The Chromium pixel test confirmed that constructing a gradient before those transforms alone does not preserve its placement. Test the result at multiple font sizes/anchors and transformed ancestors. Existing clip scopes and isolated opacity remain in charge of compositing. Native borrowed-canvas state restoration and failed-frame publication rules remain applicable.

### 5. Extend inspection and keep selection geometric

Expose paint kind, coordinates and ordered stops through native inspection and the playground inspector with local-space labels. Preserve authored offsets and Color precision in inspection, even when endpoint extension or shader conversion is internal. No dedicated gradient handles or editing controls are added; ordinary source/value editing and existing stop-color tooling remain available where already supported.

Selection tests the existing fill/stroke geometry and ancestor clips whenever a paint is potentially visible. It does not sample alpha at the click location: transparent portions of an otherwise visible gradient remain selectable. All-transparent paints contribute neither painted bounds nor hits; nodes remain available in the element dropdown. Text retains conservative ink-bound selection. Bounds remain conservative across partially transparent intervals and must not be presented as exact alpha extents. Source linking, helper provenance and stale-revision checks remain unchanged.

### 6. Version and acceptance coverage

Advance the graphics profile to graphics-core-v3 because prepared scene and host destination contracts change. Update every producer/consumer, manifest/schema, expected fixture and reference together. Keep browser renderer configuration distinct from native; version its identifier if interpolation configuration changes its observable raster contract. Generic NX IR versions do not change solely for new ordinary union cases.

Add one analytic sampling corpus and native/browser rendered fixtures. The oracle computes gradient parameters and premultiplied sampling independently of adapter output. Compare interior premultiplied RGBA channels against analytic expected results within 2/255 per channel on explicit sRGB surfaces, excluding a two-physical-pixel band around shape edges, clip edges and hard-stop boundaries. At hard stops, test nearby pixels on both sides, and test exact-offset behavior in the analytic corpus. Keep font raster comparisons geometry/region-based rather than demanding identical glyph-edge pixels. Exercise fill, stroke, text, background, duplicate/missing endpoint stops, alpha, root placement, transforms, clipping and group opacity. Validate invalid values, budgets, and failed replacement behavior separately.

Provide checked-in NX examples: an illustration using linear/radial paints; a chart with a transparent area fill; shared named paint values and a helper accepting gradient Paint. Document local-space reuse, interpolation, validation, selection and all deferred features. No empirical model-quality claim is part of deterministic acceptance.

## Risks / Trade-offs

- [Backend interpolation defaults differ] → run the probe first; explicit adapter settings or bounded compatible sampling must pass the fixed corpus before shipping.
- [Canvas text's internal scale changes gradient placement] → preserve element-space paint setup and cover multiple sizes/anchors with translated and scaled ancestors.
- [Many stops inflate preparation or shader work] → scene-wide stop budget, validate before allocations, and retain existing surface/layer accounting.
- [Transparent paint is confused with absent paint] → one potential-visibility rule for preparation, bounds and hit testing; validate before removing all-transparent paints.
- [Prepared payload consumers drift] → migrate SDK/schema/native/browser fixtures atomically under graphics-core-v3.
- [Fixed sRGB interpolation is less perceptually uniform] → document it and leave a future explicit-space extension rather than exposing unsupported host behavior now.

## Migration Plan

1. Establish the analytic corpus and adapter feasibility probe.
2. Add authoring contracts and neutral paint/validation, then migrate destinations, transport, Canvas and inspection together.
3. Update profiles, limits, schemas, generated library artifacts, examples and documentation; run the acceptance suites before release.
4. There is no persisted stable graphics compatibility promise. Roll back the feature as a coordinated source/generated-artifact/fixture revert if necessary; do not retain dual prepared payloads.
