## MODIFIED Requirements

### Requirement: Only abstract records may act as base records
The system SHALL accept `extends Base` only when `Base` resolves to an abstract record declaration
through the prepared binding model of the analyzing file. That prepared binding model SHALL include
visible same-library peer declarations and exported imported-library declarations resolved through
the caller's build context, and it SHALL be able to follow visible alias chains to the underlying
record definition. Concrete records, non-record aliases, enums, and actions MUST NOT be usable as
base records.

#### Scenario: Concrete record cannot be extended
- **WHEN** a file contains `abstract type Entity = { id:int } type User extends Entity = { name:string } type Admin extends User = { level:int }`
- **THEN** analysis SHALL reject `Admin extends User` because `User` is not abstract

#### Scenario: Non-record type cannot be extended
- **WHEN** a file contains `type Identifier = int` and `type User extends Identifier = { name:string }`
- **THEN** analysis SHALL reject `User extends Identifier` because `Identifier` does not resolve to
  an abstract record declaration

#### Scenario: Peer file abstract record can be extended
- **WHEN** `base.nx` in one library contains `abstract type Field = { label:string }`
- **AND** `derived.nx` in the same library contains `type TextField extends Field = { placeholder:string? }`
- **THEN** analysis SHALL accept `TextField extends Field`

#### Scenario: Imported abstract record can be extended
- **WHEN** a host loads `../ui` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` from that registry
- **AND** analyzes `app/main.nx` containing `import "../ui"` and
  `type TextField extends Field = { placeholder:string? }`
- **AND** `../ui` exports `abstract type Field = { label:string }`
- **THEN** analysis SHALL accept `TextField extends Field`

#### Scenario: Imported alias to an abstract record can be extended
- **WHEN** a host loads `../ui` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` from that registry
- **AND** `../ui` exports `type FieldBase = Field` and `abstract type Field = { label:string }`
- **AND** `app/main.nx` contains `import "../ui"` and
  `type TextField extends FieldBase = { placeholder:string? }`
- **THEN** analysis SHALL accept `TextField extends FieldBase`

### Requirement: Derived records inherit base fields and defaults
The system SHALL treat a derived record as having the effective field set of its entire abstract
base chain plus its own declared fields, even when that base chain is resolved through prepared
bindings that target a same-library peer file or an imported library interface. Inherited fields
SHALL participate in typed construction, field access, and default application. Duplicate field
names across the base chain and derived record MUST be rejected.

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
