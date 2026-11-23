## Context

`Color` is the record `{ r g b: float32, a: float32 = 1 }`, encoded sRGB with every channel in
[0, 1]. The library validates channels in each function. The record itself enforces nothing.
Colors outside sRGB can't be held: `oklab` and `oklch` clip after converting, and the parser rejects
`color(display-p3 …)`. Consumers are `@nx/graphics` (Skia and Canvas), the IR runtimes' color
externals, swatch discovery and the picker, value printing, and SDK transport.

## Goals / Non-Goals

**Goals:**
- Hold Display P3, Oklab and Oklch colors exactly as written, including colors outside every screen.
- Leave the common case unchanged: hex literals, `rgb`, `hsl` and the named colors stay sRGB.
- Make misuse hard. No code reads a channel without naming its space. Every function has defined
  behavior for every space. Mapping a color into a gamut is either explicit or stated by the
  function that does it.
- Match CSS Color 4 and DTCG 2025.10, so browsers and token export take colors as they are.

**Non-Goals:**
- Drawing on wide-gamut surfaces. Renderers keep an sRGB surface and get the mapped color.
- Other CSS spaces (`a98-rgb`, `prophoto-rgb`, `rec2020`, `lab`, `lch`, `xyz`). A case can be added
  later without breaking readers who use the conversion functions.
- HDR, meaning channels above white.
- An OKLCH or P3 picker.

## Decisions

### `Color` is a union of color spaces

```nx
export type Color =
  | srgb {
      r: float32 where in 0.0..=1.0
      g: float32 where in 0.0..=1.0
      b: float32 where in 0.0..=1.0
      alpha: float32 where in 0.0..=1.0 = 1
    }
  | displayP3 {
      r: float32 where in 0.0..=1.0
      g: float32 where in 0.0..=1.0
      b: float32 where in 0.0..=1.0
      alpha: float32 where in 0.0..=1.0 = 1
    }
  | oklab {
      l: float32 where in 0.0..=1.0
      a: float32
      b: float32
      alpha: float32 where in 0.0..=1.0 = 1
    }
  | oklch {
      l: float32 where in 0.0..=1.0
      c: float32 where >= 0.0
      h: Hue
      alpha: float32 where in 0.0..=1.0 = 1
    }
```

Each case names its channels for what they mean, so an Oklch color has `l`, `c` and `h`, not an `r`
that holds lightness. To read channels, a program matches on the case, or converts first with
`toGamut`, `toRgb` or `toOklch`. Exhaustive matching means code that handles colors by hand is told
when a space is added. The case is also the space tag. It costs what a `$type` costs on any union
value, and channels were already float32.

