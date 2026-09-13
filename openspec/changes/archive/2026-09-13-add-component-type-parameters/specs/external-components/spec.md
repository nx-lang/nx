## ADDED Requirements

### Requirement: Generated TypeScript external components carry type parameters generically and erase them on the element
Executable TypeScript generation for an external component with type parameters SHALL emit
`<ComponentName>Props` as a generic type with one parameter per component type parameter, each
defaulting to `unknown`, SHALL emit the `<ComponentName>` factory with the same generic parameters
so that a caller's argument is inferred from the props it passes, and SHALL emit
`<ComponentName>Element` with every occurrence of a type parameter erased to `unknown` and no
generic parameter, because the element is the serializable value and the wire carries no type
argument. Neither type SHALL have a member for the type parameter. `<ComponentName>State` SHALL
erase a type parameter the same way, with no generic parameter: state is a snapshot the host holds
as data, and its instantiation was fixed at an NX use site the host never sees. The generated
TypeScript for a program whose state or update record names a type parameter SHALL type check.

#### Scenario: Generic external component emits generic Props and factory
- **WHEN** NX source declares `external component <SkiaLayout TItem:type itemsSource:TItem[]? />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated output SHALL declare `SkiaLayoutProps<TItem = unknown>` with `itemsSource` typed as an optional, nullable array of `TItem`
- **AND** the generated `SkiaLayout` factory SHALL be generic over `TItem` with the same default
- **AND** calling `SkiaLayout({ itemsSource: [{ name: "a" }] })` SHALL type check with `TItem` inferred from the argument
- **AND** calling `SkiaLayout({})` SHALL return a value whose `$type` is `"SkiaLayout"`

#### Scenario: Generic external component erases the parameter on Element
- **WHEN** NX source declares `external component <SkiaLayout TItem:type itemsSource:TItem[]? />`
- **AND** a caller requests TypeScript executable output
- **THEN** generated `SkiaLayoutElement` SHALL type `itemsSource` as an array of `unknown` and SHALL declare no generic parameter
- **AND** neither `SkiaLayoutProps` nor `SkiaLayoutElement` SHALL have a `TItem` member

#### Scenario: Generic component erases the parameter on State
- **WHEN** NX source declares `component <List TItem:type items:TItem[]? /> = { state { sel:TItem? = null } <Label /> }`
- **AND** a caller requests TypeScript executable output
- **THEN** generated `ListState` SHALL type `sel` as nullable `unknown` and SHALL declare no generic parameter
- **AND** the generated module SHALL pass `tsc --strict`
