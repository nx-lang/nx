# nx-ir-format Specification

## Purpose
TBD - created by archiving change add-nx-ir-format. Update Purpose after archive.

## Requirements

### Requirement: NX IR program artifacts are versioned and deterministic
The system SHALL define a versioned NX IR artifact emitted from a successful `ProgramArtifact`, one
per emitted module. The artifact SHALL be a little-endian binary image whose fixed header carries a
format magic, IR schema version `5` and the image's total length, and whose sections carry the
expected runtime ABI `nx-ir-runtime-v2`, the required feature list, the module table, the module's
public entrypoints, and the module's tables and declarations. Equivalent `ProgramArtifact` inputs
with equivalent IR options SHALL produce byte-identical images. A reader SHALL refuse an image whose
magic or schema version it does not implement with a diagnostic naming the version found and the
version supported, and SHALL NOT interpret the bytes that follow the header.

Schema `5` differs from schema `4` by the `seq` type kind, which carries an occurrence and replaces
the `array` and `nullable` type kinds, by the `exists`, `optionalMember` and `coalesce` node kinds
and the `{}` match pattern, and by the `null` node kind no longer being emitted. Every other kind,
table and layout is unchanged. Schema `4` differed from schema `3` by the `text` node kind, `20`, by
the `float32` binary operators `16` to `19`, and by `concat` taking string operands only.

#### Scenario: Valid program artifact emits IR metadata
- **WHEN** a caller emits NX IR from a valid `ProgramArtifact` containing a `root()` function
- **THEN** the image SHALL carry schema version `5` and runtime ABI `nx-ir-runtime-v2`
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
- **WHEN** a runtime is given an image whose schema version is `4`
- **THEN** preparation SHALL fail with a diagnostic naming version `4` and the supported version `5`
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

### Requirement: Emitted IR is boundary-clean for valid optional and content-boundary programs
For a source program that passes analysis and native evaluation, emitted NX IR SHALL preserve the
schema, default, occurrence, nominal, and content-property metadata required for supported runtimes
to evaluate public entrypoints without rejecting the program's own generated values at a boundary
schema check. The IR runtime output SHALL match native canonical JSON-compatible output for
optional union fields and content-derived required fields. Absence is the empty value: an optional
field that holds it SHALL be encoded as an omitted field, and a `*` field that holds it as an empty
array, as `occurrence-types` defines the canonical encoding; no IR node and no runtime value SHALL
stand for `null`.

#### Scenario: Nullable union field does not emit synthetic invalid case
- **WHEN** a valid program constructs a record with an omitted or explicitly empty field declared `completion?:FlowCompletion` for a discriminated union `FlowCompletion`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL encode the field as omitted, or as an explicit empty value that normalizes to the empty value
- **AND** evaluating the IR SHALL produce canonical output with no `completion` key
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
function calls, intrinsic calls, `if`, match-style `if is` forms, `let`, blocks, braced sequences,
loops, index access, member access, the presence operators and the `{}` pattern, record literals,
union cases, intrinsic elements, component descriptors, and action handlers. There SHALL be one
union-case construct covering both constant and payload cases rather than separate constructs for
enum members and union cases, and that construct SHALL mark a constant case as constant in
expression position as well as in the declaration, so a runtime produces the bare case name without
consulting the declaration. A call to one of the update intrinsics (`apply`, `merge`, `diff`,
`changed`) SHALL be encoded as an intrinsic call construct that names the intrinsic and carries its
argument expressions, distinct from a call to a declared function, and a program that contains one
SHALL list a required feature naming update-intrinsic support. A `changed` call SHALL also carry
the declared field order of the target record, so a runtime orders the result without consulting
the declaration.

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
- **WHEN** NX IR contains an index expression over a sequence value
- **THEN** supported runtimes SHALL require an integer index
- **AND** an index outside the sequence bounds SHALL fail with a runtime diagnostic rather than
  evaluating to the empty value

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

### Requirement: NX IR makes primitive-to-text conversion explicit
NX IR SHALL encode the conversion of a primitive value to its canonical text form as its own node
kind, `text`, with the layout `[20, node, str]`: the operand node and the name of the operand's
static primitive type (`int`, `int32`, `int64`, `float32`, `float64` or `boolean`). The emitter
SHALL wrap every non-string operand of a string `+` in a `text` node, so the `concat` binary
operator's operands are always strings and a runtime SHALL NOT need to coerce them. The named type
is what lets a runtime that carries every number as a `float64` print a `float32` operand in the
`float32` text form `implicit-primitive-conversions` requires.

Whether a `+` is emitted as `add` or `concat` SHALL follow the type analysis recorded for the
expression, never the syntactic form of its operands. A numeric operand that analysis widened (an
`int` at a `float64` site, or an `int` operand of a `float64` addition) SHALL be recorded at its own
type with no conversion node, because every supported runtime carries the integer and floating-point
types in one numeric representation and the widening is unobservable there; a `div` or `mod` whose
checked type is floating-point SHALL be emitted as the floating-point operator even when one operand
is an integer.

An `ir explain` rendering SHALL print a `text` node as a readable conversion naming the operand
type.

#### Scenario: A string plus an int is emitted as concat over a text node
- **WHEN** NX source contains `let f(count:int) = "Total: " + count` and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `concat` operator
- **AND** its right operand SHALL be a `text` node naming `int` over the slot of `count`
- **AND** its left operand SHALL be the string constant with no conversion node

#### Scenario: A float32 operand names its type
- **WHEN** NX source contains `let f(w:float32) = w + " px"` and NX IR is emitted
- **THEN** the left operand of the `concat` SHALL be a `text` node naming `float32`

#### Scenario: A string field access is emitted as concat
- **WHEN** NX source contains `type Item = { title:string }` and
  `let f(item:Item) = "Reorder " + item.title`, and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `concat` operator
- **AND** neither operand SHALL be wrapped in a `text` node

#### Scenario: A widened operand carries no conversion node
- **WHEN** NX source contains `let f(n:int, x:float64) = n + x` and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `add` operator whose operands are the two
  slots directly
- **AND** for `let g(n:int, x:float64) = n / x` the operator SHALL be `div`, not `idiv`

### Requirement: NX IR makes float32 arithmetic explicit
NX IR SHALL encode an addition, subtraction, multiplication or division whose checked type is
`float32` with its own binary operator: `fadd32` (`16`), `fsub32` (`17`), `fmul32` (`18`) or
`fdiv32` (`19`). A runtime SHALL evaluate each by computing the operation on its operands and
rounding the result to the nearest `float32`, so that a runtime which carries every number as a
`float64` computes the value the interpreter computes, and widening or comparing that value
observes no difference. A remainder whose checked type is `float32` SHALL stay `mod`, since a
`float32` remainder is exact. A runtime SHALL round a number the host supplies at a `float32` site
to the nearest `float32`, and SHALL refuse an integer there that a `float32` does not represent
exactly, so every `float32` value it holds is one.

#### Scenario: A float32 product is emitted as fmul32
- **WHEN** NX source contains `let f(v:float32) = v * 3` and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `fmul32` operator
- **AND** for `let g(v:float32, d:float32) = v % d` the operator SHALL be `mod`
- **AND** for `let h(v:float32, x:float64) = v * x` the operator SHALL be `mul`

