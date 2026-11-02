## ADDED Requirements

### Requirement: Conditional property spreads are feature gated
Schema 6 modules with a `[string-index("$spread"), node]` property entry SHALL list `conditional-properties-v1`.
The reserved spread entry SHALL spread a lazy intrinsic binding bag at its authored position. Ordinary entries
SHALL remain `[string-index, node]`; all property lists, including those in modules without spreads,
SHALL preserve authored order and SHALL be evaluated in that order. Empty bags
SHALL contribute no keys; empty field values SHALL retain key presence until target normalization.

#### Scenario: Nested fragments do not multiply constructions
- **WHEN** a module contains independent or nested conditional property fragments
- **THEN** each fragment SHALL produce one lazy branch selection and one spread
- **AND** emitted image size SHALL grow linearly with fragments and bindings

#### Scenario: An older runtime refuses the feature by name
- **WHEN** a module lists `conditional-properties-v1` and a runtime does not support it
- **THEN** preparation SHALL reject the module with an unsupported-feature diagnostic naming it

#### Scenario: A spread does not declare its feature
- **WHEN** a property list contains a spread without `conditional-properties-v1`
- **THEN** preparation SHALL reject the malformed module

## MODIFIED Requirements

### Requirement: NX IR encodes a call of a function-typed value by name
An element invocation whose tag is a function-typed binding SHALL be encoded as a named-call node:
the callee expression, then each argument as a name and a node, in authored property-list order.
Arguments SHALL be evaluated in that order and bound by name. A runtime SHALL
evaluate the callee to a function value and bind the arguments to that function's parameters by
name, dropping a name the function does not declare and failing with a diagnostic when a declared
parameter has no argument. A module containing a named-call node SHALL list the
`function-values-v1` feature. The existing positional call node SHALL be unchanged.

#### Scenario: Invoking a function-typed prop emits a named call
- **WHEN** NX source declares `component <Section Row:<function Item:object Index:int />: string /> = { <Row Item="a" Index=1 /> }`
- **AND** NX IR is emitted for the program
- **THEN** the body of `Section` SHALL contain a named-call node whose callee is the `Row` slot and
  whose arguments are `Item` and `Index`
- **AND** the explained text SHALL show the call with its argument names

#### Scenario: The conformance corpus covers function values
- **WHEN** the conformance corpus is inspected
- **THEN** it SHALL contain a program that declares a function type, binds a function to a
  function-typed prop and to a function-typed parameter, invokes the parameter by element and by
  call, and renders a function value in a function result
- **AND** the recorded results SHALL show the `Function` record for the rendered value and the
  results of both invocations
