## MODIFIED Requirements

### Requirement: Component declaration syntax
The parser SHALL support top-level component declarations introduced by the `component` keyword. A
component declaration MAY be preceded by the `abstract` and/or `external` modifiers and MAY declare
a single `extends BaseComponent` clause inside its element-shaped signature. Concrete non-external
component declarations SHALL use a block body introduced by `=`, abstract component declarations
SHALL omit the body, and concrete external component declarations MAY omit the body or MAY use a
body that contains only `state`.

#### Scenario: Minimal component declaration
- **WHEN** a file contains `component <Button text:string /> = { <button>{text}</button> }`
- **THEN** the parser SHALL produce a COMPONENT_DEFINITION node with name `Button`, one
  PROPERTY_DEFINITION for `text`, and a COMPONENT_BODY containing the rendered element

#### Scenario: Bodyless abstract and bodyless external component declarations
- **WHEN** a file contains `abstract component <SearchBase placeholder:string emits { SearchRequested } /> external component <SearchBox extends SearchBase showSearchIcon:bool = true /> abstract external component <RemoteSearchBase placeholder:string />`
- **THEN** the parser SHALL produce three COMPONENT_DEFINITION nodes
- **AND** SHALL preserve `SearchBase` as abstract with no base and no COMPONENT_BODY
- **AND** SHALL preserve `SearchBox` as external with base `SearchBase` and no COMPONENT_BODY
- **AND** SHALL preserve `RemoteSearchBase` as both abstract and external with no COMPONENT_BODY

#### Scenario: External component with a state-only body
- **WHEN** a file contains `external component <SearchBox placeholder:string /> = { state { query:string } }`
- **THEN** the parser SHALL produce a COMPONENT_DEFINITION named `SearchBox`
- **AND** SHALL preserve a COMPONENT_BODY containing a STATE_GROUP with one PROPERTY_DEFINITION
  named `query`
- **AND** SHALL preserve no rendered component body expression

#### Scenario: Concrete derived component declaration
- **WHEN** a file contains `component <NxSearchUi extends SearchBase showSpinner:bool = false /> = { <SearchBox /> }`
- **THEN** the parser SHALL produce a COMPONENT_DEFINITION named `NxSearchUi` whose signature base is `SearchBase`
- **AND** SHALL preserve one PROPERTY_DEFINITION named `showSpinner` and a COMPONENT_BODY

#### Scenario: Concrete bodyless component declaration is rejected
- **WHEN** a file contains `component <SearchBox placeholder:string />`
- **THEN** parsing or validation SHALL reject the declaration as an invalid component definition

#### Scenario: Empty external component body is rejected
- **WHEN** a file contains `external component <SearchBox placeholder:string /> = { }`
- **THEN** parsing or validation SHALL reject the declaration as an invalid component definition

#### Scenario: Abstract component body or external rendered body is rejected
- **WHEN** a file contains `abstract component <SearchBase /> = { <button /> } external component <SearchBox /> = { state { query:string } <button /> }`
- **THEN** parsing or validation SHALL reject both declarations as an invalid component definition

#### Scenario: Multiple base components are rejected
- **WHEN** a file contains `component <SearchBox extends SearchBase, QueryBase /> = { <button /> }`
- **THEN** parsing or validation SHALL reject the declaration as an invalid component inheritance clause

#### Scenario: Component declaration coexists with other module items
- **WHEN** a file contains imports, a component declaration, and a root element
- **THEN** the parser SHALL produce a valid MODULE_DEFINITION that includes the COMPONENT_DEFINITION alongside the other top-level items

### Requirement: Component state group
A component body SHALL support an optional `state` group before the rendered body expression for
concrete non-external components. For concrete external components, when a body is present it SHALL
consist only of a `state` group and SHALL NOT include a rendered body expression.

#### Scenario: Component with state group
- **WHEN** a file contains `component <SearchBox placeholder:string /> = { state { query:string } <TextInput /> }`
- **THEN** the parser SHALL produce a COMPONENT_BODY containing a STATE_GROUP with one
  PROPERTY_DEFINITION named `query` followed by the rendered element expression

#### Scenario: External component with state-only body
- **WHEN** a file contains `external component <SearchBox placeholder:string /> = { state { query:string } }`
- **THEN** the parser SHALL produce a COMPONENT_BODY containing a STATE_GROUP with one
  PROPERTY_DEFINITION named `query`
- **AND** SHALL preserve no rendered body expression

#### Scenario: Component body without state group
- **WHEN** a file contains `component <SearchBox placeholder:string /> = { if isReady { <TextInput /> } else { <Spinner /> } }`
- **THEN** the parser SHALL produce a valid COMPONENT_DEFINITION with no STATE_GROUP and an `if`
  expression as the component body
