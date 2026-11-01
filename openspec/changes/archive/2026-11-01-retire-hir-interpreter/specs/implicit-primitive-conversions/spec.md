## MODIFIED Requirements

### Requirement: Every primitive has one canonical text form
The system SHALL define one text form for each stringifiable primitive, and every backend — the Rust and TypeScript IR runtimes and generated code — SHALL produce that same text wherever this
capability converts a value to a string. The forms SHALL be:

- `int`, `int32`, `int64`: the value's decimal digits with no grouping, no leading zeros, and a
  leading `-` when negative.
- `float64`: the ECMAScript `Number::toString` form. An integral value prints without a fraction
  (`1`, not `1.0`), a fractional value prints the shortest digits that round-trip to the same
  `float64`, magnitudes at or above 10^21 or below 10^-6 use the exponent form (`1e+21`, `1e-7`),
  negative zero prints as `0`, and the non-finite values print as `NaN`, `Infinity` and `-Infinity`.
- `float32`: the same rules, with the shortest digits that round-trip to the same `float32`, so a
  `float32` holding `0.1` prints as `0.1` and not as the expansion of its `float64` widening.
- `boolean`: `true` or `false`.

Backends SHALL agree on an integer's text wherever they agree on its value: over the ±(2^53−1)
range `int` is specified to (`primitive-type-names`). An `int64` beyond that range is carried as a
JavaScript `number` until the `int64` carrier change, and its text is outside this guarantee.

#### Scenario: Integral floats print without a fraction
- **WHEN** a `float64` holding `1.0` is concatenated to a string
- **THEN** the text SHALL be `1`

#### Scenario: Fractional floats print the shortest round-trip digits
- **WHEN** a `float64` holding `0.1` is concatenated to a string
- **THEN** the text SHALL be `0.1`

#### Scenario: Large magnitudes use the exponent form
- **WHEN** a `float64` holding `1e21` is concatenated to a string
- **THEN** the text SHALL be `1e+21`

#### Scenario: A float32 prints as a float32
- **WHEN** a `float32` holding the nearest representation of `0.1` is concatenated to a string
- **THEN** the text SHALL be `0.1`

#### Scenario: Backends agree
- **WHEN** the same program concatenates the same `float64` value to a string
- **THEN** the Rust IR runtime, the TypeScript IR runtime and generated JavaScript SHALL produce identical text
