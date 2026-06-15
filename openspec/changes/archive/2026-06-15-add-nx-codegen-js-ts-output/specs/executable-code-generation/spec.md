## ADDED Requirements

### Requirement: CodegenProgram is built from ProgramArtifact
The system SHALL provide an `nx-codegen` API that builds a `CodegenProgram` from a successful
`ProgramArtifact`. The `CodegenProgram` SHALL preserve the program fingerprint, runtime module
identities, module provenance, module-qualified item references, entrypoints, imported item
references, source-map inputs, source spans where available, and type information needed by
executable code generation. Code generation MUST fail with diagnostics instead of emitting
executable output when the input artifact contains error diagnostics or when a required lowered
module or type environment is unavailable.

#### Scenario: Successful artifact produces codegen program
- **WHEN** a caller builds a valid `ProgramArtifact` for an NX source file with `root()`
- **THEN** `nx-codegen` SHALL produce a `CodegenProgram`
- **AND** the `CodegenProgram` SHALL identify the selected entry module and `root()` entrypoint
- **AND** the `CodegenProgram` SHALL preserve the artifact fingerprint

#### Scenario: Invalid artifact is rejected
- **WHEN** a caller requests executable code generation for a `ProgramArtifact` with static error
  diagnostics
- **THEN** `nx-codegen` SHALL return diagnostics
- **AND** `nx-codegen` SHALL NOT emit TypeScript or JavaScript output

#### Scenario: Imported references remain module-qualified
- **WHEN** a root source file imports a function from a resolved library module
- **THEN** the generated `CodegenProgram` SHALL represent the imported function as a
  module-qualified reference
- **AND** executable emitters SHALL NOT rediscover the imported target by scanning visible string
  names at emission time

### Requirement: TypeScript emission produces readable executable modules
The system SHALL emit executable TypeScript modules from a `CodegenProgram` for supported
non-reactive NX programs that produce serializable `NxValue` output. Generated TypeScript SHALL use
stable, readable names derived from NX declarations where possible, SHALL emit module
imports/exports needed for cross-module execution, SHALL emit strongly typed structural record
types whose fields use TypeScript `readonly` modifiers directly, SHALL emit `as const` enum value
objects with derived enum types, and SHALL use NX runtime helpers only for shared behavior that is
not plain array, record, or enum construction.

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
- **WHEN** NX source declares `enum Theme = light | dark`
- **AND** a caller requests TypeScript executable output
- **THEN** generated TypeScript SHALL include an exported `Theme` value object whose members are
  authored bare strings and whose object expression is asserted `as const`
- **AND** generated TypeScript SHALL include an exported `Theme` type derived from that value object
  using `typeof Theme[keyof typeof Theme]`
- **AND** generated enum values SHALL be plain authored member strings exposed through a generated
  enum value object

