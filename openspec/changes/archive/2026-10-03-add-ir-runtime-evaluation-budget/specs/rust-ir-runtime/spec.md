## MODIFIED Requirements

### Requirement: Rust runtime bounds evaluation and never panics
Every evaluation API SHALL accept runtime options holding a maximum call depth, defaulting to 100,
a maximum range length, defaulting to one million, and an operation budget, absent by default.
Exceeding the call depth or the range length SHALL fail with `nx-ir-resource-limit` naming the
limit, and a range above its limit SHALL be refused before its body runs. Every API SHALL report
every failure as an error value carrying diagnostics. No image that preparation accepted, no host
value, and no instance SHALL cause the runtime to panic or to exhaust the native stack.

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

## ADDED Requirements

### Requirement: Rust runtime resource-limit diagnostics name the limit
Every diagnostic the Rust runtime reports with the code `nx-ir-resource-limit` SHALL carry the name
of the limit that was reached and, where the limit is a number, its value, as data beside the
message. A limit the TypeScript runtime also has SHALL carry the name the TypeScript runtime gives
it: `maxOperations`, `maxCallDepth`, `maxRangeLength` and `maxExpressionNesting`. The limits only
the Rust runtime has SHALL be named `maxStackBytes` for the native stack an evaluation may use and
`maxValueNesting` for the nesting of a value at the host boundary or in component state. A
diagnostic with any other code SHALL carry no limit.

#### Scenario: The operation budget is named
- **WHEN** an evaluation fails for an exhausted operation budget of five thousand
- **THEN** the diagnostic's limit SHALL have the name `maxOperations` and the value `5000`

#### Scenario: The two runtimes name a shared limit alike
- **WHEN** the same image fails in both runtimes for runaway recursion under the default options
- **THEN** both diagnostics SHALL name the limit `maxCallDepth` with the value `100`

#### Scenario: A value nested too deeply names its own limit
- **WHEN** a host passes a value nested 300 levels deep
- **THEN** the diagnostic's limit SHALL have the name `maxValueNesting` and the value `256`
