## MODIFIED Requirements

### Requirement: Every construct of the source has a role
Each node SHALL have exactly one of these roles, covering the source shown:

- `import`: an import statement.
- `declaration`: a `let`, `type`, `action`, `component` or function declaration, with its name.
- `parameter`, `stateField`, `field`, `unionCase`, `emit`: one member of a declaration, with its
  name, its declared type and, when it has one, its default as a child.
- `typeReference`: a type written in the source, with the type spelled in NX.
- `element`: an element construction, with the declaration of its kind when it resolves.
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
