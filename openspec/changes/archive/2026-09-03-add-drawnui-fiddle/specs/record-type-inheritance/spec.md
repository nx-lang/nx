## MODIFIED Requirements

### Requirement: Derived records inherit base fields and defaults
The system SHALL treat a derived record or action as having the effective field set of its entire
abstract base chain plus its own declared fields, even when that base chain is resolved through
prepared bindings that target a same-library peer file or an imported library interface. Inherited
fields SHALL participate in typed construction, field access, and default application. Duplicate
field names across the base chain and derived declaration MUST be rejected.

Reading a field of a record-typed expression SHALL produce that field's declared type, resolved
through the record's effective field set so that an inherited field reads exactly as a locally
declared one does. Each field's type SHALL be resolved in the module that declared *that field*,
which is not necessarily the module that declared the record. Reading a name that is not a field of
the record SHALL be a diagnostic that names the fields the record has.

A nullable base SHALL read its field exactly as the non-nullable base does, and SHALL produce the
field's own declared type rather than a nullable of it.

Generated NX IR SHALL carry the effective field set. A record declaration, a record construction,
and a union case SHALL each list the fields the value carries — the base chain's first, in the order
they are materialized, then the declaration's own — with each inherited field's default and declared
type resolved in the module that declared that field. A runtime reading such IR SHALL therefore
produce, for the same program, what the interpreter produces.

A value of a derived record SHALL be accepted wherever a record it extends is expected, and SHALL
keep its own type and its own fields there rather than being narrowed to the base's. Generated NX IR
SHALL carry each record's and union's base chain, nearest first, as declaration references, so that
a runtime holding a value that names its own type can tell a derived type from an unrelated one. A
value naming a type that does not extend the expected one SHALL be rejected. Where the name a value
carries is shared by more than one declaration extending the expected type, the runtime SHALL report
the ambiguity rather than choose between them. Because the base of such a site is abstract and has
no values of its own, the IR SHALL also carry whether each record is abstract, and a value offered
*as* an abstract record — carrying no type name, or carrying that record's own — SHALL be rejected
rather than stamped with a type NX itself refuses to construct.

#### Scenario: Concrete derived record uses inherited and local fields
- **WHEN** a file contains `abstract type UserBase = { name:string age:int } type User extends UserBase = { permissions:string } let makeUser() = <User name={"Ava"} age={30} permissions={"admin"} />`
- **THEN** the system SHALL accept `User` construction using both inherited fields and the local
  `permissions` field

#### Scenario: Base default applies to concrete derived record construction
- **WHEN** a file contains `abstract type UserBase = { name:string age:int = 18 } type User extends UserBase = { permissions:string }` and the interpreter constructs `User` without supplying `age`
- **THEN** the constructed `User` value SHALL include `age = 18` from the abstract base record

#### Scenario: Duplicate inherited field name is rejected
- **WHEN** a file contains `abstract type UserBase = { name:string } type User extends UserBase = { name:string permissions:string }`
- **THEN** analysis SHALL reject `User` because `name` duplicates an inherited record field

#### Scenario: Duplicate inherited peer-file field name is rejected
- **WHEN** `base.nx` in one library contains `abstract type Field = { label:string }`
- **AND** `derived.nx` in the same library contains `type TextField extends Field = { label:string placeholder:string? }`
- **THEN** analysis SHALL reject `TextField` because `label` duplicates an inherited record field

#### Scenario: Concrete derived action uses inherited and local fields
- **WHEN** a file contains `abstract action SearchAction = { query:string source:string } action SearchSubmitted extends SearchAction = { submittedAt:string } let makeAction() = <SearchSubmitted query={"docs"} source={"toolbar"} submittedAt={"now"} />`
- **THEN** the system SHALL accept `SearchSubmitted` construction using both inherited fields and
  the local `submittedAt` field

#### Scenario: Base default applies to concrete derived action construction
- **WHEN** a file contains `abstract action SearchAction = { source:string = "ui" } action SearchSubmitted extends SearchAction = { query:string }` and the interpreter constructs `SearchSubmitted` without supplying `source`
- **THEN** the constructed `SearchSubmitted` value SHALL include `source = "ui"` from the abstract
  base action

#### Scenario: Duplicate inherited action field name is rejected
- **WHEN** a file contains `abstract action SearchAction = { query:string } action SearchSubmitted extends SearchAction = { query:string submittedAt:string }`
- **THEN** analysis SHALL reject `SearchSubmitted` because `query` duplicates an inherited action
  field

