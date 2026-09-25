## MODIFIED Requirements

### Requirement: Derived external component values satisfy abstract external base named types
Static analysis SHALL accept a value or expression whose static type is a concrete external
component when it is used in a position that expects a named type that resolves to an abstract
external component contract, whenever the concrete external component’s effective contract inherits
from that abstract base through the declared `extends` chain.

#### Scenario: Single derived value binds to abstract base variable

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question placeholder?:string /> let question: Question = <ShortTextQuestion label={"Name"} placeholder={"Enter your name"} />`
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
The interpreter SHALL accept an external component record value for an expected named type when the
expected name resolves to an external component contract and either the runtime component type name
matches that contract’s component name or that contract’s component name appears in the actual
runtime component value’s effective ancestor list, consistent with static named-type compatibility.

#### Scenario: Mixed derived values in a base-typed list evaluate successfully

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question /> external component <LongTextQuestion extends Question /> let questions: Question+ = { <ShortTextQuestion label={"Name"} /> <LongTextQuestion label={"Details"} /> } let render() = { questions }`
- **THEN** interpreting `render()` SHALL succeed
- **AND** the returned sequence SHALL contain two records whose runtime type names are `ShortTextQuestion`
  and `LongTextQuestion` respectively

#### Scenario: Interpreter rejects unrelated external component at parameter coercion

- **WHEN** a file contains `abstract external component <A label:string /> external component <C label:string /> let take(a: A): string = { "ok" } let render() = { take(<C label={"x"} />) }`
- **THEN** interpreting `render()` SHALL fail with a type mismatch attributable to parameter coercion
  for `take`

### Requirement: Generated TypeScript external components carry type parameters generically and erase them on the element
Executable TypeScript generation for an external component with type parameters SHALL emit
`<ComponentName>Props` as a generic type with one parameter per component type parameter, each
defaulting to `unknown`, SHALL emit the `<ComponentName>` factory with the same generic parameters
so that a caller's argument is inferred from the props it passes, and SHALL emit
`<ComponentName>Element` with every occurrence of a type parameter erased to `unknown` and no
generic parameter, because the element is the serializable value and the wire carries no type
argument. An optional prop (`name?:T`) SHALL be an optional property of `<ComponentName>Props`;
on `<ComponentName>Element` an optional prop SHALL likewise be an optional property whatever its
occurrence, since an empty optional field is an omitted key; an optional `+` prop, when present, is
an array. Neither type SHALL have a member for the type parameter. `<ComponentName>State` SHALL
erase a type parameter the same way, with no generic parameter: state is a snapshot the host holds
as data, and its instantiation was fixed at an NX use site the host never sees. The generated
TypeScript for a program whose state or update record names a type parameter SHALL type check.

#### Scenario: Generic external component emits generic Props and factory
- **WHEN** NX source declares `external component <SkiaLayout TItem:type itemsSource?:TItem+ />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL declare `SkiaLayoutProps<TItem = unknown>` with `itemsSource` typed as an optional array of `TItem`
- **AND** the generated `SkiaLayout` factory SHALL be generic over `TItem` with the same default
- **AND** calling `SkiaLayout({ itemsSource: [{ name: "a" }] })` SHALL type check with `TItem` inferred from the argument
- **AND** calling `SkiaLayout({})` SHALL return a value whose `$type` is `"SkiaLayout"`

#### Scenario: Generic external component erases the parameter on Element
- **WHEN** NX source declares `external component <SkiaLayout TItem:type itemsSource?:TItem+ />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated `SkiaLayoutElement` SHALL type `itemsSource` as an optional array of `unknown` and SHALL declare no generic parameter
- **AND** neither `SkiaLayoutProps` nor `SkiaLayoutElement` SHALL have a `TItem` member

#### Scenario: Generic component erases the parameter on State
- **WHEN** NX source declares `component <List TItem:type items?:TItem+ /> = { state { sel?:TItem } <Label /> }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated `ListState` SHALL type `sel` as an optional property of type `unknown` and SHALL declare no generic parameter
- **AND** the generated module SHALL pass `tsc --strict`
