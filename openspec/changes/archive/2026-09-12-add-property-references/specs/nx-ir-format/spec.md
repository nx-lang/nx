## ADDED Requirements

### Requirement: NX IR declares the property unions a program references
When a program references a derived property union `T.Property` — by naming one of its cases, by
naming it in a type annotation, or by declaring a state or prop field of that type — emitted NX IR
SHALL contain a union declaration for it. The declaration SHALL identify itself as a property
union, SHALL name its target `T`, and SHALL list its cases as constant cases in `T.Property`'s
case order, so that a runtime can validate a bare-string value of the type without consulting the
target. Property union declarations that the program does not reference SHALL NOT be emitted. A
program that references a property union SHALL list a required feature naming property-union
support, so a runtime that predates this change rejects the program rather than misreading the
declaration. A case of a property union SHALL be encoded with the existing union-case construct
as a constant case.

#### Scenario: A referenced property union is declared with its cases
- **WHEN** NX source contains `abstract type Named = { name:string } type User extends Named = { email:string? } let key = {User.Property.email}`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL contain a union declaration for `User.Property` marked as a property union targeting `User`
- **AND** that declaration SHALL list constant cases `name` then `email` and no base

#### Scenario: Unreferenced property unions are not emitted
- **WHEN** NX source contains `type User = { name:string } let user = <User name="Ada" />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL NOT contain a declaration for `User.Property`
- **AND** the required feature list SHALL be unchanged from a program without property unions

#### Scenario: A property union case is encoded as a constant union case
- **WHEN** NX source contains `type User = { name:string } let key = {User.Property.name}`
- **AND** NX IR is emitted for the program
- **THEN** the expression SHALL be encoded with the existing union-case construct referencing the `User.Property` declaration and case `name`
- **AND** the emitted canonical value SHALL be the bare string `"name"`

## MODIFIED Requirements

### Requirement: NX IR encodes the supported eager expression set
NX IR SHALL encode the supported non-reactive expression forms needed for eager evaluation,
including literals, local slot references, top-level references, unary and binary operations,
function calls, intrinsic calls, `if`, match-style `if is` forms, `let`, blocks, arrays, loops,
index access, member access, record literals, union cases, intrinsic elements, and component
descriptors. There SHALL be one union-case construct covering both constant and payload cases
rather than separate constructs for enum members and union cases, and that construct SHALL mark a
constant case as constant in expression position as well as in the declaration, so a runtime
produces the bare case name without consulting the declaration. A call to one of the update
intrinsics (`apply`, `merge`, `diff`, `changed`) SHALL be encoded as an intrinsic call construct
that names the intrinsic and carries its argument expressions, distinct from a call to a declared
function, and a program that contains one SHALL list a required feature naming update-intrinsic
support. A `changed` call SHALL also carry the declared field order of the target record, so a
runtime orders the result without consulting the declaration. Unsupported executable constructs
SHALL be reported as IR build diagnostics.

#### Scenario: Match expressions are preserved
- **WHEN** NX source contains a match-style `if value is { ... }` expression accepted by static
  analysis
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL contain an operation that preserves the scrutinee, ordered arms, patterns,
  and optional else branch

#### Scenario: Loop expressions are preserved
- **WHEN** NX source contains `for item, index in items { item }`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL preserve the iterable expression, item slot, optional index slot, and loop
  body expression

#### Scenario: Index expressions preserve bounds-sensitive semantics
- **WHEN** NX IR contains an index expression over an array value
- **THEN** supported runtimes SHALL require an integer index
- **AND** an index outside the array bounds SHALL fail with a runtime diagnostic rather than
  evaluating to `null`

#### Scenario: Constant and payload cases use one IR construct
- **WHEN** NX source contains `type Shape = circle | square { n:int }` and constructs both cases
- **AND** NX IR is emitted for the program
- **THEN** both constructions SHALL be encoded by the same union-case construct
- **AND** the construct SHALL carry enough information for a runtime to produce the bare string for
  `circle` and the `$type` map for `square`

#### Scenario: A constant case expression is marked constant
- **WHEN** NX source contains `type Mode = light | dark let m = {Mode.dark}`
- **AND** NX IR is emitted for the program
- **THEN** the union-case expression for `Mode.dark` SHALL be marked as a constant case
- **AND** a runtime evaluating it SHALL produce the bare string `"dark"` rather than a `$type` map

#### Scenario: An intrinsic call is encoded distinctly from a function call
- **WHEN** NX source contains `type User = { name:string } let v = {apply(<User name="Ada" />, <User.Update name="Bo" />)}`
- **AND** NX IR is emitted for the program
- **THEN** the call SHALL be encoded with the intrinsic call construct naming `apply` and carrying two argument expressions
- **AND** the required feature list SHALL name update-intrinsic support
- **AND** for `let c = {changed(<User.Update name="Bo" />)}` the construct SHALL name `changed` and carry the field order `name`

#### Scenario: Unsupported action handler is rejected in v1
- **WHEN** NX source requires an action-handler value that cannot be represented by the v1 IR
  feature set
- **THEN** IR emission SHALL fail with a diagnostic identifying the unsupported construct
- **AND** the emitted IR SHALL NOT silently drop the handler
