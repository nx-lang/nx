# typescript-ir-runtime Specification

## Purpose
TBD - created by archiving change add-nx-ir-format. Update Purpose after archive.

## Requirements

### Requirement: TypeScript runtime loads and prepares NX IR programs
The TypeScript runtime SHALL expose APIs that accept an NX IR artifact as bytes, validate the image
header, IR schema version, runtime ABI, required features and every structural reference before
reading anything else, and return a prepared module. Preparation SHALL read the image in place
through typed-array views rather than parsing a document, SHALL decode a string only when it is
named, SHALL index the module's declarations by name, resolve every reference to the module itself,
build entrypoint tables, precompute schema validators and default evaluators, and create efficient
evaluators over the module's node table. References to other modules SHALL be resolved by the
linking step, and a prepared module whose table names only itself SHALL be usable as a program
directly. A malformed image SHALL be reported through the runtime's diagnostic result, never as a
thrown exception from the non-throwing API.

#### Scenario: Supported IR prepares successfully
- **WHEN** a caller loads a valid NX IR image whose runtime ABI matches the TypeScript runtime
- **THEN** the runtime SHALL return a prepared module
- **AND** the prepared module SHALL expose function and component entrypoint lookup by public name
- **AND** declaration lookup SHALL be by module identity and declaration name

#### Scenario: Bytes are read without copying when aligned
- **WHEN** a caller passes an `ArrayBuffer`, or a byte view whose offset is a multiple of four
- **THEN** preparation SHALL read the image through views over those bytes
- **AND** SHALL NOT copy the image

#### Scenario: A self-contained artifact is a program
- **WHEN** a caller prepares an artifact whose module table holds only its own module
- **THEN** the caller SHALL be able to evaluate its entrypoints without a linking step

#### Scenario: Unsupported runtime ABI is rejected
- **WHEN** a caller loads NX IR requiring a runtime ABI that the TypeScript runtime does not support
- **THEN** preparation SHALL fail with an actionable diagnostic
- **AND** the runtime SHALL NOT return a prepared module

#### Scenario: Unknown required feature is rejected
- **WHEN** a caller loads NX IR that declares a required feature unknown to the TypeScript runtime
- **THEN** preparation SHALL fail with an actionable diagnostic naming that feature

#### Scenario: A malformed image is a diagnostic, not an exception
- **WHEN** a caller passes bytes that are truncated, or whose cells have been altered, to the
  non-throwing preparation API
- **THEN** the result SHALL carry a diagnostic identifying what is malformed
- **AND** the API SHALL NOT throw

#### Scenario: Evaluating an unlinked module is rejected
- **WHEN** a caller evaluates an entrypoint of a prepared module whose table names another module
- **AND** the module has not been linked
- **THEN** evaluation SHALL fail with a diagnostic saying the module must be linked first

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
- **AND** it SHALL return the same value as native interpreter evaluation for the same program

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

#### Scenario: List content binding matches the Rust interpreter
- **WHEN** the same NX program binds a single child to a `+`, `*`, exactly-one or optional content property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the
  Rust interpreter produces for it

#### Scenario: A descriptor's handler property is carried, not rejected
- **WHEN** a prepared IR function returns `<Button label="Add" onTapped=<Log /> />` for
  `external component <Button label:string emits { Tapped { } } />`
- **THEN** evaluating the function SHALL return a descriptor with `label` equal to `"Add"`
- **AND** `onTapped` SHALL be a record with `$type` `ActionHandler` and `action` `Button.Tapped`
- **AND** the record SHALL carry no `token`

### Requirement: TypeScript runtime initializes and evaluates components with host-owned state
The TypeScript runtime SHALL expose component APIs that initialize a concrete component from props,
evaluate a concrete component from props and current state, validate/normalize a complete state
object, and apply a host-provided state patch to produce a normalized next state. These operations
SHALL be pure with respect to runtime-held component instances and SHALL NOT require hidden mutable
component state.

Initialization SHALL return, beside the rendered output and the initial state, an opaque instance
value the host passes back to dispatch. The instance SHALL hold everything dispatch needs: the
component, its normalized props including any handler props the parent bound, the state, and the
handlers the rendered output refers to. Initialization MAY be given a parent instance; every
`ActionHandler` record carrying a token in the props, at any depth, SHALL then be replaced by the
handler that instance holds under the token, a token it does not hold SHALL fail with a diagnostic
naming it, and such a record given without a parent SHALL fail as an unknown field. The handler
substituted SHALL be the value the parent's table holds, so a host holding both instances can
recognize it in the child's table. Initialization MAY also be given a state object; it SHALL then
be validated as a complete state for the component and used in place of the initial state, with
tokens assigned as for any initialization, so a host can re-render an instance with new props and
the state it holds. Handler props SHALL NOT be bound in the body's scope. Every bound handler in rendered output returned by
initialization SHALL be a record with `$type` `ActionHandler` carrying the public name of the
action it accepts and an opaque string token valid only for dispatch against the instance returned
by the same call. Rendered output returned by pure evaluation SHALL represent handlers the same way
but SHALL NOT carry a token. Token assignment SHALL follow the Rust runtime's scheme, so the same
program initialized with the same props yields the same tokens in both runtimes.

#### Scenario: Component initialization materializes initial state
- **WHEN** a prepared IR program contains `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** a caller initializes `SearchBox` without props
- **THEN** the runtime SHALL materialize prop `placeholder` as `"Find docs"`
- **AND** it SHALL return state with `query` equal to `"Find docs"`
- **AND** it SHALL return rendered output whose `TextInput` value is `"Find docs"`

#### Scenario: Explicit state controls evaluation
- **WHEN** a caller evaluates prepared component `SearchBox` with state `{ query: "docs" }`
- **THEN** the runtime SHALL render the component body with `query` equal to `"docs"`
- **AND** it SHALL NOT replace the supplied state field with the default expression value

#### Scenario: Host-owned state patch is validated
- **WHEN** a caller applies state patch `{ query: "guides" }` to current state
  `{ query: "docs" }` for prepared component `SearchBox`
- **THEN** the runtime SHALL return normalized next state `{ query: "guides" }`
- **AND** it SHALL validate the patched state against `SearchBox`'s declared state schema

#### Scenario: Invalid state patch is rejected
- **WHEN** a caller applies state patch `{ query: 123 }` to a component whose `query` state field
  is `string`
- **THEN** the runtime SHALL fail with a diagnostic identifying the invalid state field
- **AND** it SHALL NOT return a partially updated state object

#### Scenario: Initialization output carries handler tokens
- **WHEN** a caller initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`
- **THEN** the rendered `Button`'s `onTapped` SHALL be a record with `$type` `ActionHandler`,
  `action` `Button.Tapped`, and token `h1-1`
- **AND** the result SHALL include an instance the caller can dispatch against

#### Scenario: A parent's handler reaches the child through the parent's instance
- **WHEN** a caller initializes `Page`, whose body renders
  `<SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />`
- **AND** initializes `SearchBox` with the rendered descriptor's fields as props and `Page`'s
  instance as the parent
- **THEN** initialization SHALL succeed
- **AND** `SearchBox`'s body SHALL NOT see `onSearchSubmitted` in scope

#### Scenario: A handler record without a parent instance is refused
- **WHEN** a caller initializes `SearchBox` with `onSearchSubmitted` set to an `ActionHandler`
  record and no parent instance
- **THEN** initialization SHALL fail with a diagnostic naming the field

#### Scenario: Pure evaluation output carries no tokens
- **WHEN** a caller evaluates `Counter` through the explicit-state evaluation API
- **THEN** the rendered `Button`'s `onTapped` SHALL be an `ActionHandler` record without a `token`

#### Scenario: Tokens match the Rust runtime
- **WHEN** the same program is initialized with the same props by the TypeScript runtime and by the
  Rust runtime
- **THEN** every handler in the two rendered outputs SHALL carry the same token

#### Scenario: Initialization with a supplied state keeps it
- **WHEN** a caller initializes `SearchBox` with props `{ placeholder: "Find" }` and state
  `{ query: "docs" }`
- **THEN** the runtime SHALL render the body with `query` equal to `"docs"` rather than the default
  `"Find"`
- **AND** it SHALL return state `{ query: "docs" }` and an instance whose tokens start at
  generation 1

#### Scenario: A supplied state is validated
- **WHEN** a caller initializes `SearchBox` with state `{ query: 123 }` or with a state missing
  `query`
- **THEN** the runtime SHALL fail with a diagnostic identifying the invalid or missing state field

#### Scenario: A handler resolved from the parent is the parent's own
- **WHEN** a `Page` instance's rendered output places a `Page`-bound handler inside the content of
  an authored `Stack`, and a caller initializes `Stack` from that descriptor's fields with `Page`'s
  instance as parent
- **THEN** the handler `Stack`'s instance holds under its own token SHALL be the same value `Page`'s
  instance holds under `Page`'s token

### Requirement: TypeScript runtime dispatches action batches against a component instance
The TypeScript runtime SHALL expose a dispatch API that takes a prepared program, an instance
returned by initialization or by a previous dispatch, and an ordered batch. Each batch entry SHALL
be either an action record the component emits, which runs the handler the parent bound on the
instance's props, or a handler invocation, a record with `$type` `ActionHandlerInvocation`, a
`token` read from the instance's most recent rendered output, and the `action` record to feed it.
Dispatch SHALL process entries in order. A handler whose owner is the instance's component SHALL
read that component's state as it stands when the handler runs, so two patches in one batch
compound; every other captured value SHALL be the value captured when the handler was created.
Every update record such a handler returns for the instance's component SHALL be applied to the
working state in order with full validation, and every other returned value SHALL be collected as
an effect. A handler whose owner is another component, or none, SHALL NOT read or patch the
instance's state, and everything it returns, update records included, SHALL be an effect. After
the batch, the body SHALL be re-rendered once against the next state, its handlers SHALL receive
fresh tokens, and dispatch SHALL return the rendered output, the ordered effects, the next state,
and the next instance. The instance passed in SHALL NOT be modified.

