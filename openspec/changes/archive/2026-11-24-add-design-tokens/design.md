## Context

The design doc (https://claude.ai/artifact/VrkdfYN3jv2Y8KjZWtxshG) settled that tokens are a
convention on environment values, not a language feature, and the naming rules: singular lowercase
categories (`env.color`), role first then variant, t-shirt scales, `standardPalette.blue600`,
camelCase in NX and hyphens only in CSS. `add-environment-values` built the mechanism. This change
decides the token set and how libraries build on it.

The API was drafted as NX source in this change, so the compiler checked it rather than a
description. It is now the library itself, in `crates/nx-program/src/std/design/`:

- `types.nx`: the value types.
- `palette.nx`: the standard palette.
- `tokens.nx`: the semantic tokens and their defaults.

The examples became two conformance corpus programs. `specs/ir-conformance/design-tokens/` checks
the default theme: the roles in each mode, the contrast targets, the scales, a text role following
`env.font`, a component token following the core, and the palette's gamut. Its `ui.nx` is a UI
library's component tokens and an extra role. `specs/ir-conformance/design-tokens-theme/` is a brand
theme that varies by mode (`brand.nx`) applied by a main module with one dark section (`main.nx`).
With the brand applied, the button is green 700 (`oklch(0.527 0.154 150.069)`) in light mode and
green 500 (`oklch(0.723 0.219 149.579)`) inside `env.colorMode=dark`, with the brand's pill radius.

## Goals / Non-Goals

**Goals:**
- A semantic tier small enough that a third-party theme can fill it and a library author can learn
  it in one page.
- Defaults that make an unthemed program look reasonable in both color modes.
- One way to extend the core that never makes two libraries collide.
- A clear line between changes that are breaking and changes that are not, before anyone depends
  on it.

**Non-Goals:**
- Component tokens for any particular control. Those belong to the UI library that draws it.
- Token export and import (`add-token-export`), and Visual's value suggestions
  (`add-visual-value-suggestions`).
- Units on dimensions. Layout units stay unitless, as in `@nx/graphics`; px versus rem is the
  host's output step.

## Decisions

### Three tiers, with only the middle one standard

- **Reference** (`standardPalette`) is an ordinary exported `let`. It is in the library because the
  default theme needs colors from somewhere, not because themes must use it.
- **Semantic** (`env.color`, `env.space`, …) is the standard: these names are what every library
  reads and every theme sets.
- **Component** tokens are each library's own.

*Alternative:* standardize component tokens too, as Fluent does. Rejected: it couples the standard
to one library's set of controls, and it is what makes Fluent's token list hard to retheme by hand.

### The palette is Tailwind CSS v4's, in OKLCH

`standardPalette` holds five hues (zinc as `gray`, `blue`, `red`, `amber` and `green`) at eleven
steps, 50 to 950, with Tailwind CSS v4's values written as `c.oklch(…)`, the form Tailwind
publishes. Tailwind is MIT licensed; the library header carries its notice.

Many of the vivid steps are outside sRGB but inside Display P3. A Display P3 screen shows them as
written. An sRGB screen, and today's Skia and Canvas renderers, show them mapped with `toGamut`,
which keeps lightness and hue and reduces chroma, so they look as Tailwind v4's colors look on an
sRGB screen. The grays are inside sRGB. Every palette color is inside Display P3, so a P3 screen
never maps.

Each hue's steps follow the convention Radix Colors names, and the reference page documents it:
- 50 to 100: backgrounds and muted fills.
- 200 to 300: borders and dividers.
- 500 to 600: solid fills and accents.
- 500 to 950 for text on light backgrounds, and 50 to 400 on dark.

*Alternatives:* Tailwind v3's sRGB hex values, which every sRGB screen shows the same but which
cannot use a P3 screen's range; Radix Colors, whose 12 steps per hue have named uses and a P3 set,
but are licensed MIT too and need two scales per hue for light and dark; and generating a palette
from a few OKLCH parameters, which is a follow-up (see Follow-ups). Tailwind v4 is the palette most
developers and AI models already know by step name.

### Default roles meet contrast targets

The default theme's role pairs meet WCAG 2 contrast in both modes, measured with `contrastRatio`,
which maps both colors into sRGB first so the result holds on any screen:
- each `on…` role is at least 4.5:1 against its role;
- `text.normal` and `text.muted` are at least 4.5:1 against `background` and `surface`;
- `focus` is at least 3:1 against `background` and `surface`.

Measured on the v4 values, the lowest are light `focus` on white (3.8:1), light `onDanger` on red
600 (4.8:1) and light `text.muted` on white (4.8:1). `text.disabled` is exempt, as WCAG exempts
inactive controls.

`border` has no target. It is for dividers and control outlines that sit beside other cues, as in
Tailwind and shadcn/ui, and a 3:1 border would be zinc 500 in light mode, which reads as heavy. A
control whose outline alone identifies it uses a component token with a stronger color.

### Extension is composition: new environment values that read the core

A library declares `environment button: ButtonTokens = <ButtonTokens fill={env.color.accent} … />`.
Defaults are evaluated where they are read, so an override of `env.color.accent`, from a theme or
an `env.` attribute, reaches buttons with no extra wiring. A theme can still set `env.button.radius`
to tune just this library. A role the core lacks, such as a selection color, goes in the library's
own value (`env.uiColor.selection`) with a default derived from a core role.

*Alternative:* inheritance, `type UIColors extends SemanticColors { selection: Color }`. Rejected:
- `env.color` keeps its declared type, so a read through it cannot see `selection`. The library
  needs its own value anyway.
- Two libraries extending the same core type would need one value that satisfies both.
- Inheritance gives no way to set a field for one library only.

### The core's contents

The roles are the intersection of the systems surveyed, at about shadcn/ui's size:
- **Color:**
  - `background`, `surface` and `surfaceMuted`.
  - `text.normal`, `text.muted` and `text.disabled`, nested under `text` because there are three or
    more variants.
  - `border` and `focus`.
  - `accent`, `danger`, `warning` and `success`, each with an `on…` role for content drawn on it.
- **Size:** `space` (`none`, `xxs` to `xxxl`), `radius` (`none`, `sm` to `xl`, `full`), and
  `borderWidth` (`thin`, `thick`).
- **Type:** `font` holds the scales (`family.sans` and `family.mono`, `size.xs` to `size.xxxl`, and
  the weights `regular`, `medium`, `semibold` and `bold`). `text` holds seven roles built from it
  (`caption`, `body`, `label`, `title`, `heading`, `display` and `code`). Changing `env.font.size.md`
  restyles every role that uses it.
- **Depth and motion:** `shadow` (`sm`, `md` and `lg`, darker in dark mode), and `motion`
  (`duration.fast`, `normal` and `slow`, and the easings `standard`, `enter` and `exit`).

`accent` rather than Material's `primary`, as the design doc settled, because `primary` reads as
"most important" in more contexts than color.
This is the platforms' name for the role: Apple's accent and tint color, Windows'
`SystemAccentColor`, CSS's `accent-color` and Radix Themes' `accentColor` all mean the main action
and selection color. Web component libraries more often say `primary` (Material, MUI, Bootstrap,
shadcn/ui, Ant Design), and Fluent 2, Atlassian and Polaris say `brand`. The reference page has a
short "what other systems call this" note, and a table mapping shadcn/ui's names, the set most
developers and AI models know:

| shadcn/ui | `@nx/design` |
|---|---|
| `background`, `foreground` | `background`, `text.normal` |
| `card`, `popover` and their `-foreground` | `surface`, with `text.normal` |
| `muted`, `muted-foreground` | `surfaceMuted`, `text.muted` |
| `primary`, `primary-foreground` | `accent`, `onAccent` |
| `destructive`, `destructive-foreground` | `danger`, `onDanger` |
| `border` | `border` |
| `ring` | `focus` |
| `secondary`, `secondary-foreground` | none: a component token |
| `accent`, `accent-foreground` (the quiet hover fill) | none: a component token |
| `input` | none: inputs use `border` |
| none | `warning`, `success` and their `on…` roles, `text.disabled` |

`focus` is its own role with a fixed default (blue 500), not a reading of `accent`, so a theme can
make the ring differ from the accent. The cost is that a theme that changes `accent` must set
`focus` too, or the ring stays blue; the corpus brand theme does, and the reference page says so.

### Modes are environment values that defaults match on

`env.colorMode: ColorMode = light | dark`. `env.color` and `env.shadow` are
`if env.colorMode is { light => … dark => … }`. A theme bundle can do the same, and because a
main-module provide is evaluated where it is read, the theme follows `env.colorMode` overrides.

### Stability

`@nx/design` starts `unstable`, as `standard-libraries` requires of a new library. Once it is
`stable`, these are breaking: removing or renaming a role, scale step, field or type; changing a
field's type or narrowing its constraint; changing a default's value in a way the reference page
documents. These are not breaking: adding a role, step or field with a default; widening a
constraint; changing a default's exact value where it is not documented. A role added later gets a
field default, so code that constructs a whole `<SemanticColors …/>` keeps compiling. Themes still
set roles through `.Update` records, so that they name only what they change and pick up a new
role's default value rather than restating every role; the reference page says so.

### Value types match DTCG

Each type maps to one DTCG 2025.10 type, so export can map them directly:
- `Color` is `color`.
- `Spacing`, `Radius`, `BorderWidth` and `FontSize` are `dimension`.
- `LineHeight` and `Opacity` are `number`.
- `FontWeight` is `fontWeight` and `FontFamily` is `fontFamily`.
- `Duration` is `duration` and `Easing` is `cubicBezier`.
- `Shadow` is `shadow` and `TextStyle` is `typography`.

They are aliases, so `padding=12` stays valid and only tools see the difference.

### One design across platforms

The tokens are platform-neutral, as Material's, Fluent's and DTCG's are: they name a value, and each
UI library maps it to its platform, whether HTML, drawn UI or a native toolkit. A value that needs
to differ by platform is an ordinary override, not a separate mechanism: the platform's UI library
documents it, and the host sets it as a root (`--env`) or the main module provides it. These are
the values most likely to need one:
- **Units.** `Spacing`, `Radius`, `BorderWidth` and `FontSize` are layout units: CSS pixels on the
  web, dp on Android, points on Apple platforms, and device-independent pixels on Windows, each
  about 1/96 inch at standard density. The renderer scales them to the screen; the tokens carry no
  unit. Export writes `px`, and may write `rem` for font sizes.
- **Font families.** The defaults are the generic names `sans-serif` and `monospace`, which every
  renderer resolves to a system font. A platform library may provide the platform's own face
  (Segoe UI, SF Pro, Roboto). A theme that names a specific font must make sure it is installed or
  bundled where it runs; the tokens don't load fonts.
- **Shadows.** `Shadow` follows CSS `box-shadow`: offset, blur, spread and color. Skia's and
  Canvas's blur sigma is half the CSS blur, so a renderer for those converts it. (No NX renderer
  draws shadows yet.) Toolkits without spread, or with elevation instead of shadows (Android),
  approximate the shadow, so a shadow may look slightly different there.

