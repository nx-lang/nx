## ADDED Requirements

### Requirement: Generated executable code evaluates the presence operators
Generated executable TypeScript and JavaScript SHALL evaluate the presence test `x?`, the step
`x?.m` and the fallback `x ?? y` to the values `presence-operators` defines,
so that the serialized result of a generated entrypoint equals the interpreter's canonical output
for the same source. The generated expression for each operator is the emitter's choice; what is
observable SHALL hold: `x?` is `true` exactly when `x` holds an item; `x?.m` is the empty value when
`x` is empty and `x.m` otherwise, with `x` evaluated once; `x ?? y` is `x` when `x` holds an item
and otherwise `y`, with `y` evaluated only then. Executable source codegen does not support match
expressions, so a program that matches the `{}` pattern SHALL be refused with a diagnostic rather
than emitted; the NX IR runtime evaluates that pattern. Generated code SHALL NOT introduce a `null` NX value to stand for absence: an empty optional
field of a record or element the generated code constructs SHALL be omitted from the object, and an
empty `*` value SHALL be an empty array, as `occurrence-types` defines the canonical encoding.

#### Scenario: Generated JavaScript tests presence
- **WHEN** NX source contains `type Book = { title:string author?:Person } let root() = { <Box has={<Book title="A" />.author?} /> }`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the serialized output SHALL carry `has` as `false`
- **AND** the output SHALL equal the interpreter's canonical output for the same program

#### Scenario: Generated JavaScript steps and falls back
- **WHEN** NX source contains `type Book = { author?:Person } let root() = { <Box name={<Book />.author?.name ?? "anonymous"} /> }`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the serialized output SHALL carry `name` as `"anonymous"`

#### Scenario: Generated JavaScript evaluates the fallback lazily
- **WHEN** NX source contains `let f(o?:int): int = { o ?? fail() }` where `fail` raises a runtime error, and `root` calls `f(1)`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the output SHALL be `1` and `fail` SHALL NOT run

#### Scenario: Generated JavaScript refuses a match on the empty pattern
- **WHEN** NX source contains `type Book = { author?:Person } let root() = { if <Book />.author is { {} => "anonymous" else => "named" } }`
- **AND** JavaScript generation is requested for it
- **THEN** generation SHALL fail with a diagnostic saying match expressions are not supported by
  executable source codegen

#### Scenario: Operator parity is validated against the interpreter
- **WHEN** a supported NX program uses `x?`, `x?.m` and `x ?? y`
- **AND** the same program is emitted and executed as generated JavaScript or TypeScript
- **THEN** automated tests SHALL verify that both executions produce equivalent `NxValue` payloads

### Requirement: A generated function's result takes its declared occurrence
Generated TypeScript and JavaScript SHALL give a function's result, and a value's initializer, the
occurrence of its declared type, as the interpreter's coercion does: an item returned where a `+`
or `*` type is declared SHALL be a one-item array. A generated function SHALL type its result by
its declared or inferred type. A generated function is typed NX code rather than a host boundary:
an empty `T?` result SHALL be the generated `nxEmpty`, as the function's TypeScript signature
`T | NxEmpty` says, where an entry call through the interpreter or the NX IR runtime returns the
host's `null`. Every other result SHALL equal the interpreter's canonical output.

#### Scenario: A declared many result is lifted
- **WHEN** NX source contains `let g(): int+ = { 5 }` and `type Ints = int+ let fives: Ints = { 5 }`
- **AND** JavaScript is generated and executed
- **THEN** `g()` and `fives` SHALL each be `[5]`, as the interpreter gives

#### Scenario: An empty optional result is nxEmpty
- **WHEN** NX source contains `let maybe(): string? = { if false { "a" } }`
- **AND** TypeScript is generated
- **THEN** `maybe` SHALL be typed as returning `string | NxEmpty` and SHALL return `nxEmpty` when called

### Requirement: Generated TypeScript declares every type its signatures name
Generated TypeScript SHALL name only types it declares or imports. It SHALL declare each type
alias a signature or field names, as a TypeScript type alias of the aliased type.
It SHALL emit a derived update record or property union whenever an emitted type names one, whether
that type was written or inferred.

#### Scenario: An alias named by a signature is declared
- **WHEN** NX source contains `type Ints = int+` and `let total(xs:Ints): int = 1`
- **AND** TypeScript is generated
- **THEN** the module SHALL declare `type Ints = readonly number[]`
- **AND** an alias imported from another module SHALL be imported as a type

#### Scenario: An inferred result names a derived declaration
- **WHEN** NX source contains `type Person = { name:string age?:int }` and
  `let edited(a:Person, b:Person) = { changed(diff(a, b)) }`
- **AND** TypeScript is generated
- **THEN** the module SHALL declare `Person_Property`, which the inferred result type names

### Requirement: Generated TypeScript models record inheritance with interfaces
Generated TypeScript SHALL declare an abstract record, and a record or union case that extends an
abstract base, as an interface. An abstract record's `$type` SHALL be typed `string`. A record or
case extending a base SHALL extend the base's interface and type `$type` as its own name. A value
of a record extending a base SHALL be accepted wherever the base is expected, including an object
literal passed directly: generated TypeScript SHALL pin such a literal to its own type with
`satisfies T as T`. JavaScript output SHALL be unchanged.

