# visualizer-library Specification

## Purpose
Defines the `@nx/visualizer` standard library: the records a visualizer declaration is written with
(`ElementVisualizer`, `PropertyVisualizer`, `ValueVisualizer`) and the unions they use (`IdPosition`,
`ValueFormat`). The library is unstable and holds only what the Visual view reads.
## Requirements
### Requirement: The visualizer library is the standard library `@nx/visualizer`
The system SHALL provide a standard library with root `@nx/visualizer` and one module,
`@nx/visualizer/visualizer.nx`, whose exports are exactly the types `ElementVisualizer`,
`IdPosition`, `PropertyVisualizer`, `ValueVisualizer` and `ValueFormat`, together with the
companions NX derives for each. The library SHALL export no function and no component, SHALL be
`unstable`, and every exported type, field and case SHALL carry a `///` doc comment. The library
SHALL build with no diagnostics, including no doc-comment warnings.

#### Scenario: The library exports exactly its types
- **WHEN** the standard library `@nx/visualizer` is analyzed
- **THEN** its exported declarations, leaving out derived companions, SHALL be the five types named above and nothing else
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
`Target`, a required field `property:Target.Property`, and an optional field `format:ValueFormat`.

#### Scenario: An element visualizer with property visualizers
- **WHEN** a module declares `type Choice = { value:string label:string description?:string }` and `visualizer Choice = <ElementVisualizer title=label id=value idPosition=end icon="circle-dot"><PropertyVisualizer property=description format=id /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer

#### Scenario: A property visualizer names a property of its target
- **WHEN** a module declares `type Choice = { label:string }` and `visualizer Choice = <ElementVisualizer><PropertyVisualizer property=value /></ElementVisualizer>`
- **THEN** analysis SHALL reject `property=value` because `value` is not a case of `Choice.Property`

### Requirement: `ValueVisualizer` formats a scalar type
`ValueVisualizer` SHALL be a record with the type parameter `Target` and a required field
`format:ValueFormat`. `ValueFormat` SHALL be the constant union `id | color`.

#### Scenario: A color alias
- **WHEN** a module declares `type Color = string` and `visualizer Color = <ValueVisualizer format=color />`
- **THEN** analysis SHALL accept the visualizer