Two more are left to later work: density (touch screens need larger targets; see Open Questions
on a `Density` mode), and reduced motion, which is a host input that `env.motion` could match on.

## Risks / Trade-offs

- **The core grows by accretion.** → The rule that a role moves into the core only when two
  libraries need it, and the reference page says so.
- **A neutral default theme is still a look.** Programs that never theme will look alike. →
  Acceptable: the defaults exist so that an unthemed program is legible, and theming is one
  `provide`.
- **`standardPalette` values copied from Tailwind CSS v4 (MIT).** → The library header carries the
  MIT notice. If Tailwind changes its values, ours stay as they are; they are a snapshot, not a
  link.
- **Vivid steps look different on sRGB and P3 screens.** → That is the intent of a wide palette, and
  `toGamut` mapping keeps hue and lightness, so a design reads the same on both. The contrast targets
  are measured after mapping into sRGB, so they hold on either.
- **`text.muted` on `surfaceMuted` is 4.4:1 in light mode**, below 4.5:1. → Not a target pair: muted
  text is for `background` and `surface`. A library that puts placeholder text on `surfaceMuted`
  uses `text.normal` or its own token.
- **`accent` means something else in shadcn/ui**, where it is the quiet hover fill, and people from
  Material, Bootstrap or shadcn/ui look for `primary`. → The reference page's naming note and
  mapping table; the `@nx/design` name follows the platforms and CSS.
