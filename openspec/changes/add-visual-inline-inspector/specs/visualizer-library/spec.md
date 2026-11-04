## MODIFIED Requirements

### Requirement: `ElementVisualizer` and `PropertyVisualizer` describe an element
`ElementVisualizer` SHALL be a record with the type parameter `Target`, optional fields `title` and
`id`, each typed `Target.Property`, a field `idPosition:IdPosition` defaulting to `start`, an
optional field `icon:string` naming an icon from the Lucide set, an optional field
`preview:PreviewVisualizer`, and an optional content field
`properties` holding one or more `<PropertyVisualizer Target=Target/>`. `IdPosition` SHALL be the
constant union `start | end`. `PropertyVisualizer` SHALL be a record with the type parameter
`Target`, a required field `property:Target.Property`, an optional field `label:string` (the name a
viewer shows for the property), an optional field `format:ValueFormat`, the editor hints as optional fields `editor:ValueEditor`, `min:float64`, `max:float64` and
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

#### Scenario: Labels
- **WHEN** a module declares `type Box = { x:float64 y:float64 width:float64 fill?:string }` and `visualizer Box = <ElementVisualizer icon="square"><PropertyVisualizer property=x label="X" /><PropertyVisualizer property=width label="Width" /><PropertyVisualizer property=fill label="Fill" /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer

### Requirement: Function visualizers mark element invocations
FunctionVisualizer SHALL be a record with the type parameter `Target`, an optional icon:string naming
a Lucide icon and an optional content field
`properties` holding one or more `<PropertyVisualizer Target=Target/>`, as ElementVisualizer has. It
SHALL be the visualizer record for an element-style function, whose `Target` is the function's
derived `F.Props`. It SHALL NOT change evaluation or select a renderer for that function's returned
value.

#### Scenario: A graphics helper has an invocation icon
- **WHEN** visualizer Star = <FunctionVisualizer icon="star"/> targets an element function
- **THEN** its invocation SHALL receive the star icon without changing the returned Polygon

#### Scenario: A graphics helper labels its properties
- **WHEN** `let <Star cx:float64 cy:float64 points:int = 5 /> = …` has `visualizer Star = <FunctionVisualizer icon="star"><PropertyVisualizer property=points label="Points" icon="star" /></FunctionVisualizer>`
- **THEN** analysis SHALL accept the visualizer, checking `points` against `Star.Props.Property`
