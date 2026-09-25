## MODIFIED Requirements

### Requirement: Catalog is derived from DrawnUI sources rather than hand-maintained
The catalog SHALL be generated from the `drawnui-react` package version the site pins exactly, by
resolving each renderable tag's public property surface through the package's TypeScript
declarations, and SHALL be regenerable on demand so that moving the pin is followed by a catalog
refresh rather than manual editing. The generated metadata SHALL record the package version the
catalog was generated from.

#### Scenario: Accessor-defined properties are captured
- **WHEN** a DrawnUI control exposes a property through a getter and setter pair rather than a field
- **THEN** the generated catalog SHALL declare that property

#### Scenario: Regeneration is reproducible
- **WHEN** the catalog is generated twice from the same package version
- **THEN** both runs SHALL produce identical catalog output

#### Scenario: Generated catalog is committed
- **WHEN** a contributor checks out the repository without running the generator
- **THEN** the generated catalog SHALL already be present
- **AND** building and running the application SHALL NOT require regenerating it

#### Scenario: A stale catalog is detected
- **WHEN** the pinned `drawnui-react` version differs from the version the committed catalog records
- **THEN** the site's tests SHALL fail with a message naming both versions and the command that
  regenerates the catalog

### Requirement: Catalog divergence from DrawnUI is recorded
Where the catalog deliberately differs from DrawnUI — because a DrawnUI type has no NX equivalent,
or because an event's parameter was dropped — the divergence SHALL be recorded in the site's
documentation.

#### Scenario: Simplified property types are documented
- **WHEN** a DrawnUI property's type is narrowed or simplified in the catalog
- **THEN** the documentation SHALL name the property and state what was simplified

#### Scenario: Dropped event parameters are documented
- **WHEN** an event's parameter is dropped from its emit's payload
- **THEN** the documentation SHALL name the event and the parameter and state why

#### Scenario: Edits to vendored DrawnUI are documented
- **WHEN** NX would need DrawnUI to behave differently from the published package the site pins
- **THEN** the site SHALL NOT patch or vendor the package's code
- **AND** the documentation SHALL record the divergence and the upstream change it waits on

### Requirement: Catalog declares templated controls
For a DrawnUI control that declares both an items collection `ItemsSource: readonly unknown[]`
and a cell factory `ItemTemplate: () => SkiaControl`, the catalog SHALL declare a type parameter
`TItem`, the property `ItemsSource?: TItem+`, and the property
`ItemTemplate?: <function Item:TItem Index:int />: DrawnNode` — an optional function-typed
property, marked on the name and needing no parentheses — on the component for the class that
declares them, so that an author binds a collection and an element function and DrawnUI realizes
and recycles cells. The generated metadata SHALL record, for every renderable control that carries
`ItemTemplate`, the parameter names the template is called with, in order, so a renderer can bind
the item and its index. `ItemsSource` and `ItemTemplate` SHALL NOT appear in the recorded
omissions. The naming of the template's parameters — `Item` for what DrawnUI sets as the cell's
binding context and `Index` for its index in the collection — SHALL be recorded as a divergence.

#### Scenario: SkiaLayout is templated
- **WHEN** the catalog is generated from the pinned `drawnui-react` package
- **THEN** `SkiaLayout` SHALL declare `TItem:type`, `ItemsSource?:TItem+` and
  `ItemTemplate?:<function Item:TItem Index:int />: DrawnNode` on the class that declares them
- **AND** `<SkiaLayout TItem=Contact ItemsSource={contacts} ItemTemplate={ContactRow} RecyclingTemplate=Enabled />` SHALL compile when `ContactRow` is `let <ContactRow Item:Contact Index:int /> = ...` and `contacts` is a `Contact+`

#### Scenario: A template of the wrong item type is rejected
- **WHEN** an author binds `TItem=Contact` and an `ItemTemplate` whose `Item` parameter is another
  type
- **THEN** compilation SHALL fail with a diagnostic showing both function types

#### Scenario: An empty collection is absence
- **WHEN** an author binds `ItemsSource={}` or leaves `ItemsSource` unset
- **THEN** compilation SHALL succeed
- **AND** the control SHALL be created without an `ItemsSource`, so DrawnUI draws no cells

#### Scenario: Metadata names the template parameters
- **WHEN** the catalog metadata is generated
- **THEN** it SHALL record, for each renderable control with `ItemTemplate`, the parameter names
  `Item` and `Index` in that order
