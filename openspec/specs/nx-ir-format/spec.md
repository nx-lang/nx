# nx-ir-format Specification

## Purpose
TBD - created by archiving change add-nx-ir-format. Update Purpose after archive.
## Requirements
### Requirement: NX IR JSON program artifacts are versioned and deterministic
The system SHALL define a versioned NX IR JSON program artifact emitted from a successful
`ProgramArtifact`. The IR document SHALL include a format identifier, IR schema version, expected
runtime ABI, program fingerprint, required feature list, public entrypoints, resolved modules,
declarations, expression data, type/schema metadata, and source provenance metadata. Equivalent
`ProgramArtifact` inputs with equivalent IR options SHALL produce byte-for-byte stable JSON output
apart from explicitly documented formatting choices.

#### Scenario: Valid program artifact emits IR metadata
- **WHEN** a caller emits NX IR from a valid `ProgramArtifact` containing a `root()` function
- **THEN** the IR JSON SHALL include the program fingerprint
- **AND** the IR JSON SHALL include the IR schema version and runtime ABI expected by loaders
- **AND** the IR JSON SHALL list `root` as a function entrypoint

#### Scenario: Equivalent inputs produce deterministic JSON
- **WHEN** two equivalent `ProgramArtifact` inputs are emitted as NX IR with the same options
- **THEN** the emitted JSON SHALL use stable ordering for modules, declarations, fields,
  properties, entrypoints, references, and expression records
- **AND** the two emitted JSON documents SHALL be equivalent for cache-key purposes

#### Scenario: Invalid artifact is rejected
- **WHEN** a caller requests NX IR emission for a `ProgramArtifact` containing static error
  diagnostics
- **THEN** IR emission SHALL fail with diagnostics
- **AND** the system SHALL NOT emit a partial IR document

### Requirement: NX IR program fingerprints are lossless for JavaScript consumers
NX IR JSON and structured IR metadata SHALL expose program fingerprints in a form that can represent
the native fingerprint without JavaScript `number` precision loss.

#### Scenario: Fingerprint exceeds JavaScript safe integer range
- **WHEN** an emitted program fingerprint is greater than JavaScript's maximum safe integer
- **THEN** the NX IR JSON SHALL encode `programFingerprint` as a decimal string
- **AND** structured metadata returned with the generated IR SHALL expose the same fingerprint as a
  string or another explicitly lossless representation
- **AND** JavaScript consumers SHALL NOT need to parse the value as `number` to compare cache
  identity

### Requirement: NX IR preserves resolved module-qualified references
NX IR SHALL represent executable references using module-qualified identifiers assigned by the
resolved program rather than relying on visible string-name lookup at runtime. Function calls,
value references, component descriptors, record construction, enum members, union cases, default
expressions, selected entrypoints, and nominal type references SHALL identify the owning module
and declaration or expression slot needed for execution. Primitive type references SHALL be encoded
separately from nominal type references so runtimes do not resolve records, unions, enums, or type
aliases through global bare declaration-name lookup.

#### Scenario: Imported function reference is module-qualified
- **WHEN** a root module imports function `answer()` from a resolved library module
- **AND** NX IR is emitted for the program
- **THEN** the call to `answer()` in the IR SHALL reference the owning library module and function
  declaration
- **AND** the TypeScript runtime SHALL NOT need to rediscover the function by scanning visible
  string names

#### Scenario: Imported component descriptor reference is module-qualified
- **WHEN** a root module constructs a component exported by another resolved module
- **AND** NX IR is emitted for the program
- **THEN** the component descriptor expression SHALL reference the concrete component declaration
  by module-qualified reference

#### Scenario: Imported nominal type reference is module-qualified
- **WHEN** a root module declares a function parameter or component prop using record, union, enum,
  or type-alias `User` imported from another module
- **AND** another module in the same IR program also declares an item named `User`
- **THEN** the emitted type reference SHALL be nominal and SHALL include the imported declaration's
  module-qualified reference
- **AND** supported runtimes SHALL normalize boundary values through that declaration reference
  rather than a bare `User` lookup

### Requirement: NX IR generation preserves directory-loaded library nominal references
NX IR generation SHALL preserve module-qualified nominal type references for declarations that
originate from libraries loaded through a library registry. Successfully validated and evaluated
program artifacts SHALL NOT fail IR generation with `codegen-missing-semantic-data` solely because a
referenced record, union, enum, or type alias came from a loaded library artifact.

#### Scenario: Library record field references dependency library type
- **WHEN** a program imports a directory-loaded `chat-link` library
- **AND** that library exposes a record or type alias containing `QuestionFlow`
- **AND** `QuestionFlow` is declared in a separate directory-loaded `question-flow` dependency
- **THEN** emitted NX IR SHALL encode the `QuestionFlow` reference as a nominal type reference
- **AND** that reference SHALL identify the owning `question-flow` module and declaration

#### Scenario: Transitive loaded library type reference is preserved
- **WHEN** a directory-loaded library declaration references `FlowStep` from another loaded library
  module
- **AND** validation and JSON evaluation for the resulting program artifact succeed
- **THEN** NX IR generation SHALL preserve the `FlowStep` type as a module-qualified nominal
  reference
