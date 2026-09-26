---
title: 'Design Goals'
description: 'High-level design goals that guide NX language decisions.'
---

NX’s design goals explain why the language looks and behaves the way it does. They anchor decisions about syntax, runtime capabilities, and tooling so that NX stays cohesive.

## Single, Integrated Language
- Markup, functions, and data modelling share one syntax—no “template language” bolted to a scripting language.
- Declarations mirror usage: a component signature is shaped exactly like the element you render.
- Any attribute can hold expressions or markup, so composition never falls back to string concatenation.

## Built for UI and Design Systems
- Components, content-marked properties, and design tokens are first-class; typed body content sits beside typed attributes.
- Namespaces keep large UI libraries organised without sacrificing readability.
- Styling and theming can be expressed as data, not strings, so tokens remain type-checked.

## Familiar Yet Improved for JSX/XAML Devs
- Starts from the element/attribute model you already know.
- Adds enforced types, expression semantics that always yield values, and runtime flexibility beyond browsers or .NET.
- Removes common pain points: no stringly-typed props, no ad hoc child typing, no forced split between markup and logic.

## Schema-Free Typed Configuration
- NX can replace JSON/YAML/XML plus a separate schema: types live in the file that uses them.
- Type annotations, defaults, and constraints travel with the data, so validation and documentation are always in sync.
- The same syntax defines UI and configuration, enabling mixed-mode files (e.g., UI + typed data sources).

## Tooling and Runtime Flexibility
- Diagnostics, hover, completion, and document symbols come from an NX language server, not a framework.
- Expression trees can be interpreted, compiled, or transpiled depending on the host runtime.
- Purity-first semantics (expressions return values, no hidden side effects) keep optimisation and reasoning straightforward.

## Example Alignment
This small example shows the goals working together: unified syntax, UI-first design, JSX/XAML familiarity, and typed data in one file. `Button` is an `external` component that the host supplies; NX checks each use of it against its props and the `Pressed` action it emits.

```nx
type Theme = { primary:string surface:string text:string }
type Style = { backgroundColor:string color:string }
type HeroLink = { label:string href?:string }

action LinkChosen = { label:string }

external component <Button tone:string href?:string content label:string emits { Pressed { } } />

component <Hero
  links:HeroLink+
  theme:Theme
  content body:Element
  emits { LinkChosen }
/> = {
  <section style=<Style backgroundColor={theme.surface} color={theme.text} />>
    {body}
    <nav>
      for link in links {
        <Button
          href={link.href}
          tone={if link.href? { "link" } else { "primary" }}
          onPressed=<LinkChosen label={link.label} />>
          {link.label}
        </Button>
      }
    </nav>
  </section>
}
```

Everything—types, styling tokens, behaviour, and markup—is expressed in NX. Behaviour is data too: pressing a button doesn't run a callback, it produces a `LinkChosen` action record, which whoever renders `Hero` handles or hands on to the host. That cohesion is the design north star.
