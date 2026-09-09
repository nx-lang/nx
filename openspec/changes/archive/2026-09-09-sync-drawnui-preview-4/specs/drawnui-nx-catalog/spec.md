## MODIFIED Requirements

### Requirement: Catalog excludes members that cannot be authored
The catalog SHALL exclude DrawnUI members that are not author-settable inputs: engine-internal and
computed state, references to engine objects, and — for this change — every callback or event
property, because the JavaScript IR runtime cannot dispatch NX actions.

#### Scenario: Callback properties are omitted
- **WHEN** a DrawnUI control declares a property whose type is a function
- **THEN** the catalog SHALL NOT declare that property

#### Scenario: Engine state is omitted
- **WHEN** a DrawnUI member exists to carry measured, cached, or otherwise engine-computed state
- **THEN** the catalog SHALL NOT declare that member as a property

#### Scenario: Engine object references are omitted
- **WHEN** a DrawnUI property's type is an engine class — a control, an effect, the canvas, or any
  other class that is not one of DrawnUI's value types
- **THEN** the catalog SHALL NOT declare that property
- **AND** it SHALL NOT declare a record type for that class
- **AND** the omission SHALL be recorded alongside the other excluded members

#### Scenario: Setting an excluded property is reported
- **WHEN** an author sets a property the catalog excludes
- **THEN** compilation SHALL fail with a diagnostic naming the unknown property
