## ADDED Requirements

### Requirement: Record inheritance declaration syntax
The parser SHALL support abstract record declarations and single-base record inheritance on `type`
record declarations using the `abstract` and `extends` keywords. A record declaration SHALL allow at
most one `extends` clause.

#### Scenario: Abstract root record declaration
- **WHEN** a file contains `abstract type UserBase = { name:string age:int }`
- **THEN** the parser SHALL produce a RECORD_DEFINITION named `UserBase` that is marked abstract and
  has no base record

#### Scenario: Abstract and concrete derived record declarations
- **WHEN** a file contains `abstract type Entity = { id:int } abstract type UserBase extends Entity = { name:string } type User extends UserBase = { permissions:string }`
- **THEN** parsing and lowering SHALL preserve `Entity` as an abstract root record, `UserBase` as an
  abstract record whose base is `Entity`, and `User` as a concrete record whose base is `UserBase`

#### Scenario: Multiple base records are rejected
- **WHEN** a file contains `type User extends Person, Auditable = { name:string }`
- **THEN** parsing or validation SHALL reject the declaration as an invalid record inheritance clause

### Requirement: Only abstract records may act as base records
The system SHALL accept `extends Base` only when `Base` resolves to an abstract record declaration.
Concrete records, non-record aliases, enums, and actions MUST NOT be usable as base records.

#### Scenario: Concrete record cannot be extended
- **WHEN** a file contains `abstract type Entity = { id:int } type User extends Entity = { name:string } type Admin extends User = { level:int }`
- **THEN** analysis SHALL reject `Admin extends User` because `User` is not abstract

#### Scenario: Non-record type cannot be extended
- **WHEN** a file contains `type Identifier = int` and `type User extends Identifier = { name:string }`
- **THEN** analysis SHALL reject `User extends Identifier` because `Identifier` does not resolve to
  an abstract record declaration

### Requirement: Derived records inherit base fields and defaults
The system SHALL treat a derived record as having the effective field set of its entire abstract base
chain plus its own declared fields. Inherited fields SHALL participate in typed construction, field
access, and default application. Duplicate field names across the base chain and derived record MUST
be rejected.

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

### Requirement: Abstract records cannot be instantiated
Abstract records SHALL be valid in type positions but MUST NOT be instantiated directly. This rule
SHALL apply to both root abstract records and abstract derived records.

#### Scenario: Root abstract record construction is rejected
- **WHEN** a file contains `abstract type UserBase = { name:string } let user = <UserBase name={"Ava"} />`
- **THEN** type checking SHALL reject the construction of `UserBase` because it is abstract

#### Scenario: Abstract derived record construction is rejected
- **WHEN** a file contains `abstract type Entity = { id:int } abstract type UserBase extends Entity = { name:string } let user = <UserBase id={1} name={"Ava"} />`
- **THEN** type checking SHALL reject the construction of `UserBase` because abstract derived records
  are not instantiable

### Requirement: Concrete derived records are substitutable for abstract parent records
The type system SHALL treat a concrete derived record value as compatible with any abstract record
type in its base chain.

#### Scenario: Concrete derived record is accepted where direct abstract parent is expected
- **WHEN** a file contains `abstract type UserBase = { name:string } type User extends UserBase = { permissions:string } let greet(user:UserBase) = user.name let value = greet(<User name={"Ava"} permissions={"admin"} />)`
- **THEN** type checking SHALL accept the call because `User` is a subtype of `UserBase`

#### Scenario: Concrete derived record is accepted where ancestor abstract parent is expected
- **WHEN** a file contains `abstract type Entity = { id:int } abstract type UserBase extends Entity = { name:string } type User extends UserBase = { permissions:string } let readId(entity:Entity) = entity.id let value = readId(<User id={1} name={"Ava"} permissions={"admin"} />)`
- **THEN** type checking SHALL accept the call because `User` is compatible with the abstract
  ancestor type `Entity`
