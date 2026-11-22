## MODIFIED Requirements

### Requirement: Discover colors through canonical type and function identity
Language-service color metadata SHALL identify the canonical @nx/color Color, its four cases, and
its parse, rgb, rgba, srgb, displayP3, oklab and oklch constructors through aliases and re-exports.
Source and Visual views SHALL show swatches for statically recognized expressions and typed
evaluated Color values. Discovery SHALL support:
- hex literals;
- parse with literal text;
- rgb, rgba, srgb, displayP3, oklab and oklch with literal numbers;
- direct `Color.srgb`, `Color.displayP3`, `Color.oklab` and `Color.oklch` records with literal
  channels and default alpha.

A swatch SHALL be painted from the color's `toCss` text, so a browser shows a wide color as its
screen allows. A swatch for a color that `inGamut(value, srgb)` rejects SHALL carry a mark, with
accessible text saying that the color is outside sRGB and looks different on sRGB screens.
Discovery SHALL NOT require graphics or execute arbitrary NX functions. Unrelated types named Color
SHALL NOT receive these semantics automatically. Existing string format=color visualizers SHALL
retain their presentation behavior.

#### Scenario: Aliased constructor and unrelated names
- **WHEN** source uses an aliased standard parse call and also a user-defined Color record
- **THEN** the standard call SHALL be recognized and the unrelated record SHALL not automatically be treated as the standard color

#### Scenario: A wide color is marked
- **WHEN** source contains `oklch(0.7, 0.3, 145)` and `#2563eb`
- **THEN** both SHALL show swatches, and only the first SHALL carry the outside-sRGB mark

### Requirement: Picker edits preserve authored structure and alpha
The playground picker SHALL support RGB and alpha editing for safely editable direct `Color.srgb`
colors. Edits SHALL change only a hex token, the literal text argument of parse when it parses to
`Color.srgb`, literal numeric arguments of rgb/rgba/srgb, or literal numeric fields of directly
authored `Color.srgb` records. They SHALL preserve unrelated source, comments, and unchanged
channel precision, use normal undo, and reject stale source revisions. Opening or dismissing a
picker SHALL not change source or quantize channels. A recognized color in another space SHALL be shown
read-only, with its swatch and its CSS text, and SHALL NOT be offered to the sRGB picker. Unsupported or computed expressions SHALL remain read-only; the
picker SHALL NOT replace them with a constant.

#### Scenario: Transparent literal is edited safely
- **WHEN** an author changes RGB on #33669980 without changing alpha
- **THEN** the edit SHALL retain alpha float32(128/255) and surrounding source/comments and SHALL be undoable

#### Scenario: High-precision value is merely inspected
- **WHEN** an author opens then closes a picker for srgb(0.123456789,0.4,0.8,0.5)
- **THEN** the source SHALL remain byte-for-byte unchanged

#### Scenario: Computed color and stale edits
- **WHEN** a property references a computed color, or a direct literal changes after its picker was opened
- **THEN** a computed expression SHALL remain read-only and a stale literal edit SHALL be rejected until rediscovery

#### Scenario: A wide color is not rewritten as sRGB
- **WHEN** a document holds `displayP3(1, 0, 0)`
- **THEN** its swatch SHALL be shown, and no picker edit SHALL be offered that would rewrite the color as hex
