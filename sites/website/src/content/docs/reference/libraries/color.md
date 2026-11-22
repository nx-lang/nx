---
title: '@nx/color'
description: 'Colors in sRGB, Display P3, Oklab and Oklch: hex literals, gamut mapping, CSS and transformations.'
---

`@nx/color` is unstable. A color is a value of the `Color` union, whose case is the color space it
was written in. Colors work in surveys, chat links, design tokens, and graphics without a renderer
or UI dependency. Evaluation targets are Rust and TypeScript IR;
`nxlang run` compiles source to IR and uses the Rust runtime. The playground and linked IR SDK
hosts use the same color representation.

```nx
import "@nx/color" as c
type Brand = { accent:c.Color }
let brand = <Brand accent=#5b6ef5/>
let root() = { c.toCss(brand.accent) }
```

## Hex literals

Use `#RGB`, `#RGBA`, `#RRGGBB`, or `#RRGGBBAA`, with alpha last. Short forms duplicate digits;
`#f008` equals `#ff000088`. Omitted alpha is opaque. Literals are `Color.srgb` values of the canonical Color
type, without imports, including in defaults and bare element properties. Quoted colors remain
strings.

```nx
let accent = #5b6ef5
let overlay = #00000080
let root() = { accent overlay }
```

## Values and constructors

`Color` is a union with one case for each color space. Every case has float32 channels and an
`alpha` that defaults to 1 and is not premultiplied:

```nx
export type Unit = float32 where in 0.0..=1.0
export type Hue = float32 where >= 0.0 && < 360.0
export type Color =
  | srgb { r: Unit  g: Unit  b: Unit  alpha: Unit = 1 }
  | displayP3 { r: Unit  g: Unit  b: Unit  alpha: Unit = 1 }
  | oklab { l: Unit  a: float32  b: float32  alpha: Unit = 1 }
  | oklch { l: Unit  c: float32 where >= 0.0  h: Hue  alpha: Unit = 1 }
```

`Unit` is `float32 where in 0.0..=1.0` and `Hue` is `float32 where >= 0.0 && < 360.0`, so a
channel out of range is a compile error where it is written and a constraint failure where a
computed or host value is bound. sRGB is the default: hex literals, `rgb`, `hsl` and the named
colors all give `Color.srgb`. Display P3 holds the colors wide-gamut screens show. Oklab and Oklch
are perceptual spaces that can describe colors outside every screen, and keep them exactly as
written.

To read channels, match on the space or convert first, so no code reads Display P3 or Oklch
numbers as sRGB ones:

```nx
import "@nx/color" as c
let red(value: c.Color): float64 = { c.toRgb(value).r }
let root() = { red(c.oklch(0.7, 0.1, 200)) }
```

Constructors accept float64 calculations, validate before narrowing, and round to float32.

| Constructor | Result | Units |
| --- | --- | --- |
| `rgb(r,g,b)` / `rgba(r,g,b,a)` | `Color.srgb` | RGB 0..255, alpha 0..1 |
| `srgb(r,g,b,alpha=1)` | `Color.srgb` | Encoded RGB 0..1 |
| `linearRgb(r,g,b,alpha=1)` | `Color.srgb` | Linear-light RGB 0..1 |
| `hsl(h,s,l,alpha=1)` | `Color.srgb` | Hue degrees, saturation/lightness 0..1 |
| `hsv(h,s,v,alpha=1)` | `Color.srgb` | Hue degrees, saturation/value 0..1 |
| `hwb(h,w,b,alpha=1)` | `Color.srgb` | Hue degrees, whiteness/blackness 0..1; sums above 1 normalize |
| `displayP3(r,g,b,alpha=1)` | `Color.displayP3` | Encoded Display P3 0..1 |
| `oklab(l,a,b,alpha=1)` | `Color.oklab` | Lightness 0..1, finite opponent channels |
| `oklch(l,c,h,alpha=1)` | `Color.oklch` | Lightness 0..1, nonnegative chroma, hue degrees |

Hue wraps modulo 360; achromatic conversion hue is zero. Strict numerical constructors reject
invalid channels. Named CSS color constants use camelCase NX identifiers, such as `c.red`,
`c.lightSkyBlue`, `c.rebeccaPurple`, and `c.transparent`. CSS text parsing remains
case-insensitive.

## Gamuts

A gamut is what a screen can show: `Gamut = srgb | displayP3`. `inGamut(value, gamut=srgb)` says
whether a screen with that gamut shows the color as written. `toGamut(value, gamut=srgb)` converts
it into that gamut, mapping a color outside it with the CSS Color 4 algorithm, which keeps
lightness and hue and reduces chroma, as browsers do. Colors are compared per space:
`#ff0000 == c.displayP3(1, 0, 0)` is false, and `toGamut` results can be compared instead.

```nx
import "@nx/color" as c
let vivid = { c.oklch(0.7, 0.3, 145) }
let root() = { c.inGamut(vivid) c.toHex(c.toGamut(vivid)) }
```

## Parsing and output

