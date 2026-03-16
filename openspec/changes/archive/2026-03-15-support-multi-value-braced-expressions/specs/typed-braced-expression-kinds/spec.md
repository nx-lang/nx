## ADDED Requirements

### Requirement: Values and elements braced expressions remain distinct parser kinds
The parser SHALL expose general-value brace forms as `values_braced_expression` and element-only
brace forms as `elements_braced_expression`. Element `if` and `for` bodies, and other element-only
grammar positions, SHALL use `elements_braced_expression` rather than `values_braced_expression`.

#### Scenario: Value position uses values braced expression
- **WHEN** a file contains `let value = {first second}`
- **THEN** the parse tree SHALL contain a `values_braced_expression` for the right-hand side

#### Scenario: Elements if body uses elements braced expression
- **WHEN** a file contains `if ready { <A/> <B/> } else { <C/> }`
- **THEN** the parse tree SHALL contain `elements_braced_expression` nodes for the `then` and `else` bodies

### Requirement: Lowering preserves element-producing child expressions
Lowering SHALL preserve element-producing braced expressions and element control-flow forms as
element child expressions instead of dropping non-literal child content during HIR construction.

#### Scenario: Braced child element list survives lowering
- **WHEN** a file contains `<List>{ <Row/> <Row/> }</List>`
- **THEN** lowering SHALL preserve the braced child content as child expressions in source order

#### Scenario: Conditional child content survives lowering
- **WHEN** a file contains `<List>{if ready { <Row/> } else { <Empty/> }}</List>`
- **THEN** lowering SHALL preserve the conditional child expression instead of discarding it as non-literal content

### Requirement: Ordinary braced child expressions remain general values in element bodies
When markup child content contains an ordinary `{...}` braced expression, the parser and lowering
layers SHALL treat it as a general value expression rather than restricting it to element-only
results. Compatibility with the surrounding markup SHALL be checked during semantic analysis and
runtime validation.

#### Scenario: Scalar braced child expression remains a preserved child expression
- **WHEN** a file contains `<List>{count}</List>`
- **THEN** the parse tree SHALL use `values_braced_expression` for the child braces
- **AND** lowering SHALL preserve the child as an expression rather than rejecting it for not being a literal element

### Requirement: Typed binding sites coerce scalars to lists and reject list narrowing
When a typed binding site expects a list type, the system SHALL coerce a scalar braced result into
a one-item list. When a typed binding site expects a scalar type, the system MUST reject a
list-valued braced result unless an explicit conversion exists.

#### Scenario: Scalar braced value is coerced at a list-typed binding site
- **WHEN** a list-typed parameter or field receives `{item}` and `item` has the expected element type
- **THEN** type checking and interpretation SHALL treat the argument as a one-item list

#### Scenario: Multi-value brace is rejected at a scalar-typed binding site
- **WHEN** a scalar-typed parameter or field receives `{first second}`
- **THEN** the system SHALL report a semantic compatibility error because the braced result is list-valued

### Requirement: Tooling and grammar references use updated braced terminology
Parser-facing grammar references and generated node kinds SHALL use the updated braced-expression
terminology, and editor tooling SHALL continue to recognize the corresponding brace regions in
source files.

#### Scenario: Generated node kinds use values braced terminology
- **WHEN** generated parser artifacts are produced for `let value = {item}`
- **THEN** the node kind exposed for the brace expression SHALL be `values_braced_expression` and SHALL not be `interpolation_expression`

#### Scenario: Grammar references stay aligned
- **WHEN** the repository documents braced expression grammar
- **THEN** `nx-grammar.md` and `nx-grammar-spec.md` SHALL both describe `ValuesBracedExpression`, `ElementsBracedExpression`, and the updated text and embed brace forms

#### Scenario: VS Code continues to highlight braced value regions
- **WHEN** the VS Code grammar tokenizes `class="btn {first second}"`
- **THEN** the brace region SHALL remain highlighted as an interpolation or value-brace region and the inner identifiers SHALL remain inside the brace meta scope
