# NX Language

Language support for [NX](https://nxlang.org), a typed language for markup, data and the logic
between them. Open any `.nx` file to get highlighting, diagnostics as you type, hover information
and completions.

```nx
type Task = { title:string  done:boolean = false }

let <TaskList tasks:Task+ /> =
  <ul>
    for task in tasks {
      <li checked={task.done}>{task.title}</li>
    }
  </ul>
```

New to NX? Start with [Getting Started](https://nxlang.org/tutorials/getting-started/) or try it
without installing anything in the [playground](https://nxlang.org/playground).

## Features

- **Syntax highlighting** for `.nx` files, and for ` ```nx ` code blocks in Markdown.
- **Diagnostics** from the NX language server: syntax and type errors appear as you type.
- **Hover** shows the declaration and type of what is under the cursor.
- **Completions** for keywords, types, declarations, components, their properties and union cases.
- **Outline**: types, functions, components and values show up in the Outline view and in Go to
  Symbol in Editor.
- **Snippets** (type `nx` to list them) for elements, unions, `if`, `for`, `match` and braced values.
- Comment toggling, bracket matching and auto-closing pairs.

The language server ships inside the extension, so there is nothing else to install. It is packaged
for Windows (x64), macOS (Apple silicon) and Linux (x64).

## Settings

| Setting | Default | Description |
|---|---|---|
| `nx.server.path` | empty | Path to an `nx-lsp` executable to use in place of the packaged one, for example a build of your own. |

## Links

- Documentation: [nxlang.org](https://nxlang.org)
- Playground: [nxlang.org/playground](https://nxlang.org/playground)
- Issues and source: [github.com/nx-lang/nx](https://github.com/nx-lang/nx)
- Building, testing and releasing the extension:
  [CONTRIBUTING.md](https://github.com/nx-lang/nx/blob/main/src/vscode/CONTRIBUTING.md)

## Editor assets for other editors: `@nx-lang/language`

The same grammar, language configuration and snippets are published to npm as
`@nx-lang/language`, for web editors such as Monaco or Shiki. It needs no VS Code install.

```bash
pnpm add @nx-lang/language
```

```ts
import grammar from '@nx-lang/language/grammar';
import markdownCodeBlockGrammar from '@nx-lang/language/markdown-codeblock-grammar';
import languageConfiguration from '@nx-lang/language/language-configuration';
import snippets from '@nx-lang/language/snippets';
```

| Import path | Contents |
|---|---|
| `@nx-lang/language/grammar` | The NX TextMate grammar (`source.nx`) |
| `@nx-lang/language/markdown-codeblock-grammar` | The injection grammar for ` ```nx ` blocks in Markdown |
| `@nx-lang/language/language-configuration` | Comments, brackets and auto-closing pairs |
| `@nx-lang/language/snippets` | The snippets |
