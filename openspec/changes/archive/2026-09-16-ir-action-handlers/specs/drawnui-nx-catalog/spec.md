## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Catalog excludes members that cannot be authored
The catalog SHALL exclude DrawnUI members that are not author-settable inputs: engine-internal and
computed state, references to engine objects, and function-typed members that are not events,
such as factories and transforms that return a value.

#### Scenario: Callback properties are omitted
- **WHEN** a DrawnUI control declares a function-typed member that is not an event, one whose
  return type is not `void`, such as `ItemTemplate: () => SkiaControl` or
  `ProcessJson: (json: string) => string`
- **THEN** the catalog SHALL NOT declare that member as a property or an emit
- **AND** the omission SHALL be recorded alongside the other excluded members
- **AND** a function-typed member that is an event SHALL instead be declared as an emit, as
  "Catalog declares DrawnUI events as emits" requires

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
