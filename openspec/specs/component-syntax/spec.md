# component-syntax Specification

## Purpose
TBD - created by archiving change add-component-syntax. Update Purpose after archive.
## Requirements
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

### Requirement: Component emits group
A component signature SHALL support an optional `emits` group after its props. The `emits` group SHALL
contain one or more emitted action entries. An emitted action entry with a record-style field list
SHALL define a new component-scoped action whose public name is `<ComponentName>.<ActionName>`, and
it MAY declare a single optional `extends BaseAction` clause before the field list. An emitted
action entry without braces SHALL reference an existing action declaration. Inline emitted action
definitions SHALL remain concrete even when they extend an abstract action base.

#### Scenario: Component with multiple emitted action definitions
- **WHEN** a file contains `component <SearchBox placeholder:string emits { ValueChanged { value:string } SearchRequested { searchString:string } } /> = { <TextInput /> }`
- **THEN** the parser SHALL produce an EMITS_GROUP with two EMIT_DEFINITION entries named `ValueChanged` and `SearchRequested`

#### Scenario: Emitted action payload fields are parsed as properties
- **WHEN** a file contains `component <SearchBox emits { ValueChanged { value:string source:string } } /> = { <TextInput /> }`
- **THEN** the `ValueChanged` EMIT_DEFINITION SHALL contain two PROPERTY_DEFINITION nodes named `value` and `source`

#### Scenario: Component emits references an existing action
- **WHEN** a file contains `action ActionSharedWithMultipleComponents = { value:string }` and `component <MyComponent emits { ActionSharedWithMultipleComponents } /> = { <button /> }`
- **THEN** the parser SHALL produce an EMITS_GROUP containing an emitted action reference named `ActionSharedWithMultipleComponents` with no PROPERTY_DEFINITION children

#### Scenario: Component emits can mix definitions and references
- **WHEN** a file contains `action ActionSharedWithMultipleComponents = { value:string }` and `component <MyComponent emits { MyAction { value:string } ActionSharedWithMultipleComponents } /> = { <button /> }`
- **THEN** the parser SHALL preserve `MyAction` as an EMIT_DEFINITION with one PROPERTY_DEFINITION and `ActionSharedWithMultipleComponents` as a separate emitted action reference entry

#### Scenario: Inline emitted actions expose public qualified names
- **WHEN** a file contains `component <SearchBox emits { ValueChanged { value:string } } /> = { <TextInput /> }` and `let makeChange(value:string) = <SearchBox.ValueChanged value={value} />`
- **THEN** lowering SHALL resolve `SearchBox.ValueChanged` as the public name of the inline emitted action definition

#### Scenario: Component with inline emitted action definition that extends an abstract action
- **WHEN** a file contains `abstract action InputAction = { source:string } component <SearchBox emits { ValueChanged extends InputAction { value:string } } /> = { <TextInput /> }`
- **THEN** the parser SHALL produce an EMIT_DEFINITION entry named `ValueChanged` with base action
  `InputAction`
- **AND** lowering SHALL preserve the emitted action as the concrete public action
  `SearchBox.ValueChanged`

#### Scenario: Component emits can mix derived inline definitions and references
- **WHEN** a file contains `abstract action InputAction = { source:string } action SearchSubmitted = { query:string } component <SearchBox emits { ValueChanged extends InputAction { value:string } SearchSubmitted } /> = { <TextInput /> }`
- **THEN** the parser SHALL preserve `ValueChanged` as an EMIT_DEFINITION with base action
  `InputAction`
- **AND** SHALL preserve `SearchSubmitted` as a separate emitted action reference entry

#### Scenario: Inline emitted action rejects multiple base actions
- **WHEN** a file contains `component <SearchBox emits { ValueChanged extends InputAction, TrackingAction { value:string } } /> = { <TextInput /> }`
- **THEN** parsing or validation SHALL reject the emitted action definition as an invalid action
  inheritance clause

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

### Requirement: Component initialization materializes props, state, and rendered output
The system SHALL lower a component declaration as an executable component definition. Initializing a
component SHALL bind prop values, apply any prop defaults, evaluate state default expressions once
in declaration order, and return the rendered body expression together with the logical initial
component state.

#### Scenario: Initialization applies prop and state defaults once
- **WHEN** a module contains `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder preview:string = query } <TextInput value={preview} placeholder={placeholder} /> }` and the component is initialized without an explicit `placeholder`
- **THEN** initialization SHALL bind `placeholder` as `"Find docs"`
- **AND** SHALL materialize initial state `query="Find docs"` and `preview="Find docs"`
- **AND** SHALL return a rendered `TextInput` element whose `value` and `placeholder` are both `"Find docs"`

#### Scenario: Initialization succeeds without a state group
- **WHEN** a module contains `component <Button text:string /> = { <button>{text}</button> }` and the component is initialized with `text="Save"`
- **THEN** initialization SHALL return a rendered `button` element containing `"Save"`
- **AND** SHALL produce an empty logical component state

#### Scenario: Missing required state initializer is rejected
- **WHEN** a module contains `component <SearchBox /> = { state { query:string } <TextInput value={query} /> }` and the component is initialized
- **THEN** initialization SHALL fail because non-nullable state field `query` has no initial value

### Requirement: Component state defaults are initialization-only
The system SHALL evaluate component state default expressions only during initialization. During
dispatch, the passed-in prior state snapshot SHALL be treated as the authoritative current value of
every state field until a later change introduces declarative state-update actions.

#### Scenario: Dispatch reuses stored state instead of reevaluating defaults
- **WHEN** a module contains `component <SearchBox placeholder:string /> = { state { query:string = placeholder } <TextInput value={query} placeholder={placeholder} /> }` and a later dispatch receives a prior state snapshot whose current `query` value differs from `placeholder`
- **THEN** dispatch SHALL use the stored `query` value from the prior state snapshot as the current component state
- **AND** SHALL NOT reevaluate `query:string = placeholder`

