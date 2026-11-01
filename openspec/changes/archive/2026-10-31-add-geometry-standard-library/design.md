## Context

See proposal.md for motivation. The compiler embeds standard-library NX source in
`crates/nx-api/src/artifacts.rs` and supplies ordinary, content-versioned library images.
Graphics already aliases the canonical color type, which provides a model for sharing geometry.

`graphics.nx` currently declares `Point` and `ViewBox` records, a drawable `Rect` external
component, and a graphics-specific `Transform` union. Polygon and helper output use `Point`;
Drawing requires `ViewBox`. nx-api indexes linked construction identities and generic IR emission preserves them for neutral scene
preparation. nx-graphics uses kurbo internally; this implementation detail is independent of
public NX record names. Native Skia and browser Canvas consume the same prepared scene.

The branch incorporates main at `2d3a639`, including the merged value-constraint support.
Numeric constraints are interval patterns, reject NaN at every constrained boundary, and retain
ordinary base-value encodings. `Interval::contains` explicitly rejects NaN. Runtime binding
checks already report `nx-ir-constraint`; static failures report `constraint-violation`.

## Goals / Non-Goals

**Goals:**

- Establish canonical, renderer-independent geometry values with ordinary NX import semantics.
- Preserve graphics appearance, validation, transforms, and numerical precision through migration.
- Make the distinction between a rectangle value and a drawable rectangle explicit.
- Keep dependencies one-way: graphics depends on geometry; geometry does not depend on graphics.

**Non-Goals:**

- Vector arithmetic, rectangle operations, numerical generics, integer geometry, or a new runtime
  geometry capability. These can be added with concrete consumers and dedicated contracts.
- Shared affine transforms, paths, Geometry unions, shape combination, hit-testing engines,
  clipping API changes, shaders, or relative gradient coordinates.
- Changes to graphics coordinate direction, SVG-style scalar shape properties, Transform lists,
  renderer internals, or the legacy HIR evaluator's supported graphics profile.

## Decisions

### 1. Three ordinary float64 records in @nx/geometry

Add `crates/nx-api/src/std/geometry/geometry.nx` as the root module, with complete declaration
and property doc comments:

```nx
export type Point = {
  x: float64 where finite
  y: float64 where finite
}

export type Size = {
  width: float64 where >= 0.0
  height: float64 where >= 0.0
}

export type Rect = {
  x: float64 where finite
  y: float64 where finite
  width: float64 where finite && >= 0.0
  height: float64 where finite && >= 0.0
}
```

All fields are required. There are no implicit origins or sizes. Values describe axis-aligned
geometry in the coordinate system selected by the consumer. Geometry itself does not require
pixels, a viewport, an axis direction, or a rendering backend. `x` and `y` locate the rectangle's
origin; documentation uses origin rather than universally calling it the top-left corner.

Point and Rect have finite coordinates. Rect dimensions must also be finite and nonnegative.
Size dimensions must be nonnegative but may be positive infinity, making Size useful for
unbounded available layout space. Every constrained field rejects NaN under the existing
constraint contract. Both signs of zero are accepted; there is no new zero canonicalization.
A zero-area rectangle is a valid value, not a missing-value sentinel. A future
rectangle-from-points operation can normalize arbitrarily ordered endpoints explicitly.

Use the existing checks at binding sites: constants can fail during analysis and dynamic
values fail with `nx-ir-constraint` during supported IR evaluation. Arithmetic still produces
an unconstrained base type; a constrained input does not prove its computed output is valid.
The legacy interpreter retains its existing compile-time-only constraint guarantee.

Geometry prescribes no application-specific coordinate maximum and no cross-field constraint
that x+width or y+height must be finite. Future edge/transform operations must define their
own overflow behavior. Drawing.viewBox remains a shared Rect with the stronger positive-dimension
rule enforced during preparation: current constraints apply to scalar types, not predicates
over a record's fields. Graphics also retains defensive validation of raw SDK values and
backend limits, even when source constraints ordinarily protect construction.

float64 preserves precision for application calculations and matches current public graphics
coordinates. Backend float32 conversions and rejection rules remain renderer responsibilities.
Generic scalar parameters or paired float32 records would add API surface without a present need.

### 2. Add a finite numeric pattern term using the existing interval model

Extend the pattern vocabulary with `finite`, accepted for float32/float64 and aliases of those
types, in both `where` clauses and numeric `is` arms. Examples:

```nx
type Coordinate = float64 where finite

type Extent = float64 where finite && >= 0.0

let classify(x: float64): string = {
  if x is {
    finite => "finite"
    else => "non-finite"
  }
}
```

