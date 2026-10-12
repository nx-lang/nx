## MODIFIED Requirements

### Requirement: Component state defaults are initialization-only
The system SHALL evaluate component state default expressions only during initialization. During
dispatch, the passed-in prior state snapshot SHALL be the current value of every state field, and
the only way a state field changes SHALL be an update record for that component returned by a
handler and applied by dispatch. Default expressions SHALL NOT be re-evaluated when state changes.

#### Scenario: Dispatch reuses stored state instead of reevaluating defaults
- **WHEN** a module contains `component <SearchBox placeholder:string /> = { state { query:string = placeholder } <TextInput value={query} placeholder={placeholder} /> }` and a later dispatch receives a prior state snapshot whose current `query` value differs from `placeholder`
- **THEN** dispatch SHALL use the stored `query` value from the prior state snapshot as the current component state
- **AND** SHALL NOT reevaluate `query:string = placeholder`

#### Scenario: An update record is the only way to change state
- **WHEN** a module contains `component <SearchBox placeholder:string emits { Cleared } /> = { state { query:string = placeholder } <TextInput value={query} onTextChanged=<State.Update query={action.text} /> onCleared={ <State.Update query="" /> <Cleared /> } /> }`
- **THEN** dispatching the `onTextChanged` handler with `text="docs"` SHALL leave the next snapshot's `query` as `"docs"`
- **AND** a field the update record does not name SHALL keep its prior value

### Requirement: `Update` is a reserved emitted action name
A component SHALL NOT declare an inline emitted action named `Update`, `Property` or `State`,
because `<Component>.State` is its derived state record, and `<Component>.Update` and
`<Component>.Property` are kept free since they read as a derived update record and property union,
which a component does not have. Lowering SHALL
reject such a declaration with a diagnostic that explains the reservation. Referencing a shared
action named `Update`, `Property` or `State` in `emits` SHALL be rejected for the same reason. A
component SHALL NOT declare a state field or prop named `State` either, since inside its body the
bare name `State` denotes the state record.

#### Scenario: Inline emit named Update is rejected
- **WHEN** a file contains `component <Form emits { Update { value:string } } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit because `Form.Update` is reserved for derived update records

#### Scenario: Inline emit named State is rejected
- **WHEN** a file contains `component <Form emits { State { value:string } } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit because `Form.State` is reserved for the component's state record

#### Scenario: Shared action named State cannot be emitted
- **WHEN** a file contains `action State = { value:string } component <Form emits { State } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit reference because the qualified name `Form.State` would collide with the derived state record

#### Scenario: A prop or state field named State is rejected
- **WHEN** a file contains `component <Form State:string /> = { <Panel /> }` or `component <Form /> = { state { State:string = "" } <Panel /> }`
- **THEN** lowering SHALL reject the field because the bare name `State` denotes the component's state record inside its body

#### Scenario: Shared action named Update cannot be emitted
- **WHEN** a file contains `action Update = { value:string } component <Form emits { Update } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit reference because the handler name `onUpdate` and the qualified name `Form.Update` are reserved for derived update records

#### Scenario: Inline emit named Property is rejected
- **WHEN** a file contains `component <Form emits { Property { name:string } } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit because `Form.Property` is reserved for derived property unions

#### Scenario: Shared action named Property cannot be emitted
- **WHEN** a file contains `action Property = { name:string } component <Form emits { Property } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit reference because the qualified name `Form.Property` would collide with the derived property union
