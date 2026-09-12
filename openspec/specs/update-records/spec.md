# update-records Specification

## Purpose
Defines the derived `T.Update` patch record every record-shaped declaration and every component
state block gets: its shape, the absent-versus-null rule, how it is constructed and validated, how
the bare `Update` tag resolves inside a component, and how it is encoded on the wire.
## Requirements
### Requirement: Every record-shaped declaration has a derived update record
For every record-shaped declaration `T` in scope — a `type` record, an `action` record, an inline
emitted action, and a component's `state` block — the system SHALL make a derived record named
`T.Update` available without any declaration by the author. `T.Update` SHALL have exactly the
effective fields of `T` (including inherited fields), each with the same declared type as in `T`,
and every field SHALL be optional in the sense that it may be absent. A component's `T.Update`
SHALL be derived from its effective state fields and SHALL NOT include its props. An update record
SHALL NOT be an action record, SHALL NOT be a plain record, SHALL NOT be abstract, and SHALL NOT be
usable as the base of an `extends` clause. An element or record-literal tag spelled `X.Update` that
does not resolve to a derived update record SHALL be a diagnostic naming `X`, rather than being
treated as an unknown host element, since the `.Update` suffix always names a derived record.

#### Scenario: Update record of a plain record has the same fields
- **WHEN** a file contains `type User = { name:string email:string? } let patch = <User.Update name="Ada" />`
- **THEN** type checking SHALL accept `User.Update` as a record type with fields `name:string` and `email:string?`
- **AND** the value of `patch` SHALL be of type `User.Update`

#### Scenario: Update record of a component is derived from its state
- **WHEN** a file contains `component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }` and `let reset(): Counter.Update = <Counter.Update count=0 />`
- **THEN** type checking SHALL accept `Counter.Update` with the single field `count:int`
- **AND** type checking SHALL reject `<Counter.Update step=2 />` because `step` is a prop, not a state field

#### Scenario: Update record includes inherited fields
- **WHEN** a file contains `abstract type Named = { name:string } type User extends Named = { email:string }` and `let patch = <User.Update name="Ada" email="ada@example.com" />`
- **THEN** type checking SHALL accept both `name` and `email` on `User.Update`

#### Scenario: Update record is not an action and cannot be extended
- **WHEN** a file contains `type User = { name:string } action Rename extends User.Update = { }`
- **THEN** type checking SHALL reject the `extends` clause because `User.Update` is a derived update record

#### Scenario: Update record of a nested update record does not exist
- **WHEN** a file contains `type User = { name:string } let patch = <User.Update.Update name="Ada" />`
- **THEN** type checking SHALL reject `User.Update.Update` as an unknown type

#### Scenario: Update tag for an unknown target is rejected
- **WHEN** a file contains `let icon = <Icons.Update size=3 />` and nothing named `Icons` is declared
- **THEN** type checking SHALL reject the tag with a diagnostic saying `Icons` is not a record, action, or component with state

#### Scenario: A component without state has no update record
- **WHEN** a file contains `component <Form /> = { <Button onTapped=<Update /> /> }`
- **THEN** type checking SHALL report exactly one diagnostic, saying `Form` declares no state and so has no update record

### Requirement: An absent field means unchanged and null means set to null
Constructing an update record SHALL NOT apply the target's field defaults and SHALL NOT require any
field. A field that is not supplied SHALL be absent from the resulting value, which is distinct from
that field being `null`. Supplying `null` for a field SHALL be accepted only when the target field's
declared type is nullable, and SHALL produce a present field whose value is `null`. Supplying a
field the target does not declare SHALL be rejected, and supplying a value of the wrong type SHALL
be rejected, both at type checking and again at runtime construction.

#### Scenario: Unsupplied fields are absent rather than defaulted
- **WHEN** a file contains `type User = { name:string = "anon" email:string? } let patch = <User.Update email="ada@example.com" />`
- **THEN** evaluating `patch` SHALL produce a `User.Update` value whose `email` is `"ada@example.com"`
- **AND** the value SHALL NOT contain a `name` field, and in particular SHALL NOT contain `name` set to `"anon"` or to `null`

#### Scenario: Null is accepted for a nullable field and preserved as present
- **WHEN** a file contains `type User = { name:string email:string? } let patch = <User.Update email={null} />`
- **THEN** evaluating `patch` SHALL produce a value that contains `email` with the value `null`

