## ADDED Requirements

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
