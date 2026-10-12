## MODIFIED Requirements

### Requirement: `Update` is a reserved emitted action name
A component SHALL NOT declare an inline emitted action named `Update`, `Property`, `State` or
`Props`, because `<Component>.State` is its derived state record and `<Component>.Props` its derived
props record, and `<Component>.Update` and `<Component>.Property` are kept free since they read as a
derived update record and property union, which a component does not have. Lowering SHALL reject
such a declaration with a diagnostic that explains the reservation. Referencing a shared action
named `Update`, `Property`, `State` or `Props` in `emits` SHALL be rejected for the same reason. A
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

#### Scenario: Inline emit named Props is rejected
- **WHEN** a file contains `component <Form emits { Props { value:string } } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit because `Form.Props` is reserved for the component's props record

#### Scenario: Shared action named Props cannot be emitted
- **WHEN** a file contains `action Props = { value:string } component <Form emits { Props } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit reference because the qualified name `Form.Props` would collide with the derived props record
