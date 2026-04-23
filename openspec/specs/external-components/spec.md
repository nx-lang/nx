# external-components Specification

## Purpose
Defines external component declarations that expose public UI contracts without NX implementation, including evaluation as typed component records, stateless lifecycle bindings, and host serialization identity preservation.
## Requirements
### Requirement: External components expose public UI contracts without NX implementation
The system SHALL treat an `external component` declaration as a component contract consisting of its
effective props, prop defaults, content props, emitted actions, and optional declared host-managed
state. External components SHALL NOT require an NX render body in order to participate in
invocation checking, and declared external state SHALL NOT become part of the component's
invocation surface.

#### Scenario: Concrete external component is usable from NX call sites
- **WHEN** a file contains `external component <SearchBox placeholder:string = "Find docs" showSearchIcon:bool = true /> let render() = <SearchBox />`
- **THEN** analysis SHALL accept the invocation of `SearchBox`
- **AND** SHALL treat `placeholder` and `showSearchIcon` as the external component's public props

#### Scenario: Abstract external contract can be extended by another external component
- **WHEN** a file contains `abstract external component <SearchBase placeholder:string emits { SearchRequested } /> external component <SearchBox extends SearchBase showSearchIcon:bool = true />`
- **THEN** analysis SHALL accept `SearchBox extends SearchBase`
- **AND** SHALL treat `SearchBox` as inheriting `placeholder`, `SearchRequested`, and `showSearchIcon`

#### Scenario: Declared external state is preserved without becoming a prop
- **WHEN** a file contains `external component <SearchBox placeholder:string /> = { state { query:string } } let render() = <SearchBox placeholder="Docs" />`
- **THEN** analysis SHALL accept the invocation of `SearchBox`
- **AND** SHALL treat `placeholder` as the external component's public prop
- **AND** SHALL preserve `query` as declared external state rather than as an invocable prop

### Requirement: Evaluating an external component produces a typed component record
When NX evaluates an element targeting a concrete external component, the interpreter SHALL produce
a typed record-like component value whose type name is the component name and whose fields are the
normalized effective props, inherited or defaulted prop values, content bindings, and any bound
emitted-action handlers. Declared external state SHALL remain host-managed metadata and SHALL NOT
introduce an NX render body or NX-evaluated state fields on that component value.

#### Scenario: Function returns an external component record with normalized defaults
- **WHEN** a file contains `external component <SearchBox placeholder:string = "Find docs" showSearchIcon:bool = true /> let render() = <SearchBox />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value with `placeholder="Find docs"` and `showSearchIcon=true`

#### Scenario: Derived external component record includes inherited and local props
- **WHEN** a file contains `abstract external component <SearchBase placeholder:string = "Find docs" /> external component <SearchBox extends SearchBase showSearchIcon:bool = true /> let render() = <SearchBox />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value that includes inherited prop `placeholder="Find docs"` and local prop `showSearchIcon=true`

#### Scenario: Bound emitted-action handlers are preserved on external component values
- **WHEN** a file contains `action SearchRequested = { query:string } action DoSearch = { query:string } external component <SearchBox emits { SearchRequested } /> let render() = <SearchBox onSearchRequested=<DoSearch query={action.query} /> />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value that retains a bound `onSearchRequested` handler

#### Scenario: Declared external state does not add NX-evaluated record fields
- **WHEN** a file contains `external component <SearchBox placeholder:string /> = { state { query:string } } let render() = <SearchBox placeholder="Docs" />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value with `placeholder="Docs"`
- **AND** SHALL NOT require an NX render body or an NX-evaluated `query` field on that component value

### Requirement: External components are stateless in lifecycle bindings
The public NX component lifecycle bindings SHALL treat concrete external components as NX-stateless
contract instances even when they declare host-managed state. Initialization SHALL return the typed
external component record and an empty NX-managed component-state snapshot. Dispatch SHALL validate
declared emitted actions and invoke any bound handlers without requiring an NX render body or
NX-managed local state.

