## ADDED Requirements

### Requirement: A constrained numeric type takes numeric editor hints as its base does
Analysis SHALL treat a constrained numeric type, written inline or through an alias, as its base type
when it decides whether an editor hint fits a property, so `editor=field`, `min`, `max`, `step` and a
number `ValueChoice` SHALL fit an `int where …` or `float64 where …` property and SHALL NOT be
reported as `editor-hint-mismatch`. A hint that fits SHALL NOT be reported for lying outside the
constraint's interval; the viewer intersects the two, as `viewer` requires.

#### Scenario: A max hint on a constrained property
- **WHEN** an element has `columns: int where in 1..=1000` and its visualizer has `<PropertyVisualizer property=columns max=12>`
- **THEN** analysis SHALL report no `editor-hint-mismatch` for `max=12`

#### Scenario: A hint through a constrained alias
- **WHEN** a file contains `type Ratio = float32 where in 0.0..=1.0`, an element with `opacity: Ratio`, and `<PropertyVisualizer property=opacity step=0.05>`
- **THEN** analysis SHALL report no `editor-hint-mismatch` for `step=0.05`
