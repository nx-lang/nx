## MODIFIED Requirements

### Requirement: The visualizer library is the standard library `@nx/visualizer`
The system SHALL provide a standard library with root `@nx/visualizer` and one module,
`@nx/visualizer/visualizer.nx`, whose exports are exactly the types `ElementVisualizer`,
`IdPosition`, `PropertyVisualizer`, `ValueVisualizer`, `ValueFormat` and `ValueEditor`, together with
the companions NX derives for each. The library SHALL export no function and no component, SHALL be
`unstable`, and every exported type, field and case SHALL carry a `///` doc comment. The library
SHALL build with no diagnostics, including no doc-comment warnings.

#### Scenario: The library exports exactly its types
- **WHEN** the standard library `@nx/visualizer` is analyzed
- **THEN** its exported declarations, leaving out derived companions, SHALL be the six types named above and nothing else
- **AND** analysis SHALL report no error, warning, info or hint

#### Scenario: A module imports the library by name
- **WHEN** a module contains `import "@nx/visualizer"` and a visualizer declaration using `ElementVisualizer`
- **THEN** analysis SHALL resolve `ElementVisualizer` to the library's export

### Requirement: `ElementVisualizer` and `PropertyVisualizer` describe an element
`ElementVisualizer` SHALL be a record with the type parameter `Target`, optional fields `title` and
`id`, each typed `Target.Property`, a field `idPosition:IdPosition` defaulting to `start`, an
optional field `icon:string` naming an icon from the Lucide set, and an optional content field
`properties` holding one or more `<PropertyVisualizer Target=Target/>`. `IdPosition` SHALL be the
constant union `start | end`. `PropertyVisualizer` SHALL be a record with the type parameter
`Target`, a required field `property:Target.Property`, an optional field `format:ValueFormat`, the
editor hints `editor:ValueEditor?`, `min:float64?`, `max:float64?`, `step:float64?`, `stops:float64*`
and `unit:string?`, a field `multiline:boolean` defaulting to `false`, and an optional field
`placeholder:string`.

#### Scenario: An element visualizer with property visualizers
- **WHEN** a module declares `type Choice = { value:string label:string description?:string }` and `visualizer Choice = <ElementVisualizer title=label id=value idPosition=end icon="circle-dot"><PropertyVisualizer property=description format=id /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer

#### Scenario: A property visualizer names a property of its target
- **WHEN** a module declares `type Choice = { label:string }` and `visualizer Choice = <ElementVisualizer><PropertyVisualizer property=value /></ElementVisualizer>`
- **THEN** analysis SHALL reject `property=value` because `value` is not a case of `Choice.Property`

#### Scenario: A property with editor hints
- **WHEN** a module declares `type Card = { title:string padding:int = 8 body?:string }` and `visualizer Card = <ElementVisualizer><PropertyVisualizer property=padding editor=slider min=0 max=48 stops={ 0 4 8 16 24 48 } unit="px" /><PropertyVisualizer property=body multiline=true placeholder="What it says" /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer

### Requirement: `ValueVisualizer` formats a scalar type
`ValueVisualizer` SHALL be a record with the type parameter `Target`, an optional field
`format:ValueFormat`, and the editor hints `editor:ValueEditor?`, `min:float64?`, `max:float64?`,
`step:float64?`, `stops:float64*` and `unit:string?`. `ValueFormat` SHALL be the constant union
`id | color`.

#### Scenario: A color alias
- **WHEN** a module declares `type Color = string` and `visualizer Color = <ValueVisualizer format=color />`
- **THEN** analysis SHALL accept the visualizer

#### Scenario: A spacing alias with no format
- **WHEN** a module declares `type Spacing = int` and `visualizer Spacing = <ValueVisualizer editor=slider min=0 max=64 stops={ 0 4 8 16 24 32 48 64 } unit="px" />`
- **THEN** analysis SHALL accept the visualizer

## ADDED Requirements

### Requirement: Editor hints say how a value is edited
`ValueEditor` SHALL be the constant union `field | slider | dropdown | segmented`, each case with a doc
comment saying the editor it asks for: a number field, a slider with a number field, a list of a
union's cases, and a union's cases as one row of buttons. The doc comments SHALL say that `min`,
`max`, `step` and `stops` bound and step a number, that `stops` are the values a slider snaps to, that
`unit` is shown after a number and never written, that `multiline` lets a string's field hold line
breaks, and that `placeholder` is shown in an empty field and never written. A hint SHALL change how a
viewer edits a value, never what the value is.

#### Scenario: A hint is not a constraint
- **WHEN** a module's `Card` has `padding` hinted `max=48` and a document writes `<Card padding=64 />`
- **THEN** analysis SHALL report no diagnostic for the value