#### Scenario: Handler invocation patches the component's state
- **WHEN** a caller initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`,
  reads the token from the rendered `Button`'s `onTapped`, and dispatches one invocation of that
  token with `<Button.Tapped />`
- **THEN** dispatch SHALL return state with `count` equal to `1`
- **AND** the rendered output SHALL reflect `count` equal to `1`
- **AND** the effect list SHALL be empty

#### Scenario: State reads in a handler are live within a batch
- **WHEN** a caller dispatches two invocations of the same `onTapped=<Update count={count + 1} />`
  token in one batch against a `Counter` at `count` `0`
- **THEN** dispatch SHALL return state with `count` equal to `2`

#### Scenario: A shadowed state name keeps its captured value
- **WHEN** a component body binds `for count in tens { <Button onTapped=<Update count={count + 1} /> /> }`
  with `tens` holding `10`, inside a component whose state field `count` is `0`
- **AND** a caller dispatches that handler
- **THEN** the update SHALL set `count` to `11`

#### Scenario: Non-update results of a handler invocation are effects
- **WHEN** a caller dispatches an invocation of a handler bound as
  `onTapped={<Update count=0 /> <Saved />}` inside a component that emits `Saved`
- **THEN** dispatch SHALL apply the update to the state
- **AND** SHALL return `Saved` as the only effect

#### Scenario: An emitted action runs the parent-bound handler
- **WHEN** a caller initializes `SearchBox` from a parent's rendered descriptor whose
  `onSearchSubmitted` is `<DoSearch search={action.searchString} />`, passing the parent's instance
- **AND** dispatches `<SearchBox.SearchSubmitted searchString="docs" />` against the instance
- **THEN** dispatch SHALL return `<DoSearch search="docs" />` as the only effect
- **AND** the state SHALL be unchanged

#### Scenario: Update records returned by a parent-bound handler are effects
- **WHEN** a caller dispatches `<SearchBox.SearchSubmitted searchString="docs" />` against a
  `SearchBox` instance whose parent bound `onSearchSubmitted=<Update query={action.searchString} />`
  inside the parent's own body
- **THEN** dispatch SHALL NOT change the `SearchBox` state
- **AND** SHALL return the parent's update record in the effect list

#### Scenario: A handler in a content child is unowned by the child
- **WHEN** `Page`'s body renders `<Stack><Button onTapped=<Update count={count + 1} /> /></Stack>`
  and a caller initializes `Stack` from `Page`'s rendered output, passing `Page`'s instance
- **AND** dispatches an invocation of the `Button`'s token against the `Stack` instance
- **THEN** dispatch SHALL return `Page.Update` as the only effect
- **AND** `Stack`'s state SHALL be unchanged

#### Scenario: An emitted action with no bound handler is a no-op
- **WHEN** a caller dispatches an action the component emits against an instance whose parent
  bound no handler for it
- **THEN** dispatch SHALL succeed with no effects and unchanged state
- **AND** SHALL still return rendered output and a next instance

#### Scenario: Dispatch returns rendered output and fresh tokens in every case
- **WHEN** a caller dispatches an empty batch
- **THEN** dispatch SHALL return the rendered output for the unchanged state
- **AND** the tokens in it SHALL differ from the tokens in the previous rendered output
- **AND** a token from the previous output SHALL be rejected by the next dispatch

#### Scenario: A token from a stale instance is rejected
- **WHEN** a caller dispatches a handler invocation whose token was read from an earlier rendered
  output than the one the supplied instance was returned with
- **THEN** dispatch SHALL fail with a diagnostic naming the unknown handler token
- **AND** the supplied instance SHALL remain usable

#### Scenario: An invocation with the wrong action type is rejected
- **WHEN** a caller dispatches a token bound for `Button.Tapped` together with a
  `Slider.EndChanged` action
- **THEN** dispatch SHALL fail with a type-mismatch diagnostic naming the expected action

#### Scenario: An action the component does not emit is rejected
- **WHEN** a caller dispatches `<Other.Changed />` against a component that does not emit it
- **THEN** dispatch SHALL fail with a diagnostic naming the component and the action

#### Scenario: A dispatch batch is atomic
- **WHEN** a caller dispatches a batch whose first entry patches `count` to `5` and whose second
  entry names an unknown token
- **THEN** dispatch SHALL fail with the unknown-token diagnostic
- **AND** dispatching a valid batch afterwards against the original instance SHALL observe `count`
  at its original value

#### Scenario: Update validation failures abort the batch
- **WHEN** a handler invoked during dispatch returns an update record whose field value does not
  match the state field's type
- **THEN** dispatch SHALL fail with a diagnostic naming the field
- **AND** SHALL NOT return a next instance

### Requirement: TypeScript runtime rejects a program that requires action handlers it cannot run
A prepared program SHALL only be produced when every required feature is known to the runtime.
Because a module with a handler lists the action-handler feature, a runtime built before this
change SHALL refuse it by that name rather than silently drop the binding.

#### Scenario: An older runtime refuses a program with handlers
- **WHEN** NX IR listing the action-handler feature is loaded by a runtime that does not know it
- **THEN** preparation SHALL fail with a diagnostic naming that feature

### Requirement: TypeScript runtime validates JSON boundary values against IR schemas
The TypeScript runtime SHALL use IR schema metadata to normalize and validate public boundary
values, including function arguments, component props, component state, state patches, enum values,
records, sequences and optional values, and union cases. Sequences and optional values SHALL be
decoded as `occurrence-types` defines the canonical encoding: at a `?` or `*` site a missing key,
`null` and an empty array are the empty value and a one-element array at a `?` site is its element;
at a `+` site a missing key, `null` and an empty array are rejected; at an exactly-one site `null`
is rejected. Missing required fields, unknown fields, invalid enum members, and type mismatches
SHALL produce diagnostics consistent with existing NX runtime behavior.

#### Scenario: Missing required prop is rejected
- **WHEN** a caller initializes a prepared component requiring prop `label:string`
- **AND** the caller omits `label`
- **THEN** the runtime SHALL fail with a diagnostic identifying the missing prop

#### Scenario: Host absence decodes to the empty value where zero is admitted
- **WHEN** a caller initializes a prepared component declaring `subtitle?:string tags?:string+` with props `{ subtitle: null, tags: [] }`, and again with `{}`
- **THEN** both initializations SHALL succeed with `subtitle` and `tags` empty
- **AND** the rendered output SHALL be identical for the two

#### Scenario: Host absence is rejected where at least one is required
- **WHEN** a caller initializes a prepared component declaring `items:string+` with props `{ items: [] }`, or `{ items: null }`
- **THEN** the runtime SHALL fail with a diagnostic naming `items`

#### Scenario: Unknown state field is rejected
- **WHEN** a caller evaluates a component with state object `{ query: "docs", extra: true }`
- **AND** the component state schema does not declare `extra`
- **THEN** the runtime SHALL fail with a diagnostic identifying the unknown state field

#### Scenario: Unknown enum member is rejected
- **WHEN** a caller supplies string `"blue"` for a prop whose declared type is enum `ThemeMode`
- **AND** `ThemeMode` does not declare member `blue`
- **THEN** the runtime SHALL fail with a diagnostic identifying the invalid enum member

#### Scenario: Same-named nominal declarations do not collide
- **WHEN** a prepared IR program contains two modules that each declare a record named `User`
- **AND** an exported function parameter type references one of those records by module-qualified
  nominal type reference
- **THEN** the runtime SHALL normalize the argument using the referenced declaration
- **AND** it SHALL NOT select the other `User` declaration by bare name

#### Scenario: Non-entrypoint declarations are not public host API targets
- **WHEN** a prepared IR program contains a function declaration not listed in function
  entrypoints
- **AND** a caller evaluates that function by name through the public runtime API
- **THEN** the runtime SHALL fail with a missing entrypoint diagnostic
- **AND** it SHALL NOT fall back to global declaration-name lookup

### Requirement: TypeScript IR runtime accepts values produced by valid generated IR
The TypeScript IR runtime SHALL normalize record, union, and component values produced inside the
same prepared IR program with the same effective schema used for public boundary inputs. For valid
IR emitted from successfully analyzed source, runtime evaluation SHALL NOT fail with
`nx-ir-boundary-*` diagnostics for an absent optional union field, content-property fields, single
values at `+`- or `*`-typed fields, or record discriminators that were valid in native evaluation.

A single value at a `+`- or `*`-typed field SHALL normalize to a one-element array. This is the
language's own lift — the interpreter evaluates `xs={3.0}` and `xs={ <Item /> }` to one-element
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
- **AND** it SHALL equal the value the Rust interpreter produces for the same program

#### Scenario: Single-value list coercion matches the Rust interpreter
- **WHEN** the same NX program binds a single value to a `+`- or `*`-typed property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the
  Rust interpreter produces for it

#### Scenario: A constructed record binds to a record-typed property
- **WHEN** a prepared IR program constructs a record value and binds it to a property whose declared
  type is that record
- **THEN** the TypeScript IR runtime SHALL normalize the value against the declared record's fields
- **AND** it SHALL NOT report `nx-ir-boundary-field` for the carried `$type` discriminator

#### Scenario: A record-typed property default normalizes
- **WHEN** a component property of a record type declares a record-construction default
- **AND** a descriptor omits that property
- **THEN** the property SHALL hold the constructed default

#### Scenario: Record field normalization matches the Rust interpreter
- **WHEN** the same NX program binds a constructed record to a record-typed property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the
  Rust interpreter produces for it

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

### Requirement: TypeScript runtime renders primitives in the canonical text form
The TypeScript IR runtime SHALL evaluate a `text` node by converting its operand to the canonical
text form `implicit-primitive-conversions` defines for the primitive type the node names. For
`int`, `int32`, `int64`, `float64` and `boolean` that form is what JavaScript's `String()` produces
for the carried value. For `float32` the runtime SHALL print the shortest digits that round-trip to
the same `float32`, not the expansion of the `float64` the value is carried in. A `text` node whose
operand is not a number or boolean at run time, or that names a type outside that set, SHALL fail
with an `nx-ir-operator` diagnostic naming the node.

The `concat` operator SHALL take two strings and join them, and SHALL fail with an
`nx-ir-operator` diagnostic when either operand is not a string, because schema `4` places the
conversion in a `text` node rather than in `concat`. The validator SHALL accept node kind `20` with
the layout `[20, node, str]` and SHALL reject it with any other layout.

Numeric comparison and arithmetic between values analysis widened need no runtime work, since the
runtime carries every numeric type as a JavaScript `number`; the runtime SHALL continue to compare
and compute them numerically.

#### Scenario: A text node over an int prints its digits
- **WHEN** a prepared program evaluates a `concat` whose right operand is a `text` node naming `int`
  over a slot holding `3`, with the left operand the string `"Total: "`
- **THEN** the result SHALL be `"Total: 3"`

#### Scenario: A text node over a float32 prints as a float32
- **WHEN** a prepared program evaluates a `text` node naming `float32` over a slot holding the
  `float64` widening of the nearest `float32` to `0.1`
- **THEN** the result SHALL be `"0.1"`

#### Scenario: An integral float prints without a fraction
- **WHEN** a prepared program evaluates a `text` node naming `float64` over a slot holding `1`
- **THEN** the result SHALL be `"1"`

#### Scenario: A boolean prints as true or false
- **WHEN** a prepared program evaluates a `text` node naming `boolean` over a slot holding `true`
- **THEN** the result SHALL be `"true"`

#### Scenario: concat over a non-string operand is refused
- **WHEN** a malformed image supplies a `concat` whose operand evaluates to a number
- **THEN** evaluation SHALL fail with an `nx-ir-operator` diagnostic

#### Scenario: Mixed numeric comparison compares numerically
- **WHEN** a prepared program evaluates `n < x` emitted from `n:int` holding `2` and `x:float64`
  holding `2.5`
- **THEN** the result SHALL be `true`

### Requirement: TypeScript IR runtime behavior is validated against existing NX semantics
The implementation SHALL include automated tests that emit NX IR from source or program artifacts,
execute the IR through the TypeScript runtime, and compare results against native interpreter
evaluation for the supported non-reactive subset. Component tests SHALL cover descriptor
construction, initialization, explicit state evaluation, state patch validation, conditional
content based on state, lone-child content binding at each occurrence, and dispatch.

#### Scenario: Function parity test compares interpreter and IR runtime
- **WHEN** a supported NX program uses primitives, arithmetic, conditionals, match expressions,
  sequences, optional values and the presence operators, loops, records, unions, enums, member
  access, and function calls
- **THEN** automated tests SHALL verify that TypeScript IR runtime output matches native
  interpreter output for the same program

#### Scenario: Component parity test compares interpreter and IR runtime
- **WHEN** a supported NX component uses props, state defaults, conditional content, child
  component descriptors, and explicit state evaluation
- **THEN** automated tests SHALL verify that TypeScript IR runtime rendered output matches native
  interpreter rendered output for the same props and state

#### Scenario: Dispatch parity test compares interpreter and IR runtime
- **WHEN** a component binds handlers that patch its state, emit to its parent, and return effects
- **AND** the same batches of handler invocations and emitted actions are dispatched through both
  runtimes
- **THEN** automated tests SHALL verify that the rendered output, including tokens, and the
  ordered effects match between the TypeScript runtime and the native interpreter

### Requirement: TypeScript runtime normalizes update records as patches
When the TypeScript runtime normalizes a value whose declared type is an update record — from an
IR construction expression or from host input at a boundary — it SHALL keep absent fields absent
rather than filling them from defaults or with the empty value, SHALL decode a present `null` or
empty array as a present empty field only for fields the declaration marks clearable and SHALL
reject it naming the field otherwise, SHALL reject unknown fields, SHALL stamp the value with the
update record's `$type`, and SHALL encode a present empty field as `null` in canonical output, as
`update-records` requires.

#### Scenario: Evaluated update record omits unsupplied fields
- **WHEN** a prepared IR program evaluates `<User.Update email={} />` for `type User = { name:string = "anon" email?:string }`
- **THEN** the result SHALL be `{ $type: "User.Update", email: null }`
- **AND** the result SHALL NOT have a `name` property

#### Scenario: Host input for an update record keeps absence
- **WHEN** a caller passes `{ $type: "User.Update", name: "Ada" }` where a `User.Update` prop is expected
- **THEN** normalization SHALL accept the value without reporting `email` as missing
- **AND** the normalized value SHALL NOT have an `email` property

#### Scenario: Null for a non-nullable update field is rejected
- **WHEN** a caller passes `{ $type: "User.Update", name: null }` or `{ $type: "User.Update", name: [] }` for a `User` whose `name` is `string`
- **THEN** normalization SHALL fail with a diagnostic naming `name`

#### Scenario: A program that uses update records requires the feature
- **WHEN** a prepared IR program lists the update-record feature in its required features
- **THEN** a runtime that supports this change SHALL prepare it
- **AND** the feature name SHALL appear in the diagnostic a runtime without support reports

### Requirement: TypeScript runtime applies a component update record to host-owned state
The state-patch operation SHALL accept the component's update record value, in addition to a plain
partial state object, and SHALL apply it with the same semantics: present fields replace the
current value, absent fields keep it, a present empty value clears an optional state field so the
next state carries no key for it, and the result is validated against the state schema. A present
empty value for a state field that is not optional SHALL be rejected naming the field.

#### Scenario: Component update record patches state
- **WHEN** a caller applies `{ $type: "Counter.Update", count: 3 }` to current state `{ count: 1, label: "x" }` for prepared component `Counter`
- **THEN** the runtime SHALL return next state `{ count: 3, label: "x" }`

#### Scenario: A present empty clears an optional state field
- **WHEN** a caller applies `{ $type: "Search.Update", query: null }` to current state `{ query: "docs" }` for a prepared component whose state declares `query?:string`
- **THEN** the runtime SHALL return a next state with no `query` key
- **AND** the same patch against a component whose state declares `query:string` SHALL fail with a diagnostic naming `query`

#### Scenario: An update record for a different component is rejected
- **WHEN** a caller applies a value whose `$type` is `Other.Update` to `Counter` state
- **THEN** the runtime SHALL fail with a diagnostic naming the mismatched update record

### Requirement: TypeScript runtime normalizes property union values as constant cases
When the TypeScript runtime normalizes a value whose declared type is a property union — from an
IR union-case expression or from host input at a boundary — it SHALL accept the bare string of any
case listed in the property union's declaration, SHALL reject any other value with a diagnostic
naming the property union and the offending value, and SHALL keep the value as that bare string.
A prepared program that lists the property-union feature in its required features SHALL be
accepted by a runtime that supports this change, and the feature name SHALL appear in the
diagnostic a runtime without support reports.

#### Scenario: Evaluated property union case is the bare field name
- **WHEN** a prepared IR program evaluates `User.Property.email` for `type User = { name:string email?:string }`
- **THEN** the result SHALL be the string `"email"`

#### Scenario: Host input for a property union is validated against the cases
- **WHEN** a caller passes `"nickname"` where a `User.Property` prop is expected
- **THEN** normalization SHALL fail with a diagnostic naming `User.Property` and `nickname`
- **AND** passing `"name"` SHALL be accepted unchanged

### Requirement: TypeScript runtime evaluates the update intrinsics and exports them as helpers
The TypeScript runtime SHALL evaluate the intrinsic call construct for `apply`, `merge`, `diff`,
and `changed` with the semantics the `update-records` capability specifies: present fields
replace, absent fields keep, a present empty field is carried as present and empty, `merge` lets
the second argument win, `diff` lists only differing fields comparing records and sequences
structurally, and `changed` lists present fields in declaration order. The result of `apply` SHALL
be stamped with the target record's `$type`, the results of `merge` and `diff` with the update
record's `$type`, and the result of `changed` SHALL be an array of bare strings. Applying a present
empty field to a record SHALL leave that field absent from the result's keys, which is how the
canonical encoding writes an empty optional field. The runtime SHALL also export the same four
operations as functions a host can call on values it holds, typed so that the record and update
arguments share one record type parameter. A prepared program that lists the update-intrinsic
feature SHALL be accepted by a runtime that supports this change.

#### Scenario: Evaluated apply replaces present fields only
- **WHEN** a prepared IR program evaluates `apply(<User name="Ada" email="x@y" />, <User.Update email={} />)` for `type User = { name:string email?:string }`
- **THEN** the result SHALL be `{ $type: "User", name: "Ada" }` with no `email` key

#### Scenario: Evaluated diff and changed agree
- **WHEN** a prepared IR program evaluates `changed(diff(<User name="Ada" email="x@y" />, <User name="Bo" email="x@y" />))` for the same `User`
- **THEN** the result SHALL be `["name"]`

#### Scenario: Exported helper applies a host-held update
- **WHEN** a host calls the exported apply helper with `{ $type: "User", name: "Ada", email: "x@y" }` and `{ $type: "User.Update", email: null }`
- **THEN** the helper SHALL return `{ $type: "User", name: "Ada" }` with no `email` key
- **AND** the exported merge helper called with `{ $type: "User.Update", name: "Ada" }` and `{ $type: "User.Update", name: "Bo" }` SHALL return `{ $type: "User.Update", name: "Bo" }`

#### Scenario: Exported helpers reject mismatched targets
- **WHEN** a host calls the exported apply helper with a `User` record and an update whose `$type` is `Team.Update`
- **THEN** the helper SHALL fail with a diagnostic naming both types

### Requirement: The TypeScript IR runtime is published as an npm package
The TypeScript IR runtime SHALL be publishable and published to the npm registry as
`@nx-lang/ir-runtime`, so that a browser or Node host can evaluate persisted NX IR without an NX
checkout. The published package SHALL carry its type declarations.

#### Scenario: A host installs the runtime from the registry
- **WHEN** a JavaScript project installs the published `@nx-lang/ir-runtime` package
- **THEN** it SHALL be able to prepare an NX IR program and evaluate its `root` entrypoint, with
  TypeScript types available for the exported API

#### Scenario: Runtime and SDK versions agree
- **WHEN** a host installs `@nx-lang/ir-runtime` and `@nx-lang/sdk-wasm` at the same release version
- **THEN** IR emitted by the SDK SHALL satisfy the runtime's format, schema and required-feature
  checks

### Requirement: TypeScript runtime links an entry module against prepared modules
The TypeScript runtime SHALL expose a linking step that takes a prepared entry module and a resolver
the host supplies, asks the resolver for each module in the entry's module table by identity, and
returns a linked program. Linking SHALL check that every resolved module's version equals the version
the entry recorded, SHALL fail naming the module and both versions when they differ unless the host
opts into linking across versions, SHALL fail naming the module and declaration when a referenced
declaration is absent from the resolved module, and SHALL let the host reuse one prepared module
across any number of linked programs.

#### Scenario: A snippet links against a prepared catalog
- **WHEN** a host has prepared the `drawnui` artifact once
- **AND** links a snippet artifact whose module table names `drawnui` with the same version
- **THEN** linking SHALL succeed
- **AND** evaluating the snippet's `root` SHALL construct descriptors of `drawnui`'s components

#### Scenario: A version mismatch is refused by default
- **WHEN** the snippet recorded `drawnui` version `9` and the resolver returns version `10`
- **THEN** linking SHALL fail with a diagnostic naming `drawnui`, `9` and `10`

#### Scenario: A host may link across versions
- **WHEN** the host opts into linking across versions for the same mismatch
- **AND** every declaration the snippet references exists in version `10`
- **THEN** linking SHALL succeed

#### Scenario: A missing declaration is refused
- **WHEN** the resolved `drawnui` does not declare a component the snippet references
- **THEN** linking SHALL fail with a diagnostic naming `drawnui` and the missing declaration

#### Scenario: A module the resolver cannot supply is refused
- **WHEN** the resolver returns nothing for a module in the table
- **THEN** linking SHALL fail with a diagnostic naming that module's identity

#### Scenario: One prepared module serves many programs
- **WHEN** a host links a hundred snippet artifacts against the same prepared `drawnui`
- **THEN** `drawnui` SHALL be prepared once
- **AND** each linked program SHALL evaluate independently

### Requirement: TypeScript runtime passes the conformance corpus
The TypeScript runtime's tests SHALL run every artifact in the conformance corpus and compare each
named entrypoint's canonical value, and each lifecycle's rendered outputs and effects, against the
corpus's expected results, including the artifacts emitted without a debug section and those that
link to another corpus artifact.

#### Scenario: The corpus is part of the runtime's test run
- **WHEN** the runtime's tests run
- **THEN** every corpus artifact SHALL be prepared, linked where its module table requires, and
  evaluated
- **AND** every corpus lifecycle SHALL be initialized and its batches dispatched in order
- **AND** a difference from an expected result SHALL fail the run naming the program and the
  entrypoint or lifecycle

### Requirement: TypeScript runtime renders function values and calls them
The TypeScript runtime SHALL read the function type kind, the named-call node and the
`function-values-v1` feature. A `reference` node naming a function SHALL evaluate to a function
value wherever it appears, a named-call node SHALL invoke the function its callee evaluates to
with the arguments bound by name under the subset rule, and canonical output SHALL render a
function value as the `Function` record `function-values` defines. A
`Function` record in host-supplied props — to descriptor construction or component
initialization — SHALL be accepted at a function-typed prop when it names a function declaration
of the linked program, and SHALL be refused with a diagnostic otherwise. The runtime SHALL export
`callFunction(program, value, args)`, which takes a `Function` record and an object of arguments
keyed by parameter name, binds each argument to the parameter of that name, drops an argument the
function does not declare, fails with a diagnostic naming the first parameter that is missing,
and returns the canonical result. A function value SHALL be equal to another only when both name
the same declaration.

#### Scenario: A rendered descriptor carries the Function record
- **WHEN** a linked program declares `let <Row Item:object Index:int />: string = "r"` in `main.nx` and `external component <List ItemTemplate?:<function Item:object Index:int />: string /> let root() = <List ItemTemplate={Row} />`
- **AND** `evaluateFunction(program, "root")` is called
- **THEN** the result's `ItemTemplate` SHALL be `{ "$type": "Function", "module": "main.nx", "name": "Row" }`

#### Scenario: callFunction invokes by name and drops extras
- **WHEN** the host passes that record to `callFunction(program, record, { Item: item, Index: 3, Extra: 1 })`
- **THEN** the runtime SHALL invoke `Row` with `Item` and `Index` bound and `Extra` dropped
- **AND** SHALL return the canonical result `"r"`

#### Scenario: callFunction reports a missing parameter
- **WHEN** the host passes that record to `callFunction(program, record, { Item: item })`
- **THEN** the call SHALL fail with a diagnostic naming `Index`

#### Scenario: A Function record reaches a child instance through its props
- **WHEN** a parent's rendered output carries a `Function` record on a function-typed prop of an
  authored child component
- **AND** the host initializes the child from those fields
- **THEN** initialization SHALL accept the record
- **AND** the child's body invoking that prop SHALL call the named function

#### Scenario: A Function record naming no declaration is refused
- **WHEN** the host supplies `{ "$type": "Function", "module": "main.nx", "name": "Nope" }` at a
  function-typed prop
- **THEN** the runtime SHALL fail with a diagnostic naming `Nope`

#### Scenario: A module needing function values is refused by an older runtime
- **WHEN** a runtime that does not implement `function-values-v1` prepares a module that lists it
- **THEN** preparation SHALL fail with a diagnostic naming the feature

### Requirement: TypeScript runtime supplies the prelude module itself
The TypeScript runtime package SHALL carry the compiled image of the prelude that its compiler
release emits, and linking SHALL supply it for the prelude's reserved identity whenever the host's
resolver returns no module for that identity — at any depth of the link, including for a module the
resolver did supply — so that no host has to ship, prepare or resolve the prelude. A module the
host's resolver does return for that identity SHALL be used instead. The built-in prelude SHALL be
prepared at most once and reused across linked programs, and preparing a self-contained program
SHALL succeed for an image whose only other module is the prelude. A declaration the image
references that the runtime's prelude lacks SHALL fail linking with the missing-declaration
diagnostic naming the prelude and the declaration. An image whose module table records a prelude
version other than the one the resolved prelude carries SHALL fail linking with the version
diagnostic, whether the prelude came from the built-in image or from the host, unless the host opts
into linking across versions. The checked-in image SHALL be regenerated from the prelude's source by
a documented command, and a test SHALL fail when it is stale.

#### Scenario: A self-contained program that builds a range
- **WHEN** a host prepares a program from one image whose module table lists only the prelude
- **THEN** preparation SHALL succeed with no resolver and the program SHALL evaluate

#### Scenario: A host resolver that knows nothing of the prelude
- **WHEN** a snippet and a catalog each reference `Range` and the host's resolver supplies the catalog alone
- **THEN** linking SHALL succeed, with the runtime's prelude serving both modules

#### Scenario: A host overrides the prelude
- **WHEN** the host's resolver returns a prepared module for the prelude's identity
- **THEN** linking SHALL use that module and SHALL NOT use the built-in one

#### Scenario: A prelude declaration the runtime lacks
- **WHEN** an image references a prelude declaration that the runtime's prelude does not hold
- **THEN** linking SHALL fail naming the prelude and that declaration

#### Scenario: An image compiled against another prelude contract
- **WHEN** an image records a prelude version other than the one this runtime's prelude carries, and references only declarations that prelude holds
- **THEN** linking SHALL fail with the version diagnostic naming the prelude
- **AND** linking SHALL succeed when the host opts into linking across versions

#### Scenario: The checked-in image is current
- **WHEN** the prelude's source changes and the runtime's image is not regenerated
- **THEN** a test SHALL fail naming the command that regenerates it

### Requirement: TypeScript runtime iterates a range without building it
The TypeScript runtime SHALL accept the required feature `ranges-v1` and SHALL evaluate a
`forRange` node by evaluating its iterable to a `Range` record and running the body once for each
integer from `start` upward — stopping before `end`, or after it when `endInclusive` is true — with
the item bound to that integer and the index, when present, bound to its position from zero. It
SHALL yield the list of body values, SHALL yield the empty list when the range holds no integers,
and SHALL NOT allocate the list of integers. A `forRange` iterable that does not evaluate to a
`Range` record with integer bounds SHALL fail evaluation with a diagnostic, as a `for` over a
non-list does.

The runtime SHALL refuse to iterate a range holding more integers than a limit the host may set in
its runtime options, with a default of one million, failing with the resource-limit diagnostic that
names the limit before running the body at all.

#### Scenario: A half-open and a closed range
- **WHEN** an image evaluates `for i in 0..4 { i * i }` and `for p in 1..=3 { p }`
- **THEN** the runtime SHALL return the lists `0 1 4 9` and `1 2 3`

#### Scenario: An empty range runs no body
- **WHEN** an image evaluates `for i in 5..2 { i }`
- **THEN** the runtime SHALL return the empty list

#### Scenario: A host-supplied range iterates
- **WHEN** a host passes `{ "$type": "Range", "start": 2, "end": 4, "endInclusive": true }` for a parameter typed `<Range T=int/>` and the function iterates it
- **THEN** the runtime SHALL run the body for `2`, `3` and `4`

#### Scenario: The names the runtime recognizes are the prelude's
- **WHEN** a program links against the built-in prelude
- **THEN** the prelude's `Range` declaration SHALL carry exactly the field names the runtime's range check requires, typed `object`, `object` and `boolean`

#### Scenario: An oversized range is refused
- **WHEN** an image evaluates `for i in 0..2000000 { i }` under the default options
- **THEN** evaluation SHALL fail with the resource-limit diagnostic naming the limit of one million
- **AND** the same image SHALL evaluate under options that raise the limit above two million

#### Scenario: An older runtime refuses the module by name
- **WHEN** a runtime that does not know `ranges-v1` prepares a module that lists it
- **THEN** preparation SHALL fail naming the feature `ranges-v1`

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

#### Scenario: Operator results match the Rust interpreter
- **WHEN** the same NX program uses `x?`, `x?.m`, `x ?? y` and a `{}` pattern
- **THEN** the values the TypeScript runtime produces SHALL match the values the Rust interpreter produces for the same inputs

#### Scenario: An operator node without the feature is malformed
- **WHEN** a hand-built schema-`5` image contains a `coalesce` node and does not list `occurrence-v1`
- **THEN** preparation SHALL fail with a diagnostic naming `occurrence-v1`

#### Scenario: An older runtime refuses the module by name
- **WHEN** a runtime that does not implement `occurrence-v1` prepares a module that lists it
- **THEN** preparation SHALL fail with a diagnostic naming the feature

### Requirement: TypeScript runtime accepts any function at a function reference site
The TypeScript runtime SHALL read the function reference type kind, validating its result operand,
and the `function-reference-type-v1` feature. Where it normalizes a value against a function
reference type — a host-supplied argument, prop, state value, patch or batch entry, a record or
component field, a parameter and a result — it SHALL accept a function value of the linked
program, and SHALL accept a host-supplied `Function` record,
`{ "$type": "Function", "module", "name" }`, when it names a function declaration of the linked
program, whatever that function's parameters. It SHALL NOT compare the function's result with the
type's result operand, as it compares no part of a signature at a site typed by a function type
with stated parameters. It SHALL refuse a `Function` record that names a module the program does
not link, or a name that module does not declare as a function, with the `nx-ir-function-value`
diagnostic naming the function, and SHALL refuse any other value — a string, a number, a boolean,
a record of another `$type`, a `Function` record without a string `module` and `name` — with the
`nx-ir-boundary-type` diagnostic naming the site. Occurrences over the type SHALL follow the rules
the runtime applies to an occurrence over any type. Canonical output SHALL render a value at such a
site as the `Function` record `function-values` defines. The runtime SHALL NOT call the function
when it normalizes, stores, compares or renders the value. `callFunction` SHALL be unchanged, so a
record read from such a field is callable with arguments keyed by the function's own parameter
names, each validated against that parameter's declared type. The package SHALL export a type
`NxFunctionRecord` describing the record: `$type` the literal `"Function"`, `module` a string and
`name` a string.

#### Scenario: A rendered function reference field carries the Function record
- **WHEN** a linked program declares, in `main.nx`, `type Tool = { fn: <function ... />: object* } let double(n:int): int = {n * 2} let root() = <Tool fn={double} />`
- **AND** `evaluateFunction(program, "root")` is called
- **THEN** the result SHALL be `{ "$type": "Tool", "fn": { "$type": "Function", "module": "main.nx", "name": "double" } }`

#### Scenario: A field with a stated result renders the same record
- **WHEN** a linked program declares, in `main.nx`, `type Args = { q:string } type Tool = { build: <function ... />: Args } let make(q:string): Args = <Args q={q} /> let root() = <Tool build={make} />`
- **AND** `evaluateFunction(program, "root")` is called
- **THEN** the result's `build` field SHALL be `{ "$type": "Function", "module": "main.nx", "name": "make" }`

#### Scenario: A host supplies a function of any signature
- **WHEN** the first program also declares `let greet(name:string, loud?:boolean): string = {name}` and `let pass(tool:Tool): Tool = {tool}`
- **AND** the host calls `evaluateFunction(program, "pass", [{ "$type": "Tool", "fn": { "$type": "Function", "module": "main.nx", "name": "greet" } }])`
- **THEN** the call SHALL succeed and the result's `fn` SHALL be that record

#### Scenario: A record naming no function is refused
- **WHEN** the host supplies `{ "$type": "Function", "module": "main.nx", "name": "Nope" }` at the `fn` field
- **THEN** the runtime SHALL fail with `nx-ir-function-value` naming `Nope`
- **AND** `{ "$type": "Function", "module": "other.nx", "name": "double" }`, where the program links no `other.nx`, SHALL fail with `nx-ir-function-value` naming `other.nx`

#### Scenario: A non-function at a function reference site is refused
- **WHEN** the host supplies `"double"`, `1`, `{ "$type": "Tool" }` or `{ "$type": "Function", "name": "double" }` at the `fn` field
- **THEN** each SHALL fail with `nx-ir-boundary-type` naming the field

#### Scenario: A value that names something other than a function is refused
- **WHEN** the host supplies `{ "$type": "Function", "module": "main.nx", "name": "Tool" }` at the `fn` field
- **THEN** the runtime SHALL fail with `nx-ir-function-value`, because `Tool` is a record and not a function

#### Scenario: Occurrences over a function reference type follow the ordinary rules
- **WHEN** a program declares `type AnyFn = <function ... />: object* type Kit = { all:AnyFn+ one?:AnyFn }` and the host supplies a `Kit` with `all` set to `[]`, and another with `one` omitted and `all` holding one record
- **THEN** the first SHALL be refused because `all` admits no empty value
- **AND** the second SHALL be accepted, with `one` normalized to the empty value

#### Scenario: A record from a function reference field is callable by its own parameters
- **WHEN** the host reads the `fn` record of the first scenario and calls `callFunction(program, record, { n: 4 })`
- **THEN** the call SHALL return `8`
- **AND** `callFunction(program, record, { n: "four" })` SHALL fail with a diagnostic naming `n`, because the argument does not have the parameter's declared type

#### Scenario: A module needing the type is refused by an older runtime
- **WHEN** a runtime that does not implement `function-reference-type-v1` prepares a module that lists it
- **THEN** preparation SHALL fail with a diagnostic naming the feature

#### Scenario: The corpus program passes
- **WHEN** the TypeScript runtime runs the function reference conformance program
- **THEN** every entrypoint SHALL produce the recorded result

### Requirement: TypeScript runtime enforces an operation budget
The runtime options every evaluation API accepts SHALL include `maxOperations`, the most
operations one call may cost, counted as `nx-ir-format` defines an operation. When the option is
absent the call SHALL be unlimited, so a host that does not set it sees no change. A value that is
not a non-negative safe integer SHALL be refused with a diagnostic naming the option before
anything is evaluated; it SHALL NOT be read as unlimited.

One budget SHALL cover one call of a public API and everything that call evaluates: the function
and every function it calls, parameter defaults, value initializers, a component's state defaults
and body, and, for `dispatchComponentActions`, every handler the batch runs and the render that
follows. The APIs are `evaluateFunction`, `callFunction`, `constructComponentDescriptor`,
`initializeComponent`, `evaluateComponent`, `dispatchComponentActions`, `normalizeComponentState`
and `applyComponentStatePatch`. Each call SHALL start with the whole budget; nothing SHALL carry
over from an earlier call.

A charge that would take the count above the budget SHALL fail the call with
`nx-ir-resource-limit` before the operation charged is performed. The diagnostic SHALL name the
declaration the charge belongs to, as `nx-ir-format` assigns it, and, when the charge is for a
node and the image carries its debug section, the node's span, and its `limit` SHALL be
`{ name: "maxOperations", value }` with the budget the host set. The call SHALL return nothing, and a failed dispatch SHALL leave the instance it was given
unchanged and usable, as for any other failure. A budget equal to the cost of the call SHALL
succeed.

#### Scenario: Nested loops end at the budget
- **WHEN** an image evaluates `for a in 0..1000 { for b in 0..1000 { for c in 0..1000 { a + b + c } } }` under `maxOperations: 100000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` is `{ name: "maxOperations", value: 100000 }`
- **AND** the diagnostic SHALL name the function the loops are written in
- **AND** no more than one hundred thousand operations SHALL have been performed

#### Scenario: An absent budget is unlimited
- **WHEN** an image evaluates `for i in 0..1000000 { i }` under options that do not set `maxOperations`
- **THEN** evaluation SHALL return the list of one million integers

#### Scenario: The budget is exact
- **WHEN** an evaluation costs `n` operations
- **THEN** it SHALL succeed under `maxOperations: n`
- **AND** it SHALL fail with `nx-ir-resource-limit` under `maxOperations: n - 1`

#### Scenario: A list that doubles on each call is stopped
- **WHEN** an image evaluates `let grow(n:int, xs:int+): int+ = { if n == 0 { xs } else { grow(n - 1, { xs xs }) } }` with `60` and a list of one integer under `maxOperations: 100000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`
- **AND** no list longer than one hundred thousand items SHALL have been built

#### Scenario: A string that doubles on each call is stopped
- **WHEN** an image evaluates `let grow(n:int, s:string): string = { if n == 0 { s } else { grow(n - 1, s + s) } }` with `60` and `"x"` under `maxOperations: 100000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`

#### Scenario: A value shared many times over is stopped wherever it is walked
- **WHEN** a function builds a record that holds one value twice, forty levels deep, and the value is checked against its type at each call, compared with another like it, or returned to the host, under `maxOperations: 100000`
- **THEN** each evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`
- **AND** SHALL NOT take time or memory that grows with the 2^40 values the record is as a tree

