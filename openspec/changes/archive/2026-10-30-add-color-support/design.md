## Context

See proposal.md for motivation. Graphics currently exports `Color = string` and its neutral Rust preparer uses csscolorparser to convert strings to RGBA. The survey example duplicates a string alias and a `ValueVisualizer format=color` declaration. The viewer already supports swatches and string color editors in `packages/viewer`; those are presentation hints, not validation.

Standard libraries are ordinary NX modules. `@nx/math` demonstrates capability-gated external functions with Rust and TypeScript IR implementations. Graphics already links transitive standard-library images and transports ordinary records through WASM. These mechanisms are sufficient for an independent color library.

## Goals / Non-Goals

**Goals:** one shared color contract; renderer-free evaluation; deliberate validation; portable host conversions; useful authoring and editing in both Source and Visual views.

**Non-Goals:** new runtime primitive, implicit string coercion, generic refinement types, automatic validation of every record construction, gradients, theme resolution, retained wide gamut/HDR, ICC color management, full CSS expression evaluation, and external ReachMe deployments. Legacy HIR interpreter support is excluded, consistently with graphics; documentation must identify supported IR entry points and the separate interpreter-retirement dependency for other entry points.

## Decisions

### 1. Use an ordinary canonical record in `@nx/color`

Export `Color` with `r`, `g`, `b` float32 fields and `a:float32 = 1`. Channels are non-premultiplied, encoded sRGB in [0,1]; alpha is linear coverage in [0,1]. Signed zero is accepted and normalized by constructors. This provides ample color precision and ordinary record serialization without adding VM value kinds. Standard-library declaration identity, including through aliases/re-exports, identifies the type for tooling; a user record named Color is not automatically a color.

Example intended public API (including new literal syntax):

```nx
import "@nx/color" as c

type Brand = { accent:c.Color }
let brand = <Brand accent=#5b6ef5/>
let overlay = { c.srgb(0.2, 0.4, 0.8, 0.5) }
let css = { c.toCss(brand.accent) }
```

Literal syntax does not require a new VM primitive: lower a hex literal to the same canonical record produced by constructors. Unlike a string alias, this gives numerical channels and a distinct declared type without implicit string conversions.

All numeric function arguments and non-Color conversion records use float64 for compatibility with NX math and ordinary calculations. Color-returning functions validate input before narrowing, compute intermediates in float64, then round channels to float32 using IEEE nearest/ties-to-even (Math.fround semantics in TypeScript). The Color contract is four float32 fields, not a guarantee that every host's boxed record occupies 16 bytes. Normalize signed zero. Direct records must respect NX's existing float32 field typing; validate additionally rejects host-supplied non-float32-representable channel values rather than silently accepting extra precision. CSS serialization must recover the same float32 components on reparse; transformation parity uses absolute channel error <= 2e-6, not the former 1e-15 requirement. Nonlinear algorithms are pinned by fixtures; neither implementation may byte-quantize intermediate values.

Records are not opaque: users can directly construct invalid channel values. All library functions accepting a Color and graphics preparation must validate their inputs, including records received through SDKs. Documentation must not claim that type checking alone guarantees valid channels. Static tooling diagnoses direct literals where determinable; arbitrary application hosts must validate incoming values using the library contract.

### 2. Hex literals and canonical type resolution

Add one color expression for #RGB, #RGBA, #RRGGBB, and #RRGGBBAA with case-insensitive hex digits and alpha last. Literal decoding uses byte/255 rounded to float32. Support expression, default, sequence, and unbraced element-property positions, such as `<Brand accent=#5b6ef5/>`. Quotes still mean strings. Identical channel values compare equally regardless of spelling.

Recognize malformed hash-prefixed tokens as a whole for useful diagnostics: invalid digits/length or an adjacent identifier character must not become a valid color plus stray tokens. Whitespace, operators, punctuation, comments, and element delimiters terminate tokens. Hashes inside strings/comments remain unchanged. Preserve literal source spelling on formatting/round trips unless the user edits its color. Update grammar source, generated parser/AST, lowering, highlighters, source/value printers, and grammar reference together.

Resolve the literal type through the standard-library registry's canonical `@nx/color` module identity, never a source-level name lookup. Compiler discovery loads that declaration without an explicit import; emitted images include the implicit color module using normal identity/version and dependency mechanisms. A literal alone lowers to record constants and does not require standard-color-v1 external-function support. A missing canonical module is an explicit compile/link error, never a user-defined fallback. Library declarations themselves must not use literals in a way that introduces initialization cycles. Runtime formatting of Color may emit hex only when byte-representable; otherwise emit a constructor/record that retains float32 components.

### 3. Public color API and text contract

