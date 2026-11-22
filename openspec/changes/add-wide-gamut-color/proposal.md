## Why

`Color` in `@nx/color` is encoded sRGB, with each channel limited to [0, 1]. sRGB is the smallest
gamut in common use. Most phones and Macs, and many laptops, show Display P3, about 25% more colors,
mostly saturated reds, oranges and greens. Modern color work is moving to OKLCH, which can describe
colors outside both. Tailwind CSS v4's palette is OKLCH, and part of it is outside sRGB.

Today NX can't hold those colors. `oklch(…)` clips each sRGB channel after converting, which shifts
both hue and lightness, and `color(display-p3 …)` text is rejected. This is why `@nx/design`
(`add-design-tokens`) uses Tailwind v3's sRGB palette. Brand colors defined in P3 or OKLCH can't be
written exactly.

Modern systems agree on the model: a color is its color space plus float channels. CSS Color 4 and
5, DTCG 2025.10 (the format `add-token-export` will write), Apple's CGColor, Android's 64-bit color,
Flutter since 3.27 and Skia all work this way. Older APIs that are sRGB only, such as WinUI's
`Color`, are what they moved away from. The risk with wide gamut is misuse: reading P3 numbers as
sRGB ones, or measuring contrast on a color the screen can't show. The design here makes the safe
path the only path.

## What Changes

- **BREAKING: `Color` becomes a union of color spaces:** `Color.srgb { r g b alpha }`,
  `Color.displayP3 { r g b alpha }`, `Color.oklab { l a b alpha }` and `Color.oklch { l c h alpha }`.
  Channel ranges are value constraints on the fields, so even a directly written record is checked.
  Code reads channels by matching on the space, or by converting first. Nothing reads `.r` without
  naming the space it belongs to. Alpha is named `alpha` in every space, replacing `a`, because
  Oklab already has a channel named `a`.
- **Hex literals and the familiar constructors stay sRGB.** `#2563eb`, `rgb`, `hsl`, `hwb`, `srgb`
  and the named colors give `Color.srgb`, so most code changes only where it reads channels. The
  grammar doesn't change.
- **New constructors:**
  - `displayP3(r, g, b, alpha)` gives `Color.displayP3`.
  - `oklab(…)` and `oklch(…)` now keep the color they're given, in their own space, instead of
    clipping it to sRGB.
  - `parse` accepts `color(display-p3 …)`, and returns `oklab(…)` and `oklch(…)` text in their own
    spaces.
- **Explicit gamut functions:**
  - `inGamut(c, gamut)` says whether a screen with that gamut can show the color.
  - `toGamut(c, gamut)` converts the color into sRGB or P3, mapping an out-of-gamut color with the
    CSS Color 4 algorithm, which keeps lightness and hue and reduces chroma.
- **Every function accepts any space, with defined results:**
  - Functions that need sRGB map the color into sRGB first, as an sRGB screen would. These are
    `toHex`, `toRgb` and the other sRGB channel records, and the WCAG luminance and contrast
    functions.
  - `toCss` writes the color in its own space.
  - Transformations return a color in their input's space.
  - `mix` and `composite` return the wider of their inputs' spaces.
- **BREAKING: `lighten`, `darken`, `saturate`, `desaturate`, `rotateHue`, `complement` and
  `grayscale` work in OKLCH instead of HSL.** HSL exists only inside sRGB, so keeping it would push
  every wide color back into sRGB. OKLCH also gives the even steps these functions are meant for.
- **Runtime feature `standard-color-v2`** replaces `standard-color-v1` for the new operation set.
- **Graphics accepts any color.** Scene preparation maps each color into sRGB, so prepared paints,
  both renderers and the `graphics-core-v4` profile keep their current sRGB contract. Drawing on P3
  surfaces is later work.
- **Tooling:**
  - Swatches are shown for the new constructors and case records. Browsers draw them from `toCss`
    text, which they map for the screen.
  - A swatch outside sRGB gets a mark.
  - The picker edits sRGB colors as today and shows wide colors read-only.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `color-library`: `Color` as a union of spaces, the new constructors and gamut functions, defined
  behavior for every function on every space, CSS parsing and output for P3, Oklab and Oklch, and
  the `standard-color-v2` feature.
- `color-literals`: hex literals give `Color.srgb`.
- `color-tooling`: discovery, diagnostics and swatches for the new forms; picker edits only sRGB
  colors.
- `graphics-library`: graphics accepts any `Color` and maps it into sRGB at preparation.

## Impact

- **Standard library:** `crates/nx-program/src/std/color/color.nx` and `operations.nx`.
- **Runtimes:** the color externals in `crates/nx-value/src/color.rs` (Rust) and
  `runtime/typescript/src/color.ts`. Conversions go through unclamped linear-light sRGB. Gamut
  mapping follows CSS Color 4, and the two runtimes agree within the existing tolerance.
- **Compiler:** hex literals lower to `Color.srgb`. Swatch discovery and value printing in
  `nx-language-service` and `nx-api` recognize the four cases.
- **Graphics:** `nx-graphics` preparation maps colors into sRGB. The Skia and Canvas paths are
  otherwise unchanged.
- **Transport:** colors cross JSON, MessagePack and the SDKs as ordinary union-case records
  (`"$type": "Color.srgb"`). SDK clients that build colors by hand need the new shape.
- **Viewer and playground:** swatches, the out-of-sRGB mark, and picker rules.
- **Docs:** the `@nx/color` and `@nx/graphics` reference pages.
- **Follow-ups:**
  - P3 drawing surfaces in Skia and Canvas.
  - An OKLCH picker.
  - `@nx/design` could then adopt an OKLCH palette (an open question on `add-design-tokens`).
