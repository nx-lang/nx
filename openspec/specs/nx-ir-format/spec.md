# nx-ir-format Specification

## Purpose
TBD - created by archiving change add-nx-ir-format. Update Purpose after archive.
## Requirements
### Requirement: NX IR program artifacts are versioned and deterministic
The system SHALL define a versioned NX IR artifact emitted from a successful `ProgramArtifact`, one
per emitted module. The artifact SHALL be a little-endian binary image whose fixed header carries a
format magic, IR schema version `3` and the image's total length, and whose sections carry the
expected runtime ABI `nx-ir-runtime-v2`, the required feature list, the module table, the module's
public entrypoints, and the module's tables and declarations. Equivalent `ProgramArtifact` inputs
with equivalent IR options SHALL produce byte-identical images. A reader SHALL refuse an image whose
magic or schema version it does not implement with a diagnostic naming the version found and the
version supported, and SHALL NOT interpret the bytes that follow the header.

#### Scenario: Valid program artifact emits IR metadata
- **WHEN** a caller emits NX IR from a valid `ProgramArtifact` containing a `root()` function
- **THEN** the image SHALL carry schema version `3` and runtime ABI `nx-ir-runtime-v2`
- **AND** the image SHALL list `root` as a function entrypoint
- **AND** the module table's first entry SHALL carry the module's identity and fingerprint

#### Scenario: Equivalent inputs produce byte-identical images
- **WHEN** two equivalent `ProgramArtifact` inputs are emitted as NX IR with the same options
- **THEN** the emitted images SHALL use stable ordering for the module table, tables,
  declarations, fields, properties, entrypoints and nodes
- **AND** the two emitted images SHALL be byte-identical

#### Scenario: Invalid artifact is rejected
- **WHEN** a caller requests NX IR emission for a `ProgramArtifact` containing static error
  diagnostics
- **THEN** IR emission SHALL fail with diagnostics
- **AND** the system SHALL NOT emit a partial artifact

#### Scenario: An older schema is refused
- **WHEN** a runtime is given an image whose schema version is `2`
- **THEN** preparation SHALL fail with a diagnostic naming version `2` and the supported version `3`
- **AND** the reader SHALL NOT interpret the bytes that follow the header

#### Scenario: A document that is not an image is refused
- **WHEN** a runtime is given bytes that do not begin with the NX IR magic
- **THEN** preparation SHALL fail with a diagnostic saying the input is not an NX IR image

### Requirement: NX IR program fingerprints are lossless for JavaScript consumers
Every fingerprint in an NX IR artifact, whether of the artifact's own module or of a module in its
table, SHALL be encoded in a form that represents the native fingerprint without JavaScript `number`
precision loss.

#### Scenario: Fingerprint exceeds JavaScript safe integer range
- **WHEN** an emitted module fingerprint is greater than JavaScript's maximum safe integer
- **THEN** the artifact SHALL store the full 64-bit value, as a low and a high 32-bit cell
- **AND** a JavaScript reader SHALL recombine the cells without converting the value to `number`
- **AND** structured metadata returned with the generated IR SHALL expose the same fingerprint as a
  string or another explicitly lossless representation
- **AND** JavaScript consumers SHALL NOT need to parse the value as `number` to compare cache
  identity

### Requirement: NX IR preserves resolved module-qualified references
NX IR SHALL represent every reference to a declaration as a module-table slot and the declaration's
name, with slot `0` meaning the artifact's own module. Function calls, value references, component
descriptors, record construction, union cases, default expressions, selected entrypoints and nominal
type references SHALL all use this form, and no reference SHALL depend on the position of a
declaration in its module or of a module in a program. Top-level declaration names SHALL be unique
within a module, and the compiler SHALL reject a module that declares one name twice. Primitive type
references SHALL be encoded separately from nominal type references so runtimes do not resolve
records, unions or type aliases through global bare-name lookup.

#### Scenario: Imported function reference is module-qualified
- **WHEN** a root module imports function `answer()` from a resolved library module
- **AND** NX IR is emitted for the program
- **THEN** the call to `answer()` SHALL reference the library module's table slot and the name
  `answer`
