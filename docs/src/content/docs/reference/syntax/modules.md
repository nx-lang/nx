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
contenttype "./schemas/html5.nx"
import "./components.nx"
import { Button, Input } from "./ui/controls.nx"
import "./collections.nx" as Collections
import { List as CollectionsList } from "./collections/list.nx"

private let WelcomeMessage = <span>Hello World</span>

<MainApp>
  <WelcomeMessage/>
</MainApp>
```

## Imports
- `contenttype "<path>"` is optional, but when present it must be the first statement.
- `import "<path>"` brings every exported symbol into scope.
- `import "<path>" as Namespace` keeps imported symbols grouped under `Namespace`.
- Selective imports (`import { ... } from "<path>"`) target specific identifiers and support inline aliasing (`Name as Alias`).

## Root Elements
- A root element at the end of the file behaves like `main`. Tooling can render it immediately or expose it as the module default.
- Alternatively, export named bindings and let consumers choose what to render.

## Access Modifiers
- `private` keeps declarations internal to the module.

## See also
- Language Tour: [Modules & Imports](/language-tour/modules-and-imports)
- Grammar: [nx-grammar.md – Module Definition](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition)
