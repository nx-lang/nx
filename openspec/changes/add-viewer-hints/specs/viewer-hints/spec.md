## ADDED Requirements

### Requirement: A `viewer` declaration gives a hint for one declaration
The system SHALL accept, as a module member, a viewer declaration `viewer Target = Value`, where
`Target` is a type name, qualified or not, and `Value` is an expression. `viewer` SHALL be a keyword
only as the first token of a module member, so a prop, field, parameter, binding or function named
`viewer` SHALL keep working. A viewer declaration SHALL take no visibility modifier, SHALL declare no
name, and SHALL NOT be referable from any expression or type. Analysis SHALL type-check its value
like any other module member. A viewer declaration SHALL NOT be lowered to NX IR, SHALL NOT be
evaluated by any runtime or the interpreter, and SHALL NOT change the module's exports or
declaration schema.

#### Scenario: A hint beside its declaration
- **WHEN** a module imports `@nx/viewer` and contains `type Choice = { value:string label:string }` followed by `viewer Choice = <ElementViewer title=label badge=value />`
- **THEN** analysis SHALL accept the module with no diagnostics

#### Scenario: `viewer` stays an ordinary name elsewhere
- **WHEN** a file contains `type Settings = { viewer:string }` and `let viewer = "Ada"` and `component <Panel viewer:string /> = { <Label text={viewer} /> }`
- **THEN** analysis SHALL accept all three declarations

#### Scenario: A hint takes no visibility
- **WHEN** a module contains `export viewer Choice = <ElementViewer title=label />`
- **THEN** analysis SHALL reject the declaration with a diagnostic saying a viewer declaration takes no visibility modifier

#### Scenario: Nothing reaches the IR
- **WHEN** a module with viewer declarations is built to NX IR
- **THEN** the image SHALL be identical to the image of the same module with its viewer declarations removed

### Requirement: A hint's target decides its hint record
The target of a viewer declaration SHALL resolve, through the module's ordinary name resolution, to
a declaration the module can see, and the declaration's value SHALL have the hint record of the
target's kind as its type: `ElementViewer` for a record, an action (an inline emitted action
included) or a component, concrete, abstract or external; `UnionViewer` for a union whose cases are
all constant cases; and `ValueViewer` for an alias of a primitive type. Any other target SHALL be a
diagnostic naming the target and saying it cannot have a viewer: a function, a value, a union with
a payload case, an alias of a non-primitive type, a generic record, component or union, and a
derived declaration (`T.Update`, `T.Property`, `T.State` and its companions, `T.Props` and its
companions). A target that does not resolve SHALL be reported as an unknown name.

#### Scenario: Each kind takes its record
- **WHEN** a module imports `@nx/viewer` and declares `type Choice = { label:string }`, `type ChoiceLayout = list | grid | chips`, `type ImageUrl = string` and `component <Card title:string /> = { <Panel /> }`, with `viewer Choice = <ElementViewer title=label />`, `viewer ChoiceLayout = <UnionViewer />`, `viewer ImageUrl = <ValueViewer format=image />` and `viewer Card = <ElementViewer title=title />`
- **THEN** analysis SHALL accept all four hints

#### Scenario: The wrong record for the kind
- **WHEN** a module declares `type ChoiceLayout = list | grid` and `viewer ChoiceLayout = <ElementViewer />`
- **THEN** analysis SHALL reject the value because `ElementViewer` is not `UnionViewer`

#### Scenario: A target that cannot have a viewer
- **WHEN** a module declares `let answer = 42` and `viewer answer = <ValueViewer format=text />`, and `type Range = { T:type start:T end:T }` and `viewer Range = <ElementViewer />`
- **THEN** analysis SHALL reject both declarations, each with a diagnostic naming its target and saying it cannot have a viewer

#### Scenario: A derived declaration is not a target
- **WHEN** a module declares `type User = { name:string }` and `viewer User.Update = <ElementViewer />`
- **THEN** analysis SHALL reject the declaration saying `User.Update` cannot have a viewer

#### Scenario: An emitted action is a target
- **WHEN** a module declares `component <Form emits { Submitted { email:string } } /> = { <Panel /> }` and `viewer Form.Submitted = <ElementViewer title=email />`
- **THEN** analysis SHALL accept the hint

