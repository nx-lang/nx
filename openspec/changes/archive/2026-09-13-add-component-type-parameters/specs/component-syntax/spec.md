## ADDED Requirements

### Requirement: Component signatures accept type parameter definitions
The parser SHALL accept, inside a component signature, a property definition whose type is the
keyword `type`, and SHALL produce for it a PROPERTY_DEFINITION whose type node is the `type`
keyword rather than a type reference. Such a definition MUST appear after the optional `extends`
clause and before every other property definition of the signature; parsing or validation SHALL
reject one that follows a regular property definition. Parsing or validation SHALL reject a `type`
property type in a record definition, an action definition, an emitted action field list, a state
group, and a function parameter list.

#### Scenario: Leading type parameters parse
- **WHEN** a file contains `external component <SkiaLayout extends SkiaControl TItem:type layoutType:string? itemsSource:TItem[]? />`
- **THEN** the parser SHALL produce a COMPONENT_DEFINITION with base `SkiaControl` and three PROPERTY_DEFINITION nodes
- **AND** the first SHALL be named `TItem` with the `type` keyword as its type
- **AND** the remaining two SHALL carry ordinary type references

#### Scenario: Type parameter after a regular property is rejected
- **WHEN** a file contains `component <Bad items:object[] TItem:type /> = { <Label /> }`
- **THEN** parsing or validation SHALL reject `TItem` as a type parameter declared after a property

#### Scenario: Type parameter outside a component signature is rejected
- **WHEN** a file contains `type Box = { T:type value:T }` and `component <C /> = { state { T:type } <Label /> }` and `let f(T:type) = 1`
- **THEN** parsing or validation SHALL reject each `type`-typed definition as unsupported outside a component signature
