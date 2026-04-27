## ADDED Requirements

### Requirement: Property-list fragments participate in content-property binding rules
Conditional property-list fragments SHALL participate in the same content-property validation rules
as direct explicit properties. A content property supplied by a property-list fragment MUST be
treated as an explicit named property on every reachable path where that fragment is active.

#### Scenario: Content property supplied by every branch satisfies required content
- **WHEN** a component declares `content body:Element`
- **AND** a call site contains `<Panel if compact { body=<span /> } else { body=<section /> } />`
- **THEN** type checking SHALL accept the invocation because every reachable branch supplies
  `body`

#### Scenario: Conditional content property conflicts with body content
- **WHEN** a component declares `content body:Element`
- **AND** a call site contains `<Panel if compact { body=<span /> }><Badge /></Panel>`
- **THEN** type checking SHALL reject the invocation because `body` can be supplied by both a named
  property fragment and element body content on the `compact` path

#### Scenario: Mutually exclusive content property branches are accepted
- **WHEN** a component declares `content body:Element`
- **AND** a call site contains `<Panel if compact { body=<span /> } else { body=<section /> } />`
- **THEN** type checking SHALL NOT report a duplicate content-property diagnostic for the mutually
  exclusive `body` branches
