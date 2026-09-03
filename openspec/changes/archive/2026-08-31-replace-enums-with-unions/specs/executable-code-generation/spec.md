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
- **WHEN** NX source defines `let root(): int[] = { 1 2 3 }`
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

### Requirement: JavaScript emission produces executable ESM
The system SHALL emit executable JavaScript ESM from a `CodegenProgram` using the same emitter
pipeline as TypeScript emission with type-only syntax disabled. Generated JavaScript SHALL NOT
require a TypeScript runtime compiler to execute, and SHALL use the same plain record and
constant-case runtime semantics as the generated TypeScript target.

#### Scenario: Root function emits executable JavaScript
- **WHEN** NX source defines `let root() = { "hello" }`
- **AND** a caller requests JavaScript executable output
- **THEN** generated JavaScript SHALL include an exported callable entrypoint for `root`
- **AND** the generated JavaScript SHALL execute in a standard ESM host without TypeScript syntax

#### Scenario: JavaScript and TypeScript targets agree
- **WHEN** a supported NX program is emitted as both TypeScript and JavaScript
- **THEN** executing each emitted entrypoint SHALL produce equivalent serialized `NxValue` payloads

### Requirement: Runtime helper encoding matches canonical NxValue
Generated output SHALL encode values using the same canonical `NxValue` semantics as the existing
public runtime boundary. Array values SHALL encode as plain JavaScript arrays. Constant union case
values SHALL encode as bare authored case strings. Record values and payload union cases SHALL
encode as plain object payloads with a `$type` discriminator when a type name is present. The
generated runtime helper surface SHALL NOT include array, record, or constant-case construction
helpers unless a later semantic requirement makes those helpers necessary. Action handlers and
component lifecycle state SHALL NOT be encoded by this change.

#### Scenario: Enum object exposes bare member string
- **WHEN** generated output returns a constant union case value
- **THEN** the generated value object SHALL expose the value as the bare authored case string
- **AND** the payload SHALL NOT include a type wrapper

#### Scenario: Record and union object literals encode typed records
- **WHEN** generated output returns a record or payload union case value
- **THEN** generated code SHALL encode the value as a direct object payload with `$type`
- **AND** declared fields SHALL be emitted as normal properties in deterministic generated-source
  order