#### Scenario: A widened float32 product agrees with the interpreter
- **WHEN** `let scaled(v:float32): float64 = { v * 3 }` is evaluated by the TypeScript IR runtime
  with `v` holding the `float32` nearest `2.3`
- **THEN** the result SHALL be `6.899999618530273`, as the interpreter computes it
- **AND** `scaled(2.3) == 6.899999618530273` SHALL be `true`

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
A `+` or `*` value SHALL evaluate as an array, including the empty array for an empty `*` value; a
`?` value that holds an item SHALL evaluate as that item, and an empty `?` value SHALL be an
omitted key where it is a record field, as `occurrence-types` defines the canonical encoding.
Constant union cases SHALL evaluate as authored case strings, records and payload union cases SHALL
evaluate as object/map payloads with `$type` discriminators when their type requires one, and
numeric values that cannot safely round-trip through JavaScript numbers SHALL use a lossless tagged
representation. A runtime that cannot hold such a number SHALL refuse it explicitly where it reads
it, and SHALL NOT take a value a host passes for one because of its shape.

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

#### Scenario: An empty optional field is an omitted key and an empty sequence is an empty array
- **WHEN** NX source contains `type Book = { title:string author?:string tags?:string+ } let b() = <Book title="B" />` and `let t(b:Book): string* = { b.tags }`
- **AND** the values of `b()` and `t(b())` are produced through an NX IR runtime
- **THEN** the canonical output of `b()` SHALL contain `title` and neither an `author` nor a `tags` key, since an empty optional field is not stored whatever its occurrence
- **AND** the canonical output of `t(b())` SHALL be the empty array

#### Scenario: Large integer literal is lossless
- **WHEN** NX source contains an integer literal that cannot be represented exactly as a JavaScript
  number
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL encode that literal with enough information for a runtime to preserve the
  exact integer value, or to refuse the integer explicitly when it cannot hold it

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
type without consulting the target: every field optional in the sense that it may be absent, no
default expressions, and the clearable mark copied from the target field's optional mark, so the
runtime accepts a present empty value only where `T` declares the field `?:`. Update record
declarations that the program does not reference SHALL NOT be emitted, so that programs which never
use a patch produce the same IR as before. A program that references an update record SHALL list a
required feature naming update-record support, so a runtime that predates this change rejects the
program rather than misreading the declaration.

