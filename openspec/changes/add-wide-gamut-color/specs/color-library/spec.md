## ADDED Requirements

### Requirement: Color is a union of color spaces
`Color` SHALL be a union with the cases `srgb { r g b alpha }`, `displayP3 { r g b alpha }`, `oklab
{ l a b alpha }` and `oklch { l c h alpha }`, each field float32 and each `alpha` non-premultiplied
with default 1. The library SHALL export `Unit = float32 where in 0.0..=1.0` and `Hue = float32
where >= 0.0 && < 360.0`. `r`, `g`, `b`, `l` and every `alpha` SHALL have type `Unit`. Oklch `c`
SHALL be constrained to `>= 0.0` and `h` SHALL have type `Hue`. Oklab `a` and `b` SHALL be finite
and otherwise unconstrained. `srgb` and `displayP3` channels SHALL be encoded with the sRGB transfer
curve, using the sRGB and Display P3 primaries respectively. Because the ranges are value
constraints, an ordinary record construction SHALL be checked by the existing constant and runtime
constraint rules. `validate` SHALL accept any case and fail with `color-invalid-channel`, naming the
field, for a value that bypassed those rules, such as an SDK record. Library functions SHALL
validate every Color input. Constructors SHALL validate before narrowing, SHALL round stored channels
to IEEE float32 nearest/ties-to-even, and SHALL normalize signed zero. No library function SHALL
expose a channel of a color without naming its space, through a case or a conversion.

#### Scenario: Omitted alpha and precision
- **WHEN** srgb receives (0.123456789, 0.4, 0.8) without alpha
- **THEN** it SHALL return `Color.srgb` with those channels rounded to float32 and alpha 1, without byte quantization

#### Scenario: A constrained record is checked
- **WHEN** a module declares `let c: Color = <Color.srgb r=2 g=0 b=0 />`
- **THEN** analysis SHALL report `2.0` as outside the constraint for `r`

#### Scenario: Invalid SDK record
- **WHEN** validate or a formatter receives an SDK-supplied `Color.oklch` with `c` = -1, NaN, or infinity
- **THEN** it SHALL report color-invalid-channel for `c` without producing output

#### Scenario: A wide color keeps its space
- **WHEN** a program evaluates `oklch(0.7, 0.3, 145)`
- **THEN** the result SHALL be `Color.oklch` with those channels, although the color is outside sRGB and Display P3

### Requirement: Gamut checking and mapping are explicit
The library SHALL export `Gamut = srgb | displayP3`, `inGamut(value, gamut = srgb): boolean` and
`toGamut(value, gamut = srgb): Color`.
- `inGamut` SHALL return true when the color converted to the gamut's RGB space has every channel
  within 2e-6 of [0, 1].
- `toGamut` SHALL return `Color.srgb` or `Color.displayP3`. A color in the gamut SHALL convert
  without mapping, with each channel clamped into [0, 1] when it is within that tolerance. Any other
  color SHALL be mapped with the CSS Color 4 gamut-mapping algorithm: reduce Oklch chroma by binary
  search at constant lightness and hue, using ΔE_OK with a just-noticeable difference of 0.02, then
  clip.
- A color with Oklch lightness of at least 1 SHALL map to white, and one with lightness of at most 0
  SHALL map to black, preserving alpha.

Rust and TypeScript SHALL agree within absolute channel error 2e-6.

#### Scenario: An in-gamut color is unchanged
- **WHEN** `toGamut(#2563eb)` is evaluated
- **THEN** it SHALL equal `#2563eb`
- **AND** `inGamut(#2563eb)` SHALL be true

#### Scenario: A P3 color is mapped for sRGB and kept for P3
- **WHEN** `displayP3(1, 0, 0)` is checked and mapped
- **THEN** `inGamut(c, srgb)` SHALL be false and `inGamut(c, displayP3)` true
- **AND** `toGamut(c, srgb)` SHALL be `Color.srgb` with every channel in [0, 1] and Oklch hue within 2 degrees of the input's

## MODIFIED Requirements

### Requirement: Independent standard color library
The system SHALL provide unstable `@nx/color` with canonical `Color`, `Gamut` and `ParseResult`
types and the functions, conversion records, interpolation-space union, and named-color constants
specified below. Exports SHALL be documented; self-explanatory named-color constants MAY share one
group comment. Color-only programs SHALL analyze, link, and evaluate on Rust and TypeScript IR
without importing graphics or linking a renderer. Programs that call color functions SHALL require
the runtime feature `standard-color-v2`, which replaces `standard-color-v1`. Missing module images
or required `standard-color-v2` runtime support SHALL produce explicit failures. Aliased imports
SHALL preserve declaration identity.

