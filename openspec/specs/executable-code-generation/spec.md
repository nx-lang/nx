# executable-code-generation Specification

## Purpose
Define executable TypeScript and JavaScript generation for supported non-reactive NX programs.

## Requirements

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
for supported NX expressions. Generated component initialization and evaluation APIs MAY evaluate a
selected concrete component entry body eagerly, using normalized props and state. Component
descriptor construction MUST remain atomic: constructing `<Component ... />` SHALL NOT evaluate
that component's implementation body. The generated runtime helper surface MUST NOT expose
reactive dependency tracking, invalidation, subscription, signal, dispatch, state-update reducer,
or action-handler invocation APIs as part of this change. Unsupported reactive, dispatch,
state-update, or action-handler constructs, if encountered, MUST produce codegen diagnostics rather
than implicit behavior.

#### Scenario: Generated expressions evaluate eagerly
- **WHEN** generated output evaluates an NX expression containing conditionals, loops, arrays, or
  function calls
- **THEN** the expression SHALL be evaluated when the generated entrypoint is invoked
- **AND** the generated output SHALL NOT register reactive dependencies for later invalidation

#### Scenario: Reactive semantics are not advertised
- **WHEN** a caller inspects the generated runtime helper output for this change
- **THEN** the helper surface SHALL NOT include public signal, subscription, invalidation, or
  dependency-graph APIs

#### Scenario: Generated component APIs are non-reactive
- **WHEN** a caller inspects generated component descriptor functions, schema entries, and runtime
  helper output
- **THEN** the generated surface MAY include schema component initialization and evaluation APIs
- **AND** SHALL NOT include public dispatch, state-update reducer, subscription, or action-handler
  invocation APIs

#### Scenario: Component descriptor construction does not deep-render children
- **WHEN** a generated component body constructs `<Child />`
- **THEN** generated output SHALL construct a `Child` descriptor eagerly
- **AND** SHALL NOT evaluate `Child`'s implementation body unless `Child` is separately selected
  as the component entry being initialized or evaluated

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
- **AND** SHALL return rendered output equivalent to interpreter component initialization

#### Scenario: Defaulted prop is optional in typed props
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { placeholder }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include an exported `SearchBoxProps` type where `placeholder` is
  optional
- **AND** calling `SearchBox({})` or `SearchBox()` SHALL construct a descriptor with `placeholder`
  equal to `"Find docs"`

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
- **AND** declares `external component <ShortTextQuestion extends Question placeholder:string? />`
- **AND** generated code evaluates `<ShortTextQuestion />`
- **THEN** the descriptor SHALL have `$type` equal to `"ShortTextQuestion"`
- **AND** SHALL include inherited prop `label` with value `"Untitled"`

#### Scenario: Parent returns two external child descriptors of same type
- **WHEN** NX source declares `external component <TextInput id:string label:string value:string = "" />`
- **AND** declares `component <QuestionFlow /> = { { <TextInput id="firstName" label="First name" /> <TextInput id="lastName" label="Last name" /> } }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL type the rendered result as a readonly collection of
  `TextInputElement`
- **AND** evaluating `QuestionFlowSchema.evaluateJson({})` SHALL return two `TextInputElement`
  values with distinct `id` and `label` fields
- **AND** both values SHALL include normalized default `value` equal to `""`

### Requirement: Function element expressions still evaluate eagerly
Executable TypeScript and JavaScript generation SHALL preserve the existing function element-call
semantics. When an element expression resolves to a function, generated code SHALL call that
function and substitute its returned value into the surrounding expression rather than returning a
descriptor for the function itself.

#### Scenario: Function element call substitutes returned component descriptor
- **WHEN** NX source declares `external component <Question label:string />`
- **AND** declares `let MakeQuestion(label:string) = { <Question label={label} /> }`
- **AND** declares `let root() = { <MakeQuestion label="Name" /> }`
- **THEN** generated output for `root` SHALL return a `Question` component descriptor
- **AND** SHALL NOT return a descriptor whose `$type` is `"MakeQuestion"`

### Requirement: Generated component state uses generated types and helpers
Executable TypeScript and JavaScript generation SHALL emit state types and helper functions for
each concrete component that declares state. Generated schema entry objects SHALL normalize
JSON-compatible state input, and generated helpers SHALL materialize initial state from normalized
props and state defaults. Abstract components SHALL NOT emit state helpers.

#### Scenario: Stateful component emits state helper surface
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include a `SearchBox` descriptor function and `SearchBoxSchema`
  entry object
- **AND** SHALL include a `SearchBoxState` type or equivalent exported state helper surface
  associated with `SearchBox`

#### Scenario: Initialization returns rendered output and initial state
- **WHEN** a caller initializes generated `SearchBox` without explicit props or state
- **THEN** generated output SHALL materialize prop `placeholder` as `"Find docs"`
- **AND** SHALL materialize initial state `query` as `"Find docs"`
- **AND** SHALL return both rendered output and the JSON-compatible initial state

#### Scenario: Evaluation with omitted state uses initial state
- **WHEN** a caller evaluates generated `SearchBox` without supplying state
- **THEN** generated output SHALL materialize the same initial state used by initialization
- **AND** SHALL evaluate the component body with that state

#### Scenario: Evaluation with explicit state uses supplied state
- **WHEN** a caller evaluates generated `SearchBox` with state `{ query: "docs" }`
- **THEN** generated output SHALL evaluate the component body with `query` equal to `"docs"`
- **AND** SHALL NOT reevaluate the `query` default in place of the supplied state

#### Scenario: Abstract component emits no state helper
- **WHEN** NX source declares `abstract component <SearchBase placeholder:string />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL NOT include a `SearchBaseState` type or state helper

