# declaration-visibility Specification

## Purpose
Define private, default-internal, and export visibility for top-level NX declarations across file,
library, and consumer boundaries.

## Requirements
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

### Requirement: Private declarations are visible only within the declaring file
The system SHALL allow references to a `private` declaration only from the file that declares it.
Other files in the same library and external consumers MUST NOT resolve that declaration.

#### Scenario: Same file can reference a private declaration
- **WHEN** a file contains `private let footerText = "Built with NX"` and `let <Footer/> =
  <footer>{footerText}</footer>`
- **THEN** the file SHALL compile successfully

#### Scenario: Peer file in the same library cannot resolve a private declaration
- **WHEN** `helpers.nx` contains `private let formatName(name:string) = name` and `page.nx` in the
  same library references `formatName`
- **THEN** analysis SHALL report an unresolved-name error for `formatName`

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
importing consumer libraries, SHALL remain visible within the declaring library or program, and
SHALL define the type surface eligible for external code generation.

#### Scenario: Export declaration is visible to a consumer import
- **WHEN** library `../ui` contains `export component <Button/> = { <button/> }` and another
  library contains `import "../ui"`
- **THEN** the import SHALL make `Button` available to the consumer

#### Scenario: Export declaration is visible to a peer file in the same library
- **WHEN** `button.nx` contains `export component <Button/> = { <button/> }` and `page.nx` in the
  same library references `Button`
- **THEN** analysis SHALL resolve `Button` successfully

#### Scenario: Exported type declaration is included in generated code
- **WHEN** `types.nx` contains `export type Theme = string`
- **THEN** NX code generation SHALL include `Theme` in the generated output

#### Scenario: Default-internal type declaration is excluded from generated code
- **WHEN** `types.nx` contains `type Theme = string`
- **THEN** NX code generation SHALL omit `Theme` from the generated output
