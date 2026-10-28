---
title: 'The math library'
description: 'Imported float64 mathematics, numerical conventions, and runtime compatibility.'
---

`@nx/math` is an **unstable** standard library carried by the compiler. Import it explicitly;
its names are ordinary declarations and can be qualified or selectively imported.

```nx
import "@nx/math" as math

let radius:float64 = 20.0
let angle:float64 = { math.degreesToRadians(60.0) }
let root():float64 = { radius * math.cos(angle) }
```

The library's signatures and `///` documentation are declared in NX source using
[`external`](/reference/syntax/functions#external-functions). Hover and completion show those
comments; Rust and TypeScript supply the validated runtime implementations. For example, the
library declares:

```nx fragment
/// Returns the sine of x, measured in radians. Preserves negative zero.
/// Rejects non-finite inputs.
export external sin(x:float64):float64
```

All parameters and results are `float64`, with NX's existing contextual numeric conversions.
The constants `pi` and `tau` are the nearest binary64 values of π and 2π. Trigonometric functions
use radians; `degreesToRadians` and `radiansToDegrees` convert explicitly.

| Functions | Parameters | Meaning |
| --- | --- | --- |
| `sin`, `cos`, `tan` | `x` | Trigonometric functions, radians |
| `asin`, `acos`, `atan` | `x` | Inverse functions, results in radians |
| `atan2` | `y, x` | Signed direction in `[-pi, pi]` |
| `sqrt` | `x` | Square root; requires `x >= 0` |
| `hypot` | `x, y` | Distance with protection against intermediate squaring overflow |
| `abs`, `sign` | `x` | Magnitude; sign is `-1`, `0`, or `1` |
| `min`, `max` | `a, b` | Minimum or maximum of two values |
| `clamp` | `value, min, max` | Inclusive bounds; requires `min <= max` |
| `floor`, `ceil`, `trunc` | `x` | Round down, up, or toward zero |
| `round` | `x` | Nearest integer; halfway values round toward positive infinity |
| `roundTiesToEven` | `x` | Nearest integer; halfway values round to an even integer |
| `degreesToRadians`, `radiansToDegrees` | `x` | Angle conversions |
| `lerp` | `a, b, t` | Linear interpolation; extrapolation is allowed |

`asin` and `acos` require `x` in `[-1, 1]`; `asin` and `atan` return values in `[-pi/2, pi/2]`,
and `acos` returns `[0, pi]`. `atan2(0.0, 0.0)` is valid. `lerp` returns the exact endpoint at
`t = 0` or `t = 1`, returns `a` for equal endpoints otherwise, and avoids intermediate overflow
for a representable convex combination. It does not clamp `t`.

## JavaScript conventions and differences

Names, radians, `atan2(y, x)`, and `round` follow JavaScript's conventions. For example,
`round(-3.5)` is `-3.0`; `roundTiesToEven(-3.5)` is `-4.0`. NX uses lowercase `pi` and `tau`,
static numeric types, and binary `min`, `max`, and `hypot` instead of variadic functions.

The library preserves operation-specific signed zeros. `sin`, `tan`, `asin`, `atan`, `sqrt`,
`sign`, rounding, and angle conversions preserve an input `-0`. `round(-0.5)`,
`roundTiesToEven(-0.5)`, `ceil(-0.1)`, and `trunc(-0.1)` produce `-0`. `abs(-0)` is `+0`;
`min` prefers `-0` and `max` prefers `+0` when comparing zeros. `hypot` of zeros returns `+0`.
`clamp` is defined as `min(max(value, min), max)`, including those zero rules.

`atan2` distinguishes all four zero pairs: `(y,x)` of `(+0,+0)`, `(-0,+0)`, `(+0,-0)`, and
`(-0,-0)` returns `+0`, `-0`, `+pi`, and `-pi` respectively. Serialization keeps NX's existing
zero representation and does not promise a signed-zero round trip.

Unlike JavaScript, math operations diagnose invalid numerical values rather than returning
NaN or infinity:

| Diagnostic | Cause |
| --- | --- |
| `nx-math-non-finite-input` | Any argument is NaN or infinity |
| `nx-math-domain` | Invalid domain or reversed clamp bounds |
| `nx-math-overflow` | The final result is not finite |

Errors name the operation and include an authored call location when available. These rules
apply to the math library; they do not change general arithmetic.

## Runtimes and compatibility

Rust and TypeScript implement NX runtime evaluation. .NET calls the Rust library through the
native SDK binding; it has no independent C# evaluator. Executable TypeScript and JavaScript
use the same numerical contract. Existing unsupported source-generation constructs, such as
certain indirect function calls, remain explicitly unsupported.

Compiled math library images require `standard-math-v1`. Hosts must supply the emitted math
library image when linking an importer, as with other standard libraries. Older runtimes reject
that feature before evaluation. Upgrade compiler and runtime packages together.

For executable JavaScript in `program-module` format, supply `nx:runtime` with the helper bundle
from the same compiler release. Math requires its `nxStandardMath` export; an older bundle can
lack that export even when its `NX_JS_RUNTIME_ABI` matches. The ABI names the calling contract,
not a minimum helper release. Generated `files` output includes its matching runtime helper.

Exact sign/comparison/rounding/endpoint cases are tested exactly, including zero signs. Inexact
reference cases use absolute error at most `1e-12 * max(1, abs(reference))`. This is a conformance
criterion, not a promise of bit-identical transcendental results across hosts or universal correct
rounding. Near-pole tangent results may be large finite values: a floating approximation of π/2
is not treated as an exact mathematical pole.
