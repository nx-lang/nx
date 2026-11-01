## ADDED Requirements

### Requirement: TypeScript IR runtime behavior is validated against the native runtime
The implementation SHALL include automated tests that emit NX IR from source or program artifacts,
execute the IR through the TypeScript runtime, and compare results against native evaluation of
the same source, which the Rust IR runtime performs, for the supported non-reactive subset.
Component tests SHALL cover descriptor construction, initialization, explicit state evaluation,
state patch validation, conditional content based on state, lone-child content binding at each
occurrence, and dispatch.

#### Scenario: Function parity test compares the two IR runtimes
- **WHEN** a supported NX program uses primitives, arithmetic, conditionals, match expressions,
  sequences, optional values and the presence operators, loops, records, unions, enums, member
  access, and function calls
- **THEN** automated tests SHALL verify that TypeScript IR runtime output matches native output for
  the same program

#### Scenario: Component parity test compares the two IR runtimes
- **WHEN** a supported NX component uses props, state defaults, conditional content, child
  component descriptors, and explicit state evaluation
- **THEN** automated tests SHALL verify that TypeScript IR runtime rendered output matches native
  rendered output for the same props and state

#### Scenario: Dispatch parity test compares the two IR runtimes
- **WHEN** a component binds handlers that patch its state, emit to its parent, and return effects
- **AND** the same batches of handler invocations and emitted actions are dispatched through both
  runtimes
- **THEN** automated tests SHALL verify that the rendered output, including tokens, and the
  ordered effects match between the TypeScript runtime and the native runtime

### Requirement: Component descriptors are recursively validated at typed sites
When a component-typed value has no discriminator or names the expected component declaration,
its props SHALL be recursively normalized against that declaration at host and program-internal
typed sites. Defaults SHALL be filled, unknown props and invalid nested records SHALL be rejected,
and bound emit handlers SHALL retain their identity. Existing cross-component descriptor behavior
SHALL remain unchanged. Both runtimes SHALL charge the same operations for equivalent work.

#### Scenario: A descriptor passes through a typed parameter and result
- **WHEN** a program constructs a component descriptor with a defaulted prop and a bound emit handler
- **AND** passes it through a component-typed function parameter and result
- **THEN** Rust and TypeScript SHALL preserve the same default and handler and report equal operation counts

#### Scenario: A nested host update record is invalid
- **WHEN** a host component-typed prop names the expected declaration and contains an update record with an unknown field or an invalid required-field clear
- **THEN** both runtimes SHALL reject it with a boundary diagnostic


## MODIFIED Requirements

### Requirement: TypeScript runtime evaluates IR function entrypoints
The TypeScript runtime SHALL evaluate public IR function entrypoints using eager NX semantics for
the supported expression set. Function evaluation SHALL bind normalized arguments, execute
module-qualified calls and references through the prepared program, normalize a function's result
and a value's initializer to the declared type the declaration carries, enforce resource or
recursion limits where exposed, and return canonical JSON-compatible NX values. An entry call —
`evaluateFunction` or `callFunction` — SHALL return `null` for an empty result of a function whose
declaration sets the optional-result flag, as `nx-ir-format` defines it; a call inside the program
SHALL see the empty array.

#### Scenario: Root function evaluates through IR
- **WHEN** a prepared IR program contains `let root() = { 1 + 2 }`
- **AND** a caller evaluates function entrypoint `root`
- **THEN** the runtime SHALL return `3`

#### Scenario: A declared result is normalized and an empty optional result is null
- **WHEN** a prepared IR program contains `let many(): int+ = { 5 }`, `let none(): int? = { if false { 1 } }` and `let caller(): int* = { none() }`
- **AND** a caller evaluates each as a function entrypoint
- **THEN** the runtime SHALL return `[5]`, `null` and `[]`

#### Scenario: Cross-module function call evaluates through IR
- **WHEN** a prepared IR program contains a root function that calls an imported library function
- **AND** a caller evaluates the root function
- **THEN** the runtime SHALL resolve the call through the module-qualified IR reference
- **AND** it SHALL return the same value as Rust IR runtime evaluation for the same program

#### Scenario: Match expression evaluates through IR
- **WHEN** a prepared IR function uses a match-style `if is` expression over a union value
- **AND** a caller evaluates that function
- **THEN** the runtime SHALL select the first matching arm in authored order
- **AND** it SHALL evaluate the else branch only when no arm matches

#### Scenario: Out-of-bounds array index is rejected
- **WHEN** a prepared IR function evaluates an index expression whose index is negative or
  greater than or equal to the sequence length
- **THEN** the runtime SHALL fail with a diagnostic identifying the out-of-bounds index
- **AND** it SHALL NOT return the empty value for the missing item