- **AND** the TypeScript runtime SHALL resolve it through the linked module's declarations by name

#### Scenario: Imported component descriptor reference is module-qualified
- **WHEN** a root module constructs a component exported by another resolved module
- **AND** NX IR is emitted for the program
- **THEN** the component descriptor expression SHALL reference the component by that module's table
  slot and the component's name

#### Scenario: Imported nominal type reference is module-qualified
- **WHEN** a root module declares a function parameter or component prop using record, union, or
  type-alias `User` imported from another module
- **AND** another module in the same program also declares an item named `User`
- **THEN** the emitted type reference SHALL be nominal and SHALL name the imported module's slot and
  `User`
- **AND** supported runtimes SHALL normalize boundary values through that declaration rather than a
  bare `User` lookup

#### Scenario: A regenerated module keeps old references valid
- **WHEN** a snippet artifact references `SkiaLabel` in `drawnui`
- **AND** `drawnui` is regenerated with a new control declared before `SkiaLabel`
- **THEN** the snippet artifact, unchanged, SHALL still reference `SkiaLabel`
- **AND** a runtime linking it against the regenerated `drawnui` SHALL resolve `SkiaLabel`

#### Scenario: Duplicate top-level names are rejected
- **WHEN** a module declares `let Card = 1` and `type Card = { }`
- **THEN** analysis SHALL report an error naming `Card` and both declarations

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
index access, member access, record literals, union cases, intrinsic elements, component
descriptors, and action handlers. There SHALL be one union-case construct covering both constant
and payload cases rather than separate constructs for enum members and union cases, and that
construct SHALL mark a constant case as constant in expression position as well as in the
declaration, so a runtime produces the bare case name without consulting the declaration. A call to
one of the update intrinsics (`apply`, `merge`, `diff`, `changed`) SHALL be encoded as an intrinsic
call construct that names the intrinsic and carries its argument expressions, distinct from a call
to a declared function, and a program that contains one SHALL list a required feature naming
update-intrinsic support. A `changed` call SHALL also carry the declared field order of the target
record, so a runtime orders the result without consulting the declaration.

An action-handler binding (`onTapped=<Update count={count + 1} />`) SHALL be encoded as an
action-handler node kind that carries a reference to the component and the name of the emit the
handler answers, a reference to the action record it accepts, the slot the handler's `action`
binding occupies, a reference to the owner component whose state the handler may patch (absent for
a handler bound outside any component body), and the body node. The node SHALL NOT carry an
evaluated environment; captured locals are the slots the body references. The public name of the
action a runtime reports for the handler SHALL be the action reference's declaration name. A
module that contains the node SHALL list a required feature naming action-handler support, and a
module that contains none SHALL NOT list it. Unsupported executable constructs SHALL be reported
as IR build diagnostics.

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

#### Scenario: A handler bound inside a component body is encoded with its owner
- **WHEN** NX source declares `external component <Button emits { Tapped { } } />` and
  `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`
- **AND** NX IR is emitted for the program
- **THEN** the `onTapped` property of the `Button` descriptor SHALL be encoded with the
  action-handler node kind
- **AND** the node SHALL reference component `Button`, name emit `Tapped`, and reference the
  `Button.Tapped` action record
- **AND** the node SHALL reference `Counter` as the owner
- **AND** the body SHALL be the `Counter.Update` construction, referencing `count` through the
  slot of `Counter`'s `count` state field
- **AND** the required feature list SHALL name action-handler support

#### Scenario: A handler bound outside a component body has no owner
- **WHEN** NX source contains `let root() = { <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> /> }`
- **AND** NX IR is emitted for the program
- **THEN** the node SHALL carry no owner
- **AND** the body SHALL reference `action` through the slot the node declares for it
- **AND** that slot SHALL be the next slot of the enclosing frame, as a `let` binding's would be

#### Scenario: A program without handlers lists no handler feature
- **WHEN** NX source binds no action handler
- **AND** NX IR is emitted for the program
- **THEN** the required feature list SHALL NOT name action-handler support

