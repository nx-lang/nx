## Why

NX import handling is still centered on single-module paths and an unused `contenttype` directive, which no longer matches the language direction. Moving imports to library directories and defining library-scoped visibility now is necessary to unblock the grammar, compiler pipeline, tooling, and docs from continuing to encode outdated module-level assumptions.

## What Changes

- **BREAKING** Replace `ModulePath`-based imports with `LibraryPath`-based imports across the grammar, parser, lowering, interpreter, VS Code parser, and documentation.
- **BREAKING** Remove the `contenttype` directive from the grammar and the rest of the stack.
- **BREAKING** Change import resolution so local imports target library directories containing recursive `.nx` sources rather than individual module files.
- Add selective import prefix aliasing with syntax such as `import { Foo as MyLib.Foo } from "../mylib"`, where the alias must be a qualified name whose final segment matches the imported declaration name.
- Define import semantics for duplicate library imports, deferred ambiguity errors, and diagnostics that point users toward qualified selective imports or namespace imports.
- Add `internal` visibility and make declarations public by default, with visibility determined per declaration across file, library, and consumer boundaries.

## Capabilities

### New Capabilities
- `declaration-visibility`: Define `private`, `internal`, and default public visibility for declarations within a library directory and for external consumers.

### Modified Capabilities
- `module-imports`: Change import syntax and semantics from module paths to library paths, add selective qualified-prefix aliasing, and define ambiguity and duplicate-import rules for library-based resolution.
- `contenttype-directive`: Remove the `contenttype` directive from the language grammar, syntax tree, and lowered representations.

## Impact

- Affected docs: [nx-grammar.md](/home/bret/src/nx/nx-grammar.md), [nx-grammar-spec.md](/home/bret/src/nx/nx-grammar-spec.md), and related language documentation.
- Affected code: parser, syntax tree/AST types, lowering, interpreter/runtime import resolution, diagnostics, tests, and the VS Code extension parser.
- Affected APIs: import syntax, visibility keywords, and compile-time name-resolution behavior.
