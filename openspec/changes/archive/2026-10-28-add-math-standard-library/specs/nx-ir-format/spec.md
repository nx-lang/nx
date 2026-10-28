## ADDED Requirements

### Requirement: Standard math bodies use a closed capability-gated expression
The IR SHALL encode runtime-provided standard math bodies with a dedicated expression kind containing a recognized operation identifier and argument references, distinct from update intrinsics. A module containing that expression SHALL declare `standard-math-v1`. Ordinary calls/references/function values SHALL retain existing encodings. Preparing SHALL validate operation, arity, argument references and feature declaration; linking SHALL validate all supplied dependency images. Unsupported runtimes SHALL reject the required feature or unknown expression before evaluation. Unrelated image layouts and math-free output SHALL remain unchanged unless an actual incompatible layout change requires the ordinary schema-version migration.

#### Scenario: Imported math body links with caller
- **WHEN** an ordinary caller links the math library image containing runtime-provided function bodies
- **THEN** the math image SHALL declare standard-math-v1 and the ordinary caller SHALL keep normal call/reference encoding
- **AND** a runtime lacking standard-math-v1 SHALL reject the dependency before executing it

#### Scenario: Malformed math body is refused
- **WHEN** an image has an unknown math operation, wrong argument count, invalid reference or absent required feature
- **THEN** preparation SHALL fail with a diagnostic identifying the malformed construct

#### Scenario: Existing program does not change
- **WHEN** the same non-math program and libraries are emitted before and after this feature
- **THEN** their images SHALL remain byte-identical
