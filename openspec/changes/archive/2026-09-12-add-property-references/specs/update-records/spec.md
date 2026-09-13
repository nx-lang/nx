## ADDED Requirements

### Requirement: Intrinsic functions operate on update records generically
The system SHALL provide four intrinsic functions over update records, callable with paren call
syntax and typed by rule from their arguments rather than by declared signatures: `apply`,
`merge`, `diff`, and `changed`. An intrinsic name SHALL resolve before any lexical binding, SHALL
NOT be shadowed by a parameter, `let`, loop variable, or import, and a top-level declaration that
would bind one of the four names SHALL be rejected with a diagnostic naming the intrinsic. Every
intrinsic SHALL require its arguments to agree on a single target `T`, SHALL reject an argument
whose type is not the required record or update record with a diagnostic naming the argument and
the expected type, and SHALL reject a wrong argument count. Every intrinsic SHALL preserve the
absent-versus-null rule: a present `null` is carried as `null`, and a field that is absent from
every input is absent from the output.

#### Scenario: Intrinsic names cannot be redeclared or shadowed
- **WHEN** a file contains `let apply(a:int, b:int) = {a + b}`
- **THEN** type checking SHALL reject the declaration with a diagnostic saying `apply` is an intrinsic function
- **AND** in a file containing `type User = { name:string } let f(merge:User) = {merge(<User.Update />, <User.Update />)}` the call SHALL resolve to the intrinsic rather than the parameter

#### Scenario: Wrong argument count and wrong argument types are rejected
- **WHEN** a file contains `type User = { name:string } let a = {apply(<User name="Ada" />)} let b = {apply("Ada", <User.Update />)}`
- **THEN** type checking SHALL reject `a` for taking one argument where two are required
- **AND** SHALL reject `b` because `string` is not a record

### Requirement: `apply` returns a record with the update's present fields replaced
`apply(record, update)` SHALL require `record` to be of a concrete record-shaped type `T` and
`update` to be of `T.Update`, and SHALL produce a value of type `T` in which each field present in
`update` replaces the corresponding field of `record`, including a present `null` for a nullable
field, and every other field keeps the value it had in `record`. The result SHALL be a complete
`T` value: it SHALL NOT be an update record, and it SHALL NOT re-evaluate any field default.

#### Scenario: Present fields replace and absent fields keep
- **WHEN** a file contains `type User = { name:string email:string? } let u = <User name="Ada" email="ada@example.com" /> let v = {apply(u, <User.Update email={null} />)}`
- **THEN** evaluating `v` SHALL produce a `User` whose `name` is `"Ada"` and whose `email` is `null`
- **AND** the type of `v` SHALL be `User`

#### Scenario: Empty update returns an equal record
- **WHEN** a file contains `type User = { name:string = "anon" } let v = {apply(<User name="Ada" />, <User.Update />)}`
- **THEN** evaluating `v` SHALL produce a `User` whose `name` is `"Ada"`

#### Scenario: Update for a different target is rejected
- **WHEN** a file contains `type User = { name:string } type Team = { name:string } let v = {apply(<User name="Ada" />, <Team.Update name="Core" />)}`
- **THEN** type checking SHALL reject the call because `Team.Update` is not `User.Update`

#### Scenario: Derived record accepts its own update, not its base's
- **WHEN** a file contains `abstract type Named = { name:string } type User extends Named = { email:string } let v = {apply(<User name="Ada" email="a@b" />, <Named.Update name="Bo" />)}`
- **THEN** type checking SHALL reject the call because `Named.Update` is not `User.Update`

### Requirement: `merge` composes two update records with the later winning
`merge(first, second)` SHALL require both arguments to be of the same `T.Update` and SHALL produce
a `T.Update` containing every field present in either argument, taking the value from `second`
for any field present in both, including a present `null`. Applying the merged update SHALL be
equivalent to applying `first` and then `second`.

#### Scenario: Later update wins per field and absence is preserved
- **WHEN** a file contains `type User = { name:string email:string? age:int? } let m = {merge(<User.Update name="Ada" email="x@y" />, <User.Update email={null} />)}`
- **THEN** evaluating `m` SHALL produce a `User.Update` whose `name` is `"Ada"` and whose `email` is `null`
- **AND** the value SHALL NOT contain `age`

#### Scenario: Updates for different targets cannot be merged
- **WHEN** a file contains `type User = { name:string } type Team = { name:string } let m = {merge(<User.Update />, <Team.Update />)}`
- **THEN** type checking SHALL reject the call because the two updates target different records

### Requirement: `diff` produces the update that turns one record into another
`diff(before, after)` SHALL require both arguments to be of the same concrete record-shaped type
`T` and SHALL produce a `T.Update` containing exactly the fields whose values differ between
`before` and `after`, each with its value from `after`. A field that is `null` in `after` and
not `null` in `before` SHALL be present as `null`. Field values SHALL be compared with the
language's equality, comparing records and lists structurally. Applying the result to `before`
SHALL produce a record equal to `after`.

#### Scenario: Only differing fields are present
- **WHEN** a file contains `type User = { name:string email:string? } let d = {diff(<User name="Ada" email="x@y" />, <User name="Ada" email={null} />)}`
- **THEN** evaluating `d` SHALL produce a `User.Update` containing `email` as `null`
- **AND** the value SHALL NOT contain `name`

#### Scenario: Equal records produce an empty update
- **WHEN** a file contains `type User = { name:string } let d = {diff(<User name="Ada" />, <User name="Ada" />)}`
- **THEN** evaluating `d` SHALL produce a `User.Update` with no fields

### Requirement: `changed` lists the present fields of an update record
`changed(update)` SHALL require `update` to be of some `T.Update` and SHALL produce a
`T.Property[]` containing one case per field present in `update`, including fields present as
`null`, ordered as the cases of `T.Property` are declared rather than as the update was written.

#### Scenario: Present fields are listed in declaration order
- **WHEN** a file contains `type User = { name:string email:string? age:int? } let keys = {changed(<User.Update age={null} name="Ada" />)}`
- **THEN** evaluating `keys` SHALL produce the list of `User.Property` cases `name` then `age`
- **AND** the type of `keys` SHALL be `User.Property[]`

#### Scenario: Empty update has no changed fields
- **WHEN** a file contains `type User = { name:string } let keys = {changed(<User.Update />)}`
- **THEN** evaluating `keys` SHALL produce an empty list

#### Scenario: A non-update argument is rejected
- **WHEN** a file contains `type User = { name:string } let keys = {changed(<User name="Ada" />)}`
- **THEN** type checking SHALL reject the call because `User` is not an update record
