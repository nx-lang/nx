## ADDED Requirements

### Requirement: TypeScript runtime renders primitives in the canonical text form
The TypeScript IR runtime SHALL evaluate a `text` node by converting its operand to the canonical
text form `implicit-primitive-conversions` defines for the primitive type the node names. For
`int`, `int32`, `int64`, `float64` and `boolean` that form is what JavaScript's `String()` produces
for the carried value. For `float32` the runtime SHALL print the shortest digits that round-trip to
the same `float32`, not the expansion of the `float64` the value is carried in. A `text` node whose
operand is not a number or boolean at run time, or that names a type outside that set, SHALL fail
with an `nx-ir-operator` diagnostic naming the node.

The `concat` operator SHALL take two strings and join them, and SHALL fail with an
`nx-ir-operator` diagnostic when either operand is not a string, because schema `4` places the
conversion in a `text` node rather than in `concat`. The validator SHALL accept node kind `20` with
the layout `[20, node, str]` and SHALL reject it with any other layout.

Numeric comparison and arithmetic between values analysis widened need no runtime work, since the
runtime carries every numeric type as a JavaScript `number`; the runtime SHALL continue to compare
and compute them numerically.

#### Scenario: A text node over an int prints its digits
- **WHEN** a prepared program evaluates a `concat` whose right operand is a `text` node naming `int`
  over a slot holding `3`, with the left operand the string `"Total: "`
- **THEN** the result SHALL be `"Total: 3"`

#### Scenario: A text node over a float32 prints as a float32
- **WHEN** a prepared program evaluates a `text` node naming `float32` over a slot holding the
  `float64` widening of the nearest `float32` to `0.1`
- **THEN** the result SHALL be `"0.1"`

#### Scenario: An integral float prints without a fraction
- **WHEN** a prepared program evaluates a `text` node naming `float64` over a slot holding `1`
- **THEN** the result SHALL be `"1"`

#### Scenario: A boolean prints as true or false
- **WHEN** a prepared program evaluates a `text` node naming `boolean` over a slot holding `true`
- **THEN** the result SHALL be `"true"`

#### Scenario: concat over a non-string operand is refused
- **WHEN** a malformed image supplies a `concat` whose operand evaluates to a number
- **THEN** evaluation SHALL fail with an `nx-ir-operator` diagnostic

#### Scenario: Mixed numeric comparison compares numerically
- **WHEN** a prepared program evaluates `n < x` emitted from `n:int` holding `2` and `x:float64`
  holding `2.5`
- **THEN** the result SHALL be `true`
