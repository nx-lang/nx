## ADDED Requirements

### Requirement: Definitions are reachable by canonical identity as well as by visible name
The system SHALL support resolving a definition by its declaring origin without requiring a visible
name for it in the module performing the resolution. Lookup by visible name SHALL remain available
for names an author writes, and SHALL NOT be the only means by which analysis, code generation, or
evaluation reaches a definition.

#### Scenario: A definition resolves through an origin its module cannot name
- **WHEN** a lowered reference carries the declaring origin of a definition that the using module
  neither declares nor imports
- **THEN** code generation and evaluation SHALL resolve that reference to the declaration
- **AND** resolution SHALL NOT depend on a binding for that name being visible in the using module

#### Scenario: A visible name resolving to a different definition does not capture the reference
- **WHEN** the using module has a visible binding whose name matches the declared name carried by a
  reference to a different definition
- **THEN** the reference SHALL resolve to the definition its origin names
- **AND** the visible binding SHALL NOT be substituted for it
