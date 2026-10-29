# graphics-rendering Specification

## Purpose

Defines safe preparation and direct rendering of the initial NX graphics profile through a backend-neutral drawing contract with an initial Skia implementation.

## Requirements

### Requirement: Scene preparation validates known declaration identities before painting
The system SHALL accept evaluated NX graphics values with their linked declaration identities, validate the entire scene against the supported profile and host limits, and prepare semantic geometry before painting. Unknown/spoofed component identities, malformed paths, invalid dimensions/numbers and excess node/depth/path/surface/layer complexity SHALL fail preparation. Diagnostics SHALL identify the affected scene element and provenance when available. Prepared content SHALL NOT require a standardized NX display list.

#### Scenario: Invalid path fails before any paint
- **WHEN** a later child contains malformed SVG path data after an otherwise valid earlier child
- **THEN** preparation SHALL fail before any child is painted and SHALL identify the bad child

#### Scenario: Name spoofing is rejected
- **WHEN** a user declaration shares the display name Rect but is not the linked graphics Rect contract
- **THEN** it SHALL NOT be accepted as that primitive merely by matching its name

### Requirement: Graphics painting preserves geometry and compositing semantics
Rendering SHALL preserve child paint order, local affine transforms using SVG matrix order, Drawing view-box fit/alignment/viewport clipping, fill/stroke rules, and clipping in transformed local coordinates. Opacity SHALL apply to the isolated result of the entire node subtree. Zero-width strokes SHALL paint nothing; odd dash patterns SHALL repeat to an even pattern and all-zero patterns SHALL be solid. SVG path arcs and open/closed subpath semantics SHALL be supported. Numeric backend conversions SHALL reject unrepresentable values rather than silently overflow.

#### Scenario: Transform order is observable
- **WHEN** a local point (1,0) uses the transform list translate(10,0), scale(2,2)
- **THEN** its transformed coordinate SHALL be (12,0), before root viewport mapping

#### Scenario: Group opacity preserves overlap
- **WHEN** two overlapping opaque shapes are children of a group with opacity 0.5
- **THEN** the group SHALL be composited once at 0.5 and the overlap SHALL NOT accumulate child opacity

#### Scenario: Clip holes are independent of fill rules
- **WHEN** a node has an evenodd compound clip and a nonzero shape fill
- **THEN** clip holes SHALL remain transparent and the shape fill SHALL retain nonzero semantics

### Requirement: Direct Skia support remains an optional host adapter
The system SHALL provide direct Skia rendering to a host-supplied canvas and an offscreen raster surface. Drawing operations SHALL dispatch directly without constructing an intermediate standardized NX command list. Backend-neutral public contracts SHALL contain no Skia/platform objects, and using the compiler/generic runtime SHALL NOT require linking Skia. All exits SHALL restore the host canvas state; painting failure SHALL be reported and SHALL NOT be described as a successful frame.

#### Scenario: Borrowed canvas state is restored
- **WHEN** painting completes or a backend reports failure inside a nested clip/opacity scope
- **THEN** the host canvas save depth, transform and clip SHALL be restored to entry state
- **AND** a failure SHALL be returned to the host; pixel rollback on a borrowed canvas SHALL NOT be promised

#### Scenario: Direct rendering works without a recorder
- **WHEN** a host paints a prepared graphics-core scene with only the Skia destination installed
- **THEN** it SHALL render successfully without any recorder/display-list implementation

### Requirement: Measured text and painted text use the same shaped run
The text service SHALL resolve host-supplied font identities, shape supported single-line text, retain the glyph positions/resources used for measurement, and paint that layout. Missing font or glyph coverage SHALL be an error. The service SHALL return logical advance, ink bounds, ascent/descent and baseline, and anchoring SHALL use shaped advance. No implicit machine-dependent font fallback SHALL replace a requested face.

#### Scenario: Combining glyphs are measured consistently
- **WHEN** a supplied font supports a string with a combining sequence or ligature
- **THEN** measured bounds/advance and rendered glyph positions SHALL derive from the same shaped layout

#### Scenario: Missing glyph coverage fails clearly
- **WHEN** the chosen host font cannot render a required character
- **THEN** preparation SHALL identify the text/font failure and SHALL NOT silently substitute missing-glyph boxes
