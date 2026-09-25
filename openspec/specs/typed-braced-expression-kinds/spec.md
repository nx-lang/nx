# typed-braced-expression-kinds Specification

## Purpose
TBD - created by archiving change support-multi-value-braced-expressions. Update Purpose after archive.

## Requirements

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
When a typed binding site expects a `+` or `*` type, the system SHALL lift an exactly-one braced
result into a one-item sequence, by the lattice in `occurrence-types`: an exactly-one value
satisfies every occurrence, and the lift applies once. When a typed binding site is an optional
property declared `name?:T+`, the system SHALL check a written value against the property's read
type `T*`, so a non-empty braced sequence whose item type is compatible with `T` binds as its items
and the empty value binds as empty. When a typed binding site expects an exactly-one type, the
system MUST reject a braced result that carries an occurrence — a multi-item sequence, a `+` or `*`
value, or an optional — unless an explicit conversion exists. There is no distinction between an
optional sequence and a sequence of optionals: neither is expressible, and the system SHALL NOT
carry one.

#### Scenario: Scalar braced value is coerced at a list-typed binding site
- **WHEN** a `+`-typed parameter or field receives `{item}` and `item` has the expected item type
- **THEN** type checking and interpretation SHALL treat the argument as a one-item sequence

#### Scenario: Multi-item braced value binds to nullable list field
- **WHEN** a record declares `links?:ChatBrandLink+` and source constructs `<Brand links={ <ChatBrandLink /> <ChatBrandLink /> } />`
- **THEN** type checking SHALL accept the `links` binding as a `ChatBrandLink+` value at the
  optional field, whose read type is `ChatBrandLink*`
- **AND** interpretation SHALL preserve the supplied items rather than treating the field as empty
  or omitted

#### Scenario: Annotated nullable list let accepts braced list literal
- **WHEN** source contains `let links:ChatBrandLink* = { <ChatBrandLink /> <ChatBrandLink /> }`
- **THEN** type checking SHALL accept the binding because `ChatBrandLink+` satisfies
  `ChatBrandLink*`

#### Scenario: Nullable list widening still rejects incompatible element types
- **WHEN** a typed binding site is declared `links?:ChatBrandLink+` and receives `{ <ChatBrandLink /> <OtherLink /> }`
- **THEN** type checking SHALL reject the binding unless `OtherLink` is compatible with
  `ChatBrandLink`
- **AND** the diagnostic SHALL name the expected type as `ChatBrandLink*`

#### Scenario: List of nullable elements remains distinct from nullable list
- **WHEN** a file contains `type A = string?+`, `type B = string+?` and `type C = (string?)+`
- **THEN** the system SHALL reject each on its second suffix, as `occurrence-types` requires, so
  neither an optional sequence nor a sequence of optionals is a type a binding site can expect
- **AND** a site declared `names?:string+` SHALL be the one spelling of a sequence that may be
  absent, reading `string*`

#### Scenario: Multi-value brace is rejected at a scalar-typed binding site
- **WHEN** an exactly-one-typed parameter or field receives `{first second}`
- **THEN** the system SHALL report a semantic compatibility error because the braced result is a
  `+` sequence

#### Scenario: An optional is rejected at an exactly-one binding site
- **WHEN** a file contains `type Box = { name:string }`, `let o:string? = {}` and `let b = <Box name={o} />`
- **THEN** type checking SHALL reject the binding, naming `string?` and `string`

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
