---
title: 'Modules & Imports'
description: 'Organize NX files with imports, local declarations, and a root element.'
---

Each NX file is a module. Imports now target libraries, where a library is a directory of `.nx`
files. Every `.nx` file under that directory contributes declarations recursively, so there is no
barrel file to maintain.

## Imports

```nx fragment
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

let <Footer text:string = {footerText}/> =
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

## Built-in declarations

NX's built-in types are declared in the **NX prelude**, a module every file imports automatically.
You never name it or enable it — `Range` is simply there:

```nx
let squares = { for i in 0..4 { i * i } }
let bounds:<Range T=int/> = {1..=5}
```

The prelude sits under the reserved `@nx/` root. Its names are shadowable: declare your own `Range`
and yours wins, silently, with no ambiguity — though `..` and `..=` then stop working in that file,
because they construct the built-in one.

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
- Reference: [Ranges](/reference/syntax/types#ranges)
- Reference: [Functions & Components](/reference/syntax/functions)
- Grammar: [nx-grammar.md – Module Definition](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition)
