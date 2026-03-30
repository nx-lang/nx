## MODIFIED Requirements

### Requirement: Component emits group
A component signature SHALL support an optional `emits` group after its props. The `emits` group SHALL
contain one or more emitted action entries. An emitted action entry with a record-style field list
SHALL define a new component-scoped action, and an emitted action entry without braces SHALL reference
an existing action declaration.

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
