# graphics-preview Specification

## Purpose

Gives agents and developers a local render-and-inspect loop with reproducible images, geometry and text measurements, and actionable graphics diagnostics.

Graphics evaluation targets the Rust and TypeScript IR runtimes. The legacy HIR interpreter
is outside this graphics profile; its import/type-matching limitations are accepted.
Native preview compiles with nx-api/nx-codegen and evaluates linked images with nx-ir-runtime.
Runtime acceptance and negative tests use IR, with no HIR parity requirement.

## Requirements

### Requirement: A local preview host evaluates and renders NX source
The system SHALL provide an in-repository executable/example and reusable APIs accepting a source entry/root, viewport, device scale, supplied fonts, limits and output destination. Successful execution SHALL produce a PNG and a structured manifest naming the revision, profile, viewport, font identities and renderer configuration. No external model service, network asset fetch, sibling checkout or running fiddle SHALL be required. Evaluation (including math failures), preparation or rendering failure SHALL return diagnostics without publishing a new successful/partial preview. Evaluation SHALL supply required transitive library images through existing linking APIs; revision identity SHALL incorporate their content versions through the compiled program fingerprint.

#### Scenario: Preview runs offline
- **WHEN** a caller submits a valid NX fixture and local fonts with networking unavailable
- **THEN** the host SHALL evaluate the source, render actual pixels and produce PNG/manifest artifacts

#### Scenario: Failed replacement is not published
- **WHEN** a new revision fails evaluation, validation or rendering after an earlier preview succeeded
- **THEN** the response SHALL identify the failed revision and SHALL NOT present the earlier or partially painted image as its successful output

### Requirement: Inspection results identify elements and their coordinate spaces
The preview manifest SHALL include deterministic revision-scoped element IDs, parent/paint order, declaration identity, provenance when available, local geometry bounds, transformed painted bounds, clipped visible bounds and text metrics where applicable. Exact versus conservative bounds SHALL be distinguished, and empty/fully clipped content SHALL be explicit. Repeating the same evaluated structure and configuration SHALL preserve IDs; cross-edit persistent identity SHALL NOT be promised. Inspection requests naming a different revision SHALL fail as stale.

#### Scenario: Painted bounds differ from geometry bounds
- **WHEN** a stroked transformed shape extends beyond its geometry and intersects a clip
- **THEN** inspection SHALL distinguish geometry, painted and visible bounds with units/spaces and estimate flags

#### Scenario: Stale inspection cannot address a replacement node
- **WHEN** an inspection refers to an ID from a previous revision after children were reordered
- **THEN** the request SHALL fail its revision check rather than select the current occupant of the same position

### Requirement: Agents can request reproducible text measurements
A caller SHALL be able to measure supported text with the same font registry and text properties used by preview. Measurement SHALL return font identity, advance, ink bounds and baseline metrics sufficient for deliberate placement. Invalid or unsupported requests SHALL return diagnostics consistent with scene preparation.

#### Scenario: Agent places a label from its advance
- **WHEN** a caller measures a supported run and places it with an anchor using that response
- **THEN** preview SHALL use the same shaped metrics and positioning convention

### Requirement: Agent authoring evaluation is repeatable and separate from deterministic tests
The system SHALL provide versioned tasks for icons, diagrams, geometric illustrations and text-bearing infographics, including requested parameter/placement revisions. Evaluation records SHALL include model/version/budget when applicable, render configuration, artifacts, renderability, edit success, output size, round trips, latency and the visual-review rubric. CI SHALL validate deterministic fixtures without model calls; empirical quality claims SHALL require recorded model trials.

#### Scenario: Helper comparison uses the same task conditions
- **WHEN** an evaluator compares primitive-only and helper-assisted generation
- **THEN** task prompts, assets, fonts, model and budget SHALL be recorded consistently and requested revisions SHALL be evaluated separately from initial appearance

### Requirement: Playground renders IR-authored drawings in the browser
The playground SHALL provide a graphics adapter in Preview that compiles source, links required library images,
evaluates root using TypeScript IR and prepares it through the shared neutral Rust graphics core.
It SHALL render the resulting bounded scene through the built-in HTML Canvas 2D API, with explicitly
supplied fonts, without a server, CanvasKit, renderer WASM, native Skia or HIR evaluation dependency.
The existing compiler WASM module MAY perform compilation and shared neutral preparation. Existing component preview SHALL
continue to work. Browser text configuration SHALL be distinguished from native Skia.

