## MODIFIED Requirements

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

### Requirement: NX IR encodes the supported eager expression set
NX IR SHALL encode the supported non-reactive expression forms needed for eager evaluation,
including literals, local slot references, top-level references, unary and binary operations,
function calls, `if`, match-style `if is` forms, `let`, blocks, arrays, loops, index access,
member access, record literals, union cases, intrinsic elements, and component descriptors. There
SHALL be one union-case construct covering both constant and payload cases rather than separate
constructs for enum members and union cases. Unsupported executable constructs SHALL be reported as
IR build diagnostics.

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

#### Scenario: Unsupported action handler is rejected in v1
- **WHEN** NX source requires an action-handler value that cannot be represented by the v1 IR
  feature set
- **THEN** IR emission SHALL fail with a diagnostic identifying the unsupported construct
- **AND** the emitted IR SHALL NOT silently drop the handler

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
