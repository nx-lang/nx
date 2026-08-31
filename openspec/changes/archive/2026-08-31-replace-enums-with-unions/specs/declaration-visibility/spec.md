## MODIFIED Requirements

### Requirement: Top-level declarations support explicit visibility modifiers
The parser and lowered representation SHALL support `private` and `export` as optional visibility
modifiers on top-level `let`, `type`, record, and `component` declarations. When no visibility
keyword is present, the declaration SHALL have visibility `internal`.

#### Scenario: Private declaration is lowered with private visibility
- **WHEN** a file contains `private let formatName(name:string) = name`
- **THEN** parsing and lowering SHALL preserve `formatName` as a top-level declaration with
  visibility `private`

#### Scenario: Export declaration is lowered with export visibility
- **WHEN** a file contains `export component <Button/> = { <button/> }`
- **THEN** parsing and lowering SHALL preserve `Button` as a top-level declaration with visibility
  `export`

#### Scenario: Declaration without a visibility modifier is internal
- **WHEN** a file contains `type Theme = string`
- **THEN** parsing and lowering SHALL preserve `Theme` as a top-level declaration with visibility
  `internal`

#### Scenario: Union declaration accepts a visibility modifier
- **WHEN** a file contains `export type ThemeMode = light | dark`
- **THEN** parsing and lowering SHALL preserve `ThemeMode` as a top-level declaration with
  visibility `export`
