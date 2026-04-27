## Why

The grammar already accepts conditional property-list forms, including match-style property
fragments, but lowering currently drops them because element properties are modeled as a flat list.
This creates a silent correctness gap for component and element call sites that should be able to
choose zero or more properties through ordinary control flow.

## What Changes

- Introduce first-class property-list fragment semantics for markup property lists.
- Support conditional property fragments (`if condition { ... } else { ... }`) as producers of
  zero or more element properties.
- Support match-style property fragments (`if value is { Pattern => ... else => ... }`) with the
  same union case validation, narrowing, and exhaustiveness rules as value match expressions.
- Define required-property analysis across conditional branches so a required prop is satisfied
  only when every reachable branch provides it.
- Define path-sensitive duplicate-property analysis: mutually exclusive branches may provide the
  same property, but duplicates on one possible path are rejected.
- Reject ambiguous static-plus-conditional duplicates instead of relying on source order or
  last-wins behavior.
- Replace the current silent drop behavior with explicit lowering, analysis, runtime, and
  documentation support.

## Capabilities

### New Capabilities
- `property-list-fragments`: Conditional and match-style property-list fragments for element,
  component, record, and union case invocations.

### Modified Capabilities
- `discriminated-unions`: Union match narrowing and exhaustiveness rules also apply inside
  property-list match fragments.
- `content-properties`: Conditional property fragments participate in the same content-property
  duplicate and required-binding rules as ordinary explicit properties.

## Impact

- Affects syntax/HIR lowering for element property lists, likely replacing or augmenting
  `Element.properties: Vec<Property>` with a property-fragment representation.
- Affects scope walking, type checking, component/record/union-case binding validation, interpreter
  element evaluation, runtime output, docs, examples, and VS Code grammar tests.
- Does not introduce new runtime dependencies or change the existing value-expression `Expr::Match`
  model.
