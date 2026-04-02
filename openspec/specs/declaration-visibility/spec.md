# declaration-visibility Specification

## Purpose
Define public, internal, and private visibility for top-level NX declarations across file,
library, and consumer boundaries.

## Requirements
### Requirement: Top-level declarations support explicit visibility modifiers
The parser and lowered representation SHALL support `private` and `internal` as optional visibility
modifiers on top-level `let`, `type`, `enum`, record, and `component` declarations. When no
visibility keyword is present, the declaration SHALL be public.

#### Scenario: Private declaration is lowered with private visibility
- **WHEN** a file contains `private let formatName(name:string) = name`
- **THEN** parsing and lowering SHALL preserve `formatName` as a top-level declaration with
  visibility `private`

#### Scenario: Internal declaration is lowered with internal visibility
- **WHEN** a file contains `internal component <Button/> = { <button/> }`
- **THEN** parsing and lowering SHALL preserve `Button` as a top-level declaration with visibility
  `internal`

#### Scenario: Declaration without a visibility modifier is public
- **WHEN** a file contains `type Theme = string`
- **THEN** parsing and lowering SHALL preserve `Theme` as a top-level declaration with visibility
  `public`

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

### Requirement: Internal declarations are visible throughout the declaring library
The system SHALL make an `internal` declaration visible to other files in the same library while
excluding it from the exported declarations visible to external consumers.

#### Scenario: Peer file in the same library can resolve an internal declaration
- **WHEN** `helpers.nx` contains `internal let formatName(name:string) = name` and `page.nx` in the
  same library references `formatName`
- **THEN** analysis SHALL resolve `formatName` successfully

#### Scenario: External consumer cannot import an internal declaration
- **WHEN** library `../ui` contains `internal component <ButtonBase/> = { <button/> }` and another
  library contains `import "../ui"`
- **THEN** the import SHALL NOT make `ButtonBase` available to the consumer

### Requirement: Public declarations are exported to consumers by default
Declarations without a visibility modifier SHALL be exported from their library and SHALL be visible
within the declaring file, to peer files in the same library, and to importing consumer libraries.

#### Scenario: Public declaration is visible to a consumer import
- **WHEN** library `../ui` contains `component <Button/> = { <button/> }` and another library
  contains `import "../ui"`
- **THEN** the import SHALL make `Button` available to the consumer

#### Scenario: Public declaration is visible to a peer file in the same library
- **WHEN** `button.nx` contains `component <Button/> = { <button/> }` and `page.nx` in the same
  library references `Button`
- **THEN** analysis SHALL resolve `Button` successfully