#### Scenario: External component initialization returns a typed record and empty snapshot
- **WHEN** a host initializes `SearchBox` from a `ProgramArtifact` containing `external component <SearchBox placeholder:string = "Find docs" /> = { state { query:string } }` without passing explicit props
- **THEN** initialization SHALL return a rendered `SearchBox` value with `placeholder="Find docs"`
- **AND** SHALL return an empty NX-managed component-state snapshot for that `SearchBox` instance

#### Scenario: External component dispatch uses bound handlers without local state
- **WHEN** a component instance created from `external component <SearchBox emits { SearchRequested } /> = { state { query:string } }` with bound handler `onSearchRequested=<DoSearch query={action.query} />` dispatches `<SearchRequested query="docs" />`
- **THEN** dispatch SHALL return an effect action list containing exactly one `DoSearch` action with `query="docs"`
- **AND** SHALL return a next component-state snapshot representing the same empty NX-managed external-component state

### Requirement: External component values preserve component identity across host serialization
The system SHALL preserve the component identity and normalized prop fields when a host serializes
the result of evaluating a concrete external component value to JSON or another wire format so that
a client can instantiate the corresponding UI component.

#### Scenario: JSON serialization preserves external component identity and props
- **WHEN** a host serializes the result of evaluating source containing `external component <SearchBox placeholder:string showSearchIcon:bool /> let render() = <SearchBox placeholder="Docs" showSearchIcon=true />` to JSON
- **THEN** the serialized payload SHALL preserve component identity `SearchBox`
- **AND** SHALL preserve normalized prop fields `placeholder="Docs"` and `showSearchIcon=true`

### Requirement: Derived external component values satisfy abstract external base named types

Static analysis SHALL accept a value or expression whose static type is a concrete external
component when it is used in a position that expects a named type that resolves to an abstract
external component contract, whenever the concrete external component’s effective contract inherits
from that abstract base through the declared `extends` chain.

#### Scenario: Single derived value binds to abstract base variable

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question placeholder:string? /> let question: Question = <ShortTextQuestion label={"Name"} placeholder={"Enter your name"} />`
- **THEN** type checking SHALL report no errors for the binding to `question`

#### Scenario: Interpreter returns derived value through function typed at base

- **WHEN** the same declarations exist and a function `render()` returns `{ question }` where
  `question` is typed as `Question` and initialized with `<ShortTextQuestion ... />`
- **THEN** interpreting `render()` SHALL succeed
- **AND** the returned component record SHALL retain the concrete runtime identity `ShortTextQuestion`

#### Scenario: Unrelated external component is rejected for abstract base binding

- **WHEN** a file contains `abstract external component <A label:string /> external component <B extends A /> external component <C label:string /> let x: A = <C label={"x"} />`
- **THEN** type checking SHALL report at least one error for the binding to `x`

### Requirement: Runtime external component record values match expected types using contract ancestry

When the interpreter validates that a value matches an expected named type and the value is an
external component record, the check SHALL succeed when the expected name resolves to an external
component contract and either the runtime component type name matches that contract’s component
name or that contract’s component name appears in the actual runtime component value’s effective
ancestor list, consistent with static named-type compatibility.

#### Scenario: Mixed derived values in a base-typed list evaluate successfully

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question /> external component <LongTextQuestion extends Question /> let questions: Question[] = { <ShortTextQuestion label={"Name"} /> <LongTextQuestion label={"Details"} /> } let render() = { questions }`
- **THEN** interpreting `render()` SHALL succeed
- **AND** the returned list SHALL contain two records whose runtime type names are `ShortTextQuestion`
  and `LongTextQuestion` respectively

#### Scenario: Interpreter rejects unrelated external component at parameter coercion

- **WHEN** a file contains `abstract external component <A label:string /> external component <C label:string /> let take(a: A): string = { "ok" } let render() = { take(<C label={"x"} />) }`
- **THEN** interpreting `render()` SHALL fail with a type mismatch attributable to parameter coercion
  for `take`

