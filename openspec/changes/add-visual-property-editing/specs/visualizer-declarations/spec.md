## ADDED Requirements

### Requirement: A property's editor hints come from its visualizer or its type
Each editor hint of a property SHALL resolve on its own (`editor`, `min`, `max`, `step`, `unit`,
`multiline`, `placeholder` and `choices`): to the value its resolved `PropertyVisualizer` sets, and
otherwise, for the hints `ValueVisualizer` has, to the value set by the `ValueVisualizer` of the alias
the property's declared type names, read through occurrence suffixes as `format` is; otherwise to
none. A `PropertyVisualizer` that sets some hints SHALL leave the others to the alias. The choices
are one hint: a visualizer that gives any SHALL replace all of an earlier or an alias's, in its
order.

#### Scenario: An alias's hints reach every property it types
- **WHEN** a module declares `type Spacing = int` with `visualizer Spacing = <ValueVisualizer min=0 max=64><ValueChoice value=8 label="md" /></ValueVisualizer>`, and `type Card = { padding:Spacing = 8 gap?:Spacing }`
- **THEN** the resolved hints of `Card.padding` and `Card.gap` SHALL be the minimum `0`, the maximum `64` and the choice `8` labeled `md`

#### Scenario: A property visualizer overrides one hint
- **WHEN** `Spacing` is hinted as above and `Card`'s visualizer has `<PropertyVisualizer property=padding max=32 unit="px" />`
- **THEN** the resolved hints of `Card.padding` SHALL be the minimum `0`, the maximum `32`, the unit `px` and the choice `8` labeled `md`

#### Scenario: A property's choices replace its type's
- **WHEN** `Spacing` is hinted as above and `Card`'s visualizer has `<PropertyVisualizer property=gap><ValueChoice value=2 /><ValueChoice value=4 /></PropertyVisualizer>`
- **THEN** the resolved choices of `Card.gap` SHALL be `2` and `4`, without labels, and those of `Card.padding` SHALL be `8` labeled `md`

### Requirement: An editor hint that does not fit its property is a warning
Analysis SHALL report an `editor-hint-mismatch` warning at a hint that does not fit the type of the
property it resolves for, and a viewer SHALL ignore that hint: `editor=field`, `min`, `max`, `step`
or `unit` where the type is not `int`, `int32`, `int64`, `float32`, `float64` or an alias of one;
`editor=dropdown` or `editor=segmented` where it is not a union of cases with no properties;
`multiline` or `placeholder` where it is not a string or an alias of one; and a `ValueChoice` whose
`value` is not a number literal, signed or not, where the type is a number, or not a string literal
where it is a string, and any choice where the type is neither. A `ValueVisualizer`'s hint
SHALL be checked against its target. `min` greater than `max` SHALL be reported as an
`editor-hint-range` warning. These SHALL be warnings, not errors, since a hint never changes what a
program means.

#### Scenario: A field on a string
- **WHEN** `Card`'s visualizer has `<PropertyVisualizer property=title editor=field />` and `title` is a `string`
- **THEN** analysis SHALL report `editor-hint-mismatch` at `editor=field`

#### Scenario: A string choice for a number
- **WHEN** `Card`'s visualizer has `<PropertyVisualizer property=padding><ValueChoice value="wide" /></PropertyVisualizer>` and `padding` is an `int`
- **THEN** analysis SHALL report `editor-hint-mismatch` at `value="wide"`

#### Scenario: An empty range
- **WHEN** a `ValueVisualizer` sets `min=10 max=0`
- **THEN** analysis SHALL report `editor-hint-range`
