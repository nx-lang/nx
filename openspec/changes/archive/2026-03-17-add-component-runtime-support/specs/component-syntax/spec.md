## ADDED Requirements

### Requirement: Component initialization materializes props, state, and rendered output
The system SHALL lower a component declaration as an executable component definition. Initializing a
component SHALL bind prop values, apply any prop defaults, evaluate state default expressions once
in declaration order, and return the rendered body expression together with the logical initial
component state.

#### Scenario: Initialization applies prop and state defaults once
- **WHEN** a module contains `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder preview:string = query } <TextInput value={preview} placeholder={placeholder} /> }` and the component is initialized without an explicit `placeholder`
- **THEN** initialization SHALL bind `placeholder` as `"Find docs"`
- **AND** SHALL materialize initial state `query="Find docs"` and `preview="Find docs"`
- **AND** SHALL return a rendered `TextInput` element whose `value` and `placeholder` are both `"Find docs"`

#### Scenario: Initialization succeeds without a state group
- **WHEN** a module contains `component <Button text:string /> = { <button>{text}</button> }` and the component is initialized with `text="Save"`
- **THEN** initialization SHALL return a rendered `button` element containing `"Save"`
- **AND** SHALL produce an empty logical component state

#### Scenario: Missing required state initializer is rejected
- **WHEN** a module contains `component <SearchBox /> = { state { query:string } <TextInput value={query} /> }` and the component is initialized
- **THEN** initialization SHALL fail because non-nullable state field `query` has no initial value

### Requirement: Component state defaults are initialization-only
The system SHALL evaluate component state default expressions only during initialization. During
dispatch, the passed-in prior state snapshot SHALL be treated as the authoritative current value of
every state field until a later change introduces declarative state-update actions.

#### Scenario: Dispatch reuses stored state instead of reevaluating defaults
- **WHEN** a module contains `component <SearchBox placeholder:string /> = { state { query:string = placeholder } <TextInput value={query} placeholder={placeholder} /> }` and a later dispatch receives a prior state snapshot whose current `query` value differs from `placeholder`
- **THEN** dispatch SHALL use the stored `query` value from the prior state snapshot as the current component state
- **AND** SHALL NOT reevaluate `query:string = placeholder`
