## Purpose

Provides a small pure mathematical standard library with predictable float64 behavior for NX graphics, charts, layout calculations and general authored computations.

## ADDED Requirements

### Requirement: Math is an imported unstable float64 library
The system SHALL provide unstable `@nx/math` with float64 pi and tau constants and these float64-returning functions: sin(x), cos(x), tan(x), asin(x), acos(x), atan(x), atan2(y,x), sqrt(x), hypot(x,y), abs(x), sign(x), min(a,b), max(a,b), clamp(value,min,max), floor(x), ceil(x), round(x), roundTiesToEven(x), trunc(x), degreesToRadians(x), radiansToDegrees(x), and lerp(a,b,t). All parameters SHALL be float64 and follow existing numeric conversion rules. Trigonometric input/output angles SHALL be radians. pi and tau SHALL be nearest-binary64 pi and 2*pi. Names SHALL require import and SHALL NOT become reserved global identifiers.

#### Scenario: Aliased imported math
- **WHEN** a program imports `@nx/math` as math and evaluates math.sin(math.pi / 2.0)
- **THEN** the result SHALL approximate 1 within the documented tolerance
- **AND** a user function named sin outside that import SHALL remain an ordinary user function

### Requirement: Math rejects invalid numerical inputs and domains
Every public math function SHALL reject non-finite arguments with `nx-math-non-finite-input`, invalid domains/bounds with `nx-math-domain`, and a non-finite final result with `nx-math-overflow`. sqrt SHALL reject negative input; asin and acos SHALL reject inputs outside [-1,1]; clamp SHALL reject min greater than max. atan2 SHALL accept both-zero finite coordinates using the signed-zero cases below. Errors SHALL identify the operation and available call origin. Existing general arithmetic behavior SHALL remain unchanged. The library SHALL preserve operation-specific signed-zero behavior rather than normalize every zero result. Strict finite/domain/overflow diagnostics SHALL be documented as a deliberate difference from JavaScript; dynamic coercion and variadic signatures SHALL NOT be added.

#### Scenario: Invalid square root and reversed bounds
- **WHEN** sqrt(-1.0) or clamp(1.0,3.0,2.0) is evaluated
- **THEN** evaluation SHALL fail with nx-math-domain naming the operation

#### Scenario: Host-provided non-finite value
- **WHEN** a host supplies NaN or infinity to a public math parameter
- **THEN** evaluation SHALL fail with nx-math-non-finite-input instead of silently propagating it

### Requirement: Rounding and geometry helpers have explicit edge behavior
round SHALL use nearest integer with ties toward positive infinity, matching JavaScript Math.round. roundTiesToEven SHALL use nearest integer with ties to even. Both SHALL preserve -0 inputs and a negative sign when a negative value rounds to zero. floor/ceil/trunc SHALL retain their conventional directions and float64 results. atan2(y,x) SHALL use conventional quadrants with range [-pi,pi], including finite signed-zero coordinate cases. asin and atan SHALL return radians in [-pi/2,pi/2], and acos in [0,pi]. hypot SHALL avoid intermediate squaring overflow when its result is representable. clamp SHALL include its bounds and equal min(max(value,min),max). lerp SHALL permit extrapolation, return exact endpoints for t=0/1 before returning a for equal endpoints, preserve those endpoint zero signs, avoid avoidable convex-combination overflow, and SHALL NOT clamp t. Degree conversion SHALL avoid avoidable intermediate overflow and preserve zero input signs.

For finite inputs, shared JavaScript-named functions SHALL follow ECMAScript operation-specific zero behavior: sin/tan/asin/atan/sqrt preserve -0 input, cos of either zero returns 1, abs(-0) returns +0, sign preserves either zero and returns -1/1 for nonzero negative/positive input, min prefers -0 and max prefers +0 when comparing zeros, and hypot of zero inputs returns +0. ceil/trunc of negative fractions approaching zero SHALL return -0. This SHALL NOT change existing generic serialization's representation of zeros.

#### Scenario: JavaScript-compatible and explicit nearest-even rounding
- **WHEN** round is called with 2.5, 3.5, -2.5 and -3.5
- **THEN** results SHALL be 3.0, 4.0, -2.0 and -3.0 respectively
- **WHEN** roundTiesToEven is called with those inputs
- **THEN** results SHALL be 2.0, 4.0, -2.0 and -4.0 respectively
- **WHEN** round(-0.5), roundTiesToEven(-0.5), ceil(-0.1) or trunc(-0.1) is evaluated
- **THEN** its value SHALL be negative zero

#### Scenario: Finite signed-zero atan2 cases
- **WHEN** atan2 is evaluated for (+0,+0), (-0,+0), (+0,-0) and (-0,-0) in y,x order
- **THEN** results SHALL be +0, -0, +pi and -pi respectively without a domain error

#### Scenario: Signed zeros in comparisons and sign
- **WHEN** min and max compare +0 with -0 in either order
- **THEN** min SHALL return -0 and max SHALL return +0
- **WHEN** sign is evaluated for -0, +0, -2 and 2
- **THEN** results SHALL be -0, +0, -1 and 1 respectively

#### Scenario: Inverse trigonometric domains
- **WHEN** asin(1.0), acos(1.0) and atan(0.0) are evaluated
- **THEN** results SHALL be approximately pi/2, exactly +0 and exactly +0 respectively
- **WHEN** asin(1.1) or acos(-1.1) is evaluated
- **THEN** evaluation SHALL fail with nx-math-domain

#### Scenario: Large finite distance and interpolation
- **WHEN** hypot(1e308,1e308) is evaluated
- **THEN** the result SHALL be finite and approximately 1.4142135623730951e308
- **WHEN** lerp(-1e308,1e308,0.5) is evaluated
- **THEN** the result SHALL be finite and approximately zero

#### Scenario: Extrapolation and overflow
- **WHEN** lerp(0.0,10.0,1.5) is evaluated
- **THEN** the result SHALL be 15.0
- **WHEN** a mathematical operation produces an unrepresentable finite-range result
- **THEN** evaluation SHALL fail with nx-math-overflow

### Requirement: Math implementations pass a shared numerical conformance corpus
The Rust and TypeScript IR runtimes, supported legacy evaluation path and executable TypeScript/JavaScript outputs SHALL agree on exact comparison/sign/rounding/endpoint results, including zero signs checked independently of ordinary numeric equality, and diagnostic categories. Inexact numerical fixtures SHALL compare against independent reference values with absolute error at most 1e-12 times max(1,abs(reference)); the corpus SHALL include quadrants, signed zeros, tiny/large values, negative inputs, overflow and near-pole trigonometry. No bit-identical or universally correctly-rounded transcendental guarantee SHALL be claimed.

#### Scenario: Same operation across execution paths
- **WHEN** a shared reference fixture is executed through each supported path
- **THEN** its value SHALL satisfy the reference tolerance or its specified diagnostic SHALL match
- **AND** passing only by agreement between two implementations SHALL NOT replace independent reference checks