#### Scenario: Unsupported action handler is rejected in v1
- **WHEN** NX source binds an action handler
- **AND** NX IR is emitted for the program
- **THEN** emission SHALL succeed, carrying the handler as the action-handler node
- **AND** the same source SHALL still fail executable TypeScript or JavaScript generation as
  `executable-code-generation` requires

### Requirement: NX IR encodes component contracts, descriptors, and state metadata
NX IR SHALL preserve effective component prop contracts, declared state fields, defaults, content
field metadata, abstract/external/concrete component flags, the effective emits of the component
(each emit's name and a reference to the action record it carries, inherited emits included, in
declaration order), component body expressions where available, and schemas needed to normalize
props and state at runtime. Component descriptor expressions SHALL remain atomic and SHALL encode
normalized descriptor construction rather than deep-rendering the referenced component body. A
descriptor's handler properties SHALL be encoded as properties of the descriptor, alongside its
declared props, so a runtime can carry the parent's bindings with the instance.

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

#### Scenario: Component declarations carry their emits
- **WHEN** NX source declares `action Reset = { }` and
  `component <Counter emits { Reset ValueChanged { value:int } } /> = { <Label /> }`
- **AND** NX IR is emitted for the program
- **THEN** the `Counter` declaration SHALL list emits `Reset` and `ValueChanged` in declaration
  order
- **AND** `Reset` SHALL reference the `Reset` action record and `ValueChanged` SHALL reference the
  inline `Counter.ValueChanged` record
- **AND** a component that declares no emits SHALL list none

#### Scenario: An inherited emit references its declaring module
- **WHEN** a component extends a base declared in another module whose inline emit is `Tapped`
- **AND** NX IR is emitted for the extending component's module
- **THEN** the extending component's `emits` SHALL list `Tapped` referencing the base module's
  `<Base>.Tapped` record

#### Scenario: A descriptor keeps its handler property beside its props
- **WHEN** NX source constructs `<Counter step=2 onReset=<Log /> />` for a component that declares
  prop `step` and emits `Reset`
- **AND** NX IR is emitted for the program
- **THEN** the descriptor SHALL carry both `step` and `onReset` as properties
- **AND** `onReset`'s value SHALL be the action-handler node

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
An NX IR artifact SHALL be able to carry a debug section holding source spans for declarations and
nodes and the module's source text, and the section SHALL be optional: an artifact without it SHALL
prepare and evaluate exactly as one with it. The module's source fingerprint SHALL be present
whether or not the section is. A runtime diagnostic SHALL cite the span when the section is present
and the declaration name when it is not, and SHALL never need to read a source file from disk.

#### Scenario: Runtime diagnostic can identify source expression
- **WHEN** the TypeScript IR runtime reports a runtime diagnostic for an expression of an artifact
  that carries a debug section
- **THEN** the diagnostic SHALL identify the originating source identity and span

#### Scenario: A stripped artifact still diagnoses
- **WHEN** the same diagnostic arises in the same artifact emitted without a debug section
- **THEN** the diagnostic SHALL name the declaration the expression belongs to
- **AND** evaluation up to that point SHALL be identical to the artifact with the section

#### Scenario: Stripping is the emitter's choice
- **WHEN** a caller emits an artifact and asks for no debug section
- **THEN** the artifact SHALL contain no spans and no source text
- **AND** the artifact with and without the section SHALL differ only in that section

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

### Requirement: NX IR erases component type parameters
NX IR SHALL NOT carry component type parameters. A component's prop schema and state schema in IR
SHALL describe each field with the type parameter replaced by the top type `object`, the
component's derived update record SHALL describe each field the same way, and a component
descriptor in IR SHALL NOT include a property for a type argument the source bound. IR emitted for
a program with generic components SHALL remain deterministic and boundary-clean.

#### Scenario: Prop schema carries the erased type
- **WHEN** NX source declares `external component <SkiaLayout TItem:type itemsSource:TItem[]? />`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration in IR SHALL include a prop schema for `itemsSource` typed as a nullable list of `object`
- **AND** it SHALL NOT include a prop schema or any other entry for `TItem`

