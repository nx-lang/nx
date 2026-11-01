## MODIFIED Requirements

### Requirement: A `visualizer` declaration presents one declaration
The system SHALL accept, as a module member, a visualizer declaration `visualizer Target = Value`, where
`Target` is a type name, qualified or not, and `Value` is an expression. `visualizer` SHALL be a keyword
only as the first token of a module member, so a prop, field, parameter, binding or function named
`visualizer` SHALL keep working. A visualizer declaration SHALL take no visibility modifier, SHALL declare no
name, and SHALL NOT be referable from any expression or type. Analysis SHALL type-check its value
like any other module member. A visualizer declaration SHALL NOT be lowered to NX IR, SHALL NOT be
evaluated by any runtime, and SHALL NOT change the module's exports or
declaration schema.

#### Scenario: A visualizer beside its type
- **WHEN** a module imports `@nx/visualizer` and contains `type Choice = { value:string label:string }` followed by `visualizer Choice = <ElementVisualizer title=label id=value />`
- **THEN** analysis SHALL accept the module with no diagnostics

#### Scenario: `visualizer` stays an ordinary name elsewhere
- **WHEN** a file contains `type Settings = { visualizer:string }` and `let visualizer = "Ada"` and `component <Panel visualizer:string /> = { <Label text={visualizer} /> }`
- **THEN** analysis SHALL accept all three declarations

#### Scenario: A visualizer declaration takes no visibility
- **WHEN** a module contains `export visualizer Choice = <ElementVisualizer title=label />`
- **THEN** analysis SHALL reject the declaration with a diagnostic saying a visualizer declaration takes no visibility modifier

#### Scenario: Nothing reaches the IR
- **WHEN** a module with visualizer declarations is built to NX IR
- **THEN** the image SHALL be identical to the image of the same module with its visualizer declarations removed, apart from the module fingerprint, which hashes the module's source text
