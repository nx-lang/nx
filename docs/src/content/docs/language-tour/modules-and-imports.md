---
title: 'Modules & Imports'
description: 'Organize NX files with imports, local declarations, and a root element.'
---

Each NX file is a module. You can import symbols, declare locals, and end with an optional root element.

## Imports

```nx
contenttype "./schemas/html5.nx"
import { Button, Input } from "./ui/controls.nx"
import "./tokens.nx" as Tokens
import { Stack as LayoutStack } from "./layout.nx"
```

- `contenttype "<path>"` must come first and acts like a module prelude.
- `import "<path>"` brings all exports into scope; `import "<path>" as Name` keeps them under a namespace.
- Selective imports use braces (`import { Name } from "<path>"`) and support inline aliasing (`Name as Alias`).

## Declarations and visibility

```nx
private let footerText = "Built with NX"

let <Footer text:string = footerText/> =
  <footer>{text}</footer>
```

- `private` keeps bindings inside the module.
- Use `let` and `type` as needed before the root element.

## Root element

```nx
<App>
  <Header/>
  <Main/>
</App>
```

If present, the final element is the module’s rendered output or default export, depending on the host runtime.

## See also (Reference/Grammar)
- Reference: [Modules](/reference/syntax/modules)
- Reference: [Functions & Components](/reference/syntax/functions)
- Grammar: [nx-grammar.md – Module Definition](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition)
