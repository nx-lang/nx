# component-syntax Specification

## Purpose
TBD - created by archiving change add-component-syntax. Update Purpose after archive.

## Requirements

### Requirement: Component declaration syntax
The parser SHALL support top-level component declarations introduced by the `component` keyword. A component declaration SHALL use an element-shaped signature and a block body introduced by `=`.

#### Scenario: Minimal component declaration
- **WHEN** a file contains `component <Button text:string /> = { <button>{text}</button> }`
- **THEN** the parser SHALL produce a COMPONENT_DEFINITION node with name `Button`, one PROPERTY_DEFINITION for `text`, and a COMPONENT_BODY containing the rendered element

#### Scenario: Component declaration coexists with other module items
- **WHEN** a file contains imports, a `component` declaration, and a root element
- **THEN** the parser SHALL produce a valid MODULE_DEFINITION that includes the COMPONENT_DEFINITION alongside the other top-level items

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

### Requirement: Component state group
A component body SHALL support an optional `state` group before the rendered body expression. The `state` group SHALL use record-style property definitions.

#### Scenario: Component with state group
- **WHEN** a file contains `component <SearchBox placeholder:string /> = { state { query:string } <TextInput /> }`
- **THEN** the parser SHALL produce a COMPONENT_BODY containing a STATE_GROUP with one PROPERTY_DEFINITION named `query` followed by the rendered element expression

#### Scenario: Component body without state group
- **WHEN** a file contains `component <SearchBox placeholder:string /> = { if isReady { <TextInput /> } else { <Spinner /> } }`
- **THEN** the parser SHALL produce a valid COMPONENT_DEFINITION with no STATE_GROUP and an `if` expression as the component body

### Requirement: Component declaration keywords
The parser SHALL recognize `component`, `emits`, and `state` as declaration keywords within component syntax.

#### Scenario: Component keyword starts a component declaration
- **WHEN** a file contains `component <SearchBox /> = { <TextInput /> }`
- **THEN** the parser SHALL recognize `component` as the declaration keyword for a COMPONENT_DEFINITION

#### Scenario: Emits and state keywords introduce their respective groups
- **WHEN** a file contains a component signature with `emits { Changed { value:string } }` and a body with `state { query:string }`
- **THEN** the parser SHALL recognize `emits` as the start of an EMITS_GROUP and `state` as the start of a STATE_GROUP