#### Scenario: Author revises a playground drawing
- **WHEN** a user opens the graphics example and changes a Star from five to seven tips
- **THEN** Preview SHALL show the revised geometry and expose the current revision and node bounds
- **AND** a malformed path or invalid helper SHALL show diagnostics without presenting an earlier
  scene as the current result

#### Scenario: Browser rendering shares validated semantics
- **WHEN** a Drawing uses view-box placement, path arcs, evenodd clips and group opacity
- **THEN** preparation SHALL use the same nx-graphics geometry/limit contracts as the native host
- **AND** fonts SHALL be loaded from bundled bytes and missing glyphs or unsupported faces SHALL
  produce diagnostics instead of implicit system fallback

#### Scenario: Output and Preview share IR evaluation
- **WHEN** graphics source evaluates successfully on TypeScript IR
- **THEN** Output SHALL show its value and Preview SHALL render that same value without HIR evaluation

### Requirement: Preview selection shares a renderer-independent authored-source contract
Preview adapters SHALL associate current rendered IDs with authored source sites. Selecting a
rendered item SHALL select its authored expression in the existing Visual or Source mode. Selecting
an expression in Visual SHALL select every current rendering associated with that expression,
including repeated loop output. Graphics helpers SHALL retain primitive construction identity
separately from authored invocation provenance. Stale source snapshots SHALL NOT select a new output.

#### Scenario: Helper selection navigates without switching mode
- **WHEN** a user selects a Star, Arrow, RegularPolygon, Arc or RingSector rendering while Visual is active
- **THEN** its authored invocation SHALL be selected in Visual and the mode SHALL remain Visual
- **AND** selection while Source is active SHALL select the corresponding source range without changing mode

#### Scenario: One expression produces multiple rendered items
- **WHEN** a Visual expression is evaluated repeatedly in a loop or expands into multiple records
- **THEN** all associated current rendered items SHALL be selected

#### Scenario: Structured Question and Choice use the same contract
- **WHEN** a structured root has no runnable component and contains Question and Choice records
- **THEN** Output SHALL show the structured value using TypeScript IR and source-linked record annotations
- **AND** selecting a Question in Visual SHALL reveal/highlight its matching output records
- **AND** selecting a Choice in Output SHALL select its authored Choice in Visual without switching mode

### Requirement: Generic playground preview routing
The playground SHALL evaluate browser output using TypeScript IR and present the same current value
and construction provenance through universal structured Output and registered preview adapters.
It SHALL offer Preview only when a registered adapter supports the result, SHALL render graphics
in Preview rather than a Drawing tab, and SHALL preserve component actions and timeline behavior.

#### Scenario: Supported Drawing
- **WHEN** evaluation produces the genuine graphics Drawing declaration, including through an import alias
- **THEN** Preview is selected initially and renders that value, with Output available for inspection
- **AND** a same-name user declaration does not activate the graphics previewer

#### Scenario: Unsupported structured value
- **WHEN** evaluation produces a value without a registered previewer
- **THEN** only Output is shown, with bidirectional source selection and no duplicate structured Preview

#### Scenario: Interaction and user choice
- **WHEN** the visitor selects Output, edits the document, or interacts with a running component
- **THEN** their Output preference survives edits and Output reflects the current rendered component value
- **AND** opening another document resets to Preview when supported

#### Scenario: Temporary errors
- **WHEN** a supported preview encounters compilation, evaluation, or rendering errors
- **THEN** its tab remains available during the failure, diagnostics are visible, stale values are marked,
and a stale graphics scene cannot be presented as current

### Requirement: Libraries declare preview associations in NX
Types SHALL be able to declare an optional typed preview descriptor in their visualizer metadata.
The renderer ID SHALL be resolved with existing visualizer precedence and nominal declaration
identity. Hosts SHALL activate only bundled supported renderer IDs and SHALL NOT execute or load
code from the descriptor. Unknown renderer IDs SHALL leave the value available in Output.

#### Scenario: Graphics library owns its association
- **WHEN** a Drawing is evaluated, including one returned by an imported helper
- **THEN** the playground SHALL discover the graphics renderer ID from loaded library metadata
- **AND** no playground mapping from the Drawing module/type name to the renderer SHALL be required

#### Scenario: Unsupported host renderer
- **WHEN** an application type requests a renderer the host does not support
- **THEN** it SHALL remain Output-only without loading external code

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
