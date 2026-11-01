## Why

NX needs reusable geometry values for graphics, layout, input handling, and document tooling without requiring a graphics dependency. Graphics currently owns its own point and viewport rectangle types, and its drawable `Rect` occupies the conventional name for a rectangle value.

## What Changes

- Add an unstable, compiler-owned `@nx/geometry` standard library with documented `Point`, `Size`, and `Rect` records using `float64` fields.
- Enforce geometry domains using value constraints: Point coordinates are finite; Rect coordinates and dimensions are finite with nonnegative dimensions; Size dimensions are nonnegative and may be positive infinity. Zero dimensions remain valid and NaN is rejected.
- Extend numeric value patterns with a floating-point `finite` term, composable with existing interval terms. Lower it to the existing interval representation; no separate not-NaN term is needed because every existing constraint already rejects NaN.
- **BREAKING**: Move the canonical `Point` declaration to geometry. Graphics re-exports it through a type alias, following its existing shared-color pattern.
- **BREAKING**: Rename the graphics drawable `Rect` to `Rectangle`; export the shared geometry `Rect` from graphics instead. No compatibility component alias is retained.
- **BREAKING**: Replace graphics `ViewBox` with the shared `Rect` as the type of `Drawing.viewBox`; remove the separate `ViewBox` export while retaining positive viewport dimensions.
- Annotate public numeric fields and helper parameters in graphics.nx with scalar constraints for finiteness, opacity/stop ranges, nonnegative lengths, positive gradient/text dimensions, and helper count/sweep limits; retain relational and backend validation.
- Migrate declaration resolution, preparation, preview integration, examples, documentation, and fixtures to the canonical geometry identities and `Rectangle` name.
- Publish the changed graphics contract as `graphics-core-v4` with coordinated native/browser support.
- Keep the first library deliberately small. Geometry operations, vectors, shared affine transforms, structured paths, clipping geometry, and Boolean operations are future extensions. Existing graphics transforms, path strings, and local-unit gradient semantics remain unchanged.

## Capabilities

### New Capabilities

- `geometry-library`: Shared constrained geometry records, numeric and coordinate semantics, ordinary imports, canonical identities, and cross-runtime evaluation.

### Modified Capabilities

- `value-constraints`: Add the `finite` floating-point pattern term with existing constraint checks and numeric matching semantics.

- `graphics-library`: Shared geometry consumption, rectangle element naming, viewport type migration, and profile revision.
- `graphics-rendering`: Validate the new canonical declaration identities and profile without changing drawing semantics.
- `graphics-preview`: Preview and playground support for the migrated contract and examples.

## Impact

- Add compiler-owned geometry source and register it in `crates/nx-api/src/artifacts.rs`, including standard-library dependency discovery and content versioning.
- Update `crates/nx-api/src/std/graphics/graphics.nx`, graphics identity adapters in nx-codegen, neutral scene preparation in nx-graphics, and profile consumers in native and browser rendering.
- Exercise Rust and TypeScript IR linking/evaluation, SDK/schema/type generation, and editor declaration presentation through existing standard-library mechanisms; no new IR encoding or runtime intrinsic is required.
- Update active NX fixtures, playground samples, and website references. Historical archived OpenSpec changes remain historical records.
- Extend syntax/lowering, constraint normalization, diagnostics, source presentation, and conformance coverage for `finite`; use existing interval IR, schema export, and runtime enforcement. General predicate calls and standalone not-NaN constraints are out of scope.
- No compatibility layer, external geometry package, or renderer dependency is introduced.
