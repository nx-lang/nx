# typescript-ir-runtime Specification

## Purpose
TBD - created by archiving change add-nx-ir-format. Update Purpose after archive.
## Requirements
### Requirement: TypeScript runtime loads and prepares NX IR programs
The TypeScript runtime SHALL expose APIs that accept an NX IR artifact as bytes, validate the image
header, IR schema version, runtime ABI, required features and every structural reference before
reading anything else, and return a prepared module. Preparation SHALL read the image in place
through typed-array views rather than parsing a document, SHALL decode a string only when it is
named, SHALL index the module's declarations by name, resolve every reference to the module itself,
build entrypoint tables, precompute schema validators and default evaluators, and create efficient
evaluators over the module's node table. References to other modules SHALL be resolved by the
linking step, and a prepared module whose table names only itself SHALL be usable as a program
directly. A malformed image SHALL be reported through the runtime's diagnostic result, never as a
thrown exception from the non-throwing API.

#### Scenario: Supported IR prepares successfully
- **WHEN** a caller loads a valid NX IR image whose runtime ABI matches the TypeScript runtime
- **THEN** the runtime SHALL return a prepared module
- **AND** the prepared module SHALL expose function and component entrypoint lookup by public name
- **AND** declaration lookup SHALL be by module identity and declaration name

#### Scenario: Bytes are read without copying when aligned
- **WHEN** a caller passes an `ArrayBuffer`, or a byte view whose offset is a multiple of four
- **THEN** preparation SHALL read the image through views over those bytes
- **AND** SHALL NOT copy the image

#### Scenario: A self-contained artifact is a program
- **WHEN** a caller prepares an artifact whose module table holds only its own module
- **THEN** the caller SHALL be able to evaluate its entrypoints without a linking step

#### Scenario: Unsupported runtime ABI is rejected
- **WHEN** a caller loads NX IR requiring a runtime ABI that the TypeScript runtime does not support
- **THEN** preparation SHALL fail with an actionable diagnostic
- **AND** the runtime SHALL NOT return a prepared module

#### Scenario: Unknown required feature is rejected
- **WHEN** a caller loads NX IR that declares a required feature unknown to the TypeScript runtime
- **THEN** preparation SHALL fail with an actionable diagnostic naming that feature

#### Scenario: A malformed image is a diagnostic, not an exception
- **WHEN** a caller passes bytes that are truncated, or whose cells have been altered, to the
  non-throwing preparation API
- **THEN** the result SHALL carry a diagnostic identifying what is malformed
- **AND** the API SHALL NOT throw

#### Scenario: Evaluating an unlinked module is rejected
- **WHEN** a caller evaluates an entrypoint of a prepared module whose table names another module
- **AND** the module has not been linked
- **THEN** evaluation SHALL fail with a diagnostic saying the module must be linked first

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

### Requirement: TypeScript runtime normalizes update records as patches
When the TypeScript runtime normalizes a value whose declared type is an update record — from an
IR construction expression or from host input at a boundary — it SHALL keep absent fields absent
rather than filling them from defaults or with `null`, SHALL accept `null` only for fields the
declaration marks nullable, SHALL reject unknown fields, and SHALL stamp the value with the update
record's `$type`.

#### Scenario: Evaluated update record omits unsupplied fields
- **WHEN** a prepared IR program evaluates `<User.Update email={null} />` for `type User = { name:string = "anon" email:string? }`
- **THEN** the result SHALL be `{ $type: "User.Update", email: null }`
- **AND** the result SHALL NOT have a `name` property

#### Scenario: Host input for an update record keeps absence
- **WHEN** a caller passes `{ $type: "User.Update", name: "Ada" }` where a `User.Update` prop is expected
- **THEN** normalization SHALL accept the value without reporting `email` as missing
- **AND** the normalized value SHALL NOT have an `email` property

