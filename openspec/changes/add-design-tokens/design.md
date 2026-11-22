## Context

The design doc (https://claude.ai/artifact/VrkdfYN3jv2Y8KjZWtxshG) settled that tokens are a
convention on environment values, not a language feature, and the naming rules: singular lowercase
categories (`env.color`), role first then variant, t-shirt scales, `palette.blue600`, camelCase in
NX and hyphens only in CSS. `add-environment-values` built the mechanism. This change decides the
token set and how libraries build on it.

The proposed API is written as NX source in this change, so it is checked by the compiler rather
than described:

- `api/types.nx`: the value types.
- `api/palette.nx`: the reference palette.
- `api/tokens.nx`: the semantic tokens and their defaults.
- `examples/ui.nx`: a UI library's component tokens and an extra role.
- `examples/brand.nx`: a theme that varies by mode.
- `examples/app.nx`: a main module applying the theme, with one dark section.

They compile, and evaluate in the TypeScript IR runtime. With the brand applied, the example button
is `#15803d` (green 700) in light mode and `#22c55e` (green 500) inside `env.colorMode=dark`, with
the brand's pill radius. At implementation the `api/` files become
`crates/nx-program/src/std/design/` and the examples become a conformance corpus program.

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

- **Reference** (`palette`) is an ordinary exported `let`. It is in the library because the default
  theme needs colors from somewhere, not because themes must use it.
- **Semantic** (`env.color`, `env.space`, …) is the standard: these names are what every library
  reads and every theme sets.
- **Component** tokens are each library's own.

*Alternative:* standardize component tokens too, as Fluent does. Rejected: it couples the standard
to one library's set of controls, and it is what makes Fluent's token list hard to retheme by hand.

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

### Modes are environment values that defaults match on

`env.colorMode: ColorMode = light | dark`. `env.color` and `env.shadow` are
`if env.colorMode is { light => … dark => … }`. A theme bundle can do the same, and because a
main-module provide is evaluated where it is read, the theme follows `env.colorMode` overrides.

### Stability

`@nx/design` starts `unstable`, as `standard-libraries` requires of a new library. Once it is
`stable`, these are breaking: removing or renaming a role, scale step, field or type; changing a
field's type or narrowing its constraint; changing a default's value in a way the reference page
documents. These are not breaking: adding a role, step or field with a default; widening a
constraint; changing a default's exact value where it is not documented. Because adding a field
breaks code that constructs a whole `<SemanticColors …/>`, themes set roles through `.Update`
records, and the reference page says so.

### Value types match DTCG

Each type maps to one DTCG 2025.10 type, so export can map them directly:
- `Color` is `color`.
- `Spacing`, `Radius`, `BorderWidth` and `FontSize` are `dimension`.
- `LineHeight` and `Opacity` are `number`.
- `FontWeight` is `fontWeight` and `FontFamily` is `fontFamily`.
- `Duration` is `duration` and `Easing` is `cubicBezier`.
- `Shadow` is `shadow` and `TextStyle` is `typography`.

They are aliases, so `padding=12` stays valid and only tools see the difference.

## Risks / Trade-offs

- **The core grows by accretion.** → The rule that a role moves into the core only when two
  libraries need it, and the reference page says so.
- **A neutral default theme is still a look.** Programs that never theme will look alike. →
  Acceptable: the defaults exist so that an unthemed program is legible, and theming is one
  `provide`.
- **`palette` values copied from Tailwind CSS v3 (MIT).** → See Open Questions.
- **Fourteen color roles may still be too few for a full UI library** (no hover or pressed states).
  → State colors stay component tokens, derived with `@nx/color` operations such as `darken`.

## Migration Plan

None. Nothing uses `@nx/design` yet. The old tutorial's hand-rolled `Theme` record is replaced.

## Open Questions

- **Palette source.** The palette values are Tailwind CSS v3's, which is MIT licensed. Keep them
  with an attribution line in the library header, or pick NX's own values?
- **More modes.** Add `highContrast` to `ColorMode`, and a `Density = comfortable | compact` mode
  that `env.space` matches on? I'd leave both out until a library needs them.
- **Graphics adopting the types.** Type `@nx/graphics` properties such as `Text.fontSize` and
  `Stroke.width` with the aliases, which value suggestions need. That makes `@nx/graphics` depend on
  `@nx/design`. I'd do it in `add-visual-value-suggestions`, where it is used.
- **Hover and pressed states.** These stay component tokens for now. Should the core add
  `accentHover`, or a standard way to derive state colors?
