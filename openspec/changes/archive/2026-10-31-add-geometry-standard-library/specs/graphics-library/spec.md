## MODIFIED Requirements

### Requirement: Graphics declarations are available as an unstable standard library
The system SHALL carry `@nx/graphics` as ordinary NX standard-library source with Drawing, Group, Rectangle, Circle, Ellipse, Line, Polyline, Polygon, Path, and Text contracts. It SHALL declare an unstable `graphics-core-v4` profile and require no UI library/renderer. Imports, analysis, IR emission/linking and evaluation SHALL follow existing standard-library and external-component contracts. Alias spelling SHALL NOT change component identity. Generic value and IR encodings SHALL remain unchanged.

#### Scenario: Standalone drawing evaluates under an alias
- **WHEN** a program imports `@nx/graphics` under a local alias and returns a Drawing containing a Rectangle
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
The graphics-core-v4 profile SHALL support solid/none, linear-gradient and centered radial-gradient paint, centered strokes including caps/joins/dashes, affine transforms, both fill and clip winding rules, isolated opacity and view-box placement. Text SHALL support shaped single-line host-font content with size/weight/style and start/middle/end anchors at an alphabetic baseline. Unsupported paints, effects, images, multiline, text strokes, nonzero letter spacing and baseline alternatives SHALL produce diagnostics before painting.

#### Scenario: Unsupported content is not silently ignored
- **WHEN** a drawing requests a shadow, unsupported paint variant or unsupported text feature
- **THEN** preparation SHALL report the feature and element and SHALL NOT publish a successful rendering of a reduced approximation

### Requirement: Changed graphics color semantics have a new profile
Graphics SHALL identify the shared-color, gradient paint, and shared-geometry contract as graphics-core-v4 in documentation, preparation metadata, previews, and fixtures. Existing geometry, solid/none paint behavior, opacity, and unsupported-effect diagnostics SHALL remain applicable. Supported native and browser paths SHALL consume the shared color record consistently.

#### Scenario: Matching native and browser color semantics
- **WHEN** a scene with a translucent numerical Color is prepared for native Skia and browser Canvas
- **THEN** both SHALL report graphics-core-v4 and preserve the same sRGB-plus-alpha color contract

### Requirement: Gradient paints are ordinary reusable NX values
Paint SHALL expose color for a uniform canonical Color, linearGradient with explicit float64 x1/y1/x2/y2 and stops, and radialGradient with explicit float64 cx/cy/radius and stops. Each GradientStop SHALL have a float64 offset and canonical Color. Coordinates SHALL be element-local user units. Gradients SHALL be accepted in geometric fills, strokes, text fills, Drawing backgrounds and helper Paint parameters through ordinary typed values and calls. They SHALL require neither new graphics IR operations nor a paint registry. The earlier solid, linear and radial case names SHALL NOT be retained as aliases. The initial contract SHALL clamp outside endpoint stops and SHALL expose no spread, units or interpolation-space option.

#### Scenario: Reuse across fill and stroke
- **WHEN** an imported function returns a named gradient Paint used by a Rectangle fill, a Line stroke and a Star helper
- **THEN** Rust and TypeScript IR evaluation SHALL preserve the same gradient geometry, ordered stops and canonical Color values in each use
- **AND** no renderer callback SHALL be required during evaluation

#### Scenario: Text and background accept gradients
- **WHEN** a Drawing background and a supported single-line Text fill use radial and linear Paint values
- **THEN** both SHALL type-check and evaluate with their gradient values intact
- **AND** text strokes SHALL remain unsupported

## ADDED Requirements

### Requirement: Graphics consumes canonical shared geometry
Graphics SHALL import and re-export Point, Size, and Rect as aliases of their canonical
`@nx/geometry` declarations rather than duplicate records. Polygon and Polyline vertices and
helper-produced points SHALL use shared Point. Drawing.viewBox SHALL accept shared Rect and
SHALL retain strictly positive width/height validation. The separate ViewBox declaration SHALL
be removed. Graphics-only imports SHALL resolve required geometry dependencies transitively.

#### Scenario: A viewport and points are shared across libraries
- **WHEN** an application passes a directly imported geometry Rect as Drawing.viewBox and directly imported geometry Points as Polygon.points
- **THEN** analysis and supported IR evaluation SHALL accept the canonical values without field copying or conversion

#### Scenario: Graphics re-exports have canonical ownership
- **WHEN** a program constructs Point, Size, or Rect through an alias of the graphics import
- **THEN** the resulting declaration identity SHALL belong to `@nx/geometry/geometry.nx`
- **AND** constructing the corresponding directly imported geometry type SHALL produce the same nominal type