`finite` excludes both infinities and every NaN; it includes zero, negative zero, subnormal
values, and both largest finite values. Treat it as a contextual pattern term so declarations
and fields named finite remain legal. On integer and nonnumeric bases report invalid-constraint
rather than introducing an integer no-op refinement. Existing unsupported call/operand/OR
forms remain unsupported; this does not add arbitrary expression predicates.

Lower finite to inclusive [-MAX, MAX] using the selected float base's actual maximum finite
value (float32::MAX or float64::MAX). Intersect with other terms and alias constraints using
the existing interval model. Thus finite and the equivalent explicitly bounded interval are
the same structural type, existing subset/widening rules apply, and no interval encoding,
runtime opcode, or new math capability is necessary. An unconstrained float32 can still hold
infinity, so widening alone cannot prove it satisfies finite float64.

Preserve the authored finite spelling in hover and source-tree/editor presentation. Normalized
IR and JSON Schema use ordinary finite numeric endpoints; runtime diagnostics may use the
existing normalized-interval spelling. Verify boundary round trips and schema agreement rather
than introducing a new schema extension. Existing integer-only visualizer bounds/step behavior
is unchanged; these large float bounds do not introduce a new slider policy.

Do not add notNaN in this change. Size already rejects NaN with >= 0.0. No initial geometry field
needs an unrestricted signed float accepting infinities while rejecting only NaN. A future
standalone notNaN term could map to Interval::ALL, whose contains already excludes NaN, but
there is no present consumer justifying that additional language surface.

Alternatives considered: spelling platform maximum literals in each geometry field avoids
syntax work but is opaque and precision-dependent; arbitrary isFinite calls require a larger
predicate language and possible runtime dependencies; banning infinity in Size excludes a useful
layout convention. A single finite term keeps the current interval architecture intact.

### 3. Re-export canonical values from graphics

Graphics imports geometry under a descriptive alias and exposes ordinary type aliases:

```nx
import "@nx/geometry" as geometry

export type Point = geometry.Point

export type Size = geometry.Size

export type Rect = geometry.Rect
```

Direct and re-exported values have the same declaration identity, owned by
`@nx/geometry/geometry.nx`. Do not introduce wrapper records or duplicate nominal declarations.
Aliased imports and mixed direct/re-export imports follow existing duplicate/ambiguity rules.

Keep point-valued properties such as Polygon.points typed with the shared Point. Change
Drawing.viewBox to shared Rect and remove ViewBox. Positive width and height are properties of
the Drawing consumer, not of all Rect values. Do not replace scalar shape/gradient coordinates
with nested Point/Rect properties in this change.

Keeping a ViewBox alias was considered, but provides a second name without new value semantics.
The `viewBox` property and its documentation retain the viewport concept.

### 4. Rectangle names the drawable element

Rename the graphics external component and its visualizer to Rectangle. Keep all existing
properties, defaults, rounded-corner rules, inheritance, fill/stroke behavior, and icons.
`Rect` is exclusively the shared value; it is not a deprecated component alias. A Rect used
where a graphical child is expected must not become a drawable rectangle by name or shape.

The alternative of preserving the drawable Rect and naming the value Rectangle reverses the
useful distinction chosen in discussion. Separate namespaces alone would also force consumers
to disambiguate two common Rect declarations unnecessarily.

### 5. Migrate nominal identities and revise the graphics profile together

Update nx-api's construction-declaration index and exercise nx-codegen's generic IR emission so Point and Rect resolve to geometry and
Rectangle resolves to graphics. Audit generic nested values and same-named user declarations;
short display names alone must never authorize a graphics primitive. Shared geometry records
are data, not renderable external components.

Use `graphics-core-v4` for preparation metadata, profile checks, fixtures, documentation, and
preview manifests. All current paint and rendering semantics carry forward. Keep explicit
unsupported-profile rejection. Do not silently support old Rect component images or add a
legacy v3 path. This revision reflects changed public nominal identities and helps diagnose
mixed compiler/renderer artifacts.

Private kurbo Rect/Point types and unrelated rectangle classes stay unchanged. The migration is
semantic, not a repository-wide textual substitution. Historical archived changes stay intact.

### 6. Use standard-library infrastructure without a new runtime feature

Register geometry as an unstable source-only library with package/namespace metadata following
other entries (`@nx-lang/geometry`, `NxLang.Geometry`). Use normal module resolution, image
emission, versioning, and linking. Graphics-only consumers obtain the geometry dependency
transitively; direct geometry consumers do not acquire math, graphics, fonts, or a renderer.
No geometry intrinsic, host callback, IR opcode, serialization format, or external dependency
is needed. Test the supported Rust and TypeScript IR paths and existing schema/SDK surfaces.

### 7. Annotate graphics scalar contracts at their source