#### Scenario: Survey colors without graphics
- **WHEN** a program imports only `@nx/color`, declares a brand record with a Color field, and constructs its value using parse
- **THEN** Rust and TypeScript IR evaluation SHALL return equivalent typed values without graphics, Skia, Canvas, or a UI runtime

#### Scenario: Unsupported runtime fails explicitly
- **WHEN** an emitted program needs a color function but its runtime lacks standard-color-v2
- **THEN** evaluation SHALL fail with a missing-feature diagnostic rather than substitute a string or unvalidated record

### Requirement: Explicit portable string conversion
toHex SHALL map its input into sRGB as `toGamut` does and then emit lowercase #rrggbbaa, rounding
normalized channels with floor(channel*255+0.5). toCss SHALL emit standalone CSS text in the
color's own space, with no mapping:
- `rgb(R G B / A)` for `srgb`, with RGB scaled by 255;
- `color(display-p3 r g b / A)` for `displayP3`;
- `oklab(l a b / A)` for `oklab`;
- `oklch(l c h / A)` for `oklch`.

Its precision SHALL be sufficient for reparsing to recover the identical case and stored float32
channels. Conversion SHALL NOT modify Color values. Zero alpha SHALL NOT erase channels. Textual
output SHALL NOT require an environment to resolve.

#### Scenario: Byte conversion is explicit
- **WHEN** toHex receives `Color.srgb` (1,0,0,0.5)
- **THEN** it SHALL return "#ff000080" and the original alpha SHALL remain 0.5

#### Scenario: Transparent channels and precision survive CSS conversion
- **WHEN** toCss receives a Color storing the float32-rounded channels of (0.123456789,0.4,0.8,0)
- **THEN** its emitted channels SHALL preserve all four stored float32 channels on reparse, including the nonzero RGB values

#### Scenario: Wide colors keep their space in CSS
- **WHEN** toCss receives `displayP3(1, 0, 0)` and toHex receives the same color
- **THEN** toCss SHALL return `color(display-p3 1 0 0 / 1)` and toHex SHALL return the hex of `toGamut(c, srgb)`

### Requirement: Color uses ordinary value transport
Color SHALL use the existing typed union-case record representations of IR, JSON, MessagePack and
the SDKs, with each case identified as union cases are today (for example `"$type": "Color.srgb"`
in JSON). Schema/type export SHALL describe each case's numerical fields, their constraints and the
alpha default. Colors SHALL NOT implicitly serialize as CSS strings or require a new generic value
tag. The API reference SHALL distinguish typed fields from runtime range validation and document
explicit host conversion, including `toGamut` and `toCss`.

#### Scenario: Nested binding round trip
- **WHEN** an application record containing a `Color.srgb` and a `Color.oklch` crosses Rust, TypeScript, WASM, and supported SDK JSON/MessagePack boundaries
- **THEN** the linked Color identity, each case, its numerical channels, and the ordinary record shape SHALL survive the round trip

### Requirement: Familiar constructors and conversion functions
The library SHALL provide this public API. All numeric parameters and channel-record members SHALL
be float64 except Color's float32 fields; all optional alpha parameters SHALL be named `alpha` and
default to 1. All color-consuming functions SHALL accept every case and validate inputs. Numerical
constructor bounds SHALL be enforced without implicit clamping except the explicitly stated HWB
rule. Hue SHALL wrap finite degrees modulo 360 and achromatic conversion hue SHALL be zero.

| API | Contract |
| --- | --- |
| rgb(r,g,b), rgba(r,g,b,a) | `Color.srgb`; RGB channels 0..255; rgba alpha 0..1 |
| srgb(r,g,b,alpha=1), linearRgb(r,g,b,alpha=1) | `Color.srgb`; channels 0..1, encoded or linear sRGB respectively |
| hsl(h,s,l,alpha=1), hsv(h,s,v,alpha=1), hwb(h,w,b,alpha=1) | `Color.srgb`; hue degrees; other channels 0..1; HWB w+b>1 normalizes their sum to 1 |
| displayP3(r,g,b,alpha=1) | `Color.displayP3`; encoded channels 0..1 |
| oklab(l,a,b,alpha=1), oklch(l,c,h,alpha=1) | `Color.oklab` and `Color.oklch`; lightness 0..1, finite Oklab a/b, nonnegative chroma, hue degrees; stored as given, never clipped |
| toRgb(value), toHsl(value), toHsv(value), toHwb(value), toLinearRgb(value) | the sRGB channel records, computed from `toGamut(value, srgb)`; fields match constructor names/units, using alpha |
| toOklab(value), toOklch(value) | OklabChannels and OklchChannels, converted exactly with no gamut mapping |
| inGamut(value,gamut=srgb), toGamut(value,gamut=srgb) | defined by the gamut requirement |
| parse(text), tryParse(text), validate(value), toHex(value), toCss(value) | parsing, validation, and output contracts defined in their requirements |

