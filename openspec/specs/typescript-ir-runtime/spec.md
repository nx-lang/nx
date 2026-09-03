# typescript-ir-runtime Specification

## Purpose
TBD - created by archiving change add-nx-ir-format. Update Purpose after archive.
## Requirements
### Requirement: TypeScript runtime loads and prepares NX IR programs
The TypeScript runtime SHALL expose APIs that accept NX IR JSON or parsed IR objects, validate the
format identifier, IR schema version, runtime ABI, required features, and structural references,
and return a prepared program object. Preparation SHALL resolve references, build entrypoint
tables, precompute schema validators/default evaluators, validate nominal type references, and
create efficient evaluators for the supported eager expression set.

#### Scenario: Supported IR prepares successfully
- **WHEN** a caller loads a valid NX IR JSON document whose runtime ABI matches the TypeScript
  runtime
- **THEN** the runtime SHALL return a prepared program object
- **AND** the prepared program SHALL expose function and component entrypoint lookup by public name
- **AND** semantic declaration lookup SHALL use module-qualified declaration references

#### Scenario: Unsupported runtime ABI is rejected
- **WHEN** a caller loads NX IR requiring a runtime ABI that the TypeScript runtime does not support
- **THEN** preparation SHALL fail with an actionable diagnostic
- **AND** the runtime SHALL NOT return a prepared program

#### Scenario: Unknown required feature is rejected
- **WHEN** a caller loads NX IR that declares a required feature unknown to the TypeScript runtime
- **THEN** preparation SHALL fail with an actionable diagnostic naming that feature

### Requirement: TypeScript runtime evaluates IR function entrypoints
The TypeScript runtime SHALL evaluate public IR function entrypoints using eager NX semantics for
the supported expression set. Function evaluation SHALL bind normalized arguments, execute
module-qualified calls and references through the prepared program, enforce resource or recursion
limits where exposed, and return canonical JSON-compatible NX values.

#### Scenario: Root function evaluates through IR
- **WHEN** a prepared IR program contains `let root() = { 1 + 2 }`
- **AND** a caller evaluates function entrypoint `root`
- **THEN** the runtime SHALL return `3`

#### Scenario: Cross-module function call evaluates through IR
- **WHEN** a prepared IR program contains a root function that calls an imported library function
- **AND** a caller evaluates the root function
- **THEN** the runtime SHALL resolve the call through the module-qualified IR reference
- **AND** it SHALL return the same value as native interpreter evaluation for the same program

#### Scenario: Match expression evaluates through IR
- **WHEN** a prepared IR function uses a match-style `if is` expression over a union value
- **AND** a caller evaluates that function
- **THEN** the runtime SHALL select the first matching arm in authored order
- **AND** it SHALL evaluate the else branch only when no arm matches

#### Scenario: Out-of-bounds array index is rejected
- **WHEN** a prepared IR function evaluates an array index expression whose index is negative or
  greater than or equal to the array length
- **THEN** the runtime SHALL fail with a diagnostic identifying the out-of-bounds index
- **AND** it SHALL NOT return `null` for the missing array element

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

### Requirement: TypeScript runtime initializes and evaluates components with host-owned state
The TypeScript runtime SHALL expose component APIs that initialize a concrete component from props,
evaluate a concrete component from props and current state, validate/normalize a complete state
object, and apply a host-provided state patch to produce a normalized next state. These operations
SHALL be pure with respect to runtime-held component instances and SHALL NOT require hidden mutable
component state.

#### Scenario: Component initialization materializes initial state
- **WHEN** a prepared IR program contains `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** a caller initializes `SearchBox` without props
- **THEN** the runtime SHALL materialize prop `placeholder` as `"Find docs"`
- **AND** it SHALL return state with `query` equal to `"Find docs"`
- **AND** it SHALL return rendered output whose `TextInput` value is `"Find docs"`

#### Scenario: Explicit state controls evaluation
- **WHEN** a caller evaluates prepared component `SearchBox` with state `{ query: "docs" }`
- **THEN** the runtime SHALL render the component body with `query` equal to `"docs"`
- **AND** it SHALL NOT replace the supplied state field with the default expression value

#### Scenario: Host-owned state patch is validated
- **WHEN** a caller applies state patch `{ query: "guides" }` to current state
  `{ query: "docs" }` for prepared component `SearchBox`
- **THEN** the runtime SHALL return normalized next state `{ query: "guides" }`
- **AND** it SHALL validate the patched state against `SearchBox`'s declared state schema

#### Scenario: Invalid state patch is rejected
- **WHEN** a caller applies state patch `{ query: 123 }` to a component whose `query` state field
  is `string`
- **THEN** the runtime SHALL fail with a diagnostic identifying the invalid state field
- **AND** it SHALL NOT return a partially updated state object

### Requirement: TypeScript runtime validates JSON boundary values against IR schemas
The TypeScript runtime SHALL use IR schema metadata to normalize and validate public boundary
values, including function arguments, component props, component state, state patches, enum values,
records, arrays, nullable values, and union cases. Missing required fields, unknown fields, invalid
enum members, and type mismatches SHALL produce diagnostics consistent with existing NX runtime
behavior.

#### Scenario: Missing required prop is rejected
- **WHEN** a caller initializes a prepared component requiring prop `label:string`
- **AND** the caller omits `label`
- **THEN** the runtime SHALL fail with a diagnostic identifying the missing prop

#### Scenario: Unknown state field is rejected
- **WHEN** a caller evaluates a component with state object `{ query: "docs", extra: true }`
- **AND** the component state schema does not declare `extra`
- **THEN** the runtime SHALL fail with a diagnostic identifying the unknown state field

#### Scenario: Unknown enum member is rejected
- **WHEN** a caller supplies string `"blue"` for a prop whose declared type is enum `ThemeMode`
- **AND** `ThemeMode` does not declare member `blue`
- **THEN** the runtime SHALL fail with a diagnostic identifying the invalid enum member

#### Scenario: Same-named nominal declarations do not collide
- **WHEN** a prepared IR program contains two modules that each declare a record named `User`
- **AND** an exported function parameter type references one of those records by module-qualified
  nominal type reference
- **THEN** the runtime SHALL normalize the argument using the referenced declaration
- **AND** it SHALL NOT select the other `User` declaration by bare name

#### Scenario: Non-entrypoint declarations are not public host API targets
- **WHEN** a prepared IR program contains a function declaration not listed in function
  entrypoints
- **AND** a caller evaluates that function by name through the public runtime API
- **THEN** the runtime SHALL fail with a missing entrypoint diagnostic
- **AND** it SHALL NOT fall back to global declaration-name lookup

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

### Requirement: TypeScript IR runtime behavior is validated against existing NX semantics
The implementation SHALL include automated tests that emit NX IR from source or program artifacts,
execute the IR through the TypeScript runtime, and compare results against native interpreter
evaluation for the supported non-reactive subset. Component tests SHALL cover descriptor
construction, initialization, explicit state evaluation, state patch validation, and conditional
content based on state.

#### Scenario: Function parity test compares interpreter and IR runtime
- **WHEN** a supported NX program uses primitives, arithmetic, conditionals, match expressions,
  arrays, loops, records, unions, enums, member access, and function calls
- **THEN** automated tests SHALL verify that TypeScript IR runtime output matches native
  interpreter output for the same program

#### Scenario: Component parity test compares interpreter and IR runtime
- **WHEN** a supported NX component uses props, state defaults, conditional content, child
  component descriptors, and explicit state evaluation
- **THEN** automated tests SHALL verify that TypeScript IR runtime rendered output matches native
  interpreter rendered output for the same props and state