- **A theme that changes `accent` but not `focus` keeps a blue focus ring.** → The reference page
  and the tutorial's brand theme set both.
- **Reading a token is expensive today.** One `env.color` read costs about 28,000 operations and
  one `env.button.radius` read about 232,000 in the brand corpus program, because the runtime
  evaluates a default at every read, evaluates a whole record default to read one field, and builds
  every top-level value, such as `standardPalette`'s 55 colors, each time it is read. → Recorded in
  `specs/future.md` with memoizing defaults and caching top-level values, which change the cost
  model and are their own change.
- **Fourteen color roles may still be too few for a full UI library** (no hover or pressed states).
  → State colors stay component tokens, derived with `@nx/color` operations such as `darken`. These
  work in OKLCH, so derived states keep their hue and stay in the wide gamut.

## Migration Plan

None. Nothing uses `@nx/design` yet. The old tutorial's hand-rolled `Theme` record is replaced.

## Open Questions

- **More modes.** Add `highContrast` to `ColorMode`, and a `Density = comfortable | compact` mode
  that `env.space` matches on? I'd leave both out until a library needs them.
- **Graphics adopting the types.** Type `@nx/graphics` properties such as `Text.fontSize` and
  `Stroke.width` with the aliases, which value suggestions need. That makes `@nx/graphics` depend on
  `@nx/design`. I'd do it in `add-visual-value-suggestions`, where it is used.
- **Hover and pressed states.** These stay component tokens for now. Should the core add
  `accentHover`, or a standard way to derive state colors?

## Follow-ups

- **A palette generator.** `paletteFromHue(…)` building eleven steps from an OKLCH lightness curve
  and a hue, mapped with `toGamut(…, displayP3)`, so a brand can make a hue that matches the
  standard steps.
- **Contrast-target tokens.** Roles defined by a target ratio against another role, as Adobe's
  Leonardo does, so a theme that changes `background` keeps its text legible.
- **Token export.** `add-token-export` writes DTCG `color` values with `colorSpace: "oklch"` and
  CSS `oklch()`, so the wide values survive into other tools.
