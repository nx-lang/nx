## ADDED Requirements

### Requirement: Catalog declares templated controls
For a DrawnUI control that declares both an items collection `ItemsSource: readonly unknown[]`
and a cell factory `ItemTemplate: () => SkiaControl`, the catalog SHALL declare a type parameter
`TItem`, the property `ItemsSource: TItem[]?`, and the property
`ItemTemplate: (<function Item:TItem Index:int />: DrawnNode)?` on the component for the class
that declares them, so that an author binds a collection and an element function and DrawnUI
realizes and recycles cells. The generated metadata SHALL record, for every renderable control
that carries `ItemTemplate`, the parameter names the template is called with, in order, so a
renderer can bind the item and its index. `ItemsSource` and `ItemTemplate` SHALL NOT appear in
the recorded omissions. The naming of the template's parameters — `Item` for what DrawnUI sets
as the cell's binding context and `Index` for its index in the collection — SHALL be recorded as
a divergence.

#### Scenario: SkiaLayout is templated
- **WHEN** the catalog is generated from the vendored DrawnUI sources
- **THEN** `SkiaLayout` SHALL declare `TItem:type`, `ItemsSource:TItem[]?` and
  `ItemTemplate:(<function Item:TItem Index:int />: DrawnNode)?` on the class that declares them
- **AND** `<SkiaLayout TItem=Contact ItemsSource={contacts} ItemTemplate={ContactRow} RecyclingTemplate=Enabled />` SHALL compile when `ContactRow` is `let <ContactRow Item:Contact Index:int /> = ...`

#### Scenario: A template of the wrong item type is rejected
- **WHEN** an author binds `TItem=Contact` and an `ItemTemplate` whose `Item` parameter is another
  type
- **THEN** compilation SHALL fail with a diagnostic showing both function types

#### Scenario: Metadata names the template parameters
- **WHEN** the catalog metadata is generated
- **THEN** it SHALL record, for each renderable control with `ItemTemplate`, the parameter names
  `Item` and `Index` in that order

## MODIFIED Requirements

### Requirement: Catalog excludes members that cannot be authored
The catalog SHALL exclude DrawnUI members that are not author-settable inputs: engine-internal and
computed state, references to engine objects, and function-typed members that are neither events
nor the cell factory of a templated control, such as transforms that return a value.

#### Scenario: Callback properties are omitted
- **WHEN** a DrawnUI control declares a function-typed member that is not an event and not
  `ItemTemplate`, one whose return type is not `void`, such as
  `ProcessJson: (json: string) => string`
- **THEN** the catalog SHALL NOT declare that member as a property or an emit
- **AND** the omission SHALL be recorded alongside the other excluded members
- **AND** a function-typed member that is an event SHALL instead be declared as an emit, as
  "Catalog declares DrawnUI events as emits" requires
- **AND** `ItemTemplate` beside `ItemsSource` SHALL instead be declared as "Catalog declares
  templated controls" requires

#### Scenario: Engine state is omitted
- **WHEN** a DrawnUI member exists to carry measured, cached, or otherwise engine-computed state
- **THEN** the catalog SHALL NOT declare that member as a property

#### Scenario: Engine object references are omitted
- **WHEN** a DrawnUI property's type is an engine class — a control, an effect, the canvas, or any
  other class that is not one of DrawnUI's value types
- **THEN** the catalog SHALL NOT declare that property
- **AND** it SHALL NOT declare a record type for that class
- **AND** the omission SHALL be recorded alongside the other excluded members

#### Scenario: Setting an excluded property is reported
- **WHEN** an author sets a property the catalog excludes
- **THEN** compilation SHALL fail with a diagnostic naming the unknown property
