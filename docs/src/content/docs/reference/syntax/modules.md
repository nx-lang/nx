---
title: 'Modules'
description: 'Structure of NX source files, imports, and main elements.'
---

An NX file is still a module, but imports now target libraries. A library is a directory whose `.nx`
files contribute declarations recursively. For the full grammar, see
[nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition).

## File Layout
- Imports appear at the top of the file and pull in declarations from library directories.
- Local declarations use `let`, `type`, `enum`, `action`, or `component`.
- A module may export a root element directly or expose named bindings.

```nx
import "./theme"
import { Button, Input } from "./ui"
import { Stack as Layout.Stack } from "./layout"

let greeting = "Hello"
private let WelcomeMessage = <span>Hello World</span>
export let accentName = "hello"

<Layout.Stack>
  <text>{accentName}</text>
  <WelcomeMessage/>
  <Button/>
  <Input/>
</Layout.Stack>
```

## Imports
- `import "<library>"` brings every visible export into scope unqualified.
- `import "<library>" as Namespace` keeps imported symbols under `Namespace`.
- `import { Name } from "<library>"` imports specific declarations without qualification.
- `import { Name as Prefix.Name } from "<library>"` adds only a qualification prefix; the final
  segment must remain `Name`.
- Importing the same library path twice in one file is a compile error.
- If two libraries export the same unqualified name, NX reports an error only when that ambiguous
  name is used.
- Local directory libraries are supported today. Git directory URLs and HTTP zip URLs parse, but
  currently resolve with a "not yet supported" diagnostic.

## Libraries
- A library is a directory, not a barrel file.
- Every `.nx` file under that directory contributes declarations recursively.
- Declarations are internal by default, so helper bindings stay inside the library unless marked
  `export`.

## Root Elements
- A root element at the end of the file behaves like `main`. Tooling can render it immediately or expose it as the module default.
- Alternatively, export named bindings and let consumers choose what to render.

## Visibility

| Keyword | Same file | Other library files | Consumers |
| --- | --- | --- | --- |
| `private` | Yes | No | No |
| default | Yes | Yes | No |
| `export` | Yes | Yes | Yes |

- `private` keeps declarations in the current file only.
- Omitting a visibility keyword shares declarations across files in the same library or program
  while hiding them from external consumers.
- `export` exposes declarations to importing libraries.

## See also
- Language Tour: [Modules & Imports](/language-tour/modules-and-imports)
- Grammar: [nx-grammar.md – Module Definition](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition)