#### Scenario: Null is rejected for a non-nullable field
- **WHEN** a file contains `type User = { name:string } let patch = <User.Update name={null} />`
- **THEN** type checking SHALL reject the construction because `name` is not nullable

#### Scenario: Unknown and mistyped fields are rejected
- **WHEN** a file contains `type User = { name:string } let a = <User.Update nickname="A" /> let b = <User.Update name=3 />`
- **THEN** type checking SHALL reject `nickname` as an unknown field of `User.Update`
- **AND** type checking SHALL reject `name=3` as a type mismatch against `string`

#### Scenario: Runtime construction enforces the same rules when static analysis is bypassed
- **WHEN** runtime evaluation is asked to construct `User.Update` from host-supplied fields that include an unknown field or `null` for a non-nullable field
- **THEN** construction SHALL fail with a runtime error naming the offending field
- **AND** SHALL NOT produce a value with the field silently dropped or coerced

#### Scenario: Empty update record is valid
- **WHEN** a file contains `type User = { name:string } let none = <User.Update />`
- **THEN** evaluating `none` SHALL produce a `User.Update` value with no fields

### Requirement: The bare `Update` tag resolves to the enclosing component's update record
Inside a component body — its state defaults, its rendered body expression, and any handler body
bound within it — an element or record-literal tag spelled exactly `Update` SHALL denote that
component's own `<Component>.Update` record, regardless of any other declaration named `Update`.
Outside a component body a bare `Update` tag SHALL be a diagnostic that directs the author to the
qualified `<Type>.Update` spelling. A resolved bare `Update` SHALL be indistinguishable from the
qualified form in every downstream artifact.

#### Scenario: Bare Update in a handler body resolves to the component
- **WHEN** a file contains `external component <Button emits { Tapped } />` and `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`
- **THEN** type checking SHALL resolve `Update` inside the handler as `Counter.Update`
- **AND** SHALL check `count={count + 1}` against the state field `count:int`

#### Scenario: Bare Update in a state default resolves to the component
- **WHEN** a file contains `component <Editor /> = { state { pending:Editor.Update = <Update /> } <Panel /> }`
- **THEN** type checking SHALL accept the default as an `Editor.Update` value

#### Scenario: Bare Update outside a component is rejected with the qualified spelling
- **WHEN** a file contains `type User = { name:string } let patch = <Update name="Ada" />`
- **THEN** type checking SHALL reject the construction
- **AND** the diagnostic SHALL say that a bare `Update` needs an enclosing component and SHALL show the `<Type>.Update` form

#### Scenario: Bare Update takes precedence over a same-named declaration inside a component
- **WHEN** a file contains `type Update = { note:string }` and `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count=1 /> /> }`
- **THEN** inside the `Counter` body `Update` SHALL denote `Counter.Update`
- **AND** the handler SHALL type check against the `count` state field rather than against `note`

### Requirement: Update records have a canonical wire encoding that preserves absence
The canonical value encoding of an update record SHALL carry its `$type` discriminator as
`<Target>.Update`, SHALL include a key only for each present field, SHALL encode a present `null`
field as `null`, and SHALL NOT emit keys for absent fields. Decoding host input that names an
update record type SHALL preserve the same distinction: a missing key SHALL decode to an absent
field, and SHALL NOT be filled from the target's defaults or with `null`. This SHALL hold for both
the JSON and MessagePack forms of the canonical encoding.

#### Scenario: JSON encoding omits absent fields
- **WHEN** `<User.Update email={null} />` is evaluated for a `User` with fields `name:string email:string?` and the result is encoded as JSON
- **THEN** the JSON SHALL be an object with exactly the keys `$type` and `email`
- **AND** `$type` SHALL be `"User.Update"` and `email` SHALL be `null`

#### Scenario: Host input with a missing key decodes as absent
- **WHEN** a host supplies the JSON object `{ "$type": "User.Update", "name": "Ada" }` where an update record is expected
- **THEN** the decoded value SHALL contain `name` and SHALL NOT contain `email`
- **AND** the runtime SHALL NOT report `email` as a missing required field

#### Scenario: MessagePack round-trip preserves absence
- **WHEN** an update record with one present field and one absent field is encoded as MessagePack and decoded again
- **THEN** the decoded value SHALL contain only the present field

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