#### Scenario: Inline emitted action inherits base fields and rejects duplicates
- **WHEN** a file contains `abstract action SearchAction = { query:string } component <SearchBox emits { SearchSubmitted extends SearchAction { query:string submittedAt:string } } /> = { <TextInput /> }`
- **THEN** analysis SHALL reject `SearchBox.SearchSubmitted` because `query` duplicates an
  inherited action field

#### Scenario: A declared field reads at its declared type
- **WHEN** a file contains `type User = { name:string score:int } external component <TextInput value:string /> let show(u:User) = { <TextInput value={u.score} /> }`
- **THEN** type checking SHALL type `u.score` as `int`
- **AND** it SHALL reject the property because `value` expects `string`

#### Scenario: An inherited field reads like a declared one
- **WHEN** a file contains `abstract type UserBase = { name:string } type User extends UserBase = { role:string } external component <TextInput value:string /> abstract external component <Node /> component <Row extends Node u:User /> = { <TextInput value={u.name} /> }`
- **THEN** type checking SHALL accept `u.name` as `string`

#### Scenario: A field's type resolves in the module that declared the field
- **WHEN** `model.nx` contains `export type Hue = Red | Green export type Swatch = { hue:Hue }`
- **AND** `main.nx` contains `import { Swatch } from "./model.nx" type Hue = Blue | Violet external component <Paint colour:Hue? /> abstract external component <Node /> component <Chip extends Node s:Swatch /> = { <Paint colour={s.hue} /> }`
- **THEN** type checking SHALL type `s.hue` as `model.nx`'s `Hue`, not the local one
- **AND** it SHALL reject the property with a diagnostic distinguishing the two same-named unions

#### Scenario: A nullable base reads its field
- **WHEN** a file contains `type User = { name:string } external component <TextInput value:string /> abstract external component <Node /> component <Row extends Node u:User? /> = { <TextInput value={u.name} /> }`
- **THEN** type checking SHALL accept `u.name` as `string`, not as `string?`

#### Scenario: A nullable base still rejects a name that is not a field
- **WHEN** a file contains `type User = { name:string } external component <TextInput value:string /> let show(u:User?) = { <TextInput value={u.nombre} /> }`
- **THEN** type checking SHALL reject `u.nombre` and name `name` as a field the record has

#### Scenario: Reading a name that is not a field names the fields that exist
- **WHEN** a file contains `type User = { name:string } external component <TextInput value:string /> let show(u:User) = { <TextInput value={u.nombre} /> }`
- **THEN** type checking SHALL reject `u.nombre`
- **AND** the diagnostic SHALL name `name` as a field the record has

#### Scenario: Generated IR carries an inherited field and its default
- **WHEN** a program declares `abstract type Base = { name:string = "anon" }` and
  `type User extends Base = { role:string }`, and constructs `<User role="admin" />`
- **THEN** the value a runtime produces from the emitted IR SHALL carry `name` equal to `"anon"`
- **AND** it SHALL equal the value the Rust interpreter produces for the same program

#### Scenario: A union case carries the fields of its base
- **WHEN** a union extends an abstract record and one of its cases is constructed
- **THEN** the value a runtime produces from the emitted IR SHALL carry the base's fields as well as
  the case's own
- **AND** it SHALL equal the value the Rust interpreter produces for the same program

#### Scenario: A derived record is accepted where its base is expected
- **WHEN** a program declares `abstract type Base = { name:string }` and
  `type User extends Base = { role:string }`, and passes `<User name="Ada" role="admin" />` to a
  property declared `owner:Base`
- **THEN** the value a runtime produces from the emitted IR SHALL carry `$type` of `"User"` and both
  fields
- **AND** it SHALL equal the value the Rust interpreter produces for the same program

#### Scenario: A union case is accepted where the union's base is expected
- **WHEN** a union extends an abstract record and one of its cases is passed to a property declared
  at that base's type
- **THEN** the value a runtime produces from the emitted IR SHALL carry the case's own `$type` and
  the base's fields
- **AND** it SHALL equal the value the Rust interpreter produces for the same program

#### Scenario: A value of the abstract base itself is rejected
- **WHEN** a value offered at a boundary declared at an abstract record's type names that record, or
  names no type at all
- **THEN** the runtime SHALL reject it rather than produce a value of the abstract record
- **AND** a value of a record extending it SHALL still be accepted

#### Scenario: A record that does not extend the expected one is rejected
- **WHEN** a value naming a declared record that does not extend the expected record reaches a
  boundary declared at that record's type
- **THEN** the runtime SHALL reject it and name both the expected type and the type the value claims
