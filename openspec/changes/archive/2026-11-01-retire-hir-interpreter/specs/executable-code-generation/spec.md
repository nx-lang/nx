## ADDED Requirements

### Requirement: Generated behavior is validated against the native runtime
The executable code generation implementation SHALL include tests that compare generated
TypeScript/JavaScript behavior with native evaluation of the same program for supported
non-reactive programs. The parity tests SHALL cover primitives, arithmetic, conditionals, arrays,
loops, function calls, records, discriminated unions, enum values, element expressions that
serialize to `NxValue::Record`, and cross-module imports.

#### Scenario: Native parity for supported program
- **WHEN** a supported NX program is evaluated natively
- **AND** the same program is emitted and executed as generated JavaScript or TypeScript
- **THEN** both executions SHALL produce equivalent `NxValue` payloads
- **AND** object property order SHALL NOT be treated as semantic during the comparison

#### Scenario: Unsupported executable construct reports diagnostic
- **WHEN** codegen encounters an NX construct outside the supported initial executable generation
  subset
- **THEN** codegen SHALL return a diagnostic that identifies the unsupported construct
- **AND** codegen SHALL NOT emit silently incomplete executable output

### Requirement: NX IR emission is validated against source-level expectations
Executable generation tests SHALL verify that NX IR emitted from source evaluates, on the Rust IR
runtime and on the TypeScript IR runtime, to a value the test states. These tests SHALL cover
functions, component descriptor construction, component initialization, explicit state component
evaluation, and host-owned state update validation. Because both runtimes evaluate the same emitted
IR, agreement between them SHALL NOT be the only check of what the emitter produced.

#### Scenario: IR emission is checked for functions
- **WHEN** executable generation emits NX IR for a supported function program
- **THEN** tests SHALL execute the emitted IR through both IR runtimes
- **AND** SHALL compare each result with the expected value the test states

#### Scenario: IR emission is checked for components
- **WHEN** executable generation emits NX IR for a supported component program
- **THEN** tests SHALL execute descriptor construction, initialization, explicit state evaluation,
  and state patch validation through both IR runtimes
- **AND** SHALL compare rendered values with the expected values the test states

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
  value as Rust IR runtime evaluation

#### Scenario: Element-like record emits executable TypeScript
- **WHEN** NX source defines an intrinsic element expression or record-like element construction
  that evaluates to a serializable `NxValue::Record`
- **AND** a caller requests TypeScript executable output
- **THEN** executing the generated entrypoint SHALL produce the same serialized `NxValue` shape as
  Rust IR runtime evaluation

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
- **AND** the output SHALL equal the Rust IR runtime's canonical output for the same program

### Requirement: Generated code emits component descriptor functions and schema entry objects
Executable TypeScript and JavaScript code generation SHALL emit host-neutral component descriptor
functions and schema entry objects for concrete NX component declarations. Generated descriptor
functions SHALL construct atomic component descriptors. Generated schema entry objects SHALL expose
JSON initialization and evaluation APIs for selecting that component as an executable entry point.
Generated TypeScript MAY emit abstract/base contracts for abstract component declarations when
needed to preserve inherited props, but abstract components MUST NOT be directly instantiable or
evaluable.

#### Scenario: Concrete component emits a descriptor function and schema entry
- **WHEN** NX source declares `component <SearchBox placeholder:string /> = { <TextInput value={placeholder} /> }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include an exported `SearchBox` descriptor function
- **AND** generated output SHALL include a `SearchBoxSchema` entry object that can initialize and
  evaluate `SearchBox` from JSON-compatible props

#### Scenario: Abstract component is contract-only
- **WHEN** NX source declares `abstract component <SearchBase placeholder:string />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL NOT expose an API that directly initializes or evaluates
  `SearchBase`
- **AND** concrete descendants MAY use the generated base contract for inherited props

#### Scenario: Derived component preserves inherited props
- **WHEN** NX source declares `abstract component <SearchBase placeholder:string = "Docs" />`
- **AND** declares `component <SearchBox extends SearchBase /> = { <TextInput value={placeholder} /> }`
- **AND** a caller initializes `SearchBox` through generated output without an explicit
  `placeholder`
- **THEN** generated evaluation SHALL bind inherited prop `placeholder` to `"Docs"`
- **AND** SHALL return rendered output equivalent to Rust IR runtime component initialization

#### Scenario: Defaulted prop is optional in typed props
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { placeholder }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include an exported `SearchBoxProps` type where `placeholder` is
  optional
- **AND** calling `SearchBox({})` or `SearchBox()` SHALL construct a descriptor with `placeholder`
  equal to `"Find docs"`

### Requirement: Generated component behavior is validated against runtime semantics
The executable code generation implementation SHALL include tests that compare generated component
descriptor construction and generated component entry evaluation with existing NX runtime
semantics for supported non-reactive component scenarios.

