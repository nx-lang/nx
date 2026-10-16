## MODIFIED Requirements

### Requirement: Every construct of the source has a role
Each node SHALL have exactly one of these roles, covering the source shown:

- `import`: an import statement.
- `declaration`: a `let`, `type`, `action`, `component`, `visualizer` or function declaration, with its
  name; a `visualizer` declaration's name is its target as written.
- `parameter`, `stateField`, `field`, `unionCase`, `emit`: one member of a declaration, with its
  name, its declared type and, when it has one, its default as a child.
- `typeReference`: a type written in the source, with the type spelled in NX.
- `element`: an element construction, with the declaration of its kind when it resolves; a union
  case's element, as `<Status.shipped … />`, refers to its union.
- `attribute`: a `name=value` attribute, or the run of element content bound to a content
  property, with the property's name.
- `text`: a run of element text, with its text type when it has one (`markdown` in
  `<Note:markdown>`); `embed`: an `@{…}` inside it.
- `literal`: a string, number or boolean, with its value; `empty`: `{}`.
- `case`: a union case, written bare or qualified, with the case name and the union's declaration.
- `reference`: a name that refers to a declaration, a parameter, a state field or a local binding;
  `member`: a member access, with the member name and the object as its child.
- `operator`: a prefix, binary or postfix operator, with the operator token as written (`&&`,
  `!=`, `??`, `..=`, the `?` of `x?`) and its operands as children; `call`: a call, with the callee
  and arguments as children; `sequence`: a braced list of values, with its items as children.
- `condition`: an `if`, with its test and branches, or its arms, as children; `match`: an `is`
  expression, with the scrutinee and its arms; `matchArm`: one arm of an `is` expression or of a
  condition list, with its patterns or test and its result; `loop`: a `for`, with its bindings, its
  iterable and its body; `binding`: a name a `for` binds, its item or its index.
- `comment` and `docComment`: a comment, with its text.
- `unparsed`: a region the parser could not read.

A node SHALL carry its resolved type spelled in NX whenever the type checker gives the construct
one, and the index of the declaration it refers to in the declaration table whenever it refers to
one.

#### Scenario: An element and its attributes
- **WHEN** a document holds `<SingleChoice id="role" layout=chips allowsOther={true}>` with
  `<Choice … />` children bound to the content property `choices`
- **THEN** the tree SHALL hold an `element` node referring to `SingleChoice`
- **AND** `attribute` nodes named `id`, `layout`, `allowsOther` and `choices`, the last marked as
  bound by content
- **AND** a `literal` node with the value `"role"`, a `case` node named `chips` referring to
  `ChoiceLayout`, and a `literal` node with the value `true` marked as braced

#### Scenario: An operator keeps its token
- **WHEN** a document holds `if step == 3 && role != "engineer" { … }`
- **THEN** the tree SHALL hold a `condition` node whose test is an `operator` node with the token
  `&&`, whose operands are `operator` nodes with the tokens `==` and `!=`

#### Scenario: A handler attribute
- **WHEN** a component binds `onTapped=<State.Update count={count + 1} />` inside its body
- **THEN** the `attribute` node for `onTapped` SHALL be marked as a handler
- **AND** the `element` node for the update SHALL be marked as an update of the enclosing
  component's state

#### Scenario: Comments are nodes
- **WHEN** a document holds a line comment above a `let`
- **THEN** the tree SHALL hold a `comment` node with the comment's text

#### Scenario: A document that does not parse
- **WHEN** a document holds a declaration followed by a line the parser cannot read
- **THEN** the tree SHALL hold the declaration's nodes and an `unparsed` node covering the unread
  region

#### Scenario: A visualizer declaration
- **WHEN** a document holds `visualizer Choice = <ElementVisualizer title=label />`
- **THEN** the tree SHALL hold a `declaration` node named `Choice` and marked as a visualizer declaration
- **AND** an `element` node referring to `ElementVisualizer` and an `attribute` node named `title` whose
  value is a `case` node named `label` referring to `Choice.Property`

## ADDED Requirements

### Requirement: The answer carries resolved visualizer declarations when asked
A `sourceTree` request SHALL accept an optional `visualizers` option, holding an optional list of host
visualizer module identities. When the option is absent the answer SHALL carry no visualizers and SHALL be
unchanged from an answer without this feature. When it is present, the answer SHALL resolve the
program's visualizer declarations and the named host visualizer modules, as the visualizer-declarations capability defines, and
SHALL attach to each declaration entry with a resolved visualizer a `visualizer` value:

- for a record, an action or a component: its `title`, `id` and `icon` when set, its `idPosition`
  when it has an `id`, and a `properties` list holding every property of the entry once, in the
  resolved order (the properties the visualizer lists, then the rest in declaration order), each with
  its name and its resolved `format` when set;
- for an alias: its `format`.

An entry for a record, action or component that has no visualizer but has a property with a resolved
format SHALL carry a `visualizer` value with the formats. When visualizers are asked for, the
declaration table SHALL also list each alias with a visualizer that types a property of a listed
declaration, so a reader can read the property's format without another query. A host visualizer
declaration module the workspace snapshot does not hold SHALL be reported in the answer as missing
and SHALL NOT fail the query. The `visualizer` values SHALL round-trip through JSON without loss.

#### Scenario: A question with visualizers
- **WHEN** a client asks with `visualizers` for the source tree of a document that constructs
  `<SingleChoice … />`, and the survey library declares a visualizer for `Question` with `title=label`
  and `id=id` and for `SingleChoice` with `icon="list"` and a property visualizer for `allowsOther`
- **THEN** the entry for `SingleChoice` SHALL carry a `visualizer` value with the title `label`, the id
  `id`, the id position `start` and the icon `list`
- **AND** its `properties` SHALL start with `allowsOther` and list every other property of
  `SingleChoice` once

#### Scenario: The alias a property uses comes along
- **WHEN** a client asks with `visualizers` and `Choice`'s `tint` property is typed by `Color`, whose
  visualizer has the format `color`, and no node of the document names `Color`
- **THEN** the declaration table SHALL hold `Color` with a `visualizer` value with the format `color`

#### Scenario: Without the option nothing changes
- **WHEN** a client asks for the source tree of the same document without `visualizers`
- **THEN** no declaration entry SHALL carry a `visualizer` value

#### Scenario: A host module overrides a kind
- **WHEN** a client asks with `visualizers` naming a host module that declares a visualizer for
  `SingleChoice` with `icon="list-todo"`
- **THEN** the entry for `SingleChoice` SHALL carry the icon `list-todo`

#### Scenario: A missing host module
- **WHEN** a client asks with `visualizers` naming a module identity the snapshot does not hold
- **THEN** the answer SHALL list that identity as missing and SHALL carry the program's own visualizers