Apply constraints to public numeric declarations throughout graphics.nx, including union-case
fields, external-component props, and exported helper parameters. Use inline constraints for
readability, and a private constrained item alias for dashArray because the current constraint
syntax cannot attach a constraint directly to a sequence. Do not add new public numeric aliases
or change existing scalar types merely to express constraints.

| Fields / parameters | Constraint |
| --- | --- |
| Coordinates, translations, matrix coefficients, rotations/skews, scale factors, dashOffset | float64 where finite |
| GradientStop.offset; opacity on Drawing, GraphicsElement, and every helper | float64 where in 0.0..=1.0 |
| Shape widths/heights/radii, stroke width, corner radii, helper radii and arrow dimensions, dash items | float64 where finite && >= 0.0 |
| Paint.radialGradient.radius; Text.fontSize | float64 where finite && > 0.0 |
| Stroke.miterLimit | float64 where finite && >= 1.0 |
| Text.fontWeight | float64 where in 1.0..=1000.0 |
| RegularPolygon/Star.pointCount | int where in 3..=4096 |
| Arc/RingSector.sweepAngle | float64 where in -360.0..=360.0 |

Closed finite intervals already exclude infinities and NaN; they do not need an additional
finite term. Zero and negative scales remain legal, and angles are not restricted to one turn
except the existing sweepAngle contract. Keep fontWeight's current float64 representation:
the interval enforces its range, while preparation retains the whole-number requirement.
Changing it to int would be an additional public API change unnecessary for this work.

Preserve defaults and optionality, including optional ry and Transform.scale.y. A dash item alias
can be written `type DashLength = float64 where finite && >= 0.0`, then used as
`dashArray?: DashLength+`. Each present item is checked; omission and the empty sequence remain
valid under existing occurrence semantics.

Keep scene preparation as the authoritative defensive boundary for arbitrary host values.
Retain all relationships and resource checks that scalar intervals cannot express: distinct
linear endpoints, stop ordering/count, point counts in sequences, innerRadius <= outerRadius,
arrow width/length relationships, noncoincident endpoints, positive Drawing.viewBox dimensions,
integer fontWeight, computed-coordinate overflow, backend gradient precision thresholds, path
syntax, and configured budgets. Finite scalar fields do not imply a usable transform or a
finite computed result. Do not forbid singular matrices or zero scales globally.

Public helper constraints will report invalid scalar arguments as constraint-violation or
nx-ir-constraint earlier than existing helper/scene diagnostics. Update tests and documentation
to expect this intentional change in failure timing. Keep cross-field checks and their existing
diagnostic context. Function bodies trust parameter constraints: remove duplicate scalar checks
and unreachable error branches, including in private helpers called only with those validated
parameters. Retain defenses protecting raw SDK scenes and checks on calculated intermediate
values. This source migration introduces no geometry host callback or math intrinsic.

## Risks / Trade-offs

- [Rect changes from drawable to value] → Migrate active sources and generated examples together;
  add a negative test that Rect is not accepted as a graphics child and document the rename.
- [Type aliases lose identity in adapters or generated bindings] → Test both direct geometry
  imports and graphics re-exports, nested Point sequences, Rect viewports, and user-name collisions.
- [Source constraints mistaken for protection of arbitrary host values] → Document constrained
  binding sites and retain raw-value validation tests for graphics consumers.
- [NaN or infinity accidentally accepted by finite] → Test both precisions, both infinities,
  NaN, signed zeros, subnormals, maximum finite values, narrowing, aliases, and runtime parity.
- [Size infinity reaches a renderer] → Document Size as potentially unbounded and require finite
  dimensions in APIs that allocate or render concrete surfaces.
- [A valid zero Rect is rejected everywhere] → Test standalone zero values separately from a
  Drawing's intentionally stricter positive viewBox dimensions.
- [Public float64 mistaken for float64 backend rendering] → Document conversion boundaries and
  retain existing backend-range tests.
- [Profile/source version mismatch] → Migrate profile gates and emitted images together, then
  exercise native preview and browser transport end to end.

## Migration Plan

1. Add finite pattern syntax/lowering and conformance tests using the merged constraint infrastructure.
2. Add constrained geometry declarations, registration, and standalone analysis/IR tests.
3. Update graphics aliases, viewport type, Rectangle component, identity adapters, and profile.
4. Migrate active source fixtures, preview/playground examples, and documentation; leave internal
   renderer geometry types and historical archives alone.
5. Verify library identity/linking and graphics regression suites in both supported runtimes,
   then run native/browser preview checks and strict OpenSpec validation.

This is one coordinated unstable-API update with no compatibility shim. If it must be rolled
back, revert the library/graphics integration and associated profile/sample changes together.
