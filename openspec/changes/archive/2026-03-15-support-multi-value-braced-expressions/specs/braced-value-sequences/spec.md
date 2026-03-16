## ADDED Requirements

### Requirement: Values braced expressions support singleton and space-delimited list forms
The parser SHALL recognize `ValuesBracedExpression` as `{ ... }` containing either a single
`ValueExpression` or a space-delimited sequence of one or more `ValueListItemExpression` entries,
preserving item order. The same list semantics SHALL apply to `EmbedBracedExpression` inside
`@{ ... }`.

#### Scenario: Singleton braced value parses as a values braced expression
- **WHEN** a file contains `let value = {count}`
- **THEN** the parser SHALL produce a `values_braced_expression` node with one ordered item `count`

#### Scenario: Space-delimited brace list parses in source order
- **WHEN** a file contains `let value = {first second <Badge/>}`
- **THEN** the parser SHALL produce a `values_braced_expression` node with three ordered items in the source order `first`, `second`, `<Badge/>`

#### Scenario: Embedded text braces use the same list semantics
- **WHEN** a file contains `<p:html>Hello @{first second}</p>`
- **THEN** the parser SHALL produce an `embed_braced_expression` with two ordered items `first` and `second`

### Requirement: Non-list-safe expressions must be parenthesized inside braced lists
A braced value sequence SHALL accept bare list items only from the `ValueListItemExpression`
subset. Expressions outside that subset, including binary and prefix-unary expressions, MUST be
parenthesized before they can appear as list items.

#### Scenario: Parenthesized binary expression is accepted as a list item
- **WHEN** a file contains `let value = {(a + b) c}`
- **THEN** the parser SHALL accept the braced list and treat `(a + b)` and `c` as separate ordered items

#### Scenario: Bare binary expression in a list is rejected
- **WHEN** a file contains `let value = {a + b c}`
- **THEN** the parser SHALL report a parse error because `a + b` is not parenthesized as a list item

#### Scenario: Singleton binary expression remains legal
- **WHEN** a file contains `let value = {a - b}`
- **THEN** the parser SHALL accept the braced expression as a single `ValueExpression`

### Requirement: Values braced expressions infer scalar or list types from source arity
A `ValuesBracedExpression` with one source item SHALL infer to that item's type. A
`ValuesBracedExpression` with more than one source item SHALL infer to a list of the most specific
common item type. If no more specific common type exists, the inferred list type SHALL be
`object[]`.

#### Scenario: Singleton braced value keeps a scalar type
- **WHEN** type inference analyzes `let value = {1}`
- **THEN** `value` SHALL infer as `int` rather than `int[]`

#### Scenario: Multi-item braced value infers a list type
- **WHEN** type inference analyzes `let value = {1 2 3}`
- **THEN** `value` SHALL infer as `int[]`

#### Scenario: Heterogeneous element list falls back to object
- **WHEN** type inference analyzes `let value = {<A/> <B/>}`
- **THEN** `value` SHALL infer as `object[]`
