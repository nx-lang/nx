## Purpose
Defines what identifies a discriminated union, union case, record, or component across module
boundaries in NX, and how that identity is carried through interfaces, lowering, code generation,
and runtime values so that a nominal type means the same thing everywhere it is observed.

## ADDED Requirements

### Requirement: A nominal type is identified by where it is declared
A nominal type SHALL be identified by the module that declares it together with its stable definition
identity within that module. Its name SHALL be display information. Two nominal types SHALL be the
same type when and only when they have the same declaring origin; sharing a name SHALL NOT make two
types equal, and being reached under different names SHALL NOT make one type two.

#### Scenario: Same-named types declared in different modules are different types
- **WHEN** a module declares a union named `Fit` and imports a component whose property is typed by a
  different union also named `Fit`
- **THEN** the local union's cases SHALL NOT be accepted at that property
- **AND** the local union's qualified form SHALL NOT be accepted at that property

#### Scenario: A same-named type is rejected even when it declares an identical shape
- **WHEN** a module declares a union whose name and case names both match the union that types an
  imported component's property, and the two are declared in different modules
- **THEN** the local type SHALL NOT be accepted at that property
- **AND** the rejection SHALL NOT depend on the two declarations differing in case names or payload
  field types

#### Scenario: A union case's payload shape is not what makes it the same case
- **WHEN** a module declares a union sharing a name and case names with a foreign union, and one of
  its cases declares a payload field of a different type
- **THEN** constructing that local case SHALL NOT be accepted where the foreign union is expected
- **AND** a value SHALL NOT reach a payload field typed by the declaring module through a same-named
  local declaration

#### Scenario: A same-named record in another module is a different type
- **WHEN** a module declares a record sharing a name with the record that types an imported
  declaration's property
- **THEN** constructing the local record SHALL NOT be accepted at that property
- **AND** a value SHALL NOT reach a field typed by the declaring module through a same-named local
  declaration

#### Scenario: An inheritance chain is walked in the module that declared each link
- **WHEN** a record or component extends a base, and the module asking whether it satisfies that
  base declares an unrelated type sharing the base's name
- **THEN** each link SHALL be resolved in the module that wrote the `extends` clause
- **AND** the local declaration sharing the base's name SHALL NOT make an unrelated lineage satisfy
  it
- **AND** a lineage the asking module cannot name SHALL still satisfy the base it actually extends

#### Scenario: A resolved type is not re-selected by its spelling
- **WHEN** a value whose type resolved to a foreign declaration is used where that declaration must
  be consulted again — for its base, for a member's type, or for a common supertype — and the asking
  module declares an unrelated type sharing the spelling
- **THEN** the declaration consulted SHALL be the one the resolved type names
- **AND** the local declaration SHALL NOT supply the base, the member, or the supertype

#### Scenario: Two foreign declarations sharing a name are kept apart
- **WHEN** one module receives values typed by two different foreign declarations that share a
  display name, without naming either declaration itself
- **THEN** each value SHALL keep reaching the declaration it names, for its base and its members
- **AND** neither declaration SHALL displace the other

#### Scenario: One type reached under two names is one type
- **WHEN** a module reaches the same declared union both through a selective import and through a
  wildcard import alias
- **THEN** values obtained through either name SHALL be accepted interchangeably wherever that union
  is expected

#### Scenario: Identity survives a rename at the import boundary
- **WHEN** a module imports a nominal type under a different visible name than the one it was
  declared with
- **THEN** it SHALL be treated as the declared type
- **AND** its members or cases SHALL resolve against the declaration, not against the visible name

### Requirement: A contract resolves in the namespace of the module that wrote it
A declaration's type references SHALL be resolved in the namespace of the module that declares it,
which SHALL include the types that module imported as well as the ones it declares. A consumer SHALL
NOT re-resolve a foreign declaration's type references in its own namespace, and SHALL NOT be
required to have those types visible in order to understand the contract.

#### Scenario: A consumer reads a foreign contract without seeing its types
- **WHEN** a module imports only a component whose property is typed by a nominal type declared
  beside it
- **THEN** the property's type SHALL be understood as that declared type, including its members or
  cases
- **AND** no import of that type SHALL be required for the contract to be read

#### Scenario: A foreign contract does not bind to a same-named local type
- **WHEN** the consuming module declares a type whose name matches one used in a foreign contract
- **THEN** the foreign contract SHALL continue to denote the type its own module declared

#### Scenario: A contract naming a type its own module imported denotes that type
- **WHEN** a declaration's property is typed by a nominal type its declaring module imported rather
  than declared, and the consuming module declares a different type sharing that name
- **THEN** the property SHALL denote the type the declaring module imported
- **AND** the consuming module's same-named declaration SHALL NOT be accepted at that property

#### Scenario: A union's base and its cases' fields resolve where the union was declared
- **WHEN** a union declares an abstract base and payload fields typed by nominal names, and a
  consuming module declares unrelated types sharing those names
- **THEN** the base and the field types SHALL denote what the union's own module named
- **AND** the union SHALL still satisfy the base it extends wherever that base is expected

#### Scenario: An imported value's annotation denotes the type its own module named
- **WHEN** a module imports a value whose declared type is a nominal name, and declares a different
  type sharing that name
- **THEN** the imported value SHALL have the type its declaring module named

