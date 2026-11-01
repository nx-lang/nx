## MODIFIED Requirements

### Requirement: Union absence is the empty value
The system SHALL represent absence as the empty value `{}` when a discriminated-union value is
expected through a type whose occurrence admits zero — an optional property `p?:Union` or an
optional type `Union?`. The system MUST NOT synthesize an undeclared fieldless case such as
`<Union>.undefined` or `Union.undefined` to represent absence, in the Rust IR runtime, the TypeScript IR
runtime or any code generation target. A declared fieldless case SHALL continue to normalize as a
scoped union case value and SHALL remain distinct from `{}`: a `State?` that holds `State.idle`
is present. Absence of a `?`-typed union SHALL be matched with the `{}` pattern, and a match over
`Union?` whose arms cover `{}` and every case SHALL be exhaustive, as `presence-operators` defines.

#### Scenario: Omitted optional union field is empty
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } type QuestionFlow = { completion?:FlowCompletion } let root(): QuestionFlow = <QuestionFlow />`
- **THEN** type checking SHALL accept the omitted optional `completion` field
- **AND** interpretation SHALL bind `completion` to the empty value
- **AND** the canonical output SHALL NOT contain a `completion` key and SHALL NOT include `$type: "FlowCompletion.undefined"`

#### Scenario: Explicit empty optional union field is empty
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } type QuestionFlow = { completion?:FlowCompletion } let root(): QuestionFlow = <QuestionFlow completion={} />`
- **THEN** type checking SHALL accept the explicit empty `completion` field
- **AND** the value SHALL equal `<QuestionFlow />`
- **AND** the normalized output SHALL NOT include a union discriminator for `completion`

#### Scenario: Declared fieldless union case remains a case value
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } let root(): FlowCompletion? = FlowCompletion.continue`
- **THEN** interpretation SHALL normalize the result as a `FlowCompletion.continue` union case
- **AND** the result SHALL remain distinct from the empty value, so `root()?` SHALL evaluate to `true`

#### Scenario: Absence of an optional union is matched with the empty pattern
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } let label(c?:FlowCompletion): string = { if c is { {} => "pending" continue => "continue" end => c.message } }`
- **THEN** type checking SHALL accept the match as exhaustive
- **AND** `label({})` SHALL evaluate to `"pending"`
