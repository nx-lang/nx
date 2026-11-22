## MODIFIED Requirements

### Requirement: Graphics consumes the shared color contract
@nx/graphics SHALL import and re-export the canonical @nx/color Color rather than declaring a
separate string alias. Solid paint colors, gradient stops and drawing backgrounds SHALL accept a
shared Color in any of its spaces, including values authored as hex literals. Standard-library
linking SHALL include color transitively when required, without requiring an application-level
import. Scene preparation SHALL validate each color's channels for its space, including raw SDK
records, and SHALL then convert it with `toGamut(value, srgb)` before painting, so prepared paints
hold sRGB float32 channels. Direct strings in Color fields SHALL fail type checking or scene
validation rather than being silently reparsed. Native and browser renderers SHALL preserve
numerical channel precision from the prepared color until backend conversion.

#### Scenario: One shared color across application and drawing
- **WHEN** a program supplies the same @nx/color Color to an application brand record and a graphics solid fill
- **THEN** both SHALL accept the canonical value without a duplicate type or string conversion

#### Scenario: Graphics-only import links constructors
- **WHEN** a program imports graphics and its evaluated program needs the transitive color image
- **THEN** linking SHALL include the required image or report its absence before painting

#### Scenario: Invalid SDK color is rejected before painting
- **WHEN** a later scene element contains a `Color.srgb` channel outside [0,1], a negative `Color.oklch` chroma, or non-finite channels
- **THEN** preparation SHALL fail before painting any element and identify the element/channel

#### Scenario: A wide color is mapped at preparation
- **WHEN** a Rectangle is filled with `displayP3(1, 0, 0)`
- **THEN** its prepared paint SHALL hold the channels of `toGamut(displayP3(1, 0, 0), srgb)`

### Requirement: Changed graphics color semantics have a new profile
Graphics SHALL identify the shared-color, gradient paint, and shared-geometry contract, with colors
in any space mapped into sRGB at preparation, as graphics-core-v5 in documentation, preparation
metadata, previews, and fixtures. Existing geometry, solid/none paint behavior, opacity, and
unsupported-effect diagnostics SHALL remain applicable. Supported native and browser paths SHALL
consume the prepared sRGB color consistently.

#### Scenario: Matching native and browser color semantics
- **WHEN** a scene with a translucent `Color.oklch` is prepared for native Skia and browser Canvas
- **THEN** both SHALL report graphics-core-v5 and draw the same mapped sRGB-plus-alpha color
