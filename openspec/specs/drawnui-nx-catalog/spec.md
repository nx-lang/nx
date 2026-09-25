# drawnui-nx-catalog Specification

## Purpose
Defines the NX external component catalog that mirrors the DrawnUI control set, so NX authors can
describe a drawn user interface using the same control and property names DrawnUI itself uses.

## Requirements

### Requirement: Catalog declares the full DrawnUI control set
The catalog SHALL declare one NX external component for every control DrawnUI exposes as a
renderable tag, and SHALL declare an abstract external component for every shared base in DrawnUI's
control hierarchy so that inherited properties are declared once.

#### Scenario: Every renderable control is authorable
- **WHEN** an author writes an element whose name matches any DrawnUI renderable control
- **THEN** the catalog SHALL declare a matching external component
- **AND** the component name SHALL be spelled exactly as DrawnUI spells it

#### Scenario: Shared properties are declared on abstract bases
- **WHEN** two or more controls share a property because they share a DrawnUI base class
- **THEN** the catalog SHALL declare that property on an abstract external component
- **AND** the concrete components SHALL extend that abstract component rather than redeclaring the
  property

#### Scenario: Inherited properties are accepted at call sites
- **WHEN** an author sets a property that a control inherits from a base rather than declaring itself
- **THEN** compilation SHALL succeed
- **AND** the property SHALL appear in the evaluated output

### Requirement: Catalog is derived from DrawnUI sources rather than hand-maintained
The catalog SHALL be generated from the vendored DrawnUI TypeScript sources by resolving each
control's public property surface through the TypeScript type system, and SHALL be regenerable on
demand so that a DrawnUI sync is followed by a catalog refresh rather than manual editing.

#### Scenario: Accessor-defined properties are captured
- **WHEN** a DrawnUI control exposes a property through a getter and setter pair rather than a field
- **THEN** the generated catalog SHALL declare that property

#### Scenario: Regeneration is reproducible
- **WHEN** the catalog is generated twice from the same DrawnUI sources
- **THEN** both runs SHALL produce identical catalog output

#### Scenario: Generated catalog is committed
- **WHEN** a contributor checks out the repository without running the generator
- **THEN** the generated catalog SHALL already be present
- **AND** building and running the application SHALL NOT require regenerating it

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

### Requirement: Catalog declares DrawnUI events as emits
The catalog SHALL declare every DrawnUI event — an optional function-typed member whose return type
is `void` or includes it — as an emit of the component that stands for the class declaring it,
under DrawnUI's own name for the event, so that an author binds `on<Event>` on any control the
event reaches through inheritance. The emit's payload SHALL be derived from the callback's
parameters after the leading sender: a parameter whose type maps to an NX primitive SHALL become a
field under the parameter's name, and a parameter whose type has no NX mapping SHALL be dropped and
recorded. The generated metadata SHALL record, for every renderable control and every event it
carries, the callback's parameter names in order, so a renderer can build the action from the
callback's arguments.

#### Scenario: An event becomes an emit under its DrawnUI name
- **WHEN** DrawnUI declares `Tapped?: (sender: SkiaControl, e: ControlTappedEventArgs) => void` on
  `SkiaControl`
- **THEN** the catalog SHALL declare `Tapped` in the emits of the abstract `SkiaControl` component
- **AND** `<SkiaButton onTapped=... />` SHALL compile

#### Scenario: Primitive parameters become payload fields
- **WHEN** DrawnUI declares `Toggled?: (sender: SkiaToggle, value: boolean) => void`
- **THEN** the catalog SHALL declare the emit as `Toggled { value:boolean }`
- **AND** a handler bound as `onToggled` SHALL read `action.value` as a boolean

#### Scenario: Engine-typed parameters are dropped and recorded
- **WHEN** an event's parameter is typed as an engine object or a CanvasKit type, such as
  `ControlTappedEventArgs`, `SkiaGesturesParameters`, `SKPoint` or `Error`
- **THEN** the emit SHALL carry no field for it
- **AND** the omission SHALL be recorded alongside the other excluded members, naming the event,
  the parameter and its type

#### Scenario: Inherited events are declared once
- **WHEN** an event is declared on a DrawnUI base class
- **THEN** the catalog SHALL declare the emit on the abstract component for that class only
- **AND** every control extending it SHALL accept the handler property

#### Scenario: Metadata records the parameter order
- **WHEN** the catalog metadata is generated
- **THEN** it SHALL record, for each renderable control, every event it carries with the callback's
  parameter names in order, inherited events included

#### Scenario: A handler for an event the control does not have is rejected
- **WHEN** an author binds `onNope` on a catalog control
- **THEN** compilation SHALL fail with a diagnostic naming the unknown property

### Requirement: Catalog compiles to NX IR
The catalog SHALL be expressible in the subset of NX that NX IR code generation supports, so that
the fiddle's compile pipeline never fails because of the catalog itself.

#### Scenario: Union-valued property defaults survive IR generation
- **WHEN** a catalog property's default value is a union case
- **THEN** NX IR generation SHALL succeed

#### Scenario: Catalog alone is valid NX
- **WHEN** the catalog is compiled with no author source
- **THEN** compilation SHALL report no errors

### Requirement: Catalog divergence from DrawnUI is recorded
Where the catalog deliberately differs from DrawnUI — because a DrawnUI type has no NX equivalent,
because an event's parameter was dropped, or because the vendored DrawnUI source was edited to suit
NX — the divergence SHALL be recorded in the sample app's documentation.

#### Scenario: Simplified property types are documented
- **WHEN** a DrawnUI property's type is narrowed or simplified in the catalog
- **THEN** the documentation SHALL name the property and state what was simplified

#### Scenario: Dropped event parameters are documented
- **WHEN** an event's parameter is dropped from its emit's payload
- **THEN** the documentation SHALL name the event and the parameter and state why

#### Scenario: Edits to vendored DrawnUI are documented
- **WHEN** the vendored DrawnUI source is edited rather than copied verbatim
- **THEN** the documentation SHALL record what was changed and why

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
