## Purpose

Defines the Rust crate that prepares, links and evaluates NX IR images without the NX compiler, so
that a Rust host can run compiled NX programs with the same results as the TypeScript IR runtime.

## ADDED Requirements

### Requirement: The Rust IR runtime does not depend on the NX compiler
The Rust IR runtime SHALL be a crate whose dependency closure contains the NX IR format crate and
the NX value crate and no crate that parses, lowers, type-checks or emits NX. The NX IR format —
its constants, kind tables, image writer, validating reader and explainer — SHALL be available from
a crate that the emitter and the runtime both depend on and that depends on neither.

#### Scenario: A host builds the runtime without the compiler
- **WHEN** a Rust project depends on the IR runtime crate alone
- **THEN** the build SHALL NOT compile the NX syntax, HIR, type-checking, interpreter or codegen
  crates

#### Scenario: The emitter and the runtime share one format implementation
- **WHEN** the emitter writes an image and the runtime reads it
- **THEN** both SHALL use the same schema version, runtime ABI, feature names and kind numbers from
  the format crate

### Requirement: Rust runtime prepares NX IR modules
The Rust runtime SHALL expose an API that accepts an NX IR image as bytes and returns a prepared
module or an error. Preparation SHALL validate, before anything else is read, everything
`nx-ir-format` requires a reader to validate: the magic and schema version, the recorded length,
every section, offset array and string range, the string encoding, the runtime ABI, the required
features, and every entry of every table against its layout. Preparation SHALL read the image in
place without copying its tables. A prepared module SHALL expose its identity, its version, and its
function and component entrypoints by public name, and SHALL be shareable across threads and across
any number of linked programs. A prepared module whose module table names only itself, or only
itself and the prelude, SHALL be usable as a program through a single prepare-and-link call.

#### Scenario: A valid image prepares
- **WHEN** a caller prepares a valid schema-5 image whose runtime ABI the runtime supports
- **THEN** the runtime SHALL return a prepared module
- **AND** the module SHALL list the image's function and component entrypoints by name

#### Scenario: An unsupported schema, ABI or feature is refused by name
- **WHEN** a caller prepares an image of another schema version, an image naming another runtime
  ABI, or an image listing a required feature the runtime does not know
- **THEN** preparation SHALL return an error whose diagnostic names the version, the ABI or the
  feature
- **AND** SHALL NOT return a prepared module

#### Scenario: A damaged image is an error, never a panic
- **WHEN** a caller prepares an image cut at any four-byte boundary, or an image in which any one
  cell has been overwritten
- **THEN** preparation SHALL either return an error with a diagnostic or return a prepared module
- **AND** SHALL NOT panic

#### Scenario: A self-contained image is a program
- **WHEN** a caller prepares an image whose module table holds only its own module, or its own
  module and the prelude
- **THEN** the caller SHALL be able to evaluate its entrypoints without supplying a resolver

#### Scenario: Evaluating an unlinked module is refused
- **WHEN** a caller evaluates an entrypoint of a prepared module whose table names another module
  and that has not been linked
- **THEN** evaluation SHALL return the `nx-ir-unlinked` diagnostic

### Requirement: Rust runtime links an entry module against prepared modules
The Rust runtime SHALL expose a linking step that takes a prepared entry module and a resolver the
host supplies, asks the resolver for each module in the entry's module table by identity, and
returns a linked program. Linking SHALL fail naming the module when the resolver supplies nothing
for it, SHALL fail naming the module and both versions when a resolved module's version differs
from the version the entry recorded unless the host opts into linking across versions, and SHALL
fail naming the module and the declaration when a referenced declaration is absent from the
resolved module. Linking SHALL NOT copy a prepared module.

#### Scenario: A snippet links against a prepared catalog
- **WHEN** a host prepares a catalog image once and links a snippet image that names it at the same
  version
- **THEN** linking SHALL succeed
- **AND** evaluating the snippet's `root` SHALL construct descriptors of the catalog's components

