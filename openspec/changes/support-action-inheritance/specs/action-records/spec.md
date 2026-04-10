## MODIFIED Requirements

### Requirement: Action declaration syntax
The parser SHALL support top-level action declarations introduced by the `action` keyword,
including optional `abstract` modifiers and single-base inheritance through `extends`. An action
declaration SHALL use the same `= { ... }` record-style property definition syntax as a normal
record declaration and SHALL allow at most one `extends` clause.

#### Scenario: Minimal action declaration
- **WHEN** a file contains `action ValueChanged = { value:string }`
- **THEN** the parser SHALL produce an ACTION_DEFINITION node with name `ValueChanged` and one
  PROPERTY_DEFINITION named `value`

#### Scenario: Abstract root action declaration
- **WHEN** a file contains `abstract action InputAction = { source:string }`
- **THEN** the parser SHALL produce an ACTION_DEFINITION node named `InputAction` that is marked
  abstract and has no base action

#### Scenario: Abstract and concrete derived action declarations
- **WHEN** a file contains `abstract action InputAction = { source:string } abstract action SearchAction extends InputAction = { query:string } action SearchSubmitted extends SearchAction = { submittedAt:string }`
- **THEN** parsing and lowering SHALL preserve `InputAction` as an abstract root action,
  `SearchAction` as an abstract action whose base is `InputAction`, and `SearchSubmitted` as a
  concrete action whose base is `SearchAction`

#### Scenario: Multiple base actions are rejected
- **WHEN** a file contains `action SearchSubmitted extends InputAction, TrackingAction = { query:string }`
- **THEN** parsing or validation SHALL reject the declaration as an invalid action inheritance
  clause

#### Scenario: Action declaration coexists with other module items
- **WHEN** a file contains an `action` declaration, a `component` declaration, and a root element
- **THEN** the parser SHALL produce a valid MODULE_DEFINITION that includes the ACTION_DEFINITION
  alongside the other top-level items

### Requirement: Action records remain record-compatible
The system SHALL treat an action declaration, including abstract and derived action declarations, as
a record-compatible declaration everywhere normal records are accepted, while preserving that the
declared record is an action. Derived actions SHALL inherit base action fields and SHALL remain
compatible with abstract parent actions in type positions.

#### Scenario: Action declaration lowers as a record item with action identity
- **WHEN** a file contains `action SaveRequested = { value:string }`
- **THEN** HIR lowering SHALL produce a record item named `SaveRequested` that is marked as an
  action record rather than a plain record

#### Scenario: Derived action lowers as an action record with preserved ancestry
- **WHEN** a file contains `abstract action InputAction = { source:string } action ValueChanged extends InputAction = { value:string }`
- **THEN** HIR lowering SHALL produce `InputAction` and `ValueChanged` as record items marked as
  action records
- **AND** `ValueChanged` SHALL preserve `InputAction` as its abstract base action rather than
  lowering as a plain record

#### Scenario: Derived action can be used in record construction positions
- **WHEN** a file contains `abstract action InputAction = { source:string } action ValueChanged extends InputAction = { value:string } let change = <ValueChanged source={"keyboard"} value={"docs"} />`
- **THEN** lowering and type checking SHALL accept `ValueChanged` anywhere a normal record name is
  accepted
- **AND** SHALL lower the element-shaped construction as a record literal targeting `ValueChanged`

#### Scenario: Derived action is accepted where abstract parent action is expected
- **WHEN** a file contains `abstract action InputAction = { source:string } action ValueChanged extends InputAction = { value:string } let read(action:InputAction) = action.source let value = read(<ValueChanged source={"keyboard"} value={"docs"} />)`
- **THEN** type checking SHALL accept the call because `ValueChanged` is compatible with the
  abstract action type `InputAction`

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