#### Scenario: A viewport retains stricter dimension rules
- **WHEN** a well-typed Rect with zero width is supplied as Drawing.viewBox
- **THEN** scene preparation SHALL reject that viewport before painting
- **AND** a standalone zero-width Rect SHALL remain a valid geometry value
- **AND** raw SDK viewports with negative or non-finite dimensions SHALL still be rejected before painting, even if they bypass NX constrained construction

#### Scenario: Geometry links transitively
- **WHEN** a source imports only graphics and returns a Drawing with shared geometry values
- **THEN** all-module emission and linking SHALL provide the required geometry image
- **AND** omitting a required image SHALL fail through ordinary linking rather than fabricate values

#### Scenario: ViewBox is no longer a second rectangle type
- **WHEN** source refers to the removed graphics ViewBox export
- **THEN** analysis SHALL report the missing name and documentation SHALL direct authors to Rect

### Requirement: Drawable rectangles have an unambiguous name
The graphics rectangle external component SHALL be named Rectangle, with the existing rectangle
properties, defaults, rounded-corner behavior, Shape inheritance, and presentation icon. The
name Rect SHALL identify only the shared geometry value and SHALL NOT be a component alias.

#### Scenario: Rectangle preserves appearance
- **WHEN** an existing rounded, transformed, stroked rectangle fixture migrates its drawable name to Rectangle
- **THEN** prepared geometry, corner-radius handling, paint, opacity, and clipping SHALL remain equivalent

#### Scenario: A geometry rectangle is not a drawable element
- **WHEN** a program places a shared Rect value in Drawing.children
- **THEN** invocation checking SHALL reject it as a non-Element value
- **AND** raw-value scene preparation SHALL NOT reinterpret that record as a Rectangle component

### Requirement: Graphics numeric declarations enforce scalar domains
The public numeric declarations in graphics.nx SHALL encode scalar domains with value constraints
on fields, union cases, component props, and helper parameters. Coordinates, transform values,
angles and signed offsets SHALL be finite. Shape/stroke/corner/helper lengths and dash items
SHALL be finite and nonnegative. Radial paint radius and fontSize SHALL be finite and strictly
positive. Opacity and gradient stop offsets SHALL be in [0,1]; miterLimit SHALL be finite and at
least 1; fontWeight SHALL retain float64 representation constrained to [1,1000]. RegularPolygon
and Star pointCount SHALL be integers in [3,4096], and Arc/RingSector sweepAngle SHALL be in
[-360,360]. Defaults, optionality, zero/negative scale support, and existing shape semantics SHALL
be retained. Binding failures SHALL use existing static/runtime constraint diagnostics.

Scalar constraints SHALL NOT replace relational, whole-number fontWeight, computed-overflow,
path, sequence, backend-representability, or resource-budget validation. Scene preparation SHALL
retain defensive checking for raw host values that bypass constrained NX binding sites.

#### Scenario: Invalid scalar arguments fail at their binding
- **WHEN** a constant opacity=1.5, negative corner radius, zero fontSize, or helper pointCount=2 is supplied
- **THEN** analysis SHALL report constraint-violation at the offending binding
- **WHEN** the same invalid values are supplied dynamically in supported IR evaluation
- **THEN** evaluation SHALL report nx-ir-constraint before successful rendering

#### Scenario: Signed transform values remain legal
- **WHEN** a transform uses finite negative translation, negative scale, zero scale, or rotation beyond 360 degrees
- **THEN** scalar constraint checks SHALL accept those values
- **AND** existing transform and rendering rules SHALL continue to apply

#### Scenario: Optional constrained values and sequence items
- **WHEN** a source omits Rectangle.ry or Transform.scale.y, or supplies no dashArray items
- **THEN** existing omission/default semantics SHALL be preserved
- **WHEN** a dashArray item is negative, infinite, or NaN
- **THEN** its constrained binding SHALL fail

#### Scenario: Cross-field validation remains necessary
- **WHEN** a gradient has valid individual offsets in decreasing order or coincident linear endpoints
- **THEN** graphics validation SHALL still reject it before painting
- **WHEN** fontWeight is 400.5 or a raw SDK opacity is outside [0,1]
- **THEN** scene preparation SHALL retain its existing rejection behavior

#### Scenario: Public helpers retain relational guards
- **WHEN** a Star has individually valid radii but innerRadius exceeds outerRadius, or an Arrow has coincident endpoints
- **THEN** existing helper validation SHALL reject the invalid relationship
- **AND** scalar constraints SHALL NOT be treated as proving the relationship valid