#### Scenario: A version mismatch is refused unless the host allows it
- **WHEN** the snippet recorded catalog version `9` and the resolver returns version `10`
- **THEN** linking SHALL fail with `nx-ir-link-version` naming the catalog, `9` and `10`
- **AND** linking SHALL succeed when the host opts into linking across versions and every
  referenced declaration exists

#### Scenario: A missing module or declaration is refused
- **WHEN** the resolver returns nothing for a module in the table, or the resolved module lacks a
  declaration the entry references
- **THEN** linking SHALL fail with `nx-ir-link-missing-module` naming the module, or
  `nx-ir-link-missing-declaration` naming the module and the declaration

#### Scenario: One prepared module serves programs on several threads
- **WHEN** a host links many snippet images against one prepared catalog and evaluates them on
  different threads
- **THEN** the catalog SHALL have been prepared once
- **AND** each program SHALL evaluate independently

### Requirement: Rust runtime supplies the prelude module itself
The Rust runtime crate SHALL carry the compiled image of the prelude that its compiler release
emits, and linking SHALL supply it for the prelude's reserved identity whenever the host's resolver
returns no module for that identity, at any depth of the link. A module the host's resolver does
return for that identity SHALL be used instead. The built-in prelude SHALL be prepared at most once
per process. The version and missing-declaration checks SHALL apply to the prelude as to any other
module. The command that regenerates the TypeScript runtime's prelude image SHALL regenerate the
Rust runtime's image in the same run, and a test SHALL fail when either is stale.

#### Scenario: A program that builds a range needs no resolver
- **WHEN** a host prepares a program from one image whose module table lists only the prelude
- **THEN** preparation SHALL succeed and the program SHALL evaluate

#### Scenario: A host overrides the prelude
- **WHEN** the host's resolver returns a prepared module for the prelude's identity
- **THEN** linking SHALL use that module and SHALL NOT use the built-in one

#### Scenario: The checked-in image is current
- **WHEN** the prelude's source changes and the Rust runtime's image is not regenerated
- **THEN** a test SHALL fail naming the command that regenerates it

### Requirement: Rust runtime evaluates functions with the TypeScript runtime's results
The Rust runtime SHALL evaluate a function entrypoint by name with positional arguments, and SHALL
call the function a `Function` record names with arguments keyed by parameter name. For every node
kind, type kind, constant kind, declaration kind and required feature of schema 5, the canonical
value the Rust runtime returns SHALL equal the canonical value `typescript-ir-runtime` requires for
the same images and the same inputs, and where that capability requires a diagnostic the Rust
runtime SHALL fail with a diagnostic of the same code naming the same item. The one exception is
an integer outside JavaScript's safe range, as an operand or as a result: the Rust runtime SHALL
compute it as a 64-bit integer, wrapping at 64 bits, where the TypeScript runtime refuses the
operand or loses precision. A name that is not a
function entrypoint SHALL fail with the missing-entrypoint diagnostic. An entry call SHALL return
the host `null` for an empty result of a function whose declaration sets the optional-result flag.
A call by `Function` record SHALL drop an argument the function does not declare and SHALL fail
naming the first declared parameter the arguments lack.

#### Scenario: A root function evaluates
- **WHEN** a prepared program contains `let root() = { 1 + 2 }` and a caller evaluates `root`
- **THEN** the runtime SHALL return the integer `3`

#### Scenario: A declared result is normalized and an empty optional result is null
- **WHEN** a prepared program contains `let many(): int+ = { 5 }`, `let none(): int? = { if false { 1 } }` and `let caller(): int* = { none() }`
- **AND** a caller evaluates each as a function entrypoint
- **THEN** the runtime SHALL return a one-element sequence holding `5`, the host `null`, and the
  empty sequence

#### Scenario: A cross-module call resolves through the link
- **WHEN** a linked program's root function calls a function of another module
- **THEN** the call SHALL resolve through the module-qualified reference
- **AND** the result SHALL equal the TypeScript runtime's for the same images

#### Scenario: A declaration that is not an entrypoint is not callable by name
- **WHEN** a caller evaluates by name a function the image does not list among its function
  entrypoints
- **THEN** the runtime SHALL fail with `nx-ir-missing-entrypoint`