#### Scenario: Component descriptor parity
- **WHEN** a supported NX program constructs a component descriptor through ordinary expression
  evaluation
- **AND** the same program is emitted and executed as generated JavaScript or TypeScript
- **THEN** both executions SHALL produce equivalent JSON-compatible component descriptor payloads

#### Scenario: Component entry evaluation parity
- **WHEN** a supported NX component is evaluated by the Rust IR runtime with explicit props and state
- **AND** the same component is emitted and evaluated through generated JavaScript or TypeScript
- **THEN** both executions SHALL produce equivalent rendered payloads

### Requirement: Program module output preserves resolved cross-module behavior
JavaScript program module output SHALL preserve the behavior of resolved cross-module NX programs
without emitting relative imports between generated NX source modules. References to declarations
from different runtime modules SHALL be emitted as deterministic, collision-free local generated
names inside the single program module.

#### Scenario: Cross-module function import is flattened
- **WHEN** a `ProgramArtifact` contains a root module that imports a supported function or value
  from another resolved module
- **AND** the caller requests JavaScript program module output
- **THEN** the generated program module SHALL contain executable code for the imported declaration
  and the root entrypoint in the same ESM source module
- **AND** executing the generated root entrypoint with the supplied NX runtime SHALL produce the
  same JSON-compatible value as Rust IR runtime evaluation
- **AND** the generated program module SHALL NOT import another generated NX module by relative
  path

#### Scenario: Cross-module component schema import is flattened
- **WHEN** a `ProgramArtifact` contains a component that constructs or references a concrete
  component declared in another resolved module
- **AND** the caller requests JavaScript program module output
- **THEN** generated component descriptor functions and schema values SHALL remain coherent inside
  the single generated program module
- **AND** executing the generated component entry API with the supplied NX runtime SHALL preserve
  descriptor and evaluation behavior equivalent to existing JavaScript file output

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
against the Rust IR runtime on the same source.

#### Scenario: Generated JavaScript serializes a property union case as its field name
- **WHEN** NX source contains `type User = { name:string email?:string } let root() = <Box key={User.Property.email} />`
- **AND** JavaScript is generated and executed for `root`
- **THEN** the serialized output SHALL carry `key` as the string `"email"`
- **AND** the output SHALL equal the Rust IR runtime's canonical output for the same program

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

### Requirement: Generated executable code evaluates the presence operators
Generated executable TypeScript and JavaScript SHALL evaluate the presence test `x?`, the step
`x?.m` and the fallback `x ?? y` to the values `presence-operators` defines,
so that the serialized result of a generated entrypoint equals the Rust IR runtime's canonical output
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
- **AND** the output SHALL equal the Rust IR runtime's canonical output for the same program

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

#### Scenario: Operator parity is validated against the Rust IR runtime
- **WHEN** a supported NX program uses `x?`, `x?.m` and `x ?? y`
- **AND** the same program is emitted and executed as generated JavaScript or TypeScript
- **THEN** automated tests SHALL verify that both executions produce equivalent `NxValue` payloads

### Requirement: A generated function's result takes its declared occurrence
Generated TypeScript and JavaScript SHALL give a function's result, and a value's initializer, the
occurrence of its declared type, as the IR runtimes' coercion does: an item returned where a `+`
or `*` type is declared SHALL be a one-item array. A generated function SHALL type its result by
its declared or inferred type. A generated function is typed NX code rather than a host boundary:
an empty `T?` result SHALL be the generated `nxEmpty`, as the function's TypeScript signature
`T | NxEmpty` says, where an entry call through an NX IR runtime returns the
host's `null`. Every other result SHALL equal the Rust IR runtime's canonical output.

#### Scenario: A declared many result is lifted
- **WHEN** NX source contains `let g(): int+ = { 5 }` and `type Ints = int+ let fives: Ints = { 5 }`
- **AND** JavaScript is generated and executed
- **THEN** `g()` and `fives` SHALL each be `[5]`, as the Rust IR runtime gives

#### Scenario: An empty optional result is nxEmpty
- **WHEN** NX source contains `let maybe(): string? = { if false { "a" } }`
- **AND** TypeScript is generated
- **THEN** `maybe` SHALL be typed as returning `string | NxEmpty` and SHALL return `nxEmpty` when called

## REMOVED Requirements

### Requirement: Generated behavior is validated against the interpreter
**Reason**: The interpreter is deleted.
**Migration**: Replaced by "Generated behavior is validated against the native runtime", with the
same coverage and the Rust IR runtime as the native side.

### Requirement: NX IR emission is validated against interpreter semantics
**Reason**: The interpreter is deleted, and it was the one engine that did not consume emitted IR.
**Migration**: Replaced by "NX IR emission is validated against source-level expectations".