### Requirement: TypeScript runtime constructs component descriptors atomically
The TypeScript runtime SHALL evaluate component descriptor expressions as atomic descriptor
construction. Descriptor construction SHALL normalize props and content through the component's
effective prop contract and SHALL return a canonical descriptor payload without evaluating the
referenced component body.

Content binding SHALL respect the declared occurrence of the content property. When that property
is declared `+` or `*`, the bound value SHALL be an array regardless of how many children were
supplied, including exactly one, which binds a one-element array. When that property is declared
exactly-one or optional (`?:`), a single child SHALL bind to the child itself. This is the one
lone-child rule every engine shares: the child's value, lifted once to the declared type.

A handler property on a descriptor (`on<Emit>` for an emit the component declares) SHALL NOT be
treated as an unknown prop, wherever a descriptor's properties are normalized: in a descriptor
expression and in a component-typed value the host supplies. It SHALL be kept on the descriptor
beside the normalized props and SHALL appear in canonical output as a record with `$type`
`ActionHandler` carrying the public name of the action the handler accepts. A handler property that
names no emit of the component SHALL fail with a diagnostic naming the property and the component,
and one whose handler answers another emit or component SHALL fail with a type-mismatch diagnostic.

#### Scenario: Descriptor construction does not render component body
- **WHEN** a prepared IR function returns `<Child label="Name" />`
- **AND** `Child` is a concrete NX component with an implementation body
- **THEN** evaluating the function SHALL return a descriptor with `$type` equal to `"Child"`
- **AND** it SHALL include normalized prop `label`
- **AND** it SHALL NOT evaluate `Child`'s implementation body

#### Scenario: External component descriptor applies inherited defaults
- **WHEN** a prepared IR program contains `external component <ShortTextQuestion extends Question />`
  where inherited prop `label` has default `"Untitled"`
- **AND** descriptor construction evaluates `<ShortTextQuestion />`
- **THEN** the returned descriptor SHALL include `$type` equal to `"ShortTextQuestion"`
- **AND** it SHALL include `label` equal to `"Untitled"`

#### Scenario: A single child of a list-typed content property binds as a list
- **WHEN** a component declares `content items:Badge+`
- **AND** a descriptor for it is constructed with exactly one child
- **THEN** the content property SHALL be an array holding that one child
- **AND** descriptor construction SHALL NOT report a boundary type error

#### Scenario: Several children of a list-typed content property bind as a list
- **WHEN** a component declares a content property typed `+` or `*`
- **AND** a descriptor for it is constructed with more than one child
- **THEN** the content property SHALL be an array holding those children in the order supplied

#### Scenario: A single child of a non-list content property binds directly
- **WHEN** a component declares `content body:Element` or `content body?:Element`
- **AND** a descriptor for it is constructed with exactly one child
- **THEN** the content property SHALL be that child itself rather than an array

#### Scenario: List content binding matches the Rust IR runtime
- **WHEN** the same NX program binds a single child to a `+`, `*`, exactly-one or optional content property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the Rust IR runtime produces for it

#### Scenario: A descriptor's handler property is carried, not rejected
- **WHEN** a prepared IR function returns `<Button label="Add" onTapped=<Log /> />` for
  `external component <Button label:string emits { Tapped { } } />`
- **THEN** evaluating the function SHALL return a descriptor with `label` equal to `"Add"`
- **AND** `onTapped` SHALL be a record with `$type` `ActionHandler` and `action` `Button.Tapped`
- **AND** the record SHALL carry no `token`

### Requirement: TypeScript IR runtime accepts values produced by valid generated IR
The TypeScript IR runtime SHALL normalize record, union, and component values produced inside the
same prepared IR program with the same effective schema used for public boundary inputs. For valid
IR emitted from successfully analyzed source, runtime evaluation SHALL NOT fail with
`nx-ir-boundary-*` diagnostics for an absent optional union field, content-property fields, single
values at `+`- or `*`-typed fields, or record discriminators that were valid in native evaluation.

A single value at a `+`- or `*`-typed field SHALL normalize to a one-element array. This is the
language's own lift — the Rust IR runtime evaluates `xs={3.0}` and `xs={ <Item /> }` to one-element
sequences, and the IR records such a value at its own type rather than as a sequence, leaving the
lift to normalization.

A field whose type is spelled through a type alias SHALL normalize as the type the alias stands for,
however many aliases it is spelled through. A type alias is transparent in NX — `type Ints = int+`
*is* a sequence — so the IR SHALL carry what an alias resolves to rather than the alias, and a
sequence reached that way SHALL take the single-value lift and the content binding like any other.

