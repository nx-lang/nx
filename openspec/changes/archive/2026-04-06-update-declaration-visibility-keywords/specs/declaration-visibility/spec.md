## MODIFIED Requirements

### Requirement: Top-level declarations support explicit visibility modifiers
The parser and lowered representation SHALL support `private` and `export` as optional visibility
modifiers on top-level `let`, `type`, `enum`, record, and `component` declarations. When no
visibility keyword is present, the declaration SHALL have visibility `internal`.

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

## ADDED Requirements

### Requirement: Default declarations are visible throughout the declaring library or program
The system SHALL make a declaration with no visibility keyword visible to other files in the same
library and to other root modules in the same non-library program, while excluding it from the
declarations visible to external library consumers.

#### Scenario: Peer file in the same library can resolve a default declaration
- **WHEN** `helpers.nx` contains `let formatName(name:string) = name` and `page.nx` in the same
  library references `formatName`
- **THEN** analysis SHALL resolve `formatName` successfully

#### Scenario: Other root modules in the same program can resolve a default declaration
- **WHEN** a non-library program includes `helpers.nx` containing `let answer() = 42`
- **AND** `main.nx` in the same program references `answer()`
- **THEN** analysis SHALL resolve `answer()` successfully within that program

#### Scenario: External consumer cannot import a default declaration
- **WHEN** library `../ui` contains `component <ButtonBase/> = { <button/> }` and another library
  contains `import "../ui"`
- **THEN** the import SHALL NOT make `ButtonBase` available to the consumer

### Requirement: Export declarations are visible to consumers explicitly
Declarations marked `export` SHALL be included in the library export metadata available to
importing consumer libraries and SHALL remain visible within the declaring library or program.

#### Scenario: Export declaration is visible to a consumer import
- **WHEN** library `../ui` contains `export component <Button/> = { <button/> }` and another
  library contains `import "../ui"`
- **THEN** the import SHALL make `Button` available to the consumer

#### Scenario: Export declaration is visible to a peer file in the same library
- **WHEN** `button.nx` contains `export component <Button/> = { <button/> }` and `page.nx` in the
  same library references `Button`
- **THEN** analysis SHALL resolve `Button` successfully

## REMOVED Requirements

### Requirement: Internal declarations are visible throughout the declaring library
**Reason**: The `internal` source keyword is removed. Library-scoped visibility remains, but it is
now the default when no visibility keyword is present.
**Migration**: None. Declarations that should remain library-local or program-local use no
visibility modifier.

### Requirement: Public declarations are exported to consumers by default
**Reason**: External exposure now requires an explicit `export` modifier so libraries do not
publish declarations unintentionally.
**Migration**: Add `export` to every declaration that must remain available to importing libraries.
