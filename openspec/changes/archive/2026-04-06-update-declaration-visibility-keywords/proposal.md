## Why

NX currently makes top-level declarations public by default and requires authors to opt into
library-local visibility with `internal`. That default is backward for library authorship: symbols
become part of a library's external API unless the author remembers to hide them, and the language
surface carries both `internal` and `private` even though the common library-local case should be
implicit.

## What Changes

- **BREAKING** Remove the `internal` visibility keyword from top-level declarations.
- **BREAKING** Change the default visibility of top-level declarations from public to library-local
  visibility.
- **BREAKING** Add an `export` keyword for top-level declarations that must be visible outside the
  declaring library.
- Preserve `private` as file-local visibility for top-level declarations.
- Define the new visibility model consistently for libraries and non-library programs: `private`
  stays module-local, default visibility is available throughout the containing library or program,
  and `export` exposes the declaration to external consumers when the declaration lives in a
  library.
- Update grammar, documentation, editor assets, and tests to remove `internal` from the language
  surface and reflect explicit `export` semantics.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `declaration-visibility`: Replace explicit `internal` plus implicit public visibility with
  implicit library-local visibility plus explicit `export` visibility, while preserving `private`
  as file-local visibility.

## Impact

- Affected docs: [nx-grammar.md](/home/bret/src/nexara/external/nx/nx-grammar.md),
  [nx-grammar-spec.md](/home/bret/src/nexara/external/nx/nx-grammar-spec.md), and the language
  visibility documentation under OpenSpec.
- Affected code: grammar/parser, syntax tree and HIR visibility representations, semantic analysis,
  library export table construction, and related tests.
- Affected APIs: visibility keywords in NX source and the set of declarations exposed by a library
  import.