The API table in `specs/color-library/spec.md` is the normative function inventory and units. Provide the fixed CSS named-color constants (including aliases) plus transparent; every constant has canonical Color type. Use camelCase NX identifiers (for example, lightSkyBlue and rebeccaPurple), while CSS parsing remains case-insensitive. Keep the hex-literal declarations together under one group comment without repetitive individual comments or blank lines. No unqualified global color names are introduced.

Keep sRGB as the stored representation, but support RGB, normalized sRGB, HSL, HSV, HWB, linear-sRGB, Oklab, and Oklch construction/conversion. CSS-facing rgb/rgba use 0..255 channels; srgb uses 0..1. Hue is clockwise degrees, wrapped modulo 360; HSL/HSV/HWB numerical proportions are 0..1, unlike CSS text percentages. Returned channel records use descriptive named fields and float64 members; only stored Color components are float32. Achromatic hue is deterministically 0. HWB normalizes whiteness/blackness when their sum exceeds 1. Use CSS Color 4 conversion equations and matrices; HSV uses the conventional six-sector RGB/HSV definition. Oklab/Oklch and linear constructors convert to encoded sRGB and explicitly clip final RGB to [0,1]; do not claim lossless wide-gamut storage or CSS gamut-mapping equivalence. Oklab a/b may be any finite number, Oklch chroma must be nonnegative; reject non-finite intermediates as color-invalid-channel.

Parse hex, ASCII-case-insensitive named colors, transparent, CSS rgb()/rgba(), hsl()/hsla(), hwb(), oklab(), oklch(), and color(srgb ...)/color(srgb-linear ...). Support CSS Color 4 numeric/percentage ranges, legacy comma syntax only for RGB/HSL, modern space/slash syntax, and hue deg/grad/rad/turn units. Trim outer ASCII whitespace. Clamp CSS textual ranges as CSS specifies, while numeric constructors reject out-of-range values; Oklab/Oklch conversion clips explicitly as above. Reject currentColor, system colors, none/missing components, variables, calc(), relative colors, color-mix(), other profiles and functions. This is a documented standalone-color subset, not a CSS evaluator. Parsing errors return color-invalid-text or ParseResult.invalid; tryParse malformed inputs do not fail evaluation.

Transformations are immutable. HSL-based lighten/darken and saturate/desaturate add/subtract an absolute normalized amount then clamp the affected channel. They do not promise perceptually uniform brightness. Hue rotation wraps; complement rotates by 180 degrees; grayscale sets HSL saturation to zero; invert blends encoded RGB toward 1-RGB by its amount. These preserve alpha. withAlpha replaces alpha; fadeIn/fadeOut add/subtract alpha then clamp. Explicit channel adjustment is available by toHsl/toRgb followed by the corresponding constructor; do not add an ambiguous catch-all adjust/scale API.

mix uses weight of the SECOND color (0 = first, 1 = second), default 0.5, with selectable srgb, linearSrgb, or oklab (default oklab). Convert to the chosen rectangular space, premultiply all three coordinates by alpha, interpolate coordinates and alpha, unpremultiply, convert back, then clip RGB. Interior all-transparent mixes return zero RGB. Endpoints return the original validated colors, preserving hidden RGB. No polar hue interpolation is included in this version. composite implements foreground-over-background in linear-sRGB with premultiplied intermediate math; transparent output is zero RGB. This explicit linear-space operation is not a promise of matching every browser canvas compositing configuration.

relativeLuminance and contrastRatio use WCAG 2.2 sRGB transfer and luminance equations; they require opaque colors so no implicit background is assumed. Call composite first for translucent colors and ensure its output is opaque. bestTextColor compares two opaque candidates (black then white by default) against an opaque background, returns the higher-ratio candidate, and chooses the first on ties; it does not claim universal accessibility compliance. Ratio threshold comparisons use unrounded results. Functions reject invalid parameters with color-invalid-channel (including invalid weight/amount/opacity), without silent constructor clamping.

Source references (pin versions and independent numerical expectations in the corpus):

