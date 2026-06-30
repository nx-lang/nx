## ADDED Requirements

### Requirement: Explicit null values bind to nullable type references
The type checker SHALL accept an explicit `null` literal at any typed binding site whose expected
type reference is nullable. This SHALL include nullable primitive, record, enum, component,
action-record, and discriminated-union type references. The type checker SHALL continue to reject
explicit `null` when the expected type reference is not nullable.

#### Scenario: Nullable union field accepts explicit null
- **WHEN** source contains `type InitialExperience = | welcome { message:string } type ChatLinkConfig = { initialExperience:InitialExperience? } let root(): ChatLinkConfig = <ChatLinkConfig initialExperience={null} />`
- **THEN** type checking SHALL accept the `initialExperience` assignment
- **AND** interpretation SHALL normalize `initialExperience` to `null`

#### Scenario: Nullable union helper return accepts explicit null
- **WHEN** source contains `type InitialExperience = | welcome { message:string } let none(): InitialExperience? = { null }`
- **THEN** type checking SHALL accept the helper return value
- **AND** interpretation SHALL return `null`

#### Scenario: Non-nullable union target rejects explicit null
- **WHEN** source contains `type InitialExperience = | welcome { message:string } let invalid(): InitialExperience = { null }`
- **THEN** type checking SHALL reject the `null` return value
- **AND** the diagnostic SHALL identify that `null` is not compatible with non-nullable `InitialExperience`
