## MODIFIED Requirements

### Requirement: Evaluating an external component produces a typed component record
When NX evaluates an element targeting a concrete external component, the Rust and TypeScript IR runtimes SHALL produce
a typed record-like component value whose type name is the component name and whose fields are the
normalized effective props, inherited or defaulted prop values, content bindings, and any bound
emitted-action handlers. Declared external state SHALL remain host-managed metadata and SHALL NOT
introduce an NX render body or NX-evaluated state fields on that component value.

#### Scenario: Function returns an external component record with normalized defaults
- **WHEN** a file contains `external component <SearchBox placeholder:string = "Find docs" showSearchIcon:boolean = true /> let render() = <SearchBox />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value with `placeholder="Find docs"` and `showSearchIcon=true`

#### Scenario: Derived external component record includes inherited and local props
- **WHEN** a file contains `abstract external component <SearchBase placeholder:string = "Find docs" /> external component <SearchBox extends SearchBase showSearchIcon:boolean = true /> let render() = <SearchBox />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value that includes inherited prop `placeholder="Find docs"` and local prop `showSearchIcon=true`

#### Scenario: Bound emitted-action handlers are preserved on external component values
- **WHEN** a file contains `action SearchRequested = { query:string } action DoSearch = { query:string } external component <SearchBox emits { SearchRequested } /> let render() = <SearchBox onSearchRequested=<DoSearch query={action.query} /> />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value that retains a bound `onSearchRequested` handler

#### Scenario: Declared external state does not add NX-evaluated record fields
- **WHEN** a file contains `external component <SearchBox placeholder:string /> = { state { query:string } } let render() = <SearchBox placeholder="Docs" />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value with `placeholder="Docs"`
- **AND** SHALL NOT require an NX render body or an NX-evaluated `query` field on that component value

### Requirement: Derived external component values satisfy abstract external base named types
Static analysis SHALL accept a value or expression whose static type is a concrete external
component when it is used in a position that expects a named type that resolves to an abstract
external component contract, whenever the concrete external component’s effective contract inherits
from that abstract base through the declared `extends` chain.

#### Scenario: Single derived value binds to abstract base variable

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question placeholder?:string /> let question: Question = <ShortTextQuestion label={"Name"} placeholder={"Enter your name"} />`
- **THEN** type checking SHALL report no errors for the binding to `question`

#### Scenario: A runtime returns a derived value through function typed at base

- **WHEN** the same declarations exist and a function `render()` returns `{ question }` where
  `question` is typed as `Question` and initialized with `<ShortTextQuestion ... />`
- **THEN** interpreting `render()` SHALL succeed
- **AND** the returned component record SHALL retain the concrete runtime identity `ShortTextQuestion`

#### Scenario: Unrelated external component is rejected for abstract base binding

- **WHEN** a file contains `abstract external component <A label:string /> external component <B extends A /> external component <C label:string /> let x: A = <C label={"x"} />`
- **THEN** type checking SHALL report at least one error for the binding to `x`

### Requirement: Runtime external component record values match expected types using contract ancestry
The Rust and TypeScript IR runtimes SHALL accept an external component record value for an expected named type when the
expected name resolves to an external component contract and either the runtime component type name
matches that contract’s component name or that contract’s component name appears in the actual
runtime component value’s effective ancestor list, consistent with static named-type compatibility.

#### Scenario: Mixed derived values in a base-typed list evaluate successfully

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question /> external component <LongTextQuestion extends Question /> let questions: Question+ = { <ShortTextQuestion label={"Name"} /> <LongTextQuestion label={"Details"} /> } let render() = { questions }`
- **THEN** interpreting `render()` SHALL succeed
- **AND** the returned sequence SHALL contain two records whose runtime type names are `ShortTextQuestion`
  and `LongTextQuestion` respectively

#### Scenario: A runtime rejects an unrelated external component at parameter coercion

- **WHEN** a file contains `abstract external component <A label:string /> external component <C label:string /> let take(a: A): string = { "ok" } let render() = { take(<C label={"x"} />) }`
- **THEN** interpreting `render()` SHALL fail with a type mismatch attributable to parameter coercion
  for `take`
