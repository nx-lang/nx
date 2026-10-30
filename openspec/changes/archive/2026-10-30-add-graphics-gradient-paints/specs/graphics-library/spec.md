## MODIFIED Requirements

### Requirement: Graphics declarations are available as an unstable standard library
The system SHALL carry `@nx/graphics` as ordinary NX standard-library source with Drawing, Group, Rect, Circle, Ellipse, Line, Polyline, Polygon, Path, and Text contracts. It SHALL declare an unstable `graphics-core-v3` profile and require no UI library/renderer. Imports, analysis, IR emission/linking and evaluation SHALL follow existing standard-library and external-component contracts. Alias spelling SHALL NOT change component identity. Generic value and IR encodings SHALL remain unchanged.

#### Scenario: Standalone drawing evaluates under an alias
- **WHEN** a program imports `@nx/graphics` under a local alias and returns a Drawing containing a Rect
- **THEN** it SHALL type-check, link and evaluate with declared defaults and the canonical graphics declaration identities
- **AND** no Level 2 UI library or layout engine SHALL be required

#### Scenario: Imported props and defaults work
- **WHEN** an imported concrete shape uses inherited opacity/transform props and omits defaulted coordinates
- **THEN** invocation checking and evaluated output SHALL preserve those props/defaults without a source-local redeclaration

#### Scenario: Math is a transitive standard-library dependency
- **WHEN** a consumer imports only `@nx/graphics` and evaluates a helper whose source imports `@nx/math`
- **THEN** compilation and all-module emission SHALL resolve and include the required graphics and `@nx/math/math.nx` images with their existing identities/content versions
- **AND** evaluation SHALL use ordinary calls into the capability-gated math dependency without copying numerical implementation bodies into graphics

#### Scenario: Missing math runtime support fails explicitly
- **WHEN** the host omits a required math image or the evaluator does not implement its `standard-math-v1` feature
- **THEN** linking or preparation SHALL fail before painting, rather than silently substitute geometry or a host-specific approximation

### Requirement: Initial profile is explicit about supported content
The graphics-core-v3 profile SHALL support solid/none, linear-gradient and centered radial-gradient paint, centered strokes including caps/joins/dashes, affine transforms, both fill and clip winding rules, isolated opacity and view-box placement. Text SHALL support shaped single-line host-font content with size/weight/style and start/middle/end anchors at an alphabetic baseline. Unsupported paints, effects, images, multiline, text strokes, nonzero letter spacing and baseline alternatives SHALL produce diagnostics before painting.

#### Scenario: Unsupported content is not silently ignored
- **WHEN** a drawing requests a shadow, unsupported paint variant or unsupported text feature
- **THEN** preparation SHALL report the feature and element and SHALL NOT publish a successful rendering of a reduced approximation

### Requirement: Changed graphics color semantics have a new profile
Graphics SHALL identify the shared-color and gradient paint contract as graphics-core-v3 in documentation, preparation metadata, previews, and fixtures. Existing geometry, solid/none paint behavior, opacity, and unsupported-effect diagnostics SHALL remain applicable. Supported native and browser paths SHALL consume the shared color record consistently.

#### Scenario: Matching native and browser color semantics
- **WHEN** a scene with a translucent numerical Color is prepared for native Skia and browser Canvas
- **THEN** both SHALL report graphics-core-v3 and preserve the same sRGB-plus-alpha color contract

## ADDED Requirements

### Requirement: Gradient paints are ordinary reusable NX values
Paint SHALL expose color for a uniform canonical Color, linearGradient with explicit float64 x1/y1/x2/y2 and stops, and radialGradient with explicit float64 cx/cy/radius and stops. Each GradientStop SHALL have a float64 offset and canonical Color. Coordinates SHALL be element-local user units. Gradients SHALL be accepted in geometric fills, strokes, text fills, Drawing backgrounds and helper Paint parameters through ordinary typed values and calls. They SHALL require neither new graphics IR operations nor a paint registry. The earlier solid, linear and radial case names SHALL NOT be retained as aliases. The initial contract SHALL clamp outside endpoint stops and SHALL expose no spread, units or interpolation-space option.

#### Scenario: Reuse across fill and stroke
- **WHEN** an imported function returns a named gradient Paint used by a Rect fill, a Line stroke and a Star helper
- **THEN** Rust and TypeScript IR evaluation SHALL preserve the same gradient geometry, ordered stops and canonical Color values in each use
- **AND** no renderer callback SHALL be required during evaluation

#### Scenario: Text and background accept gradients
- **WHEN** a Drawing background and a supported single-line Text fill use radial and linear Paint values
- **THEN** both SHALL type-check and evaluate with their gradient values intact
- **AND** text strokes SHALL remain unsupported

### Requirement: Gradient inputs have explicit validity rules
Gradients SHALL contain at least two stops with finite offsets in [0,1] in nondecreasing order and valid canonical Colors. Equal offsets SHALL preserve sequence order. Linear endpoints SHALL be distinct and radial radius SHALL be positive. All geometry SHALL be finite and satisfy configured coordinate/backend ranges. At backend precision, gradient vector length and radial radius SHALL be strictly greater than 1/32768 local units, including a float32 rounding margin; Text SHALL apply a minimum multiplier of max(1, fontSize/1000) to preserve the range after reference-font normalization. Invalid paints SHALL fail preparation even on invisible content; preparation SHALL NOT sort, clamp invalid offsets, replace invalid colors or repair degenerate geometry.

#### Scenario: Malformed raw SDK paint
- **WHEN** a raw SDK scene supplies an out-of-order offset, invalid Color, non-finite coordinate, coincident linear endpoints or nonpositive radial radius
- **THEN** preparation SHALL reject the scene before painting and identify the affected element and paint property, with stop index where applicable

#### Scenario: Valid repeated offsets
- **WHEN** two neighboring stops have the same offset
- **THEN** preparation SHALL retain both stops in authored order as a hard transition
