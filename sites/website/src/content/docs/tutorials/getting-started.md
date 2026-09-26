---
title: 'Getting Started'
description: 'Try NX in the browser, then use it from .NET or JavaScript through the published packages.'
---

You can write and run NX without installing anything, then call it from your own application
through a published package, using only the package manager you already have.

## 1) Try it in the playground

Open the [playground](/playground), pick an example, and edit it. It compiles as you type, in your
browser, and reports errors against your source.

Here is a small NX program to paste in. It declares a record type, a component that takes one, and
ends with the element that is the program's result:

```nx
type User = { name:string }

let <Greeting user:User /> =
  <p>Hello, {user.name}!</p>

<Greeting user=<User name="Ada" /> />
```

It evaluates to a `p` element whose content is `"Hello, "`, `"Ada"` and `"!"`.

## 2) Editor support

A VS Code extension with highlighting, diagnostics, hover and completion is on its way to the
Visual Studio Marketplace and Open VSX, and isn't published yet. Until it is,
[Contributing](/contributing/) describes how to run it from source.

## 3) Use NX from .NET

The [`NxLang.Sdk`](https://www.nuget.org/packages/NxLang.Sdk) package compiles and evaluates NX
from any .NET 10 application, on Linux x64, macOS arm64 and Windows x64:

```bash
dotnet add package NxLang.Sdk
```

```csharp
using System.Text.Json;
using NxLang.Nx;

string source = """
    type User = { name:string }
    let <Greeting user:User /> = <p>Hello, {user.name}!</p>
    <Greeting user=<User name="Ada" /> />
    """;

JsonElement value = NxRuntime.EvaluateJson(source);
Console.WriteLine(value); // {"$type":"p","content":["Hello, ","Ada","!"]}
```

`NxRuntime.Evaluate<T>` deserializes the result into your own type instead, and
`NxProgramArtifact` builds a program once to evaluate it many times.

## 4) Use NX from JavaScript

In a browser or under Node, [`@nx-lang/sdk-wasm`](https://www.npmjs.com/package/@nx-lang/sdk-wasm)
compiles NX to NX IR, and [`@nx-lang/ir-runtime`](https://www.npmjs.com/package/@nx-lang/ir-runtime)
evaluates that IR:

```bash
npm install @nx-lang/sdk-wasm @nx-lang/ir-runtime
```

```js
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { evaluateFunction, prepareNxIrProgram } from "@nx-lang/ir-runtime";

const host = createNxHost(await loadNxModule());
const artifact = host.buildProgramArtifact(`
  type User = { name:string }
  let <Greeting user:User /> = <p>Hello, {user.name}!</p>
  <Greeting user=<User name="Ada" /> />
`);
const [{ bytes }] = artifact.generateNxIr();
artifact.dispose();

const program = prepareNxIrProgram(bytes);
console.log(evaluateFunction(program, "root"));
// { $type: "p", content: ["Hello, ", "Ada", "!"] }
```

`loadNxModule` reads the compiler from disk under Node. In a browser, compile it from its URL
with `compileNxModule(fetch(url))`. An application that only runs NX it compiled ahead of time
ships the IR bytes and needs only `@nx-lang/ir-runtime`.

## 5) Next steps

- The [Language Tour](/language-tour/elements) walks through elements, functions, expressions and
  types.
- [Building Your First Component](/tutorials/building-your-first-component) builds something a
  little larger.
- The [Reference](/reference/syntax/elements) gives the exact rules when you need them.

To build NX from source, use the `nxlang` command-line tool, or work on NX itself, see
[Contributing](/contributing/).
