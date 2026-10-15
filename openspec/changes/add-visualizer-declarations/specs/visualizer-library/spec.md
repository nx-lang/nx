## ADDED Requirements

### Requirement: The visualizer library is the standard library `@nx/visualizer`
The system SHALL provide a standard library with root `@nx/visualizer` and one module,
`@nx/visualizer/visualizer.nx`, whose exports are exactly the types `ElementVisualizer`, `IdPosition`,
`ElementLayout`, `PropertyVisualizer`, `Placement`, `UnionVisualizer`, `CaseVisualizer`, `ValueVisualizer` and
`ValueFormat`, together
with the companions NX derives for each. The library SHALL export no function and no component,
SHALL be `unstable`, and every exported type, field and case SHALL carry a `///` doc comment. The
library SHALL build with no diagnostics, including no doc-comment warnings.

#### Scenario: The library exports exactly its types
- **WHEN** the standard library `@nx/visualizer` is analyzed
- **THEN** its exported declarations, leaving out derived companions, SHALL be the nine types named above and nothing else
- **AND** analysis SHALL report no error, warning, info or hint

#### Scenario: A module imports the library by name
- **WHEN** a module contains `import "@nx/visualizer"` and a visualizer declaration using `ElementVisualizer`
- **THEN** analysis SHALL resolve `ElementVisualizer` to the library's export

### Requirement: `ElementVisualizer` and `PropertyVisualizer` describe an element
`ElementVisualizer` SHALL be a record with the type parameter `Target`, optional fields `name:string`,
`title`, `subtitle`, `badge`, `image` and `id`, each of the last five typed `Target.Property`, a field
`idPosition:IdPosition` defaulting to `beginning`, an optional field `icon:string` naming an icon from
the Lucide set, a field `layout:ElementLayout` defaulting to `card`, and an optional content field
`properties` holding one or more `<PropertyVisualizer Target=Target/>`. `IdPosition` SHALL be the constant
union `beginning | end`. `ElementLayout` SHALL be the constant union `card | row |
inline`. `PropertyVisualizer` SHALL be a record with the type parameter `Target`, a required field
`of:Target.Property`, optional fields `label:string`, `whenTrue:string`, `whenFalse:string` and
`format:ValueFormat`, and a field `place:Placement` defaulting to `body`. `Placement` SHALL be the
constant union `body | details`.

#### Scenario: An element visualizer with property visualizers
- **WHEN** a module declares `type Choice = { value:string label:string description?:string }` and `visualizer Choice = <ElementVisualizer layout=row title=label subtitle=description badge=value><PropertyVisualizer of=value place=details /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer

#### Scenario: A property visualizer names a property of its target
- **WHEN** a module declares `type Choice = { label:string }` and `visualizer Choice = <ElementVisualizer><PropertyVisualizer of=value /></ElementVisualizer>`
- **THEN** analysis SHALL reject `of=value` because `value` is not a case of `Choice.Property`

### Requirement: `UnionVisualizer` and `CaseVisualizer` describe a union's cases
`UnionVisualizer` SHALL be a record with the type parameter `Target` and an optional content field
`cases` holding one or more `<CaseVisualizer Target=Target/>`. `CaseVisualizer` SHALL be a record with the
type parameter `Target`, a required field `of:Target`, and optional fields `label:string` and
`example:string`.

#### Scenario: Case labels
- **WHEN** a module declares `type ChoiceLayout = list | grid | chips` and `visualizer ChoiceLayout = <UnionVisualizer><CaseVisualizer of=list label="a list" /><CaseVisualizer of=chips label="chips" example="◉ A  ○ B" /></UnionVisualizer>`
- **THEN** analysis SHALL accept the visualizer

#### Scenario: A case that is not in the union
- **WHEN** `ChoiceLayout` is `list | grid | chips` and its visualizer has `<CaseVisualizer of=table />`
- **THEN** analysis SHALL reject `of=table` because `table` is not a case of `ChoiceLayout`

### Requirement: `ValueVisualizer` formats a scalar type
`ValueVisualizer` SHALL be a record with the type parameter `Target` and a required field
`format:ValueFormat`. `ValueFormat` SHALL be the constant union `text | markdown | url | image | color
| email | phone | id`.

#### Scenario: An image URL alias
- **WHEN** a module declares `type ImageUrl = string` and `visualizer ImageUrl = <ValueVisualizer format=image />`
- **THEN** analysis SHALL accept the visualizer
