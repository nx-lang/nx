# rust-ir-runtime Specification

## Purpose

Defines the Rust crate that prepares, links and evaluates NX IR images without the NX compiler, so
that a Rust host can run compiled NX programs with the same results as the TypeScript IR runtime.

## Requirements

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
compute it as a 64-bit integer, wrapping at 64 bits, where the TypeScript runtime refuses such
an integer where it reads one from an image, and loses precision where arithmetic passes 2^53. A name that is not a
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
SHALL be accepted at an integer site. At an `object` site that wrapper
SHALL be read as the record it is, as the TypeScript runtime reads it. Serialized to JSON, a value
the Rust runtime returns SHALL equal the canonical value the TypeScript runtime returns, comparing
numbers by value and ignoring key order; an integer outside JavaScript's safe range, which the
TypeScript runtime does not hold, SHALL be compared with its canonical JSON form in either
spelling.

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
a maximum range length, defaulting to one million, an operation budget, absent by default, and a
stack budget, defaulting to one mebibyte. Exceeding the call depth or the range length SHALL fail
with `nx-ir-resource-limit` naming the limit, and a range above its limit SHALL be refused before
its body runs. Every API SHALL report every failure as an error value carrying diagnostics. No
image that preparation accepted, no host value, and no instance SHALL cause the runtime to panic
or, on a thread with the stack budget free where the call began, to exhaust the native stack.

The stack budget SHALL be the native stack one call of an evaluation API may use, measured from
where the call began, and SHALL bound every recursion the runtime makes over an image or a value
on behalf of that call: evaluating nested expressions, checking a value against a type, converting
a value at the host boundary, comparing two values, and finding how deeply state nests after a
patch. Reporting a failure SHALL NOT take stack in proportion to how deeply the failing value sits:
a diagnostic names where the value is, and that path is as long as the value is deep and is
written where the failure was found, which is where the least stack is left. An API that takes no options, which the update helpers and the comparisons of an instance's
handlers are, SHALL walk under the default budget. An evaluation that would use more SHALL fail with `nx-ir-resource-limit` naming
`maxStackBytes` and the budget in force, before the stack is exceeded. The budget is the host's
statement of what its thread has free: the runtime SHALL honor a budget below the default, so that
a host whose whole stack is smaller than a mebibyte gets a diagnostic where the default would
overrun it, and a budget above it. The bound on nested expressions SHALL stay fixed and SHALL apply
whatever the stack budget is.

The operation budget SHALL be counted as `nx-ir-format` defines an operation, and an absent budget
SHALL be unlimited. One budget SHALL cover one call of an evaluation API and everything that call
evaluates — for `dispatch_component_actions`, every handler the batch runs and the render that
follows — and each call SHALL start with the whole budget. A charge that would take the count above
the budget SHALL fail the call with `nx-ir-resource-limit` before the operation charged is
performed, naming the declaration the charge belongs to, as `nx-ir-format` assigns it, and, when
the charge is for a node and the image carries its debug section, the node's span. A walk the
runtime makes over a value for a purpose of its own, such as bounding how deeply state nests,
SHALL visit a value reached from several places once. For the same images, the same input and the same budget, the
Rust runtime SHALL fail at the node the TypeScript runtime fails at, and SHALL succeed where it
succeeds, wherever the two runtimes compute the same values.

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

#### Scenario: Nested loops end at the budget
- **WHEN** an image evaluates `for a in 0..1000 { for b in 0..1000 { for c in 0..1000 { a + b + c } } }` under an operation budget of one hundred thousand
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` naming the operation budget and its value
- **AND** no more than one hundred thousand operations SHALL have been performed

#### Scenario: An absent budget is unlimited
- **WHEN** an image evaluates `for i in 0..1000000 { i }` under the default options
- **THEN** evaluation SHALL return the list of one million integers

#### Scenario: The budget is exact and agrees with the TypeScript runtime
- **WHEN** an evaluation costs `n` operations in the TypeScript runtime
- **THEN** the Rust runtime SHALL evaluate it under a budget of `n`
- **AND** SHALL fail with `nx-ir-resource-limit` under a budget of `n - 1`, naming the same declaration and, with a debug section, the same span

#### Scenario: A value that doubles on each call is stopped before it is allocated
- **WHEN** a function doubles a list, or a string, on each of 60 nested calls under an operation budget of one hundred thousand
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` naming the operation budget
- **AND** the runtime SHALL NOT have allocated a list or a string the budget does not pay for