All named CSS colors including aliases SHALL be exported as canonical `Color.srgb` constants, plus
transparent. NX constant identifiers SHALL use camelCase word boundaries, such as lightSkyBlue,
darkOliveGreen, and rebeccaPurple; CSS text parsing SHALL remain ASCII case-insensitive. Named
constants SHALL use hex literals in a compact group with one shared comment. Fully transparent
colors SHALL retain hidden channels except operations explicitly producing zero channels at zero
alpha. Conversion matrices and transfer functions SHALL follow the pinned CSS Color 4 reference,
converting between spaces through linear-light XYZ D65 without clamping intermediates; HSV SHALL use
conventional six-sector conversion. Non-finite inputs or intermediates SHALL fail with
color-invalid-channel. Equivalent input decoding SHALL yield identical float32 channels on Rust and
TypeScript; nonlinear transformations SHALL agree within absolute channel error 2e-6 and SHALL not
quantize to bytes internally.

#### Scenario: CSS and normalized constructors agree
- **WHEN** rgb(255,0,0), srgb(1,0,0), hsl(360,1,0.5), and the named red constant are evaluated
- **THEN** all SHALL equal #ff0000 with opaque alpha

#### Scenario: Conversion edge cases
- **WHEN** an achromatic color is converted to HSL/HSV or hwb(0,1,1) is constructed
- **THEN** achromatic hue SHALL be zero and the HWB result SHALL have equal encoded RGB channels of 0.5

#### Scenario: sRGB channel records come from the mapped color
- **WHEN** toRgb receives `oklch(0.7, 0.3, 145)`
- **THEN** it SHALL return the channels of `toGamut(value, srgb)`, each in 0..255
- **AND** toOklch of the same value SHALL return l 0.7, c 0.3 and h 145

### Requirement: CSS functional parsing is explicit and portable
parse and tryParse SHALL accept rgb/rgba, hsl/hsla, hwb, oklab, oklch, color(srgb ...),
color(srgb-linear ...) and color(display-p3 ...) using CSS Color 4 numeric/percentage and hue-unit
rules. rgb, hsl, hwb, color(srgb) and color(srgb-linear) SHALL give `Color.srgb`, color(display-p3)
SHALL give `Color.displayP3`, and oklab and oklch SHALL give their own cases. Legacy comma syntax
SHALL be supported only for RGB/HSL; modern space/slash syntax SHALL be supported for the listed
functions. Hue units SHALL include deg, grad, rad, turn and unitless degrees. Textual out-of-range
CSS channels SHALL be clamped according to the pinned CSS rules, unlike strict numeric constructors.
Oklab and Oklch text SHALL be stored without gamut mapping. Contextual values, missing
components/none, CSS variables, calc, relative colors, color-mix expressions, and other profiles
SHALL be rejected. Parser acceptance SHALL NOT depend on a browser, renderer, or machine
configuration.

#### Scenario: Familiar CSS text
- **WHEN** parse receives "rgb(255, 0, 0)", "rgb(100% 0% 0% / 50%)", or "hsl(0.5turn 100% 50%)"
- **THEN** it SHALL return respectively opaque red, half-transparent red, and opaque cyan as `Color.srgb` using float32 channels

#### Scenario: Wide CSS text keeps its space
- **WHEN** parse receives "color(display-p3 1 0 0)" and "oklch(70% 0.3 145)"
- **THEN** it SHALL return `Color.displayP3` (1,0,0,1) and `Color.oklch` (0.7,0.3,145,1)

### Requirement: Immutable transformations preserve defined channel semantics
The library SHALL provide withAlpha(value,alpha), fadeIn(value,amount), fadeOut(value,amount),
lighten(value,amount), darken(value,amount), saturate(value,amount), desaturate(value,amount),
rotateHue(value,degrees), complement(value), grayscale(value), and invert(value,amount=1). Amounts
and alpha SHALL be finite in [0,1].
- withAlpha SHALL replace alpha. fadeIn/fadeOut SHALL add/subtract and clamp alpha.
- lighten/darken SHALL add/subtract amount to Oklch lightness and clamp it to [0,1].
- saturate/desaturate SHALL multiply Oklch chroma by 1+amount and 1−amount respectively.
- rotateHue SHALL add degrees to Oklch hue, wrapping modulo 360. complement SHALL rotate by 180
  degrees. grayscale SHALL set Oklch chroma to zero.
