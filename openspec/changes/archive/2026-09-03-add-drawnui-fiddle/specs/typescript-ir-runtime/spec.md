## MODIFIED Requirements

### Requirement: TypeScript runtime constructs component descriptors atomically
The TypeScript runtime SHALL evaluate component descriptor expressions as atomic descriptor
construction. Descriptor construction SHALL normalize props and content through the component's
effective prop contract and SHALL return a canonical descriptor payload without evaluating the
referenced component body.

Content binding SHALL respect the declared type of the content property. When that property's type
is a list, the bound value SHALL be a list regardless of how many children were supplied, including
exactly one. When that property's type is not a list, a single child SHALL bind to the child itself.

#### Scenario: Descriptor construction does not render component body
- **WHEN** a prepared IR function returns `<Child label="Name" />`
- **AND** `Child` is a concrete NX component with an implementation body
- **THEN** evaluating the function SHALL return a descriptor with `$type` equal to `"Child"`
- **AND** it SHALL include normalized prop `label`
- **AND** it SHALL NOT evaluate `Child`'s implementation body

#### Scenario: External component descriptor applies inherited defaults
- **WHEN** a prepared IR program contains `external component <ShortTextQuestion extends Question />`
  where inherited prop `label` has default `"Untitled"`
- **AND** descriptor construction evaluates `<ShortTextQuestion />`
- **THEN** the returned descriptor SHALL include `$type` equal to `"ShortTextQuestion"`
- **AND** it SHALL include `label` equal to `"Untitled"`

#### Scenario: A single child of a list-typed content property binds as a list
- **WHEN** a component declares a content property typed as a list of components
- **AND** a descriptor for it is constructed with exactly one child
- **THEN** the content property SHALL be a list holding that one child
- **AND** descriptor construction SHALL NOT report a boundary type error

#### Scenario: Several children of a list-typed content property bind as a list
- **WHEN** a component declares a content property typed as a list of components
- **AND** a descriptor for it is constructed with more than one child
- **THEN** the content property SHALL be a list holding those children in the order supplied

#### Scenario: A single child of a non-list content property binds directly
- **WHEN** a component declares a content property whose type is not a list
- **AND** a descriptor for it is constructed with exactly one child
- **THEN** the content property SHALL be that child itself rather than a list

#### Scenario: List content binding matches the Rust interpreter
- **WHEN** the same NX program binds a single child to a list-typed content property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the
  Rust interpreter produces for it

### Requirement: TypeScript IR runtime accepts values produced by valid generated IR
The TypeScript IR runtime SHALL normalize record, union, and component values produced inside the
same prepared IR program with the same effective schema used for public boundary inputs. For valid
IR emitted from successfully analyzed source, runtime evaluation SHALL NOT fail with
`nx-ir-boundary-*` diagnostics for nullable union absence, content-property fields, single values
at list-typed fields, or record discriminators that were valid in native evaluation.

A single value at a list-typed field SHALL normalize to a list holding that one value. This is the
language's own coercion — the interpreter evaluates `xs={3.0}` and `xs={ <Item /> }` to one-element
lists, and the IR records such a value at its own type rather than as a list, leaving the coercion
to normalization.

A field whose type is spelled through a type alias SHALL normalize as the type the alias stands for,
however many aliases it is spelled through. A type alias is transparent in NX — `type Ints = int[]`
*is* a list — so the IR SHALL carry what an alias resolves to rather than the alias, and a list
reached that way SHALL take the single-value coercion and the list content binding like any other.

A record value carries a `$type` discriminator, stamped by record construction. Where such a value
is normalized into a record-typed field, the declared type of the field SHALL supply the field list,
so the carried discriminator selects nothing and SHALL be dropped rather than reported as an unknown
field. Before it is dropped it SHALL be checked, and where the declared type is a concrete record a
value carrying no discriminator SHALL be accepted, since a host writing a plain object has none to
give.

A discriminator naming a record that extends the declared one names a value of an acceptable type,
not a foreign one. Such a value SHALL be normalized against the schema of the type it names — the
declared type's field list has no room for the derived fields — and SHALL keep its own discriminator
rather than being restamped with the declared name. A discriminator naming a type that does not
extend the declared one SHALL be rejected with `nx-ir-boundary-type`. A discriminator is a name and
not an identity, so where more than one declaration of that name extends the declared type, the
runtime SHALL reject the value with `nx-ir-boundary-type` naming the ambiguity rather than choose
one of them.

An abstract record has no values of its own, so no value SHALL be normalized *as* one. Generated NX
IR SHALL record whether a record is abstract, and where the declared type of a field is an abstract
record the runtime SHALL reject a value carrying no discriminator, a value whose discriminator names
that record, and a value whose discriminator names another abstract record extending it, each with
`nx-ir-boundary-type`. This holds at the host boundary the line analysis holds for NX source, which
rejects constructing an abstract record: without it a host object would be stamped with a type name
no NX program can produce. A discriminator naming a concrete record that extends the declared one
SHALL continue to be accepted.

