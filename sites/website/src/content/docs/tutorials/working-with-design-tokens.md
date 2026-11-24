---
title: 'Working with Design Tokens'
description: 'Read the standard design tokens, build a component on them, and write a light and dark brand theme.'
---

Design tokens in NX are [environment values](/reference/syntax/environment) from the standard
library [`@nx/design`](/reference/libraries/design). A component reads `env.color.accent` or
`env.space.md` instead of a raw color or number, and a theme changes what those names hold. This
tutorial builds a button that reads the tokens, gives it tokens of its own, writes a brand theme
with a light and a dark mode, and applies it to a whole app with one section in dark mode. It builds
on the [Language Tour](/language-tour/elements) and the [Building Your First
Component](/tutorials/building-your-first-component) tutorial.

## 1) Read the tokens

Start a file named `app.nx`, or work in the [playground](/play). Import the library, and read a
token with `env.` and its name:

```nx
import "@nx/design"

external component <Box fill: Color radius: Radius padding: Spacing content Children?: Element+ />
external component <Text value: string color: Color style: TextStyle />

let greeting() = {
  <Box fill={env.color.surface} radius={env.radius.lg} padding={env.space.md}>
    <Text value="Hello" color={env.color.text.normal} style={env.text.body} />
  </Box>
}
```

- `Box` and `Text` stand for your host's drawing primitives. Their properties use the library's
  value types, `Color`, `Radius`, `Spacing` and `TextStyle`, which tell tools what kind of value
  each one takes.
- Every token has a default, so this already renders as a neutral light theme.
- A misspelled token is a compile error: `env.space.huge` lists the steps `SpaceScale` has.

## 2) Give a component tokens of its own

A UI library declares its components' tokens as environment values of its own, whose defaults read
the core tokens. Each block from here on continues the same file:

```nx fragment
/// What a button looks like.
export type ButtonTokens = {
  fill: Color       /// The background.
  text: Color       /// The label color.
  radius: Radius    /// The corner radius.
  padding: Spacing  /// The space around the label.
  label: TextStyle  /// The label's text style.
}

/// What a button looks like, derived from the core tokens.
export environment button: ButtonTokens = <ButtonTokens
  fill={env.color.accent}
  text={env.color.onAccent}
  radius={env.radius.md}
  padding={env.space.sm}
  label={env.text.label}
/>

/// A button, styled entirely from `env.button`.
export component <Button text: string /> = {
  <Box fill={env.button.fill} radius={env.button.radius} padding={env.button.padding}>
    <Text value={text} color={env.button.text} style={env.button.label} />
  </Box>
}
```

- A default is evaluated where it is read, so anything that changes `env.color.accent` changes the
  button's fill too, with no extra wiring.
- A theme can still set `env.button.radius` to change this library's buttons and nothing else.
- Don't add roles by extending `SemanticColors`. A role the core lacks goes in a value of the
  library's own, such as `env.uiColor.selection`, with a default that reads a core role.

## 3) Write a brand theme

A theme is an `env.Update` bundle. It sets roles through their `.Update` records, so it names only
what it changes. To differ by mode, match on `env.colorMode`:

```nx fragment
/// The brand: green accents, a lighter green in dark mode, and pill-shaped buttons.
export let brand: env.Update = <env.Update
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
  button=<ButtonTokens.Update radius=9999 />
/>
```

- `standardPalette` is the library's palette, Tailwind CSS v4's colors. A brand can use its own
  colors instead: any `Color` works.
- Set `focus` wherever you set `accent`, or the focus ring stays the default blue.
- The core roles `accent` and `radius` restyle every library that reads them; the `button` line
  tunes this one library.

## 4) Apply it app-wide, with one dark section

`provide env with brand` at the top of the main module applies the theme to the whole program. An
`env.` attribute changes a single value for one element and everything inside it. To apply a theme
to one section only, write `provide env with brand { … }` around it instead.

```nx fragment
provide env with brand

let root() = {
  <Box fill={env.color.background} radius=0 padding=0>
    <Button text="Save" />
    <Box env.colorMode=dark fill={env.color.background} radius=0 padding={env.space.lg}>
      <Button text="Save" />
    </Box>
  </Box>
}
```

The first button is green 700. The second sits in a dark section, so the theme's dark case applies
and it is green 500 with dark text, on the dark theme's background. The theme follows the mode
because it is evaluated where it is read. A host usually sets the mode for the whole program from
the system setting, as a root value of `env.colorMode`.

## 5) Check it

- Paste the whole file into the [playground](/play): it runs as you type and shows what `root`
  returns.
- The type checker flags a role that doesn't exist, a value of the wrong type, and a value outside
  its type's range, such as a negative `Spacing`.
- The default theme's text and `on…` colors meet WCAG contrast targets in both modes. A theme that
  changes them can check its own with `@nx/color`'s `contrastRatio`.

## Next steps

- Read the [`@nx/design` reference](/reference/libraries/design) for every token and its default.
- Follow the [style guide's token rules](/reference/style-guide#design-tokens) when naming your
  own.
- Move the button and its tokens into a library directory and import them across apps.
