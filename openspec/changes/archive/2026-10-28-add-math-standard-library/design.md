## Context

The compiler already carries `@nx/agent` and `@nx/visualizer` as standard-library source with reserved identities, content-derived versions and ordinary IR modules. Math functions are absent. Existing `apply`/`merge`/`diff`/`changed` intrinsics use globally reserved names and an update-specific IR node. Rust and TypeScript evaluate IR, and executable TypeScript/JavaScript generation also exists. Graphics currently proposes private NX numerical approximations; this change replaces that workaround with a reusable dependency.

## Goals / Non-Goals

**Goals:** a small imported float64 API with clear numeric behavior, ordinary callable identities, explicit runtime compatibility and reproducible tests. Existing programs not linking math retain their encodings and semantics.

**Non-Goals:** new public grammar, globally reserved math names, arbitrary host/FFI bindings, generic numerics, stochastic/time functions, a full scientific library, or Skia work. No requirement that generated source gain unrelated previously unsupported features such as all indirect-call forms.

## Decisions

### 1. An unstable, imported library with ordinary signatures

Use `import "@nx/math" as math` and calls such as `math.sin(angle)`. Constants `pi` and `tau` are float64 nearest-binary64 values of pi and 2*pi (hexadecimal encodings 400921fb54442d18 and 401921fb54442d18). No degree-based overloads: conversion helpers are named explicitly. Public function signatures are:

| Name | Parameters (all float64) | Result |
|---|---|---|
| sin, cos, tan, asin, acos, atan | x | float64 |
| atan2 | y, x | float64 |
| sqrt | x | float64 |
| hypot | x, y | float64 |
| abs, sign, floor, ceil, round, roundTiesToEven, trunc | x | float64 |
| min, max | a, b | float64 |
| clamp | value, min, max | float64 |
| degreesToRadians, radiansToDegrees | x | float64 |
| lerp | a, b, t | float64 |

Use existing contextual numeric conversions; add no implicit integer result conversions. min/max/hypot are binary, not variadic; clamp bounds are inclusive, lerp permits extrapolation. For the first version all functions use a bounded runtime implementation table so finite checks, domain errors, overflow and operation-specific signed-zero behavior are uniform. Constants use ordinary source declarations. Ordinary NX composition remains available to consumers, and future internal implementation changes can move helpers to source without changing this public contract. Do not add special failure intrinsics merely to implement three short helpers in source in this first revision.

### 2. Trusted standard declarations without a new source-language escape hatch

Extend the standard-library registry with typed compiler-owned descriptors for runtime-provided functions alongside source-authored declarations. The registry synthesizes ordinary callable declarations/signatures and validated intrinsic bodies before downstream analysis/codegen; it does not ship fake source implementations returning 0. User libraries cannot supply or replace descriptors. Every entry has its owner module, function identity/signature, closed numerical operation ID and semantic revision. Diagnostics/type schemas/hover see the public signature with the standard-library identity; synthetic origin must be explicit when no actual source span exists.

Version math from its ordinary source plus a canonical ordered serialization of descriptors (signature, operation mapping and semantic revision). Preserve existing library source-only hashes byte-for-byte. Keep math resolution lazy under the existing once-per-process standard-library mechanism. Registry integrity checks enforce mapping/signature agreement before program emission. Public function references continue to use ordinary module/declaration identity; the implementation is attached to the declaration body, so calls through a value do not depend on recognizing the call-site spelling.

Alternatives: new `external function` grammar or global `sin` intrinsic names would broaden the language/host binding contract unnecessarily. Pure NX approximations complicate accuracy and cost. Matching a user function named sin would break shadowing.

### 3. A separate capability-gated numerical body node

Add a dedicated `standardMath` IR node carrying closed operation ID and argument-node references, not additional meanings for update intrinsic opcodes. Allocate the next unused node kind during implementation and document its exact table layout in docs/nx-ir-format.md. It can use the existing node encoding without changing unrelated table layouts. Require `standard-math-v1` on each module containing the node. The standard math module contains the implementation bodies; ordinary callers and function-value references remain ordinary calls and references. An importer that only names math does not need a fake feature flag; linking must validate every dependency image including math.

Validate known operation, exact arity, operands and feature use at prepare time; older runtimes must reject the feature/node before executing. This is an additive required-feature extension of the current image schema if its layouts remain unchanged, with no reinterpretation of existing node kinds; if implementation reveals a layout change, bump schema and generated fixtures together rather than bypassing the existing compatibility rules. Add writer/reader/explain support and malformed-image cases. No user-visible new IR value type or function serialization envelope.

### 4. One strict finite numeric contract, distinct from general arithmetic

All public math arguments must be finite float64 values. Reject NaN and +/-Infinity as `nx-math-non-finite-input`, invalid domains/bounds as `nx-math-domain`, and non-finite final results as `nx-math-overflow`. Include operation name and source origin when available. Do not redefine existing arithmetic's non-finite or division behavior. Graphics keeps its independent finite geometry checks.

sqrt rejects x < 0. asin/acos reject abs(x) > 1; atan accepts any finite x. asin and atan return radians in [-pi/2,pi/2]; acos returns [0,pi]. atan2(y,x) returns [-pi,pi] and accepts zero coordinates, matching JavaScript's finite signed-zero/quadrant cases: atan2(+0,+0)=+0, atan2(-0,+0)=-0, atan2(+0,-0)=+pi, atan2(-0,-0)=-pi.