#### Scenario: A value shared many times over is stopped wherever it is walked
- **WHEN** a function builds a record that holds one value twice, forty levels deep, and the value is checked against its type at each call, compared with another like it, returned to the host, or stored in component state, under an operation budget of one hundred thousand
- **THEN** each evaluation SHALL fail with `nx-ir-resource-limit` naming the operation budget
- **AND** SHALL NOT take time or memory that grows with the 2^40 values the record is as a tree

#### Scenario: A batch shares one budget and leaves the instance usable
- **WHEN** a host dispatches a batch whose handlers together cost more than the operation budget
- **THEN** dispatch SHALL fail with `nx-ir-resource-limit` naming the operation budget
- **AND** the instance given SHALL dispatch a later, cheaper batch successfully

#### Scenario: A small stack budget ends deep recursion with a diagnostic
- **WHEN** a host evaluates a function that recurses several hundred calls deep under a raised
  call-depth limit and a stack budget of 64 kibibytes
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose limit is named `maxStackBytes`
  with the value `65536`
- **AND** the same call under a stack budget that holds the build's frames, which in an optimized
  build the default does, SHALL return the function's result or fail for the nesting bound

#### Scenario: The default stack budget is unchanged
- **WHEN** a host evaluates under default options
- **THEN** the stack budget in force SHALL be `1048576` bytes

#### Scenario: A deeply nested host value is refused within the stack budget
- **WHEN** a host passes a value nested 200 levels deep to a parameter of a declared type under a
  stack budget too small to check it
- **THEN** the call SHALL fail with `nx-ir-resource-limit` naming `maxStackBytes`
- **AND** SHALL NOT exhaust the native stack

#### Scenario: A comparison of deep values stops at the stack budget
- **WHEN** two values nested 200 levels deep are compared, or state that deep is checked after a
  patch, under a stack budget that a few levels use up
- **THEN** the walk SHALL fail with `nx-ir-resource-limit` naming `maxStackBytes`
- **AND** a function that recurses to a depth of its caller's choosing and then compares two host
  values nested 100 levels deep SHALL, on a thread whose stack is the budget and a margin, return
  the comparison's result or that diagnostic at every depth, and SHALL NOT exhaust the stack

#### Scenario: A failure at the bottom of a deep value is reported within the stack budget
- **WHEN** a host passes a record nested 200 levels deep whose innermost field has the wrong type,
  on a thread whose stack is the budget and a margin, for budgets on both sides of the one the
  check first fits in
- **THEN** each call SHALL fail with `nx-ir-resource-limit` naming `maxStackBytes` or with
  `nx-ir-boundary-type` naming the whole path to the field
- **AND** no call SHALL exhaust the stack

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

### Requirement: Rust runtime accepts any function at a function reference site
The Rust runtime SHALL read the function reference type kind, validating its result operand, and
the `function-reference-type-v1` feature, and SHALL normalize, validate, compare and render a value
at a site of a function reference type with the rules `typescript-ir-runtime` states for that
site: a function value of the linked program is accepted, a host-supplied `Function` record is
accepted when it names a function declaration of the linked program, whatever its signature, a
record that names none is refused with `nx-ir-function-value` naming the function, and any other
value is refused with `nx-ir-boundary-type` naming the site. For the same images and the same
inputs the canonical value it returns SHALL equal the TypeScript runtime's, and where that runtime
fails the Rust runtime SHALL fail with a diagnostic of the same code naming the same item. A call
by `Function` record SHALL be unchanged.

#### Scenario: A function reference field renders the Function record
- **WHEN** a prepared program declares, in `main.nx`, `type Tool = { fn: <function ... />: object* } let double(n:int): int = {n * 2} let root() = <Tool fn={double} />` and a caller evaluates `root`
- **THEN** the result's `fn` field, serialized to JSON, SHALL be `{ "$type": "Function", "module": "main.nx", "name": "double" }`

#### Scenario: A host-supplied record is validated against the program
- **WHEN** a host supplies a `Tool` whose `fn` is a `Function` record naming `double`, one naming `Nope`, and one whose `fn` is the string `"double"`
- **THEN** the first SHALL be accepted
- **AND** the second SHALL fail with `nx-ir-function-value` naming `Nope`
- **AND** the third SHALL fail with `nx-ir-boundary-type` naming `fn`

#### Scenario: The corpus program agrees with the TypeScript runtime
- **WHEN** the Rust runtime runs the function reference conformance program
- **THEN** every entrypoint SHALL produce the recorded result the TypeScript runtime produces

