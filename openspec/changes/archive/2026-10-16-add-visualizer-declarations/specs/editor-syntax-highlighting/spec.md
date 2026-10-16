## ADDED Requirements

### Requirement: Visualizer declarations are scoped as declarations
The grammar SHALL scope `visualizer` as a declaration keyword when it is the first token of a module
member followed by a type name and `=`, SHALL scope the target name as a type reference, and SHALL
scope the value as any other right-hand side. `visualizer` in any other position SHALL keep the scope
the same position gives any other name.

#### Scenario: A visualizer declaration
- **WHEN** a file contains `visualizer SingleChoice = <ElementVisualizer icon="circle-dot" />`
- **THEN** `visualizer` SHALL be scoped `keyword.declaration.visualizer.nx`
- **AND** `SingleChoice` SHALL be scoped `entity.name.type.nx`
- **AND** `ElementVisualizer` SHALL be scoped as an element tag, as in any other value

#### Scenario: `visualizer` as a property name
- **WHEN** a file contains `type Settings = { visualizer:string }`
- **THEN** `visualizer` SHALL be scoped as a property name and not as a keyword
