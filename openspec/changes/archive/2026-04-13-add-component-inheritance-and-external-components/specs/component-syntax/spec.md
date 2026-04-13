## MODIFIED Requirements

### Requirement: Component declaration syntax
The parser SHALL support top-level component declarations introduced by the `component` keyword. A
component declaration MAY be preceded by the `abstract` and/or `external` modifiers and MAY declare
a single `extends BaseComponent` clause inside its element-shaped signature. Concrete component
declarations SHALL use a block body introduced by `=`, while abstract or external component
declarations SHALL omit the body and define only their public props and emitted actions.

#### Scenario: Minimal component declaration
- **WHEN** a file contains `component <Button text:string /> = { <button>{text}</button> }`
- **THEN** the parser SHALL produce a COMPONENT_DEFINITION node with name `Button`, one PROPERTY_DEFINITION for `text`, and a COMPONENT_BODY containing the rendered element

#### Scenario: Bodyless abstract and external component declarations
- **WHEN** a file contains `abstract component <SearchBase placeholder:string emits { SearchRequested } /> external component <SearchBox extends SearchBase showSearchIcon:bool = true /> abstract external component <RemoteSearchBase placeholder:string />`
- **THEN** the parser SHALL produce three COMPONENT_DEFINITION nodes
- **AND** SHALL preserve `SearchBase` as abstract with no base and no COMPONENT_BODY
- **AND** SHALL preserve `SearchBox` as external with base `SearchBase` and no COMPONENT_BODY
- **AND** SHALL preserve `RemoteSearchBase` as both abstract and external with no COMPONENT_BODY

#### Scenario: Concrete derived component declaration
- **WHEN** a file contains `component <NxSearchUi extends SearchBase showSpinner:bool = false /> = { <SearchBox /> }`
- **THEN** the parser SHALL produce a COMPONENT_DEFINITION named `NxSearchUi` whose signature base is `SearchBase`
- **AND** SHALL preserve one PROPERTY_DEFINITION named `showSpinner` and a COMPONENT_BODY

#### Scenario: Concrete bodyless component declaration is rejected
- **WHEN** a file contains `component <SearchBox placeholder:string />`
- **THEN** parsing or validation SHALL reject the declaration as an invalid component definition

#### Scenario: Abstract or external component body is rejected
- **WHEN** a file contains `abstract component <SearchBase /> = { <button /> } external component <SearchBox /> = { <button /> }`
- **THEN** parsing or validation SHALL reject both declarations as invalid component definitions

#### Scenario: Multiple base components are rejected
- **WHEN** a file contains `component <SearchBox extends SearchBase, QueryBase /> = { <button /> }`
- **THEN** parsing or validation SHALL reject the declaration as an invalid component inheritance clause

#### Scenario: Component declaration coexists with other module items
- **WHEN** a file contains imports, a component declaration, and a root element
- **THEN** the parser SHALL produce a valid MODULE_DEFINITION that includes the COMPONENT_DEFINITION alongside the other top-level items

### Requirement: Component declaration keywords
The parser SHALL recognize `abstract`, `external`, `component`, `extends`, `emits`, and `state`
as declaration keywords within component syntax.

#### Scenario: Component keyword starts a component declaration
- **WHEN** a file contains `component <SearchBox /> = { <TextInput /> }`
- **THEN** the parser SHALL recognize `component` as the declaration keyword for a COMPONENT_DEFINITION

#### Scenario: Abstract, external, and extends participate in component declarations
- **WHEN** a file contains `abstract external component <SearchBox extends SearchBase />`
- **THEN** the parser SHALL recognize `abstract` and `external` as component modifiers
- **AND** SHALL recognize `extends` as the start of the component base clause

#### Scenario: Emits and state keywords introduce their respective groups
- **WHEN** a file contains a component signature with `emits { Changed { value:string } }` and a concrete body with `state { query:string }`
- **THEN** the parser SHALL recognize `emits` as the start of an EMITS_GROUP and `state` as the start of a STATE_GROUP

### Requirement: Component invocation action handler bindings
A component invocation SHALL interpret a property named `on<ActionName>` as an emitted action
handler binding when the target component declares or inherits `ActionName` in its effective emits
set. The handler body SHALL use the same expression syntax as any other property value and SHALL
have access to an implicit `action` identifier during lowering and invocation. When the matched
emitted action is inherited from an ancestor inline emit definition, `action` SHALL be bound as
that ancestor component's public emitted action type.

#### Scenario: Call site binds a handler for a shared emitted action
- **WHEN** a file contains `action SearchSubmitted = { searchString:string }`, `component <SearchBox emits { SearchSubmitted } /> = { <TextInput /> }`, and `<SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />`
- **THEN** lowering SHALL treat `onSearchSubmitted` as a handler binding for emitted action `SearchSubmitted` rather than as an ordinary component prop

#### Scenario: Call site binds a handler for an inline emitted action
- **WHEN** a file contains `component <SearchBox emits { ValueChanged { value:string } } /> = { <TextInput /> }` and `<SearchBox onValueChanged=<TrackSearch value={action.value} /> />`
- **THEN** lowering SHALL treat `onValueChanged` as a handler binding for emitted action `ValueChanged`
- **AND** SHALL bind `action` inside the handler body as a `SearchBox.ValueChanged` action value

#### Scenario: Call site binds a handler for an inherited shared emitted action
- **WHEN** a file contains `action SearchSubmitted = { searchString:string } abstract component <SearchBase emits { SearchSubmitted } /> component <NxSearchUi extends SearchBase /> = { <TextInput /> }` and `<NxSearchUi onSearchSubmitted=<DoSearch search={action.searchString} /> />`
- **THEN** lowering SHALL treat `onSearchSubmitted` as a handler binding for inherited emitted action `SearchSubmitted`

#### Scenario: Call site binds a handler for an inherited inline emitted action
- **WHEN** a file contains `abstract component <SearchBase emits { ValueChanged { value:string } } /> component <NxSearchUi extends SearchBase /> = { <TextInput /> }` and `<NxSearchUi onValueChanged=<TrackSearch value={action.value} /> />`
- **THEN** lowering SHALL treat `onValueChanged` as a handler binding for inherited emitted action `ValueChanged`
- **AND** SHALL bind `action` inside the handler body as a `SearchBase.ValueChanged` action value

#### Scenario: Non-matching on-prefixed properties remain ordinary props
- **WHEN** a file contains `component <SearchBox onClick:string /> = { <button /> }` and `<SearchBox onClick="primary" />`
- **THEN** lowering SHALL preserve `onClick` as an ordinary component prop because the component does not emit `Click`
