## Context

NX currently parses imports as string-based module paths and stores them on the HIR `Module`, but the rest of the pipeline still treats each `.nx` file as an isolated compilation unit. `contenttype` is represented in the grammar, CST, HIR, docs, syntax highlighting, and tests even though it is not used. Selective imports also still model `as` as a free-form rename, which does not match the desired qualified-prefix behavior.

This change introduces a library layer above individual files. Imports now target library directories, not single source files. Every `.nx` file under an imported directory contributes declarations to that library, and visibility is evaluated across three scopes: current file, same library, and external consumers. The parser, lowered model, interpreter, diagnostics, VS Code grammar/tests, and docs all need to converge on the new rules together.

## Goals / Non-Goals

**Goals:**
- Replace `ModulePath` imports with `LibraryPath` imports everywhere in the stack.
- Remove `contenttype` from grammar, CST, HIR, docs, tests, and editor tooling.
- Support local directory libraries by recursively loading `.nx` files from an imported directory.
- Add selective import prefix aliasing such as `import { Foo as MyLib.Foo } from "../mylib"`.
- Make declarations public by default and add `internal` as a library-scoped visibility level.
- Detect duplicate imports from the same library within a file and defer ambiguous-name diagnostics until use.

**Non-Goals:**
- Fetching Git-based libraries or HTTP zip libraries.
- Introducing barrel files, manifests, or explicit export lists.
- Solving package versioning, caching, or remote dependency management.
- Preserving backward compatibility for the removed `contenttype` directive or module-file import shape.

## Decisions

### 1. Keep file modules, add an explicit library aggregation layer

Each `.nx` file remains its own parsed and lowered module, but import resolution operates on a new library abstraction backed by a directory. A library root is the resolved `LibraryPath` directory; all `.nx` files under that directory, recursively, belong to the same library. For direct compilation of a standalone file, the file's parent directory is treated as its library root unless the compiler already loaded it under a broader library root.

Imported libraries are indexed once per canonical local path and expose a declaration table annotated with source file and visibility metadata.

Alternative considered:
- Treat every directory import as syntactic sugar for importing an implicit barrel file.

Why rejected:
- The user requirement explicitly removes barrel files. Recursive aggregation keeps the source model aligned with the language definition.

### 2. `LibraryPath` stays a string literal in syntax, while resolution only supports local directories for now

The grammar and CST will rename `ModulePath` to `LibraryPath`, but the surface syntax remains a quoted string literal. This keeps the grammar stable for all three intended path categories:
- Relative or absolute local directory paths
- Git URL directory paths
- HTTP URL zip paths

Only local directory paths are resolved in this change. Git and HTTP forms are parsed and lowered but fail during semantic resolution with an explicit "not yet supported" diagnostic.

Alternative considered:
- Restrict the grammar to local filesystem-looking paths until remote support exists.

Why rejected:
- That would force another grammar change later and make the docs diverge from the intended language shape.

### 3. Selective `as` aliases are stored as qualifier prefixes, not arbitrary renamed symbols

Wildcard imports keep the existing `import "<library>" as Prefix` form, where `Prefix` is a single identifier. Selective imports change from arbitrary aliases to qualified-prefix aliases:

```nx
import { Foo as MyLib.Foo } from "../mylib"
```

The parser will accept a `QualifiedName` after `as` for selective imports. Semantic validation then enforces:
- exactly one dot
- the final segment matches the imported name

Lowering stores the imported name plus an optional qualifier prefix rather than a free-form alias. This makes the representation match the language rule directly and avoids carrying invalid renamed states deeper into the pipeline.

Alternative considered:
- Keep storing a full alias name and validate the suffix later.

Why rejected:
- Resolver logic would still need to peel the alias apart repeatedly. Storing only the qualifier prefix makes ambiguity handling and imported symbol emission simpler.

### 4. Duplicate imports are checked after path normalization, not in the parser

`import "../mylib"` and `import "../foo/../mylib"` must be treated as the same library import once normalized. Duplicate-import detection therefore happens in semantic analysis after local filesystem paths are resolved relative to the current source file and canonicalized.

