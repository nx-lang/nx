## ADDED Requirements

### Requirement: The viewer library is the standard library `@nx/viewer`
The system SHALL provide a standard library with root `@nx/viewer` and one module,
`@nx/viewer/viewer.nx`, whose exports are exactly the types `ElementViewer`, `IdPosition`,
`ElementLayout`, `PropertyViewer`, `Placement`, `UnionViewer`, `CaseViewer`, `ValueViewer` and
`ValueFormat`, together
with the companions NX derives for each. The library SHALL export no function and no component,
SHALL be `unstable`, and every exported type, field and case SHALL carry a `///` doc comment. The
library SHALL build with no diagnostics, including no doc-comment warnings.

#### Scenario: The library exports exactly its types
- **WHEN** the standard library `@nx/viewer` is analyzed
- **THEN** its exported declarations, leaving out derived companions, SHALL be the nine types named above and nothing else
- **AND** analysis SHALL report no error, warning, info or hint

#### Scenario: A module imports the library by name
- **WHEN** a module contains `import "@nx/viewer"` and a viewer declaration using `ElementViewer`
- **THEN** analysis SHALL resolve `ElementViewer` to the library's export

### Requirement: `ElementViewer` and `PropertyViewer` describe an element
`ElementViewer` SHALL be a record with the type parameter `Target`, optional fields `name:string`,
`title`, `subtitle`, `badge`, `image` and `id`, each of the last five typed `Target.Property`, a field
`idPosition:IdPosition` defaulting to `beginning`, an optional field `icon:string` naming an icon from
the Lucide set, a field `layout:ElementLayout` defaulting to `card`, and an optional content field
`properties` holding one or more `<PropertyViewer Target=Target/>`. `IdPosition` SHALL be the constant
union `beginning | end`. `ElementLayout` SHALL be the constant union `card | row |
inline`. `PropertyViewer` SHALL be a record with the type parameter `Target`, a required field
`of:Target.Property`, optional fields `label:string`, `whenTrue:string`, `whenFalse:string` and
`format:ValueFormat`, and a field `place:Placement` defaulting to `body`. `Placement` SHALL be the
constant union `body | details`.

#### Scenario: An element viewer with property viewers
- **WHEN** a module declares `type Choice = { value:string label:string description?:string }` and `viewer Choice = <ElementViewer layout=row title=label subtitle=description badge=value><PropertyViewer of=value place=details /></ElementViewer>`
- **THEN** analysis SHALL accept the viewer

#### Scenario: A property viewer names a property of its target
- **WHEN** a module declares `type Choice = { label:string }` and `viewer Choice = <ElementViewer><PropertyViewer of=value /></ElementViewer>`
- **THEN** analysis SHALL reject `of=value` because `value` is not a case of `Choice.Property`

### Requirement: `UnionViewer` and `CaseViewer` describe a union's cases
`UnionViewer` SHALL be a record with the type parameter `Target` and an optional content field
`cases` holding one or more `<CaseViewer Target=Target/>`. `CaseViewer` SHALL be a record with the
type parameter `Target`, a required field `of:Target`, and optional fields `label:string` and
`example:string`.

#### Scenario: Case labels
- **WHEN** a module declares `type ChoiceLayout = list | grid | chips` and `viewer ChoiceLayout = <UnionViewer><CaseViewer of=list label="a list" /><CaseViewer of=chips label="chips" example="◉ A  ○ B" /></UnionViewer>`
- **THEN** analysis SHALL accept the viewer

#### Scenario: A case that is not in the union
- **WHEN** `ChoiceLayout` is `list | grid | chips` and its viewer has `<CaseViewer of=table />`
- **THEN** analysis SHALL reject `of=table` because `table` is not a case of `ChoiceLayout`

### Requirement: `ValueViewer` formats a scalar type
`ValueViewer` SHALL be a record with the type parameter `Target` and a required field
`format:ValueFormat`. `ValueFormat` SHALL be the constant union `text | markdown | url | image | color
| email | phone | id`.

#### Scenario: An image URL alias
- **WHEN** a module declares `type ImageUrl = string` and `viewer ImageUrl = <ValueViewer format=image />`
- **THEN** analysis SHALL accept the viewer
