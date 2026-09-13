## MODIFIED Requirements

### Requirement: `Update` is a reserved emitted action name
A component SHALL NOT declare an inline emitted action named `Update` or `Property`, because
`<Component>.Update` is the component's derived update record and `<Component>.Property` is its
derived property union. Lowering SHALL reject such a declaration with a diagnostic that explains
the reservation. Referencing a shared action named `Update` or `Property` in `emits` SHALL be
rejected for the same reason.

#### Scenario: Inline emit named Update is rejected
- **WHEN** a file contains `component <Form emits { Update { value:string } } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit because `Form.Update` is reserved for the component's update record

#### Scenario: Shared action named Update cannot be emitted
- **WHEN** a file contains `action Update = { value:string } component <Form emits { Update } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit reference because the handler name `onUpdate` and the qualified name `Form.Update` would collide with the derived update record

#### Scenario: Inline emit named Property is rejected
- **WHEN** a file contains `component <Form emits { Property { name:string } } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit because `Form.Property` is reserved for the component's property union

#### Scenario: Shared action named Property cannot be emitted
- **WHEN** a file contains `action Property = { name:string } component <Form emits { Property } /> = { <Panel /> }`
- **THEN** lowering SHALL reject the emit reference because the qualified name `Form.Property` would collide with the derived property union
