## Why

NX graphics already supports geometry, text, transforms and clipping, but solid-only paint limits illustrations, chart fills, backgrounds and icons. The shared `@nx/color` contract now provides the foundation for gradients that behave consistently in native exports and browser previews.

## What Changes

- Rename the uniform paint case to Paint.color and introduce Paint.linearGradient and Paint.radialGradient with linear and centered radial gradients using local coordinates, explicit ordered color stops, transparency, endpoint clamping and one specified interpolation contract.
- Support gradients everywhere Paint is accepted: geometric fills, strokes, text fills and Drawing backgrounds, including existing parametric helpers.
- Validate complete gradient data before painting, including raw SDK values and resource limits; retain actionable element/property diagnostics.
- Carry backend-neutral prepared paints through Skia, WASM/SDK transport and Canvas without changing generic NX values or IR opcodes.
- Extend inspection, geometry-based selection, reusable-paint examples and deterministic native/browser fixtures.
- **BREAKING**: Advance the unstable graphics profile to `graphics-core-v3` and replace color-only prepared paint/destination contracts. Migrate internal consumers together without compatibility shims.
- Exclude shadows, images, multiline text, repeat/reflect, bounding-box coordinates, focal/elliptical radial controls, sweep/mesh gradients, additional interpolation spaces and a dedicated gradient editor.

## Capabilities

### New Capabilities

None; gradients extend the existing graphics capabilities.

### Modified Capabilities

- `graphics-library`: Gradient authoring contracts, supported-content scope and profile version.
- `graphics-rendering`: Neutral gradient validation/preparation, interpolation and direct adapter painting.
- `graphics-preview`: Paint inspection, selection, bounded preview behavior and cross-renderer acceptance fixtures.

## Impact

`crates/nx-api/src/std/graphics/graphics.nx`, generated library images and compiler/runtime graphics tests; `crates/nx-graphics` and `crates/nx-graphics-skia`; WASM native serialization and TypeScript graphics bindings; playground Canvas, inspection and hit testing; preview schemas/configuration, examples, fixtures and graphics reference documentation. Reuse canonical Color validation and existing IR evaluation; introduce no Skia dependency into neutral graphics and no CanvasKit, network assets or new color-library operations.
