---
title: 'Working with Design Tokens'
description: 'Incorporate design tokens into NX workflows.'
---

Design tokens in NX are just typed data. This tutorial shows how to declare tokens, pass them through components, and enforce usage with the type system. It builds on the [Language Tour](/language-tour/elements) and the [Building Your First Component](/tutorials/building-your-first-component) tutorial.

## 1) Define token types
Create `examples/nx/tokens.nx`:

```nx
type <ColorToken name:string value:string/>
type <SpaceToken name:string value:int/>
type <FontToken name:string family:string size:int weight:int/>

type <Theme
  primary:ColorToken
  surface:ColorToken
  text:ColorToken
  space:SpaceToken[]
  fonts:FontToken[]/>
```

- Tokens are strongly typed; you can constrain units (`int` for spacing) and required fields.
- Group tokens into a `Theme` to pass around as a single object.

## 2) Create a base theme

```nx
let baseTheme: Theme =
  <Theme
    primary=<ColorToken name="primary" value="#5B6EF5"/>
    surface=<ColorToken name="surface" value="#0B0C10"/>
    text=<ColorToken name="text" value="#F2F4F8"/>
    space={[
      <SpaceToken name="xs" value=4/>,
      <SpaceToken name="sm" value=8/>,
      <SpaceToken name="md" value=12/>,
      <SpaceToken name="lg" value=16/>
    ]}
    fonts={[
      <FontToken name="body" family="Inter" size=16 weight=400/>,
      <FontToken name="heading" family="Inter" size=24 weight=700/>
    ]}/>
```

## 3) Consume tokens in components
Wire tokens into layout and styling instead of raw strings:

```nx
let <Panel theme:Theme title:string> content:Element </Panel> =
  let spacing = theme.space[2].value  // "md" spacing
  <section
    style=<Style
      backgroundColor={theme.surface.value}
      color={theme.text.value}
      padding={`${spacing}px`}
      fontFamily={theme.fonts[0].family}
    />>
    <h2>{title}</h2>
    <div>{content}</div>
  </section>
```

- Because the theme is typed, you can’t accidentally pass an unknown token name.
- Interpolation works in attributes (`padding={`${spacing}px`}`) just like other expressions.

## 4) Swap themes without changing components

```nx
let <App theme:Theme/> =
  <Panel theme={theme} title="Design Tokens">
    <p>Tokens flow through the component tree.</p>
  </Panel>

<App theme={baseTheme}/>
```

Drop in an alternate `Theme` instance to re-skin the UI without touching component code.

## 5) Validate usage
- Run `cargo test --workspace` to ensure changes build.
- Use `tree-sitter parse` (from `crates/nx-syntax`) on `examples/nx/tokens.nx` to confirm syntax if you have the CLI.
- When you pass tokens through props, the type checker flags missing or mistyped fields—no separate JSON/YAML schema required.

## 6) Extend the pattern
- Add semantic tokens (e.g., `info`, `warning`, `error`) that map to base colors.
- Create a `Button` component that derives its `tone` from `Theme.primary`.
- Export themes from a dedicated module and import them across apps or design tools.