#### Scenario: A result too large for the budget is refused as it is written
- **WHEN** a function returns a list of 100 references to one string of 16,384 UTF-16 code units under `maxOperations: 1000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`
- **AND** the diagnostic SHALL name the function and carry no span

#### Scenario: A batch shares one budget
- **WHEN** a host dispatches a batch of three handler invocations, each of whose handlers costs more than a third of `maxOperations`
- **THEN** dispatch SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`
- **AND** the instance given SHALL dispatch a later, cheaper batch successfully

#### Scenario: A state default is under the budget
- **WHEN** a component's state field defaults to `for i in 0..100000 { i }` and a host initializes it under `maxOperations: 1000`
- **THEN** initialization SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`

#### Scenario: Each call starts with the whole budget
- **WHEN** a host evaluates a function costing 600 operations twice with the same options object, whose `maxOperations` is 1000
- **THEN** both evaluations SHALL succeed

#### Scenario: A budget that is not a count is refused
- **WHEN** a host evaluates a function under `maxOperations: NaN`, `maxOperations: -1` or `maxOperations: 1.5`
- **THEN** each call SHALL fail with a diagnostic naming `maxOperations`
- **AND** no node SHALL have been evaluated

### Requirement: TypeScript runtime resource-limit diagnostics name the limit
Every diagnostic the runtime reports with the code `nx-ir-resource-limit` SHALL carry a `limit`
with the `name` of the limit that was reached and, where the limit is a number, its `value`. The
names SHALL be `maxOperations`, `maxCallDepth` and `maxRangeLength` for the limits a host sets,
`maxExpressionNesting` for the fixed nesting bound, and `engine` for a limit of the JavaScript
engine, which has no value. A diagnostic with any other code SHALL carry no `limit`. The message
SHALL continue to state the limit in words.

