## ADDED Requirements

### Requirement: Gradient inspection and selection retain authoring context
Native inspection and the playground inspector SHALL expose paint kind, local gradient coordinates and ordered stops with original offsets and canonical Color precision. A paint SHALL be potentially visible if any stop has positive alpha. Such gradients SHALL use geometry-based selection with existing ancestor clips and conservative painted/visible bounds; transparent intervals SHALL remain selectable. All-transparent gradient paints SHALL contribute neither painted bounds nor geometry hits, while their elements SHALL remain available for explicit inspector selection. Text SHALL retain conservative ink-bound selection. Existing source linking, helper invocation provenance and stale-revision rejection SHALL remain applicable.

#### Scenario: Transparent interval remains selectable
- **WHEN** an otherwise potentially visible gradient has a fully transparent interval inside a filled shape and the author clicks that interval within the applicable clips
- **THEN** the shape SHALL be selectable by its fill geometry
- **AND** inspection SHALL label its bounds as conservative where transparency can make them overestimates

#### Scenario: Entirely transparent gradient is inspectable without painting
- **WHEN** every stop in a shape's only paint has zero alpha
- **THEN** that paint SHALL contribute no painted bounds or hit target
- **AND** the element dropdown SHALL still permit inspecting its gradient data

#### Scenario: Inspecting reusable helper paint
- **WHEN** a Star uses a shared gradient value and its rendering is selected
- **THEN** inspection SHALL expose the gradient's authored stop values and local coordinates
- **AND** existing selection SHALL navigate to the authored Star invocation without changing Visual or Source mode

### Requirement: Gradient acceptance covers native and browser output reproducibly
The repository SHALL carry shared deterministic gradient fixtures and an independent analytic sampling oracle. Native Skia and browser Canvas output on explicit sRGB surfaces SHALL match expected interior premultiplied RGBA channels within 2/255 per channel, excluding a two-physical-pixel band around geometry, clip and hard-stop edges. Exact-offset behavior SHALL be checked analytically and hard transitions SHALL be checked on both sides. Fixtures SHALL cover linear/radial shape fills, strokes, text, backgrounds, alpha, duplicate and missing endpoint stops, viewBox placement, transforms, clipping, isolated opacity and all-transparent paints. Text raster checks SHALL use interior regions and measured geometry rather than identical glyph-edge pixels. Invalid paints and resource failures SHALL preserve existing native atomic-publication and browser stale-frame behavior. Deterministic acceptance SHALL require no model service or network assets.

#### Scenario: Both renderers pass transparent gradient fixtures
- **WHEN** native and browser fixtures render opaque and unequal-alpha transitions including colored fully transparent endpoints
- **THEN** both SHALL satisfy the same analytic premultiplied color expectations within the declared interior tolerance
- **AND** adapter-specific defaults SHALL NOT excuse interpolation mismatches

#### Scenario: Invalid replacement cannot appear successful
- **WHEN** a valid gradient preview is followed by an invalid stop sequence or a scene exceeding its gradient budget
- **THEN** native preview SHALL leave the prior successful pointer unchanged and return failure for the new revision
- **AND** browser preview SHALL show diagnostics without presenting the prior frame as the successful current revision

### Requirement: Gradient examples document reusable paint authoring
The graphics reference and checked-in examples SHALL demonstrate a linear/radial illustration, a chart with a transparent area fill, and reuse of named Paint values or functions including a shape helper. Documentation SHALL explain local coordinate reuse, stop constraints, endpoint clamping, hard stops, premultiplied encoded-sRGB interpolation, geometry-based selection and deferred gradient features. It SHALL distinguish gradient interpolation from the default color.mix and color.composite spaces.

#### Scenario: Author follows reusable paint example
- **WHEN** an author opens the gradient example and changes a named paint's stop color
- **THEN** its consuming shapes SHALL reflect the change through ordinary NX evaluation in native and browser previews
- **AND** documentation SHALL explain that reuse preserves explicit local gradient coordinates rather than automatically fitting each shape's bounds