A record value carries a `$type` discriminator, stamped by record construction. Where such a value
is normalized into a record-typed field, the declared type of the field SHALL supply the field list,
so the carried discriminator selects nothing and SHALL be dropped rather than reported as an unknown
field. Before it is dropped it SHALL be checked, and where the declared type is a concrete record a
value carrying no discriminator SHALL be accepted, since a host writing a plain object has none to
give.

A discriminator naming a record that extends the declared one names a value of an acceptable type,
not a foreign one. Such a value SHALL be normalized against the schema of the type it names — the
declared type's field list has no room for the derived fields — and SHALL keep its own discriminator
rather than being restamped with the declared name. A discriminator naming a type that does not
extend the declared one SHALL be rejected with `nx-ir-boundary-type`. A discriminator is a name and
not an identity, so where more than one declaration of that name extends the declared type, the
runtime SHALL reject the value with `nx-ir-boundary-type` naming the ambiguity rather than choose
one of them.

An abstract record has no values of its own, so no value SHALL be normalized *as* one. Generated NX
IR SHALL record whether a record is abstract, and where the declared type of a field is an abstract
record the runtime SHALL reject a value carrying no discriminator, a value whose discriminator names
that record, and a value whose discriminator names another abstract record extending it, each with
`nx-ir-boundary-type`. This holds at the host boundary the line analysis holds for NX source, which
rejects constructing an abstract record: without it a host object would be stamped with a type name
no NX program can produce. A discriminator naming a concrete record that extends the declared one
SHALL continue to be accepted.

#### Scenario: Nullable union absence passes runtime normalization
- **WHEN** a prepared IR program evaluates a record whose field `completion?:FlowCompletion` was not written
- **THEN** the TypeScript IR runtime SHALL accept the empty value for that field
- **AND** the canonical output SHALL carry no `completion` key

#### Scenario: Invalid undeclared union case remains rejected
- **WHEN** a host boundary input or malformed IR value supplies `$type: "FlowCompletion.undefined"` for a `FlowCompletion` union that does not declare `undefined`
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
- **AND** it SHALL NOT reinterpret the undeclared case as the empty value

#### Scenario: Content-populated required field does not report missing
- **WHEN** a prepared IR program constructs a component or record value whose required content property is supplied through element body content
- **THEN** the TypeScript IR runtime SHALL apply the content binding before required-field validation
- **AND** it SHALL NOT report `nx-ir-boundary-field` for the content property

#### Scenario: A single value at a list-typed property normalizes to a list of one
- **WHEN** a prepared IR program binds a value that is not a sequence to a property declared `+` or `*`
- **THEN** the property SHALL hold an array containing that one value
- **AND** it SHALL NOT report `nx-ir-boundary-type` for the value not being an array

#### Scenario: A list spelled through an alias is still a list
- **WHEN** a program declares `type Ints = int+` and binds `xs={3}` to a prop typed `Ints`, or binds
  one child to a content property typed through an alias of a `+` or `*` type
- **THEN** the property SHALL hold an array of one
- **AND** it SHALL equal the value the Rust IR runtime produces for the same program

#### Scenario: Single-value list coercion matches the Rust IR runtime
- **WHEN** the same NX program binds a single value to a `+`- or `*`-typed property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the Rust IR runtime produces for it

#### Scenario: A constructed record binds to a record-typed property
- **WHEN** a prepared IR program constructs a record value and binds it to a property whose declared
  type is that record
- **THEN** the TypeScript IR runtime SHALL normalize the value against the declared record's fields
- **AND** it SHALL NOT report `nx-ir-boundary-field` for the carried `$type` discriminator

#### Scenario: A record-typed property default normalizes
- **WHEN** a component property of a record type declares a record-construction default
- **AND** a descriptor omits that property
- **THEN** the property SHALL hold the constructed default

#### Scenario: Record field normalization matches the Rust IR runtime
- **WHEN** the same NX program binds a constructed record to a record-typed property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the Rust IR runtime produces for it

#### Scenario: Public boundary validation still rejects malformed host input
- **WHEN** a host supplies JSON with an unknown field, a missing required field, `null` at an exactly-one field, or an invalid union discriminator
- **THEN** the TypeScript IR runtime SHALL continue to reject the input with an `nx-ir-boundary-*` diagnostic
- **AND** it SHALL NOT treat this generated-IR parity requirement as permission to accept malformed host input

#### Scenario: A host record discriminator naming another type is rejected
- **WHEN** a host supplies `{ "$type": "Ghost", "name": "Ada" }` for a prop whose declared type is
  the record `User`
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
- **AND** it SHALL NOT return the value restamped as a `User`

#### Scenario: A host record with no discriminator is accepted
- **WHEN** a host supplies `{ "name": "Ada" }` for a prop whose declared type is the concrete record
  `User`
- **THEN** the TypeScript IR runtime SHALL normalize the value against `User`'s fields
- **AND** the normalized value SHALL carry `$type` equal to `"User"`