#### Scenario: A contract typed by an alias denotes what the alias names in its own module
- **WHEN** a declaration's property is typed by a type alias its declaring module wrote, and the
  consuming module declares a different type sharing the alias's name
- **THEN** the property SHALL denote the type the alias names in the module that wrote it
- **AND** the consuming module's same-named declaration SHALL NOT be accepted at that property
- **AND** the type the alias actually names SHALL still be accepted at that property

#### Scenario: An imported function's signature denotes the types its own module named
- **WHEN** a module imports a function whose parameter or return type is a nominal name, and
  declares a different type sharing that name
- **THEN** the parameter and return types SHALL denote what the declaring module named
- **AND** this SHALL hold whether the function is reached from a workspace peer or from a built
  library's interface

### Requirement: An inheritance chain is walked by declaration
A walk over a record's or component's inheritance chain SHALL track what it has visited by
declaration rather than by name. Two declarations sharing a name SHALL NOT be read as one link
visited twice.

#### Scenario: Extending a same-named base in another module is not a cycle
- **WHEN** a record or component extends a base declared in another module that shares its own name
- **THEN** no inheritance cycle SHALL be reported
- **AND** the fields the base declares SHALL still be inherited
- **AND** a declaration that genuinely extends itself SHALL still be reported as a cycle

### Requirement: A resolved member reference carries its origin through lowering
A reference to a union case that has been resolved during analysis SHALL carry its
declaring origin through lowering, code generation, and evaluation. Reaching the declaration SHALL
NOT depend on the type being nameable in the module that uses it.

#### Scenario: A bare member lowers without importing its type
- **WHEN** a module sets a union-typed property of an imported component using a bare member name,
  and does not import the union
- **THEN** the member SHALL lower to a reference that resolves during code generation
- **AND** evaluation SHALL produce the same value as the qualified form of that member
- **AND** no diagnostic SHALL require the union to be imported

#### Scenario: A bare case of a foreign union lowers without importing the union
- **WHEN** a module sets a union-typed property of an imported component using a bare payloadless
  case name, and does not import the union
- **THEN** the case SHALL lower to a reference that resolves during code generation
- **AND** a case that inherits fields from an abstract base SHALL be constructed with those fields and
  their defaults

#### Scenario: Generated output never carries an unresolvable reference
- **WHEN** a module is analyzed without diagnostics
- **THEN** generated code and NX IR SHALL contain no reference that failed to resolve to a
  declaration
- **AND** this SHALL hold for a case reached through the name an import alias bound, whose spelling
  has more than one segment

#### Scenario: A resolution that reaches no declaration is reported
- **WHEN** a bare name is recorded as resolved but carries no declaring origin
- **THEN** a diagnostic SHALL be reported
- **AND** the reference SHALL NOT reach evaluation as an unresolved bare name

### Requirement: Host input is checked against the declaration that expects it
A value supplied by an embedder SHALL carry a type name and no declaring origin. That name SHALL be
read as a label in the namespace of the module that declared the expectation — the property's or the
emitted action's own module — and SHALL NOT be resolved in the module that bound the handler or
constructed the element. The declaration the label reaches SHALL govern construction, and its
inheritance chain SHALL be compared by declaration rather than by spelling. A component addressed by
a host SHALL be evaluated in the module that declares it.

#### Scenario: A host action is checked against the declaration the component emits
- **WHEN** a module binds a handler to an emitted action of an imported component, and declares its
  own action sharing that action's name
- **THEN** host input SHALL be validated against the action the emitting module declares
- **AND** a value reaching a field that module typed differently SHALL be rejected
- **AND** host input matching the emitted declaration SHALL still be accepted

#### Scenario: An imported component is evaluated in the module that declares it
- **WHEN** a host initializes or evaluates a component the addressing module imported rather than
  declared
- **THEN** the component's body and its prop and state defaults SHALL be evaluated in the module
  that declares them
- **AND** host input SHALL be checked against that module's declarations, including for a handler
  bound on a component the addressing module did not declare

#### Scenario: A host label whose lineage only shares a name with the expectation is rejected
- **WHEN** a host supplies a record whose base merely shares a name with the base a component's
  property expects
- **THEN** the value SHALL NOT be accepted at that property
- **AND** a record extending the expected base SHALL still be accepted, including one declared in a
  module the expecting module imported it from

### Requirement: Diagnostics distinguish same-named nominal types
When a diagnostic reports a mismatch between two nominal types that share a name, it SHALL identify
which module declares each of them.

#### Scenario: A same-name mismatch names the declaring modules
- **WHEN** a value of a local `Fit` is supplied where an imported `Fit` is expected
- **THEN** the diagnostic SHALL distinguish the two types by their declaring modules
- **AND** it SHALL NOT read as though a type were incompatible with itself

#### Scenario: A qualified case of a same-named union is distinguished from the expectation
- **WHEN** a case of a local union is supplied where a same-named imported union is expected
- **THEN** the diagnostic SHALL name the declaring module of both the expected union and the
  supplied case
- **AND** it SHALL NOT read as though a case of the expected union were being rejected

#### Scenario: A message naming one union says which one when another shares its name
- **WHEN** a bare or qualified name is rejected against a union, and a different union sharing that
  union's display name is visible where the name was written
- **THEN** the diagnostic SHALL name the declaring module of the union it is describing
- **AND** it SHALL leave the name unqualified when no other union there shares it
