## ADDED Requirements

### Requirement: Properties and aliases carry their constraint's bounds
Each property entry and each alias declaration SHALL carry a `bounds` value when its type, directly
or through aliases, is constrained: the entries of a declaration's `properties` and `state` lists,
and a declaration that is an alias. It SHALL be present whether or not the request asks for
`visualizers`. `bounds` SHALL hold `min` and `max` as numbers for the ends
the interval has, and `minExclusive` and `maxExclusive`, each present and `true` only when that end
is exclusive. The property's `type` SHALL spell the constraint as written. An unconstrained property
SHALL carry no `bounds`. The value SHALL round-trip through JSON without loss.

#### Scenario: A closed range
- **WHEN** a client asks for the source tree of a document that constructs `<Paint opacity=0.5 />`, where `Paint.opacity` is typed `float64 where value in 0.0..=1.0`
- **THEN** the `properties` entry for `opacity` in `Paint`'s declaration SHALL carry `bounds` with
  `min` `0` and `max` `1` and no exclusive flags

#### Scenario: Through an alias, with an exclusive end
- **WHEN** `Paint.layer` is typed `Unit`, where `type Unit = float64 where value >= 0.0 && value < 1.0`
- **THEN** the entry for `layer` SHALL carry `bounds` with `min` `0`, `max` `1` and `maxExclusive`
  `true`, and `Unit`'s declaration SHALL carry the same `bounds`
