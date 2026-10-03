---
title: 'Modules'
description: 'Structure of NX source files, imports, and main elements.'
---

An NX file is still a module, but imports now target libraries. A library is a directory whose `.nx`
files contribute declarations recursively. For the full grammar, see
[nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#module-definition).

## File Layout
- Imports appear at the top of the file and pull in declarations from library directories.
- Local declarations use `let`, `type`, `action`, or `component`.
- A module may export a root element directly or expose named bindings.

```nx fragment
import "./theme"
import { Button, Input } from "./ui"
import { Stack as Layout.Stack } from "./layout"

let greeting = "Hello"
private let <WelcomeMessage/> = <span>Hello World</span>
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
- `import "@nx/<name>"` imports a [standard library](#standard-libraries) in any of the forms above.
  The path is not relative to the importing file, so it is written the same way everywhere.
- Local directory libraries are supported today. Git directory URLs and HTTP zip URLs parse, but
  currently resolve with a "not yet supported" diagnostic.

## Libraries
- A library is a directory, not a barrel file.
- Every `.nx` file under that directory contributes declarations recursively.
- Declarations are internal by default, so helper bindings stay inside the library unless marked
  `export`.

## Built-in declarations
NX's built-in types are declared in the **NX prelude**, a module every file imports automatically.
Nothing names it and nothing enables it: its exported declarations are simply in scope in every
module, in every build — a single file, a workspace, a library, an editor session.

- The prelude lives under the reserved `@nx/` root, as the module `@nx/prelude.nx`. The whole root
  is reserved: a workspace may not supply a module under it, and a host may not load a library
  there.
- **Its names are shadowable.** A declaration of your own, or a name from any import you wrote, takes
  the name silently, exactly as it would over a wildcard import. There is no diagnostic and no
  ambiguity: a prelude name is bound last, and only where nothing else claimed it.
- Shadowing a built-in disables the syntax that means it. A module that declares its own `Range`
  cannot use `..` or `..=` there, because those operators construct *the prelude's* `Range`; the
  element form still works, and every other module is unaffected.
- A library's exports are its own declarations. The prelude is never re-exported.

The prelude is one ordinary NX declaration:

```nx
export type Range = {
  T:type
  start:T
  end:T
  endInclusive:boolean
}
```

See [Ranges](/reference/syntax/types#ranges) for what `Range` means and
[`for`](/reference/syntax/for#counting-with-a-range) for counting over one.

## Standard libraries
A **standard library** is NX source the compiler carries, like the prelude, but in scope only where
it is imported. It is named by the reserved root `@nx/<name>`:

```nx
import "@nx/agent"

let assistant = <Agent name="support">Be brief.</Agent>
```

- Nothing has to be installed, loaded or enabled. The import resolves in every build and tool: a
  single file, a workspace, a library, an editor session, `nxlang`.
- Every import form works: `import "@nx/agent"`, `import "@nx/agent" as Ai`, and
  `import { Agent, Tool } from "@nx/agent"`.
- Without the import the library's names are not in scope, so a file that declares its own `Tool`
  is unaffected. Once imported it behaves as any library does: a name it shares with another import
  is an error only where the name is used, and importing it twice is a compile error.
- A path under `@nx/` that names no standard library is an error that lists the ones that exist. A
  library's modules are not importable on their own, so `import "@nx/agent/agent.nx"` is the same
  error.
- A host can put a standard library in scope for the source it compiles, so authored files need no
  import line.

| Library | Stability | What it holds |
| --- | --- | --- |
| [`@nx/agent`](/reference/libraries/agent) | unstable | Types for declaring an AI agent, its documents and its tools |

An **unstable** library may change incompatibly in any NX release, including a patch release. A
**stable** one changes incompatibly only in a release whose version marks a breaking change.

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