#### Scenario: Nullable union absence passes runtime normalization
- **WHEN** a prepared IR program evaluates a nullable union field to `null`
- **THEN** the TypeScript IR runtime SHALL accept the value for that nullable union field
- **AND** it SHALL return `null` in canonical output

#### Scenario: Invalid undeclared union case remains rejected
- **WHEN** a host boundary input or malformed IR value supplies `$type: "FlowCompletion.undefined"` for a `FlowCompletion` union that does not declare `undefined`
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
- **AND** it SHALL NOT reinterpret the undeclared case as `null`

#### Scenario: Content-populated required field does not report missing
- **WHEN** a prepared IR program constructs a component or record value whose required content property is supplied through element body content
- **THEN** the TypeScript IR runtime SHALL apply the content binding before required-field validation
- **AND** it SHALL NOT report `nx-ir-boundary-field` for the content property

#### Scenario: A single value at a list-typed property normalizes to a list of one
- **WHEN** a prepared IR program binds a value that is not a list to a property whose declared type
  is a list
- **THEN** the property SHALL hold a list containing that one value
- **AND** it SHALL NOT report `nx-ir-boundary-type` for the value not being an array

#### Scenario: A list spelled through an alias is still a list
- **WHEN** a program declares `type Ints = int[]` and binds `xs={3}` to a prop typed `Ints`, or binds
  one child to a content property typed through an alias of a list
- **THEN** the property SHALL hold a list of one
- **AND** it SHALL equal the value the Rust interpreter produces for the same program

#### Scenario: Single-value list coercion matches the Rust interpreter
- **WHEN** the same NX program binds a single value to a list-typed property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the
  Rust interpreter produces for it

#### Scenario: A constructed record binds to a record-typed property
- **WHEN** a prepared IR program constructs a record value and binds it to a property whose declared
  type is that record
- **THEN** the TypeScript IR runtime SHALL normalize the value against the declared record's fields
- **AND** it SHALL NOT report `nx-ir-boundary-field` for the carried `$type` discriminator

#### Scenario: A record-typed property default normalizes
- **WHEN** a component property of a record type declares a record-construction default
- **AND** a descriptor omits that property
- **THEN** the property SHALL hold the constructed default

#### Scenario: Record field normalization matches the Rust interpreter
- **WHEN** the same NX program binds a constructed record to a record-typed property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the
  Rust interpreter produces for it

#### Scenario: Public boundary validation still rejects malformed host input
- **WHEN** a host supplies JSON with an unknown field, a missing non-nullable required field, or an invalid union discriminator
- **THEN** the TypeScript IR runtime SHALL continue to reject the input with an `nx-ir-boundary-*` diagnostic
- **AND** it SHALL NOT treat this generated-IR parity requirement as permission to accept malformed host input

#### Scenario: A host record discriminator naming another type is rejected
- **WHEN** a host supplies `{ "$type": "Ghost", "name": "Ada" }` for a prop whose declared type is
  the record `User`
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
- **AND** it SHALL NOT return the value restamped as a `User`

#### Scenario: A host record with no discriminator is accepted
- **WHEN** a host supplies `{ "name": "Ada" }` for a prop whose declared type is the concrete record
  `User`
- **THEN** the TypeScript IR runtime SHALL normalize the value against `User`'s fields
- **AND** the normalized value SHALL carry `$type` equal to `"User"`

#### Scenario: A derived record binds to a base-typed property
- **WHEN** a prepared IR program binds a value of a record extending `Base` to a property whose
  declared type is `Base`
- **THEN** the TypeScript IR runtime SHALL normalize the value against the derived record's fields
- **AND** the normalized value SHALL keep the derived record's `$type` and its own fields

#### Scenario: A host object with no discriminator at an abstract-typed property is rejected
- **WHEN** a host supplies `{ "name": "Ada" }` for a prop whose declared type is an abstract record
  `Base`
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
  asking for a concrete type extending `Base`
- **AND** it SHALL NOT return the value stamped as a `Base`

#### Scenario: A discriminator naming an abstract record is rejected
- **WHEN** a host supplies a value whose `$type` names an abstract record, for a prop whose declared
  type is that record or a record it extends
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
  naming the abstract type
- **AND** it SHALL continue to accept a value whose `$type` names a concrete record extending the
  declared type

#### Scenario: A discriminator two declarations share is reported
- **WHEN** a value's `$type` names a record that two declarations in the program both declare, and
  both extend the declared type of the site the value reaches
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
  naming the ambiguity
- **AND** it SHALL NOT normalize the value against either declaration
