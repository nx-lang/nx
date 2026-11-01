# geometry-library Specification

## Purpose

Provides shared two-dimensional geometry values for NX applications, independently of graphics,
layout engines, rendering backends, and host-specific coordinate systems.

## Requirements

### Requirement: Geometry is an ordinary unstable standard library
The system SHALL provide `@nx/geometry` as an unstable compiler-owned standard library with root
module identity `@nx/geometry/geometry.nx`. It SHALL be available through existing analysis,
build, language-service, SDK, and type-generation entry points without host registration or
filesystem lookup. Direct geometry imports SHALL NOT require graphics, math, fonts, a renderer,
or a runtime geometry capability. Export visibility, import aliases, content-derived versions,
and image emission SHALL follow the standard-library contracts.

#### Scenario: A non-graphics application imports geometry
- **WHEN** a standalone program imports only `@nx/geometry` and constructs a Point, Size, and Rect
- **THEN** analysis SHALL succeed without a graphics library import or host-provided library
- **AND** its linked libraries SHALL NOT include graphics or math solely because of geometry

#### Scenario: Geometry declarations remain opt-in
- **WHEN** a module refers to Point without a geometry import, re-export, or local declaration
- **THEN** analysis SHALL report the unresolved name under ordinary import rules

### Requirement: Canonical geometry values use float64 records
The library SHALL export Point with required float64 x/y fields, Size with required float64
width/height fields, and Rect with required float64 x/y/width/height fields. They SHALL be
ordinary NX records, with canonical declaration identities owned by the geometry root module.
They SHALL have no implicit property defaults, graphics appearance, element inheritance, or
renderer-specific payload.

#### Scenario: Fractional and large coordinates retain public precision
- **WHEN** a program evaluates a Rect with x=10000000.25, y=-2.5, width=0.125, and height=3.75
- **THEN** the value SHALL preserve those float64 fields without intermediate float32 conversion

#### Scenario: Missing dimensions are diagnosed
- **WHEN** a program constructs a Size or Rect without a required width or height
- **THEN** ordinary invocation checking SHALL diagnose the missing property

### Requirement: Geometry domains are enforced through scalar value constraints
Point x/y and Rect x/y SHALL use finite float64 constraints. Rect width/height SHALL use finite
nonnegative float64 constraints. Size width/height SHALL use nonnegative float64 constraints
that accept positive infinity. All fields SHALL reject NaN through the existing constraint
contract. Zero and negative zero SHALL be accepted, with no new zero canonicalization.
The types SHALL prescribe no unit, origin, or axis direction. Constraints SHALL be checked at
the compile-time and runtime sites specified by value-constraints, with existing diagnostics.
No application coordinate limit or cross-field finite-edge predicate SHALL be imposed.

#### Scenario: Zero-area values remain usable outside graphics
- **WHEN** a non-graphics program constructs a Size with width=0 and a Rect with height=0
- **THEN** construction and supported IR evaluation SHALL succeed
- **AND** neither value SHALL be interpreted as an omitted optional value

#### Scenario: Negative dimensions are constraint violations
- **WHEN** a program binds a negative constant dimension to Size or Rect
- **THEN** analysis SHALL report constraint-violation at the binding
- **WHEN** a dynamic negative dimension reaches the same site during supported IR evaluation
- **THEN** evaluation SHALL fail with nx-ir-constraint

#### Scenario: Unbounded Size is distinct from concrete Rect
- **WHEN** a positive-infinity value is bound to a Size dimension
- **THEN** the value SHALL satisfy the nonnegative constraint
- **WHEN** it is bound to a Rect dimension or Point/Rect coordinate
- **THEN** the finite constraint SHALL reject it

#### Scenario: NaN fails every geometry field
- **WHEN** a NaN value reaches any Point, Size, or Rect scalar constrained binding site
- **THEN** the existing constraint-violation or nx-ir-constraint diagnostic SHALL be produced
- **AND** no successful geometry value SHALL be constructed through that binding

#### Scenario: Coordinate meaning belongs to the consumer
- **WHEN** separate APIs document document-space and window-space Point values
- **THEN** both SHALL use the same Point declaration without geometry imposing pixel units or a y-axis direction

### Requirement: Geometry values evaluate through existing IR contracts
Rust and TypeScript IR runtimes SHALL construct, pass, return, and nest geometry records through
ordinary evaluation without a geometry host callback or new IR encoding. Emitted references
SHALL retain canonical module ownership, including through import aliases and library re-exports.
Required images SHALL be emitted and linked using the standard-library image contract.

#### Scenario: Geometry values round-trip across a function boundary
- **WHEN** the same fixture returns a record containing Point, Size, and a sequence of Rect values
- **THEN** Rust and TypeScript IR evaluation SHALL preserve equivalent field values and canonical types

#### Scenario: Import aliases do not change ownership
- **WHEN** two programs import geometry with different local aliases and emit its library image
- **THEN** the images SHALL have the same canonical identities and content version for the same sources