- invert SHALL interpolate encoded RGB toward 1−RGB by amount, in the input's space for `srgb` and
  `displayP3`, and in Display P3 after `toGamut(value, displayP3)` for `oklab` and `oklch`.

Every transformation except invert of an Oklab or Oklch input SHALL return a color in its input's
space. A result in `srgb` or `displayP3` SHALL be mapped into that gamut as `toGamut` does. All
transformations other than the alpha operations SHALL preserve alpha. All SHALL leave inputs
unchanged and return validated float32 Color values.

#### Scenario: Predictable adjustments
- **WHEN** lighten receives a `Color.srgb` with Oklch lightness 0.4 and alpha 0.5, with amount 0.2, and fadeOut receives that original color with amount 0.8
- **THEN** lighten SHALL yield a `Color.srgb` with Oklch lightness approximately 0.6, Oklch hue within 1 degree of the input's and unchanged alpha
- **AND** fadeOut SHALL yield alpha zero with unchanged RGB

#### Scenario: A wide color stays wide
- **WHEN** rotateHue receives `displayP3(1, 0, 0)` with 120 degrees
- **THEN** it SHALL return `Color.displayP3`, mapped into the Display P3 gamut rather than sRGB

### Requirement: Mixing and compositing specify color space and transparency
mix(first,second,weight=0.5,space=oklab) SHALL accept InterpolationSpace = srgb | linearSrgb |
oklab. Weight SHALL be finite in [0,1] and refer to the second color. Mixing SHALL:
- convert both inputs to the selected rectangular space without clamping;
- premultiply coordinates by alpha;
- interpolate coordinates and alpha;
- unpremultiply;
- convert to the result space.

The result space of mix and composite SHALL be the wider of the inputs' spaces, ordering `srgb` <
`displayP3` < `oklab` = `oklch`. When the two inputs are `oklab` and `oklch`, the result SHALL take
the first input's space. A result in `srgb` or `displayP3` SHALL be mapped into that gamut as
`toGamut` does.

Endpoints SHALL preserve the respective input. Interior zero-alpha results SHALL use zero
channels. composite(foreground,background) SHALL implement source-over in linear light, using
premultiplied intermediate channels and zero channels when output alpha is zero. Invalid weights
SHALL fail. Neither function SHALL mutate inputs.

#### Scenario: Transparency does not contaminate a mix
- **WHEN** mix blends opaque red and transparent blue at weight 0.5 in srgb
- **THEN** it SHALL yield red RGB with alpha 0.5 rather than purple

#### Scenario: Linear mixing differs from encoded mixing
- **WHEN** opaque black and white are mixed equally in srgb and linearSrgb
- **THEN** the former SHALL have encoded RGB approximately 0.5 and the latter approximately 0.735357

#### Scenario: Two hex colors mix to sRGB, a wide one widens the result
- **WHEN** mix blends #ff0000 with #0000ff, and then #ff0000 with `displayP3(0, 1, 0)`
- **THEN** the first result SHALL be `Color.srgb` and the second `Color.displayP3`

### Requirement: Luminance and contrast use explicit backgrounds
relativeLuminance(value) SHALL return the float64 WCAG 2.2 relative luminance of
`toGamut(value, srgb)`, using sRGB decoding with breakpoint 0.04045 and weights 0.2126, 0.7152,
0.0722. contrastRatio(first,second) SHALL return (max(L1,L2)+0.05)/(min(L1,L2)+0.05). Both SHALL
reject translucent inputs with color-invalid-channel; no background SHALL be assumed.
bestTextColor(background,first=black,second=white) SHALL require opaque inputs and return the
candidate, unmapped, with the higher unrounded contrast ratio, choosing first on ties. These
functions SHALL NOT claim general accessibility compliance or implement APCA.

#### Scenario: Known contrast endpoints
- **WHEN** contrastRatio compares black with white or compares a color with itself
- **THEN** it SHALL return 21 or 1 respectively

#### Scenario: Alpha requires compositing
- **WHEN** relativeLuminance receives #00000080
- **THEN** it SHALL reject the input and require an explicitly composited opaque color

#### Scenario: Contrast is measured on what an sRGB screen shows
- **WHEN** contrastRatio compares `oklch(0.7, 0.3, 145)` with white
- **THEN** it SHALL equal contrastRatio of `toGamut(oklch(0.7, 0.3, 145))` with white

## REMOVED Requirements

### Requirement: Color is an ordinary sRGB record
**Reason:** `Color` becomes a union of color spaces, and its ranges become value constraints that
ordinary construction enforces.
**Migration:** Read channels by matching on `Color.srgb`, or convert with `toRgb` or `toGamut`.
Rename `a` to `alpha`. Records written as `<Color r=… g=… b=… />` become `<Color.srgb … />`.
