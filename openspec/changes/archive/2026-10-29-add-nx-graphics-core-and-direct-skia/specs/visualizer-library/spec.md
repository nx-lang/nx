## MODIFIED Requirements

### Requirement: The visualizer library is the standard library `@nx/visualizer`
The system SHALL provide a standard library with root `@nx/visualizer` and one module,
`@nx/visualizer/visualizer.nx`, whose exports are exactly the types `ElementVisualizer`,
`IdPosition`, `PropertyVisualizer`, `ValueVisualizer`, `ValueFormat`, `ValueEditor` and
`ValueChoice`, `PreviewVisualizer` and `FunctionVisualizer`, together with
the companions NX derives for each. The library SHALL export no function and no component, SHALL be
`unstable`, and every exported type, field and case SHALL carry a `///` doc comment. The library
SHALL build with no diagnostics, including no doc-comment warnings.

#### Scenario: The library exports exactly its types
- **WHEN** the standard library `@nx/visualizer` is analyzed
- **THEN** its exported declarations, leaving out derived companions, SHALL be the nine types named above and nothing else
- **AND** analysis SHALL report no error, warning, info or hint

#### Scenario: A module imports the library by name
- **WHEN** a module contains `import "@nx/visualizer"` and a visualizer declaration using `ElementVisualizer`
- **THEN** analysis SHALL resolve `ElementVisualizer` to the library's export

### Requirement: `ElementVisualizer` and `PropertyVisualizer` describe an element
`ElementVisualizer` SHALL be a record with the type parameter `Target`, optional fields `title` and
`id`, each typed `Target.Property`, a field `idPosition:IdPosition` defaulting to `start`, an
optional field `icon:string` naming an icon from the Lucide set, an optional field
`preview:PreviewVisualizer`, and an optional content field
`properties` holding one or more `<PropertyVisualizer Target=Target/>`. `IdPosition` SHALL be the
constant union `start | end`. `PropertyVisualizer` SHALL be a record with the type parameter
`Target`, a required field `property:Target.Property`, an optional field `format:ValueFormat`, the
editor hints as optional fields `editor:ValueEditor`, `min:float64`, `max:float64` and
`step:float64`, a field `multiline:boolean` defaulting to `false`, an optional field
`placeholder:string`, and an optional content field `choices` holding one or more `ValueChoice`.

#### Scenario: An element visualizer with property visualizers
- **WHEN** a module declares `type Choice = { value:string label:string description?:string }` and `visualizer Choice = <ElementVisualizer title=label id=value idPosition=end icon="circle-dot"><PropertyVisualizer property=description format=id /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer

#### Scenario: A property visualizer names a property of its target
- **WHEN** a module declares `type Choice = { label:string }` and `visualizer Choice = <ElementVisualizer><PropertyVisualizer property=value /></ElementVisualizer>`
- **THEN** analysis SHALL reject `property=value` because `value` is not a case of `Choice.Property`

#### Scenario: A property with editor hints
- **WHEN** a module declares `type Card = { title:string padding:int = 8 body?:string }` and `visualizer Card = <ElementVisualizer><PropertyVisualizer property=padding min=0 max=48><ValueChoice value=8 label="md" /></PropertyVisualizer><PropertyVisualizer property=body multiline=true placeholder="What it says" /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer

## ADDED Requirements

### Requirement: Preview descriptors name host implementations
PreviewVisualizer SHALL be a record with a required renderer:string. In visualizer declarations,
renderer SHALL be a nonempty string literal. Descriptors SHALL be read without evaluating NX
functions, loading code, or making network requests. A host decides whether the renderer ID is supported.

#### Scenario: A type requests a preview
- **WHEN** a type visualizer sets preview=<PreviewVisualizer renderer="app.survey"/>
- **THEN** its resolved metadata SHALL contain preview.renderer="app.survey"
- **AND** computed or empty renderer IDs SHALL be diagnosed

### Requirement: Function visualizers mark element invocations
FunctionVisualizer SHALL be a record with an optional icon:string naming a Lucide icon. It SHALL
be the visualizer record for an element-style function. It SHALL NOT change evaluation or select
a renderer for that function's returned value.

#### Scenario: A graphics helper has an invocation icon
- **WHEN** visualizer Star = <FunctionVisualizer icon="star"/> targets an element function
- **THEN** its invocation SHALL receive the star icon without changing the returned Polygon