- [CSS Color 4, 29 October 2026](https://www.w3.org/TR/2026/CRD-css-color-4-20261029/) for syntax, conversion, and premultiplied interpolation.
- [Sass color module](https://sass-lang.com/documentation/modules/color/) for familiar operation vocabulary; NX's explicitly documented units and formulas take precedence over Sass defaults.
- [WCAG 2.2, 12 December 2024](https://www.w3.org/TR/2024/REC-WCAG22-20241212/#dfn-relative-luminance) for luminance and contrast, using 0.04045 as the sRGB transfer breakpoint.

### 4. Reuse standard-library runtime mechanisms

Keep canonical types/constants in `color.nx` and functions in `operations.nx`, using a narrowly validated import between registered sibling modules. Literal-only programs can link the contracts image without the operations image or external-function feature. Register both unstable source modules and implement closed functions under required feature `standard-color-v1`, following math's capability gating and ordinary external-function dispatch. Compiler-synthesized `standardColor` body nodes (kind 27) carry an operation ID and argument list; importers retain ordinary calls and values remain ordinary records. Validate opcode, arity, earlier-node references, and required feature before execution. Use a renderer-independent Rust color utility shared by the Rust IR functions, language service, and graphics validation; provide the equivalent pure TypeScript implementation with the same data corpus. No browser DOM parser, network lookup, or Skia link is permitted. Dynamic input limits follow existing runtime string/work budgets; grammar scanning is linear.

Color values retain ordinary typed-record identities and encodings. Do not serialize them implicitly as strings. SDK consumers explicitly call `toCss`/`toHex` when a host API requires strings. Existing schema/type export must expose fields/defaults rather than claim channel constraints the type system cannot enforce. Missing feature implementations or linked module images fail explicitly.

### 5. Add semantic tooling without changing string visualizers

Expose canonical-color identity and statically discoverable values through language-service metadata and bindings, rather than recognizing a declaration's display name. Existing `ValueVisualizer` remains for scalar presentation; do not attach it illegally to the Color record or change the meaning of user string aliases.

Discover supported direct color expressions: hex literals, `parse` with a literal string, `rgb`, `rgba`, and `srgb` with literal numeric arguments, and directly authored Color records with literal channels and default alpha. Resolve standard function identities, including import aliases. Static diagnostics use the same parser/channel rules, never execute arbitrary NX code, and point to the offending string/channel. Literal errors in `tryParse` intentionally do not become diagnostics: that API models recoverable external data.

Source and Visual views show swatches for recognized expressions and typed evaluated Color values. The picker handles alpha explicitly with a separate control if the browser RGB picker lacks it. Preserve float precision until the user commits an edit; a displayed byte approximation alone must not dirty the document.

Picker edits replace only a hex token, a literal argument of `parse`, numeric arguments of `rgb`/`rgba`/`srgb`, or numeric fields of a direct Color record. Preserve alpha and untouched float32 values; hex edits use lowercase six digits for opaque colors or eight when alpha is present, retaining an existing explicit alpha suffix; preserve surrounding comments, imports, and source. If record syntax/comments prevent a safe field edit, offer a read-only swatch. Imported values, references, calls with computed inputs, and other computed expressions are read-only unless the user explicitly navigates to an editable definition. Edits use revision-checked language-service text edits and normal undo; stale edits fail and request rediscovery.

### 6. Make graphics a downstream consumer

Graphics imports/re-exports the canonical Color and changes all solid/background color fields to it. It transitively links `@nx/color`, alongside existing math. Neutral preparation validates channel records and converts to backend floats only at the destination boundary. Canvas explicitly formats colors; Skia consumes numerical channels. Neither adapter reparses authored strings.

Advance the graphics profile to `graphics-core-v2` because authored and evaluated color contracts change. Update native/browser manifests, schema fixtures, browser prepared-scene consumers and expected profiles together. Geometry, Paint.none, group opacity, and supported effect limitations retain existing behavior. No compatibility shim or parallel string/record representation is required for this unstable API.

## Risks / Trade-offs

- [Implicit standard-module dependency for literals] → resolve canonical registry identity, test import-free linking and cycle prevention.
- [Byte-based literal alpha differs from arbitrary float32 alpha] → do not rewrite higher-precision constructors to hex automatically.
- [Expanded operation surface] → pin units, default interpolation space, clipping, transparency, and reference fixtures before implementation.
- [Public records admit invalid data] → validate at library and rendering boundaries; document application-host responsibilities and test raw SDK input.
- [Rust/TypeScript parse or formatting differences] → one versioned corpus of valid/invalid inputs, channels, alpha, and conversion tolerances.
- [Special tooling for a standard record] → key on canonical linked identities and expose metadata centrally, not name-based renderer heuristics.
- [Breaking external consumers] → provide string-to-record and record-to-CSS migration examples; explicitly enumerate unsupported former functional inputs.
- [Profile and serialization drift] → round-trip nested Color values through IR, WASM, schema export, and SDK bindings; update fixtures atomically with graphics integration.

## Migration Plan

Deliver the library/runtime corpus first, then tooling and graphics integration. Migrate checked-in graphics source/expected values, rendering fixtures, survey declarations/defaults, and design-token documentation. Preserve string-color visualizer fixtures as compatibility coverage for that independent presentation feature. Publish examples for external ReachMe adoption without assuming access to its source.

Use `c.red` or `#ff0000` for authored named/hex colors, CSS-unit `c.rgba(...)` or normalized `c.srgb(...)` for computed inputs, `c.toCss(value)` for string host APIs, and `tryParse` at external-data boundaries. Color-only applications import no graphics library. A rollback before release reverts this change and regenerates dependent library images/fixtures together; no persisted stable color wire format is promised.
