## ADDED Requirements

### Requirement: A number's editor keeps within its constraint
The viewer SHALL bound a number's editor by its property's `bounds`, with or without editor hints,
and SHALL edit a value as a number when its declared type is a constrained numeric type, whose
spelling now carries a `where` clause, as it does for the base type. These rules add to "Editor hints
choose and shape a value's editor" for a property with `bounds`, and change nothing for one without:
the field, arrows, steppers and drag SHALL stay within the bounds, and a typed number outside them
SHALL stay in the field with a note and SHALL NOT be committed. The note SHALL read `At least N` or
`At most N` for an inclusive end and `Above N` or `Below N` for an exclusive end. When the property
also has `min` or `max` hints, the editor SHALL keep within the intersection of the hints and the
bounds, so a hint can narrow the range the editor offers but not widen it; a hint range that does not
overlap the bounds SHALL be ignored and the bounds used alone. Steppers and arrows SHALL
NOT step onto or past an exclusive end: a step that would SHALL leave the value where it is. Bounds
arrive with an integer's ends already inclusive. A choice outside the bounds SHALL be refused as a choice outside the hints is. When a
floating-point property has no `step` hint and its bounds have both ends, its step SHALL be the
largest power of ten not above a tenth of the span, so a `0.0..=1.0` number steps by `0.1`; an
integer's step SHALL stay `1`.

#### Scenario: A constraint alone bounds the steppers
- **WHEN** the reader steps up `opacity=0.9` twice, where `opacity` is `float64 where in 0.0..=1.0` with no visualizer
- **THEN** the first step SHALL commit `1.0` and the second SHALL leave the value at `1.0`

#### Scenario: Out of range
- **WHEN** the reader types `1.5` in that field and presses Enter
- **THEN** no `nx-edit` SHALL fire and a note beside the field SHALL read `At most 1`

#### Scenario: An exclusive end
- **WHEN** the reader types `0` in the field of a number with `bounds` `min` `0` and `minExclusive` `true`
- **THEN** no `nx-edit` SHALL fire and the note SHALL read `Above 0`

#### Scenario: A hint narrows the constraint
- **WHEN** `columns` is `int where in 1..=1000`, hinted `max=12`, and the reader types `20`
- **THEN** the note SHALL read `At most 12`
- **AND** with the hint `max=2000` instead, typing `1500` SHALL show `At most 1000`
