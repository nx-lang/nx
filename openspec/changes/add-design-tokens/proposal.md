## Why

`add-environment-values` (#74) gave NX the mechanism for design-system tokens: typed values with a
default that any body reads with `env.`, any subtree overrides, and the main module or the host sets
app-wide. What NX still lacks is the token set itself. Without one, every UI library invents its own
names for "the accent color" and "medium spacing", a theme written for one library does nothing for
another, and tools such as Visual's value suggestions and token export have nothing standard to
offer.

Other systems agree on the shape but not on the names. DTCG 2025.10 standardizes only the value
types and the file format. Material 3, Fluent 2, Carbon, Spectrum and Primer each define their own
names, in three tiers: reference tokens (raw palettes and scales), semantic tokens (roles such as
`accent` and `surface`), and component tokens (`button.radius`) that default to semantic ones. The
systems that are easiest to retheme, such as shadcn/ui and Radix Themes, keep the semantic tier
small, which is what lets third parties target it.

This change gives NX that semantic tier as a standard library, `@nx/design`, small and meant to stay
stable, and the convention by which UI libraries extend it.

## What Changes

- **A new standard library, `@nx/design`**, depending only on `@nx/color` and the prelude, with
  three parts:
  - **Value types**: type aliases with constraints for DTCG's value types: `Spacing`, `Radius`,
    `BorderWidth`, `FontSize`, `LineHeight`, `FontWeight`, `FontFamily`, `Opacity`, `Duration`,
    plus records `Easing`, `Shadow` and `TextStyle`, and `Color` re-exported from `@nx/color`.
  - **Reference tokens**: `palette`, five hues (gray, blue, red, amber, green) in eleven steps,
    `palette.blue600`, with Tailwind CSS v4's OKLCH values (MIT). The vivid steps use a Display P3
    screen's range and map into sRGB elsewhere. Nothing requires it; it is what the default theme
    is built from.
  - **Semantic tokens**: the core set, as environment values: `env.colorMode`, `env.color` (14
    roles: background, surface, surfaceMuted, text.normal / muted / disabled, border, focus, accent
    and onAccent, danger, warning and success with their `on…` roles), `env.space`, `env.radius`,
    `env.borderWidth`, `env.font` (families, sizes, weights), `env.text` (seven text roles:
    caption, body, label, title, heading, display, code), `env.shadow` and `env.motion`. Their
    defaults are a neutral light and dark theme, so a program that imports the library looks
    reasonable with no setup, and meet WCAG contrast targets in both modes.
- **Extension by composition, not inheritance.** A UI library adds its own component tokens as
  environment values whose defaults read the core (`environment button: ButtonTokens =
  <ButtonTokens fill={env.color.accent} … />`), and a role the core lacks in its own namespace,
  derived from a core role. A role moves into the core when two libraries need it.
- **Themes are `env.Update` bundles.** A theme that sets only core roles restyles every library; it
  can also set a library's component tokens to tune that library alone. A theme can vary by mode
  with `if env.colorMode is { … }`.
- **A stability contract for the core.** `@nx/design` starts `unstable`, as every new standard
  library does. The design records what will count as breaking once it is `stable`: removing or
  renaming a role, a scale step or a field, or changing a type. Adding a role is not breaking for
  readers, and is breaking only for code that constructs a whole `SemanticColors`, which is why
  themes use `.Update` records.
- **Documentation.** A reference page for `@nx/design`, the token convention and naming rules in the
  style guide, and the "Working with Design Tokens" tutorial rewritten around the library.

## Capabilities

### New Capabilities

- `design-tokens`: the `@nx/design` standard library (value types, palette, semantic environment
  values and their defaults), the rule for extending it, themes as bundles, and its stability
  contract.

### Modified Capabilities

None. `@nx/design` is a standard library under the existing `standard-libraries` requirements.

## Impact

- **Compiler**: one new standard library in `crates/nx-program/src/std/design/`, registered beside
  `@nx/color`. No language, IR or runtime change.
- **Tests**: the library's own diagnostics-free check, a conformance corpus program that reads the
  tokens in both modes and through a theme, and the example extension.
- **Docs**: `reference/libraries/design.md`, the style guide's token section, and the tutorial.
- **Follow-ups**: `add-visual-value-suggestions` offers these tokens first in Visual;
  `add-token-export` writes them as CSS custom properties and DTCG.
