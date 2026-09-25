## MODIFIED Requirements

### Requirement: DrawnUI types map onto NX types by documented rules
The catalog SHALL map each DrawnUI property type onto an NX type by a fixed, documented set of
rules, so that an evaluated NX value carries enough information for a renderer to reconstruct the
value DrawnUI expects. Optionality SHALL be carried by the property name, as `optional-properties`
defines: an author-settable DrawnUI property `x: T` or `x?: T` SHALL become `x?: T`, and an array
property `x: T[]` or `x?: T[]` SHALL become `x?: T+`, since an empty TypeScript array is absence to
DrawnUI and `{}` is absence to NX. The catalog SHALL NOT spell `?` or `*` in a property's type slot.

#### Scenario: String-literal unions become NX unions
- **WHEN** a DrawnUI property's type is a union of string literals
- **THEN** the catalog SHALL declare an NX union type whose case names are spelled exactly as those
  string literals
- **AND** an author SHALL be able to write a case name unqualified at that property

#### Scenario: Structured values become NX record types
- **WHEN** a DrawnUI property's type is a structured value such as a thickness, corner radius,
  shadow, point, or gradient
- **THEN** the catalog SHALL declare an NX record type with the same field names
- **AND** the evaluated value SHALL identify which record type it is

#### Scenario: Colors are strings
- **WHEN** a DrawnUI property's type is a color
- **THEN** the catalog SHALL declare it as a string

#### Scenario: Grid track sizes are strings for now
- **WHEN** a DrawnUI property expresses a grid track size
- **THEN** the catalog SHALL declare it as a string
- **AND** the catalog SHALL record that a discriminated union is the intended eventual model

#### Scenario: Optional and array-typed properties carry the mark on the name
- **WHEN** DrawnUI exposes `MaxLines: number` and `Shadows: SkiaShadow[]` as author-settable
  properties
- **THEN** the catalog SHALL declare `MaxLines?: float64` and `Shadows?: SkiaShadow+`
- **AND** an author who leaves either unset, or writes `Shadows={}`, SHALL bind the empty value

#### Scenario: Container children are typed by the control hierarchy
- **WHEN** a DrawnUI control accepts child controls
- **THEN** the catalog SHALL declare a content property `content Children?: DrawnNode+`, one or more
  of the abstract control base, optional on the name
- **AND** an author SHALL be able to nest any catalog control inside it, and a lone child SHALL
  bind a one-item sequence

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
- **WHEN** the catalog is generated from the vendored DrawnUI sources
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
