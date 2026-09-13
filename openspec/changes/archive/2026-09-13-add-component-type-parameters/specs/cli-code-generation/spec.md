## ADDED Requirements

### Requirement: Generated contracts erase component type parameters in C# and carry them generically in TypeScript
The C# `typegen` emitter SHALL map a component type parameter to `object` wherever an exported
contract references it, SHALL NOT declare a generic parameter on the generated record, and SHALL
NOT include a member for the type parameter. The TypeScript `typegen` emitter SHALL declare the
exported contract type with one generic parameter per component type parameter, each defaulting
to `unknown`, SHALL reference the parameter by name inside the type, and SHALL NOT include a
member for it. List and nullable suffixes on a type-parameter reference SHALL be preserved around
the erased or generic type. The exported contract's parameter list SHALL be the component's
effective one, including parameters inherited from an abstract base declared in another module of
the library. A generic component's `<Name>_update` companion and, for an external component, its
`<Name>_state` record SHALL erase a state field's type parameter in both languages — to `object`
in C# and `unknown` in TypeScript — with no generic parameter, because no host names that
instantiation: an NX use site fixed it.

#### Scenario: C# external contract erases the parameter
- **WHEN** NX source declares `export external component <SkiaLayout TItem:type itemsSource:TItem[]? />`
- **AND** a caller requests C# output
- **THEN** the generated `SkiaLayout` record SHALL declare `itemsSource` as a nullable list of `object`
- **AND** SHALL NOT declare a generic type parameter or a `TItem` property

#### Scenario: TypeScript external contract is generic with an unknown default
- **WHEN** NX source declares `export external component <SkiaLayout TItem:type itemsSource:TItem[]? />`
- **AND** a caller requests TypeScript output
- **THEN** the generated type SHALL be declared as `SkiaLayout<TItem = unknown>` with `itemsSource` as a nullable array of `TItem`
- **AND** SHALL NOT declare a `TItem` property
- **AND** a caller writing `SkiaLayout` with no argument SHALL get `itemsSource` typed as a nullable array of `unknown`

#### Scenario: Derived contract carries a parameter inherited across modules
- **WHEN** a library's `base.nx` declares `export abstract external component <ItemsBase TItem:type items:TItem[]? />`
- **AND** its `derived.nx` declares `export external component <ContactList extends ItemsBase extra:TItem[]? />`
- **THEN** the generated TypeScript SHALL declare `ContactList<TItem = unknown>` extending `ItemsBaseBase<TItem>` with `extra` as a nullable array of `TItem`
- **AND** the generated C# `ContactList` SHALL declare `extra` as a nullable list of `object` with no `TItem` anywhere

#### Scenario: State and update companion erase the parameter
- **WHEN** NX source declares `export external component <Picker TItem:type items:TItem[]? /> = { state { sel:TItem? } }`
- **THEN** the generated TypeScript `Picker_state` SHALL type `sel` as nullable `unknown` and `Picker_update` SHALL type it as optional nullable `unknown`
- **AND** the generated C# `Picker_state` SHALL declare `Sel` as nullable `object` and `Picker_update` SHALL expose it as `NxOptional<object?>`
- **AND** neither language's output SHALL declare a `TItem` member on either type
