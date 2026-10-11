## ADDED Requirements

### Requirement: Program artifacts spell runtime values as annotated NX text
A program artifact built by the wasm SDK SHALL offer `formatValue(value, options)`, which SHALL take
a value in the canonical JSON output format, as the IR runtimes return it, and return an
`NxValueText` annotated against the artifact's program as `evaluateNx()` annotates `root`'s value.
The text SHALL be what `evaluateNx()` would return for the same value, from the same formatter. An
`ActionHandler` record, which the runtimes return for a handler, SHALL be spelled as that record,
with its `action` and `token` properties, rather than refused. Canonical JSON does not tell a whole
`float64` from an `int`, nor a constant union case from a string, so a number or a string SHALL be
read by the type its position declares, a record's property, in the entry module or a library the
program links, or the type `options.type` names for the whole value. A whole number SHALL be an
`int` when no position declares its type or the position's type is a type parameter, which the
value does not carry. `options.state` SHALL name a component of the entry module whose
state the value is, and the value SHALL then be spelled one `name=value` per line, in the order the
component declares its state fields, each read by its field's type and declared at that field. A
value that is not in the canonical format, or that has no NX spelling for another reason, SHALL fail
with an `NxEvaluationError` whose diagnostic says where in the value the problem is, and the
artifact SHALL remain usable afterwards. The program is not evaluated.

#### Scenario: A rendered component
- **WHEN** a host builds a program declaring `external component <Button label:string emits {
  Tapped { } } />` and calls `formatValue` with `{ "$type": "Row", "content": [{ "$type":
  "Button", "label": "Add", "onTapped": { "$type": "ActionHandler", "action": "Button.Tapped",
  "token": "h1-1" } }] }`
- **THEN** the text SHALL spell the `Row` element holding a `Button` element whose `onTapped`
  property is `<ActionHandler action="Button.Tapped" token="h1-1" />`
- **AND** the `Button` node's declaration SHALL be the span of its declaration

#### Scenario: The same text as root's
- **WHEN** a host evaluates a program's `root` through an IR runtime and passes the result to
  `formatValue`, and that value holds no action handler and no number outside a declared property
  or in a property typed by a type parameter
- **THEN** the text SHALL equal what `evaluateNx()` returns for the same program

#### Scenario: A whole float
- **WHEN** a program declares `type Size = { width:float64 }` and a host formats
  `{ "$type": "Size", "width": 3 }`
- **THEN** the text SHALL be `<Size width=3.0 />`

#### Scenario: A number with its type given
- **WHEN** a host formats `3` with `{ type: "float64" }`, and `3` with no options
- **THEN** the texts SHALL be `3.0` and `3`

#### Scenario: A component's state
- **WHEN** a program declares `component <Counter />` with `state { count:int = 0 ratio:float64 =
  1 }` and a host formats `{ "ratio": 1, "count": 4 }` with `{ state: "Counter" }`
- **THEN** the text SHALL be `count=4` and `ratio=1.0` on two lines, each a property node declared
  at its state field

#### Scenario: A value outside the canonical format
- **WHEN** a host calls `formatValue` with a record whose `$type` is not a string
- **THEN** it SHALL throw an `NxEvaluationError` naming the value's pointer
- **AND** the artifact SHALL answer later calls

### Requirement: Annotated text nodes carry their value's pointer
Every node of an `NxValueText` the wasm SDK returns, from `evaluateNx()` or `formatValue()`, SHALL
carry `pointer`: the RFC 6901 JSON pointer of its value within the value's canonical JSON form. A
property node SHALL carry the pointer of the property's value, the whole value's pointer SHALL be
the empty string, and an item of a sequence SHALL be addressed by its index.

#### Scenario: Pointers into a rendered output
- **WHEN** a host formats the `Row` of the previous requirement
- **THEN** the `Button` record's node SHALL carry `/content/0`, the `onTapped` property's node and
  the `ActionHandler` record's node SHALL both carry `/content/0/onTapped`, and the `Row` node SHALL
  carry the empty string

#### Scenario: The ABI version moves with the new export
- **WHEN** the wrapper for this version loads a module built before `formatValue` existed
- **THEN** loading SHALL fail with the version mismatch error, naming versions 8 and 7
