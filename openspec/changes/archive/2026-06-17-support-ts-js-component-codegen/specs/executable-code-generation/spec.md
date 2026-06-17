## ADDED Requirements

### Requirement: Generated code emits component descriptor functions and schema entry objects
Executable TypeScript and JavaScript code generation SHALL emit host-neutral component descriptor
functions and schema entry objects for concrete NX component declarations. Generated descriptor
functions SHALL construct atomic component descriptors. Generated schema entry objects SHALL expose
JSON initialization and evaluation APIs for selecting that component as an executable entry point.
Generated TypeScript MAY emit abstract/base contracts for abstract component declarations when
needed to preserve inherited props, but abstract components MUST NOT be directly instantiable or
evaluable.

#### Scenario: Concrete component emits a descriptor function and schema entry
- **WHEN** NX source declares `component <SearchBox placeholder:string /> = { <TextInput value={placeholder} /> }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include an exported `SearchBox` descriptor function
- **AND** generated output SHALL include a `SearchBoxSchema` entry object that can initialize and
  evaluate `SearchBox` from JSON-compatible props

#### Scenario: Abstract component is contract-only
- **WHEN** NX source declares `abstract component <SearchBase placeholder:string />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL NOT expose an API that directly initializes or evaluates
  `SearchBase`
- **AND** concrete descendants MAY use the generated base contract for inherited props

#### Scenario: Derived component preserves inherited props
- **WHEN** NX source declares `abstract component <SearchBase placeholder:string = "Docs" />`
- **AND** declares `component <SearchBox extends SearchBase /> = { <TextInput value={placeholder} /> }`
- **AND** a caller initializes `SearchBox` through generated output without an explicit
  `placeholder`
- **THEN** generated evaluation SHALL bind inherited prop `placeholder` to `"Docs"`
- **AND** SHALL return rendered output equivalent to interpreter component initialization

### Requirement: Component expressions emit atomic component descriptors
Executable TypeScript and JavaScript generation SHALL treat element expressions that resolve to
concrete components as atomic component descriptor construction. A component descriptor SHALL use
the canonical record-like component value shape with `$type` equal to the concrete component name
and fields for normalized props, content, and defaults. Constructing a component descriptor MUST
NOT evaluate that component's implementation body.

#### Scenario: Component expression returns descriptor without evaluating body
- **WHEN** NX source declares `component <Child label:string /> = { <Text value={label} /> }`
- **AND** declares `let root() = { <Child label="Name" /> }`
- **AND** a caller executes generated JavaScript for `root`
- **THEN** generated output SHALL return a descriptor with `$type` equal to `"Child"` and
  `label` equal to `"Name"`
- **AND** SHALL NOT evaluate `Child`'s component body while constructing that descriptor

#### Scenario: Parent component returns child descriptors
- **WHEN** NX source declares `component <Flow /> = { <Question label="Name" /> }`
- **AND** declares `external component <Question label:string />`
- **AND** a caller evaluates generated output for `Flow`
- **THEN** generated output SHALL return a `Question` component descriptor in the rendered output
- **AND** the descriptor SHALL preserve normalized prop `label`

#### Scenario: Component descriptor applies inherited defaults
- **WHEN** NX source declares `abstract external component <Question label:string = "Untitled" />`
- **AND** declares `external component <ShortTextQuestion extends Question placeholder:string? />`
- **AND** generated code evaluates `<ShortTextQuestion />`
- **THEN** the descriptor SHALL have `$type` equal to `"ShortTextQuestion"`
- **AND** SHALL include inherited prop `label` with value `"Untitled"`

### Requirement: Function element expressions still evaluate eagerly
Executable TypeScript and JavaScript generation SHALL preserve the existing function element-call
semantics. When an element expression resolves to a function, generated code SHALL call that
function and substitute its returned value into the surrounding expression rather than returning a
descriptor for the function itself.

#### Scenario: Function element call substitutes returned component descriptor
- **WHEN** NX source declares `external component <Question label:string />`
- **AND** declares `let MakeQuestion(label:string) = { <Question label={label} /> }`
- **AND** declares `let root() = { <MakeQuestion label="Name" /> }`
- **THEN** generated output for `root` SHALL return a `Question` component descriptor
- **AND** SHALL NOT return a descriptor whose `$type` is `"MakeQuestion"`

### Requirement: Generated component state uses generated types and helpers
Executable TypeScript and JavaScript generation SHALL emit state types and helper functions for
each concrete component that declares state. Generated schema entry objects SHALL normalize
JSON-compatible state input, and generated helpers SHALL materialize initial state from normalized
props and state defaults. Abstract components SHALL NOT emit state helpers.

#### Scenario: Stateful component emits state helper surface
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include a `SearchBox` descriptor function and `SearchBoxSchema`
  entry object
- **AND** SHALL include a `SearchBoxState` type or equivalent exported state helper surface
  associated with `SearchBox`

#### Scenario: Initialization returns rendered output and initial state
- **WHEN** a caller initializes generated `SearchBox` without explicit props or state
- **THEN** generated output SHALL materialize prop `placeholder` as `"Find docs"`
- **AND** SHALL materialize initial state `query` as `"Find docs"`
- **AND** SHALL return both rendered output and the JSON-compatible initial state