#### Scenario: A damaged function reference entry is refused
- **WHEN** a cell of an `anyFunction` type entry in a corpus image is overwritten with an arbitrary value
- **THEN** preparation SHALL either refuse the image with a diagnostic or read it as a valid image, and SHALL NOT panic

### Requirement: Rust runtime resource-limit diagnostics name the limit
Every diagnostic the Rust runtime reports with the code `nx-ir-resource-limit` SHALL carry the name
of the limit that was reached and, where the limit is a number, its value, as data beside the
message. A limit the TypeScript runtime also has SHALL carry the name the TypeScript runtime gives
it: `maxOperations`, `maxInputSize`, `maxCallDepth`, `maxRangeLength` and `maxExpressionNesting`. The limits only
the Rust runtime has SHALL be named `maxStackBytes` for the native stack an evaluation may use,
`maxValueNesting` for the nesting of a value at the host boundary or in component state, and
`maxComponentDepth` for the nesting of component instances in an instance tree. A diagnostic with
any other code SHALL carry no limit.

#### Scenario: The operation budget is named
- **WHEN** an evaluation fails for an exhausted operation budget of five thousand
- **THEN** the diagnostic's limit SHALL have the name `maxOperations` and the value `5000`

#### Scenario: The two runtimes name a shared limit alike
- **WHEN** the same image fails in both runtimes for runaway recursion under the default options
- **THEN** both diagnostics SHALL name the limit `maxCallDepth` with the value `100`

#### Scenario: A value nested too deeply names its own limit
- **WHEN** a host passes a value nested 300 levels deep
- **THEN** the diagnostic's limit SHALL have the name `maxValueNesting` and the value `256`

#### Scenario: A tree nested too deeply names its own limit
- **WHEN** a visit of an instance tree is for a node 101 component instances deep under the
  default options
- **THEN** the diagnostic's limit SHALL have the name `maxComponentDepth` and the value `100`

### Requirement: Rust runtime limits the size of host input
The runtime options every evaluation API accepts SHALL include an input limit, absent by default,
measured as `nx-ir-format` defines the input size of a call. An absent limit SHALL be unlimited.
The limit SHALL cover every host value passed to a public method that takes runtime options, the
same values the TypeScript runtime measures; a component instance SHALL NOT be measured, and
restoring a stored instance, which takes no options, SHALL keep the bounds it has. A call whose
input is larger than the limit SHALL fail with `nx-ir-resource-limit` naming the limit
`maxInputSize` and its value, before the program is looked at, before any value is checked against
a type and before any node is evaluated, having stopped measuring as soon as the size passed the
limit, within the bound `nx-ir-format` sets. Measuring the input SHALL charge nothing to the
operation budget.

The Rust runtime SHALL measure the size the TypeScript runtime measures for the same input, so
that the input limit refuses the same calls in both. The input limit SHALL be applied before the
runtime's bound on the nesting of a host value: input that passes both SHALL be reported for its
size. Input within the limit that nests more deeply than that bound SHALL still be refused for its
nesting, which the TypeScript runtime does not bound.

#### Scenario: Input over the limit is refused before anything runs
- **WHEN** a host evaluates a function with a list of 20,000 integers under an input limit of 1,000
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose limit has the name `maxInputSize` and the value `1000`
- **AND** no node SHALL have been evaluated

#### Scenario: The limit is exact and agrees with the TypeScript runtime
- **WHEN** the input of a call has the size `n` in the TypeScript runtime
- **THEN** the Rust runtime SHALL accept it under an input limit of `n`
- **AND** SHALL refuse it under a limit of `n - 1`

#### Scenario: An absent limit is unlimited
- **WHEN** a host evaluates a function with a list of one million integers under the default options
- **THEN** evaluation SHALL proceed as it did before the option existed

#### Scenario: A batch is measured whole before any of it runs
- **WHEN** a host dispatches a batch whose first entry is small and whose second is larger than the limit, under an operation budget of zero
- **THEN** the dispatch SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`, not the budget
- **AND** no handler SHALL have run

#### Scenario: Oversized input is reported before any other fault of the call
- **WHEN** a host evaluates a component with props of the wrong type and a state larger than the limit
- **THEN** the call SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`

#### Scenario: Size is reported before nesting
- **WHEN** a host passes a value nested 2,000 deep under an input limit of 1,000
- **THEN** the call SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`
- **AND** a value nested 300 deep, of size 301, under the same limit SHALL fail naming the nesting bound

#### Scenario: A refused dispatch leaves the instance usable
- **WHEN** a host dispatches a batch larger than the input limit
- **THEN** dispatch SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`
- **AND** the instance given SHALL dispatch a later, smaller batch successfully

