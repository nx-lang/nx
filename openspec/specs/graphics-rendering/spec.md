# graphics-rendering Specification

## Purpose

Defines safe preparation and direct rendering of the initial NX graphics profile through a backend-neutral drawing contract with an initial Skia implementation.

## Requirements

### Requirement: Scene preparation validates known declaration identities before painting
The system SHALL accept evaluated NX graphics values with their linked declaration identities, validate the entire scene against the supported profile and host limits, and prepare semantic geometry before painting. Unknown/spoofed component identities, malformed paths, invalid dimensions/numbers and excess node/depth/path/surface/layer complexity SHALL fail preparation. Diagnostics SHALL identify the affected scene element and provenance when available. Prepared content SHALL NOT require a standardized NX display list.

#### Scenario: Invalid path fails before any paint
- **WHEN** a later child contains malformed SVG path data after an otherwise valid earlier child
- **THEN** preparation SHALL fail before any child is painted and SHALL identify the bad child

#### Scenario: Name spoofing is rejected
- **WHEN** a user declaration shares the display name Rect but is not the linked graphics Rect contract
- **THEN** it SHALL NOT be accepted as that primitive merely by matching its name

### Requirement: Graphics painting preserves geometry and compositing semantics
Rendering SHALL preserve child paint order, local affine transforms using SVG matrix order, Drawing view-box fit/alignment/viewport clipping, fill/stroke rules, and clipping in transformed local coordinates. Opacity SHALL apply to the isolated result of the entire node subtree. Zero-width strokes SHALL paint nothing; odd dash patterns SHALL repeat to an even pattern and all-zero patterns SHALL be solid. SVG path arcs and open/closed subpath semantics SHALL be supported. Numeric backend conversions SHALL reject unrepresentable values rather than silently overflow.

#### Scenario: Transform order is observable
- **WHEN** a local point (1,0) uses the transform list translate(10,0), scale(2,2)
- **THEN** its transformed coordinate SHALL be (12,0), before root viewport mapping

#### Scenario: Group opacity preserves overlap
- **WHEN** two overlapping opaque shapes are children of a group with opacity 0.5
- **THEN** the group SHALL be composited once at 0.5 and the overlap SHALL NOT accumulate child opacity

#### Scenario: Clip holes are independent of fill rules
- **WHEN** a node has an evenodd compound clip and a nonzero shape fill
- **THEN** clip holes SHALL remain transparent and the shape fill SHALL retain nonzero semantics

### Requirement: Direct Skia support remains an optional host adapter
The system SHALL provide direct Skia rendering to a host-supplied canvas and an offscreen raster surface. Drawing operations SHALL dispatch directly without constructing an intermediate standardized NX command list. Backend-neutral public contracts SHALL contain no Skia/platform objects, and using the compiler/generic runtime SHALL NOT require linking Skia. All exits SHALL restore the host canvas state; painting failure SHALL be reported and SHALL NOT be described as a successful frame.

#### Scenario: Borrowed canvas state is restored
- **WHEN** painting completes or a backend reports failure inside a nested clip/opacity scope
- **THEN** the host canvas save depth, transform and clip SHALL be restored to entry state
- **AND** a failure SHALL be returned to the host; pixel rollback on a borrowed canvas SHALL NOT be promised

#### Scenario: Direct rendering works without a recorder
- **WHEN** a host paints a prepared graphics-core scene with only the Skia destination installed
- **THEN** it SHALL render successfully without any recorder/display-list implementation

### Requirement: Measured text and painted text use the same shaped run
The text service SHALL resolve host-supplied font identities, shape supported single-line text, retain the glyph positions/resources used for measurement, and paint that layout. Missing font or glyph coverage SHALL be an error. The service SHALL return logical advance, ink bounds, ascent/descent and baseline, and anchoring SHALL use shaped advance. No implicit machine-dependent font fallback SHALL replace a requested face.

#### Scenario: Combining glyphs are measured consistently
- **WHEN** a supplied font supports a string with a combining sequence or ligature
- **THEN** measured bounds/advance and rendered glyph positions SHALL derive from the same shaped layout

#### Scenario: Missing glyph coverage fails clearly
- **WHEN** the chosen host font cannot render a required character
- **THEN** preparation SHALL identify the text/font failure and SHALL NOT silently substitute missing-glyph boxes

