# action-records Specification

## Purpose
Define the parser and HIR behavior for top-level `action` declarations and their record compatibility.

## Requirements

### Requirement: Action declaration syntax
The parser SHALL support top-level action declarations introduced by the `action` keyword. An action
declaration SHALL use the same `= { ... }` record-style property definition syntax as a normal record
declaration.

#### Scenario: Minimal action declaration
- **WHEN** a file contains `action ValueChanged = { value:string }`
- **THEN** the parser SHALL produce an ACTION_DEFINITION node with name `ValueChanged` and one
  PROPERTY_DEFINITION named `value`

#### Scenario: Action declaration coexists with other module items
- **WHEN** a file contains an `action` declaration, a `component` declaration, and a root element
- **THEN** the parser SHALL produce a valid MODULE_DEFINITION that includes the ACTION_DEFINITION
  alongside the other top-level items

### Requirement: Action records remain record-compatible
The system SHALL treat an action declaration as a record-compatible declaration everywhere normal
records are accepted, while preserving that the declared record is an action.

#### Scenario: Action declaration lowers as a record item with action identity
- **WHEN** a file contains `action SaveRequested = { value:string }`
- **THEN** HIR lowering SHALL produce a record item named `SaveRequested` that is marked as an action
  record rather than a plain record

#### Scenario: Action declaration can be used in record construction positions
- **WHEN** a file contains `action SaveRequested = { value:string }` and `let save(message:string) = <SaveRequested value={message} />`
- **THEN** lowering SHALL accept `SaveRequested` anywhere a normal record name is accepted and SHALL
  lower the element-shaped construction as a record literal targeting `SaveRequested`
