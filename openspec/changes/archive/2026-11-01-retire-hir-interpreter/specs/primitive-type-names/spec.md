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

## MODIFIED Requirements

### Requirement: `int` has one specified range on every backend
`int` SHALL be exact over ±(2^53−1) on every backend. That range SHALL NOT vary by target,
by host word size, or by which NX implementation evaluates the program: `int` SHALL NOT be an
implementation-defined width. Backends MAY store an `int` in any slot that holds the whole
specified range — a C# `long`, a Rust `i64`, a JavaScript `number` — because a backend's choice
among those is unobservable to an NX program.

Arithmetic on `int` SHALL be defined as checked rather than wrapping: a result outside the
specified range is an error, not a silently truncated value.

Enforcement of both the range and the checked-arithmetic rule is deliberately deferred to a later
change, which also covers user-declared ranges (`1..10`) so that one bounds-check mechanism serves
both. Until that change lands, the range is a specified guarantee that the runtime does not yet
police, and integer arithmetic continues to wrap.

#### Scenario: `int` is specified independently of the evaluating backend
- **WHEN** the same NX program is evaluated by the Rust IR runtime, by generated C#, and by
  generated TypeScript
- **THEN** the specified range of `int` SHALL be identical in all three

#### Scenario: A backend may choose any storage that covers the range
- **WHEN** a backend stores `int` in a C# `long`, a Rust `i64`, or a JavaScript `number`
- **THEN** the choice SHALL NOT change the specified range
