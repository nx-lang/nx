## ADDED Requirements

### Requirement: External components expose public UI contracts without NX implementation
The system SHALL treat an `external component` declaration as a component contract consisting only
of its effective props, prop defaults, content props, and emitted actions. External components
SHALL NOT require an NX body or local state in order to participate in invocation checking.

#### Scenario: Concrete external component is usable from NX call sites
- **WHEN** a file contains `external component <SearchBox placeholder:string = "Find docs" showSearchIcon:bool = true /> let render() = <SearchBox />`
- **THEN** analysis SHALL accept the invocation of `SearchBox`
- **AND** SHALL treat `placeholder` and `showSearchIcon` as the external component's public props

#### Scenario: Abstract external contract can be extended by another external component
- **WHEN** a file contains `abstract external component <SearchBase placeholder:string emits { SearchRequested } /> external component <SearchBox extends SearchBase showSearchIcon:bool = true />`
- **THEN** analysis SHALL accept `SearchBox extends SearchBase`
- **AND** SHALL treat `SearchBox` as inheriting `placeholder`, `SearchRequested`, and `showSearchIcon`

### Requirement: Evaluating an external component produces a typed component record
When NX evaluates an element targeting a concrete external component, the interpreter SHALL produce
a typed record-like component value whose type name is the component name and whose fields are the
normalized effective props, inherited or defaulted prop values, content bindings, and any bound
emitted-action handlers. The interpreter SHALL NOT evaluate an NX body for the external component.

#### Scenario: Function returns an external component record with normalized defaults
- **WHEN** a file contains `external component <SearchBox placeholder:string = "Find docs" showSearchIcon:bool = true /> let render() = <SearchBox />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value with `placeholder="Find docs"` and `showSearchIcon=true`

#### Scenario: Derived external component record includes inherited and local props
- **WHEN** a file contains `abstract external component <SearchBase placeholder:string = "Find docs" /> external component <SearchBox extends SearchBase showSearchIcon:bool = true /> let render() = <SearchBox />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value that includes inherited prop `placeholder="Find docs"` and local prop `showSearchIcon=true`

#### Scenario: Bound emitted-action handlers are preserved on external component values
- **WHEN** a file contains `action SearchRequested = { query:string } action DoSearch = { query:string } external component <SearchBox emits { SearchRequested } /> let render() = <SearchBox onSearchRequested=<DoSearch query={action.query} /> />`
- **THEN** interpreting `render()` SHALL return a `SearchBox` value that retains a bound `onSearchRequested` handler

### Requirement: External components are stateless in lifecycle bindings
The public NX component lifecycle bindings SHALL treat concrete external components as stateless
contract instances. Initialization SHALL return the typed external component record and an empty
component-state snapshot. Dispatch SHALL validate declared emitted actions and invoke any bound
handlers without requiring an NX body or local state.

#### Scenario: External component initialization returns a typed record and empty snapshot
- **WHEN** a host initializes `SearchBox` from a `ProgramArtifact` containing `external component <SearchBox placeholder:string = "Find docs" showSearchIcon:bool = true />` without passing explicit props
- **THEN** initialization SHALL return a rendered `SearchBox` value with `placeholder="Find docs"` and `showSearchIcon=true`
- **AND** SHALL return an empty component-state snapshot for that `SearchBox` instance

#### Scenario: External component dispatch uses bound handlers without local state
- **WHEN** a component instance created from `external component <SearchBox emits { SearchRequested } />` with bound handler `onSearchRequested=<DoSearch query={action.query} />` dispatches `<SearchRequested query="docs" />`
- **THEN** dispatch SHALL return an effect action list containing exactly one `DoSearch` action with `query="docs"`
- **AND** SHALL return a next component-state snapshot representing the same empty external-component state

### Requirement: External component values preserve component identity across host serialization
The system SHALL preserve the component identity and normalized prop fields when a host serializes
the result of evaluating a concrete external component value to JSON or another wire format so that
a client can instantiate the corresponding UI component.

#### Scenario: JSON serialization preserves external component identity and props
- **WHEN** a host serializes the result of evaluating source containing `external component <SearchBox placeholder:string showSearchIcon:bool /> let render() = <SearchBox placeholder="Docs" showSearchIcon=true />` to JSON
- **THEN** the serialized payload SHALL preserve component identity `SearchBox`
- **AND** SHALL preserve normalized prop fields `placeholder="Docs"` and `showSearchIcon=true`