#### Scenario: Runaway recursion names the call-depth limit
- **WHEN** a program evaluates a function that calls itself without end under the default options
- **THEN** the diagnostic's `limit` SHALL be `{ name: "maxCallDepth", value: 100 }`

#### Scenario: An oversized range names the range limit
- **WHEN** an image evaluates `for i in 0..2000000 { i }` under the default options
- **THEN** the diagnostic's `limit` SHALL be `{ name: "maxRangeLength", value: 1000000 }`

#### Scenario: A host tells the limits apart without the message
- **WHEN** one evaluation fails for an exhausted budget and another for runaway recursion
- **THEN** the two diagnostics SHALL have the same code and different `limit.name` values

### Requirement: TypeScript runtime bounds expression nesting and reports engine limits as diagnostics
The runtime SHALL refuse to nest expressions more than 1,000 deep across every call of one
evaluation, the bound the Rust runtime holds, failing with `nx-ir-resource-limit` whose `limit` is
`{ name: "maxExpressionNesting", value: 1000 }`. The bound SHALL be fixed: raising `maxCallDepth`
SHALL NOT raise it.

A `RangeError` the JavaScript engine raises while a public API evaluates — an exhausted call
stack, a string longer than the engine holds, an array longer than the engine holds — SHALL be
reported as an `NxIrRuntimeError` with the code `nx-ir-resource-limit` whose `limit` names
`engine`, and SHALL NOT reach the host as a `RangeError`. Placing the items of one sequence in
another, or the effects of a dispatched handler in a batch's effects, SHALL NOT depend on how many
arguments the engine accepts in one call.

