## MODIFIED Requirements

### Requirement: Scene preparation validates known declaration identities before painting
The system SHALL accept evaluated NX graphics values with their linked declaration identities, validate the entire scene against the supported profile and host limits, and prepare semantic geometry before painting. Unknown/spoofed component identities, malformed paths, invalid dimensions/numbers and excess node/depth/path/surface/layer complexity SHALL fail preparation. Diagnostics SHALL identify the affected scene element and provenance when available. Prepared content SHALL NOT require a standardized NX display list.

#### Scenario: Invalid path fails before any paint
- **WHEN** a later child contains malformed SVG path data after an otherwise valid earlier child
- **THEN** preparation SHALL fail before any child is painted and SHALL identify the bad child

#### Scenario: Name spoofing is rejected
- **WHEN** a user declaration shares the display name Rectangle but is not the linked graphics Rectangle contract
- **THEN** it SHALL NOT be accepted as that primitive merely by matching its name

### Requirement: Prepared gradients are neutral and bounded
Prepared paints SHALL retain kind, validated geometry and ordered canonical stop colors without platform objects. Host transport SHALL identify this contract as graphics-core-v4 and unsupported profile consumers SHALL reject it explicitly. A configurable positive integer gradient_stops limit SHALL bound total supplied stops per scene, defaulting to 100000; each consuming paint occurrence SHALL count even for shared values and invisible content. Budget checking SHALL precede stop-resource allocation and painting. Existing coordinate, surface and peak layer limits SHALL remain enforced, including any temporary gradient raster resources. Stops SHALL be fully validated before all-transparent paints are classified as nonpainting.

#### Scenario: Aggregate stop budget cannot be bypassed with sharing
- **WHEN** two elements reuse a three-stop paint and the configured gradient_stops limit is five
- **THEN** preparation SHALL fail on the second occurrence before painting or allocating resources beyond the limit

#### Scenario: Invalid hidden content is still rejected
- **WHEN** an opacity-zero node contains a gradient with invalid stops or an all-transparent gradient contains an invalid Color channel
- **THEN** preparation SHALL return a contextual diagnostic rather than skip validation

#### Scenario: SDK transport preserves paint details
- **WHEN** a prepared scene passes from neutral preparation through WASM serialization to the browser SDK
- **THEN** paint kind, local geometry, stop ordering, offsets and float32 Color precision SHALL be preserved under graphics-core-v4
- **AND** a host accepting only graphics-core-v3 SHALL reject the changed payload explicitly
