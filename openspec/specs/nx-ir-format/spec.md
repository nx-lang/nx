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
value references, component descriptors, record construction, union cases, default expressions,
selected entrypoints, and nominal type references SHALL identify the owning module and declaration
or expression slot needed for execution. Primitive type references SHALL be encoded separately from
nominal type references so runtimes do not resolve records, unions, or type aliases through global
bare declaration-name lookup.

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
- **WHEN** a root module declares a function parameter or component prop using record, union, or
  type-alias `User` imported from another module
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
function calls, intrinsic calls, `if`, match-style `if is` forms, `let`, blocks, arrays, loops,
index access, member access, record literals, union cases, intrinsic elements, and component
descriptors. There SHALL be one union-case construct covering both constant and payload cases
rather than separate constructs for enum members and union cases, and that construct SHALL mark a
constant case as constant in expression position as well as in the declaration, so a runtime
produces the bare case name without consulting the declaration. A call to one of the update
intrinsics (`apply`, `merge`, `diff`, `changed`) SHALL be encoded as an intrinsic call construct
that names the intrinsic and carries its argument expressions, distinct from a call to a declared
function, and a program that contains one SHALL list a required feature naming update-intrinsic
support. A `changed` call SHALL also carry the declared field order of the target record, so a
runtime orders the result without consulting the declaration. Unsupported executable constructs
SHALL be reported as IR build diagnostics.

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

#### Scenario: Constant and payload cases use one IR construct
- **WHEN** NX source contains `type Shape = circle | square { n:int }` and constructs both cases
- **AND** NX IR is emitted for the program
- **THEN** both constructions SHALL be encoded by the same union-case construct
- **AND** the construct SHALL carry enough information for a runtime to produce the bare string for
  `circle` and the `$type` map for `square`

#### Scenario: A constant case expression is marked constant
- **WHEN** NX source contains `type Mode = light | dark let m = {Mode.dark}`
- **AND** NX IR is emitted for the program
- **THEN** the union-case expression for `Mode.dark` SHALL be marked as a constant case
- **AND** a runtime evaluating it SHALL produce the bare string `"dark"` rather than a `$type` map

#### Scenario: An intrinsic call is encoded distinctly from a function call
- **WHEN** NX source contains `type User = { name:string } let v = {apply(<User name="Ada" />, <User.Update name="Bo" />)}`
- **AND** NX IR is emitted for the program
- **THEN** the call SHALL be encoded with the intrinsic call construct naming `apply` and carrying two argument expressions
- **AND** the required feature list SHALL name update-intrinsic support
- **AND** for `let c = {changed(<User.Update name="Bo" />)}` the construct SHALL name `changed` and carry the field order `name`

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
Array values SHALL evaluate as arrays, constant union cases SHALL evaluate as authored case strings,
records and payload union cases SHALL evaluate as object/map payloads with `$type` discriminators
when their type requires one, and numeric values that cannot safely round-trip through JavaScript
numbers SHALL use a lossless tagged representation.

#### Scenario: Enum output remains a bare string
- **WHEN** NX source evaluates `Theme.dark` where `Theme` is a constant union
- **AND** the value is produced through an NX IR runtime
- **THEN** the canonical output value SHALL be the bare string `"dark"`
- **AND** the output SHALL NOT wrap the value in an object

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

### Requirement: NX IR declares the update records a program references
When a program references a derived update record `T.Update` — by constructing one, by naming it in
a type annotation, or by declaring a state or prop field of that type — emitted NX IR SHALL contain
a declaration for it. The declaration SHALL identify itself as an update record, SHALL name its
target `T`, and SHALL carry its own field schemas so that a runtime can normalize a value of the
type without consulting the target: every field optional, no default expressions, and nullability
copied from the target field. Update record declarations that the program does not reference SHALL
NOT be emitted, so that programs which never use a patch produce the same IR as before. A program
that references an update record SHALL list a required feature naming update-record support, so a
runtime that predates this change rejects the program rather than misreading the declaration.

#### Scenario: A referenced update record is declared with its own schema
- **WHEN** NX source contains `type User = { name:string = "anon" email:string? } let patch = <User.Update email={null} />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL contain a declaration for `User.Update` marked as an update record targeting `User`
- **AND** that declaration SHALL list fields `name` and `email`, both optional, with `email` nullable and neither carrying a default

#### Scenario: Unreferenced update records are not emitted
- **WHEN** NX source contains `type User = { name:string } let user = <User name="Ada" />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL NOT contain a declaration for `User.Update`
- **AND** the required feature list SHALL be unchanged from a program without update records

#### Scenario: Construction of an update record is encoded as record construction
- **WHEN** NX source constructs `<Counter.Update count=1 />` for a component `Counter` with state `count:int`
- **AND** NX IR is emitted for the program
- **THEN** the expression SHALL be encoded with the existing record construction operation targeting the `Counter.Update` declaration
- **AND** the emitted canonical value SHALL carry `$type` `Counter.Update` and only the `count` key

### Requirement: NX IR declares the property unions a program references
When a program references a derived property union `T.Property` — by naming one of its cases, by
naming it in a type annotation, or by declaring a state or prop field of that type — emitted NX IR
SHALL contain a union declaration for it. The declaration SHALL identify itself as a property
union, SHALL name its target `T`, and SHALL list its cases as constant cases in `T.Property`'s
case order, so that a runtime can validate a bare-string value of the type without consulting the
target. Property union declarations that the program does not reference SHALL NOT be emitted. A
program that references a property union SHALL list a required feature naming property-union
support, so a runtime that predates this change rejects the program rather than misreading the
declaration. A case of a property union SHALL be encoded with the existing union-case construct
as a constant case.

#### Scenario: A referenced property union is declared with its cases
- **WHEN** NX source contains `abstract type Named = { name:string } type User extends Named = { email:string? } let key = {User.Property.email}`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL contain a union declaration for `User.Property` marked as a property union targeting `User`
- **AND** that declaration SHALL list constant cases `name` then `email` and no base

#### Scenario: Unreferenced property unions are not emitted
- **WHEN** NX source contains `type User = { name:string } let user = <User name="Ada" />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL NOT contain a declaration for `User.Property`
- **AND** the required feature list SHALL be unchanged from a program without property unions

#### Scenario: A property union case is encoded as a constant union case
- **WHEN** NX source contains `type User = { name:string } let key = {User.Property.name}`
- **AND** NX IR is emitted for the program
- **THEN** the expression SHALL be encoded with the existing union-case construct referencing the `User.Property` declaration and case `name`
- **AND** the emitted canonical value SHALL be the bare string `"name"`
