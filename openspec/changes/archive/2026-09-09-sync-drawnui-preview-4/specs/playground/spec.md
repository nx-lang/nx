## ADDED Requirements

### Requirement: Drawn text uses the demo's font configuration
The site SHALL register the same fonts and the same font defaults as the DrawnUI demo site, so that
an example ported from a demo page measures and draws its text as the original does, including
where the original names no font family.

#### Scenario: A label with no font family draws in the demo's text font
- **WHEN** an example draws a label that does not set a font family
- **THEN** the label SHALL be drawn in the text font the demo registers, not in the engine's
  built-in face

#### Scenario: A button caption follows the same default
- **WHEN** an example draws a button that does not set a font family
- **THEN** its caption SHALL be drawn in the same text font

#### Scenario: A named font family still wins
- **WHEN** an example sets a font family on a label or button
- **THEN** that family SHALL be used rather than the default