#### Scenario: Descriptor omits the type argument
- **WHEN** NX source declares `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource:TItem[]? /> let v = <SkiaLayout TItem=Contact itemsSource={} />`
- **AND** NX IR is emitted for the program
- **THEN** the descriptor expression for `v` SHALL carry a property for `itemsSource` and no property for `TItem`

#### Scenario: Update record carries the erased type
- **WHEN** NX source declares `component <List TItem:type items:TItem[]? /> = { state { sel:TItem? = null } <Label /> } let u = <List.Update sel=null />`
- **AND** NX IR is emitted for the program
- **THEN** the `List.Update` declaration in IR SHALL type `sel` as nullable `object` and SHALL NOT mention `TItem`

### Requirement: An artifact carries one module and names the modules it links against
An NX IR artifact SHALL carry exactly one module. It SHALL carry a module table whose first entry
describes the artifact's own module and whose remaining entries describe every module the artifact
references, each entry giving the module's logical identity, the version string the build was given
for it (empty when none was given) and a fingerprint of its source. A program that spans several
modules SHALL be emitted as one artifact per module, and a build SHALL let the caller choose which
modules to emit.

#### Scenario: A snippet compiled against a catalog names the catalog
- **WHEN** a workspace holds `drawnui` with version `9` and `input.nx`, and `input.nx` uses a control
  `drawnui` declares
- **AND** the caller emits an artifact for `input.nx` only
- **THEN** the build SHALL return one artifact
- **AND** its module table SHALL list `input.nx` first and `drawnui` with version `9` second
- **AND** the artifact SHALL contain no declaration from `drawnui`

#### Scenario: A module nothing references is not in the table
- **WHEN** a workspace holds a module the entry never imports, directly or transitively
- **THEN** the entry's artifact SHALL NOT list that module

#### Scenario: A catalog emits as its own artifact
- **WHEN** the caller emits an artifact for `drawnui` alone
- **THEN** the artifact's module table SHALL hold only `drawnui`
- **AND** it SHALL carry every external component, union and record `drawnui` declares

### Requirement: NX IR modules are encoded as flat tables
A module SHALL be encoded as a string table, a type table, a constant table, a node table and a
declaration list. Node kinds, type kinds and declaration kinds SHALL be small integers; every name,
literal and type SHALL appear once in its table and be referenced by index; children SHALL be node
indices or index ranges. Slots SHALL be integers local to the declaration that owns them, and
element identities SHALL be integers local to the module.

The string table SHALL be stored as an offset array over one UTF-8 blob, so a reader borrows a
name as a slice of the image and MAY decode only the names it resolves. Every other table SHALL be
stored as an offset array over a flat pool of 32-bit cells, so entry `k` is addressable by index
without reading the entries before it. Sections of the image SHALL be listed in a directory in its
header, and a reader SHALL skip a directory entry whose kind it does not know, so a section can be
added without a schema change.

#### Scenario: A name is written once
- **WHEN** a module uses the property name `Text` on forty elements
- **THEN** the emitted artifact SHALL contain the string `Text` once in its string table
- **AND** every use SHALL refer to it by index

#### Scenario: A type is written once
- **WHEN** twenty component props share the type nullable `string`
- **THEN** the type table SHALL contain nullable `string` once
- **AND** each prop SHALL refer to it by index

#### Scenario: Slots are declaration-local integers
- **WHEN** a function declares two parameters and a `let` inside its body
- **THEN** their slots SHALL be the integers `0`, `1` and `2` within that function
- **AND** another declaration SHALL be free to use the same integers

#### Scenario: An entry is addressable without reading its predecessors
- **WHEN** a runtime resolves a declaration referenced by a linked module
- **THEN** it SHALL read that declaration's entry without decoding the entries before it

#### Scenario: A name is decoded only when it is named
- **WHEN** a module of five hundred strings is prepared and three of its declarations are resolved
- **THEN** the reader SHALL decode only the strings those resolutions name

#### Scenario: An unknown section is skipped
- **WHEN** an image's directory lists a section of a kind the reader does not know, beside every
  section it requires