#### Scenario: Recursion past a raised call depth is still a diagnostic
- **WHEN** a program evaluates a function that calls itself without end under `maxCallDepth: 1000000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxExpressionNesting` or `engine`
- **AND** the call SHALL NOT throw a `RangeError`

#### Scenario: A long list is spliced
- **WHEN** an image evaluates an element whose content is `for i in 0..200000 { i }`
- **THEN** evaluation SHALL return the element with two hundred thousand content items

#### Scenario: A handler returns many effects
- **WHEN** a host dispatches an action whose parent-bound handler returns 200,000 actions
- **THEN** dispatch SHALL return the 200,000 effects

#### Scenario: A string past the engine's limit is a diagnostic
- **WHEN** a function doubles a string on each of 40 nested calls under the default options
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit`
- **AND** the call SHALL NOT throw a `RangeError`

### Requirement: TypeScript runtime refuses an integer outside the safe range
The runtime carries every number as a JavaScript number and SHALL NOT hold an integer outside
JavaScript's safe range in any other form. Reading such a literal from an image SHALL fail with
`nx-ir-number`, naming the declaration and, with a debug section, the literal's span; the image
SHALL still prepare, and a path that does not reach the literal SHALL run. The refusal is charged
as any node is: under a budget the node's operation is taken before it fails.

A value the host passes that has the shape canonical JSON gives such an integer,
`{ "$type": "nx.int", "value": "<digits>" }`, SHALL be a record at `object`: returned unchanged,
compared as a record is, matched as a pattern as a record is, and charged as a record is, whatever
else it holds. At a parameter typed as an integer it SHALL be refused, as any value that is not a
number is. The runtime SHALL export nothing that makes such an integer.

#### Scenario: A literal outside the safe range is refused where it is read
- **WHEN** a prepared program evaluates `let big() = { 9007199254740993 }`
- **THEN** evaluation SHALL fail with `nx-ir-number` naming `big` and the literal
- **AND** another function of the same image that does not reach the literal SHALL evaluate

#### Scenario: Whatever would have used it does not run
- **WHEN** a function compares its argument with a literal outside the safe range, or matches it as a pattern
- **THEN** evaluation SHALL fail with `nx-ir-number`

#### Scenario: The record a host passes is a record
- **WHEN** a host passes `{ "$type": "nx.int", "value": "1152921504606846976" }` at `object` and the function returns it
- **THEN** the result SHALL be that record, for the cost of a record with one string field
- **AND** the same value at a parameter typed `int` SHALL fail with `nx-ir-boundary-type`

#### Scenario: Text or fields in that shape are paid for
- **WHEN** a host passes a record named `nx.int` whose `value` is a megabyte of text, or that holds a megabyte beside its `value`, and a function returns it 20 times
- **THEN** the call SHALL be refused under a budget of 100,000

### Requirement: TypeScript runtime limits the size of host input
The runtime options every evaluation API accepts SHALL include `maxInputSize`, the largest input
size one call may be given, measured as `nx-ir-format` defines the input size of a call. When the
option is absent the input SHALL be unlimited and the runtime SHALL NOT measure it, so a host that
does not set it sees no change. A value that is not a non-negative safe integer SHALL be refused
with a diagnostic naming the option before anything is measured or evaluated.

The limit SHALL cover what the host passes to one call: the arguments of `evaluateFunction`; the
function record and the arguments of `callFunction`; the props and the content of
`constructComponentDescriptor`; the props of `initializeComponent` and the state its options carry;
the props and the state of `evaluateComponent`; the batch of `dispatchComponentActions`; the state
of `normalizeComponentState`; and the current state and the patch of `applyComponentStatePatch`. A
component instance, and the parent instance of `initializeComponent`, SHALL NOT be measured.

A call whose input is larger than the limit SHALL fail with `nx-ir-resource-limit` whose `limit`
is `{ name: "maxInputSize", value }` with the limit the host set, before the program is looked
at, before any value is checked against a type and before any node is evaluated. The runtime SHALL
stop measuring as soon as the size passes the limit, within the bound `nx-ir-format` sets, and
SHALL measure without recursion, so a value that is too large, that nests deeply, or that holds
itself SHALL be refused by the limit and SHALL NOT exhaust the engine's stack or memory first.
Measuring the input SHALL charge nothing to `maxOperations`. A refused dispatch SHALL leave the
instance it was given usable.

#### Scenario: Input over the limit is refused before anything runs
- **WHEN** a host evaluates a function with a list of 20,000 integers under `maxInputSize: 1000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` is `{ name: "maxInputSize", value: 1000 }`
- **AND** no node SHALL have been evaluated and no argument checked against its parameter's type

#### Scenario: Input at the limit is accepted
- **WHEN** the input of a call has the size `n`
- **THEN** the call SHALL proceed under `maxInputSize: n`
- **AND** SHALL be refused under `maxInputSize: n - 1`

#### Scenario: An absent limit measures nothing
- **WHEN** a host evaluates a function with a list of one million integers under options that do not set `maxInputSize`
- **THEN** evaluation SHALL proceed as it did before the option existed

#### Scenario: A value that holds itself is refused by the limit
- **WHEN** a host passes an object one of whose fields is the object itself, under `maxInputSize: 1000`
- **THEN** the call SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`
- **AND** SHALL NOT fail for a limit of the JavaScript engine

