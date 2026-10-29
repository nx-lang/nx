# Delivered math baseline for graphics

`@nx/math` and source-authored external function declarations were delivered and pushed in
`e46b846e1fda66469e5abc16aeb62cb2433203d3` on 2026-10-28. The corresponding changes are archived:

- [Math library](../2026-10-28-add-math-standard-library/proposal.md), with
  [validation](../2026-10-28-add-math-standard-library/validation.md).
- [External declarations](../2026-10-28-add-external-function-declarations/proposal.md), with
  [review/verification](../2026-10-28-add-external-function-declarations/review.md).

The public contracts are the main [math spec](../../../specs/math-library/spec.md),
[runtime standard-functions spec](../../../specs/runtime-standard-functions/spec.md),
[math reference](../../../../sites/website/src/content/docs/reference/libraries/math.md), and
[compiled NX source](../../../../crates/nx-api/src/std/math/math.nx).

## Available capabilities

Math already provides float64 `pi`/`tau` and 22 functions. Graphics helpers need no further numerical
language additions for this scope:

| Graphics use | Existing operations | Integration convention |
| --- | --- | --- |
| Polygon/star vertices, arc/sector endpoints | `sin`, `cos`, `degreesToRadians` | Public angles are clockwise degrees; convert once. Positive sine increases y in the y-down graphics coordinate system. |
| Arrow direction and distances | `hypot`, optionally `atan2` | Prefer hypot to intermediate squaring. atan2 takes y,x and returns radians; reject coincident endpoints separately before normalization. |
| Dimensions and interpolation | `abs`, `min`, `max`, `clamp`, `lerp` | Keep explicit helper constraints; do not clamp an invalid authored value silently. |
| Rounding and conversions when required | `floor`, `ceil`, `round`, `roundTiesToEven`, `trunc`, `radiansToDegrees` | round follows JavaScript ties toward positive infinity; ties-to-even is explicit. |

NX already widens `int` to float64 exactly. Compute angular steps with floating-point operands
before division, such as `360.0 * index / count`; integer division first would truncate. Keep any
periodic reduction of rotation/start angles in degree space before converting to radians, and
retain the original signed sweep so a full 360-degree ring is not mistaken for an empty sweep.
This uses existing NX arithmetic, not a new math primitive.

All math arguments must be finite. The delivered error categories are
`nx-math-non-finite-input`, `nx-math-domain`, and `nx-math-overflow`; preserve the operation and
available authored call origin when returning preview diagnostics. General arithmetic is unchanged:
finite inputs to graphics can still overflow during subtraction, multiplication, or backend
conversion. Math's numerical validation does not replace graphics validation.

Math accepts signed-zero atan2 coordinates, including both-zero inputs. That does not make a
zero-length Arrow valid. Math also does not establish valid vertex counts, radius relationships,
sweep ranges, complexity limits or backend scalar ranges.

## Remaining graphics integration

1. Import `@nx/math` from the production graphics NX source using ordinary imports. Graphics
   helpers use authored `let` or component bodies. Keep graphics module runtime-function lists
   empty; the existing math library owns the numerical implementations.
2. Test compilation through the standard-library registry with a consumer importing only graphics.
   The graphics catalog does not yet exist, so this is still task 1.1, not an already-proven graphics
   integration. Do not require callers to enable or install math separately.
3. Emit and link the transitive `@nx/math/math.nx` image at its compiler-produced content version.
   Its bodies declare `standard-math-v1`; caller/graphics modules retain ordinary call/reference
   encodings and their own required features. Missing images or unsupported features fail before
   painting. Use the existing program fingerprint, which incorporates library content versions,
   in preview revision identity.
4. Keep evaluation failures separate from successful frame publication. Do not replace failed
   numerical results with zero coordinates. No Skia canvas operation or successful PNG publication
   follows a failed evaluation.
5. Add full RegularPolygon/Star/Arrow/Arc/RingSector fixtures and helper-specific invalid-input
   checks. Reuse [the renderer-free geometry example](../../../../examples/math-graphics/main.nx) as a
   numerical smoke test, not as an already-completed implementation of those graphics helpers.

The Rust and TypeScript evaluators, and executable TS/JS generation, already implement the same
math contract. .NET uses the Rust binding. The neutral renderer/Skia adapter receives evaluated
geometry and needs no math-image loader or NX interpreter to paint it; its native geometry work
may use native numerical APIs with the specified range/finite checks.

Math references use `abs(error) <= 1e-12 * max(1, abs(reference))` for inexact cases, with exact
checks for specified endpoints/rounding/zero behavior. Keep this separate from graphics' `1e-5`
construction/conversion tolerance; the math tolerance is not a guarantee of bit-identical
transcendental results or permission to compare only two runtimes without independent references.

## Readiness verification

Rechecked on 2026-10-28 at `e46b846`:

```sh
cargo test -q -p nx-api math_standard_library_signatures_identity_and_imports
cargo test -q -p nx-codegen standard_math
node runtime/typescript/test/math.test.mjs
```

Results: the library signature/import test passes; all 11 focused Rust math tests pass, including
Rust IR/legacy evaluation, independent references, function values, origins, feature gates,
malformed images, budgets and renderer-free geometry; all 184 reference cases pass in TypeScript
IR and generated TS/JS, with geometry checks in the same runner. These checks require no Skia.

This completes the delivered-dependency readiness task only. Graphics source imports, helpers,
scene preparation, native rendering and preview remain open implementation tasks. Recheck these
contracts when changing the baseline or starting graphics implementation against a later release.
