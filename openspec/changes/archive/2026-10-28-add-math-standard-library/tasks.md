## 1. Library declarations and identity

- [x] 1.1 Add the closed trusted standard-function descriptor path alongside source-authored standard libraries; verify user libraries cannot register/replace implementations and source-only library hashes remain unchanged.
- [x] 1.2 Register unstable @nx/math with the specified float64 signatures/constants and deterministic descriptor-aware versioning; verify aliases, selective imports, shadowing, hover/schema signatures and lazy standard-library loading.
- [x] 1.3 Add direct/passed/returned function-reference fixtures for standard math; verify they resolve ordinary declaration identities and same-named authored functions retain their own bodies.

## 2. IR and compatibility

- [x] 2.1 Allocate/document the dedicated standardMath expression and standard-math-v1 feature, extending writer/reader/explain validation; verify valid images and malformed operation/arity/reference/feature fixtures.
- [x] 2.2 Emit trusted standard-function bodies while retaining normal calls/references; verify math's dependency image is capability-gated and a runtime without support rejects it before execution.
- [x] 2.3 Extend content-version/format fixtures as needed without changing unrelated layouts; verify representative math-free images are byte-identical and all supported format readers remain synchronized.

## 3. Numerical implementation and errors

- [x] 3.1 Implement Rust sin/cos/tan/asin/acos/atan/atan2/sqrt/hypot with finite/domain/result validation and operation-specific signed zeros; verify independent reference/inverse-domain/quadrant/both-zero-atan2/large-distance fixtures and diagnostic categories.
- [x] 3.2 Implement Rust abs/sign/min/max/clamp/floor/ceil/round/roundTiesToEven/trunc with JavaScript-compatible round and explicit nearest-even rounding; verify halfway negatives, negative-zero results, zero comparisons/sign, reversed clamp bounds, exact outputs and non-finite host inputs.
- [x] 3.3 Implement angle conversions and robust lerp including exact endpoints/equal values/extrapolation; verify convex interpolation at large opposite values, conversion overflow boundaries and out-of-range t behavior.
- [x] 3.4 Implement equivalent TypeScript runtime operations/wrappers; verify the same golden corpus (including new inverse trig/sign/both rounding modes and sign-bit checks), retained function-value behavior and Rust/TypeScript comparisons within stated tolerances.
- [x] 3.5 Integrate origin-aware errors and one-operation body accounting in both runtimes; verify nested/function-value error origins and bounded evaluation under exhausted budgets.
- [x] 3.6 Preserve parity in the supported legacy interpreter without depending on its retirement; verify direct/aliased math fixtures through that path or remove this task only after retirement is independently complete.

## 4. Generated code and binding paths

- [x] 4.1 Emit executable TypeScript math support by trusted identity with necessary semantic wrappers; run generated programs for every function and invalid-domain/rounding/overflow case.
- [x] 4.2 Emit executable JavaScript math support with the same contracts as TypeScript; run generated programs using the shared corpus and verify operation names/categories on failures.
- [x] 4.3 Exercise Node, WASM and the .NET binding to the Rust runtime entry points with a math-linked program and emitted library images; verify no new binding API or arbitrary callback registration is required.
- [x] 4.4 Preserve existing unsupported indirect-call diagnostics in executable generators; verify supported direct math calls work and no unrelated function-value codegen support is claimed.

## 5. Conformance, docs and downstream readiness

- [x] 5.1 Add independent reference values with provenance/method and named near-pole, huge-angle, subnormal, signed-zero and overflow fixtures; verify every path satisfies exact/tolerance/error expectations instead of only agreeing with another runtime.
- [x] 5.2 Update the NX documentation as part of implementation: add `sites/website/src/content/docs/reference/libraries/math.md`, link it from the relevant language-tour functions/import pages and documentation navigation, and update `docs/nx-ir-format.md` for standardMath and standard-math-v1. Document signatures, radians, JavaScript-compatible round, explicit roundTiesToEven, signed-zero behavior, deliberate strict finite/domain/result policy, tolerances, instability and compiler/runtime compatibility; verify all public examples compile/evaluate, documentation links/build pass and release notes list @nx/math.
- [x] 5.3 Add a compiler/runtime-only polygon/star/arrow-coordinate example using @nx/math; verify identical primitive-coordinate expectations in Rust/TypeScript without Skia or graphics host code.
- [x] 5.4 Run focused library/IR/runtime/codegen/binding suites and strict OpenSpec validation; record commands/results and confirm the graphics change can consume the delivered library without numerical approximations.