### Requirement: A viewer declaration binds its hint records' `Target`
The system SHALL bind `Target` for every construction of an `@nx/viewer` record in a viewer
declaration's value that does not bind it, as if written `Target=X`. `X` SHALL be the target itself
when the target is a record, an action, a union or an alias, and the target's derived props record
`T.Props` when the target is a component, so that the hint names a component's props through
`T.Props.Property`. A construction that binds `Target` explicitly SHALL bind it to `X`, and any other
binding SHALL be a diagnostic. This binding SHALL apply only inside a viewer declaration's value;
everywhere else the ordinary rule that a generic record's construction binds every type parameter
SHALL hold.

#### Scenario: Children take the target too
- **WHEN** a module declares `type Choice = { value:string label:string }` and `viewer Choice = <ElementViewer title=label><PropertyViewer of=value label="Stored as" /></ElementViewer>`
- **THEN** analysis SHALL accept the hint, checking `of=value` against `Choice.Property`

#### Scenario: A component's hint names its props
- **WHEN** a module declares `component <Question label:string help?:string /> = { <Panel /> }` and `viewer Question = <ElementViewer title=label subtitle=help />`
- **THEN** analysis SHALL accept the hint, checking `title` and `subtitle` against `Question.Props.Property`

#### Scenario: A misspelled property is a type error
- **WHEN** a module declares `type Choice = { label:string }` and `viewer Choice = <ElementViewer title=lable />`
- **THEN** analysis SHALL reject `lable` because it is not a case of `Choice.Property`

#### Scenario: A wrong explicit target
- **WHEN** a module declares `type Choice = { label:string }` and `type Other = { name:string }` and `viewer Choice = <ElementViewer Target=Other title=name />`
- **THEN** analysis SHALL reject the binding `Target=Other` because the hint's target is `Choice`

#### Scenario: Outside a hint the rule is unchanged
- **WHEN** a module imports `@nx/viewer` and contains `let h = <ElementViewer />`
- **THEN** analysis SHALL reject the element because `Target` was not specified

### Requirement: A hint's value is constant
A viewer declaration's value SHALL be built only from constructions of `@nx/viewer` records, string
literals, boolean literals, and bare names that resolve contextually to a property or a case. A
braced expression, a reference to a value, a conditional, a loop, an interpolated string or a call
in a hint's value SHALL be a diagnostic saying a hint's values must be written out. The system
SHALL therefore be able to read every hint without evaluating it.

#### Scenario: A computed label is rejected
- **WHEN** a module declares `type Choice = { label:string }` and `let prefix = "Pick"` and `viewer Choice = <ElementViewer name={prefix} />`
- **THEN** analysis SHALL reject `{prefix}` with a diagnostic saying a hint's values must be written out

#### Scenario: Literals and bare names are accepted
- **WHEN** a module declares `component <Question label:string required:boolean = true /> = { <Panel /> }` and `viewer Question = <ElementViewer name="Question" title=label><PropertyViewer of=required whenTrue="Required" whenFalse="Optional" /></ElementViewer>`
- **THEN** analysis SHALL accept the hint

### Requirement: Hints are gathered from the program and from host modules
The hints that apply to a program SHALL be every viewer declaration in the program's modules: the
entry module and every module it imports, directly or through other modules, the standard libraries
included. A hint SHALL apply without being exported or imported, and a hint in a module the program
does not load SHALL NOT apply. A host MAY name further hint modules, by module identity, when it asks
for hints; each SHALL be analyzed against the program's view of the declarations it imports, and its
hints SHALL apply after the program's. A host hint module that does not analyze cleanly SHALL
contribute the hints that do, and its diagnostics SHALL be reported for that module.

#### Scenario: A library's hints come with it
- **WHEN** a library module declares `export type Choice = { label:string }` with `viewer Choice = <ElementViewer title=label />`, and a document imports `Choice` from it
- **THEN** the document's resolved hint for `Choice` SHALL have the title `label`

#### Scenario: A host module rewords a library
- **WHEN** the program's library hints `viewer SingleChoice = <ElementViewer name="Single choice" />`, and the host names a hint module containing `viewer SingleChoice = <ElementViewer name="Pick one" />`
- **THEN** the resolved hint for `SingleChoice` SHALL have the name `Pick one`

#### Scenario: An unloaded module's hints do not apply
- **WHEN** a workspace holds a module with `viewer Choice = <ElementViewer name="Option" />` that the document's program does not import and the host does not name
- **THEN** the document's resolved hint for `Choice` SHALL NOT have the name `Option`