#### Scenario: A deeply nested value is refused by the limit
- **WHEN** a host passes an array nested 100,000 deep under `maxInputSize: 1000`
- **THEN** the call SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`

#### Scenario: Every kind of input is measured
- **WHEN** a host passes more than the limit as props, as content, as a state, as a batch, or as a state patch
- **THEN** the call SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`

#### Scenario: A batch is measured whole before any of it runs
- **WHEN** a host dispatches a batch whose first entry is small and whose second is larger than the limit
- **THEN** the dispatch SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`
- **AND** no handler SHALL have run

#### Scenario: Oversized input is reported before any other fault of the call
- **WHEN** a host evaluates a component with props of the wrong type and a state larger than the limit, or names an entrypoint the program lacks with arguments larger than the limit
- **THEN** each call SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`

#### Scenario: An instance is not measured
- **WHEN** a host dispatches a batch of one small entry against an instance whose state holds 50,000 values, under `maxInputSize: 100`
- **THEN** the dispatch SHALL proceed

#### Scenario: The limit and the budget are told apart
- **WHEN** one call fails for its input and another for its operation budget
- **THEN** the two diagnostics SHALL have the same code and different `limit.name` values

#### Scenario: A limit that is not a size is refused
- **WHEN** a host evaluates a function under `maxInputSize: NaN`, `maxInputSize: -1` or `maxInputSize: 1.5`
- **THEN** each call SHALL fail with a diagnostic naming `maxInputSize`
- **AND** nothing SHALL have been measured or evaluated

### Requirement: TypeScript runtime reports what a call used
The runtime options every evaluation API accepts SHALL include `usage`, an object of the host's
that the runtime writes to. On every call given one, the runtime SHALL first remove `operations`
and `inputSize` from it, and when the call ends, whether it returns or throws, SHALL set
`operations` to the operations the call used if `maxOperations` was set, and `inputSize` to the
input size of the call if `maxInputSize` was set and the input was within it. The numbers SHALL be
those `nx-ir-format` defines; for a call that fails for its budget, the charge that was refused
SHALL NOT be counted. With `usage` absent the runtime SHALL do nothing for it, and with
`maxOperations` absent it SHALL count nothing for it. The object SHALL NOT be measured as input.

A `usage` that is not an object the runtime can write to SHALL be refused with `nx-ir-options`
before anything is measured or evaluated: a value that is no object, or an object to which the
runtime cannot write both members and remove them again at the start of the call, as a frozen
object or one that cannot be extended. The write at the end of a call SHALL
NOT change the call's outcome: if it fails, the call SHALL still return what it returned or throw
what it threw. One object given to calls that overlap holds the numbers of whichever ended last;
a host that wants each call's numbers gives each call its own.

#### Scenario: A successful call reports its count
- **WHEN** a host evaluates a function that costs 29 operations under `{ maxOperations: 100000, usage }`
- **THEN** `usage.operations` SHALL be 29 when the call returns