`parse(text)` returns Color or fails with `color-invalid-text`. `tryParse(text)` returns either
`ParseResult.parsed { value:Color }` or `ParseResult.invalid { message:string }`. Use it for external
text that may be malformed. `validate(value)` reports invalid channels with `color-invalid-channel`.

The supported standalone CSS subset includes hex, names, transparent, rgb/rgba, hsl/hsla, hwb,
oklab, oklch, and color(srgb ...)/color(srgb-linear ...)/color(display-p3 ...). Display P3, Oklab
and Oklch text gives a color in its own space; the rest give `Color.srgb`. Modern space/slash syntax, RGB/HSL legacy
commas, percentages, and hue angle units are supported. Textual CSS channels clamp according to
CSS rules, unlike strict numerical constructors. Contextual colors, variables, missing none
components, calc, relative colors, and color-mix expressions are excluded.

`toHex(value)` emits lowercase eight-digit #rrggbbaa, mapping into sRGB first and quantizing to
bytes. `toCss(value)` writes the color in its own space, retaining float32 precision:
`rgb(R G B / A)`, `color(display-p3 R G B / A)`, `oklab(L A B / A)` or `oklch(L C H / A)`. Browsers
map the last three for the screen they draw on. Ordinary serialization keeps the record; conversion
to a host's CSS string is explicit.

```nx
import "@nx/color" as c
let root() = { c.tryParse("rgb(100% 0% 0% / 50%)") }
```

## Conversions and transformations

Every function accepts a color in any space. `toRgb`, `toHsl`, `toHsv`, `toHwb`, and `toLinearRgb`
are defined only in sRGB, so they map the color into sRGB first, as an sRGB screen would. `toOklab`
and `toOklch` are exact for every color. Each returns a named record using its constructor's channel
units and an `alpha` field. Conversion fields are float64; stored Color fields are float32.
Operations are immutable.

| Function | Behavior |
| --- | --- |
| `withAlpha(value,alpha)` | Replace alpha, in the same space |
| `fadeIn(value,amount)` / `fadeOut(value,amount)` | Add/subtract alpha and clamp |
| `lighten(value,amount)` / `darken(value,amount)` | Add/subtract OKLCH lightness |
| `saturate(value,amount)` / `desaturate(value,amount)` | Scale OKLCH chroma by `1 ± amount` |
| `rotateHue(value,degrees)` / `complement(value)` | Rotate OKLCH hue by degrees / 180 degrees |
| `grayscale(value)` | Set OKLCH chroma to zero |
| `invert(value,amount=1)` | Blend encoded RGB toward its inverse |

Amounts/alpha are 0..1. The OKLCH transformations return a color in their input's space, mapped
into its gamut when that space is sRGB or Display P3; they keep hue and lightness steadier than
HSL, which exists only inside sRGB. `invert` works in the color's own RGB space, and gives Display
P3 for an Oklab or Oklch color. Other transformations preserve alpha.

## Mixing, compositing, and contrast

`mix(first,second,weight=0.5,space=oklab)` uses the weight of the second color. Interpolation
spaces are srgb, linearSrgb, and oklab. Premultiplied interpolation prevents transparent RGB from
contaminating visible mixtures. At weight 0 or 1, `mix` returns that input unchanged, in its own
space. `composite(foreground,background)` performs source-over in linear light. Both return the
wider of their inputs' spaces, ordering sRGB < Display P3 < Oklab = Oklch, and the first input's
space when the two are Oklab and Oklch, so mixing two hex colors stays sRGB.

`relativeLuminance` and `contrastRatio` use WCAG 2.2, which is defined in sRGB, so they measure the
color mapped into sRGB, as the least capable screen shows it. They require opaque inputs. Composite
translucent colors against an explicit opaque background first. `bestTextColor(background,first,second)`
chooses the candidate with higher contrast, defaults to black/white, and chooses first on ties. It
returns the candidate as written.
These calculations alone do not establish accessibility compliance.

```nx
import "@nx/color" as c
let surface = { c.composite(#00000080, #ffffff) }
let root() = { c.bestTextColor(surface) }
```

## Tooling and migration

Editable literals have swatches and an alpha-aware picker. Swatches are painted from the color's
`toCss` text, and a color outside sRGB gets a small mark, since an sRGB screen shows it differently.
The picker edits sRGB colors; Display P3, Oklab and Oklch colors are shown read-only rather than
rewritten as hex. Computed expressions stay read-only; opening the picker does not change precision
or rewrite source. Edits preserve surrounding source and participate in normal undo.

Graphics accepts a color in any space and maps it into sRGB when a scene is prepared. Hosts and SDK
clients that build colors by hand write the union case, as
`{"$type": "Color.srgb", "r": 1, "g": 0, "b": 0, "alpha": 1}`. Programs that call color operations
need the runtime feature `standard-color-v2`. Replace `color="#2563eb"` with `color=#2563eb`, use
constructors for computed channels, and call `toCss` for string host APIs. Color-only applications
never need to import graphics.
