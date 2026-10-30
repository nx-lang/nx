---
title: '@nx/color'
description: 'Portable float32 colors, hex literals, CSS conversions and transformations.'
---

`@nx/color` is unstable. Its ordinary color records work in surveys, chat links, design tokens,
and graphics without a renderer or UI dependency. Evaluation targets are Rust and TypeScript IR;
the legacy HIR interpreter is outside this profile. `nxlang run` currently uses that interpreter
and reports that colors require the IR runtime. Use the playground, linked IR SDK evaluation,
or `nx-graphics-preview` for color programs.

```nx
import "@nx/color" as c
type Brand = { accent:c.Color }
let brand = <Brand accent=#5b6ef5/>
let root() = { c.toCss(brand.accent) }
```

## Hex literals

Use `#RGB`, `#RGBA`, `#RRGGBB`, or `#RRGGBBAA`, with alpha last. Short forms duplicate digits;
`#f008` equals `#ff000088`. Omitted alpha is opaque. Literals infer the canonical Color type without
imports, including in defaults and bare element properties. Quoted colors remain strings.

```nx
let accent = #5b6ef5
let overlay = #00000080
let root() = { accent overlay }
```

## Values and constructors

Color contains float32 `r`, `g`, `b`, and `a` fields. RGB is encoded sRGB; alpha defaults to 1
and is not premultiplied. Valid channels are finite and in 0..1. Constructors accept float64
calculations, validate before narrowing, and round to float32. Direct records do not enforce
ranges automatically: application hosts must validate external data.

| Constructor | Units |
| --- | --- |
| `rgb(r,g,b)` / `rgba(r,g,b,a)` | RGB 0..255, alpha 0..1 |
| `srgb(r,g,b,a=1)` | Encoded RGB 0..1 |
| `linearRgb(r,g,b,a=1)` | Linear-light RGB 0..1 |
| `hsl(h,s,l,a=1)` | Hue degrees, saturation/lightness 0..1 |
| `hsv(h,s,v,a=1)` | Hue degrees, saturation/value 0..1 |
| `hwb(h,w,b,a=1)` | Hue degrees, whiteness/blackness 0..1; sums above 1 normalize |
| `oklab(l,a,b,alpha=1)` | Lightness 0..1, finite opponent channels |
| `oklch(l,c,h,alpha=1)` | Lightness 0..1, nonnegative chroma, hue degrees |

Hue wraps modulo 360; achromatic conversion hue is zero. Strict numerical constructors reject
invalid channels. Oklab/Oklch results convert to sRGB and clip RGB; they do not retain wide-gamut
values. Named CSS color constants use camelCase NX identifiers, such as `c.red`, `c.lightSkyBlue`,
`c.rebeccaPurple`, and `c.transparent`. CSS text parsing remains case-insensitive.

## Parsing and output

`parse(text)` returns Color or fails with `color-invalid-text`. `tryParse(text)` returns either
`ParseResult.parsed { value:Color }` or `ParseResult.invalid { message:string }`. Use it for external
text that may be malformed. `validate(value)` reports invalid channels with `color-invalid-channel`.

The supported standalone CSS subset includes hex, names, transparent, rgb/rgba, hsl/hsla, hwb,
oklab, oklch, and color(srgb ...)/color(srgb-linear ...). Modern space/slash syntax, RGB/HSL legacy
commas, percentages, and hue angle units are supported. Textual CSS channels clamp according to
CSS rules, unlike strict numerical constructors. Contextual colors, variables, missing none
components, calc, relative colors, and color-mix expressions are excluded.

`toHex(value)` emits lowercase eight-digit #rrggbbaa, explicitly quantizing to bytes.
`toCss(value)` emits standalone rgba(...) retaining float32 precision. Ordinary serialization
keeps the record; conversion to a host's CSS string is explicit.

```nx
import "@nx/color" as c
let root() = { c.tryParse("rgb(100% 0% 0% / 50%)") }
```

## Conversions and transformations

`toRgb`, `toHsl`, `toHsv`, `toHwb`, `toLinearRgb`, `toOklab`, and `toOklch` return named records
using their constructor's channel units and an `alpha` field. Conversion fields are float64;
stored Color fields are float32. Operations are immutable.

| Function | Behavior |
| --- | --- |
| `withAlpha(value,alpha)` | Replace alpha |
| `fadeIn(value,amount)` / `fadeOut(value,amount)` | Add/subtract alpha and clamp |
| `lighten(value,amount)` / `darken(value,amount)` | Add/subtract absolute HSL lightness and clamp |
| `saturate(value,amount)` / `desaturate(value,amount)` | Add/subtract absolute HSL saturation and clamp |
| `rotateHue(value,degrees)` / `complement(value)` | Rotate hue by degrees / 180 degrees |
| `grayscale(value)` | Set HSL saturation to zero |
| `invert(value,amount=1)` | Blend encoded RGB toward its inverse |

Amounts/alpha are 0..1. Other transformations preserve alpha. HSL lightness is not perceptually
uniform brightness.

## Mixing, compositing, and contrast

`mix(first,second,weight=0.5,space=oklab)` uses the weight of the second color. Spaces are srgb,
linearSrgb, and oklab. Premultiplied interpolation prevents transparent RGB from contaminating
visible mixtures. Endpoints preserve their inputs. `composite(foreground,background)` performs
source-over in linear sRGB; this chosen space need not match every browser painting configuration.

`relativeLuminance` and `contrastRatio` use WCAG 2.2 and require opaque inputs. Composite
translucent colors against an explicit opaque background first. `bestTextColor(background,first,second)`
chooses the candidate with higher contrast, defaults to black/white, and chooses first on ties.
These calculations alone do not establish accessibility compliance.

```nx
import "@nx/color" as c
let surface = { c.composite(#00000080, #ffffff) }
let root() = { c.bestTextColor(surface) }
```

## Tooling and migration

Editable literals have swatches and an alpha-aware picker. Computed expressions stay read-only;
opening the picker does not change precision or rewrite source. Edits preserve surrounding source
and participate in normal undo.

Graphics profile graphics-core-v3 consumes this same Color. Replace `color="#2563eb"` with
`color=#2563eb`, use constructors for computed channels, and call `toCss` for string host APIs.
Color-only applications never need to import graphics.
