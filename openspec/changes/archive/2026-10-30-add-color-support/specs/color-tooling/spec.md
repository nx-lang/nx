## Purpose

Give authors consistent color diagnostics, swatches, and safe editing of shared NX colors without requiring a graphics preview or executing arbitrary source code.

## ADDED Requirements

### Requirement: Discover colors through canonical type and function identity
Language-service color metadata SHALL identify the canonical @nx/color Color and its parse/rgb/rgba/srgb constructors through aliases and re-exports. Source and Visual views SHALL show swatches for statically recognized expressions and typed evaluated Color values. Discovery SHALL support hex literals, parse with literal text, rgb/rgba/srgb with literal numbers, and direct Color records with literal channels/default alpha. It SHALL NOT require graphics or execute arbitrary NX functions. Unrelated types named Color SHALL NOT receive these semantics automatically. Existing string format=color visualizers SHALL retain their presentation behavior.

#### Scenario: Aliased constructor and unrelated names
- **WHEN** source uses an aliased standard parse call and also a user-defined Color record
- **THEN** the standard call SHALL be recognized and the unrelated record SHALL not automatically be treated as the standard color

### Requirement: Literal diagnostics agree with runtime validation
Hex literals and statically recognized parse literal inputs and direct constructor/channel literals SHALL receive diagnostics consistent with the color-library parsing and range rules. Diagnostics SHALL locate the offending input and use color-literal diagnostics for malformed hex tokens and color-invalid-text or color-invalid-channel for library inputs. Computed expressions SHALL not be speculatively evaluated. Malformed literal text passed to tryParse SHALL NOT produce a static invalid-color diagnostic.

#### Scenario: Invalid authored literal
- **WHEN** source contains a standard parse("#xyz") call
- **THEN** analysis SHALL mark that string with color-invalid-text before rendering or runtime evaluation

#### Scenario: Recoverable parsing is intentional
- **WHEN** source calls tryParse("#xyz")
- **THEN** static analysis SHALL permit its invalid-result path without emitting color-invalid-text

### Requirement: Picker edits preserve authored structure and alpha
The playground picker SHALL support RGB and alpha editing for safely editable direct colors. Edits SHALL change only a hex token, the literal text argument of parse, literal numeric arguments of rgb/rgba/srgb, or literal numeric fields of directly authored Color records. They SHALL preserve unrelated source, comments, and unchanged channel precision, use normal undo, and reject stale source revisions. Opening or dismissing a picker SHALL not change source or quantize channels. Unsupported or computed expressions SHALL remain read-only; the picker SHALL NOT replace them with a constant.

#### Scenario: Transparent literal is edited safely
- **WHEN** an author changes RGB on #33669980 without changing alpha
- **THEN** the edit SHALL retain alpha float32(128/255) and surrounding source/comments and SHALL be undoable

#### Scenario: High-precision value is merely inspected
- **WHEN** an author opens then closes a picker for srgb(0.123456789,0.4,0.8,0.5)
- **THEN** the source SHALL remain byte-for-byte unchanged

#### Scenario: Computed color and stale edits
- **WHEN** a property references a computed color, or a direct literal changes after its picker was opened
- **THEN** a computed expression SHALL remain read-only and a stale literal edit SHALL be rejected until rediscovery

### Requirement: Literal picker edits retain precision deliberately
Hex edits SHALL emit lowercase six-digit RGB when opaque, or eight-digit RGBA when alpha is present, retaining an existing explicit alpha suffix. RGB-only changes SHALL preserve existing alpha. Constructor edits SHALL preserve unchanged stored float32 channels and SHALL NOT silently rewrite higher-precision colors to byte-based hex. Other constructor/conversion/transformation expressions SHALL remain read-only unless a safe literal-edit operation is supported explicitly.

#### Scenario: Short literal edit
- **WHEN** a user edits #f008 without changing its alpha
- **THEN** the editor SHALL emit an eight-digit color retaining alpha 0x88 and modify only that token
