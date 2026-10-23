# source-tree Specification

## Purpose
Defines the source tree: a public, typed projection of one NX document that the language service
returns whole. Every piece of the source is a node with a role, a range, a parent and a stable key,
and the declarations the nodes refer to are listed once in a table beside them. The tree carries
facts about the source and nothing about presentation, so a viewer, a change view, an outline, a
lint rule or a structural editor can each build on it. It is distinct from the parser's syntax
tree, which stays internal and free to change.
## Requirements
### Requirement: The language service answers a source tree for a document
The language service SHALL answer a `sourceTree` query for a document of a workspace snapshot with
the document's URI, identity and version, a list of nodes and a list of declarations. The answer
SHALL be computed from the same analysis that answers hover and diagnostics for that snapshot, so
that a node's type and declaration agree with what hover reports at any position inside it. A
query for a document the snapshot does not hold SHALL fail the way the other document queries
fail. The answer SHALL round-trip through JSON without loss.

#### Scenario: The tree agrees with hover
- **WHEN** a client asks for the source tree of a document and for hover at a position inside an
  element of type `SingleChoice`
- **THEN** the element's node SHALL have the type `SingleChoice`
- **AND** its declaration entry SHALL name the declaration hover describes

#### Scenario: An unknown document
- **WHEN** a client asks for the source tree of a URI the snapshot does not hold
- **THEN** the query SHALL fail with the same error a hover query for that URI fails with

### Requirement: Nodes are a flat list in source order with parents first
The nodes SHALL be listed in order of their start offset, each parent before its children, and
each node other than a top-level one SHALL carry the index of its parent. Each node SHALL carry its
role and the range it covers, as an editor range: zero-based line and UTF-16 character positions
together with UTF-8 byte offsets, as every other language-service answer gives them. A node's
range SHALL contain the ranges of its children, and the ranges of two siblings SHALL NOT overlap.

#### Scenario: A parent precedes its children
- **WHEN** a document holds `let a = <Card title="Hi" />`
- **THEN** the node for the `let` SHALL precede the element's node, which SHALL precede the
  attribute's node, which SHALL precede the literal's node
- **AND** each of the last three SHALL carry the index of the one before it as its parent

#### Scenario: Ranges count UTF-16 units and bytes
- **WHEN** a literal follows an emoji on its line
- **THEN** the literal node's character offsets SHALL count the emoji as two units
- **AND** its byte offsets SHALL count it as four bytes

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

### Requirement: Every token belongs to exactly one node
Every token of a document SHALL lie in the range of at least one node, and SHALL belong to the
smallest node whose range contains it. The tokens that belong to a node and to none of its children
SHALL be only the punctuation and keywords of the node's role (angle brackets, `=`, braces,
parentheses, commas, quotes, the slash of a closing tag, and the keywords that introduce the
construct) and the tokens the node carries as its `name`, `value` or `textType`. A name, a literal,
an operator, a type or a comment SHALL always belong to a node that says what it is.

#### Scenario: Coverage over the repository
- **WHEN** the conformance test computes the source tree of every `.nx` file in the repository
- **THEN** every token of every file SHALL belong to a node under the rule above

#### Scenario: A forgotten construct fails the test
- **WHEN** the tree builder omits a node for a construct a file uses
- **THEN** the conformance test SHALL fail naming the file, the line and the token left uncovered

### Requirement: A node's key is stable under edits elsewhere
Each node SHALL carry a key that identifies it within the document by path rather than by offset:
the name of the enclosing top-level declaration, then the names of the attributes, members, arms
and slots (a declaration's `value`, an operator's `left`, a condition's `test`) on the way to the
node, and for an item of a sequence, of element content or of a branch, its position among the
items. A comment or an unparsed region, which names nothing, SHALL be keyed by the sibling it
precedes. Two nodes of one tree SHALL NOT share a key. A key is an identifier to compare for
equality, not a path to split: a segment written from the source, such as an arm's patterns, may
hold the separator, and the tree's structure is given by parent indices. Comparing the trees of
two versions of a document by key SHALL pair a node that an edit did not touch with itself,
whatever was inserted or removed in other declarations or in other attributes.

#### Scenario: An insertion elsewhere does not move a key
- **WHEN** a document declares `let a = …` and `let b = <Card title="Hi" />`, and an edit inserts a
  new declaration between them
- **THEN** the `title` attribute of `b` SHALL have the same key in both trees

#### Scenario: Items are keyed by position
- **WHEN** an element has three `<Choice … />` children bound to `choices`
- **THEN** their keys SHALL end in `choices[0]`, `choices[1]` and `choices[2]`

### Requirement: The declaration table describes what nodes refer to
The answer SHALL list once each declaration a node refers to, whether this document or another
module of the program declares it, the standard library included. Each entry SHALL carry the
declaration's module identity and name, its kind (`record`, `action`, `union`, `alias`, `component`,
`function`, `value`), its range when this document declares it, and its doc comment when it has
one. A record's, action's or component's entry SHALL list its properties in declaration order,
inherited ones first, each with its name, its type spelled in NX, whether it is optional, whether it
is the content property, the source text of its default when it has one, and its doc comment; and
SHALL list the entries of its bases, nearest first. A component with state SHALL also list its state
fields the same way. A union's entry SHALL list its cases with their doc comments. An alias's
entry SHALL give the type it names. A value declaration's entry SHALL give its type and the range
of its value, so a tool can show what a reference points at by reading the target's nodes.