#### Scenario: Null for a non-nullable update field is rejected
- **WHEN** a caller passes `{ $type: "User.Update", name: null }` for a `User` whose `name` is `string`
- **THEN** normalization SHALL fail with a diagnostic naming `name`

#### Scenario: A program that uses update records requires the feature
- **WHEN** a prepared IR program lists the update-record feature in its required features
- **THEN** a runtime that supports this change SHALL prepare it
- **AND** the feature name SHALL appear in the diagnostic a runtime without support reports

### Requirement: TypeScript runtime applies a component update record to host-owned state
The state-patch operation SHALL accept the component's update record value, in addition to a plain
partial state object, and SHALL apply it with the same semantics: present fields replace the
current value, absent fields keep it, a present `null` sets a nullable field to `null`, and the
result is validated against the state schema.

#### Scenario: Component update record patches state
- **WHEN** a caller applies `{ $type: "Counter.Update", count: 3 }` to current state `{ count: 1, label: "x" }` for prepared component `Counter`
- **THEN** the runtime SHALL return next state `{ count: 3, label: "x" }`

#### Scenario: An update record for a different component is rejected
- **WHEN** a caller applies a value whose `$type` is `Other.Update` to `Counter` state
- **THEN** the runtime SHALL fail with a diagnostic naming the mismatched update record

### Requirement: TypeScript runtime normalizes property union values as constant cases
When the TypeScript runtime normalizes a value whose declared type is a property union — from an
IR union-case expression or from host input at a boundary — it SHALL accept the bare string of any
case listed in the property union's declaration, SHALL reject any other value with a diagnostic
naming the property union and the offending value, and SHALL keep the value as that bare string.
A prepared program that lists the property-union feature in its required features SHALL be
accepted by a runtime that supports this change, and the feature name SHALL appear in the
diagnostic a runtime without support reports.

#### Scenario: Evaluated property union case is the bare field name
- **WHEN** a prepared IR program evaluates `User.Property.email` for `type User = { name:string email:string? }`
- **THEN** the result SHALL be the string `"email"`

#### Scenario: Host input for a property union is validated against the cases
- **WHEN** a caller passes `"nickname"` where a `User.Property` prop is expected
- **THEN** normalization SHALL fail with a diagnostic naming `User.Property` and `nickname`
- **AND** passing `"name"` SHALL be accepted unchanged

### Requirement: TypeScript runtime evaluates the update intrinsics and exports them as helpers
The TypeScript runtime SHALL evaluate the intrinsic call construct for `apply`, `merge`, `diff`,
and `changed` with the semantics the `update-records` capability specifies: present fields
replace, absent fields keep, a present `null` is carried, `merge` lets the second argument win,
`diff` lists only differing fields comparing records and lists structurally, and `changed` lists
present fields in declaration order. The result of `apply` SHALL be stamped with the target
record's `$type`, the results of `merge` and `diff` with the update record's `$type`, and the
result of `changed` SHALL be an array of bare strings. The runtime SHALL also export the same four
operations as functions a host can call on values it holds, typed so that the record and update
arguments share one record type parameter. A prepared program that lists the update-intrinsic
feature SHALL be accepted by a runtime that supports this change.

#### Scenario: Evaluated apply replaces present fields only
- **WHEN** a prepared IR program evaluates `apply(<User name="Ada" email="x@y" />, <User.Update email={null} />)` for `type User = { name:string email:string? }`
- **THEN** the result SHALL be `{ $type: "User", name: "Ada", email: null }`

#### Scenario: Evaluated diff and changed agree
- **WHEN** a prepared IR program evaluates `changed(diff(<User name="Ada" email="x@y" />, <User name="Bo" email="x@y" />))` for the same `User`
- **THEN** the result SHALL be `["name"]`

