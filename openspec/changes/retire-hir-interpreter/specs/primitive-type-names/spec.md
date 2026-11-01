## ADDED Requirements

### Requirement: Sized integer arithmetic follows the existing int backend contract
Arithmetic on `int32` and `int64` values SHALL use the backend's existing `int` arithmetic.
Native IR execution SHALL NOT add the retired interpreter's special 32-bit wrapping for `int32`.
This SHALL NOT extend the existing precision or overflow guarantees of `int`, require exact
JavaScript arithmetic beyond its safe integer range, or change the Rust runtime's i64 overflow behavior.

#### Scenario: int32 addition does not wrap at 32 bits
- **WHEN** a program adds two `int32` values of `2000000000`
- **THEN** the Rust IR runtime, generated JavaScript and TypeScript IR runtime SHALL return `4000000000`

#### Scenario: Backend limits are not promoted to universal parity
- **WHEN** numeric behavior is tested beyond the shared exact integer range
- **THEN** tests SHALL assert the existing backend-specific precision and overflow contract
- **AND** SHALL NOT require exact cross-backend equality for all i64 values