### Requirement: Gradient sampling is defined across renderers
Linear gradient position SHALL be the projection of the local point onto the endpoint vector divided by its squared length. Radial gradient position SHALL be distance from the local center divided by radius. Before the first stop and after the last stop, sampling SHALL use the corresponding endpoint color. Between distinct offsets, sampling SHALL interpolate premultiplied encoded-sRGB channels and alpha linearly, then unpremultiply for positive alpha; zero-alpha output SHALL use zero RGB. At an offset with multiple stops, the last stop SHALL win, with the first coincident stop defining the limit from below. Native and browser adapters SHALL honor this contract rather than silently accepting differing defaults.

#### Scenario: Transparent endpoint cannot contaminate RGB
- **WHEN** an opaque red stop at 0 transitions to a fully transparent blue stop at 1
- **THEN** the midpoint SHALL have alpha 0.5 and unpremultiplied encoded RGB (1,0,0)

#### Scenario: Missing endpoints clamp and duplicate offsets form hard stops
- **WHEN** stops are red at 0.25, red at 0.5, blue at 0.5 and blue at 0.75
- **THEN** positions below 0.25 SHALL be red and positions above 0.75 SHALL be blue
- **AND** the limit from below 0.5 SHALL be red and sampling at 0.5 SHALL be blue

### Requirement: Gradient paints preserve existing drawing semantics
Gradients SHALL be sampled in the painted element's local coordinate system and undergo its complete transform and viewBox mapping. Stroke gradients SHALL use local position rather than path length. Radial gradients SHALL become elliptical under nonuniform transforms. Drawing background gradients SHALL use viewBox coordinates while their geometry continues to cover the viewport. Text paint coordinates SHALL be independent of adapter-internal typography normalization. Gradient rendering SHALL preserve clips, isolated subtree opacity, child order, supplied-font shaping and borrowed-canvas restoration on success or failure.

#### Scenario: Gradient follows transformed geometry
- **WHEN** a radial-filled shape is translated and nonuniformly scaled within a clipped group
- **THEN** its gradient SHALL be translated and stretched with the shape and clipped by the same ancestor scopes

#### Scenario: Background retains drawing coordinates
- **WHEN** a Drawing with a nonzero viewBox origin uses contain placement and a gradient background
- **THEN** the background SHALL cover the viewport including letterboxing and sample its gradient through the same viewBox-to-viewport transform

#### Scenario: Text gradient survives internal font scaling
- **WHEN** the same local gradient fills supported text at different font sizes and start/middle/end anchors under a transformed group
- **THEN** sampling at a given element-local coordinate SHALL be independent of internal reference-font scaling and anchoring optimizations
- **AND** text geometry SHALL continue to use the measured shaped run

### Requirement: Prepared gradients are neutral and bounded
Prepared paints SHALL retain kind, validated geometry and ordered canonical stop colors without platform objects. Host transport SHALL identify this contract as graphics-core-v3 and unsupported profile consumers SHALL reject it explicitly. A configurable positive integer gradient_stops limit SHALL bound total supplied stops per scene, defaulting to 100000; each consuming paint occurrence SHALL count even for shared values and invisible content. Budget checking SHALL precede stop-resource allocation and painting. Existing coordinate, surface and peak layer limits SHALL remain enforced, including any temporary gradient raster resources. Stops SHALL be fully validated before all-transparent paints are classified as nonpainting.

#### Scenario: Aggregate stop budget cannot be bypassed with sharing
- **WHEN** two elements reuse a three-stop paint and the configured gradient_stops limit is five
- **THEN** preparation SHALL fail on the second occurrence before painting or allocating resources beyond the limit

#### Scenario: Invalid hidden content is still rejected
- **WHEN** an opacity-zero node contains a gradient with invalid stops or an all-transparent gradient contains an invalid Color channel
- **THEN** preparation SHALL return a contextual diagnostic rather than skip validation

#### Scenario: SDK transport preserves paint details
- **WHEN** a prepared scene passes from neutral preparation through WASM serialization to the browser SDK
- **THEN** paint kind, local geometry, stop ordering, offsets and float32 Color precision SHALL be preserved under graphics-core-v3
- **AND** a host accepting only graphics-core-v2 SHALL reject the changed payload explicitly