round returns the nearest integral float64, with ties toward positive infinity, matching JavaScript Math.round. Thus round(2.5)=3, round(-3.5)=-3 and round(-0.5)=-0. Do not implement this simply as floor(x+0.5): that loses negative zero and can introduce addition-rounding errors. roundTiesToEven explicitly supplies nearest-even rounding: 2.5→2, 3.5→4, -2.5→-2, -3.5→-4. Both preserve input -0 and the negative sign when rounding a negative value to zero. floor/ceil/trunc keep their conventional directions and float64 results; ceil/trunc of a negative fraction tending to zero produce -0.

Preserve operation-specific signed zeros instead of globally normalizing them. For finite inputs, functions with JavaScript counterparts follow ECMAScript zero rules: sin/tan/asin/atan/sqrt preserve a -0 input, cos(±0)=1, abs(-0)=+0, sign(±0)=±0, sign(positive)=1, sign(negative)=-1, min(+0,-0)=-0 and max(+0,-0)=+0 regardless of operand order. hypot uses scaling or an equivalent overflow-resistant algorithm, returns +0 for zero inputs, and keeps hypot(1e308,1e308) finite. Check zero sign at the value/host boundary in tests; existing JSON/text serialization conventions are unchanged and are not a guarantee of a signed-zero round trip.

clamp rejects min > max and is defined for valid bounds as min(max(value,min),max), using those signed-zero rules. lerp checks t=0 and t=1 first and returns a or b exactly, including zero signs; then equal endpoints return a, including a's zero sign. For 0<=t<=1 use an overflow-resistant convex-combination algorithm; extrapolation is allowed and final overflow is diagnosed. It never clamps t. Degree conversion uses positive precomputed conversion factors to avoid avoidable intermediate overflow and preserves a zero input's sign.

Exact operations (abs, sign, min/max/clamp and both rounding modes) have exact expected float64 results. Finite transcendental/hypot/conversion/interpolation fixtures compare to independent reference values and each other with abs(error) <= 1e-12 * max(1,abs(reference)); exact endpoints and integer/axis cases have dedicated exact checks where specified. Near-pole tan, huge-angle argument reduction and subnormal/overflow boundaries need named fixtures and explicit expectations; do not imply a universal correctly-rounded guarantee or approve pathological cases by only comparing two implementations. Domain policy does not attempt to identify mathematical tan poles from a floating approximation of pi/2: a finite returned result is allowed and a non-finite one fails.

#### Compatibility boundary with JavaScript

Use the familiar names, radians and atan2(y,x) order, operation-specific signed zeros, and Math.round's tie direction from [ECMAScript Math](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math-object). The library deliberately differs in static float64 typing, finite/domain/overflow diagnostics instead of NaN/infinity propagation, lowercase pi/tau, binary rather than variadic min/max/hypot, and the additional roundTiesToEven/clamp/lerp/conversion helpers. No coercion, omitted arguments or public variadic syntax is added. Accurate sequence reductions and broader exp/log/pow functions remain future work. A direction/vector helper can reject a zero-length vector without changing standard atan2 behavior.

### 5. Runtime and generated-source parity

Rust uses established f64 facilities, TypeScript uses Math with wrappers for differing semantics; JavaScript executable generation uses the same semantic helpers as TypeScript. Rust and TypeScript are the supported runtime implementations; .NET consumes Rust through the existing native FFI binding and has no separate managed evaluator. All enforce the same validation/signed-zero/rounding/overflow rules. Shared golden fixtures exercise direct calls, aliases, passed/returned function values in both IR runtimes, and supported direct-call generated source. The legacy HIR interpreter receives the same behavior while it remains a supported path; do not make math depend on the unrelated retire-hir-interpreter change. Node/WASM/native/.NET runtime entry points use the existing runtime with no new public binding calls.

Each standardMath node adds one operation, besides the ordinary costs of its argument nodes and function call/body evaluation. The closed primitive performs bounded work independent of user sequence/string size. No per-host callback or hidden loop bypasses budgets. Budget failures retain normal evaluation/origin behavior.

Executable generation resolves trusted identities and emits known numerical helpers; it never binds arbitrary source names to host Math. Source generators which already reject a particular indirect-call form continue to emit their existing unsupported-feature diagnostic for it, rather than claim newly broad codegen support. Basic math calls and constants must work for both executable targets.

## Risks / Trade-offs

- Host math differences near ill-conditioned inputs → independent golden values, named boundary fixtures and stated tolerance; no bit-identical transcendental promise.
- Registry descriptors create a new compiler-owned declaration path → closed table, owner/signature verification, deterministic versions and user-spoof tests.
- Different rounding/overflow behavior across targets → explicit wrappers, shared corpus and generated-source execution tests.
- Non-finite library policy differs from existing arithmetic → clearly document the library boundary and test NaN/Infinity supplied through host inputs.
- IR reader drift → feature-gated body node, malformed-image tests and generated format tables/fixtures updated together.
- Existing interpreter retirement runs separately → keep parity while supported, with no dependency on that change.

## Migration Plan

Land this change first, including cross-runtime/generator conformance and unstable documentation. Verify images not using math remain unchanged. Update graphics to import math and test helper geometry rather than reimplementing numerical primitives. Existing users opt in by import. Unsupported older runtimes reject math's feature explicitly; compiler/runtime packages used together must be upgraded together. No stable graphics data or external application deployment is changed.
