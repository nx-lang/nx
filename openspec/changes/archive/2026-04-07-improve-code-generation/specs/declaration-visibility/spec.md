## MODIFIED Requirements

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
