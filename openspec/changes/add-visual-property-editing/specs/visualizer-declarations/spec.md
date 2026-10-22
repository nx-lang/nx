## ADDED Requirements

### Requirement: A property's editor hints come from its visualizer or its type
Each editor hint of a property SHALL resolve on its own (`editor`, `min`, `max`, `step`, `stops`,
`unit`, `multiline` and `placeholder`): to the value its resolved `PropertyVisualizer` sets, and
otherwise, for the hints `ValueVisualizer` has, to the value set by the `ValueVisualizer` of the alias
the property's declared type names, read through occurrence suffixes as `format` is; otherwise to
none. A `PropertyVisualizer` that sets some hints SHALL leave the others to the alias.

#### Scenario: An alias's slider reaches every property it types
- **WHEN** a module declares `type Spacing = int` with `visualizer Spacing = <ValueVisualizer editor=slider min=0 max=64 />`, and `type Card = { padding:Spacing = 8 gap?:Spacing }`
- **THEN** the resolved hints of `Card.padding` and `Card.gap` SHALL be the editor `slider`, the minimum `0` and the maximum `64`

#### Scenario: A property visualizer overrides one hint
- **WHEN** `Spacing` is hinted as above and `Card`'s visualizer has `<PropertyVisualizer property=padding max=32 unit="px" />`
- **THEN** the resolved hints of `Card.padding` SHALL be the editor `slider`, the minimum `0`, the maximum `32` and the unit `px`

### Requirement: An editor hint that does not fit its property is a warning
Analysis SHALL report an `editor-hint-mismatch` warning at a hint that does not fit the type of the
property it resolves for, and a viewer SHALL ignore that hint: `editor=slider`, `editor=field`, `min`,
`max`, `step`, `stops` or `unit` where the type is not `int`, `float32`, `float64` or an alias of one;
`editor=dropdown` or `editor=segmented` where it is not a union of cases with no properties; and
`multiline` or `placeholder` where it is not a string or an alias of one. A `ValueVisualizer`'s hint
SHALL be checked against its target. `min` greater than `max` SHALL be reported as an
`editor-hint-range` warning. These SHALL be warnings, not errors, since a hint never changes what a
program means.

#### Scenario: A slider on a string
- **WHEN** `Card`'s visualizer has `<PropertyVisualizer property=title editor=slider />` and `title` is a `string`
- **THEN** analysis SHALL report `editor-hint-mismatch` at `editor=slider`

#### Scenario: An empty range
- **WHEN** a `ValueVisualizer` sets `min=10 max=0`
- **THEN** analysis SHALL report `editor-hint-range`
