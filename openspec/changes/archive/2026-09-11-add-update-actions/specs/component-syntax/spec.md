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
- **WHEN** a module contains `component <SearchBox placeholder:string emits { Cleared } /> = { state { query:string = placeholder } <TextInput value={query} onTextChanged=<Update query={action.text} /> onCleared=[<Update query="" />, <Cleared />] /> }`
- **THEN** dispatching the `onTextChanged` handler with `text="docs"` SHALL leave the next snapshot's `query` as `"docs"`
- **AND** a field the update record does not name SHALL keep its prior value

## ADDED Requirements

### Requirement: `Update` is a reserved emitted action name
A component SHALL NOT declare an inline emitted action named `Update`, because
`<Component>.Update` is the component's derived update record. Lowering SHALL reject such a
declaration with a diagnostic that explains the reservation. Referencing a shared action named
`Update` in `emits` SHALL be rejected for the same reason.

#### Scenario: Inline emit named Update is rejected
- **WHEN** a file contains `component <Form emits { Update { value:string } } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit because `Form.Update` is reserved for the component's update record

#### Scenario: Shared action named Update cannot be emitted
- **WHEN** a file contains `action Update = { value:string } component <Form emits { Update } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit reference because the handler name `onUpdate` and the qualified name `Form.Update` would collide with the derived update record