*Alternative: a record with a `space` field* (`{ space: ColorSpace = srgb, c0 c1 c2 alpha }`, like
DTCG's `components`). Rejected: the channel names can't say what they hold, and nothing stops code
from reading `c0` as red.

*Alternative: extended-range sRGB*, where channels may go below 0 or above 1 (Apple's model). This
was the likely choice in `specs/future.md`. Rejected:
- `r` would still read as red while meaning something else.
- Oklch colors written by hand would lose their authored form, so Visual couldn't show
  `oklch(0.62 0.19 260)` back.
- Converting to and from linear sRGB picks up float rounding.

### Ranges are value constraints on the fields

Channels, lightness and alpha are `float32 where in 0.0..=1.0`, written on each field, and hue is
`Hue = float32 where >= 0.0 && < 360.0`, as `add-value-constraints` allows. The 0 to 1 range has no
alias: no name for it reads well to every audience (`Unit` reads as the unit type), and
`@nx/graphics` already writes its opacities the same way. A literal out of range is a compile error, and a computed one fails
with `nx-ir-constraint` where it is bound. This replaces the rule that "ordinary record construction
does not enforce channel ranges". The library's `color-invalid-channel` check stays for SDK values
and for non-finite intermediates. Hue is stored in [0, 360), and `oklch()` wraps it.

### Alpha is `alpha` in every space

Oklab has a channel named `a`, so the alpha field is named `alpha` everywhere. That's also the name
the channel records already use. Constructors take `alpha` too. `rgba(r, g, b, a)` keeps CSS's
parameter name.

### Spaces: sRGB, Display P3, Oklab, Oklch

- **sRGB:** the default and every hex literal.
- **Display P3:** the wide gamut real screens show. It is the one `color(display-p3 …)` and Apple
  APIs use.
- **Oklab and Oklch:** perceptual spaces for authoring and palettes, which can describe colors
  outside both gamuts.

`Gamut = srgb | displayP3` names what a screen can show. Only the two RGB spaces are gamuts.

### Defined behavior for every function

The rule: a function keeps a wide color wide unless its result is defined only in sRGB. Then it maps
into sRGB first and says so in its doc comment.

| Function | Input space | Result |
| --- | --- | --- |
| `toGamut(c, gamut)` | any | `Color.srgb` or `Color.displayP3`, mapped if outside |
| `inGamut(c, gamut)` | any | whether `c` fits, within 2e-6 per channel |
| `toHex`, `toRgb`, `toHsl`, `toHsv`, `toHwb`, `toLinearRgb` | any | sRGB, mapped first |
| `relativeLuminance`, `contrastRatio`, `bestTextColor` | any | measured on the sRGB-mapped color |
| `toOklab`, `toOklch` | any | exact, never mapped |
| `toCss` | any | the color's own space |
| `withAlpha`, `fadeIn`, `fadeOut` | any | same space |
| `lighten` … `grayscale` | any | same space, mapped into its gamut if RGB |
| `invert` | sRGB, P3 | same space |
| `invert` | Oklab, Oklch | Display P3 |
| `mix`, `composite` | any | the wider input space |

"Wider" orders the spaces sRGB < Display P3 < Oklab = Oklch. When both inputs are Oklab or Oklch and
they differ, the result takes the first input's space. Two sRGB colors always give sRGB, so mixing
hex colors stays hex.

Contrast is measured after mapping into sRGB because WCAG 2.2 is defined in sRGB, and an sRGB screen
is the least a color is shown on. Measuring on the mapped color means a check can't pass on a color
the screen can't show.

### Gamut mapping is CSS Color 4's

`toGamut` follows the CSS Color 4 gamut-mapping algorithm:
- Convert to Oklch.
- Binary-search chroma down, holding lightness and hue, until the clipped color is within ΔE_OK 0.02
  of the unclipped one.
- Clip.

Browsers use the same algorithm, so a color mapped by NX matches the one the browser shows for its
`toCss` text. The pinned reference is the one `color-library` already pins for its matrices.

### Hue-based transformations move to OKLCH

`lighten`, `darken`, `saturate`, `desaturate`, `rotateHue`, `complement` and `grayscale` used HSL,
which exists only inside sRGB. They now work in OKLCH for every input:
- `lighten` and `darken` add and subtract L.
- `saturate` and `desaturate` scale C by `1 ± amount`, since chroma has no fixed top.
- `rotateHue` and `complement` turn H.
- `grayscale` sets C to 0.

The result goes back into the input's space and, for an RGB space, is mapped into its gamut. This
changes results for sRGB colors: `lighten(#2563eb, 0.1)` gives a different blue than it does today.
The new blue is closer to "the same color, lighter", because OKLCH keeps hue and chroma steady where
HSL does not.

*Alternative:* keep HSL for sRGB inputs and use OKLCH only for wide ones. Rejected: the same call
would mean different things depending on how its input was written. See Decided Questions.

### Literals, transport and the runtime feature

- Hex literals lower to a `Color.srgb` constant. Value printing writes an sRGB color whose channels
  are each exactly a byte as the hex literal that reads back as it, `#rrggbb`, or `#rrggbbaa` when
  alpha is not 1. Any other color is written as its case record, as
  `<Color.srgb r=0.1 g=0.0 b=0.0 alpha=1.0 />`, which is exact and names its space. Either way the
  value gets a swatch painted from its CSS text, and the outside-sRGB mark where it applies.
- Transport is the ordinary union-case record, `{"$type": "Color.srgb", "r": 1, "g": 0, "b": 0,
  "alpha": 1}`, through IR, JSON, MessagePack and the SDKs. No new value tag.
- The operation set changes, so emitted programs require `standard-color-v2`. A runtime without it
  fails with a missing-feature diagnostic, as `v1` does today. Literal-only programs still need no
  feature.

### Graphics maps at preparation

Scene preparation applies `toGamut(c, srgb)` to every paint color and gradient stop. Prepared paints
keep their sRGB float contract, so `graphics-rendering`'s interpolation rules and both renderers are
unchanged, and the profile stays `graphics-core-v4`: it names what renderers consume, which this
change keeps. P3 surfaces are a follow-up
(Skia `Color4f` with a P3 `ColorSpace`, and Canvas `{ colorSpace: "display-p3" }`).

### Tooling

- **Swatches:** discovery adds `displayP3`, `oklab` and `oklch` calls with literal numbers, the four
  case records with literal channels, and the new `parse` forms. Swatches are painted from `toCss`
  text. Browsers support `color(display-p3 …)` and `oklch()`, and map them for the screen.
- **Out-of-sRGB mark:** a swatch for a color outside sRGB gets a small mark, with a tooltip saying
  it shows differently on sRGB screens.
- **Picker:** edits `Color.srgb` colors exactly as today. Wide colors are shown read-only, with no
  picker, rather than being rewritten to hex.

## Risks / Trade-offs

- **Every reader of `.r` breaks.** → In the repo these are graphics preparation, the runtimes'
  externals, the viewer and the tests. No back-compat is needed yet. A program that wants sRGB
  channels calls `toRgb` or matches on `toGamut(c)`.
- **Changed results for `lighten` and its kin on sRGB colors.** → Recorded as a breaking change.
  Tests and examples that pin exact HSL results are updated.
- **Gamut mapping is costlier than clipping**: a binary search of about 10 to 15 steps. → It only
  runs for colors outside the gamut and at output edges.
- **Equality is per space.** `#ff0000` and `oklch(0.628 0.258 29.2)` show the same red but are not
  `==`. → Documented. Compare `toGamut` results if needed.
- **The TypeScript and Rust runtimes must agree on mapping.** → Conformance cases for mapped colors,
  with the existing 2e-6 tolerance.

## Migration Plan

No back-compat. In one change:
- the library and both runtimes move;
- literals lower to `Color.srgb`;
- graphics, tooling, examples, tests and fixtures move to the new shape;
- `standard-color-v2` replaces `standard-color-v1`.

## Decided Questions

Settled with Bret on 2026-11-22:
- **OKLCH for `lighten` and its kin, for every input,** rather than HSL for sRGB inputs. One meaning
  per function, wide colors stay wide, and OKLCH keeps hue steady.
- **Oklab stays a stored space beside Oklch.** It matches CSS, keeps authored text exact, and is
  where `mix` interpolates by default.
- **P3 drawing in Skia and Canvas is a follow-up.** Mapping into sRGB at preparation is what an sRGB
  screen shows, and it keeps this change to the value model and the library.
