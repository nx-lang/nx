## ADDED Requirements

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
