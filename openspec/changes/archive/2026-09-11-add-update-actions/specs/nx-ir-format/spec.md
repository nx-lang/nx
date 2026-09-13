## ADDED Requirements

### Requirement: NX IR declares the update records a program references
When a program references a derived update record `T.Update` — by constructing one, by naming it in
a type annotation, or by declaring a state or prop field of that type — emitted NX IR SHALL contain
a declaration for it. The declaration SHALL identify itself as an update record, SHALL name its
target `T`, and SHALL carry its own field schemas so that a runtime can normalize a value of the
type without consulting the target: every field optional, no default expressions, and nullability
copied from the target field. Update record declarations that the program does not reference SHALL
NOT be emitted, so that programs which never use a patch produce the same IR as before. A program
that references an update record SHALL list a required feature naming update-record support, so a
runtime that predates this change rejects the program rather than misreading the declaration.

#### Scenario: A referenced update record is declared with its own schema
- **WHEN** NX source contains `type User = { name:string = "anon" email:string? } let patch = <User.Update email={null} />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL contain a declaration for `User.Update` marked as an update record targeting `User`
- **AND** that declaration SHALL list fields `name` and `email`, both optional, with `email` nullable and neither carrying a default

#### Scenario: Unreferenced update records are not emitted
- **WHEN** NX source contains `type User = { name:string } let user = <User name="Ada" />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL NOT contain a declaration for `User.Update`
- **AND** the required feature list SHALL be unchanged from a program without update records

#### Scenario: Construction of an update record is encoded as record construction
- **WHEN** NX source constructs `<Counter.Update count=1 />` for a component `Counter` with state `count:int`
- **AND** NX IR is emitted for the program
- **THEN** the expression SHALL be encoded with the existing record construction operation targeting the `Counter.Update` declaration
- **AND** the emitted canonical value SHALL carry `$type` `Counter.Update` and only the `count` key
