## 1. The library

- [ ] 1.1 Replace `Color` in `crates/nx-program/src/std/color/color.nx` with the union in `api/color.nx` (`Unit`, `Hue`, `Gamut`, the four cases, `alpha`), keeping the named colors as hex literals. Remove `api/color.nx` from the change.
- [ ] 1.2 Update `crates/nx-program/src/std/color/operations.nx` to `api/operations.nx`: `displayP3`, `inGamut`, `toGamut`, `alpha` parameters, and the doc comments that say which functions map into sRGB. Remove `api/operations.nx`.
- [ ] 1.3 Lower hex literals to `Color.srgb` constants, and recognize the four cases wherever the compiler identifies the canonical Color (value printing in `nx-api`, `nx-hir` lowering).

## 2. Runtimes

- [ ] 2.1 Rust (`crates/nx-value/src/color.rs`): read and build each case; convert through linear-light XYZ D65 with unclamped intermediates; implement CSS Color 4 gamut mapping for `toGamut` and every function that maps; `displayP3`; `oklab`/`oklch` without clipping; `color(display-p3 …)` parsing; `toCss` per space; OKLCH transformations; result spaces for `mix` and `composite`; contrast on the sRGB-mapped color.
- [ ] 2.2 TypeScript (`runtime/typescript/src/color.ts`): the same.
- [ ] 2.3 Rename the runtime feature to `standard-color-v2` in codegen, both runtimes and their feature checks.
- [ ] 2.4 Tests on both runtimes for every scenario, including agreement within 2e-6 on mapped colors (P3 red into sRGB, an Oklch color outside P3, and lightness at 0 and 1).

## 3. Graphics and transport

- [ ] 3.1 `nx-graphics` preparation: validate each case and map paints and gradient stops into sRGB; bump the profile to `graphics-core-v5` in metadata, previews, fixtures and docs.
- [ ] 3.2 Move graphics, transport, WASM and SDK tests and fixtures to the union-case shape (`"$type": "Color.srgb"`, `alpha`).

## 4. Tooling

- [ ] 4.1 `nx-language-service`: swatch discovery for `displayP3`, `oklab`, `oklch`, the case records and the new `parse` forms; diagnostics for their literal channels.
- [ ] 4.2 Viewer, value view and playground: paint swatches from `toCss` text, add the outside-sRGB mark with accessible text, and make the picker read-only for wide colors.

## 5. Documentation

- [ ] 5.1 Update `reference/libraries/color.md`: the four spaces, the gamut functions, the table of what maps into sRGB, the OKLCH transformations, equality per space, and examples.
- [ ] 5.2 Update `reference/libraries/graphics.md` for mapping at preparation and `graphics-core-v5`.
- [ ] 5.3 In `specs/future.md`, drop the pointer to this change from "Wide-Gamut Color" and keep only the follow-ups it lists.
- [ ] 5.4 Check every new code block with the website's code-block check.

## 6. Completion

- [ ] 6.1 Run the Rust and TypeScript test suites, `cargo fmt` and `cargo clippy`, then the review loop, then validate strictly, sync and archive.
