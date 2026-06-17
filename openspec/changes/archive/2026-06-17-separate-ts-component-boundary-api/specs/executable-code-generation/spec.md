## ADDED Requirements

### Requirement: TypeScript components expose typed core APIs
Executable TypeScript generation SHALL emit concrete normal NX components as strongly typed
component functions. The primary generated TypeScript API for a normal component SHALL accept a
generated `<ComponentName>Props` type, SHALL evaluate the component body through generated
TypeScript code, and SHALL NOT require callers to pass `NxValue` for props or component state when
the corresponding NX type can be represented as a generated TypeScript type. Generated JSON
initialization, evaluation, validation, and diagnostics SHALL be exposed through `Schema`-suffixed
boundary values or adapters rather than through the primary component function.

#### Scenario: Stateless component emits typed function
- **WHEN** NX source declares `component <Title label:string /> = { label }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include an exported `TitleProps` type with readonly field
  `label: string`
- **AND** generated output SHALL include an exported `Title` function that accepts `TitleProps`
- **AND** calling `Title({ label: "Docs" })` SHALL return `"Docs"`
- **AND** the public `TitleProps` parameter type SHALL NOT be `NxValue`

#### Scenario: Defaulted prop is optional in typed props
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { placeholder }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include an exported `SearchBoxProps` type where `placeholder` is
  optional
- **AND** calling `SearchBox({})` or `SearchBox()` SHALL evaluate the component body with
  `placeholder` equal to `"Find docs"`

#### Scenario: Stateful component keeps typed state separate from JSON
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** `TextInput` is an external component
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include typed props and state surfaces for `SearchBox`
- **AND** generated typed state APIs SHALL use a generated `SearchBoxState` type rather than
  `NxValue`
- **AND** JSON state parsing and diagnostics SHALL be exposed through `SearchBoxSchema` or an
  adapter reachable from `SearchBoxSchema`

### Requirement: TypeScript schemas provide JSON boundary support
Executable TypeScript generation SHALL emit `Schema`-suffixed runtime values for generated
declarations that require JSON validation, JSON normalization, serialization, or diagnostic
conversion. Schema values SHALL contain or reference runtime metadata for the declaration shape and
SHALL use shared runtime helpers for primitive, array, record, enum, union, component props,
component state, and external element validation where those shapes can be represented as metadata.

#### Scenario: Component schema validates JSON props
- **WHEN** NX source declares `component <SearchBox placeholder:string /> = { placeholder }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include `SearchBoxSchema`
- **AND** `SearchBoxSchema` SHALL expose a JSON boundary operation that accepts untyped or
  JSON-compatible input for props
- **AND** supplying JSON props that omit `placeholder` through that boundary operation SHALL report
  a missing-field diagnostic
- **AND** calling the typed `SearchBox` function SHALL remain a strongly typed TypeScript call

#### Scenario: Schema metadata is shared for repeated field validation
- **WHEN** generated TypeScript output includes multiple declarations with `string` fields
- **THEN** generated schema values SHALL reference a shared string schema helper or metadata value
  for those fields
- **AND** generated output SHALL NOT emit independent custom string-validation control flow for
  each field unless it is emitted behind the same schema API as an implementation optimization

#### Scenario: Expression defaults remain generated typed code
- **WHEN** NX source declares a state field default that depends on a prop, such as
  `state { query:string = placeholder }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated schema metadata SHALL describe the state field's structural type
- **AND** generated typed code SHALL compute the prop-dependent default when materializing initial
  state
- **AND** JSON state validation SHALL reuse the schema metadata for supplied state values

### Requirement: Component expression generation follows component kind
Executable TypeScript and JavaScript generation SHALL distinguish normal components from external
components when emitting component element expressions. Element expressions targeting normal
components SHALL evaluate the target component body through the generated typed render path.
Element expressions targeting external components SHALL construct an external element value and
SHALL NOT attempt to evaluate a render body for that external component.

#### Scenario: Normal child component is rendered by parent
- **WHEN** NX source declares `external component <TextInput value:string />`
- **AND** declares `component <Child value:string /> = { <TextInput value={value} /> }`
- **AND** declares `component <Parent /> = { <Child value="docs" /> }`
- **AND** a caller evaluates generated output for `Parent`
- **THEN** generated output SHALL evaluate `Child`'s component body
- **AND** the rendered result SHALL be the `TextInput` external element with `value` equal to
  `"docs"`
- **AND** the rendered result SHALL NOT be a serializable element whose `$type` is `"Child"`

#### Scenario: External child component becomes element value
- **WHEN** NX source declares `external component <TextInput value:string />`
- **AND** declares `component <SearchBox /> = { <TextInput value="docs" /> }`
- **AND** a caller evaluates generated output for `SearchBox`
- **THEN** generated output SHALL return a `TextInput` external element value
- **AND** the element value SHALL preserve `$type` equal to `"TextInput"` and `value` equal to
  `"docs"`

#### Scenario: Parent renders two external children of same type
- **WHEN** NX source declares `external component <TextInput id:string label:string value:string = "" />`
- **AND** declares `component <QuestionFlow /> = { { <TextInput id="firstName" label="First name" /> <TextInput id="lastName" label="Last name" /> } }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL type the rendered result as a readonly collection of
  `TextInputElement`
- **AND** evaluating `QuestionFlow()` SHALL return two `TextInputElement` values with distinct
  `id` and `label` fields
- **AND** both values SHALL include normalized default `value` equal to `""`
