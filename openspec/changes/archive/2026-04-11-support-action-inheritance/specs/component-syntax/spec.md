## MODIFIED Requirements

### Requirement: Component emits group
A component signature SHALL support an optional `emits` group after its props. The `emits` group
SHALL contain one or more emitted action entries. An emitted action entry with a record-style field
list SHALL define a new component-scoped action whose public name is `<ComponentName>.<ActionName>`,
and it MAY declare a single optional `extends BaseAction` clause before the field list. An emitted
action entry without braces SHALL reference an existing action declaration. Inline emitted action
definitions SHALL remain concrete even when they extend an abstract action base.

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