#### Scenario: Evaluation with omitted state uses initial state
- **WHEN** a caller evaluates generated `SearchBox` without supplying state
- **THEN** generated output SHALL materialize the same initial state used by initialization
- **AND** SHALL evaluate the component body with that state

#### Scenario: Evaluation with explicit state uses supplied state
- **WHEN** a caller evaluates generated `SearchBox` with state `{ query: "docs" }`
- **THEN** generated output SHALL evaluate the component body with `query` equal to `"docs"`
- **AND** SHALL NOT reevaluate the `query` default in place of the supplied state

#### Scenario: Abstract component emits no state helper
- **WHEN** NX source declares `abstract component <SearchBase placeholder:string />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL NOT include a `SearchBaseState` type or state helper

### Requirement: Generated component APIs provide throwing and result-returning variants
Generated component entry APIs SHALL provide primary initialization and evaluation operations that
throw typed `NxRuntimeError` failures for invalid host input or generated runtime errors. Generated
component entry APIs SHALL also provide result-returning variants that report the same failures as
diagnostics without throwing.

#### Scenario: Invalid props throw typed runtime error
- **WHEN** a generated component requires prop `label:string`
- **AND** a caller evaluates the component with a JSON value that omits `label`
- **THEN** the throwing generated `evaluateJson` API SHALL throw an `NxRuntimeError`
- **AND** the error SHALL include a diagnostic code or message identifying the missing prop

#### Scenario: Try evaluation returns diagnostics
- **WHEN** a generated component requires prop `label:string`
- **AND** a caller evaluates the component through `tryEvaluateJson` with a JSON value that omits
  `label`
- **THEN** the result SHALL indicate failure without throwing
- **AND** SHALL include diagnostics equivalent to the throwing API failure

### Requirement: Generated component code rejects action-handler bindings
Executable TypeScript and JavaScript generation SHALL reject component action-handler bindings in
this change. When generated code would need to preserve, serialize, or invoke an action handler,
codegen MUST return diagnostics and MUST NOT emit silently incomplete executable output.

#### Scenario: Component action handler binding is rejected
- **WHEN** NX source declares `component <SearchBox emits { SearchSubmitted } /> = { <TextInput /> }`
- **AND** an expression constructs `<SearchBox onSearchSubmitted=<DoSearch query={action.query} /> />`
- **AND** a caller requests executable TypeScript or JavaScript output
- **THEN** codegen SHALL fail with a diagnostic that action-handler codegen is unsupported
- **AND** SHALL NOT emit executable output that drops the handler binding

### Requirement: Generated component behavior is validated against runtime semantics
The executable code generation implementation SHALL include tests that compare generated component
descriptor construction and generated component entry evaluation with existing NX runtime
semantics for supported non-reactive component scenarios.

#### Scenario: Component descriptor parity
- **WHEN** a supported NX program constructs a component descriptor through ordinary expression
  evaluation
- **AND** the same program is emitted and executed as generated JavaScript or TypeScript
- **THEN** both executions SHALL produce equivalent JSON-compatible component descriptor payloads

#### Scenario: Component entry evaluation parity
- **WHEN** a supported NX component is evaluated by the interpreter with explicit props and state
- **AND** the same component is emitted and evaluated through generated JavaScript or TypeScript
- **THEN** both executions SHALL produce equivalent rendered payloads

## MODIFIED Requirements

### Requirement: Initial executable generation is eager and non-reactive
Executable TypeScript and JavaScript generated by this change SHALL use eager evaluation semantics
for supported NX expressions. Generated component initialization and evaluation APIs MAY evaluate a
selected concrete component entry body eagerly, using normalized props and state. Component
descriptor construction MUST remain atomic: constructing `<Component ... />` SHALL NOT evaluate
that component's implementation body. The generated runtime helper surface MUST NOT expose
reactive dependency tracking, invalidation, subscription, signal, dispatch, state-update reducer,
or action-handler invocation APIs as part of this change. Unsupported reactive, dispatch,
state-update, or action-handler constructs, if encountered, MUST produce codegen diagnostics rather
than implicit behavior.

#### Scenario: Generated expressions evaluate eagerly
- **WHEN** generated output evaluates an NX expression containing conditionals, loops, arrays, or
  function calls
- **THEN** the expression SHALL be evaluated when the generated entrypoint is invoked
- **AND** the generated output SHALL NOT register reactive dependencies for later invalidation

#### Scenario: Reactive semantics are not advertised
- **WHEN** a caller inspects the generated runtime helper output for this change
- **THEN** the helper surface SHALL NOT include public signal, subscription, invalidation, or
  dependency-graph APIs

#### Scenario: Generated component APIs are non-reactive
- **WHEN** a caller inspects generated component descriptor functions, schema entries, and runtime
  helper output
- **THEN** the generated surface MAY include schema component initialization and evaluation APIs
- **AND** SHALL NOT include public dispatch, state-update reducer, subscription, or action-handler
  invocation APIs

#### Scenario: Component descriptor construction does not deep-render children
- **WHEN** a generated component body constructs `<Child />`
- **THEN** generated output SHALL construct a `Child` descriptor eagerly
- **AND** SHALL NOT evaluate `Child`'s implementation body unless `Child` is separately selected
  as the component entry being initialized or evaluated
