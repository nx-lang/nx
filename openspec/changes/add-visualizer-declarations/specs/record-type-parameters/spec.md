## ADDED Requirements

### Requirement: A field can be typed by a type parameter's property union
A generic record's field type SHALL accept `P.Property`, where `P` is one of the record's type
parameters, with any occurrence suffix. Inside the record, `P.Property` SHALL be a rigid type,
satisfied only by itself. When the record is applied with `P=A`, `P.Property` SHALL become
`A.Property`, the derived property union of `A`, and a field binding SHALL be checked against that
union, so a bare name in an unbraced attribute value SHALL resolve contextually to one of its cases.
Applying the record with an argument that has no derived property union (a union, an alias, a
primitive type or a component) SHALL be a diagnostic naming the argument and the field, unless the
argument is itself a type parameter, in which case `P.Property` SHALL become that parameter's
`.Property`. `P.Property` SHALL NOT be accepted for a component's type parameter or outside a
generic record's field types, and `P.Update` and `P.State` SHALL NOT be accepted for a type
parameter.

#### Scenario: A field names a property of its argument
- **WHEN** a file contains `type Column = { T:type of:T.Property title?:string }` and `type Contact = { name:string email:string }` and `let c = <Column T=Contact of=email />`
- **THEN** analysis SHALL accept the binding, resolving `email` to the case `Contact.Property.email`

#### Scenario: A name that is not a property is rejected
- **WHEN** `Column` and `Contact` are declared as above and a file contains `let c = <Column T=Contact of=phone />`
- **THEN** analysis SHALL reject `of=phone` because `phone` is not a case of `Contact.Property`

#### Scenario: An argument with no property union
- **WHEN** `Column` is declared as above and a file contains `type Size = small | large` and `let c = <Column T=Size of=small />`
- **THEN** analysis SHALL reject the element with a diagnostic saying `Size` has no property union for the field `of`

#### Scenario: The parameter passes through to a nested record
- **WHEN** a file contains `type Column = { T:type of:T.Property }` and `type Table = { T:type content columns:<Column T=T/>+ }` and `type Contact = { name:string }` and `let t = <Table T=Contact><Column T=Contact of=name /></Table>`
- **THEN** analysis SHALL accept the binding

#### Scenario: Only a record's own type parameter
- **WHEN** a file contains `component <List TItem:type key:TItem.Property /> = { <Panel /> }`
- **THEN** analysis SHALL reject `TItem.Property` because a component's type parameter has no property union
