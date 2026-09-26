---
title: 'What is NX?'
description: 'An overview of the goals and philosophy behind the NX language.'
---

NX is a single language that combines markup and functions without forcing you to glue two ecosystems together. If you know JSX/TSX or XAML, NX feels familiar but adds strong typing, predictable semantics, and first-class data modelling so UI and configuration share one syntax.

## Why NX Exists
NX targets four overlapping goals:

- **Unified markup + logic**: One surface for elements, functions, and expressions—no split between “template” and “code-behind”.
- **UI and design systems first**: Great for component libraries, tokens, theming, and layout logic.
- **Familiar but better than JSX/XAML**: Keeps the element look-and-feel while adding strong types, expressions that always return values, and runtime flexibility beyond browsers or .NET.
- **Schema-free configuration**: Replaces JSON/YAML/XML plus schema files; constraints live directly in the NX source.

Traditional stacks force you to juggle two languages (HTML + JS, XML + C#, JSON + schema). NX keeps everything in one place so properties, content, and behaviour share the same type system, tooling, and runtime model.

## Quick Example
This is a complete module: a type, a component, markup, and logic all live together.

```nx
type User = { id:string name:string email?:string avatarUrl?:string }

let <UserCard user:User tone:string = "neutral"/> =
  <article className={"card tone-" + tone}>
    <img src={user.avatarUrl ?? "/fallback.png"} alt={user.name}/>
    <h3>{user.name}</h3>
    if user.email? { <p>{user.email}</p> } else { <p>No email on file</p> }
  </article>

<UserCard user=<User id="123" name="Ada" email="ada@example.com"/> tone="info"/>
```

- The same angle-bracket syntax builds data and calls components: `<User .../>` constructs a
  record, and `<UserCard .../>` calls a component whose signature is shaped like that call.
- `if` returns values, so you can branch inline without leaving the expression model.
- Optional fields are part of the type: `email?` may be left out, and `??` supplies a fallback for
  `avatarUrl`.
- Attributes accept markup, objects, or expressions—no stringly-typed gaps.
- A larger program imports from libraries, and a library is a directory, so
  `import { Card } from "./ui"` draws on a folder of reusable NX files instead of a single barrel
  module.

## Where NX Fits
- **UI composition and design systems**: Define tokens, themes, and components that share one syntax and type system.
- **Productivity for JSX/XAML developers**: The structure is familiar, but types are enforced and the runtime isn’t locked to one platform.
- **Typed configuration**: Replace JSON/YAML/XML plus separate schemas with a single NX file that declares both data and constraints.
- **Platform-agnostic delivery**: Interpret, transpile, or embed NX in any host. Tooling (a language server with diagnostics, hover, and completion) comes from the language, not a specific framework.
