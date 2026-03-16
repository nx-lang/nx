## MODIFIED Requirements

### Requirement: Component emits group
A component signature SHALL support an optional `emits` group after its props. The `emits` group SHALL
contain one or more emitted action entries. An emitted action entry with a record-style field list
SHALL define a new component-scoped action whose public name is `<ComponentName>.<ActionName>`, and
an emitted action entry without braces SHALL reference an existing action declaration.

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

## ADDED Requirements

### Requirement: Component invocation action handler bindings
A component invocation SHALL interpret a property named `on<ActionName>` as an emitted action
handler binding when the target component declares `ActionName` in its `emits` group. The handler
body SHALL use the same expression syntax as any other property value and SHALL have access to an
implicit `action` identifier during lowering and invocation.

#### Scenario: Call site binds a handler for a shared emitted action
- **WHEN** a file contains `action SearchSubmitted = { searchString:string }`, `component <SearchBox emits { SearchSubmitted } /> = { <TextInput /> }`, and `<SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />`
- **THEN** lowering SHALL treat `onSearchSubmitted` as a handler binding for emitted action `SearchSubmitted` rather than as an ordinary component prop

#### Scenario: Call site binds a handler for an inline emitted action
- **WHEN** a file contains `component <SearchBox emits { ValueChanged { value:string } } /> = { <TextInput /> }` and `<SearchBox onValueChanged=<TrackSearch value={action.value} /> />`
- **THEN** lowering SHALL treat `onValueChanged` as a handler binding for emitted action `ValueChanged` and SHALL bind `action` inside the handler body as a `SearchBox.ValueChanged` action value

#### Scenario: Non-matching on-prefixed properties remain ordinary props
- **WHEN** a file contains `component <SearchBox onClick:string /> = { <button /> }` and `<SearchBox onClick="primary" />`
- **THEN** lowering SHALL preserve `onClick` as an ordinary component prop because the component does not emit `Click`
