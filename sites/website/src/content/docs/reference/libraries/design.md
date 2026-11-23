---
title: '@nx/design'
description: 'Design tokens: value types, a standard palette, and the semantic tokens every NX UI library reads.'
---

`@nx/design` is unstable: its declarations may change incompatibly in any NX release. It holds the
foundations of a design system, as [environment values](/reference/syntax/environment): the
semantic tokens every UI library reads and every theme sets, the value types they use, and a
standard palette the default theme is built from. It depends only on `@nx/color`, and holds no
components; those belong to UI libraries, which read these tokens.

```nx
import "@nx/design"

external component <Box fill: Color radius: Radius padding: Spacing />

let card() = { <Box fill={env.color.surface} radius={env.radius.lg} padding={env.space.md} /> }
let root() = { card() }
```

The tokens are platform-neutral: they name values, and each UI library maps them to its platform,
whether HTML, drawn UI or a native toolkit.

## Value types

Each type maps to one [DTCG 2025.10](https://www.designtokens.org/) type, so tools can export
tokens directly. The aliases are transparent: `padding=12` is valid where a `Spacing` is expected,
and only tools see the difference.

| Type | Values | DTCG type |
| --- | --- | --- |
| `Color` | `@nx/color`'s `Color`, in any space | `color` |
| `Spacing`, `Radius`, `BorderWidth` | finite, at least 0, in layout units | `dimension` |
| `FontSize` | finite, above 0, in layout units | `dimension` |
| `LineHeight` | finite, above 0, a multiple of the font size | `number` |
| `Opacity` | 0 to 1 | `number` |
| `FontWeight` | `int` from 1 to 1000; 400 is regular, 700 bold | `fontWeight` |
| `FontFamily` | a family name, or `"sans-serif"`, `"serif"`, `"monospace"` | `fontFamily` |
| `Duration` | finite, at least 0, in milliseconds | `duration` |
| `Easing` | a cubic Bézier curve, `x1 y1 x2 y2`, as in CSS `cubic-bezier()` | `cubicBezier` |
| `Shadow` | `offsetX offsetY blur spread color`, as in CSS `box-shadow` | `shadow` |
| `TextStyle` | `family size weight lineHeight` | `typography` |

A layout unit is a CSS pixel on the web, a dp on Android, a point on Apple platforms and a
device-independent pixel on Windows: about 1/96 inch at standard density. The renderer scales it to
the screen.

```nx invalid
import "@nx/design"

let weight: FontWeight = 0
let root() = { weight }
```

## Semantic tokens

These environment values are the standard. Their defaults are a neutral light and dark theme, so a
program that imports the library looks reasonable with no setup.

| Token | Type | What it holds |
| --- | --- | --- |
| `env.colorMode` | `ColorMode` | `light` (the default) or `dark`. A host usually sets it from the system setting. |
| `env.color` | `SemanticColors` | The color roles, for the color mode in effect. |
| `env.space` | `SpaceScale` | Gaps, padding and margins. |
| `env.radius` | `RadiusScale` | Corner radii. |
| `env.borderWidth` | `BorderWidthScale` | Border and divider widths. |
| `env.font` | `Fonts` | Font families, sizes and weights. |
| `env.text` | `TextStyles` | Text roles built from `env.font`. |
| `env.shadow` | `ShadowScale` | Drop shadows, darker in dark mode. |
| `env.motion` | `Motion` | Animation durations and curves. |

### Color roles

The defaults come from the [standard palette](#the-standard-palette).

| Role | What it is for | Light | Dark |
| --- | --- | --- | --- |
| `background` | The base background of the program | white | gray 950 |
| `surface` | Raised content: cards, sheets, menus and dialogs | white | gray 900 |
| `surfaceMuted` | Quiet fills: inputs, table stripes and hovered rows | gray 100 | gray 800 |
| `text.normal` | Body text and icons | gray 900 | gray 50 |
| `text.muted` | Secondary text: captions, placeholders and metadata | gray 500 | gray 400 |
| `text.disabled` | The text of disabled controls | gray 400 | gray 600 |
| `border` | Dividers and the outlines of controls | gray 200 | gray 800 |
| `focus` | The focus ring | blue 500 | blue 400 |
| `accent` | The main action and selection color, usually the brand color | blue 600 | blue 400 |
| `onAccent` | Text and icons on `accent` | white | gray 950 |
| `danger` | Errors and destructive actions | red 600 | red 500 |
| `onDanger` | Text and icons on `danger` | white | gray 950 |
| `warning` | Warnings and caution | amber 500 | amber 400 |
| `onWarning` | Text and icons on `warning` | gray 950 | gray 950 |
| `success` | Success and completion | green 700 | green 500 |
| `onSuccess` | Text and icons on `success` | white | gray 950 |

`focus` is its own role rather than a reading of `accent`, so a theme can make the ring differ from
the accent. **A theme that changes `accent` should set `focus` too**, or the ring stays blue.

#### Contrast

In both modes the default roles meet these WCAG 2 contrast ratios, measured with `@nx/color`'s
`contrastRatio`, which maps both colors into sRGB first so the result holds on any screen:

- each `on…` role is at least 4.5:1 against its role;
- `text.normal` and `text.muted` are at least 4.5:1 against `background` and `surface`;
- `focus` is at least 3:1 against `background` and `surface`.

`text.disabled` has no target, as WCAG exempts inactive controls. `border` has none either: like
Tailwind's and shadcn/ui's borders, it is for dividers and outlines that sit beside other cues, at
about 1.2:1. A control whose outline alone identifies it needs a stronger color, from a token of
the UI library's own.

#### What other systems call these

`accent` is the platforms' name for the main action and selection color: Apple's accent and tint
color, Windows' `SystemAccentColor`, CSS's `accent-color` and Radix Themes' `accentColor`. Web
component libraries more often say `primary` (Material, MUI, Bootstrap, shadcn/ui, Ant Design), and
Fluent 2, Atlassian and Polaris say `brand`. In shadcn/ui, `accent` is something else: the quiet
fill behind a hovered menu item. Here is how shadcn/ui's color names map to these:

| shadcn/ui | `@nx/design` |
| --- | --- |
| `background`, `foreground` | `background`, `text.normal` |
| `card`, `popover` and their `-foreground` | `surface`, with `text.normal` |
| `muted`, `muted-foreground` | `surfaceMuted`, `text.muted` |
| `primary`, `primary-foreground` | `accent`, `onAccent` |
| `destructive`, `destructive-foreground` | `danger`, `onDanger` |
| `border` | `border` |
| `ring` | `focus` |
| `secondary`, `secondary-foreground` | none: a UI library's own token |
| `accent`, `accent-foreground` | none: a UI library's own token |
| `input` | none: inputs use `border` |
| none | `warning`, `success` and their `on…` roles, `text.disabled` |

### Sizes

| Scale | Steps and defaults |
| --- | --- |
| `env.space` | `none` 0, `xxs` 2, `xs` 4, `sm` 8, `md` 12, `lg` 16, `xl` 24, `xxl` 32, `xxxl` 48 |
| `env.radius` | `none` 0, `sm` 2, `md` 4, `lg` 8, `xl` 12, `full` 9999 |
| `env.borderWidth` | `thin` 1, `thick` 2 |

Every scale is nondecreasing from its smallest step to its largest.

### Type

`env.font` holds the scales: `family.sans` (`"sans-serif"`) and `family.mono` (`"monospace"`),
`size.xs` to `size.xxxl` (12, 14, 16, 18, 20, 24, 30), and the weights `regular` 400, `medium`
500, `semibold` 600 and `bold` 700. `env.text` holds seven roles built from them:

| Role | Family | Size | Weight | Line height |
| --- | --- | --- | --- | --- |
| `caption` | sans | `xs` | regular | 1.4 |
| `body` | sans | `md` | regular | 1.5 |
| `label` | sans | `sm` | medium | 1.4 |
| `title` | sans | `lg` | semibold | 1.4 |
| `heading` | sans | `xxl` | semibold | 1.3 |
| `display` | sans | `xxxl` | bold | 1.2 |
| `code` | mono | `sm` | regular | 1.5 |

Because the roles read `env.font`, changing a font scale restyles every role that uses it:

```nx
import "@nx/design"

let root() = { provide env.font.size.md=18 { env.text.body.size } }
```

```nx output
18
```

### Depth and motion

`env.shadow` has `sm` (controls), `md` (cards and menus) and `lg` (dialogs and popovers), with the
same offsets in both modes and darker colors in dark mode. `env.motion` has the durations `fast`
100, `normal` 200 and `slow` 300 milliseconds, and the easings `standard`, `enter` and `exit`.

## The standard palette

`standardPalette` holds five hues, `gray` (Tailwind's zinc), `blue`, `red`, `amber` and `green`,
each in eleven steps from 50 to 950, named hue then step: `standardPalette.blue600`. The values are
[Tailwind CSS](https://tailwindcss.com) v4's (MIT License, Copyright (c) Tailwind Labs, Inc.),
written as `Color.oklch`. Nothing requires it: a theme can fill the roles from any colors, such as
a brand's own ramp.

The steps follow a common convention:

- 50 to 100: backgrounds and muted fills;
- 200 to 300: borders and dividers;
- 500 to 600: solid fills and accents;
- 500 to 950 for text on light backgrounds, and 50 to 400 on dark.

Many of the vivid steps are outside sRGB but inside Display P3. A Display P3 screen shows them as
written. An sRGB screen, and today's Skia and Canvas renderers, show them mapped with `toGamut`,
which keeps lightness and hue and reduces chroma. The grays are inside sRGB, and every palette color
is inside Display P3.

## Extending the core

A UI library builds on the core by composition:

- **Component tokens:** the library declares them as its own environment values, whose defaults
  read the core tokens. Defaults are evaluated where they are read, so a theme that changes
  `env.color.accent` restyles the library's buttons with no extra wiring, and a theme can still set
  `env.button.radius` to tune this library alone.
- **Missing roles:** a role the core lacks goes in an environment value of the library's own, with a
  default derived from a core role.
- **No subtyping:** a library does not extend the core's record types to add roles. `env.color`
  keeps its declared type, so a read through it could not see the new role, and two libraries could
  not both extend it.

A role moves into the core only when two libraries need it.

```nx
import "@nx/design"

/// What a button looks like.
export type ButtonTokens = {
  fill: Color        /// The background.
  text: Color        /// The label color.
  radius: Radius     /// The corner radius.
  label: TextStyle   /// The label's text style.
}

/// What a button looks like, derived from the core tokens.
export environment button: ButtonTokens = <ButtonTokens
  fill={env.color.accent}
  text={env.color.onAccent}
  radius={env.radius.md}
  label={env.text.label}
/>

/// A role the core lacks, derived from a core role.
export type UIColors = {
  selection: Color  /// The background of selected rows.
}

/// This library's extra color roles.
export environment uiColor: UIColors = {
  if env.colorMode is {
    light => <UIColors selection={env.color.surfaceMuted} />
    dark => <UIColors selection={env.color.border} />
  }
}

let root() = { provide env.color.accent=#15803d { env.button.fill } }
```

## Themes

A theme is an `env.Update` bundle that sets roles through their `.Update` records. Apply it
app-wide with `provide` at the top of the main module. A bundle can't be applied to one section
yet; a section changes single values with `env.` attributes, such as `env.colorMode=dark`.
A theme that varies by mode matches on `env.colorMode` in its fields, and follows an
`env.colorMode` override wherever it is read.

```nx
import "@nx/design"

/// Green accents, a lighter green in dark mode, and rounder corners.
let brand: env.Update = <env.Update
  color={
    if env.colorMode is {
      light => <SemanticColors.Update accent={standardPalette.green700} focus={standardPalette.green600} />
      dark => <SemanticColors.Update
        accent={standardPalette.green500}
        onAccent={standardPalette.gray950}
        focus={standardPalette.green400}
      />
    }
  }
  radius=<RadiusScale.Update md=8 lg=12 />
/>

provide brand

let root() = { provide env.colorMode=dark { env.color.accent } }
```

Set roles through `.Update` records rather than constructing a whole `<SemanticColors … />`: a theme
then names only what it changes, and picks up the default of a role the core adds later.

`text` is a nested group, and `SemanticColors.Update`'s `text` field takes a whole `TextColors`. Set
one text role with a path field instead, which keeps the others, beside the `color` field:

```nx
import "@nx/design"

let brand: env.Update = <env.Update
  color=<SemanticColors.Update accent={standardPalette.green700} focus={standardPalette.green600} />
  color.text={
    if env.colorMode is {
      light => <TextColors.Update muted={standardPalette.gray600} />
      dark => <TextColors.Update muted={standardPalette.gray300} />
    }
  }
/>

provide brand

let root() = { env.color.text.muted }
```

A theme that sets the same role in both modes can write `color.text.muted={standardPalette.gray600}`.

## Per-platform values

A value that needs to differ by platform is an ordinary override, set by the host as a root value
or provided by the main module, and documented by the platform's UI library. These are the ones
most likely to need one:

- **Font families.** The generic `sans-serif` and `monospace` resolve to a system font in every
  renderer. A platform library may provide the platform's own face (Segoe UI, SF Pro, Roboto). A
  theme that names a specific font must make sure it is installed or bundled where it runs.
- **Shadows.** `Shadow` follows CSS `box-shadow`. A Skia or Canvas renderer uses half the CSS blur
  as its blur sigma. (No NX renderer draws shadows yet.) Toolkits without spread, or with elevation
  instead of shadows, approximate the shadow.
- **Units.** Layout units carry no unit of their own; export writes `px`, and may write `rem` for
  font sizes.

## Generated code

Programs that import `@nx/design` can be generated as JavaScript, which reads tokens as the IR
runtimes do, with two gaps for now:

- A main-module `provide bundle` is evaluated once, when the generated module loads. A theme that
  matches on `env.colorMode` then keeps the root mode's values in a section that sets
  `env.colorMode=dark`; the IR runtimes evaluate it where it is read.
- Generated TypeScript doesn't type-check yet, because the library's `Color` names a type from
  `@nx/color` that the generated file doesn't import.

## Stability

`@nx/design` is unstable. Once it is stable, these will be breaking changes:

- removing or renaming a token, role, scale step, field or type;
- changing a field's type, or narrowing a constraint;
- changing a default value that this page documents.

These will not:

- adding a role, step or field that has a default;
- widening a constraint;
- changing a default value that this page does not document.
