# visualizer-declarations Specification

## Purpose
Defines visualizer declarations: `visualizer Target = <…Visualizer …/>` module members, written beside a
type or in any module that can see it, that tell a viewer how to present a record, action, component or
alias. Covers which targets can have one, how the value is checked against `@nx/visualizer`, how
declarations merge along inheritance and across the program and the host's modules, and their diagnostics.
## Requirements
### Requirement: A `visualizer` declaration presents one declaration
The system SHALL accept, as a module member, a visualizer declaration `visualizer Target = Value`, where
`Target` is a type name, qualified or not, and `Value` is an expression. `visualizer` SHALL be a keyword
only as the first token of a module member, so a prop, field, parameter, binding or function named
`visualizer` SHALL keep working. A visualizer declaration SHALL take no visibility modifier, SHALL declare no
name, and SHALL NOT be referable from any expression or type. Analysis SHALL type-check its value
like any other module member. A visualizer declaration SHALL NOT be lowered to NX IR, SHALL NOT be
evaluated by any runtime or the interpreter, and SHALL NOT change the module's exports or
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

### Requirement: A visualizer declaration's target decides its visualizer record
The target of a visualizer declaration SHALL resolve, through the module's ordinary name resolution, to
a declaration the module can see, and the declaration's value SHALL have the visualizer record of the
target's kind as its type: `ElementVisualizer` for a record, an action (an inline emitted action
included) or a component, concrete, abstract or external; and `ValueVisualizer` for an alias of a
primitive type. Any other target SHALL be a diagnostic naming the target and saying it cannot have a
visualizer: a union, a function, a value, an alias of a non-primitive type, a generic record or
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

### Requirement: A visualizer declaration binds its visualizer records' `Target`
The system SHALL bind `Target` for every construction of an `@nx/visualizer` record in a visualizer
declaration's value that does not bind it, as if written `Target=X`. `X` SHALL be the target itself
when the target is a record, an action or an alias, and the target's derived props record
`T.Props` when the target is a component, so that a component's visualizer names its props
through `T.Props.Property`. A construction that binds `Target` explicitly SHALL bind it to `X`, and any other
binding SHALL be a diagnostic. This binding SHALL apply only inside a visualizer declaration's value;
everywhere else the ordinary rule that a generic record's construction binds every type parameter
SHALL hold.

#### Scenario: Children take the target too
- **WHEN** a module declares `type Choice = { value:string label:string }` and `visualizer Choice = <ElementVisualizer title=label><PropertyVisualizer property=value format=id /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer, checking `property=value` against `Choice.Property`

#### Scenario: A component's visualizer names its props
- **WHEN** a module declares `component <Question label:string help?:string /> = { <Panel /> }` and `visualizer Question = <ElementVisualizer title=label id=help />`
- **THEN** analysis SHALL accept the visualizer, checking `title` and `id` against `Question.Props.Property`

#### Scenario: A misspelled property is a type error
- **WHEN** a module declares `type Choice = { label:string }` and `visualizer Choice = <ElementVisualizer title=lable />`
- **THEN** analysis SHALL reject `lable` because it is not a case of `Choice.Property`

#### Scenario: A wrong explicit target
- **WHEN** a module declares `type Choice = { label:string }` and `type Other = { name:string }` and `visualizer Choice = <ElementVisualizer Target=Other title=name />`
- **THEN** analysis SHALL reject the binding `Target=Other` because the visualizer declaration's target is `Choice`

#### Scenario: Outside a visualizer declaration the rule is unchanged
- **WHEN** a module imports `@nx/visualizer` and contains `let h = <ElementVisualizer />`
- **THEN** analysis SHALL reject the element because `Target` was not specified

### Requirement: A visualizer declaration's value is constant
A visualizer declaration's value SHALL be built only from constructions of `@nx/visualizer` records, string
literals, boolean literals, and bare names that resolve contextually to a property or a case. A
braced expression, a reference to a value, a conditional, a loop, an interpolated string or a call
in a visualizer declaration's value SHALL be a diagnostic saying a visualizer's values must be written
out. The system SHALL therefore be able to read every visualizer declaration without evaluating it.

#### Scenario: A computed label is rejected
- **WHEN** a module declares `type Choice = { label:string }` and `let prefix = "Pick"` and `visualizer Choice = <ElementVisualizer icon={prefix} />`
- **THEN** analysis SHALL reject `{prefix}` with a diagnostic saying a visualizer's values must be written out

#### Scenario: Literals and bare names are accepted
- **WHEN** a module declares `component <Question label:string required:boolean = true /> = { <Panel /> }` and `visualizer Question = <ElementVisualizer icon="circle-help" title=label idPosition=end><PropertyVisualizer property=label format=id /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer

### Requirement: Visualizer declarations are gathered from the program and from host modules
The visualizer declarations that apply to a program SHALL be every visualizer declaration in the program's
modules: the entry module and every module it imports, directly or through other modules, the
standard libraries included. A visualizer declaration SHALL apply without being exported or imported,
and one in a module the program does not load SHALL NOT apply. A host MAY name further visualizer
modules, by module identity, when it asks for visualizers; each SHALL be analyzed against the program's
view of the declarations it imports, and its visualizer declarations SHALL apply after the program's. A
host visualizer module that does not analyze cleanly SHALL contribute the visualizer declarations that do,
and its diagnostics SHALL be reported for that module.

