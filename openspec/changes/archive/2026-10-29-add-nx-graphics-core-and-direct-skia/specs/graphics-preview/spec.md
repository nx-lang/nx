## Purpose

Gives agents and developers a local render-and-inspect loop with reproducible images, geometry and text measurements, and actionable graphics diagnostics.

## ADDED Requirements

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
The change SHALL provide versioned tasks for icons, diagrams, geometric illustrations and text-bearing infographics, including requested parameter/placement revisions. Evaluation records SHALL include model/version/budget when applicable, render configuration, artifacts, renderability, edit success, output size, round trips, latency and the visual-review rubric. CI SHALL validate deterministic fixtures without model calls; empirical quality claims SHALL require recorded model trials.

#### Scenario: Helper comparison uses the same task conditions
- **WHEN** an evaluator compares primitive-only and helper-assisted generation
- **THEN** task prompts, assets, fonts, model and budget SHALL be recorded consistently and requested revisions SHALL be evaluated separately from initial appearance

## Runtime scope clarification (2026-10-28)

Per user direction, this graphics change targets the Rust and TypeScript IR runtimes. The legacy
HIR interpreter will not be updated for graphics support; its import/type-matching limitations are
accepted and do not block this change. `retire-hir-interpreter` remains a separate later change.
The preview compiles with nx-api/nx-codegen and evaluates linked images with nx-ir-runtime directly.
Graphics runtime acceptance and negative tests use IR, with no HIR parity requirement.


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