#### Scenario: A function record is called by parameter name
- **WHEN** a host calls the function `{ "$type": "Function", "module": "main.nx", "name": "Row" }`
  with arguments `Item`, `Index` and `Extra`, and `Row` declares `Item` and `Index`
- **THEN** the runtime SHALL invoke `Row` with `Item` and `Index` bound and `Extra` dropped
- **AND** the same call without `Index` SHALL fail with a diagnostic naming `Index`

### Requirement: Rust runtime exchanges values with the host as NxValue
The Rust runtime SHALL accept arguments, props, state, patches and batch entries as `NxValue` and
SHALL return results as `NxValue`. Boundary values SHALL be normalized and validated against the
IR's schema metadata with the rules `typescript-ir-runtime` states for JSON boundary values,
including occurrences, records and their discriminators, abstract records, unions, enums, property
unions, update records and `Function` records. An integer site SHALL accept either integer variant
of `NxValue`, and a float site SHALL accept either float variant and either integer variant. The
runtime SHALL return integers as the 64-bit integer variant and floats as the 64-bit float variant.
The host `null` SHALL be read as the empty value where the site admits zero and rejected where it
does not, an `object` site admitting it as the empty list, and SHALL be written only for a cleared field of an update record and for an empty
optional result of an entry call. An integer outside JavaScript's safe range SHALL be returned as
the 64-bit integer variant, and its canonical JSON wrapper `{ "$type": "nx.int", "value": "<digits>" }`
SHALL be accepted at an integer site. Serialized to JSON, a value the Rust runtime returns SHALL
equal the canonical value the TypeScript runtime returns, comparing numbers by value, ignoring key
order, and reading such an integer in either spelling.

#### Scenario: Either integer width is accepted
- **WHEN** a host passes a 32-bit integer value and then a 64-bit integer value holding `3` for a
  parameter typed `int`
- **THEN** both calls SHALL succeed with the same result

#### Scenario: Host absence decodes by the declared occurrence
- **WHEN** a host initializes a component declaring `subtitle?:string items:string+` with
  `subtitle` set to the host `null`
- **THEN** `subtitle` SHALL be the empty value
- **AND** the same call with `items` set to the host `null` or the empty sequence SHALL fail with a
  diagnostic naming `items`

#### Scenario: Malformed host input is rejected with the boundary codes
- **WHEN** a host supplies an unknown field, omits a required field, or supplies a record whose
  `$type` names a type that does not extend the declared one
- **THEN** the runtime SHALL fail with `nx-ir-boundary-field` or `nx-ir-boundary-type` naming the
  field or the type

#### Scenario: Output agrees with the TypeScript runtime
- **WHEN** the same images are evaluated with the same inputs by both runtimes
- **THEN** the JSON form of the Rust runtime's result SHALL equal the TypeScript runtime's result

### Requirement: Rust runtime runs the component lifecycle
The Rust runtime SHALL construct a component descriptor, initialize a component from props,
evaluate a component from props and explicit state, normalize a complete state, apply a state patch
or a component update record to a state, and dispatch an ordered batch of emitted actions and
handler invocations against an instance. Each operation SHALL behave as `typescript-ir-runtime`
requires of the corresponding operation: content binding by declared occurrence, handler properties
carried as `ActionHandler` records, a parent instance supplying the handlers its tokens name, a
supplied state replacing the initial state, live state reads for a handler the component's own body
bound, update records of the instance's component patching its state and every other result
collected as an effect, one re-render after the batch, and fresh tokens on every lifecycle render.
Tokens SHALL be `h<generation>-<n>` assigned by the same walk, so the same program with the same
props and batches yields the same tokens in every NX runtime. Output of pure evaluation SHALL carry
no tokens. No operation SHALL modify an instance it was given, and a failed dispatch SHALL leave
the instance usable.

