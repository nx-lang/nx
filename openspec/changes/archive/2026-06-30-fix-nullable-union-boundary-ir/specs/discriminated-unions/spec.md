## ADDED Requirements

### Requirement: Nullable union absence normalizes to null
When a discriminated-union value is expected through a nullable type reference, the system SHALL
represent absence as `null`. The system MUST NOT synthesize undeclared fieldless cases such as
`<Union>.undefined` or `Union.undefined` to represent nullable absence. Declared fieldless union
cases SHALL continue to normalize as scoped union case values.

#### Scenario: Omitted nullable union field produces null
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } type QuestionFlow = { completion:FlowCompletion? } let root(): QuestionFlow = <QuestionFlow />`
- **THEN** type checking SHALL accept the omitted nullable `completion` field
- **AND** interpretation SHALL normalize `completion` to `null`
- **AND** the normalized output SHALL NOT include `$type: "FlowCompletion.undefined"`

#### Scenario: Explicit null nullable union field produces null
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } type QuestionFlow = { completion:FlowCompletion? } let root(): QuestionFlow = <QuestionFlow completion={null} />`
- **THEN** type checking SHALL accept the explicit nullable `completion` field
- **AND** interpretation SHALL normalize `completion` to `null`
- **AND** the normalized output SHALL NOT include a union discriminator for `completion`

#### Scenario: Declared fieldless union case remains a case value
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } let root(): FlowCompletion = FlowCompletion.continue`
- **THEN** interpretation SHALL normalize the result as a `FlowCompletion.continue` union case
- **AND** the result SHALL remain distinct from `null`
