## Purpose

Provide portable shared color values and deterministic construction and conversion for NX applications, independently of graphics or UI rendering.

## ADDED Requirements

### Requirement: Independent standard color library
The system SHALL provide unstable `@nx/color` with canonical `Color` and `ParseResult` types and the functions, conversion records, interpolation-space union, and named-color constants specified below. Exports SHALL be documented; self-explanatory named-color constants MAY share one group comment. Color-only programs SHALL analyze, link, and evaluate on Rust and TypeScript IR without importing graphics or linking a renderer. Missing module images or required `standard-color-v1` runtime support SHALL produce explicit failures. Aliased imports SHALL preserve declaration identity.

#### Scenario: Survey colors without graphics
- **WHEN** a program imports only `@nx/color`, declares a brand record with a Color field, and constructs its value using parse
- **THEN** Rust and TypeScript IR evaluation SHALL return equivalent typed values without graphics, Skia, Canvas, or a UI runtime

#### Scenario: Unsupported runtime fails explicitly
- **WHEN** an emitted program needs a color function but its runtime lacks standard-color-v1
- **THEN** evaluation SHALL fail with a missing-feature diagnostic rather than substitute a string or unvalidated record

### Requirement: Color is an ordinary sRGB record
Color SHALL contain float32 r, g, b channels in encoded sRGB and non-premultiplied alpha a with default 1. Valid channels SHALL be finite and in [0,1]. srgb SHALL accept normalized channels and default alpha to 1. rgb and rgba SHALL use 0..255 RGB channels, with rgba alpha in [0,1]. Numeric function parameters SHALL accept float64 calculations; constructors SHALL validate before narrowing and round stored channels to IEEE float32 nearest/ties-to-even. Validation of external records SHALL reject channel values not representable as float32. validate SHALL accept Color and return a validated Color. Constructors and validate SHALL reject invalid channels without clamping using `color-invalid-channel`, identifying the field. Constructors SHALL normalize signed zero. All library functions consuming Color SHALL validate it before conversion. Ordinary record construction SHALL remain possible and SHALL NOT be described as enforcing channel ranges automatically.

#### Scenario: Omitted alpha and precision
- **WHEN** srgb receives (0.123456789, 0.4, 0.8) without alpha
- **THEN** it SHALL return those channels rounded to float32 with alpha 1, without byte quantization

#### Scenario: Invalid direct record
- **WHEN** validate or a formatter receives a directly authored Color with r=2, NaN, or infinity
- **THEN** it SHALL report color-invalid-channel for r without producing formatted output

### Requirement: Deterministic text parsing and recoverable failure
parse and tryParse SHALL accept hex #RGB, #RGBA, #RRGGBB, #RRGGBBAA, the fixed CSS named-color table including aliases and rebeccapurple, and transparent. They SHALL trim outer ASCII whitespace and match ASCII names case-insensitively. Hex bytes SHALL map to float32(channel/255). Parsing SHALL additionally accept the standalone functional subset defined below. transparent SHALL produce (0,0,0,0). Contextual colors, none, CSS variables, unsupported functional syntax and color spaces SHALL be rejected. parse SHALL fail with color-invalid-text; tryParse SHALL return ParseResult.parsed with value or ParseResult.invalid with message, without failing evaluation for malformed text.

#### Scenario: Equivalent spellings
- **WHEN** parse evaluates " RED ", "#f00", and "#ff0000"
- **THEN** all three SHALL yield equal Color records with (1,0,0,1)

#### Scenario: Alpha hex order
- **WHEN** parse evaluates "#1234"
- **THEN** channels SHALL equal the float32-rounded values of (17/255,34/255,51/255,68/255), with alpha last

#### Scenario: Contextual and malformed text
- **WHEN** inputs include "currentColor", "var(--accent)", "none", "#12", or "#gg0000"
- **THEN** parse SHALL fail and tryParse SHALL return invalid for each input using the same acceptance rules on both runtimes

### Requirement: Explicit portable string conversion
toHex SHALL emit lowercase #rrggbbaa, rounding normalized channels with floor(channel*255+0.5). toCss SHALL emit a standalone rgba(R, G, B, A) string with RGB scaled by 255 and precision sufficient for reparsing to recover the identical stored float32 channels. Conversion SHALL NOT modify Color values. Zero alpha SHALL NOT erase RGB channels. Textual output SHALL NOT require an environment to resolve.