### Requirement: Hints merge along inheritance and across sources
The resolved hint of a declaration SHALL be built by applying hints in order, each later hint
overriding only what it sets. The order SHALL be: for each declaration from the furthest base to the
declaration itself, first the hint in the module that declares that declaration, then hints in other
modules of the program, then the host's hint modules in the order the host names them. Overriding
SHALL work attribute by attribute: an attribute a later hint sets SHALL replace the earlier value,
and one it leaves unset SHALL keep it. `PropertyViewer` children SHALL merge by their `of` property and
`CaseViewer` children by their `of` case, attribute by attribute. The resolved order of
`PropertyViewer`s SHALL be the later hint's, in its order, followed by the earlier hint's that the
later one does not name, in their order. A base's `name` SHALL NOT be inherited, since it names the
base itself; every other attribute SHALL be.

#### Scenario: A derived kind inherits its base's hint
- **WHEN** an abstract component `Question` with props `label` and `help` has `viewer Question = <ElementViewer name="Question" title=label subtitle=help />`, and `SingleChoice extends Question` has `viewer SingleChoice = <ElementViewer name="Single choice" />`
- **THEN** the resolved hint for `SingleChoice` SHALL have the name `Single choice`, the title `label` and the subtitle `help`

#### Scenario: The base's name is not inherited
- **WHEN** `Question` has `viewer Question = <ElementViewer name="Question" title=label />` and `MultiChoice extends Question` has no hint
- **THEN** the resolved hint for `MultiChoice` SHALL have the title `label` and no name

#### Scenario: Property viewers merge by property
- **WHEN** a library hints `<PropertyViewer of=required whenTrue="Required" whenFalse="Optional" />` on `Question`, and a host hint module hints `viewer Question = <ElementViewer><PropertyViewer of=required whenTrue="Must answer" /></ElementViewer>`
- **THEN** the resolved property viewer for `required` SHALL have `whenTrue` `Must answer` and `whenFalse` `Optional`

#### Scenario: Order puts the later hint's properties first
- **WHEN** `Question`'s hint lists property viewers for `required` then `id`, and `SingleChoice`'s hint lists `layout` then `allowsOther`
- **THEN** the resolved property viewers for `SingleChoice` SHALL be in the order `layout`, `allowsOther`, `required`, `id`

#### Scenario: Case viewers merge by case
- **WHEN** a library hints `ChoiceLayout` with case viewers labelling `list` "a list" and `chips` "chips", and a host relabels `chips` "tags"
- **THEN** the resolved case labels SHALL be `list` "a list" and `chips` "tags"

### Requirement: Duplicate hints at one level are an error
Two viewer declarations for the same target in one module SHALL be a `duplicate-viewer-hint` error at
the second, naming the first. Two viewer declarations for the same target in two modules of the
program, neither of which declares the target, SHALL be a `duplicate-viewer-hint` error naming both,
reported on each document whose program loads both: at the hint when the document holds one, and
otherwise at the document's import through which the program loads the module holding the other.
Neither of the two SHALL apply. Hints for one target in the declaring module,
in another program module and in host modules SHALL NOT conflict, since they apply in order. Two
host hint modules MAY hint the same target; the one named later SHALL apply later.

#### Scenario: Twice in one module
- **WHEN** a module declares `type Choice = { label:string }` and two hints `viewer Choice = …`
- **THEN** analysis SHALL report `duplicate-viewer-hint` at the second hint, naming the first

#### Scenario: Two program modules hint the same target
- **WHEN** a library declares `Choice`, and two other modules of the program each declare `viewer Choice = <ElementViewer name=… />`
- **THEN** analysis of a document that imports both modules SHALL report `duplicate-viewer-hint` naming both hints
- **AND** the resolved hint for `Choice` SHALL take neither name

### Requirement: A property's format comes from its hint or its type
The resolved format of a property SHALL be the `format` of its resolved `PropertyViewer` when that
sets one; otherwise, when the property's declared type names an alias with a `ValueViewer`, that
viewer's `format`; otherwise none. The alias SHALL be read from the property's declared type as
written, through occurrence suffixes (`ImageUrl?`, `ImageUrl+`), even though aliases are transparent
to type checking. A property whose type is a union with a `UnionViewer` or a record with an
`ElementViewer` SHALL be read through that declaration's resolved hint.

#### Scenario: An alias formats every property that uses it
- **WHEN** a module declares `type ImageUrl = string` with `viewer ImageUrl = <ValueViewer format=image />`, and `type Choice = { label:string imageUrl?:ImageUrl }`
- **THEN** the resolved format of `Choice.imageUrl` SHALL be `image`

#### Scenario: A property viewer overrides its type's format
- **WHEN** `ImageUrl` has the format `image` and `Choice`'s hint has `<PropertyViewer of=imageUrl format=url />`
- **THEN** the resolved format of `Choice.imageUrl` SHALL be `url`