#### Scenario: A library type is described
- **WHEN** a document imports a survey library and constructs `<SingleChoice … />`
- **THEN** the declaration table SHALL hold `SingleChoice` with the library module's identity
- **AND** its properties SHALL list `id`, `label`, `help`, `required` and `requiredMessage`, which
  it inherits from `Question`, before `allowsOther`, `layout` and `choices`
- **AND** `required` SHALL carry the default `true` and `choices` SHALL be marked as content

#### Scenario: A reference leads to its value
- **WHEN** a document declares `let teamSizeQuestion = <Integer … />` and another declaration of the
  same document refers to `teamSizeQuestion`
- **THEN** the `reference` node SHALL refer to the entry for `teamSizeQuestion`
- **AND** that entry's value range SHALL equal the range of the `<Integer … />` element node

### Requirement: The source tree is exposed to TypeScript hosts
The wasm language snapshot SHALL answer the source tree query, and `@nx-lang/language-core`'s
service SHALL expose it as `sourceTree`, beside `documentSymbols`, with the same answer the Rust
language service gives for the same snapshot. The packages that expose the query SHALL document it
as unstable until a later change commits to its shape.

#### Scenario: Parity between Rust and TypeScript
- **WHEN** the wasm SDK's language-service tests compute the source tree of each document in their
  corpus through the TypeScript service and through the Rust service
- **THEN** the two answers SHALL be equal after JSON round-trip

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
- for an element function (`let <Name … />`), which a visualizer cannot target: a `properties` list
  holding each of its properties once, in declaration order, each with its name and the `format` its
  type gives it when set;
- for an alias: its `format`.

An entry for a record, action, component or element function that has no visualizer but has a
property with a resolved format SHALL carry a `visualizer` value with the formats. When visualizers
are asked for, the declaration table SHALL also list each alias with a visualizer that types a
property of a listed declaration, so a reader can read the property's format without another query. A host visualizer
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

#### Scenario: An element function's property takes its type's format
- **WHEN** a client asks with `visualizers` for the source tree of a document that declares
  `let <Glow tint:Color size:int /> = …` and constructs `<Glow tint="#EC4899" size=3 />`, and `Color`'s
  visualizer has the format `color`
- **THEN** the entry for `Glow` SHALL carry a `visualizer` value whose `tint` property has the format
  `color` and whose `size` property has none

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

### Requirement: A resolved visualizer carries its editor hints
Each property in a declaration's `visualizer` `properties` list SHALL carry, when a `sourceTree`
request asks for `visualizers`, besides its name and format, an `editor` value holding its resolved
editor hints that fit its type, as the visualizer-declarations capability resolves them: `kind`
(`field`, `dropdown` or `segmented`), `min`, `max`, `step`, `multiline`, `placeholder` and `choices`, each only when set. `choices` SHALL list the choices that fit, in order, each with its
`value` typed as a `setValue` value is (`{ kind: "number", text }` or `{ kind: "string", value }`)
and its `label` when it has one. A hint or choice reported as `editor-hint-mismatch` SHALL be left
out, and a mismatched hint SHALL NOT hide the hint of the same name it would have replaced: its
base visualizer's, an earlier visualizer's or its type's. A resolved `min` above the resolved `max`,
reported as `editor-hint-range`, SHALL leave out both.
An alias's `visualizer` value SHALL carry its own hints the same way. A property with no hints SHALL
carry no `editor` value, and an entry whose only resolved values are hints SHALL carry a `visualizer`
value as one with a format does. The values SHALL round-trip through JSON without loss.

#### Scenario: A number's hints and choices
- **WHEN** a client asks with `visualizers` for the source tree of a document that constructs
  `<Card padding=8 />`, where `Card.padding` is typed `Spacing`, hinted `min=0 max=64`
  with the choices `<ValueChoice value=4 label="sm" />` and `<ValueChoice value=8 label="md" />`
- **THEN** the `properties` entry for `padding` in `Card`'s `visualizer` value SHALL carry an `editor`
  value with the minimum `0`, the maximum `64`, and the choices `4` labeled `sm` and
  `8` labeled `md`

#### Scenario: A mismatched hint is left out
- **WHEN** `Card.title` is a `string` hinted `editor=field placeholder="Title"`, with the choices
  `<ValueChoice value="Hi" />` and `<ValueChoice value=3 />`
- **THEN** the entry for `title` SHALL carry an `editor` value with the placeholder `Title`, the one
  choice `Hi`, and no kind

#### Scenario: An empty range is left out
- **WHEN** `Card.padding` is typed `Spacing`, hinted `min=10 max=0`
- **THEN** the entry for `padding` SHALL carry neither a minimum nor a maximum

### Requirement: The types of unwritten properties are listed
When a `sourceTree` request asks for `visualizers`, the declaration table SHALL also list the
declaration that types each property of every record, component, element function or union case the document's
nodes refer to, written or not, and the declaration each listed alias names, through its chain, so a
reader can edit a property an element does not write with its type's editor. A declaration listed only
this way SHALL NOT bring the types of its own properties.

#### Scenario: A union no element writes
- **WHEN** a client asks with `visualizers` for the source tree of a document that constructs
  `<Card title="Hi" />`, where `Card.size` is typed `Size`, a union `small | medium | large` that no
  node of the document refers to
- **THEN** the declaration table SHALL list `Size` with its three cases