#### Scenario: A derived literal is passed where its base is expected
- **WHEN** NX source contains `abstract type Shape = { id:int } type Circle extends Shape = { r:int }`,
  `let idOf(s:Shape): int = { s.id }` and `let root() = { idOf(<Circle id={1} r={2} />) }`
- **AND** TypeScript is generated
- **THEN** it SHALL declare `interface Shape` with `readonly $type: string` and
  `interface Circle extends Shape` with `readonly $type: "Circle"`
- **AND** it SHALL pass `tsc --strict`

#### Scenario: A base in another module is extended
- **WHEN** a module declares `export abstract type Shape = { id:int }` and another module declares
  `type Circle extends Shape = { r:int }` and passes a `Circle` literal where a `Shape` is expected
- **AND** TypeScript is generated
- **THEN** the second module SHALL import `Shape` as a type and declare `Circle` extending it
- **AND** it SHALL pass `tsc --strict`

## MODIFIED Requirements

### Requirement: TypeScript emission produces readable executable modules
The system SHALL emit executable TypeScript modules from a `CodegenProgram` for supported
non-reactive NX programs that produce serializable `NxValue` output. Generated TypeScript SHALL use
stable, readable names derived from NX declarations where possible, SHALL emit module
imports/exports needed for cross-module execution, SHALL emit strongly typed structural record
types whose fields use TypeScript `readonly` modifiers directly, SHALL emit `as const` value
objects with derived types for constant unions, and SHALL use NX runtime helpers only for shared
behavior that is not plain array, record, or constant-case construction.

#### Scenario: Root function emits executable TypeScript
- **WHEN** NX source defines `let root() = { 1 + 2 }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated TypeScript SHALL include an exported callable entrypoint for `root`
- **AND** executing that entrypoint through the generated runtime helpers SHALL produce the same
  value as interpreter evaluation

#### Scenario: Element-like record emits executable TypeScript
- **WHEN** NX source defines an intrinsic element expression or record-like element construction
  that evaluates to a serializable `NxValue::Record`
- **AND** a caller requests TypeScript executable output
- **THEN** executing the generated entrypoint SHALL produce the same serialized `NxValue` shape as
  interpreter evaluation

#### Scenario: Record emits as strongly typed direct object
- **WHEN** NX source declares `type User = { name: string age: int }` and constructs a `User`
- **AND** a caller requests TypeScript executable output
- **THEN** generated TypeScript SHALL include an exported `User` structural type with typed
  `readonly` fields
- **AND** generated TypeScript SHALL NOT wrap that structural type in `Readonly<T>`
- **AND** the record construction SHALL use a direct object literal with `$type` and fields in
  emitter-controlled order instead of a record helper call

#### Scenario: Enum emits as const value object with derived type
- **WHEN** NX source declares `type Theme = light | dark`
- **AND** a caller requests TypeScript executable output
- **THEN** generated TypeScript SHALL include an exported `Theme` value object whose members are
  authored bare strings and whose object expression is asserted `as const`
- **AND** generated TypeScript SHALL include an exported `Theme` type derived from that value object
  using `typeof Theme[keyof typeof Theme]`
- **AND** generated constant-case values SHALL be plain authored case strings exposed through that
  value object

#### Scenario: Array emits as plain JavaScript array with readonly TypeScript surface
- **WHEN** NX source defines `let root(): int+ = { 1 2 3 }`, or the same body at `int*`
- **AND** a caller requests TypeScript executable output
- **THEN** generated TypeScript SHALL type the return value as `readonly number[]`
- **AND** the generated expression SHALL return a plain JavaScript array literal without an array
  runtime helper call

#### Scenario: Cross-module program emits coherent TypeScript
- **WHEN** a `ProgramArtifact` contains a root module that imports a supported function, record,
  union, or value declaration from a resolved library module
- **AND** a caller requests TypeScript executable output
- **THEN** generated TypeScript SHALL include coherent module linkage for the imported target
- **AND** executing the generated entrypoint SHALL resolve the imported target without manual edits

### Requirement: Runtime helper encoding matches canonical NxValue
Generated output SHALL encode values using the same canonical `NxValue` semantics as the existing
public runtime boundary. Array values SHALL encode as plain JavaScript arrays. Constant union case
values SHALL encode as bare authored case strings. Record values and payload union cases SHALL
encode as plain object payloads with a `$type` discriminator when a type name is present. An empty
optional field SHALL be omitted from the record's object, an empty `*` field SHALL be an empty
array, and a present empty field of an update record SHALL be `null`, the one place the canonical
encoding writes it, as `update-records` requires. The generated runtime helper surface SHALL NOT
include array, record, or constant-case construction helpers unless a later semantic requirement
makes those helpers necessary. Action handlers and component lifecycle state SHALL NOT be encoded
by this change.

#### Scenario: Enum object exposes bare member string
- **WHEN** generated output returns a constant union case value
- **THEN** the generated value object SHALL expose the value as the bare authored case string
- **AND** the payload SHALL NOT include a type wrapper

#### Scenario: Record and union object literals encode typed records
- **WHEN** generated output returns a record or payload union case value
- **THEN** generated code SHALL encode the value as a direct object payload with `$type`
- **AND** declared fields SHALL be emitted as normal properties in deterministic generated-source
  order

#### Scenario: An empty optional field is omitted
- **WHEN** NX source contains `type Book = { title:string author?:string tags?:string+ } let root() = <Book title="B" />`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the output SHALL be `{ $type: "Book", title: "B" }` with neither an `author` nor a `tags` key
- **AND** the output SHALL equal the interpreter's canonical output for the same program

### Requirement: Component expressions emit atomic component descriptors
Executable TypeScript and JavaScript generation SHALL treat element expressions that resolve to
concrete components as atomic component descriptor construction. A component descriptor SHALL use
the canonical record-like component value shape with `$type` equal to the concrete component name
and fields for normalized props, content, and defaults. Constructing a component descriptor MUST
NOT evaluate that component's implementation body.

#### Scenario: Component expression returns descriptor without evaluating body
- **WHEN** NX source declares `component <Child label:string /> = { <Text value={label} /> }`
- **AND** declares `let root() = { <Child label="Name" /> }`
- **AND** a caller executes generated JavaScript for `root`
- **THEN** generated output SHALL return a descriptor with `$type` equal to `"Child"` and
  `label` equal to `"Name"`
- **AND** SHALL NOT evaluate `Child`'s component body while constructing that descriptor

#### Scenario: Parent component returns child descriptors
- **WHEN** NX source declares `component <Flow /> = { <Question label="Name" /> }`
- **AND** declares `external component <Question label:string />`
- **AND** a caller evaluates generated output for `Flow`
- **THEN** generated output SHALL return a `Question` component descriptor in the rendered output
- **AND** the descriptor SHALL preserve normalized prop `label`

#### Scenario: Component descriptor applies inherited defaults
- **WHEN** NX source declares `abstract external component <Question label:string = "Untitled" />`
- **AND** declares `external component <ShortTextQuestion extends Question placeholder?:string />`
- **AND** generated code evaluates `<ShortTextQuestion />`
- **THEN** the descriptor SHALL have `$type` equal to `"ShortTextQuestion"`
- **AND** SHALL include inherited prop `label` with value `"Untitled"`
- **AND** SHALL carry no `placeholder` key

#### Scenario: Parent returns two external child descriptors of same type
- **WHEN** NX source declares `external component <TextInput id:string label:string value:string = "" />`
- **AND** declares `component <QuestionFlow /> = { { <TextInput id="firstName" label="First name" /> <TextInput id="lastName" label="Last name" /> } }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL type the rendered result as a readonly collection of
  `TextInputElement`
