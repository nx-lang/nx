## ADDED Requirements

### Requirement: Generated TypeScript external components expose element factories
Executable TypeScript generation SHALL emit a generated public surface for each concrete external
component that includes a `<ComponentName>Props` type for typed caller input, a
`<ComponentName>Element` type for the normalized serializable external element value, and an
unsuffixed `<ComponentName>` function that constructs that element value from typed props. The
generated external element type SHALL preserve the canonical external component identity and
normalized prop fields needed by a client renderer.

#### Scenario: External component emits Props, Element, and factory
- **WHEN** NX source declares `external component <TextInput value:string />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include exported `TextInputProps` and `TextInputElement` types
- **AND** generated output SHALL include an exported `TextInput` function
- **AND** calling `TextInput({ value: "docs" })` SHALL return a value whose `$type` is
  `"TextInput"` and whose `value` field is `"docs"`

#### Scenario: External component default is reflected in Props and Element
- **WHEN** NX source declares `external component <TextInput value:string = "" />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated `TextInputProps` SHALL allow `value` to be omitted
- **AND** generated `TextInputElement` SHALL include `value` as a required normalized string field
- **AND** calling `TextInput({})` SHALL return a `TextInputElement` whose `value` is `""`

#### Scenario: External component has no component entry lifecycle API
- **WHEN** NX source declares `external component <TextInput value:string />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL NOT expose `initialize` or `evaluate` as primary methods on the
  external component factory
- **AND** any JSON validation or conversion API for `TextInput` SHALL be exposed through
  `TextInputSchema` or an adapter reachable from `TextInputSchema`

### Requirement: Generated external component schemas validate external element JSON
Executable TypeScript generation SHALL emit a `<ComponentName>Schema` value for concrete external
components that need JSON boundary support. The schema SHALL validate JSON-compatible input for the
external component's props, apply defaults, reject unknown fields according to the component
contract, construct the typed `<ComponentName>Element` value through the generated factory or an
equivalent typed constructor, and preserve `$type` identity for client rendering.

#### Scenario: External component schema accepts valid JSON props
- **WHEN** NX source declares `external component <TextInput value:string />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL include `TextInputSchema`
- **AND** using `TextInputSchema` to normalize JSON props `{ value: "docs" }` SHALL produce a
  `TextInputElement` whose `$type` is `"TextInput"` and whose `value` is `"docs"`

#### Scenario: External component schema rejects unknown JSON props
- **WHEN** NX source declares `external component <TextInput value:string />`
- **AND** a caller normalizes JSON props `{ value: "docs", extra: true }` through
  `TextInputSchema`
- **THEN** the schema boundary operation SHALL report an unknown-field diagnostic for `extra`
- **AND** it SHALL NOT silently include `extra` in the resulting `TextInputElement`

#### Scenario: Generated TypeScript uses Element terminology for external values
- **WHEN** generated TypeScript output references the normalized value produced by a concrete
  external component named `TextInput`
- **THEN** the generated public type name for that value SHALL be `TextInputElement`
- **AND** generated code, tests, and public docs for the TypeScript executable surface SHALL use
  "external element" terminology for that value rather than presenting `Descriptor` as the public
  generated suffix
