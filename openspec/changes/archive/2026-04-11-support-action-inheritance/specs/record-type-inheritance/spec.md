## MODIFIED Requirements

### Requirement: Record inheritance declaration syntax
The parser SHALL support abstract record declarations and single-base inheritance on record-like
declarations using the `abstract` and `extends` keywords. Top-level `type` record declarations,
top-level `action` declarations, and inline emitted action definitions SHALL allow at most one
`extends` clause. Inline emitted action definitions SHALL remain concrete and therefore SHALL NOT
accept `abstract`.

#### Scenario: Abstract root record declaration
- **WHEN** a file contains `abstract type UserBase = { name:string age:int }`
- **THEN** the parser SHALL produce a RECORD_DEFINITION named `UserBase` that is marked abstract and
  has no base record

#### Scenario: Abstract and concrete derived record declarations
- **WHEN** a file contains `abstract type Entity = { id:int } abstract type UserBase extends Entity = { name:string } type User extends UserBase = { permissions:string }`
- **THEN** parsing and lowering SHALL preserve `Entity` as an abstract root record, `UserBase` as an
  abstract record whose base is `Entity`, and `User` as a concrete record whose base is `UserBase`

#### Scenario: Abstract and concrete derived action declarations
- **WHEN** a file contains `abstract action InputAction = { source:string } abstract action SearchAction extends InputAction = { query:string } action SearchSubmitted extends SearchAction = { submittedAt:string }`
- **THEN** parsing and lowering SHALL preserve `InputAction` as an abstract root action,
  `SearchAction` as an abstract action whose base is `InputAction`, and `SearchSubmitted` as a
  concrete action whose base is `SearchAction`

#### Scenario: Multiple base records are rejected
- **WHEN** a file contains `type User extends Person, Auditable = { name:string }`
- **THEN** parsing or validation SHALL reject the declaration as an invalid record inheritance clause

#### Scenario: Multiple base actions are rejected
- **WHEN** a file contains `action SearchSubmitted extends InputAction, TrackingAction = { query:string }`
- **THEN** parsing or validation SHALL reject the declaration as an invalid action inheritance
  clause

#### Scenario: Inline emitted action declaration can extend an abstract action
- **WHEN** a file contains `abstract action InputAction = { source:string } component <SearchBox emits { ValueChanged extends InputAction { value:string } } /> = { <TextInput /> }`
- **THEN** parsing and lowering SHALL preserve `SearchBox.ValueChanged` as a concrete inline
  emitted action whose base is `InputAction`

#### Scenario: Inline emitted action rejects multiple base actions
- **WHEN** a file contains `component <SearchBox emits { ValueChanged extends InputAction, TrackingAction { value:string } } /> = { <TextInput /> }`
- **THEN** parsing or validation SHALL reject the emitted action definition as an invalid action
  inheritance clause

### Requirement: Only abstract records may act as base records
The system SHALL accept `extends Base` only when `Base` resolves to an abstract record-like
declaration of the same family through the prepared binding model of the analyzing file. That
prepared binding model SHALL include visible same-library peer declarations and exported
imported-library declarations resolved through the caller's build context, and it SHALL be able to
follow visible alias chains to the underlying declaration. Plain record declarations may extend only
abstract plain records. Action declarations may extend only abstract action declarations. Concrete
declarations, mismatched declaration kinds, non-record aliases, and enums MUST NOT be usable as
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

#### Scenario: Peer file abstract action can be extended
- **WHEN** `base.nx` in one library contains `abstract action SearchAction = { query:string }`
- **AND** `derived.nx` in the same library contains `action SearchSubmitted extends SearchAction = { submittedAt:string }`
- **THEN** analysis SHALL accept `SearchSubmitted extends SearchAction`

#### Scenario: Imported alias to an abstract action can be extended
- **WHEN** a host loads `../ui` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` from that registry
- **AND** `../ui` exports `type SearchActionBase = SearchAction` and
  `abstract action SearchAction = { query:string }`
- **AND** `app/main.nx` contains `import "../ui"` and
  `action SearchSubmitted extends SearchActionBase = { submittedAt:string }`
- **THEN** analysis SHALL accept `SearchSubmitted extends SearchActionBase`

#### Scenario: Inline emitted action can extend imported alias to an abstract action
- **WHEN** a host loads `../ui` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` from that registry
- **AND** `../ui` exports `type SearchActionBase = SearchAction` and
  `abstract action SearchAction = { query:string }`
- **AND** `app/main.nx` contains `import "../ui"` and
  `component <SearchBox emits { SearchSubmitted extends SearchActionBase { submittedAt:string } } /> = { <TextInput /> }`
- **THEN** analysis SHALL accept `SearchBox.SearchSubmitted extends SearchActionBase`

