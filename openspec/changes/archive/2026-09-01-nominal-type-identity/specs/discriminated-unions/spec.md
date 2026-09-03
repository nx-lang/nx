## MODIFIED Requirements

### Requirement: Payloadless union cases support contextual construction
At a binding site whose declared type is a discriminated union, the system SHALL accept a bare case
name and SHALL resolve it against that union's payloadless cases. A bare name that matches a case
requiring payload construction SHALL be rejected, and the diagnostic SHALL direct the author to the
element-style constructor. Resolution SHALL NOT require the union type to be in lexical scope at the
use site.

A resolved case SHALL lower and evaluate whether or not the using module can name its union,
including a case that inherits fields from an abstract base.

#### Scenario: Payloadless case is constructed by bare name
- **WHEN** a file contains `type LoadState = idle | loading component <View state:LoadState /> = { <div /> } let v = <View state=loading />`
- **THEN** type checking SHALL accept `loading` as a value of type `LoadState`
- **AND** interpretation SHALL produce a case value with discriminator `LoadState.loading`

#### Scenario: Bare name for a payload case is rejected
- **WHEN** a file contains `type LoadState = idle | failed { message:string } component <View state:LoadState /> = { <div /> } let v = <View state=failed />`
- **THEN** type checking SHALL reject `failed` because that case requires payload construction
- **AND** the diagnostic SHALL direct the author to `<LoadState.failed ... />`

#### Scenario: Union-typed property default accepts a bare case name
- **WHEN** a file contains `type LoadState = idle | loading external component <View state:LoadState = idle />`
- **THEN** type checking SHALL accept `idle` as the default for `state`

#### Scenario: Case of a union declared in another module is constructed by bare name
- **WHEN** a module imports a component whose property is typed by a union declared in another
  module, and does not import that union
- **THEN** a bare payloadless case name SHALL be accepted at that property
- **AND** it SHALL evaluate to the same value as the qualified form of that case

#### Scenario: Foreign case inheriting an abstract base is constructed with its base fields
- **WHEN** the union in the declaring module extends an abstract record base, and a bare payloadless
  case of it is written at a property of an imported component
- **THEN** the constructed value SHALL carry the base's fields and their defaults
