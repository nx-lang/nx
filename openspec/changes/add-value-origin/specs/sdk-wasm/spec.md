## ADDED Requirements

### Requirement: Annotated record nodes carry their origin
Each `record` node of the `NxValueText` a program artifact's `evaluateNx()` returns SHALL carry an
`origin` holding the identity of the module and the source span of the element expression that
constructed the record, as the `value-origin` capability defines it. Adding origins SHALL NOT change
the result's `text`.

#### Scenario: Two records of one type have different origins
- **WHEN** `root` returns `{ <User id="1" name="Ada" /> <User id="2" name="Sam" /> }`, each element
  written on its own line of the entry module
- **THEN** the two `record` nodes SHALL share the declaration span of `type User`
- **AND** each SHALL carry as its origin the span of its own `<User … />` element in the entry
  module

#### Scenario: The text is unchanged
- **WHEN** the wasm SDK's parity tests run with origins added
- **THEN** every result's `text` SHALL equal what `nxlang run` prints, as before
