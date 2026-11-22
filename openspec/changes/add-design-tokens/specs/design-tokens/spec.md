## ADDED Requirements

### Requirement: `@nx/design` is an unstable standard library of token types and tokens
The system SHALL provide a standard library `@nx/design`, imported as `import "@nx/design"`, that
depends only on `@nx/color` and the prelude, is free of diagnostics, and states in its source header
and reference page that it is `unstable`. It SHALL export the value types `Color` (the `@nx/color`
type), `Spacing`, `Radius`, `BorderWidth`, `FontSize`, `LineHeight`, `FontWeight`, `FontFamily`,
`Opacity` and `Duration` as constrained aliases, and the records `Easing`, `Shadow` and
`TextStyle`. Each alias SHALL stay transparent to type checking, so a value of its base type is
accepted where it is expected.

#### Scenario: A spacing value accepts a plain number
- **WHEN** a module imports `@nx/design` and declares `let gap: Spacing = 12`
- **THEN** analysis SHALL accept it

#### Scenario: A constraint is enforced
- **WHEN** a module declares `let weight: FontWeight = 0`
- **THEN** analysis SHALL report the value as outside the type's constraint

### Requirement: The library provides a reference palette
`@nx/design` SHALL export `palette: Palette`, a record of five hues, `gray`, `blue`, `red`, `amber`
and `green`, each in the steps 50, 100, 200 and so on up to 900, then 950, named hue then step
(`palette.blue600`). No other declaration of the library SHALL require a program to use the
palette.

#### Scenario: A palette step is read by name
- **WHEN** a module evaluates `palette.blue600`
- **THEN** the result SHALL be a `Color`

### Requirement: The semantic tokens are environment values with a usable default theme
`@nx/design` SHALL export these environment values:
- `colorMode: ColorMode`, where `ColorMode` is `light | dark` and the default is `light`.
- `color: SemanticColors` with the roles `background`, `surface`, `surfaceMuted`, `text` (a
  `TextColors` with `normal`, `muted` and `disabled`), `border`, `focus`, `accent`, `onAccent`,
  `danger`, `onDanger`, `warning`, `onWarning`, `success` and `onSuccess`.
- `space: SpaceScale` with the steps `none`, `xxs`, `xs`, `sm`, `md`, `lg`, `xl`, `xxl` and `xxxl`.
- `radius: RadiusScale` with the steps `none`, `sm`, `md`, `lg`, `xl` and `full`.
- `borderWidth: BorderWidthScale` with the steps `thin` and `thick`.
- `font: Fonts` with `family` (`sans`, `mono`), `size` (`xs` to `xxxl`) and `weight` (`regular`,
  `medium`, `semibold`, `bold`).
- `text: TextStyles` with the roles `caption`, `body`, `label`, `title`, `heading`, `display` and
  `code`.
- `shadow: ShadowScale` with the steps `sm`, `md` and `lg`.
- `motion: Motion` with `duration` (`fast`, `normal`, `slow`) and `easing` (`standard`, `enter`,
  `exit`).

Every scale SHALL be nondecreasing from its smallest step to its largest. The defaults of `color`
and `shadow` SHALL depend on `env.colorMode` and give a light and a dark theme in which each `on…`
role, and each `text` role except `disabled` drawn on `background` and `surface`, has a contrast
ratio of at least 4.5 against the role it is drawn on. The defaults of `text` SHALL be built from
`env.font`, so an override of a font scale reaches every text role that uses it.

#### Scenario: The color roles follow the color mode
- **WHEN** a module evaluates `env.color.background`, and `provide env.colorMode=dark { env.color.background }`
- **THEN** the first SHALL be a light color and the second a dark one

#### Scenario: A text role follows the font scale
- **WHEN** a module evaluates `provide env.font.size.md=20 { env.text.body.size }`
- **THEN** the result SHALL be `20`

#### Scenario: The default theme is legible
- **WHEN** the contrast ratio of `env.color.onAccent` against `env.color.accent` is computed in each
  color mode with `@nx/color`'s `contrastRatio`
- **THEN** each SHALL be at least 4.5

### Requirement: Libraries extend the core with environment values of their own
The library's reference page and the style guide SHALL define these extension rules for a UI
library:
- **Component tokens:** the library declares them as its own environment values, whose defaults
  read the core tokens.
- **Missing roles:** a role the core lacks goes in an environment value of the library's own, with a
  default derived from a core role.
- **No subtyping:** a library does not extend the core's record types to add roles.

A conformance program SHALL show a library's component token following an override of the core
token its default reads.

#### Scenario: A component token follows the core
- **WHEN** a library declares `environment button: ButtonTokens = <ButtonTokens fill={env.color.accent} />`
- **AND** a body reads `env.button.fill` inside `provide env.color.accent=#15803d { … }`
- **THEN** the read SHALL give `#15803d`

#### Scenario: A theme tunes one library
- **WHEN** the main module applies a bundle that sets `button=<ButtonTokens.Update radius=9999 />`
- **THEN** `env.button.radius` SHALL be `9999` and `env.radius.md` SHALL keep its default

### Requirement: Themes are environment bundles
A theme SHALL be an `env.Update` value that sets core roles through their `.Update` records, applied
app-wide with `provide theme` in the main module or around a subtree. A theme that varies by mode
SHALL do so by matching on `env.colorMode` in the bundle's fields, and SHALL follow an
`env.colorMode` override where it is read.

#### Scenario: A theme varies by mode
- **WHEN** the main module applies a bundle whose `color` field is
  `if env.colorMode is { light => <SemanticColors.Update accent={palette.green700} /> dark => <SemanticColors.Update accent={palette.green500} /> }`
- **THEN** `env.color.accent` SHALL be `palette.green700` at the root and `palette.green500` inside
  an element with `env.colorMode=dark`
- **AND** `env.color.background` SHALL keep each mode's default

### Requirement: The core changes compatibly once stable
When `@nx/design` becomes `stable`, the library SHALL treat these as breaking changes:
- removing or renaming a token, role, scale step, field or type;
- changing a field's type, or narrowing a constraint;
- changing a default value that the reference page documents.

It SHALL treat these as compatible changes:
- adding a role, step or field that has a default;
- widening a constraint;
- changing a default value that the reference page does not document.

The reference page SHALL tell theme authors to set roles through `.Update` records, which adding a
field does not break.

#### Scenario: The reference page states the contract
- **WHEN** a reader opens the `@nx/design` reference page
- **THEN** it SHALL say that the library is unstable, list what will count as breaking once it is
  stable, and recommend `.Update` records for themes
