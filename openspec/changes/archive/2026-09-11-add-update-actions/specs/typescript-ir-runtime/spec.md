## ADDED Requirements

### Requirement: TypeScript runtime normalizes update records as patches
When the TypeScript runtime normalizes a value whose declared type is an update record — from an
IR construction expression or from host input at a boundary — it SHALL keep absent fields absent
rather than filling them from defaults or with `null`, SHALL accept `null` only for fields the
declaration marks nullable, SHALL reject unknown fields, and SHALL stamp the value with the update
record's `$type`.

#### Scenario: Evaluated update record omits unsupplied fields
- **WHEN** a prepared IR program evaluates `<User.Update email={null} />` for `type User = { name:string = "anon" email:string? }`
- **THEN** the result SHALL be `{ $type: "User.Update", email: null }`
- **AND** the result SHALL NOT have a `name` property

#### Scenario: Host input for an update record keeps absence
- **WHEN** a caller passes `{ $type: "User.Update", name: "Ada" }` where a `User.Update` prop is expected
- **THEN** normalization SHALL accept the value without reporting `email` as missing
- **AND** the normalized value SHALL NOT have an `email` property

#### Scenario: Null for a non-nullable update field is rejected
- **WHEN** a caller passes `{ $type: "User.Update", name: null }` for a `User` whose `name` is `string`
- **THEN** normalization SHALL fail with a diagnostic naming `name`

#### Scenario: A program that uses update records requires the feature
- **WHEN** a prepared IR program lists the update-record feature in its required features
- **THEN** a runtime that supports this change SHALL prepare it
- **AND** the feature name SHALL appear in the diagnostic a runtime without support reports

### Requirement: TypeScript runtime applies a component update record to host-owned state
The state-patch operation SHALL accept the component's update record value, in addition to a plain
partial state object, and SHALL apply it with the same semantics: present fields replace the
current value, absent fields keep it, a present `null` sets a nullable field to `null`, and the
result is validated against the state schema.

#### Scenario: Component update record patches state
- **WHEN** a caller applies `{ $type: "Counter.Update", count: 3 }` to current state `{ count: 1, label: "x" }` for prepared component `Counter`
- **THEN** the runtime SHALL return next state `{ count: 3, label: "x" }`

#### Scenario: An update record for a different component is rejected
- **WHEN** a caller applies a value whose `$type` is `Other.Update` to `Counter` state
- **THEN** the runtime SHALL fail with a diagnostic naming the mismatched update record
