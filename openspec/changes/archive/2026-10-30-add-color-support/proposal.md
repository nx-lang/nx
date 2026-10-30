## Why

Colors are shared application data: ReachMe chat links and surveys need them even when they do not render graphics. NX currently duplicates string aliases and leaves parsing to hosts; a standard color contract would provide consistent values, validation, and editor tooling before gradients introduce more consumers.

## What Changes

- Add an unstable, renderer-independent `@nx/color` standard library exporting one canonical sRGB-plus-alpha `Color` record with float32 components and validated construction, parsing, formatting, transformation, mixing, and accessibility functions.
- Add native hex literals (`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`) that infer the canonical Color without imports, including bare element properties. Keep recoverable parsing for external input and explicit CSS conversion.
- Include CSS-oriented RGB/HSL/HWB and JavaScript-familiar HSV construction/conversion, named colors, perceptual Oklab/Oklch helpers, alpha editing, lightness/saturation/hue transformations, mixing, compositing, and luminance/contrast helpers.
- Add type-aware color swatches, literal diagnostics, and a picker that edits authored colors without overwriting computed expressions.
- **BREAKING**: Replace graphics' private string `Color` with the shared record, migrate graphics fixtures and examples, and version the changed graphics profile.
- Migrate the checked-in survey example to demonstrate color use without importing graphics. External ReachMe services are outside this repository's migration scope.
- Lower color literals to ordinary canonical records; add no runtime primitive, implicit string conversion, or general refinement-type mechanism.

## Capabilities

### New Capabilities

- `color-library`: Float32 values, parsing, conversions, transformations, mixing, compositing, contrast, and runtime availability without graphics.
- `color-literals`: Hex expression syntax, canonical type inference, import-free resolution, lowering, diagnostics, and source round trips.
- `color-tooling`: Type-aware color discovery, diagnostics, swatches, and source-preserving color edits.

### Modified Capabilities

- `graphics-library`: Consume and re-export the canonical color contract and load its transitive library dependency.

## Impact

Grammar/parser/AST, type inference, canonical literal lowering, syntax highlighting/formatting, standard-library registration and module linking in `nx-api`; Rust and TypeScript IR runtime standard-library functions; shared Rust color parsing used by neutral graphics preparation; WASM and SDK value/schema transport; language-service color discovery and source edits; playground Visual/Source views; graphics Skia/Canvas adapters; survey and graphics examples, fixtures, schemas, and documentation.

No Skia, Canvas, UI, or graphics dependency is introduced for color-only consumers. Existing string visualizers remain supported. Gradients, retained wide-gamut/HDR values, contextual theme colors, and automatic migration of external services are follow-ups.