#### Scenario: Initialization returns rendered output, state and an instance
- **WHEN** a caller initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`
- **THEN** the rendered `Button`'s `onTapped` SHALL be an `ActionHandler` record with action
  `Button.Tapped` and token `h1-1`
- **AND** the state SHALL hold `count` equal to `0`

#### Scenario: A handler invocation patches state and patches compound within a batch
- **WHEN** a caller dispatches two invocations of token `h1-1` with `<Button.Tapped />` in one
  batch against that instance
- **THEN** dispatch SHALL return state with `count` equal to `2`, no effects, and rendered output
  whose token is `h2-1`

#### Scenario: A parent-bound handler's results are effects
- **WHEN** a caller initializes `SearchBox` from a parent's rendered descriptor with the parent's
  instance, and dispatches `<SearchBox.SearchSubmitted searchString="docs" />`
- **THEN** dispatch SHALL return the parent handler's result as the only effect
- **AND** the `SearchBox` state SHALL be unchanged

#### Scenario: A stale token is rejected and the instance survives
- **WHEN** a caller dispatches a batch whose first entry patches state and whose second names a
  token from an earlier render
- **THEN** dispatch SHALL fail with `nx-ir-handler-token`
- **AND** a valid batch dispatched afterwards against the same instance SHALL observe the original
  state

#### Scenario: An invalid state or patch is rejected
- **WHEN** a caller evaluates a component with a state holding an undeclared field, or applies a
  patch whose value does not match the state field's type
- **THEN** the runtime SHALL fail with a diagnostic naming the field
- **AND** SHALL NOT return a partially updated state

#### Scenario: Pure evaluation carries no tokens
- **WHEN** a caller evaluates `Counter` from explicit state
- **THEN** the rendered `onTapped` SHALL be an `ActionHandler` record without a token

### Requirement: A Rust runtime component instance can be stored and reused
A component instance SHALL be serializable, and an instance restored from its serialized form SHALL
be accepted by dispatch and by child initialization against a program linked from the same images
that produced it, with the same results as the original instance. An instance SHALL record which
program it belongs to, by the images the program was linked from and not only by their source.
The serialized form SHALL hold a value that several handlers captured once, and SHALL nest no
deeper for a value that nests more deeply. Restoring an instance SHALL require the program it is
restored for and SHALL fail, with a diagnostic, when the serialized form belongs to another
program, when its contents do not name handlers of that program, or when its props, state or
handler properties are not ones the component accepts, so that no instance a caller holds is
unchecked. Dispatch and child
initialization SHALL refuse, with a diagnostic, an instance that belongs to another program, and
SHALL NOT re-validate the contents of an instance on each use. A handler a child instance received
from its parent SHALL compare equal to the handler the parent holds.

#### Scenario: A restored instance dispatches
- **WHEN** a host initializes a component, serializes the instance, restores it in a process that
  has prepared the same images, and dispatches a token from the stored rendered output
- **THEN** dispatch SHALL return the same rendered output, effects and state as dispatching against
  the original instance

#### Scenario: An instance from another program revision is refused
- **WHEN** a host restores a serialized instance for a program whose entry module was emitted
  from a different source than the one that produced it
- **THEN** restoring SHALL fail with a diagnostic
- **AND** dispatching an in-memory instance against such a program SHALL fail with a diagnostic
  without evaluating any handler

#### Scenario: An altered instance is refused
- **WHEN** a serialized instance names a module, declaration or node that is not a handler of the
  program, or holds a state field the component does not declare or a value of the wrong type
- **THEN** restoring SHALL fail with a diagnostic and SHALL NOT panic

#### Scenario: An instance of other images from the same source is refused
- **WHEN** a host restores or dispatches an instance against a program linked from images that
  were emitted from the same source but differ in their bytes
- **THEN** the runtime SHALL fail with a diagnostic without evaluating any handler

#### Scenario: A stored instance holds shared values once
- **WHEN** a component renders a thousand rows whose handlers each capture the same list
- **THEN** the serialized instance SHALL hold that list once
- **AND** a deserializer that refuses deeply nested input SHALL restore an instance whose state
  nests as deeply as the boundary admits

#### Scenario: A long batch cannot exhaust the stack
- **WHEN** a batch nests a component's state one level deeper with each entry
- **THEN** dispatch SHALL fail with `nx-ir-resource-limit` at the entry that crosses the value
  nesting limit

### Requirement: Rust runtime exports the update helpers
The Rust runtime SHALL export `apply`, `merge`, `diff` and `changed` as functions over host-held
values, with the semantics `update-records` specifies and `typescript-ir-runtime` requires of its
exported helpers, and SHALL evaluate the same four as intrinsic calls inside a program.

#### Scenario: Apply replaces present fields only
- **WHEN** a host applies `{ $type: "User.Update", email: null }` to
  `{ $type: "User", name: "Ada", email: "x@y" }`
- **THEN** the result SHALL be `{ $type: "User", name: "Ada" }` with no `email` key

#### Scenario: A mismatched target is rejected
- **WHEN** a host applies an update whose `$type` is `Team.Update` to a `User` record
- **THEN** the helper SHALL fail with a diagnostic naming both types

### Requirement: Rust runtime bounds evaluation and never panics
Every evaluation API SHALL accept runtime options holding a maximum call depth, defaulting to 100,
and a maximum range length, defaulting to one million. Exceeding either SHALL fail with
`nx-ir-resource-limit` naming the limit, and a range above its limit SHALL be refused before its
body runs. Every API SHALL report every failure as an error value carrying diagnostics. No image
that preparation accepted, no host value, and no instance SHALL cause the runtime to panic or to
exhaust the native stack.

#### Scenario: Unbounded recursion ends in a diagnostic
- **WHEN** a program evaluates a function that calls itself without end
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` naming the call-depth limit