#### Scenario: Array emits as plain JavaScript array with readonly TypeScript surface
- **WHEN** NX source defines `let root(): int[] = { 1 2 3 }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated TypeScript SHALL type the return value as `readonly number[]`
- **AND** the generated expression SHALL return a plain JavaScript array literal without an array
  runtime helper call

#### Scenario: Cross-module program emits coherent TypeScript
- **WHEN** a `ProgramArtifact` contains a root module that imports a supported function, record,
  union, enum, or value declaration from a resolved library module
- **AND** a caller requests TypeScript executable output
- **THEN** generated TypeScript SHALL include coherent module linkage for the imported target
- **AND** executing the generated entrypoint SHALL resolve the imported target without manual edits

### Requirement: JavaScript emission produces executable ESM
The system SHALL emit executable JavaScript ESM from a `CodegenProgram` using the same emitter
pipeline as TypeScript emission with type-only syntax disabled. Generated JavaScript SHALL NOT
require a TypeScript runtime compiler to execute, and SHALL use the same plain record/enum runtime
semantics as the generated TypeScript target.

#### Scenario: Root function emits executable JavaScript
- **WHEN** NX source defines `let root() = { "hello" }`
- **AND** a caller requests JavaScript executable output
- **THEN** generated JavaScript SHALL include an exported callable entrypoint for `root`
- **AND** the generated JavaScript SHALL execute in a standard ESM host without TypeScript syntax

#### Scenario: JavaScript and TypeScript targets agree
- **WHEN** a supported NX program is emitted as both TypeScript and JavaScript
- **THEN** executing each emitted entrypoint SHALL produce equivalent serialized `NxValue` payloads

### Requirement: Initial executable generation is eager and non-reactive
Executable TypeScript and JavaScript generated by this change SHALL use eager evaluation semantics
for supported NX expressions. The generated runtime helper surface MUST NOT expose reactive
dependency tracking, invalidation, subscription, signal, component lifecycle, dispatch, or action
handler APIs as part of this change. Unsupported reactive, lifecycle, or action-handler constructs,
if encountered, MUST produce codegen diagnostics rather than implicit behavior.

#### Scenario: Generated expressions evaluate eagerly
- **WHEN** generated output evaluates an NX expression containing conditionals, loops, arrays, or
  function calls
- **THEN** the expression SHALL be evaluated when the generated entrypoint is invoked
- **AND** the generated output SHALL NOT register reactive dependencies for later invalidation

#### Scenario: Reactive semantics are not advertised
- **WHEN** a caller inspects the generated runtime helper output for this change
- **THEN** the helper surface SHALL NOT include public signal, subscription, invalidation, or
  dependency-graph APIs

#### Scenario: Component lifecycle is not advertised
- **WHEN** a caller inspects the generated runtime helper output for this change
- **THEN** the helper surface SHALL NOT include public component initialization, state evaluation,
  dispatch, or action-handler invocation APIs

### Requirement: CLI and public APIs expose executable generation separately from DTO generation
The system SHALL expose executable TypeScript and JavaScript generation through public Rust APIs
and a new `nxlang codegen` CLI entry point that consumes the existing source/workspace analysis
pipeline and produces generated files. The types-only DTO/declaration generation surface SHALL be
exposed through `nxlang typegen` and SHALL remain separate from executable generation.

#### Scenario: CLI codegens executable TypeScript
- **WHEN** a user runs `nxlang codegen` for a valid NX source or workspace entrypoint with a
  TypeScript target
- **THEN** the CLI SHALL build a `ProgramArtifact`
- **AND** the CLI SHALL emit generated TypeScript files from `nx-codegen`
- **AND** `nxlang typegen --language typescript` DTO/type output SHALL remain separate from the
  executable output

#### Scenario: CLI codegens executable JavaScript
- **WHEN** a user runs `nxlang codegen` for a valid NX source or workspace entrypoint with a
  JavaScript target
- **THEN** the CLI SHALL build a `ProgramArtifact`
- **AND** the CLI SHALL emit generated JavaScript files from `nx-codegen`
- **AND** the emitted JavaScript SHALL be executable without a TypeScript compiler

#### Scenario: CLI generates types-only contracts separately
- **WHEN** a user runs `nxlang typegen` for a valid NX source or library with a TypeScript or C#
  target
- **THEN** the CLI SHALL emit DTO/type declaration output for interacting with NX-authored
  contracts
- **AND** the CLI SHALL NOT emit executable NX program behavior from `nxlang typegen`

### Requirement: Generated behavior is validated against the interpreter
The executable code generation implementation SHALL include tests that compare generated
TypeScript/JavaScript behavior with existing interpreter behavior for supported non-reactive
programs. The parity tests SHALL cover primitives, arithmetic, conditionals, arrays, loops,
function calls, records, discriminated unions, enum values, element expressions that serialize to
`NxValue::Record`, and cross-module imports.

#### Scenario: Interpreter parity for supported program
- **WHEN** a supported NX program is evaluated by the interpreter
- **AND** the same program is emitted and executed as generated JavaScript or TypeScript
- **THEN** both executions SHALL produce equivalent `NxValue` payloads
- **AND** object property order SHALL NOT be treated as semantic during the comparison

#### Scenario: Unsupported executable construct reports diagnostic
- **WHEN** codegen encounters an NX construct outside the supported initial executable generation
  subset
- **THEN** codegen SHALL return a diagnostic that identifies the unsupported construct
- **AND** codegen SHALL NOT emit silently incomplete executable output

### Requirement: Runtime helper encoding matches canonical NxValue
Generated output SHALL encode values using the same canonical `NxValue` semantics as the existing
public runtime boundary. Array values SHALL encode as plain JavaScript arrays. Enum values SHALL
encode as bare authored member strings. Record values and discriminated union cases SHALL encode as
plain object payloads with a `$type` discriminator when a type name is present. The generated
runtime helper surface SHALL NOT include array, record, or enum construction helpers unless a later
semantic requirement makes those helpers necessary. Action handlers and component lifecycle state
SHALL NOT be encoded by this change.

#### Scenario: Enum object exposes bare member string
- **WHEN** generated output returns an enum value
- **THEN** the generated enum value object SHALL expose the value as the bare authored member string
- **AND** the payload SHALL NOT include an enum type wrapper

#### Scenario: Record and union object literals encode typed records
- **WHEN** generated output returns a record or discriminated union case value
- **THEN** generated code SHALL encode the value as a direct object payload with `$type`
- **AND** declared fields SHALL be emitted as normal properties in deterministic generated-source
  order

### Requirement: Generated output ordering is deterministic
Executable TypeScript and JavaScript generation SHALL produce deterministic module ordering,
declaration ordering, generated identifier selection, import ordering, and emitted record/property
source ordering for equivalent `ProgramArtifact` inputs. Generated object property order SHALL be a
deterministic output property, not part of NX record value semantics.

#### Scenario: Equivalent inputs produce equivalent output
- **WHEN** two equivalent `ProgramArtifact` inputs are generated with the same codegen options
- **THEN** the emitted file list and file contents SHALL be stable across repeated runs

#### Scenario: Record properties emit in stable order
- **WHEN** generated output constructs a record from an unordered runtime or HIR source collection
- **THEN** emitted object literals SHALL use deterministic property ordering controlled by the
  emitter
- **AND** no runtime sorting helper SHALL be required for ordinary record construction

### Requirement: Program artifacts expose source text for codegen
The public `nx-api` surface SHALL expose read-only source text access for `ProgramArtifact` inputs
so `nx-codegen` can produce source maps and source-aware diagnostics without re-reading files or
depending on artifact-internal storage details.

#### Scenario: Source text lookup by identity
- **WHEN** a `ProgramArtifact` preserves source text for a source-provider module
- **THEN** public `nx-api` accessors SHALL allow `nx-codegen` to retrieve that source text by the
  module's normalized identity

#### Scenario: Source entries are iterable
- **WHEN** `nx-codegen` builds source maps for a generated program
- **THEN** public `nx-api` accessors SHALL allow it to iterate the preserved source entries needed
  for source-map construction