#### Scenario: A library's visualizers come with it
- **WHEN** a library module declares `export type Choice = { label:string }` with `visualizer Choice = <ElementVisualizer title=label />`, and a document imports `Choice` from it
- **THEN** the document's resolved visualizer for `Choice` SHALL have the title `label`

#### Scenario: A host module overrides a library
- **WHEN** the program's library declares `visualizer SingleChoice = <ElementVisualizer icon="list" />`, and the host names a visualizer module containing `visualizer SingleChoice = <ElementVisualizer icon="list-todo" />`
- **THEN** the resolved visualizer for `SingleChoice` SHALL have the icon `list-todo`

#### Scenario: An unloaded module's visualizer declarations do not apply
- **WHEN** a workspace holds a module with `visualizer Choice = <ElementVisualizer icon="option" />` that the document's program does not import and the host does not name
- **THEN** the document's resolved visualizer for `Choice` SHALL NOT have the icon `option`

### Requirement: Visualizer declarations merge along inheritance and across sources
The resolved visualizer of a declaration SHALL be built by applying visualizer declarations in order, each
later one overriding only what it sets. The order SHALL be: for each declaration from the furthest
base to the declaration itself, first the visualizer declaration in the module that declares that
declaration, then visualizer declarations in other modules of the program, then the host's visualizer modules
in the order the host names them. Overriding SHALL work attribute by attribute: an attribute a later
visualizer declaration sets SHALL replace the earlier value, and one it leaves unset SHALL keep it.
`PropertyVisualizer` children SHALL merge by their `property`, attribute by attribute. The resolved
order of `PropertyVisualizer`s SHALL be the later declaration's, in its order, followed by the
earlier declaration's that the later one does not name, in their order. Every attribute SHALL be
inherited.

#### Scenario: A derived kind inherits its base's visualizer
- **WHEN** an abstract component `Question` with props `id` and `label` has `visualizer Question = <ElementVisualizer title=label id=id icon="list-checks" />`, and `SingleChoice extends Question` has `visualizer SingleChoice = <ElementVisualizer icon="list" />`
- **THEN** the resolved visualizer for `SingleChoice` SHALL have the title `label`, the id `id` and the icon `list`

#### Scenario: A kind with no visualizer inherits its base's
- **WHEN** `Question` has `visualizer Question = <ElementVisualizer title=label icon="list-checks" />` and `MultiChoice extends Question` has no visualizer
- **THEN** the resolved visualizer for `MultiChoice` SHALL have the title `label` and the icon `list-checks`

#### Scenario: Property visualizers merge by property
- **WHEN** a library's visualizer for `Question` has `<PropertyVisualizer property=help format=id />`, and a host visualizer module declares `visualizer Question = <ElementVisualizer><PropertyVisualizer property=label format=id /></ElementVisualizer>`
- **THEN** the resolved property visualizers SHALL give both `help` and `label` the format `id`

#### Scenario: Order puts the later visualizer's properties first
- **WHEN** `Question`'s visualizer lists property visualizers for `help`, and `SingleChoice`'s visualizer lists `allowsOther`
- **THEN** the resolved property visualizers for `SingleChoice` SHALL be in the order `allowsOther`, `help`, then the rest in declaration order

### Requirement: Duplicate visualizer declarations at one level are an error
Two visualizer declarations for the same target in one module SHALL be a `duplicate-visualizer` error at
the second, naming the first. Two visualizer declarations for the same target in two modules of the
program, neither of which declares the target, SHALL be a `duplicate-visualizer` error naming both,
reported on each document whose program loads both: at the visualizer declaration when the document
holds one, and otherwise at the document's import through which the program loads the module
holding the other. Neither of the two SHALL apply. Visualizer declarations for one target in the
declaring module, in another program module and in host modules SHALL NOT conflict, since they apply
in order. Two host visualizer modules MAY declare a visualizer for the same target; the one named later
SHALL apply later.

#### Scenario: Twice in one module
- **WHEN** a module declares `type Choice = { label:string }` and two visualizer declarations `visualizer Choice = …`
- **THEN** analysis SHALL report `duplicate-visualizer` at the second visualizer declaration, naming the first

#### Scenario: Two program modules declare a visualizer for the same target
- **WHEN** a library declares `Choice`, and two other modules of the program each declare `visualizer Choice = <ElementVisualizer icon=… />`
- **THEN** analysis of a document that imports both modules SHALL report `duplicate-visualizer` naming both visualizer declarations
- **AND** the resolved visualizer for `Choice` SHALL take neither icon

### Requirement: A property's format comes from its visualizer or its type
The resolved format of a property SHALL be the `format` of its resolved `PropertyVisualizer` when that
sets one; otherwise, when the property's declared type names an alias with a `ValueVisualizer`, that
visualizer's `format`; otherwise none. The alias SHALL be read from the property's declared type as
written, through occurrence suffixes (`Color?`, `Color+`), even though aliases are transparent to
type checking.

#### Scenario: An alias formats every property that uses it
- **WHEN** a module declares `type Color = string` with `visualizer Color = <ValueVisualizer format=color />`, and `type Choice = { label:string tint?:Color }`
- **THEN** the resolved format of `Choice.tint` SHALL be `color`

#### Scenario: A property visualizer overrides its type's format
- **WHEN** `Color` has the format `color` and `Choice`'s visualizer has `<PropertyVisualizer property=tint format=id />`
- **THEN** the resolved format of `Choice.tint` SHALL be `id`