The diagnostic is a compile error scoped to the importing file and fires before symbol resolution. This rule applies regardless of whether the two imports are wildcard or selective forms.

Alternative considered:
- Reject duplicate raw string literals in the parser.

Why rejected:
- Parser-level checks cannot normalize paths and would miss equivalent local paths.

### 5. Imported names are tracked as provenance sets so ambiguity errors happen on use

Library loading builds an export table of public declarations for consumers and a wider table of public-plus-internal declarations for same-library files. File analysis then projects imports into two lookup shapes:
- unqualified bindings for plain wildcard imports and non-prefixed selective imports
- qualified bindings for namespace imports and selective imports with qualifier prefixes

Unqualified bindings store all candidate sources for a given visible name. If two different imported libraries export `Foo`, the import itself succeeds. A compile error is produced only when an unqualified reference resolves to multiple candidates. The error includes both source libraries and suggests either:
- switching to a selective import with `as Prefix.Name`
- using a namespace import

Alternative considered:
- Fail immediately when the second colliding import is encountered.

Why rejected:
- The requested behavior allows users to import overlapping libraries as long as they do not use the ambiguous name unqualified.

### 6. Visibility is an optional declaration modifier with default public semantics

Top-level `let`, `type`, `enum`, record, and `component` declarations gain an optional visibility modifier:
- `private`: visible only inside the declaring file
- `internal`: visible to any file in the same library, but not to external consumers
- no modifier: public

The HIR item model gains a `Visibility` enum so the library index can filter declarations according to the requesting file's library root. This change replaces the current doc-only notion of `private` with an implemented visibility model and adds `internal` as a reserved keyword.

Alternative considered:
- Keep module-private as the default and require `public` for exports.

Why rejected:
- The requested language model is public-by-default, optimized for component-library authoring.

### 7. Remove `contenttype` completely rather than translating it into a hidden import

`contenttype` will be removed from:
- `nx-grammar.md`
- `nx-grammar-spec.md`
- tree-sitter grammar and generated artifacts
- syntax kinds and CST helpers
- HIR `Module` fields and lowering
- docs, fixtures, and VS Code grammar/tests

There is no compatibility shim that rewrites `contenttype` into an import. Files using it become parse errors and must be updated.

Alternative considered:
- Keep parsing `contenttype` and lower it as a hidden library import.

Why rejected:
- The directive is explicitly being removed, and a hidden translation would keep dead language surface alive in the implementation.

## Risks / Trade-offs

- Ambiguous-name errors will become more common once directory imports expose many declarations at once. → Mitigation: defer the error until use and include remediation text that points to namespace imports or qualified selective imports.
- Canonical path resolution can behave differently across platforms and symlink layouts. → Mitigation: normalize relative segments first, canonicalize existing local directories where possible, and preserve the original source text for diagnostics.
- Making `internal` a reserved keyword can break existing identifiers named `internal`. → Mitigation: accept this as a language-breaking change and update tests/docs accordingly.
- Recursive library indexing can increase parse work for large directories. → Mitigation: cache libraries by canonical root path and parse each file at most once per compilation session.

## Migration Plan

1. Update [nx-grammar.md](/home/bret/src/nx/nx-grammar.md) and [nx-grammar-spec.md](/home/bret/src/nx/nx-grammar-spec.md) to remove `contenttype`, rename `ModulePath` to `LibraryPath`, describe qualified selective aliases, and add declaration visibility.
2. Update tree-sitter grammar, syntax kinds, CST helpers, queries, generated parser artifacts, and parser tests to reflect the new syntax.
3. Update HIR data structures and lowering to remove `content_type`, rename import path fields, add selective qualifier prefixes, and attach visibility to top-level items.
4. Add library indexing and resolution in the compiler/interpreter pipeline for local directory imports, duplicate-import detection, and deferred ambiguity diagnostics.
5. Update VS Code grammars/tests, language-tour/reference docs, and examples to use library directories and the new visibility rules.

## Open Questions

No blocking questions for this change. Remote Git and HTTP library resolution remain intentionally unspecified beyond parser acceptance and "not yet supported" diagnostics.