- **AND** evaluating `QuestionFlowSchema.evaluateJson({})` SHALL return two `TextInputElement`
  values with distinct `id` and `label` fields
- **AND** both values SHALL include normalized default `value` equal to `""`

### Requirement: Generated executable code evaluates property references and update intrinsics
Generated executable TypeScript and JavaScript SHALL evaluate a property union case to the bare
string of the field name, so that its serialized form matches the canonical encoding, and SHALL
emit a referenced property union as a constant union of that program's generated declarations
under the same naming and value-object rules as an authored constant union. A call to `apply`,
`merge`, `diff`, or `changed` SHALL be emitted as a call into the separately supplied NX runtime
helpers rather than inlined, SHALL preserve the absent-versus-empty rule — an absent field is
unchanged, a present empty field clears an optional field and is carried through `merge` and `diff`
as present and empty — and SHALL NOT collide with an author's declarations, which the checker
already prevents from using the intrinsic names. Generated behavior for both SHALL be validated
against the interpreter on the same source.

#### Scenario: Generated JavaScript serializes a property union case as its field name
- **WHEN** NX source contains `type User = { name:string email?:string } let root() = <Box key={User.Property.email} />`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the serialized output SHALL carry `key` as the string `"email"`
- **AND** the output SHALL equal the interpreter's canonical output for the same program

#### Scenario: Generated JavaScript applies an update through the runtime
- **WHEN** NX source contains `type User = { name:string email?:string } let root() = {apply(<User name="Ada" email="x@y" />, <User.Update email={} />)}`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the output SHALL be `{ $type: "User", name: "Ada" }` with no `email` key
- **AND** the generated module SHALL reach the operation through the supplied runtime rather than an inlined implementation

#### Scenario: Generated JavaScript lists changed fields in declaration order
- **WHEN** NX source contains `type User = { name:string email?:string age?:int } let root() = {changed(<User.Update age={} name="Ada" />)}`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the output SHALL be `["name", "age"]`

#### Scenario: Generated JavaScript encodes a present empty update field as null
- **WHEN** NX source contains `type User = { name:string email?:string } let root() = <User.Update email={} />`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the output SHALL be `{ $type: "User.Update", email: null }` with no `name` key
