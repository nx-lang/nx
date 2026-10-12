## ADDED Requirements

### Requirement: Viewer declarations are scoped as declarations
The grammar SHALL scope `viewer` as a declaration keyword when it is the first token of a module
member followed by a type name and `=`, SHALL scope the target name as a type reference, and SHALL
scope the value as any other right-hand side. `viewer` in any other position SHALL keep the scope
the same position gives any other name.

#### Scenario: A viewer declaration
- **WHEN** a file contains `viewer SingleChoice = <ElementViewer name="Single choice" />`
- **THEN** `viewer` SHALL be scoped `keyword.declaration.viewer.nx`
- **AND** `SingleChoice` SHALL be scoped `entity.name.type.nx`
- **AND** `ElementViewer` SHALL be scoped as an element tag, as in any other value

#### Scenario: `viewer` as a property name
- **WHEN** a file contains `type Settings = { viewer:string }`
- **THEN** `viewer` SHALL be scoped as a property name and not as a keyword
