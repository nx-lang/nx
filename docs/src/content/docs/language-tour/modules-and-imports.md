---
title: 'Modules & Imports'
description: 'Organize NX files with imports, local declarations, and a root element.'
---

Each NX file is a module. You can import symbols, declare locals, and end with an optional root element.

## Imports

```nx
import { Button, Input } from "./ui/controls"
import * from "./tokens"
import { Stack } from "./layout" as Layout
```

- `import *` brings all exports into scope.
- Curly-brace imports target specific names; use `as` to alias.

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

## Next up

## See also (Reference/Grammar)
- Reference: [Modules](/reference/syntax/modules)
- Reference: [Functions & Components](/reference/syntax/functions)
- Grammar: [nx-grammar.md – Module Definition](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition)
