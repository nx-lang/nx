## ADDED Requirements

### Requirement: Generated executable code evaluates property references and update intrinsics
Generated executable TypeScript and JavaScript SHALL evaluate a property union case to the bare
string of the field name, so that its serialized form matches the canonical encoding, and SHALL
emit a referenced property union as a constant union of that program's generated declarations
under the same naming and value-object rules as an authored constant union. A call to `apply`,
`merge`, `diff`, or `changed` SHALL be emitted as a call into the separately supplied NX runtime
helpers rather than inlined, SHALL preserve the absent-versus-null rule, and SHALL NOT collide
with an author's declarations, which the checker already prevents from using the intrinsic names.
Generated behavior for both SHALL be validated against the interpreter on the same source.

#### Scenario: Generated JavaScript serializes a property union case as its field name
- **WHEN** NX source contains `type User = { name:string email:string? } let root() = <Box key={User.Property.email} />`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the serialized output SHALL carry `key` as the string `"email"`
- **AND** the output SHALL equal the interpreter's canonical output for the same program

#### Scenario: Generated JavaScript applies an update through the runtime
- **WHEN** NX source contains `type User = { name:string email:string? } let root() = {apply(<User name="Ada" email="x@y" />, <User.Update email={null} />)}`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the output SHALL be `{ $type: "User", name: "Ada", email: null }`
- **AND** the generated module SHALL reach the operation through the supplied runtime rather than an inlined implementation

#### Scenario: Generated JavaScript lists changed fields in declaration order
- **WHEN** NX source contains `type User = { name:string email:string? age:int? } let root() = {changed(<User.Update age={null} name="Ada" />)}`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the output SHALL be `["name", "age"]`