#### Scenario: Byte conversion is explicit
- **WHEN** toHex receives (1,0,0,0.5)
- **THEN** it SHALL return "#ff000080" and the original alpha SHALL remain 0.5

#### Scenario: Transparent RGB and precision survive CSS conversion
- **WHEN** toCss receives a Color storing the float32-rounded channels of (0.123456789,0.4,0.8,0)
- **THEN** its emitted channels SHALL preserve all four stored float32 channels on reparse, including the nonzero RGB values

### Requirement: Color uses ordinary value transport
Color SHALL use existing typed-record IR, JSON, MessagePack, and SDK value representations. Schema/type export SHALL describe numerical fields and the alpha default. Colors SHALL NOT implicitly serialize as CSS strings or require a new generic value tag. The API reference SHALL distinguish typed fields from runtime range validation and document explicit host conversion.

#### Scenario: Nested binding round trip
- **WHEN** an application record containing a Color crosses Rust, TypeScript, WASM, and supported SDK JSON/MessagePack boundaries
- **THEN** the linked Color identity, numerical channels, and ordinary record shape SHALL survive the round trip

### Requirement: Familiar constructors and conversion functions
The library SHALL provide this public API. All numeric parameters and channel-record members SHALL be float64 except Color's float32 fields; all optional alpha parameters SHALL default to 1. All color-consuming functions SHALL validate inputs. Numerical constructor bounds SHALL be enforced without implicit clamping except the explicitly stated gamut/HWB rules. Hue SHALL wrap finite degrees modulo 360 and achromatic conversion hue SHALL be zero.

| API | Contract |
| --- | --- |
| rgb(r,g,b), rgba(r,g,b,a) | RGB channels 0..255; rgba alpha 0..1 |
| srgb(r,g,b,a=1), linearRgb(r,g,b,a=1) | Channels 0..1, encoded or linear sRGB respectively |
| hsl(h,s,l,a=1), hsv(h,s,v,a=1), hwb(h,w,b,a=1) | Hue degrees; other channels 0..1; HWB w+b>1 normalizes their sum to 1 |
| oklab(l,a,b,alpha=1), oklch(l,c,h,alpha=1) | Lightness 0..1, finite Oklab a/b, nonnegative chroma, hue degrees; convert to sRGB then clip RGB to [0,1] |
| toRgb(value), toHsl(value), toHsv(value), toHwb(value), toLinearRgb(value), toOklab(value), toOklch(value) | Return RgbChannels, HslChannels, HsvChannels, HwbChannels, LinearRgbChannels, OklabChannels, OklchChannels respectively; fields match constructor names/units, using alpha instead of a to avoid ambiguity |
| parse(text), tryParse(text), validate(value), toHex(value), toCss(value) | Parsing, validation, and output contracts defined above |

All named CSS colors including aliases SHALL be exported as canonical Color constants, plus transparent. NX constant identifiers SHALL use camelCase word boundaries, such as lightSkyBlue, darkOliveGreen, and rebeccaPurple; CSS text parsing SHALL remain ASCII case-insensitive. Named constants SHALL use hex literals in a compact group with one shared comment. Fully transparent colors SHALL retain hidden RGB except operations explicitly producing zero RGB at zero alpha. Conversion matrices/transfer functions SHALL follow the pinned CSS Color 4 reference; HSV SHALL use conventional six-sector conversion. Out-of-sRGB Oklab/Oklch inputs SHALL be clipped after conversion, not retained as wide-gamut values. Non-finite inputs/intermediates SHALL fail with color-invalid-channel. Equivalent input decoding SHALL yield identical float32 channels on Rust and TypeScript; nonlinear transformations SHALL agree within absolute channel error 2e-6 and SHALL not quantize to bytes internally.

#### Scenario: CSS and normalized constructors agree
- **WHEN** rgb(255,0,0), srgb(1,0,0), hsl(360,1,0.5), and the named red constant are evaluated
- **THEN** all SHALL equal #ff0000 with opaque alpha

#### Scenario: Conversion edge cases
- **WHEN** an achromatic color is converted to HSL/HSV or hwb(0,1,1) is constructed
- **THEN** achromatic hue SHALL be zero and the HWB result SHALL have equal encoded RGB channels of 0.5