- **AND** IR generation SHALL NOT require global bare-name lookup to rediscover `FlowStep`

### Requirement: Emitted IR is boundary-clean for valid nullable and content-boundary programs
For a source program that passes analysis and native evaluation, emitted NX IR SHALL preserve the
schema, default, nullable, nominal, and content-property metadata required for supported runtimes to
evaluate public entrypoints without rejecting the program's own generated values at a boundary
schema check. The IR runtime output SHALL match native canonical JSON-compatible output for
nullable union fields and content-derived required fields.

#### Scenario: Nullable union field does not emit synthetic invalid case
- **WHEN** a valid program constructs a record with an omitted or explicit-null field typed as a nullable discriminated union
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL encode the field absence as `null` or as an omitted nullable field that normalizes to `null`
- **AND** evaluating the IR SHALL NOT produce an undeclared union discriminator such as `FlowCompletion.undefined`

#### Scenario: Content property children satisfy required field through IR
- **WHEN** a valid program constructs a record or external component whose required content property is supplied by element body content
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL preserve the target content-field name and body expressions
- **AND** evaluating the IR SHALL populate the required content field before boundary validation reports missing fields

#### Scenario: Imported library record and union references remain boundary-clean
- **WHEN** a workspace program imports record, component, and union declarations from loaded libraries and returns a value using those imported declarations
- **AND** native evaluation succeeds and NX IR is emitted for the workspace program
- **THEN** evaluating the IR SHALL use module-qualified nominal references for boundary normalization
- **AND** the IR output SHALL match native canonical JSON-compatible output

### Requirement: NX IR encodes the supported eager expression set
NX IR SHALL encode the supported non-reactive expression forms needed for eager evaluation,
including literals, local slot references, top-level references, unary and binary operations,
function calls, `if`, match-style `if is` forms, `let`, blocks, arrays, loops, index access,
member access, record literals, enum members, union cases, intrinsic elements, and component
descriptors. Unsupported executable constructs SHALL be reported as IR build diagnostics.

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

#### Scenario: Unsupported action handler is rejected in v1
- **WHEN** NX source requires an action-handler value that cannot be represented by the v1 IR
  feature set
- **THEN** IR emission SHALL fail with a diagnostic identifying the unsupported construct
- **AND** the emitted IR SHALL NOT silently drop the handler

### Requirement: NX IR encodes component contracts, descriptors, and state metadata
NX IR SHALL preserve effective component prop contracts, declared state fields, defaults, content
field metadata, abstract/external/concrete component flags, component body expressions where
available, and schemas needed to normalize props and state at runtime. Component descriptor
expressions SHALL remain atomic and SHALL encode normalized descriptor construction rather than
deep-rendering the referenced component body.

#### Scenario: Stateful component emits state metadata
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration in IR SHALL include a prop schema for `placeholder`
- **AND** it SHALL include a state schema for `query`
- **AND** it SHALL preserve the default expression used to materialize the initial `query` value

#### Scenario: Component descriptor remains atomic
- **WHEN** NX source declares `component <Parent /> = { <Child /> }`
- **AND** NX IR is emitted for `Parent`
- **THEN** the expression for `<Child />` SHALL be represented as descriptor construction
- **AND** it SHALL NOT inline or pre-render `Child`'s body into `Parent`

### Requirement: NX IR preserves canonical NX value encoding rules
NX IR SHALL preserve enough type and value metadata for runtimes to produce canonical raw NX values.
Array values SHALL evaluate as arrays, enum values SHALL evaluate as authored member strings,
records and union cases SHALL evaluate as object/map payloads with `$type` discriminators when
their type requires one, and numeric values that cannot safely round-trip through JavaScript
numbers SHALL use a lossless tagged representation.

#### Scenario: Enum output remains a bare string
- **WHEN** NX source evaluates `Theme.dark`
- **AND** the value is produced through an NX IR runtime
- **THEN** the canonical output value SHALL be the bare string `"dark"`
- **AND** the output SHALL NOT wrap the enum value in an enum object

#### Scenario: Union case output includes discriminator
- **WHEN** NX source evaluates `LoadState.failed { message: "offline" }`
- **AND** the value is produced through an NX IR runtime
- **THEN** the canonical output value SHALL include `$type` with value `LoadState.failed`
- **AND** it SHALL include the declared `message` field

#### Scenario: Large integer literal is lossless
- **WHEN** NX source contains an integer literal that cannot be represented exactly as a JavaScript
  number
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL encode that literal with enough information for a JavaScript runtime to
  preserve the exact integer value or reject unsupported arithmetic explicitly

### Requirement: NX IR preserves source provenance for diagnostics and source maps
NX IR SHALL include source identities, optional source spans, and source entries or source-map
inputs needed for runtime diagnostics, generated tooling diagnostics, and artifact inspection
without re-reading source files from disk.

#### Scenario: Runtime diagnostic can identify source expression
- **WHEN** the TypeScript IR runtime reports a runtime diagnostic for an expression with preserved
  source span metadata
- **THEN** the diagnostic SHALL be able to identify the originating source identity and span
- **AND** the runtime SHALL NOT need to read the original source file from disk
