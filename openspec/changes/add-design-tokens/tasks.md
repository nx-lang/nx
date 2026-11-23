## 1. The library

- [ ] 1.1 Move `api/types.nx`, `api/palette.nx` and `api/tokens.nx` to `crates/nx-program/src/std/design/` as the `@nx/design` standard library, with an `unstable` source header, and register it in `crates/nx-program/src/artifacts.rs` beside `@nx/color`. Remove the `api/` drafts from the change.
- [ ] 1.2 Keep the Tailwind CSS v4 OKLCH values and MIT notice from `api/palette.nx`. Add a test that every palette color passes `inGamut(…, displayP3)`, that the grays pass `inGamut(…, srgb)`, and that `toCss` of each color parses back to the same color.
- [ ] 1.3 Add a test that the library is free of diagnostics and analyzed only when imported, and tests for the value types' constraints and alias transparency.
- [ ] 1.4 Add a test that every scale is nondecreasing, and that the default theme meets every contrast target of the spec in both color modes: each `on…` role 4.5 against its role, `text.normal` and `text.muted` 4.5 against `background` and `surface`, and `focus` 3 against both.

## 2. Conformance

- [ ] 2.1 Turn `examples/` into a conformance corpus program, `specs/ir-conformance/design-tokens/`, with entrypoints for each scenario: roles in each mode, a text role following `env.font`, a component token following the core, a theme tuning one library, a theme varying by mode, the contrast targets in each mode, and the palette's gamut. Record results and operation counts for both runtimes.
- [ ] 2.2 Check that generated JavaScript gives the same results, or record what it lacks in `specs/future.md`.

## 3. Documentation

- [ ] 3.1 Add `sites/website/src/content/docs/reference/libraries/design.md`: every type and token with its default, the palette's step roles (50–100 backgrounds, 200–300 borders, 500–600 fills, 400–950 text by mode) and how its wide colors look on sRGB and P3 screens, the contrast targets and that `border` has none, which values may need per-platform overrides (units, font families, shadows), what other systems call `accent` and a table mapping shadcn/ui's color names to ours, that a theme changing `accent` must set `focus` too, the extension rules, themes as bundles, and the stability contract.
- [ ] 3.2 Extend the style guide's token section with the naming table and the extension rules.
- [ ] 3.3 Rewrite `tutorials/working-with-design-tokens.md` around `@nx/design`: read tokens, build a component with its own tokens, write a light and dark brand theme, and apply it app-wide and to one section.
- [ ] 3.4 Check every new code block with the website's code-block check.

## 4. Completion

- [ ] 4.1 Run the Rust and TypeScript test suites, `cargo fmt` and `cargo clippy`, then the review loop, then validate strictly, sync and archive.