### Requirement: Rust runtime reports what a call used and exports the input measure
The runtime options SHALL be able to carry a usage report that the host shares with the runtime.
When a call given one ends, whether it succeeds or fails, the report SHALL hold the operations the
call used if an operation budget was set and none otherwise, and the input size of the call if an
input limit was set and the input was within it and none otherwise, replacing what an earlier call
left there. The numbers SHALL be those `nx-ir-format` defines and SHALL equal what the TypeScript
runtime reports for the same call, for a call that fails as for one that succeeds: a charge the
budget refused SHALL NOT be counted. With no report given the runtime SHALL do nothing for one, and
with no budget it SHALL count nothing for one.

The runtime SHALL export a function that returns the size of one host value as `nx-ir-format`
defines it and as the input limit measures it, and that, given a limit, stops as soon as the size
passes the limit and returns a number greater than it; and one that does the same for a map of
named values measured as one record, which is the form props, a state, a patch and arguments by
name are passed in, so that a host measures one without copying it.

#### Scenario: A successful call reports its count
- **WHEN** a host evaluates a function that costs 29 operations under a budget of 100,000 with a usage report
- **THEN** the report SHALL hold 29 operations

#### Scenario: A failed call reports what it used
- **WHEN** a host evaluates a function under a budget of 10 with a usage report and it fails for the budget
- **THEN** the report SHALL hold at most 10 operations

#### Scenario: A refused charge is not reported
- **WHEN** a call under a budget of 10 has been charged 5 operations and its next charge, 31 for a concatenation, is refused
- **THEN** the report SHALL hold 5 operations

#### Scenario: A map is measured as the record it is passed as
- **WHEN** a host measures the props map `{ "title": "Home", "count": 3 }` with the exported function for maps
- **THEN** the size SHALL be three, as the call that takes those props reports

#### Scenario: The two runtimes report alike
- **WHEN** a corpus case is evaluated in both runtimes under a budget and an input limit it fits, with a usage report
- **THEN** both reports SHALL hold the recorded operation count and the recorded input size

#### Scenario: A value measured alone has the size it has in a call
- **WHEN** a host measures a value with the exported function and then passes it as the one argument of a function under an input limit with a usage report
- **THEN** the two sizes SHALL be equal

### Requirement: Rust runtime allocation is validated against its operation count
The repository's tests SHALL measure, for the Rust runtime and every generated case of
`nx-ir-format`'s cost validation at both of its scales, the bytes the runtime asks its allocator
for during the call. The tests SHALL fail when that number exceeds a fixed multiple of the call's
operation count plus its input size, and SHALL fail when the bytes for each unit of that sum at
the larger scale are more than twice what they are at the smaller. The measure SHALL be of bytes
requested, the check SHALL block a merge, and the generated cases a known finding covers SHALL be
exempt until it is fixed. A step that copies the input a fixed number of times in a call is within
these bounds and is not what the check looks for.

#### Scenario: Allocation follows the count
- **WHEN** a generated case is evaluated at its two scales
- **THEN** at each scale the bytes allocated SHALL be within the fixed multiple of the operation count plus the input size
- **AND** the bytes for each unit of that sum SHALL at most double between the scales

#### Scenario: A copy that is not charged is caught
- **WHEN** the runtime is changed so that a value written for the host is no longer charged
- **THEN** the allocation check SHALL fail naming the seed and a case that returns a value many times

#### Scenario: The check is repeatable
- **WHEN** the allocation check is run twice in one build
- **THEN** it SHALL measure the same number of bytes for the same case
- **AND** every case SHALL be within the bounds in an unoptimized build and in an optimized one

### Requirement: Rust runtime diagnostics name the argument a failure is in
A diagnostic for a failure the Rust runtime finds in a value a host passed as an argument of the
function it called, by name or by position, SHALL carry the name of the parameter the value was
passed for, as data beside the message. It SHALL carry that name for the same failures the
TypeScript runtime names an argument for, and SHALL carry none for the failures the TypeScript
runtime names none for: a failure a default raises, a resource limit whenever it is reached, a
failure of the image or the program found while a value is checked, a failure while the body is
evaluated, a failure in the result, more positional arguments than the function has parameters,
a `Function` record given to `call_function` to say which function to call that is not such a
record or names no function, and a failure of any other entry point. For the same image and the
same arguments the two runtimes SHALL name the same argument, or both name none. No diagnostic's
code SHALL change.