#### Scenario: Exported helper applies a host-held update
- **WHEN** a host calls the exported apply helper with `{ $type: "User", name: "Ada", email: "x@y" }` and `{ $type: "User.Update", email: null }`
- **THEN** the helper SHALL return `{ $type: "User", name: "Ada", email: null }`
- **AND** the exported merge helper called with `{ $type: "User.Update", name: "Ada" }` and `{ $type: "User.Update", name: "Bo" }` SHALL return `{ $type: "User.Update", name: "Bo" }`

#### Scenario: Exported helpers reject mismatched targets
- **WHEN** a host calls the exported apply helper with a `User` record and an update whose `$type` is `Team.Update`
- **THEN** the helper SHALL fail with a diagnostic naming both types

### Requirement: The TypeScript IR runtime is published as an npm package
The TypeScript IR runtime SHALL be publishable and published to the npm registry as
`@nx-lang/ir-runtime`, so that a browser or Node host can evaluate persisted NX IR without an NX
checkout. The published package SHALL carry its type declarations.

#### Scenario: A host installs the runtime from the registry
- **WHEN** a JavaScript project installs the published `@nx-lang/ir-runtime` package
- **THEN** it SHALL be able to prepare an NX IR program and evaluate its `root` entrypoint, with
  TypeScript types available for the exported API

#### Scenario: Runtime and SDK versions agree
- **WHEN** a host installs `@nx-lang/ir-runtime` and `@nx-lang/sdk-wasm` at the same release version
- **THEN** IR emitted by the SDK SHALL satisfy the runtime's format, schema and required-feature
  checks

### Requirement: TypeScript runtime links an entry module against prepared modules
The TypeScript runtime SHALL expose a linking step that takes a prepared entry module and a resolver
the host supplies, asks the resolver for each module in the entry's module table by identity, and
returns a linked program. Linking SHALL check that every resolved module's version equals the version
the entry recorded, SHALL fail naming the module and both versions when they differ unless the host
opts into linking across versions, SHALL fail naming the module and declaration when a referenced
declaration is absent from the resolved module, and SHALL let the host reuse one prepared module
across any number of linked programs.

#### Scenario: A snippet links against a prepared catalog
- **WHEN** a host has prepared the `drawnui` artifact once
- **AND** links a snippet artifact whose module table names `drawnui` with the same version
- **THEN** linking SHALL succeed
- **AND** evaluating the snippet's `root` SHALL construct descriptors of `drawnui`'s components

#### Scenario: A version mismatch is refused by default
- **WHEN** the snippet recorded `drawnui` version `9` and the resolver returns version `10`
- **THEN** linking SHALL fail with a diagnostic naming `drawnui`, `9` and `10`

#### Scenario: A host may link across versions
- **WHEN** the host opts into linking across versions for the same mismatch
- **AND** every declaration the snippet references exists in version `10`
- **THEN** linking SHALL succeed

#### Scenario: A missing declaration is refused
- **WHEN** the resolved `drawnui` does not declare a component the snippet references
- **THEN** linking SHALL fail with a diagnostic naming `drawnui` and the missing declaration

#### Scenario: A module the resolver cannot supply is refused
- **WHEN** the resolver returns nothing for a module in the table
- **THEN** linking SHALL fail with a diagnostic naming that module's identity

#### Scenario: One prepared module serves many programs
- **WHEN** a host links a hundred snippet artifacts against the same prepared `drawnui`
- **THEN** `drawnui` SHALL be prepared once
- **AND** each linked program SHALL evaluate independently

### Requirement: TypeScript runtime passes the conformance corpus
The TypeScript runtime's tests SHALL run every artifact in the conformance corpus and compare each
named entrypoint's canonical value against the corpus's expected result, including the artifacts
emitted without a debug section and those that link to another corpus artifact.

#### Scenario: The corpus is part of the runtime's test run
- **WHEN** the runtime's tests run
- **THEN** every corpus artifact SHALL be prepared, linked where its module table requires, and
  evaluated
- **AND** a difference from an expected result SHALL fail the run naming the program and entrypoint
