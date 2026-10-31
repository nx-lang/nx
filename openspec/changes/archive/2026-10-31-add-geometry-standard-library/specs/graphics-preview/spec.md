## ADDED Requirements

### Requirement: Geometry migration is supported across preview hosts
Native preview and browser playground graphics SHALL accept drawings using Rectangle and
canonical geometry Point/Rect values, including required transitive geometry images. Successful
manifests and prepared scenes SHALL identify graphics-core-v4. Existing local preview,
inspection, selection, text, gradient, and source-provenance behavior SHALL remain applicable.
Browser consumers SHALL reject unsupported profile revisions explicitly.

#### Scenario: Native and browser preview share the migrated contract
- **WHEN** the same migrated drawing fixture is evaluated for native and browser preview
- **THEN** both SHALL render its Rectangle content with equivalent geometry and existing paint semantics
- **AND** both SHALL report graphics-core-v4

#### Scenario: Inspection identifies the drawable declaration
- **WHEN** a rendered Rectangle is inspected or selected in the playground
- **THEN** inspection SHALL identify the canonical graphics Rectangle component and retain authored source provenance
- **AND** its shared Rect viewport value SHALL NOT appear as a drawable child

#### Scenario: Browser profile mismatch is explicit
- **WHEN** the browser graphics adapter receives a scene marked with an unsupported profile revision
- **THEN** it SHALL report the mismatch rather than render it as a graphics-core-v4 scene

### Requirement: Geometry and graphics documentation use the shared vocabulary
The standard-library reference SHALL document geometry's public types, float64 representation,
coordinate-space conventions, enforced value constraints, and support for positive-infinite Size dimensions. Graphics reference
and active playground examples SHALL use Rectangle for drawable elements and Rect for viewports,
with a migration note for former Rect and ViewBox uses. Examples SHALL follow the NX source style
guide and SHALL demonstrate geometry without requiring a drawing.

#### Scenario: Non-graphics geometry example
- **WHEN** a reader builds the documented standalone geometry example
- **THEN** it SHALL construct Point, Size, and Rect with only a geometry import and ordinary evaluation

#### Scenario: Existing gradient examples migrate without changing gradient behavior
- **WHEN** a reader opens the migrated gradients playground example
- **THEN** it SHALL compile and preview using Rectangle and shared Rect values
- **AND** gradient coordinates SHALL retain their existing element-local user-unit meaning
