## ADDED Requirements

### Requirement: Replacing a color token with its value writes sRGB hex only for an sRGB color
When a reader asks Visual to replace a color token with its value, the editor SHALL open on the
token's color preview only when that color is inside sRGB, and committing it SHALL write the color as
a hex literal, its channels rounded to 8 bits as the picker's hex is. This is an explicit replacement
the reader asks for, not a picker edit of a computed expression, which stays read-only. A token whose
preview is outside sRGB SHALL NOT offer the replacement, so a wide color is never rewritten as sRGB;
the reader can still choose another name, or write the color in Source.

#### Scenario: A gray token becomes hex
- **WHEN** a Card writes `border={env.color.border}`, whose preview is `standardPalette.gray200`, a
  color inside sRGB, and the reader chooses Replace with value and commits the picker unchanged
- **THEN** the attribute SHALL read `border=` and that color's hex

#### Scenario: A wide token is not offered
- **WHEN** a token's preview is a color outside sRGB, such as `standardPalette.green500`
- **THEN** its menu SHALL NOT offer Replace with value
