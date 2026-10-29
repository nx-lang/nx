## MODIFIED Requirements

### Requirement: A visualizer declaration's target decides its visualizer record
The target of a visualizer declaration SHALL resolve, through the module's ordinary name resolution, to
a declaration the module can see, and the declaration's value SHALL have the visualizer record of the
target's kind as its type: `ElementVisualizer` for a record, an action (an inline emitted action
included) or a component, concrete, abstract or external; and `ValueVisualizer` for an alias of a
primitive type; and `FunctionVisualizer` for an element-style function. Any other target SHALL be a diagnostic naming the target and saying it cannot have a
visualizer: a union, a paren-style function, a value, an alias of a non-primitive type, a generic record or
component, and a derived declaration (`T.Update`, `T.Property`, `T.State` and its companions, `T.Props` and its
companions). A target that does not resolve SHALL be reported as an unknown name.

#### Scenario: Each kind takes its record
- **WHEN** a module imports `@nx/visualizer` and declares `type Choice = { label:string }`, `type Color = string` and `component <Card title:string /> = { <Panel /> }`, with `visualizer Choice = <ElementVisualizer title=label />`, `visualizer Color = <ValueVisualizer format=color />` and `visualizer Card = <ElementVisualizer title=title />`
- **THEN** analysis SHALL accept all three visualizer declarations

#### Scenario: The wrong record for the kind
- **WHEN** a module declares `type Color = string` and `visualizer Color = <ElementVisualizer />`
- **THEN** analysis SHALL reject the value because `ElementVisualizer` is not `ValueVisualizer`

#### Scenario: A target that cannot have a visualizer
- **WHEN** a module declares `let answer = 42` and `visualizer answer = <ValueVisualizer format=id />`, `type Range = { T:type start:T end:T }` and `visualizer Range = <ElementVisualizer />`, and `type Size = small | large` and `visualizer Size = <ElementVisualizer />`
- **THEN** analysis SHALL reject all three declarations, each with a diagnostic naming its target and saying it cannot have a visualizer

#### Scenario: A derived declaration is not a target
- **WHEN** a module declares `type User = { name:string }` and `visualizer User.Update = <ElementVisualizer />`
- **THEN** analysis SHALL reject the declaration saying `User.Update` cannot have a visualizer

#### Scenario: An emitted action is a target
- **WHEN** a module declares `component <Form emits { Submitted { email:string } } /> = { <Panel /> }` and `visualizer Form.Submitted = <ElementVisualizer title=email />`
- **THEN** analysis SHALL accept the visualizer

#### Scenario: Element functions have presentation-only visualizers
- **WHEN** an element function declares a FunctionVisualizer icon
- **THEN** analysis SHALL accept it and reject ElementVisualizer for that function
- **AND** a paren-style function SHALL remain an unsupported visualizer target