#### Scenario: A call that fails reports what it used
- **WHEN** a host evaluates a function under `{ maxOperations: 10, usage }` and it fails for the budget
- **THEN** the call SHALL throw as it does without `usage`
- **AND** `usage.operations` SHALL be at most 10

#### Scenario: A refused charge is not reported
- **WHEN** a call under `{ maxOperations: 10, usage }` has been charged 5 operations and its next charge, 31 for a concatenation, is refused
- **THEN** `usage.operations` SHALL be 5, as the Rust runtime reports

#### Scenario: A sink that cannot be written is refused
- **WHEN** a host passes `usage: Object.freeze({})`, `usage: Object.preventExtensions({})`, or `usage: 5`
- **THEN** the call SHALL fail with `nx-ir-options` naming `usage`
- **AND** nothing SHALL have been measured or evaluated

#### Scenario: Writing the report never replaces the outcome
- **WHEN** a call fails with a diagnostic and the write to `usage` at its end throws
- **THEN** the call SHALL throw the diagnostic it failed with

#### Scenario: A dispatch reports one number for the batch
- **WHEN** a host dispatches a batch of three entries under a budget with `usage`
- **THEN** `usage.operations` SHALL be what the three handlers and the render after them used together

#### Scenario: The input size is reported when it is measured
- **WHEN** a host evaluates a function with a list of 1,000 integers under `{ maxInputSize: 5000, usage }`
- **THEN** `usage.inputSize` SHALL be 1,001
- **AND** under options that do not set `maxInputSize` it SHALL be absent

#### Scenario: A report does not carry over
- **WHEN** a host passes one `usage` object to a call under a budget and then to a call with none
- **THEN** after the second call `usage.operations` SHALL be absent

### Requirement: TypeScript runtime exports the input measure
The runtime SHALL export `measureInputSize(value, limit?)`, which returns the size of one value as
`nx-ir-format` defines it and as `maxInputSize` measures it. Given a limit, it SHALL stop as soon
as the size passes the limit and return a number greater than the limit, within the bound on work
that `nx-ir-format` sets for the limit. It SHALL evaluate nothing and need no program. An object
measured this way is measured as one record, which is how the props, the state, the patch and the
arguments by name of a call are measured; an array is measured as the list it is, one more than
its items add to a call that takes them as its positional arguments, content or batch.

#### Scenario: A value measured alone has the size it has in a call
- **WHEN** a host measures a value with `measureInputSize` and then passes it as the one argument of a function under `{ maxInputSize, usage }`
- **THEN** the two sizes SHALL be equal

#### Scenario: Measuring stops at the limit
- **WHEN** a host measures a list of one million integers with a limit of 1,000
- **THEN** the result SHALL be greater than 1,000
- **AND** no more than the limit allows of the list SHALL have been read

### Requirement: TypeScript runtime diagnostics name the argument a failure is in
A diagnostic for a failure the runtime finds in a value a host passed as an argument of the
function it called, through `callFunction` or `evaluateFunction`, SHALL carry `argument`: the name
of the parameter the value was passed for. That covers a value that does not fit the parameter's
type, at any depth inside the value; any other refusal of the value itself, such as a `Function`
record in it that names no function; and a required parameter that was given no argument. The
name SHALL be the parameter's declared name, whether the host passed the arguments by name or by
position.

A diagnostic SHALL carry no `argument` when the failure is not in a value the host passed for one
parameter, even when the runtime finds it while it is checking an argument:

- a failure raised by a default, a parameter's or a record field's: by evaluating its expression,
  by a function it calls, or by its value not fitting the type it is declared with;
- a resource limit, whichever limit it is and whenever it is reached, the budget spent while an
  argument is checked included;
- a failure of the image or the program that the runtime finds while it checks a value, such as a
  schema or a reference it cannot resolve;
- a failure while the function's body is evaluated, including one in the arguments of a function
  the body calls;
- a failure in the function's result;
- more positional arguments than the function has parameters;
- the `Function` record `callFunction` is given to say which function to call, when it is not such
  a record or names no function: it is not an argument of the function;
- a failure of any other entry point.

The member is data for a program to read. The message SHALL continue to name the argument in
words, and no diagnostic's code SHALL change.

#### Scenario: A value of the wrong type names its parameter
- **WHEN** a host calls `let findPlans(teamSize:int, note?:string): string` through `callFunction`
  with `{ teamSize: "five" }`
- **THEN** the call SHALL fail with `nx-ir-boundary-type` and the diagnostic's `argument` SHALL be
  `teamSize`

#### Scenario: A failure deep inside a value names the parameter that holds it
- **WHEN** a host calls a function whose parameter `request` is a record with a field `items` of
  records, and the third item's `quantity` is a string where an `int` is declared
- **THEN** the diagnostic's `argument` SHALL be `request`

#### Scenario: A missing required argument names its parameter
- **WHEN** a host calls that `findPlans` with `{}`
- **THEN** the call SHALL fail with `nx-ir-arguments` and the diagnostic's `argument` SHALL be
  `teamSize`

#### Scenario: A positional argument is named by its parameter
- **WHEN** a host calls `findPlans` through `evaluateFunction` with the positional arguments
  `["five"]`
- **THEN** the diagnostic's `argument` SHALL be `teamSize`

#### Scenario: A function record that names no function names its parameter
- **WHEN** a host calls a function whose parameter `step` is of a function type with the record
  `{ $type: "Function", module: "main.nx", name: "missing" }`
- **THEN** the call SHALL fail with `nx-ir-function-value` and the diagnostic's `argument` SHALL
  be `step`

#### Scenario: The function record of the call itself names no argument
- **WHEN** a host calls `callFunction` with the record
  `{ $type: "Function", module: "main.nx", name: "missing" }` as the function to call
- **THEN** the call SHALL fail with `nx-ir-function-value` and the diagnostic SHALL carry no
  `argument`

#### Scenario: A failure a field default raises names no argument
- **WHEN** a program declares `type Req = { n:int share:int = { 100 / n } }` and
  `let f(req:Req): int`, and a host calls `f` with `{ req: { $type: "Req", n: 0 } }`, a value that
  fits `Req`
- **THEN** the call SHALL fail with the code a division by zero has and the diagnostic SHALL carry
  no `argument`
- **AND** a field default whose value does not fit its field's type SHALL fail with a diagnostic
  that carries no `argument`, whatever its code

#### Scenario: A parameter's default names no argument
- **WHEN** a host calls `let g(n:int, share:int = { 100 / n }): int` with `{ n: 0 }`
- **THEN** the diagnostic SHALL carry no `argument`

#### Scenario: A default that fails with a code an argument's failure has names no argument
- **WHEN** a program declares
  `type Stepper = { step: <function n:int />: int value:int = { step(1) } }`,
  `let stepperValue(s:Stepper): int` and `let needsTwo(n:int, m:int): int`, and a host calls
  `stepperValue` with a `Stepper` whose `step` is the `Function` record of `needsTwo` and that has
  no `value`
- **THEN** the call SHALL fail with `nx-ir-arguments`, raised by the call the default makes, and
  the diagnostic SHALL carry no `argument`
- **AND** a call of
  `let stepped(step: <function n:int />: int, value:int = { <step n={1} /> }): int` with that
  record for `step` and nothing for `value` SHALL fail with `nx-ir-arguments` and carry no
  `argument`
- **AND** the same call of `stepped` with the string `one` for `value` SHALL fail with a diagnostic
  whose `argument` is `value`

#### Scenario: A failure inside the function names no argument
- **WHEN** a host calls a function with arguments that fit its parameters, and the function's body
  then fails, by dividing by zero or in a call it makes to another function
- **THEN** the diagnostic SHALL carry no `argument`

#### Scenario: A limit names no argument
- **WHEN** a host calls a function with input over `maxInputSize`, or with fifty records for a
  parameter `items:Item+` under an operation budget of twenty, so that the budget is spent while
  the argument is checked
- **THEN** each diagnostic SHALL carry its `limit` and no `argument`

### Requirement: TypeScript runtime reads a host value in its JavaScript form
A value a host passes to an evaluation API SHALL be read, at any depth, as a canonical value in
the form `nx-ir-format` requires this runtime to define, which is: `null`, a boolean, a number, a
string, an array, or a plain object, which is an object whose prototype is `Object.prototype`, of
this realm or another, or nothing. The handler and function values the runtime makes for its own
use, which a host passes back as it received them, SHALL be accepted where they are today. A host
SHALL NOT need to have read the value from JSON or to write it as JSON: an object and an array
the host built are the value. This SHALL hold for every value the input limit covers: arguments,
props, content, state, a state patch and the entries of a batch. What holds those values is the
host's too: positional arguments, content and a batch SHALL each be an array, and props, a state,
a state patch and arguments by name SHALL each be a plain object. Anything else in their place
SHALL be refused as a value that is not canonical is, with no `argument`, so that nothing the
input measure counts as one value is a list the call goes on to read. The types the evaluation
APIs declare for their input SHALL admit a member that is `undefined`, and the types of what they
return SHALL NOT.

A member of a plain object whose value is `undefined` SHALL be absent, everywhere the runtime
reads the object. The input measure SHALL NOT count it. The check SHALL treat it as a member that
was left out: a field with a default takes its default, an optional field is empty, a required
field with no default is reported missing, and a name the type does not declare is not reported
unknown. The program SHALL NOT be given it: a value held at a site typed `object` SHALL be held
without the member, and the host's own object SHALL NOT be changed.

A value that is not canonical SHALL be refused: after the input limit, so that input over the
limit is still refused by the limit, and before any value is checked against a type or any node
is evaluated. The failure SHALL be `nx-ir-boundary-type` with a message that names the path to
the value and says what it is not. Every value the call goes on to check or to hold SHALL be read
for this, whether or not a type would have looked at it: a value at a site typed `object`
included, at any depth inside it. A plain object or an array that holds itself SHALL be refused
the same way when no input limit refuses it first, and SHALL NOT exhaust the engine's stack or
memory; the same object held twice, side by side, is not a value that holds itself and SHALL be
accepted, and is read once for each place it is held, as the input size counts it. An item of an array that is `undefined` is not a member that can be absent: it SHALL be
read as it is today where the array is a call's positional arguments, and SHALL be refused
anywhere else.