- **THEN** the reader SHALL prepare the module from the sections it knows
- **AND** SHALL NOT refuse the artifact for the unknown section

### Requirement: A conformance corpus defines NX IR behavior
The repository SHALL hold a corpus of NX programs, each with its expected artifact for every emitted
module, the explained text of each expected artifact, and its expected evaluation results for every
entrypoint the corpus names. A corpus program MAY also name component lifecycles: a component to
initialize, optional props, and ordered batches to dispatch against it, whose entries are written
as a host would send them, handler tokens included. The expected results SHALL then record the
rendered output of initialization and, for each batch in turn, the rendered output and the ordered
effects, all as canonical values with their handler tokens, as the interpreter produces them. The
emitter's tests SHALL check that each program emits its expected artifact byte for byte, SHALL
report a difference as explained text, and SHALL check that each committed explained text is the
explanation of its committed artifact. Every supported runtime SHALL check that it evaluates each
expected artifact to the expected results, lifecycles included. One regeneration command SHALL
write the artifacts, their explained text and the expected results together. The corpus SHALL cover
every node kind, type kind and declaration kind, cross-artifact references, and a program emitted
without its debug section. For every corpus program, the artifact emitted without a debug section
SHALL be at most six times the UTF-8 length of the program's own module source.

#### Scenario: Emitter output is pinned by the corpus
- **WHEN** the emitter is changed so that a corpus program's artifact differs
- **THEN** the emitter's corpus test SHALL fail naming the program and the difference as explained
  text

#### Scenario: A change to the emitter is readable in review
- **WHEN** the emitter is changed so that a corpus program's artifact differs
- **AND** the corpus is regenerated
- **THEN** both the image and its explained text SHALL be updated
- **AND** the explained text's diff SHALL name what changed about the program

#### Scenario: Stale explained text is caught
- **WHEN** a corpus image is committed without regenerating its explained text
- **THEN** the emitter's corpus test SHALL fail naming the program

#### Scenario: A runtime is checked against the corpus
- **WHEN** a runtime evaluates a corpus artifact's named entrypoint
- **THEN** its canonical value SHALL equal the expected result recorded for it

#### Scenario: A runtime's dispatch is checked against the corpus
- **WHEN** a runtime initializes a corpus lifecycle's component and dispatches its batches in order
- **THEN** each rendered output, its tokens included, and each effect list SHALL equal the values
  recorded for it

#### Scenario: The size budget is enforced
- **WHEN** a corpus program of 8,000 bytes of source is emitted without a debug section
- **THEN** the artifact SHALL be at most 48,000 bytes

### Requirement: An NX IR reader validates an artifact before it evaluates it
A reader SHALL establish, before answering any question about an artifact, that the image's header
is one it implements, that its recorded length equals the bytes it was given, that every section,
offset, string range, table index, module slot and optional operand the artifact contains lies
within the artifact's own bounds, and that every string is valid UTF-8. A malformed or truncated
artifact SHALL be refused with a diagnostic rather than an exception or a crash, SHALL NOT produce
a value, and SHALL NOT cause a read outside the artifact in any runtime.

#### Scenario: A truncated artifact is refused
- **WHEN** a runtime is given an image cut short at any four-byte boundary
- **THEN** preparation SHALL fail with a diagnostic
- **AND** no evaluation SHALL take place

#### Scenario: An out-of-range index is refused rather than followed
- **WHEN** an artifact's node names a string index beyond the string table
- **THEN** the reader SHALL report the artifact as malformed
- **AND** SHALL NOT read memory outside the artifact

#### Scenario: A hostile artifact cannot escape its bounds
- **WHEN** a host plays a share authored by someone else whose offsets have been altered
- **THEN** the runtime SHALL refuse it with a diagnostic rather than reading outside the artifact

#### Scenario: Every cell of an artifact can be damaged without a crash
- **WHEN** any single cell of a valid artifact is overwritten with an arbitrary value
- **THEN** every reader SHALL either refuse the artifact with a diagnostic or read it as a valid
  artifact
- **AND** no reader SHALL throw an exception or panic
