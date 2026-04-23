## ADDED Requirements

### Requirement: Derived external component values satisfy abstract external base named types

Static analysis SHALL accept a value or expression whose static type is a concrete external
component when it is used in a position that expects a named type that resolves to an abstract
external component contract, whenever the concrete external component’s effective contract inherits
from that abstract base through the declared `extends` chain.

#### Scenario: Single derived value binds to abstract base variable

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question placeholder:string? /> let question: Question = <ShortTextQuestion label={"Name"} placeholder={"Enter your name"} />`
- **THEN** type checking SHALL report no errors for the binding to `question`

#### Scenario: Interpreter returns derived value through function typed at base

- **WHEN** the same declarations exist and a function `render()` returns `{ question }` where
  `question` is typed as `Question` and initialized with `<ShortTextQuestion ... />`
- **THEN** interpreting `render()` SHALL succeed
- **AND** the returned component record SHALL retain the concrete runtime identity `ShortTextQuestion`

#### Scenario: Unrelated external component is rejected for abstract base binding

- **WHEN** a file contains `abstract external component <A label:string /> external component <B extends A /> external component <C label:string /> let x: A = <C label={"x"} />`
- **THEN** type checking SHALL report at least one error for the binding to `x`

### Requirement: Runtime external component record values match expected types using contract ancestry

When the interpreter validates that a value matches an expected named type and the value is an
external component record, the check SHALL succeed when the expected name resolves to an external
component contract and either the runtime component type name matches that contract’s component
name or that contract’s component name appears in the actual runtime component value’s effective
ancestor list, consistent with static named-type compatibility.

#### Scenario: Mixed derived values in a base-typed list evaluate successfully

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question /> external component <LongTextQuestion extends Question /> let questions: Question[] = { <ShortTextQuestion label={"Name"} /> <LongTextQuestion label={"Details"} /> } let render() = { questions }`
- **THEN** interpreting `render()` SHALL succeed
- **AND** the returned list SHALL contain two records whose runtime type names are `ShortTextQuestion`
  and `LongTextQuestion` respectively

#### Scenario: Interpreter rejects unrelated external component at parameter coercion

- **WHEN** a file contains `abstract external component <A label:string /> external component <C label:string /> let take(a: A): string = { "ok" } let render() = { take(<C label={"x"} />) }`
- **THEN** interpreting `render()` SHALL fail with a type mismatch attributable to parameter coercion
  for `take`
