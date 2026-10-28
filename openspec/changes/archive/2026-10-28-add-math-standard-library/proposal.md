## Why

NX graphics helpers need reliable trigonometry, distances and rounding, but NX has no math standard library. A small pure library with runtime-backed numerical primitives avoids bespoke approximations and creates a reusable foundation for graphics, charts and layout calculations.

## What Changes

- Add unstable `@nx/math` with float64 functions: sin, cos, tan, asin, acos, atan, atan2, sqrt, hypot, abs, sign, min, max, clamp, floor, ceil, round, roundTiesToEven, trunc, degreesToRadians, radiansToDegrees and lerp; constants pi and tau.
- Define radians, argument order, finite-input/domain/result errors, JavaScript-compatible round with explicit roundTiesToEven, operation-specific signed-zero behavior, and numerical conformance tolerances explicitly.
- Add a closed, compiler-owned mechanism for pure runtime-provided standard functions with ordinary imported identities and first-class function behavior, without new public syntax, reserved global names, or arbitrary host bindings.
- Support the mechanism in IR emission/validation, Rust and TypeScript evaluation, and executable TypeScript/JavaScript generation; preserve ordinary NX composition while keeping the initial numerical implementations behind the trusted library contract.
- Keep strict finite/domain/result diagnostics as an explicit difference from JavaScript; retain NX typing, lowercase constants, binary min/max/hypot and explicit conversion/clamp/lerp conveniences.
- Add cross-runtime/generator conformance fixtures and capability gating for older runtimes; preserve existing programs' encodings and semantics.
- Update the NX documentation as part of implementation: add the `@nx/math` library reference to the documentation site, link it from the language tour/import guidance, and update the IR reference for the new encoding and required feature. Include executable examples and the JavaScript compatibility differences.
- Make `add-nx-graphics-core-and-direct-skia` depend on this change and remove its private trigonometric/square-root approximations.

## Capabilities

### New Capabilities

- `math-library`: Public math surface, numeric behavior and conformance.
- `runtime-standard-functions`: Trusted pure standard functions, declaration identity, callable semantics, diagnostics and execution budgets.

### Modified Capabilities

- `standard-libraries`: Compiler-owned runtime-provided declarations alongside source-authored declarations and content-derived versioning.
- `nx-ir-format`: Add a capability-gated standard-function body encoding without altering ordinary call/function-value contracts.
- `executable-code-generation`: Emit equivalent math behavior and diagnostics for the new library rather than silently substituting target defaults.

## Impact

- Compiler standard-library registry and math sources/descriptors; HIR/codegen and typed declaration APIs where needed for ordinary signatures.
- NX IR writer/reader/explain/validation, Rust and TypeScript runtimes, built-in feature declarations and diagnostic/operation accounting.
- Executable TypeScript/JavaScript code generation and generated helper support; existing runtime-backed .NET/Node/WASM entry points exercise the same math contract.
- Reference documentation, cross-runtime fixtures and the graphics plan. No renderer or general-purpose foreign-function API is added.

Rust and TypeScript are the runtime implementations. .NET support is provided through the existing Rust native-library binding; this change adds no C# evaluator or executable C# backend.

## Non-goals

Integer overloads/generic numeric types, arbitrary external functions, randomness/time, vector/matrix APIs, full scientific math, bit-identical transcendental results, or implementation of graphics itself.