### Requirement: CSS functional parsing is explicit and portable
parse and tryParse SHALL accept rgb/rgba, hsl/hsla, hwb, oklab, oklch, color(srgb ...), and color(srgb-linear ...) using CSS Color 4 numeric/percentage and hue-unit rules. Legacy comma syntax SHALL be supported only for RGB/HSL; modern space/slash syntax SHALL be supported for the listed functions. Hue units SHALL include deg, grad, rad, turn and unitless degrees. Textual out-of-range CSS channels SHALL be clamped according to the pinned CSS rules, unlike strict numeric constructors. Oklab/Oklch conversion SHALL use the documented sRGB clipping policy. Contextual values, missing components/none, CSS variables, calc, relative colors, color-mix expressions, and other profiles SHALL be rejected. Parser acceptance SHALL NOT depend on a browser, renderer, or machine configuration.

#### Scenario: Familiar CSS text
- **WHEN** parse receives "rgb(255, 0, 0)", "rgb(100% 0% 0% / 50%)", or "hsl(0.5turn 100% 50%)"
- **THEN** it SHALL return respectively opaque red, half-transparent red, and opaque cyan using float32 channels

### Requirement: Immutable transformations preserve defined channel semantics
The library SHALL provide withAlpha(value,alpha), fadeIn(value,amount), fadeOut(value,amount), lighten(value,amount), darken(value,amount), saturate(value,amount), desaturate(value,amount), rotateHue(value,degrees), complement(value), grayscale(value), and invert(value,amount=1). Amounts and alpha SHALL be finite in [0,1]. withAlpha SHALL replace alpha; fadeIn/fadeOut SHALL add/subtract and clamp alpha. lighten/darken and saturate/desaturate SHALL add/subtract absolute normalized HSL lightness/saturation and clamp the result. rotateHue SHALL wrap degrees; complement SHALL rotate by 180 degrees; grayscale SHALL set HSL saturation to zero; invert SHALL interpolate encoded RGB toward 1-RGB by amount. All transformations other than alpha operations SHALL preserve alpha, leave inputs unchanged, and return validated float32 Color values.

#### Scenario: Predictable adjustments
- **WHEN** lighten receives hsl(0,1,0.4,0.5) with amount 0.2 and fadeOut receives that original color with amount 0.8
- **THEN** lighten SHALL yield HSL lightness approximately 0.6 with unchanged alpha, and fadeOut SHALL yield alpha zero with unchanged RGB

### Requirement: Mixing and compositing specify color space and transparency
mix(first,second,weight=0.5,space=oklab) SHALL accept InterpolationSpace = srgb | linearSrgb | oklab. Weight SHALL be finite in [0,1] and refer to the second color. Mixing SHALL convert to the selected rectangular space, premultiply coordinates by alpha, interpolate coordinates/alpha, unpremultiply, convert back, and clip RGB. Endpoints SHALL preserve the respective input; interior zero-alpha results SHALL use zero RGB. composite(foreground,background) SHALL implement source-over in linear-sRGB using premultiplied intermediate channels and zero RGB when output alpha is zero. Invalid weights SHALL fail. Neither function SHALL mutate inputs.

#### Scenario: Transparency does not contaminate a mix
- **WHEN** mix blends opaque red and transparent blue at weight 0.5 in srgb
- **THEN** it SHALL yield red RGB with alpha 0.5 rather than purple

#### Scenario: Linear mixing differs from encoded mixing
- **WHEN** opaque black and white are mixed equally in srgb and linearSrgb
- **THEN** the former SHALL have encoded RGB approximately 0.5 and the latter approximately 0.735357

### Requirement: Luminance and contrast use explicit backgrounds
relativeLuminance(value) SHALL return float64 WCAG 2.2 relative luminance using sRGB decoding with breakpoint 0.04045 and weights 0.2126, 0.7152, 0.0722. contrastRatio(first,second) SHALL return (max(L1,L2)+0.05)/(min(L1,L2)+0.05). Both SHALL reject translucent inputs with color-invalid-channel; no background SHALL be assumed. bestTextColor(background,first=#000000,second=#ffffff) SHALL require opaque inputs and return the candidate with higher unrounded contrast ratio, choosing first on ties. These functions SHALL NOT claim general accessibility compliance or implement APCA.

#### Scenario: Known contrast endpoints
- **WHEN** contrastRatio compares black with white or compares a color with itself
- **THEN** it SHALL return 21 or 1 respectively

#### Scenario: Alpha requires compositing
- **WHEN** relativeLuminance receives #00000080
- **THEN** it SHALL reject the input and require an explicitly composited opaque color
