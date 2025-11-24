---
title: 'Modules'
description: 'Structure of NX source files, imports, and main elements.'
---

An NX module defines imports, local declarations, and an optional root element. For the full grammar, see [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition).

## File Layout
- Imports appear at the top of the file and pull in components, values, or namespaces.
- Local declarations use `let` or `type` and are scoped to the current module.
- A module may export a root element directly or expose named bindings.

```nx
import * from "./components"
import { Button, Input } from "./ui/controls"
import { List } from "./collections" as Collections

private let WelcomeMessage = <span>Hello World</span>

<MainApp>
  <WelcomeMessage/>
</MainApp>
```

## Imports
- `import *` brings every exported symbol into scope.
- Curly-brace imports target specific identifiers and support aliasing.
- Namespace-style imports (`as Collections`) keep related components grouped.

## Root Elements
- A root element at the end of the file behaves like `main`. Tooling can render it immediately or expose it as the module default.
- Alternatively, export named bindings and let consumers choose what to render.

## Access Modifiers
- `private` keeps declarations internal to the module.

## See also
- Language Tour: [Modules & Imports](/language-tour/modules-and-imports)
- Grammar: [nx-grammar.md – Module Definition](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition)