A refusal of a value that is in what the host passed for a parameter of the function it called,
through `callFunction` or `evaluateFunction`, is a failure in that argument, and its diagnostic
SHALL carry `argument` as *TypeScript runtime diagnostics name the argument a failure is in*
requires. A refusal anywhere else SHALL carry none.

Reading the input SHALL charge nothing to `maxOperations` and SHALL NOT recurse more deeply than
a fixed bound, however deeply the input is nested. The input size
SHALL still be taken first, and `measureInputSize` SHALL still need no program: a value that is
not canonical counts one there and is refused after. No recorded result, operation count or input
size SHALL change for input that was accepted before and is accepted now.

#### Scenario: A value the host built is the value
- **WHEN** a host calls `let greet(person:Person): string`, where
  `type Person = { name:string nickname?:string title:string = "Dr" }`, with a `person` it built
  as an object literal, and again with one it read with `JSON.parse`
- **THEN** the two calls SHALL have the same result, operation count and input size

#### Scenario: An optional field set to undefined is absent
- **WHEN** a host calls `greet` with
  `{ person: { name: "Ada", nickname: undefined, title: undefined } }`
- **THEN** the call SHALL succeed, with `nickname` empty and `title` holding `Dr`
- **AND** its `inputSize` SHALL equal that of the call with `{ person: { name: "Ada" } }`

#### Scenario: A required field set to undefined is missing
- **WHEN** a host passes that function `{ person: { name: undefined } }`
- **THEN** the call SHALL fail with `nx-ir-boundary-field` naming the missing field `name`

#### Scenario: An undeclared member that is undefined is not unknown
- **WHEN** a host passes that function `{ person: { name: "Ada", extra: undefined } }`
- **THEN** the call SHALL succeed
- **AND** the same call with `extra: 1` SHALL fail with `nx-ir-boundary-field` naming `extra`

#### Scenario: An instance of a class is not a record
- **WHEN** a host passes that function a `person` that is an instance of a class with the members
  `name` and `nickname`
- **THEN** the call SHALL fail with `nx-ir-boundary-type` naming `person` and saying it is not a
  plain object
- **AND** the diagnostic's `argument` SHALL be `person`
- **AND** an object with no prototype holding the same members SHALL be accepted

#### Scenario: A value that is not canonical is refused inside an open object
- **WHEN** a host calls `let keep(extra:object): string` with `extra` holding, three levels down
  inside plain objects and arrays, a `Uint8Array` of five million bytes, or a `Date`, a `Map`, a
  function, a symbol or a big integer
- **THEN** each call SHALL fail with `nx-ir-boundary-type` naming the path to the value, with the
  `argument` `extra`
- **AND** the typed array SHALL NOT be read: the refusal SHALL take no longer than for one of
  eight bytes

#### Scenario: A refusal outside a function's arguments names no argument
- **WHEN** a host renders a component with props that hold a `Date`
- **THEN** the call SHALL fail with `nx-ir-boundary-type` naming the path to the value
- **AND** the diagnostic SHALL carry no `argument`

#### Scenario: An undefined member inside an open object is not given to the program
- **WHEN** a host calls `let echo(extra:object): object = { extra }` with
  `{ extra: { a: 1, b: undefined, c: { d: undefined } } }`
- **THEN** the result SHALL be `{ a: 1, c: {} }`
- **AND** the object the host passed SHALL still have its member `b`

#### Scenario: An open object that holds itself is refused
- **WHEN** a host passes `keep` a plain object one of whose members is the object itself, with no
  input limit set
- **THEN** the call SHALL fail with `nx-ir-boundary-type` and SHALL NOT exhaust the stack or fail
  to return

#### Scenario: The same object held twice is accepted
- **WHEN** a host passes `keep` `{ first: shared, second: shared }`, where `shared` is one plain
  object
- **THEN** the call SHALL succeed

#### Scenario: A list that is not an array is refused
- **WHEN** a host passes content or a batch as a `Set`, or positional arguments as an object with
  a `length`, whatever they hold and under any input limit they are within
- **THEN** each call SHALL fail with `nx-ir-boundary-type` saying an array was expected, with no
  `argument`
- **AND** nothing the `Set` or the object holds SHALL have been checked or evaluated

#### Scenario: A member set to undefined is written without a cast
- **WHEN** a TypeScript host passes `{ name: "Ada", nickname: undefined }` as an argument, as
  props, as a state or as a state patch
- **THEN** the call SHALL type-check against the package's declarations

#### Scenario: Input over the limit is refused by the limit first
- **WHEN** a host passes `keep` a value that holds a `Date` and is larger than `maxInputSize`
- **THEN** the call SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`
- **AND** the diagnostic SHALL carry no `argument`

#### Scenario: A handler the runtime made is still accepted
- **WHEN** a host renders a component, takes an action handler from the rendered output and passes
  it back as a prop of a child
- **THEN** the call SHALL be accepted as it is today

#### Scenario: Accepted input costs what it cost
- **WHEN** the conformance corpus is evaluated
- **THEN** every recorded result, operation count and input size SHALL be as recorded

### Requirement: TypeScript runtime carries a phase-level performance harness
The runtime package SHALL carry a benchmark harness, kept out of the published package, that times
each call a host makes on the conformance corpus's large programs: loading the runtime module,
preparing a program's images, linking, evaluating a zero-argument function, initializing a
component, initializing one from a stored state, evaluating one with a state, and dispatching an
action. It SHALL also time calls that take input: a call given at least 10,000 records as plain
data, as the same data with a member set to `undefined`, and as data nested deeper than the
runtime's check for plain data follows; and a call of a function by name with arguments under an
operation budget and an input limit.

The harness SHALL read the committed corpus images and SHALL NOT need the compiler. For each phase
it SHALL report the time of the first execution in a fresh isolate, the warm median and 95th
percentile with no limits set and with an operation budget and an input limit set, and the
operations and the input size the runtime's usage report gives, as a readable table and as JSON.

#### Scenario: A run reports every phase
- **WHEN** a contributor runs the harness
- **THEN** the report SHALL list every phase of each workload with its cold time, its warm median
  and 95th percentile with and without limits, its operations and its input size
- **AND** the same numbers SHALL be written as JSON

#### Scenario: The count in the report is the corpus's
- **WHEN** the harness runs a phase the corpus records an operation count for
- **THEN** the operations it reports SHALL equal the recorded count

#### Scenario: The harness needs no compiler
- **WHEN** the harness runs in a checkout where the compiler has not been built
- **THEN** it SHALL run every workload from the committed images

#### Scenario: The harness is not published
- **WHEN** the runtime package is packed
- **THEN** the tarball SHALL hold no file of the harness

### Requirement: The harness's workload steps run in any JavaScript engine
The steps the harness times SHALL be built by a module that imports nothing, reads no clock and
uses no Node API. Given a runtime module, a workload's images and its script, it SHALL return the
steps by phase, each of which makes its call once and returns what the usage report gave. So a
host can run the same steps in its own engine against its own build of the runtime and time them
as that engine allows, including an engine whose clock does not advance while code runs.

#### Scenario: A host runs the steps against its own runtime
- **WHEN** a host imports the module into an engine without Node APIs and passes its bundled
  runtime and the workload's images
- **THEN** each step SHALL run and return the operations and the input size of its call

#### Scenario: Two builds of the runtime run the same steps
- **WHEN** the module is given two different builds of the runtime in turn
- **THEN** it SHALL build the same steps for each, and neither SHALL affect the other

### Requirement: A change's effect on runtime speed is measured against the revision it is based on
The package SHALL provide a comparison that measures a base revision and the working tree on the
same machine in one run: each revision's committed runtime build against that revision's committed
corpus images, run alternately. The base SHALL default to the merge base with the main branch and
SHALL be nameable. No timing recorded on another machine SHALL be compared against.

For each phase the comparison SHALL report both warm medians, their ratio, how the ratio varied
between rounds and both operation counts. It SHALL name a phase as slower when the working tree is
slower than the base by more than a set fraction over the rounds, judged by the median of the
rounds' ratios and by most rounds agreeing, and SHALL list a phase that is faster by the same
measure. It SHALL compare in the same way each phase's first execution in a fresh isolate and the
time the runtime module takes to load in one. It SHALL report a phase as not comparable, and not
as slower, when the workload differs between the revisions, when the base has no such workload,
when the base runtime lacks a function the phase calls, or when the phase fails on either side.

Continuous integration SHALL run the comparison on a pull request, against the revision the
change is based on, and on a push to the main branch, against the last release, as a job that
reports and does not block a merge.

#### Scenario: A slower phase is named
- **WHEN** a change makes dispatch on the stateful program slower than the base revision by more
  than the allowed fraction
- **THEN** the comparison SHALL name that workload and phase with both medians and their ratio
- **AND** it SHALL name no phase the change did not slow

#### Scenario: A revision compared with itself names nothing
- **WHEN** the comparison is run with the base set to the working tree's own revision
- **THEN** it SHALL name no phase as slower or faster

#### Scenario: A change in operations is shown exactly
- **WHEN** a change makes a phase cost more operations than it does at the base revision
- **THEN** the report SHALL show both counts for that phase

#### Scenario: A workload that changed is not compared
- **WHEN** a change edits a workload's source or adds a workload the base revision does not have
- **THEN** the comparison SHALL report its phases as not comparable and SHALL succeed

#### Scenario: One wild round does not name a phase
- **WHEN** one round of an unchanged phase reads twice as slow as the base and the others agree
  with it
- **THEN** the comparison SHALL NOT name the phase

#### Scenario: A slower load is named
- **WHEN** a change makes the runtime module take longer to load by more than the allowed fraction
- **THEN** the comparison SHALL name the load time with both medians and their ratio

#### Scenario: Slowdowns that add up are named against the last release
- **WHEN** a push to the main branch leaves a phase slower than it was in the last release by more
  than the allowed fraction
- **THEN** the job SHALL name that phase in its report

#### Scenario: The job does not block a merge
- **WHEN** the comparison names a slower phase on a pull request
- **THEN** the job SHALL show the report on the pull request
- **AND** the pull request SHALL remain mergeable