#### Scenario: A value of the wrong type names its parameter
- **WHEN** a host calls `let findPlans(teamSize:int, note?:string): string` with `teamSize` bound
  to the string `five`
- **THEN** the call SHALL fail with `nx-ir-boundary-type` and the diagnostic SHALL name the
  argument `teamSize`

#### Scenario: A missing required argument names its parameter
- **WHEN** a host calls that `findPlans` with no arguments
- **THEN** the call SHALL fail with `nx-ir-arguments` and the diagnostic SHALL name the argument
  `teamSize`

#### Scenario: The two runtimes name an argument alike
- **WHEN** the same image is called in both runtimes with each of: a value of the wrong type, a
  record with a wrong field three levels down, a missing required argument, a `Function` record
  that names no function, input over the input limit, an operation budget spent while an argument
  is checked, a record whose field default divides by zero, arguments that fit followed by a
  division by zero in the body, a record whose field default fails with `nx-ir-arguments`, and a
  parameter left out whose default fails with `nx-ir-arguments`
- **THEN** for each call both diagnostics SHALL name the same argument, or both SHALL name none
- **AND** the conformance corpus SHALL hold each of those calls as a case or as a check it makes
  of a case, so that a difference fails the corpus tests of the runtime that differs

#### Scenario: A limit names no argument
- **WHEN** a host calls a function with input over the input limit, or with fifty records for a
  parameter `items:Item+` under an operation budget of twenty
- **THEN** each diagnostic SHALL name its limit and no argument

#### Scenario: A failure a field default raises names no argument
- **WHEN** a program declares `type Req = { n:int share:int = { 100 / n } }` and
  `let f(req:Req): int`, and a host calls `f` with a `Req` whose `n` is `0`
- **THEN** the diagnostic SHALL name no argument

#### Scenario: A default that fails with a code an argument's failure has names no argument
- **WHEN** a host passes a record that fits its type and one of whose field defaults calls a
  function value the record holds without an argument that function requires, or leaves out a
  parameter whose default makes such a call
- **THEN** the call SHALL fail with `nx-ir-arguments` and the diagnostic SHALL name no argument

#### Scenario: A failure inside the function names no argument
- **WHEN** a host calls a function with arguments that fit its parameters and the body then fails
- **THEN** the diagnostic SHALL name no argument

### Requirement: Rust runtime builds for the WebAssembly targets hosts use
The Rust IR runtime crate and the crates it depends on SHALL build for `wasm32-unknown-emscripten`
and `wasm32-wasip1` without features, patches or a C toolchain, and the repository's continuous
integration SHALL check both on every pull request. The runtime's documentation SHALL state how
much stack a release build's deepest permitted evaluation was measured to use on a WebAssembly
target, and what a host whose stack is smaller than the default budget sets: a stack budget no
larger than what is free where it calls the runtime, or a larger stack at link time.

#### Scenario: A pull request that breaks a WebAssembly build fails
- **WHEN** a change makes the runtime crate, the format crate or the value crate fail to compile
  for `wasm32-unknown-emscripten` or `wasm32-wasip1`
- **THEN** the pull request's build SHALL fail naming the target

#### Scenario: A host on a small stack finds what to set
- **WHEN** a host author reads the runtime crate's README
- **THEN** it SHALL say that the thread calling the runtime needs the stack budget free
- **AND** it SHALL give the measured stack use on a WebAssembly target and the option that lowers
  the budget

### Requirement: Rust runtime reports origins through its options
`RuntimeOptions` SHALL have `origins: Option<Arc<Origins>>`, `None` by default, which the runtime
fills as the `value-origin` capability's origins report, the way it fills `usage`. `Origins` SHALL
expose the last call's entries, each with the record's JSON pointer and a `SourceSpan` of its
origin. No method's signature or result type SHALL change. The serialized form of a component
instance SHALL hold no origins, so an instance `Program::restore_component_instance` restores SHALL
hold none: a record of its state has an origin again only once a call given a report builds it
again.

#### Scenario: A restored instance
- **WHEN** a host initializes a component whose state holds a record the program built with an
  origins report, serializes the instance, restores it and dispatches a batch with a report that
  leaves the record as it is
- **THEN** the record SHALL have no origin in the rendered output

#### Scenario: Reading a report
- **WHEN** a host sets `origins` and calls `Program::initialize_component` with an image built with
  its debug section
- **THEN** after the call the report's entries SHALL list the origins of the rendered output, equal
  to what the TypeScript runtime reports for the same call
