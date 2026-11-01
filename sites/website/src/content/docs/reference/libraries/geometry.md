---
title: '@nx/geometry'
description: 'Shared positions, sizes, and rectangle values independent of graphics.'
---

`@nx/geometry` is an unstable, compiler-owned standard library. It exports three ordinary records:
`Point`, `Size`, and `Rect`. Every field is required and uses `float64`.

```nx
import "@nx/geometry"

type Selection = {
  anchor: Point
  available: Size
  region: Rect
}

let root(): Selection =
  <Selection
    anchor=<Point x=12.5 y=-4 />
    available=<Size width=800 height=600 />
    region=<Rect x=0 y=0 width=120 height=80 />
  />
```

These values work for layout, pointer positions, document regions, and application calculations.
They require no graphics library, renderer, or host callback. The consumer determines the units,
origin, and axis directions; coordinates need not be pixels or increase downward.

| Record | Fields | Constraints |
| --- | --- | --- |
| `Point` | `x`, `y` | Finite coordinates |
| `Size` | `width`, `height` | Nonnegative; positive infinity is allowed for unbounded available space |
| `Rect` | `x`, `y`, `width`, `height` | Finite coordinates and finite, nonnegative dimensions |

All fields reject NaN. Both signs of zero are accepted. Zero dimensions describe valid geometry;
they do not mean absence. A rectangle's `x` and `y` locate its origin, which the consumer may
interpret as its top-left corner. Computing an edge such as `x + width` may overflow even when
each field is finite; operations must handle their own result domains.

Constraints are checked at ordinary NX binding sites. Statically known invalid inputs report
`constraint-violation`; dynamic violations in Rust and TypeScript IR evaluation report
`nx-ir-constraint`. Arithmetic returns the unconstrained numeric base type and may need another
check when assigned to a constrained field. The legacy HIR evaluator retains its existing
compile-time-only constraint guarantee.

Graphics re-exports these same types. Use `Rect` for rectangle data and a drawing's `viewBox`;
use `Rectangle` for a drawable shape. A drawing requires strictly positive viewport dimensions,
even though zero dimensions are valid for a standalone `Rect`. Host rendering limits still apply.
The initial geometry library does not provide arithmetic operations, vectors, transforms, or paths.