#### Scenario: A referenced update record is declared with its own schema
- **WHEN** NX source contains `type User = { name:string = "anon" email?:string } let patch = <User.Update email={} />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL contain a declaration for `User.Update` marked as an update record targeting `User`
- **AND** that declaration SHALL list fields `name` and `email`, both able to be absent, with `email` clearable, `name` not clearable, and neither carrying a default

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
- **WHEN** NX source contains `abstract type Named = { name:string } type User extends Named = { email?:string } let key = {User.Property.email}`
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
SHALL describe each field with the type parameter replaced by the top type `object`, keeping the
field's occurrence around the erased type, the component's derived update record SHALL describe
each field the same way, and a component descriptor in IR SHALL NOT include a property for a type
argument the source bound. IR emitted for a program with generic components SHALL remain
deterministic and boundary-clean.

#### Scenario: Prop schema carries the erased type
- **WHEN** NX source declares `external component <SkiaLayout TItem:type itemsSource?:TItem+ />`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration in IR SHALL include a prop schema for `itemsSource` typed `object*`: the `seq` type kind over `object` with the `*` occurrence
- **AND** it SHALL NOT include a prop schema or any other entry for `TItem`

#### Scenario: Descriptor omits the type argument
- **WHEN** NX source declares `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource?:TItem+ /> let v = <SkiaLayout TItem=Contact itemsSource={} />`
- **AND** NX IR is emitted for the program
- **THEN** the descriptor expression for `v` SHALL carry a property for `itemsSource` and no property for `TItem`

#### Scenario: Update record carries the erased type
- **WHEN** NX source declares `component <List TItem:type items?:TItem+ /> = { state { sel?:TItem } <Label /> } let u = <List.Update sel={} />`
- **AND** NX IR is emitted for the program
- **THEN** the `List.Update` declaration in IR SHALL type `sel` as `object?`, mark it clearable, and SHALL NOT mention `TItem`

### Requirement: NX IR erases record type parameters and type arguments
NX IR SHALL NOT carry record type parameters or type arguments. A generic record's field schema in
IR, and the field schema of its derived update record, SHALL describe each field with the type
parameter replaced by the top type `object`. An applied type SHALL be encoded as the nominal
reference to its record, with no encoding of its arguments, wherever a type reference appears —
including under an occurrence and inside a function type. A record construction in IR SHALL NOT
include a property for a type argument the source bound. Adding generic records SHALL NOT change
the IR schema version, and IR emitted for a program with generic records SHALL remain deterministic
and boundary-clean.

#### Scenario: Field schema carries the erased type
- **WHEN** NX source declares `type Range = { T:type start:T end:T endInclusive:boolean }`
- **AND** NX IR is emitted for the program
- **THEN** the record declaration in IR SHALL type `start` and `end` as `object` and `endInclusive` as `boolean`
- **AND** it SHALL NOT include a field or any other entry for `T`

#### Scenario: An applied type is a nominal reference
- **WHEN** NX source declares `type Range = { T:type start:T end:T }` and `type Slider = { range:<Range T=float64/> marks?:<Range T=int/>+ }`
- **AND** NX IR is emitted for the program
- **THEN** `range` SHALL be typed as the nominal reference to `Range` and `marks` as a `seq` type with the `*` occurrence over that same reference

#### Scenario: Construction omits the type argument
- **WHEN** NX source declares `type Range = { T:type start:T end:T }` and `let r = <Range T=int start={1} end={5} />`
- **AND** NX IR is emitted for the program
- **THEN** the construction of `r` SHALL carry properties for `start` and `end` and no property for `T`

#### Scenario: The schema version is unchanged
- **WHEN** NX IR is emitted for a program that declares and constructs a generic record
- **THEN** the image SHALL carry the same schema version as an image for a program without one
- **AND** an IR consumer built before generic records existed SHALL load and run it

### Requirement: An artifact carries one module and names the modules it links against
An NX IR artifact SHALL carry exactly one module. It SHALL carry a module table whose first entry
describes the artifact's own module and whose remaining entries describe every module the artifact
references, each entry giving the module's logical identity, the version string the build was given
for it (empty when none was given) and a fingerprint of its source. A program that spans several
modules SHALL be emitted as one artifact per module, and a build SHALL let the caller choose which
modules to emit.

The entry module's artifact SHALL also list, after the modules it references and in the program's
module order, every module of the program that declares a concrete record, or a union, extending an
abstract record that the parameter or result type of a function reaches, through fields, union
cases, type aliases, type arguments and the shapes extending an abstract record. Only the functions
of the modules the entry reaches through its imports, directly or transitively, count, since no
other function is linked from the entry; the declaring modules may be any of the program's. A
host may name any such shape by `$type` in a value it supplies at that site, and a runtime resolves
a shape only from a module it linked; listing the declaring modules in the entry makes a program
linked from its entry resolve every one, including a shape declared in a module nothing references.
No other artifact SHALL list a module it does not reference, so a library module's artifact stays
the same whichever program emitted it.

#### Scenario: A snippet compiled against a catalog names the catalog
- **WHEN** a workspace holds `drawnui` with version `9` and `input.nx`, and `input.nx` uses a control
  `drawnui` declares
- **AND** the caller emits an artifact for `input.nx` only
- **THEN** the build SHALL return one artifact
- **AND** its module table SHALL list `input.nx` first and `drawnui` with version `9` second
- **AND** the artifact SHALL contain no declaration from `drawnui`

#### Scenario: A module nothing references is not in the table
- **WHEN** a workspace holds a module the entry never imports, directly or transitively, and that
  declares no shape extending an abstract record a function of the program takes or returns
- **THEN** the entry's artifact SHALL NOT list that module

#### Scenario: A catalog emits as its own artifact
- **WHEN** the caller emits an artifact for `drawnui` alone
- **THEN** the artifact's module table SHALL hold only `drawnui`
- **AND** it SHALL carry every external component, union and record `drawnui` declares

#### Scenario: A function nothing imports adds nothing to the entry
- **WHEN** the entry `main.nx` imports nothing, and `tools.nx`, which nothing imports, declares
  `let use(s:Base): int` over an abstract `Base` that `base.nx` extends
- **THEN** the entry's module table SHALL hold only `main.nx`

#### Scenario: The entry lists the modules that declare a function's subtypes
- **WHEN** the entry `main.nx` declares `let f(s:Base): string`, `base.nx` declares the abstract
  `Base` and `type A extends Base`, `x.nx` declares `type X extends Base` and is imported by
  `main.nx` but never referenced, and `y.nx` declares `type Y extends Base` and is imported by
  nothing
- **THEN** the entry's module table SHALL be `main.nx`, `base.nx`, `x.nx`, `y.nx`
- **AND** the artifact of `x.nx` SHALL list only `x.nx` and `base.nx`
- **AND** a runtime that links the program from `main.nx` SHALL accept `{ "$type": "X", ... }` and
  `{ "$type": "Y", ... }` as the argument `s`

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
- **WHEN** twenty component props are declared `?:string`, so each is typed `string?`
- **THEN** the type table SHALL contain the `seq` type `string?` once
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

### Requirement: NX IR encodes function types
The type table SHALL have a function type kind. A function type entry SHALL record its result type
and, in declared order, each parameter's name, type and whether it is the content parameter, every
name and type by table index. A function type SHALL be written once in the type table and referred
to by index, like every other type. A prop, field, parameter, return or local typed by a function
type in source SHALL be typed by that entry in IR; the top type SHALL NOT stand in for it. An
optional property whose type is a function type SHALL be typed by the `seq` type with the `?`
occurrence over that entry. The explained form of an artifact SHALL render a function type in NX
spelling, parenthesized when it sits under an occurrence.

#### Scenario: A function-typed prop is typed as a function
- **WHEN** NX source declares `external component <List ItemTemplate?:<function Item:object Index:int />: string />`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration's prop schema for `ItemTemplate` SHALL be the `?` occurrence
  over a function type with the parameters `Item` of `object` and `Index` of `int` and the result
  `string`
- **AND** the explained text SHALL show it as `(<function Item:object Index:int />: string)?`

#### Scenario: Two identical function types share one entry
- **WHEN** two props are declared at the same function type
- **THEN** the type table SHALL contain that function type once

### Requirement: NX IR erases type parameters inside function types
Where a component's type parameter occurs inside a function type — as a parameter type or the
result type — the erasure that replaces the parameter with `object` SHALL reach it, so no emitted
type mentions the parameter.

#### Scenario: A template prop is erased through the function type
- **WHEN** NX source declares `external component <SkiaLayout TItem:type ItemTemplate?:<function Item:TItem Index:int />: object />`
- **AND** NX IR is emitted for the program
- **THEN** the prop schema for `ItemTemplate` SHALL be the `?` occurrence over a function type
  whose `Item` parameter is typed `object`
- **AND** the artifact SHALL NOT mention `TItem`

### Requirement: NX IR carries a function as a value
A `reference` node that names a function declaration SHALL be a value in any expression position,
not only as a call's callee: a property value, a sequence item, a function result, a field, a
call argument. A module whose node table contains such a reference in a position other than a
callee, or whose type table contains a function type, SHALL list a required feature naming
function-value support, `function-values-v1`, so a runtime that predates function values refuses
the module by name rather than by an unknown kind. A module with neither SHALL NOT list it. The
explained form SHALL render the reference by the function's module-qualified name.

#### Scenario: A function bound to a prop is a reference node
- **WHEN** NX source declares `let <Row Item:object />: string = "r" external component <List ItemTemplate?:<function Item:object />: string /> let root() = <List ItemTemplate={Row} />`
- **AND** NX IR is emitted for the program
- **THEN** the `ItemTemplate` property of the descriptor in `root` SHALL be a `reference` node
  naming `Row`
- **AND** the required feature list SHALL name function-value support

#### Scenario: A program without function values lists no feature
- **WHEN** NX IR is emitted for a program that declares functions and calls them but never binds
  one as a value and declares no function type
- **THEN** the required feature list SHALL be unchanged from before this feature existed

### Requirement: NX IR encodes a call of a function-typed value by name
An element invocation whose tag is a function-typed binding SHALL be encoded as a named-call node:
the callee expression, then each argument as a name and a node, sorted by argument name as a
property list is, since binding is by name and a stable order keeps the artifact canonical. A runtime SHALL
evaluate the callee to a function value and bind the arguments to that function's parameters by
name, dropping a name the function does not declare and failing with a diagnostic when a declared
parameter has no argument. A module containing a named-call node SHALL list the
`function-values-v1` feature. The existing positional call node SHALL be unchanged.

#### Scenario: Invoking a function-typed prop emits a named call
- **WHEN** NX source declares `component <Section Row:<function Item:object Index:int />: string /> = { <Row Item="a" Index=1 /> }`
- **AND** NX IR is emitted for the program
- **THEN** the body of `Section` SHALL contain a named-call node whose callee is the `Row` slot and
  whose arguments are `Item` and `Index`
- **AND** the explained text SHALL show the call with its argument names

#### Scenario: The conformance corpus covers function values
- **WHEN** the conformance corpus is inspected
- **THEN** it SHALL contain a program that declares a function type, binds a function to a
  function-typed prop and to a function-typed parameter, invokes the parameter by element and by
  call, and renders a function value in a function result
- **AND** the recorded results SHALL show the `Function` record for the rendered value and the
  results of both invocations

### Requirement: The prelude is an ordinary NX IR module
The prelude SHALL be an NX IR module like any library module: it SHALL have one image, under its
reserved identity, holding its own declarations, and a module that references a prelude declaration
— by constructing it, by a nominal type reference, or as a derived declaration's target — SHALL list
the prelude in its module table and reference the declaration through that slot. No prelude
declaration SHALL be copied into another module's image. The prelude's module-table entry SHALL
carry the compiler's prelude version, which names the prelude's contract rather than its text: it
SHALL be bumped when a declaration's shape changes and SHALL be left unchanged by an edit that
changes no declaration. A runtime's prelude SHALL therefore be accepted by that version and by the
declarations it holds. An emit request SHALL produce the prelude's image when it names the
prelude's identity, and an emit request for every module SHALL include it exactly when some module
of the program references it; the file name an image is written under SHALL be derived from the
identity as for any module and SHALL be valid on every supported platform. A module that references
no prelude declaration SHALL be byte-for-byte what it was before the prelude existed.

#### Scenario: A range expression links against the prelude
- **WHEN** NX source contains `let r() = { 1..5 }`
- **AND** NX IR is emitted for the program
- **THEN** the module table SHALL list the prelude's identity with the compiler's prelude version
- **AND** the body of `r` SHALL be a record construction of `Range` through the prelude's slot with `start`, `end` and `endInclusive`
- **AND** the module's own declaration list SHALL NOT include `Range`

#### Scenario: The prelude image holds the declaration once
- **WHEN** an emit request names the prelude's identity
- **THEN** the emitted image SHALL declare the record `Range` with `start` and `end` typed `object` and `endInclusive` typed `boolean`, and its derived update record and property union

#### Scenario: Every-module emission includes the prelude only when used
- **WHEN** an emit request lists every module of a program in which one module uses a range expression
- **THEN** the emitted images SHALL include the prelude's
- **AND** the same request for a program that uses no prelude declaration SHALL NOT include it

#### Scenario: An unused prelude leaves no trace
- **WHEN** NX IR is emitted for a program that uses no prelude declaration
- **THEN** each image SHALL be identical to the image the same compiler emitted before the prelude existed

### Requirement: NX IR encodes iteration over a range as its own node behind a required feature
A `for` whose iterable is a `Range` SHALL be encoded as a `forRange` node, kind `22`, with the
layout of a `for` node — item slot and name, optional index slot and name, iterable, body — whose
iterable evaluates to a `Range` record. A `for` whose iterable is a sequence SHALL be encoded as
before. A module that contains a `forRange` node SHALL list the required feature `ranges-v1`, and a
module that contains none SHALL NOT, whether or not it constructs a range. The `explain` text of a
`forRange` node SHALL read as a `for` over a range. Adding the node kind and the feature SHALL NOT
change the IR schema version.

#### Scenario: A range loop is a `forRange` node
- **WHEN** NX source contains `let squares() = { for i, n in 0..4 { i * i + n } }`
- **AND** NX IR is emitted for the program
- **THEN** the body of `squares` SHALL be a `forRange` node carrying the item `i`, the index `n`, a `Range` construction and the body
- **AND** the module SHALL list `ranges-v1` among its required features

#### Scenario: Building a range without iterating needs no feature
- **WHEN** NX source contains `let r() = { 1..=5 }` and no `for` over a range
- **THEN** the emitted module SHALL NOT list `ranges-v1`

#### Scenario: A list loop is unchanged
- **WHEN** NX source contains `let doubled(items:int+) = { for item in items { item * 2 } }`
- **THEN** the body SHALL be a `for` node and the module SHALL NOT list `ranges-v1`

### Requirement: A range is recognized by shape, and its element type is erased
A consumer of a `forRange` node SHALL recognize its iterable as a range by the value's shape — the
`$type` `Range` and the fields `start`, `end` and `endInclusive` — because a canonical value carries
`$type` as a bare declaration name with no module, and a nominal type carries no type arguments. The
prelude's `Range` SHALL therefore appear in an image with `start` and `end` typed `object`, and
`<Range T=int/>` and `<Range T=float64/>` SHALL be one declaration of one shape.

A `forRange` node SHALL be emitted only for an iterable whose declaration is the prelude's `Range`,
so a `Range` a module declares for itself SHALL NOT be iterated, and a host value SHALL be
normalized against the site's nominal type, which names its declaring module by slot. A host value
of another declaration that carries the same name and shape SHALL be accepted, as one is at any
site whose record name another module also declares.

An item's carrier SHALL be whatever numeric type the evaluating backend binds for the range's
bounds; NX's `int`, `int32` and `int64` SHALL be a distinction the checker draws rather than one an
image records. A backend whose numeric types do not separate them SHALL NOT be required to
reproduce them.

#### Scenario: The prelude's range is one declaration for every element type
- **WHEN** an image constructs `<Range T=int/>` and another constructs `<Range T=float64/>`
- **THEN** both SHALL reference the same prelude declaration, whose `start` and `end` are typed `object`

#### Scenario: A module's own `Range` is not iterated
- **WHEN** a module declares its own record named `Range` with integer `start` and `end` and a boolean `endInclusive`, and a `for` iterates a value of it
- **THEN** the compiler SHALL reject the loop rather than emit a `forRange` node

#### Scenario: The schema version is unchanged
- **WHEN** NX IR is emitted for a program that iterates a range
- **THEN** the image SHALL carry the same schema version as an image for a program without one

### Requirement: NX IR encodes occurrences with one sequence type kind and the presence operators as node kinds
The type table SHALL have one `seq` type kind that carries an occurrence — `?`, `+` or `*` — over an
exactly-one item type, and every type that carries an occurrence in source SHALL be encoded by it.
A property, field, state field or parameter declared `p?:T` SHALL be typed in IR by its read type,
`T?`, and one declared `p?:T+` by `T*`, as `optional-properties` defines the read type, so a runtime
normalizes an omitted field to the empty value without a second flag. The `array` and `nullable`
type kinds and the `null` node kind SHALL stay assigned, so no later kind reuses their numbers, and
a schema-`5` emitter SHALL NOT emit them; a schema-`5` reader that finds one SHALL report the
artifact as malformed. A `seq` type SHALL never have a `seq` item type, since no suffixed type is an
item type.

NX IR SHALL encode the presence operators of `presence-operators` as three node kinds, `exists`
(`x?`), `optionalMember` (`x?.m`) and `coalesce` (`x ?? y`), each newly assigned a number, and
SHALL encode the `{}` match pattern as a pattern form of the match construct. A module whose node
table contains any of the three nodes, or whose match arms contain a `{}` pattern, SHALL list the
required feature `occurrence-v1`, so a runtime that predates the operators refuses the module by
name; a module that contains none SHALL NOT list it, whether or not its type table contains a `seq`
type. There SHALL be no node kind for a `null` value and no node kind for a conditional operator.

An `ir explain` rendering SHALL print a `seq` type by its NX spelling (`string?`, `Person+`,
`object*`), a function type under an occurrence parenthesized, an `exists` node as `x?`, an
`optionalMember` node as `x?.m`, a `coalesce` node as `x ?? y` and the pattern as `{}`, and SHALL
NOT print the words "nullable" or "null" or the `[]` suffix.

#### Scenario: Each occurrence is one seq type
- **WHEN** NX source declares `type Book = { title:string subtitle?:string authors:Person+ tags?:string+ }`
- **AND** NX IR is emitted for the program
- **THEN** the field schema SHALL type `title` as `string`, `subtitle` as `string?`, `authors` as `Person+` and `tags` as `string*`
- **AND** each of `string?`, `Person+` and `string*` SHALL be a `seq` type entry written once in the type table
- **AND** the explained text SHALL show those spellings

#### Scenario: The retired kinds are not emitted
- **WHEN** NX IR is emitted for any program of the conformance corpus
- **THEN** no type entry SHALL use the `array` or `nullable` type kind and no node SHALL use the `null` node kind
- **AND** a hand-built schema-`5` image containing one of them SHALL be refused as malformed

#### Scenario: The presence operators are nodes behind a feature
- **WHEN** NX source contains `type Book = { author?:Person } let byline(b:Book): string = { if b.author? { b.author.name } else { b.author?.name ?? "anonymous" } }`
- **AND** NX IR is emitted for the program
- **THEN** the condition SHALL be an `exists` node over the `author` member access
- **AND** the `else` branch SHALL be a `coalesce` node whose left operand is an `optionalMember` node naming `name`
- **AND** the module SHALL list `occurrence-v1` among its required features
- **AND** the explained text SHALL show `b.author?`, `b.author?.name` and `?? "anonymous"`

#### Scenario: The empty pattern is a pattern form
- **WHEN** NX source contains `let f(o?:int): int = { if o is { {} => 0 else => o } }`
- **AND** NX IR is emitted for the program
- **THEN** the match construct SHALL carry an arm whose pattern is the `{}` pattern
- **AND** the module SHALL list `occurrence-v1`

#### Scenario: A program without the operators lists no feature
- **WHEN** NX source declares `type Book = { subtitle?:string tags:string+ }` and neither tests presence, steps through an optional nor writes a fallback or a `{}` pattern
- **AND** NX IR is emitted for the program
- **THEN** the required feature list SHALL NOT name `occurrence-v1`
- **AND** the type table SHALL still contain the `seq` types `string?` and `string+`

### Requirement: NX IR carries the result type of a function and a value
A function declaration SHALL carry its declared result type, and a value declaration its declared
type, each absent when the source declares none. A runtime SHALL normalize a function's result and
a value's initializer to that type as it normalizes an argument at a parameter, so the three
engines agree: `let g(): int+ = { 5 }` returns `[5]`, and a one-item sequence returned where `int?`
is declared is its item. Without a declared type the result SHALL be the body's value as it is,
as the interpreter leaves it.

A function declaration SHALL also carry result flags whose bit 0 is set when its result type,
declared or inferred, is a standalone `T?`. An entry call — a host evaluating the function, by
name or through a function value — SHALL then return an empty result to the host as `null`, as
`occurrence-types` requires; inside the program the empty value SHALL stay the empty array. An
`ir explain` rendering SHALL print a declared type as `: T` after the parameters and the flag as
`(optional result)`.

#### Scenario: A declared result is normalized in every engine
- **WHEN** NX source contains `let g(): int+ = { 5 }`, `let h(o?:int): int? = { for x in o { x } }`, `let k(xs?:int+): int+ = { xs ?? 5 }` and `type Ints = int+ let fives: Ints = { 5 }`
- **AND** the interpreter, the NX IR runtime and generated JavaScript each evaluate `g()`, `h(1)`, `k({})` and `fives`
- **THEN** every engine SHALL give `[5]`, `1`, `[5]` and `[5]`

#### Scenario: An empty optional result is null at the host
- **WHEN** NX source contains `let maybe(): string? = { if false { "a" } }` and `let inferred() = { if false { 1 } }`
- **AND** NX IR is emitted for the program
- **THEN** both function declarations SHALL set the optional-result flag and only `maybe` SHALL carry a declared result type
- **AND** `evaluateFunction` in the NX IR runtime SHALL return `null` for each
- **AND** the explained text SHALL show `function maybe(): string? (optional result) =` and `function inferred() (optional result) =`

### Requirement: NX IR leaves a parameter a call omits to the function
A function declaration SHALL carry each parameter's default as a node of the function's own frame,
absent when the parameter has none, emitted with the parameters before it bound so that it reads
them through their slots. A call node SHALL carry its arguments by position, where an absent
argument is a parameter the call left out, and the argument list MAY stop before the trailing
parameters it leaves out. A runtime SHALL fill each parameter a call leaves out in the callee: the
default, evaluated in the callee's frame after the parameters before it and normalized to the
parameter's type, or the empty value when the parameter is optional; a required parameter left
out SHALL be refused, naming it. The caller's image SHALL NOT contain the default, so a program
linked against a newer library receives that library's defaults. An `ir explain` rendering SHALL
print a default as `name: T = <node>` and an absent argument as `_`.

#### Scenario: A default travels with the function and a call leaves the parameter out
- **WHEN** NX source contains `let f(a:int, b:int = { a + 1 }, c?:int) = { a }` and `let r() = { <f a=1 c=2 /> }`
- **AND** NX IR is emitted for the program
- **THEN** `f`'s second parameter SHALL carry a default node and its first and third none
- **AND** the call in `r` SHALL carry three arguments, the second absent
- **AND** the explained text SHALL show `function f(a: int, b: int = (a add 1), c?: int)` and `f(1, _, 2)`

#### Scenario: Every engine fills a parameter from the callee's module
- **WHEN** the `parameter-defaults` corpus program is evaluated by the interpreter, the NX IR runtime and generated JavaScript
- **THEN** every engine SHALL give the recorded result for each entrypoint, including a default that reads a value private to the called function's module

### Requirement: NX IR encodes the function reference type
The type table SHALL have a function reference type kind, `anyFunction`, numbered `6`, whose entry
carries one operand, the result type. A prop, field, state field, parameter, result or value typed
`<function ... />: R` in source, written out or reached through an alias, SHALL be typed by that entry in IR: the top type SHALL NOT stand in for it, and a
function type with no parameters SHALL NOT stand in for it. An entry SHALL be written at most once
per result type in a module's type table and referred to by index, like every other type, and an
occurrence over it SHALL be a `seq` entry over it. A module whose type table holds an entry of the
kind SHALL list the required feature `function-reference-type-v1`, so that a runtime that predates
the type refuses the module by name rather than by an unknown kind, and a module without one SHALL
NOT list it. The schema version SHALL remain 5. A reader SHALL validate the entry's layout like any
other kind's: an entry of the kind with no operand, with more than one, or with an operand that is
not a type of the module's table SHALL be malformed. No node kind SHALL change: a function value
bound at such a site SHALL be the `reference` node `function-values` already uses, and a module
that binds one SHALL list `function-values-v1` on the terms that feature already has. The explained form of an artifact
SHALL render the type as `<function ... />: R`, parenthesized under an occurrence.

#### Scenario: A function reference field is typed by the function reference kind
- **WHEN** NX source declares `type AnyFn = <function ... />: object* type Tool = { fn:AnyFn extra?:AnyFn+ }`
- **AND** NX IR is emitted for the program
- **THEN** the field schema for `fn` SHALL be an `anyFunction` type entry whose operand is the `*` occurrence over the `object` primitive
- **AND** the field schema for `extra` SHALL be the `*` occurrence over that same entry
- **AND** the type table SHALL contain that `anyFunction` entry once
- **AND** the explained text SHALL show the types as `<function ... />: object*` and `(<function ... />: object*)*`

#### Scenario: A stated result is the entry's operand
- **WHEN** NX IR is emitted for `type Args = { q:string } type Tool = { build: <function ... />: Args any: <function ... />: object* }`
- **THEN** the type table SHALL hold two `anyFunction` entries, one whose operand is the nominal type `Args` and one whose operand is `object*`
- **AND** the explained text SHALL show `build` as `<function ... />: Args`

#### Scenario: A module that uses the type lists its feature
- **WHEN** NX IR is emitted for `type Tool = { fn: <function ... />: object* } let double(n:int): int = {n * 2} let root() = <Tool fn={double} />`
- **THEN** the required feature list SHALL name `function-reference-type-v1`
- **AND** SHALL name `function-values-v1`, because `double` is referenced as a value
- **AND** the `fn` property in `root` SHALL be a `reference` node naming `double`

#### Scenario: A module that does not use the type lists nothing new
- **WHEN** NX IR is emitted for a program that declares function types with stated parameters and binds functions to them but never writes `...`
- **THEN** the artifact SHALL be byte-for-byte what it was before the type existed

#### Scenario: A reader refuses a malformed entry
- **WHEN** an image's type table holds an entry of kind `6` with no operand, with two operands, or with an operand that indexes past the type table
- **THEN** every reader SHALL refuse the image with a diagnostic rather than reading it

#### Scenario: The conformance corpus covers the function reference type
- **WHEN** the conformance corpus is inspected
- **THEN** it SHALL contain a program that declares a record with a field typed `<function ... />: object*`, an optional one, one under `+`, and a field typed `<function ... />: R` for a record type `R`, binds functions of unlike signatures to them, passes a function reference value through a parameter and a result, compares two such values, and reads such a field's default
- **AND** the recorded results SHALL show the `Function` record for each rendered value

### Requirement: NX IR evaluation cost is counted in operations
The cost of evaluating NX IR SHALL be counted in operations, and the count SHALL be a function of
the linked images and the values the host supplies alone: two runtimes that evaluate the same
images with the same input and produce the same result SHALL count the same number of operations,
in the same order. The format's documentation SHALL state the rules, which are:

- Evaluating a node costs one operation, charged before any of the node's children is evaluated.
  A node is charged every time it is evaluated: a loop body once per iteration, a parameter's
  default each time a call leaves the parameter out, a field's default each time a construction
  leaves the field out, a state field's default each time it is evaluated, a value's initializer
  each time a `reference` node evaluates it, and an action handler's body each time a dispatch
  runs it.
- Testing one pattern of an `ifIs` arm costs one operation for the pattern's node, whether the
  runtime evaluates that node or, for a pattern that names a union case with fields, reads the
  case it names. A pattern that is not tested because an earlier pattern of its arm matched costs
  nothing.
- The callee of a `call` or `namedCall` costs one operation for its node, whether the runtime
  evaluates that node or, for a `reference` to a function, resolves the function in place.
- Placing one item in a sequence a node builds costs one operation, charged as the item is placed,
  after the child that produced it has been evaluated. The sequences a node builds are the value of
  an `array` node, the value of a `for` or `forRange` node, and the content list of a `record`,
  `unionCase`, `element` or `component` node. A child whose value is itself a sequence places each
  of its items; a child whose value is empty places none. A `for` whose iterable is one item
  rather than a sequence yields its body's value as it stands, builds no sequence and so places
  nothing. A call that binds a sequence to a content parameter builds the list anew, an item that
  is itself a sequence contributing its items, before the list is checked against the parameter's
  type: each item of the argument costs one, or, when it is itself a sequence of several items,
  one for each item it contributes. An item that is empty contributes nothing and costs one.
- Producing the string of a `binary` node with the `concat` operator costs one operation for every
  64 UTF-16 code units of that string, rounded down, charged after both operands have been
  evaluated and before the string is produced.
- Checking a value against a declared type costs one operation for each value checked against a
  type that is not a sequence type, charged before that value is checked. A sequence type costs
  nothing itself: each item is checked against the item type, and no items cost nothing. A
  sequence of one item that reaches a type that is not a sequence type is read as its item, and
  costs the one check of that item. A record, or a union case with fields, costs one and then the
  check of each field value against the field's type, a default the check fills in included,
  after the nodes of the default have been charged as any nodes are; a field that is left out and
  has no default is not checked. A value at the type `object` is one value whatever it holds. The
  checks are the ones evaluation makes: an argument or a default against its parameter, a
  function's result against its declared result type, a value declaration's value against its
  declared type, each field of a `record`, `unionCase` or `component` construction against the
  field's type, and every value the host supplies to a typed site.
- Comparing two values costs one operation for the pair, charged before they are compared, for
  the `eq` and `ne` operators, for a pattern matched by equality, and for each field the `diff`
  intrinsic compares, a field one record does not hold comparing as the empty value. A pattern is
  matched by equality unless the pattern or the scrutinee is the empty value, or both are records
  and the pattern names a type; those are decided by emptiness or by the type name and compare
  nothing. An integer outside JavaScript's safe range, in a runtime that holds one, is an integer
  for this rule as any other is: a pattern that is one is matched by equality, and the comparison
  is paid for. Beyond the pair:
  - two sequences of one length compare their items pair by pair, in order, up to and including
    the first pair that differs; the pairs after it are not compared and cost nothing;
  - a sequence of one item and a value that is not a sequence compare the item with the value;
  - two records cost one more for every 64 UTF-16 code units of the shorter of their type names
    when both have one, and two records of different types compare nothing further. Two records
    of one type are lined up by field name: every field name either of them holds costs one more
    for every 64 code units, for each record that holds it; a name both hold compares the pair of
    its values; and a name only one holds costs one operation. A record holds a field when it
    stores a value for it: an optional field that was left out or is empty is not held, and a
    field an update record clears is;
  - two action handlers made by one node that captured as many slots compare their captured
    values slot by slot, in order, up to and including the first slot that differs. A handler
    captures the slots of its frame as they stood when it was made, and a slot that was not bound
    then compares only with another that was not;
  - two strings cost one more for every 64 UTF-16 code units of the shorter, rounded down.

  The items of a sequence and the slots of a frame are in one order in every runtime, so a
  comparison that stops at the first difference among them costs the same everywhere. The fields
  of a record are not: every pair of field values two records line up is compared, and every name
  counted, whether or not an earlier one already differed, so that the cost of a comparison
  depends on the two values and not on the order a runtime holds a record's fields in. The result of a comparison SHALL NOT depend on whether a
  budget is set, nor on whether the two values are held as one.
- Writing a value for the host costs one operation for each value written, charged before it is
  written, because every value takes a place in what is written: a sequence is one, the empty
  value included, and then its items; a record is one, and one more for every 64 UTF-16 code
  units of its type name and of each field name, and then its field values; a function value and
  an action handler are one each; and a string is one and one more for every 64 UTF-16 code
  units, rounded down. Only a value the runtime itself made is a function value or an action
  handler: a record a host passes that has the type name or the internal marking of one is a
  record, entered and paid for like any other. So is the record canonical JSON spells an integer
  outside JavaScript's safe range with: at `object`, where no type says what it is, no runtime
  takes it for a number. A `null` inside a value the host passed at the type `object` is the empty
  value: one value where it is written, and one item where a list that holds it is bound to a
  content parameter. Every value an evaluation returns is written: a function's
  result, a descriptor, rendered output, each effect, and component state. A value held once and
  reached from several places is written once for each place.

A node that is not evaluated SHALL cost nothing: the branch a conditional does not take, the right
operand of an `and`, `or` or `coalesce` that the left operand decides, and the body of an action
handler when the handler is created. Work a runtime does that is none of the above — preparing and
linking images, reading the host's input before it is checked, an update intrinsic's own work
apart from the comparisons of `diff`, collecting a dispatch's effects, and assigning handler
tokens — SHALL cost nothing, and SHALL NOT
grow with the size of a value, as it is held or as it unfolds when walked as a tree: a runtime
that walks a value for a purpose of its own SHALL visit a value reached from several places once,
and SHALL NOT walk again what an evaluation did not change.

A runtime MAY offer the host a budget of operations. A runtime that does SHALL charge by these
rules and SHALL fail an evaluation at the first charge that would take the count above the budget,
without performing the operation charged. The failure SHALL name the declaration the charge
belongs to: for a node, an item placed, a `concat` or a comparison, the declaration the node
belongs to, with the node's span when the image carries its debug section; for a check against a
type, the declaration whose parameter, result, field or value the check is for, which for a field
of an update record is the update record; for an item bound to a content parameter, the function
called; and for a value written for the host, the function or component the call evaluated. The
last three carry no span.

#### Scenario: A straight-line function pays for its arguments, its nodes and its result
- **WHEN** an image of `let add(a:int, b:int) = { a + b }` is evaluated at entrypoint `add` with `1` and `2`
- **THEN** the evaluation SHALL cost six operations: two for the arguments checked against `int`,
  three for the `binary` node and its two `slot` operands, and one for the number written

#### Scenario: A loop pays for its body and for its items
- **WHEN** an image of `let root() = { for i in 0..4 { i * i } }` is evaluated at entrypoint `root`
- **THEN** the evaluation SHALL cost one operation for the `forRange` node, seven for evaluating
  its iterable once (the `record` node of the `Range`, its three literals and the three fields
  checked), three for each of the four evaluations of the body, four for the four items placed in
  the result, and five for the list and its four numbers written: 29 in all

#### Scenario: An untaken branch costs nothing
- **WHEN** an image of `let pick(flag:boolean) = { if flag { 1 } else { for i in 0..1000 { i } } }` is evaluated with `true`
- **THEN** the evaluation SHALL cost eight operations: the argument checked, the `if` node, the
  `slot` node, the `array` node the then-branch `{ 1 }` is emitted as, its literal, the one item
  placed, and the one-item list and its number written

#### Scenario: A call pays for its callee however it is resolved
- **WHEN** an image of `let one() = { 1 }` and `let root() = { one() }` is evaluated at entrypoint `root`
- **THEN** the evaluation SHALL cost four operations: the `call` node, its `reference` callee, the
  literal that is the body of `one`, and the number written

#### Scenario: A spliced list pays for every item it places
- **WHEN** a function takes `xs:int+` and its body is the list `{ xs xs }`
- **AND** it is evaluated with a list of 500 integers
- **THEN** the evaluation SHALL cost 500 operations for the items of the argument checked, one for
  the `array` node, two for the two `slot` nodes, one thousand for the items placed, and 1,001
  for the list and its items written: 2,504 in all

#### Scenario: A long concatenation pays for its length
- **WHEN** a function concatenates two strings of 1,000 UTF-16 code units each
- **THEN** the `concat` SHALL cost one operation for its node and 31 for the 2,000 code units of
  its result, besides the cost of its operands
- **AND** writing the result for the host SHALL cost 32: one for the string and 31 for its length

#### Scenario: A host value is checked and written value by value
- **WHEN** a host passes a list of 10,000 integers to a function `let same(xs:int+) = { xs }`
- **THEN** the evaluation SHALL cost 20,002 operations: 10,000 for the items checked against
  `int`, one for the `slot` node, and 10,001 for the list and its items written

#### Scenario: A check against a type visits every value of a record
- **WHEN** an image of `type Point = { x:int y:int }` and `let sum(p:Point): int = { p.x + p.y }` evaluates `sum(<Point x={1} y={2} />)`
- **THEN** building the record SHALL cost two operations for its two fields checked, besides its nodes
- **AND** passing it to `sum` SHALL cost three: one for the record and one for each field
- **AND** the result of `sum` SHALL cost one, checked against `int`

#### Scenario: A comparison costs every pair it lines up
- **WHEN** an image evaluates `<Point x={1} y={2} /> == <Point x={3} y={2} />`
- **THEN** the comparison SHALL cost three operations: one for the two records and one for each
  pair of fields, the second although the first already differs

#### Scenario: Two lists are compared up to the first pair that differs
- **WHEN** two lists of 100,000 integers that differ in their first item are compared
- **THEN** the comparison SHALL cost two operations: the pair of lists and the first pair of items
- **AND** two equal lists of three integers SHALL cost four, and two lists of different lengths one

#### Scenario: Records inside a list are compared to the end
- **WHEN** a list whose first item is `{ "a": 1, "b": 2 }` is compared with one whose first item is `{ "a": 9, "b": 9 }`
- **THEN** the comparison SHALL cost four operations: the pair of lists, the pair of records, and both pairs of fields, and no later item SHALL be compared

#### Scenario: A comparison of strings costs their length
- **WHEN** an image compares two strings of 1,000 UTF-16 code units each
- **THEN** the comparison SHALL cost 16 operations: one for the pair and 15 for the code units
- **AND** comparing one of them with a string of 63 code units SHALL cost one

#### Scenario: A value held twice is written twice
- **WHEN** a function takes `p:Point` and returns the list `{ p p }`
- **THEN** writing its result for the host SHALL cost seven operations: one for the list and three for each of the two records

#### Scenario: A list bound to a content parameter is placed again at every call
- **WHEN** a function `let ignore(content c:object): int = { 1 }` is called with a list of 100 integers the host passed at `object`
- **THEN** the call SHALL cost 100 operations for the items bound, besides its nodes and the two checks of the list and the result
- **AND** 16,000 such calls with a list of 20,000 SHALL be refused by a budget of one hundred thousand, naming `ignore` and carrying no span

#### Scenario: A one-item sequence at a single-value type is one check
- **WHEN** a function `let f(x:int) = { x }` is called with the one-item list `{ 5 }`
- **THEN** checking the argument SHALL cost one operation, not two

#### Scenario: A field left out with no default is not checked
- **WHEN** an image of `type Opt = { a:int = 1 b?:int c?:string }` evaluates `<Opt c="q" />`
- **THEN** building the record SHALL cost two operations for fields checked, the default of `a` and `c`, and none for `b`

#### Scenario: A pattern decided by emptiness or by type compares nothing
- **WHEN** an `ifIs` tests the empty pattern against the number 4, or a pattern that names a union case with fields against a record
- **THEN** testing the pattern SHALL cost one operation for its node and none for a comparison

#### Scenario: Two records compare the fields they hold
- **WHEN** an image evaluates `<Opt c="q" /> == <Opt c="q" />`
- **THEN** the comparison SHALL cost three operations: the pair of records and the pairs of `a` and of `c`, and none for `b`, which neither holds

#### Scenario: A name only one record holds costs one
- **WHEN** a host passes `{ "a": 1, "b": 2 }` and `{ "a": 1, "c": 2 }` at `object` and they are compared
- **THEN** the comparison SHALL cost four operations: the pair of records, the pair of values under `a`, and one for each of `b` and `c`
- **AND** comparing `{ "a": 1 }` with `{ "a": 1, "b": 2 }`, in either order, SHALL cost three

#### Scenario: A wide record is not compared for the price of one operation
- **WHEN** a host passes an object of 8,000 fields at `object` and it is compared with `{ "x": 1 }`, in either order
- **THEN** the comparison SHALL cost more than 8,000 operations
- **AND** 16,000 such comparisons SHALL be refused by a budget of one hundred thousand

#### Scenario: Records of different types compare nothing further
- **WHEN** a record of type `A` with two fields is compared with a record of type `B`
- **THEN** the comparison SHALL cost one operation

#### Scenario: Every item bound to a content parameter costs one
- **WHEN** a host passes the list `[null]` at `object` and it is bound to a content parameter
- **THEN** binding it SHALL cost one operation, `[1, null]` two, `[[null, null], 1]` three, and `[[], [], [1, 2, 3]]` five

#### Scenario: Two handlers compare what they captured
- **WHEN** two action handlers made by one node in a frame with two bound slots are compared
- **THEN** the comparison SHALL cost three operations when they captured equal values: the pair of handlers and each pair of captured values
- **AND** SHALL cost two when the values in their first slot differ, whether or not those in the second differ too

#### Scenario: A comparison gives one result with or without a budget
- **WHEN** a record that holds a NaN is compared with itself
- **THEN** the result SHALL be `false` under any budget and under none, in every runtime

#### Scenario: A name the host supplied costs its length
- **WHEN** a host passes, at `object`, an object whose one key is 1,048,576 UTF-16 code units long, and a function returns it 20 times in a list
- **THEN** writing the result SHALL cost one operation for the list and 20 times 16,386: one for each object, 16,384 for its key, and one for its value

#### Scenario: Text a host puts in a form the runtime gives a meaning to costs its length
- **WHEN** a host passes, at `object`, `{ "$type": "nx.int", "value": <a string of 1,048,576 UTF-16 code units> }`, or an object with the type name or the internal marking of an action handler or a function value that holds such a string, and a function returns it 20 times in a list
- **THEN** writing the result SHALL cost more than 20 times 16,384 operations in every runtime
- **AND** comparing the value with itself SHALL cost at least 16,384

#### Scenario: The JSON form of a wide integer is a record at object
- **WHEN** a host passes `{ "$type": "nx.int", "value": "1152921504606846976" }` at `object`
- **THEN** in every runtime it SHALL be written back as that record, for two operations: the record and its string
- **AND** a record of that type name that holds 8,000 further fields SHALL be paid for by its fields where it is written, and compared with a value of another type for one pair
- **AND** in a runtime that holds 64-bit integers, it SHALL NOT be equal to the program's integer 1152921504606846976 and SHALL NOT match it as a pattern, the comparison costing one pair, while that integer passed in the runtime's own form SHALL be equal to it

#### Scenario: A wide integer pattern is matched by equality
- **WHEN** a runtime that holds 64-bit integers matches a value of 1152921504606846977 against the pattern 1152921504606846976
- **THEN** the pattern SHALL NOT match
- **AND** testing it SHALL cost the pattern and one pair compared, as a pattern of 1 against a value of 2 does

#### Scenario: An empty value written is one value
- **WHEN** a host passes `{ "a": null, "b": [null, null], "c": [] }` to `let passObject(o:object): object = { o }`
- **THEN** the evaluation SHALL cost nine operations: the argument and the result checked, the `slot` node, and six values written, which are the record, the empty value under `a`, the list under `b` and its two empty items, and the empty list under `c`

#### Scenario: A value made of empty values costs its length each time it is written
- **WHEN** a host passes, at `object`, an object holding a list of 20,000 `null`s, and a function returns it 500 times in a list
- **THEN** each copy written SHALL cost 20,002 operations, and the evaluation SHALL be refused by a budget of one hundred thousand

#### Scenario: A value held and never walked costs nothing more
- **WHEN** a function builds an element that holds one value in two properties, forty levels deep, and returns a number
- **THEN** the evaluation SHALL cost what building it costs, a few hundred operations
- **AND** the same value compared with another like it, or returned to the host, SHALL be refused
  by a budget of one hundred thousand, since walked as a tree it is 2^40 values

### Requirement: The conformance corpus records operation counts
Every program of the conformance corpus SHALL record, beside its expected results, the operation
count of each entrypoint it names and, for each lifecycle it names, the operation count of
initialization and of each dispatched batch, where a batch's count covers every handler the batch
runs and the render that follows. The regeneration command that writes the expected results SHALL
write the counts in the same run. Every runtime that offers an operation budget SHALL check, for
each recorded count, that the evaluation succeeds with its recorded result under a budget equal to
the count and fails with the resource-limit diagnostic under a budget one less. The corpus SHALL
hold at least one entrypoint for each charging rule: an `ifIs` whose arm tests more than one
pattern, a call whose callee is a function `reference` and one through a function value, a
sequence built by splicing a list, a `for`, a `forRange`, a content list, a `concat` whose
result is at least 64 UTF-16 code units long, a record checked against its type as an argument, a
comparison of two records that differ in their first field, a comparison of two lists that differ
in their first item, a comparison of two handlers of one node whose captured values differ in the
first slot and in a later one, a comparison of two strings of at least 64 UTF-16 code units, a
result that holds one record in two places, a list bound to a content parameter, and a comparison
of a constructed record with one `apply` returned from a constructed update record. For each of those entrypoints the corpus SHALL
also record a budget below its count together with the declaration and the span of the diagnostic
that budget produces, and every runtime that offers an operation budget SHALL check that it fails
there.

#### Scenario: A recorded count is exact
- **WHEN** a runtime evaluates a corpus entrypoint under a budget equal to its recorded count
- **THEN** the evaluation SHALL yield the recorded result
- **AND** the same evaluation under a budget one less SHALL fail with `nx-ir-resource-limit`

#### Scenario: A lifecycle step has a count of its own
- **WHEN** a runtime dispatches a batch of a corpus lifecycle under a budget equal to the count
  recorded for that batch
- **THEN** the dispatch SHALL yield the recorded rendered output and effects
- **AND** the same dispatch under a budget one less SHALL fail with `nx-ir-resource-limit`

#### Scenario: A smaller budget stops at the recorded node
- **WHEN** a runtime evaluates a corpus entrypoint, from the image that carries its debug section,
  under a budget the corpus records a failure for
- **THEN** the diagnostic SHALL name the recorded declaration and carry the recorded span

#### Scenario: A change in a runtime's counting is caught
- **WHEN** a runtime is changed so that it charges a node twice, or does not charge an item it
  places in a sequence
- **THEN** that runtime's corpus test SHALL fail naming the program and the entrypoint

#### Scenario: Counts are regenerated with the results
- **WHEN** the corpus is regenerated after an intended change to the emitter
- **THEN** the operation counts SHALL be rewritten by the same command
- **AND** a count that changed SHALL appear in the diff as a changed number beside its entrypoint
