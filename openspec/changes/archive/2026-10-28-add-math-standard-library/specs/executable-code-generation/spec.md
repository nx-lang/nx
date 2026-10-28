## ADDED Requirements

### Requirement: Executable generators preserve math-library behavior
Executable TypeScript and JavaScript generation SHALL support direct calls and constants from @nx/math with the same finite/domain/overflow/rounding behavior as IR execution. Implementations SHALL be selected by trusted declaration identity, not source spelling, and generated output SHALL require no runtime host callback. Calls using an otherwise unsupported existing code-generation construct SHALL retain explicit unsupported-feature diagnostics rather than silently approximate behavior or expand the claimed support surface.

#### Scenario: Generated rounding matches runtime semantics
- **WHEN** a program computes round(-3.5) and roundTiesToEven(-3.5) and is emitted as executable TypeScript and JavaScript
- **THEN** both outputs SHALL return -3.0 and -4.0 respectively rather than inherit different target defaults

#### Scenario: Generated domain failure matches evaluation
- **WHEN** executable output evaluates sqrt(-1.0)
- **THEN** it SHALL fail with the nx-math-domain category naming sqrt

#### Scenario: User function is not rebound to host math
- **WHEN** a program calls an authored function named cos
- **THEN** generated source SHALL execute that authored function

#### Scenario: Generated signed-zero behavior
- **WHEN** executable output evaluates atan2(-0.0,+0.0), sign(-0.0), round(-0.5) and max(-0.0,+0.0)
- **THEN** the first three runtime values SHALL be negative zero and the last positive zero, checked by the target's sign-aware numeric observation rather than serialized text