#### Scenario: A derived record binds to a base-typed property
- **WHEN** a prepared IR program binds a value of a record extending `Base` to a property whose
  declared type is `Base`
- **THEN** the TypeScript IR runtime SHALL normalize the value against the derived record's fields
- **AND** the normalized value SHALL keep the derived record's `$type` and its own fields

#### Scenario: A host object with no discriminator at an abstract-typed property is rejected
- **WHEN** a host supplies `{ "name": "Ada" }` for a prop whose declared type is an abstract record
  `Base`
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
  asking for a concrete type extending `Base`
- **AND** it SHALL NOT return the value stamped as a `Base`

#### Scenario: A discriminator naming an abstract record is rejected
- **WHEN** a host supplies a value whose `$type` names an abstract record, for a prop whose declared
  type is that record or a record it extends
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
  naming the abstract type
- **AND** it SHALL continue to accept a value whose `$type` names a concrete record extending the
  declared type

#### Scenario: A discriminator two declarations share is reported
- **WHEN** a value's `$type` names a record that two declarations in the program both declare, and
  both extend the declared type of the site the value reaches
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
  naming the ambiguity
- **AND** it SHALL NOT normalize the value against either declaration

### Requirement: TypeScript runtime evaluates the presence operators
The TypeScript runtime SHALL read the `seq` type kind, the `exists`, `optionalMember` and
`coalesce` node kinds, the `{}` match pattern and the required feature `occurrence-v1`, and SHALL
evaluate each with the semantics `presence-operators` defines for every engine: an `exists` node
evaluates its operand once and yields `true` when it holds at least one item and `false` when it is
the empty value; an `optionalMember` node evaluates its receiver once and yields the empty value
when the receiver is empty and the named member otherwise; a `coalesce` node yields its left
operand when that holds at least one item and otherwise evaluates and yields its right operand,
which it SHALL NOT evaluate in the first case; a `{}` pattern matches exactly when the scrutinee is
the empty value. The runtime SHALL carry the empty value as one representation wherever it arises —
an omitted optional field, an untaken branch, an empty `for` — and SHALL NOT hold `null` or
`undefined` as an NX value.

A module whose schema version is below `5` SHALL be refused by the version check as before. A
schema-`5` module that contains one of the three node kinds or a `{}` pattern and does not list
`occurrence-v1` SHALL be reported as malformed, naming the feature. A runtime that does not
implement `occurrence-v1` SHALL refuse a module that lists it by name.

#### Scenario: A presence test is a boolean
- **WHEN** a prepared program evaluates `let has(b:Book): boolean = { b.author? }` for `type Book = { title:string author?:Person }`
- **THEN** `has` applied to a `Book` with no `author` SHALL return `false` and to one with an `author` SHALL return `true`

#### Scenario: A step through an absent receiver is empty
- **WHEN** a prepared program evaluates `let name(b:Book): string? = { b.author?.name }` with a `Book` whose `author` is absent
- **THEN** the canonical result SHALL be the empty value
- **AND** with an `author` present it SHALL be that author's `name`

#### Scenario: A fallback is lazy
- **WHEN** a prepared program evaluates `let f(o?:int): int = { o ?? fail() }` where `fail` raises a runtime error, with `o` holding `1`
- **THEN** the result SHALL be `1` and no error SHALL be raised
- **AND** with `o` empty the evaluation SHALL fail with `fail`'s error

#### Scenario: The empty pattern matches absence
- **WHEN** a prepared program evaluates `let f(b:Book): string = { if b.author is { {} => "anonymous" else => b.author.name } }` with a `Book` whose `author` is absent
- **THEN** the result SHALL be `"anonymous"`

#### Scenario: Operator results match the Rust IR runtime
- **WHEN** the same NX program uses `x?`, `x?.m`, `x ?? y` and a `{}` pattern
- **THEN** the values the TypeScript runtime produces SHALL match the values the Rust IR runtime produces for the same inputs

#### Scenario: An operator node without the feature is malformed
- **WHEN** a hand-built schema-`5` image contains a `coalesce` node and does not list `occurrence-v1`
- **THEN** preparation SHALL fail with a diagnostic naming `occurrence-v1`

#### Scenario: An older runtime refuses the module by name
- **WHEN** a runtime that does not implement `occurrence-v1` prepares a module that lists it
- **THEN** preparation SHALL fail with a diagnostic naming the feature

## REMOVED Requirements

### Requirement: TypeScript IR runtime behavior is validated against existing NX semantics
**Reason**: The requirement defines "existing NX semantics" as the interpreter's, and the
interpreter is deleted.
**Migration**: Replaced by "TypeScript IR runtime behavior is validated against the native
runtime", with the same coverage.
