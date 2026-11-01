## MODIFIED Requirements

### Requirement: Reading a field of an update record admits zero
Every field of `T.Update` may be absent, and an absent field SHALL read as the empty value. A
member read of a field of an update record SHALL therefore be typed as a read of an optional field:
a field declared `name:X` or `name?:X` in `T` SHALL read as `X?`, and one declared `name:X+` or
`name?:X+` SHALL read as `X*`, whether or not `T` declares the field optional. A present empty
field SHALL read as the empty value too, so a read SHALL NOT distinguish an absent field from a
cleared one; `changed` is what tells them apart. The Rust IR runtime, the TypeScript IR runtime and
generated code SHALL read an absent field and a cleared field as the same empty value.

#### Scenario: A required field of the target reads as optional
- **WHEN** a file contains `type User = { name:string email?:string }` and `let g(u:User.Update): string = { u.name }`
- **THEN** type checking SHALL reject the return value, naming `string?` and `string`
- **AND** `let g(u:User.Update): string = { u.name ?? "unchanged" }` SHALL be accepted

#### Scenario: A one-or-more field reads as zero or more
- **WHEN** a file contains `type User = { tags:string+ }` and `let t(u:User.Update) = { u.tags }`
- **THEN** the type of `u.tags` SHALL be `string*`

#### Scenario: Absent and cleared fields read as empty in every engine
- **WHEN** a file contains `type User = { name:string email?:string }` and `let e(u:User.Update) = { u.email ?? "same" }`
- **THEN** `e(<User.Update />)` and `e(<User.Update email={} />)` SHALL both evaluate to `"same"` in every engine
- **AND** `changed(<User.Update email={} />)` SHALL list `email` while `changed(<User.Update />)` SHALL be empty
