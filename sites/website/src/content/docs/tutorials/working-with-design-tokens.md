---
title: 'Working with Design Tokens'
description: 'Incorporate design tokens into NX workflows.'
---

Design tokens in NX are just typed data. This tutorial shows how to declare tokens, pass them through components, and enforce usage with the type system. It builds on the [Language Tour](/language-tour/elements) and the [Building Your First Component](/tutorials/building-your-first-component) tutorial.

## 1) Define token types
Start a file named `tokens.nx`, or work in the [playground](/playground):

```nx
type ColorToken = { name:string value:string }
type SpaceScale = { xs:int sm:int md:int lg:int }
type FontToken = { family:string size:int weight:int }
type FontScale = { body:FontToken heading:FontToken }

type Theme = {
  primary:ColorToken
  surface:ColorToken
  text:ColorToken
  space:SpaceScale
  fonts:FontScale
}
```

- Tokens are strongly typed; you can constrain units (`int` for spacing) and required fields.
- A scale is a record with one field per step, so each token has a name the compiler knows.
- Group tokens into a `Theme` to pass around as a single object.

## 2) Create a base theme
Each block from here on continues the same file:

```nx fragment
let baseTheme: Theme =
  <Theme
    primary=<ColorToken name="primary" value="#5B6EF5"/>
    surface=<ColorToken name="surface" value="#0B0C10"/>
    text=<ColorToken name="text" value="#F2F4F8"/>
    space=<SpaceScale xs=4 sm=8 md=12 lg=16/>
    fonts=<FontScale
      body=<FontToken family="Inter" size=16 weight=400/>
      heading=<FontToken family="Inter" size=24 weight=700/>
    />/>
```

## 3) Consume tokens in components
Wire tokens into layout and styling instead of raw strings:

```nx fragment
type Style = { backgroundColor:string color:string padding:string fontFamily:string }

let <Panel theme:Theme title:string  content body:Element /> =
  <section
    style=<Style
      backgroundColor={theme.surface.value}
      color={theme.text.value}
      padding={theme.space.md + "px"}
      fontFamily={theme.fonts.body.family}
    />>
    <h2>{title}</h2>
    <div>{body}</div>
  </section>
```

- Because the theme is typed, you can’t accidentally use an unknown token name:
  `theme.space.xxl` is a compile error that lists the fields `SpaceScale` has.
- `Style` is a record too, so a misspelled style property is an error rather than a string that
  silently does nothing.
- Attribute values are ordinary expressions: `theme.space.md + "px"` joins the number and the unit
  into `"12px"`.

## 4) Swap themes without changing components

```nx fragment
let <App theme:Theme/> =
  <Panel theme={theme} title="Design Tokens">
    <p>Tokens flow through the component tree.</p>
  </Panel>

<App theme={baseTheme}/>
```

Drop in an alternate `Theme` instance to re-skin the UI without touching component code.

## 5) Validate usage
- Paste the whole file into the [playground](/playground): it compiles as you type and reports
  errors against your source.
- When you pass tokens through props, the type checker flags missing or mistyped fields—no separate JSON/YAML schema required.

## 6) Extend the pattern
- Add semantic tokens (e.g., `info`, `warning`, `error`) that map to base colors.
- Create a `Button` component that derives its `tone` from `Theme.primary`.
- Move the tokens into a library directory, mark the themes `export`, and import them across apps
  with `import "./tokens"`.
