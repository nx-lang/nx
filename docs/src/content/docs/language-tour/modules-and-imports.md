---
title: 'Modules & Imports'
description: 'Organize NX files with imports, local declarations, and a root element.'
---

Each NX file is a module. Imports now target libraries, where a library is a directory of `.nx`
files. Every `.nx` file under that directory contributes declarations recursively, so there is no
barrel file to maintain.

## Imports

```nx
import "./tokens"
import "./icons" as Icons
import { Button, Stack as Layout.Stack } from "./ui"
```

- `import "<library>"` brings exports into scope unqualified.
- `import "<library>" as Name` keeps imported declarations under `Name`.
- `import { Name } from "<library>"` imports a specific declaration.
- `import { Name as Prefix.Name } from "<library>"` adds a qualification prefix without renaming the
  imported declaration itself.
- Imports are unqualified by default. You only get qualification when you use `as`.
- Importing the same library path more than once in a file is a compile error.
- Local directories resolve today. Git directory URLs and HTTP zip URLs are reserved for future
  work and currently produce explicit diagnostics.

## Declarations and visibility

```nx
let footerText = "Built with NX"
private let copyright = "2026"
export let brandName = "NX"

let <Footer text:string = footerText/> =
  <footer>{brandName} {text} @{copyright}</footer>
```

- Visibility defaults to internal within the current library or non-library program.
- `private` keeps a binding inside the current file.
- `export` makes a binding visible to external library consumers while keeping it visible inside the
  current library or program.
- Use `let` and `type` as needed before the root element.

| Keyword | Same file | Other library files | Consumers |
| --- | --- | --- | --- |
| `private` | Yes | No | No |
| default | Yes | Yes | No |
| `export` | Yes | Yes | Yes |

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