#### Scenario: Action cannot extend abstract plain record
- **WHEN** a file contains `abstract type EventBase = { source:string } action SearchSubmitted extends EventBase = { query:string }`
- **THEN** analysis SHALL reject `SearchSubmitted extends EventBase` because `EventBase` does not
  resolve to an abstract action declaration

#### Scenario: Inline emitted action cannot extend abstract plain record
- **WHEN** a file contains `abstract type EventBase = { source:string } component <SearchBox emits { SearchSubmitted extends EventBase { query:string } } /> = { <TextInput /> }`
- **THEN** analysis SHALL reject `SearchBox.SearchSubmitted extends EventBase` because `EventBase`
  does not resolve to an abstract action declaration

#### Scenario: Record cannot extend abstract action
- **WHEN** a file contains `abstract action SearchAction = { query:string } type SearchEnvelope extends SearchAction = { submittedAt:string }`
- **THEN** analysis SHALL reject `SearchEnvelope extends SearchAction` because `SearchAction` does
  not resolve to an abstract plain record declaration

### Requirement: Derived records inherit base fields and defaults
The system SHALL treat a derived record or action as having the effective field set of its entire
abstract base chain plus its own declared fields, even when that base chain is resolved through
prepared bindings that target a same-library peer file or an imported library interface. Inherited
fields SHALL participate in typed construction, field access, and default application. Duplicate
field names across the base chain and derived declaration MUST be rejected.

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

### Requirement: Abstract records cannot be instantiated
Abstract records and abstract actions SHALL be valid in type positions but MUST NOT be instantiated
directly. This rule SHALL apply to both root abstract declarations and abstract derived
declarations.

#### Scenario: Root abstract record construction is rejected
- **WHEN** a file contains `abstract type UserBase = { name:string } let user = <UserBase name={"Ava"} />`
- **THEN** type checking SHALL reject the construction of `UserBase` because it is abstract

#### Scenario: Abstract derived record construction is rejected
- **WHEN** a file contains `abstract type Entity = { id:int } abstract type UserBase extends Entity = { name:string } let user = <UserBase id={1} name={"Ava"} />`
- **THEN** type checking SHALL reject the construction of `UserBase` because abstract derived records
  are not instantiable

#### Scenario: Root abstract action construction is rejected
- **WHEN** a file contains `abstract action SearchAction = { query:string } let action = <SearchAction query={"docs"} />`
- **THEN** type checking SHALL reject the construction of `SearchAction` because it is abstract

#### Scenario: Abstract derived action construction is rejected
- **WHEN** a file contains `abstract action InputAction = { source:string } abstract action SearchAction extends InputAction = { query:string } let action = <SearchAction source={"toolbar"} query={"docs"} />`
- **THEN** type checking SHALL reject the construction of `SearchAction` because abstract derived
  actions are not instantiable

### Requirement: Concrete derived records are substitutable for abstract parent records
The type system SHALL treat a concrete derived record or action value as compatible with any
abstract declaration of the same family in its base chain.

#### Scenario: Concrete derived record is accepted where direct abstract parent is expected
- **WHEN** a file contains `abstract type UserBase = { name:string } type User extends UserBase = { permissions:string } let greet(user:UserBase) = user.name let value = greet(<User name={"Ava"} permissions={"admin"} />)`
- **THEN** type checking SHALL accept the call because `User` is a subtype of `UserBase`

#### Scenario: Concrete derived record is accepted where ancestor abstract parent is expected
- **WHEN** a file contains `abstract type Entity = { id:int } abstract type UserBase extends Entity = { name:string } type User extends UserBase = { permissions:string } let readId(entity:Entity) = entity.id let value = readId(<User id={1} name={"Ava"} permissions={"admin"} />)`
- **THEN** type checking SHALL accept the call because `User` is compatible with the abstract
  ancestor type `Entity`

#### Scenario: Concrete derived action is accepted where direct abstract parent is expected
- **WHEN** a file contains `abstract action SearchAction = { query:string } action SearchSubmitted extends SearchAction = { submittedAt:string } let read(action:SearchAction) = action.query let value = read(<SearchSubmitted query={"docs"} submittedAt={"now"} />)`
- **THEN** type checking SHALL accept the call because `SearchSubmitted` is compatible with the
  abstract action type `SearchAction`

#### Scenario: Concrete derived action is accepted where ancestor abstract parent is expected
- **WHEN** a file contains `abstract action InputAction = { source:string } abstract action SearchAction extends InputAction = { query:string } action SearchSubmitted extends SearchAction = { submittedAt:string } let readSource(action:InputAction) = action.source let value = readSource(<SearchSubmitted source={"toolbar"} query={"docs"} submittedAt={"now"} />)`
- **THEN** type checking SHALL accept the call because `SearchSubmitted` is compatible with the
  abstract ancestor action type `InputAction`