#### Scenario: An oversized range is refused
- **WHEN** an image evaluates `for i in 0..2000000 { i }` under the default options
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` naming the limit of one million
- **AND** the same image SHALL evaluate under options that raise the limit above two million

#### Scenario: An altered image that prepares is evaluated safely
- **WHEN** an image with one overwritten cell passes preparation and its entrypoints are evaluated
- **THEN** each evaluation SHALL either succeed or return an error with a diagnostic
- **AND** SHALL NOT panic

### Requirement: Rust runtime diagnostics identify the failing declaration
A diagnostic the Rust runtime reports SHALL carry a code from the `nx-ir-*` set the TypeScript
runtime uses, a message, and, for a failure during evaluation, the declaration the failing
expression belongs to as `identity::name`. When the image carries its debug section the diagnostic
SHALL also carry the expression's span in the module's source. The runtime SHALL NOT read a source
file.

#### Scenario: A runtime failure names its declaration
- **WHEN** a function `ratio` in `main.nx` divides by zero
- **THEN** the diagnostic SHALL have code `nx-ir-division-by-zero` and name `main.nx::ratio`

#### Scenario: A span is present only with the debug section
- **WHEN** the same failure occurs in the image emitted with its debug section and in the image
  emitted without it
- **THEN** the first diagnostic SHALL carry a span and the second SHALL NOT

### Requirement: Rust runtime behavior is validated against the corpus and the interpreter
The Rust runtime's tests SHALL prepare, link where the module table requires, and evaluate every
artifact of the conformance corpus, with and without its debug section, and SHALL compare each
named entrypoint's value and each lifecycle's rendered outputs, tokens and effects with the
corpus's expected results. The tests SHALL cut every corpus image at every four-byte boundary and
overwrite every cell of at least one corpus image, and SHALL fail if any such image causes a panic.
The repository's tests SHALL also compile NX source, emit IR, run it through the Rust runtime, and
compare the results with native interpreter evaluation of the same source for functions, component
initialization, explicit-state evaluation and dispatch.

#### Scenario: The corpus is part of the runtime's test run
- **WHEN** the Rust runtime's tests run
- **THEN** every corpus entrypoint SHALL be evaluated and every corpus lifecycle initialized and
  dispatched in order
- **AND** a difference from an expected result SHALL fail the run naming the program and the
  entrypoint or lifecycle

#### Scenario: Damage is survived
- **WHEN** the damage tests run
- **THEN** every truncated image SHALL be refused
- **AND** every overwritten image SHALL be refused or evaluated without a panic

#### Scenario: Emitted IR agrees with the interpreter
- **WHEN** a supported NX program is evaluated by the interpreter and its emitted IR is evaluated
  by the Rust runtime
- **THEN** the two results SHALL be equal as canonical values
