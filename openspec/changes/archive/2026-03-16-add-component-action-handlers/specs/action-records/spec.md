## ADDED Requirements

### Requirement: Inline emitted actions expose public action record names
An inline emitted action definition inside a component `emits` group SHALL introduce a public action
record whose name is `<ComponentName>.<ActionName>`. The public action record SHALL remain
record-compatible everywhere normal action records are accepted.

#### Scenario: Inline emitted action can be constructed through its public name
- **WHEN** a file contains `component <SearchBox emits { ValueChanged { value:string } } /> = { <TextInput /> }` and `let makeChange(value:string) = <SearchBox.ValueChanged value={value} />`
- **THEN** lowering SHALL accept `SearchBox.ValueChanged` in record construction position and SHALL lower the element-shaped construction as a record literal targeting `SearchBox.ValueChanged`

#### Scenario: Inline emitted action can be referenced in type positions
- **WHEN** a file contains `component <SearchBox emits { ValueChanged { value:string } } /> = { <TextInput /> }` and `let read(change:SearchBox.ValueChanged) = change.value`
- **THEN** lowering SHALL accept `SearchBox.ValueChanged` anywhere a normal action or record type name is accepted
