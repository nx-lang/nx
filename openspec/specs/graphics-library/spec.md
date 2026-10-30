# graphics-library Specification

## Purpose

Allows NX programs and AI agents to author standalone vector drawings using portable graphics contracts and reusable parameterized shapes, without a UI layout engine.

Graphics evaluation targets the Rust and TypeScript IR runtimes. The legacy HIR interpreter
is outside this graphics profile; its import/type-matching limitations are accepted.
Native preview compiles with nx-api/nx-codegen and evaluates linked images with nx-ir-runtime.
Runtime acceptance and negative tests use IR, with no HIR parity requirement.

## Requirements

### Requirement: Graphics declarations are available as an unstable standard library
The system SHALL carry `@nx/graphics` as ordinary NX standard-library source with Drawing, Group, Rect, Circle, Ellipse, Line, Polyline, Polygon, Path, and Text contracts. It SHALL declare an unstable `graphics-core-v2` profile and require no UI library/renderer. Imports, analysis, IR emission/linking and evaluation SHALL follow existing standard-library and external-component contracts. Alias spelling SHALL NOT change component identity. Generic value and IR encodings SHALL remain unchanged.

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
The graphics-core-v2 profile SHALL support solid/none paint, centered strokes including caps/joins/dashes, affine transforms, both fill and clip winding rules, isolated opacity and view-box placement. Text SHALL support shaped single-line host-font content with size/weight/style and start/middle/end anchors at an alphabetic baseline. Unsupported paints, effects, images, multiline, text strokes, nonzero letter spacing and baseline alternatives SHALL produce diagnostics before painting.

#### Scenario: Unsupported content is not silently ignored
- **WHEN** a drawing requests a gradient, shadow or unsupported text feature
- **THEN** preparation SHALL report the feature and element and SHALL NOT publish a successful rendering of a reduced approximation

### Requirement: Parametric shapes expand through ordinary NX evaluation
The library SHALL provide RegularPolygon, Star, Arrow, Arc and RingSector helpers that evaluate to core geometry without backend-specific helper implementations. They SHALL import the delivered `@nx/math` library for numerical primitives rather than private trigonometric/square-root approximations or new runtime function registrations. Public graphics degree angles SHALL be converted explicitly to radians for math calls; any atan2 use SHALL retain y,x argument order and account for its radian result. They SHALL use local logical coordinates, clockwise degrees from +x and the parameter/range contracts in the authoring reference. Geometry deviation from those mathematical constructions SHALL be bounded by 1e-5 times max(1, characteristic length). Helpers SHALL preserve explicit appearance props and document defaults.

#### Scenario: Star revision changes intent directly
- **WHEN** an agent changes a Star's point count from five to seven while retaining center/radii/rotation
- **THEN** ordinary evaluation SHALL produce fourteen alternating outer/inner vertices with unchanged center/radii/rotation and appearance
- **AND** no renderer-specific Star node SHALL be required

#### Scenario: Full ring and invalid arrow
- **WHEN** a RingSector has a full 360-degree sweep and a positive inner radius
- **THEN** its path SHALL render a closed ring with a hole using explicit winding semantics
- **WHEN** an Arrow has coincident endpoints or a head longer than its endpoint distance
- **THEN** the system SHALL return a diagnostic instead of non-finite geometry

#### Scenario: Helpers evaluate across existing runtimes
- **WHEN** the same helper fixture is evaluated by the Rust and TypeScript IR runtimes
- **THEN** both SHALL produce equivalent primitive geometry within the declared tolerance without a graphics host callback

#### Scenario: Mathematical validity does not replace helper validation
- **WHEN** an Arrow's endpoints coincide even though atan2 accepts both-zero coordinates
- **THEN** the graphics contract SHALL reject the Arrow before normalizing its zero-length direction
- **AND** radius/count/sweep/finite/backend-range checks SHALL remain independent of the math library's numerical domain checks

#### Scenario: Numerical errors preserve their runtime diagnostics
- **WHEN** a graphics source calculation fails with a math non-finite-input, domain or overflow error
- **THEN** evaluation SHALL retain that diagnostic category, operation and available authored call origin
- **AND** no successful geometry with fabricated replacement coordinates SHALL be produced

### Requirement: Graphics declarations carry presentation icons
The graphics library SHALL declare recognizable Lucide icons for Drawing, Group, basic shapes,
paths, text and the public element-style shape helpers. Element functions SHALL support a
presentation-only FunctionVisualizer icon without changing function evaluation or result identity.

#### Scenario: Authored graphics are recognizable in Visual
- **WHEN** a graphics example is opened in Visual
- **THEN** library-supplied frame/group, shape, path, text and helper icons SHALL be displayed
- **AND** helper results SHALL retain their primitive nominal identities for rendering

### Requirement: Graphics authoring names express their domain roles
The abstract graphics bases SHALL be GraphicsElement and Shape. Shape SHALL extend GraphicsElement;
Group and Text SHALL extend GraphicsElement, and basic geometric primitives SHALL extend Shape.
Drawing SHALL remain a standalone viewport root. AxisAlignment SHALL name the start/center/end
choice used for each ContentAlignment axis. RegularPolygon and Star SHALL expose pointCount,
counting polygon vertices and star outer tips respectively; generated geometry SHALL be unchanged.

#### Scenario: Renamed contracts compose normally
- **WHEN** a program imports graphics under an alias, extends its abstract bases, and constructs a
  Drawing using AxisAlignment and helpers with pointCount
- **THEN** inherited appearance/defaults and helper geometry SHALL retain their prior behavior
- **AND** the library SHALL NOT retain the old public names or count helper properties as aliases

### Requirement: Graphics consumes the shared color contract
@nx/graphics SHALL import and re-export the canonical @nx/color Color rather than declaring a separate string alias. Solid paint colors and drawing backgrounds SHALL accept shared float32 Color records, including values authored as hex literals. Standard-library linking SHALL include color transitively when required, without requiring an application-level import. Scene preparation SHALL validate finite normalized channels, including raw SDK records, before painting. Direct strings in Color fields SHALL fail type checking or scene validation rather than being silently reparsed. Native and browser renderers SHALL preserve numerical channel precision until backend conversion.

#### Scenario: One shared color across application and drawing
- **WHEN** a program supplies the same @nx/color Color to an application brand record and a graphics solid fill
- **THEN** both SHALL accept the canonical value without a duplicate type or string conversion

#### Scenario: Graphics-only import links constructors
- **WHEN** a program imports graphics and its evaluated program needs the transitive color image
- **THEN** linking SHALL include the required image or report its absence before painting

#### Scenario: Invalid SDK color is rejected before painting
- **WHEN** a later scene element contains a Color channel outside [0,1] or non-finite channels
- **THEN** preparation SHALL fail before painting any element and identify the element/channel

### Requirement: Changed graphics color semantics have a new profile
Graphics SHALL identify the changed authored/evaluated color contract as graphics-core-v2 in documentation, preparation metadata, previews, and fixtures. Existing geometry, solid/none paint behavior, opacity, and unsupported-effect diagnostics SHALL remain applicable. Supported native and browser paths SHALL consume the shared color record consistently.

#### Scenario: Matching native and browser color semantics
- **WHEN** a scene with a translucent numerical Color is prepared for native Skia and browser Canvas
- **THEN** both SHALL report graphics-core-v2 and preserve the same sRGB-plus-alpha color contract