### Requirement: TypeScript schemas provide JSON boundary support
Executable TypeScript generation SHALL emit `Schema`-suffixed runtime values for generated
declarations that require JSON validation, JSON normalization, serialization, or diagnostic
conversion. Schema values SHALL contain or reference runtime metadata for the declaration shape and
SHALL use shared runtime helpers for primitive, array, record, enum, union, component props,
component state, and external element validation where those shapes can be represented as metadata.

#### Scenario: Component schema validates JSON props
- **WHEN** NX source declares `component <SearchBox placeholder:string /> = { placeholder }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include `SearchBoxSchema`
- **AND** `SearchBoxSchema` SHALL expose a JSON boundary operation that accepts untyped or
  JSON-compatible input for props
- **AND** supplying JSON props that omit `placeholder` through that boundary operation SHALL report
  a missing-field diagnostic
- **AND** calling the typed `SearchBox` function SHALL remain a strongly typed TypeScript call

#### Scenario: Schema metadata is shared for repeated field validation
- **WHEN** generated TypeScript output includes multiple declarations with `string` fields
- **THEN** generated schema values SHALL reference a shared string schema helper or metadata value
  for those fields
- **AND** generated output SHALL NOT emit independent custom string-validation control flow for
  each field unless it is emitted behind the same schema API as an implementation optimization

#### Scenario: Expression defaults remain generated typed code
- **WHEN** NX source declares a state field default that depends on a prop, such as
  `state { query:string = placeholder }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated schema metadata SHALL describe the state field's structural type
- **AND** generated typed code SHALL compute the prop-dependent default when materializing initial
  state
- **AND** JSON state validation SHALL reuse the schema metadata for supplied state values

### Requirement: Generated component APIs provide throwing and result-returning variants
Generated component entry APIs SHALL provide primary initialization and evaluation operations that
throw typed `NxRuntimeError` failures for invalid host input or generated runtime errors. Generated
component entry APIs SHALL also provide result-returning variants that report the same failures as
diagnostics without throwing.

#### Scenario: Invalid props throw typed runtime error
- **WHEN** a generated component requires prop `label:string`
- **AND** a caller evaluates the component with a JSON value that omits `label`
- **THEN** the throwing generated `evaluateJson` API SHALL throw an `NxRuntimeError`
- **AND** the error SHALL include a diagnostic code or message identifying the missing prop

#### Scenario: Try evaluation returns diagnostics
- **WHEN** a generated component requires prop `label:string`
- **AND** a caller evaluates the component through `tryEvaluateJson` with a JSON value that omits
  `label`
- **THEN** the result SHALL indicate failure without throwing
- **AND** SHALL include diagnostics equivalent to the throwing API failure

### Requirement: Generated component code rejects action-handler bindings
Executable TypeScript and JavaScript generation SHALL reject component action-handler bindings in
this change. When generated code would need to preserve, serialize, or invoke an action handler,
codegen MUST return diagnostics and MUST NOT emit silently incomplete executable output.

#### Scenario: Component action handler binding is rejected
- **WHEN** NX source declares `component <SearchBox emits { SearchSubmitted } /> = { <TextInput /> }`
- **AND** an expression constructs `<SearchBox onSearchSubmitted=<DoSearch query={action.query} /> />`
- **AND** a caller requests executable TypeScript or JavaScript output
- **THEN** codegen SHALL fail with a diagnostic that action-handler codegen is unsupported
- **AND** SHALL NOT emit executable output that drops the handler binding

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
- **WHEN** a supported NX component is evaluated by the interpreter with explicit props and state
- **AND** the same component is emitted and evaluated through generated JavaScript or TypeScript
- **THEN** both executions SHALL produce equivalent rendered payloads
