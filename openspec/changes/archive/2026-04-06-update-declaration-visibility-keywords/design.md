## Context

NX currently exposes top-level declarations outside a library by default and uses `internal` as an
opt-in keyword for library-local visibility. That model is now inverted: the common case should be
library-local, and authors should opt into external exposure explicitly with `export`.

This change touches every layer that understands declaration visibility: grammar and keywords,
parser/CST, lowered representation, semantic analysis, library export metadata, tests, and
language documentation. It also needs one consistent rule for non-library program builds so the
same source syntax does not change meaning based on whether code is later packaged as a library.

## Goals / Non-Goals

**Goals:**
- Make omitted visibility map to internal visibility for top-level declarations.
- Add `export` as the explicit source keyword for library-facing declarations.
- Preserve `private` as file-local visibility.
- Keep one semantic visibility model that works for both libraries and non-library programs.

**Non-Goals:**
- Adding barrel files, manifest-based export lists, or per-library allowlists.
- Providing compatibility behavior for the removed `internal` keyword.
- Changing import syntax or library loading beyond how export visibility is filtered.
- Redesigning nested or local declaration visibility beyond the existing top-level rules.

## Decisions

### 1. Keep semantic visibility as `private`, `internal`, and `export`

The implementation will continue to model three semantic visibility states. Source syntax maps to
those states as follows:
- `private` keyword -> `private`
- no keyword -> `internal`
- `export` keyword -> `export`

This keeps the semantic model aligned with the requested rule change while minimizing churn in
lowering and resolution code that already reasons about library-local versus externally visible
declarations.

Alternative considered:
- Rename the semantic middle state away from `internal`.

Why rejected:
- The user request removes the `internal` keyword, not the concept of internal visibility. Keeping
  the semantic state avoids unnecessary refactors across the compiler and artifact model.

### 2. Remove `internal` from the source grammar entirely

`internal` will no longer be recognized as a visibility modifier. The parser and keyword handling
only need to support `private` and `export` for top-level visibility; legacy `internal` usage can
fail through ordinary syntax handling without any special compatibility branch or bespoke
diagnostic.

Alternative considered:
- Retain parser awareness of `internal` only to emit a targeted migration error.

Why rejected:
- No backward compatibility is needed, and special handling would add implementation churn for a
  keyword that is not used today.

### 3. Library export metadata becomes opt-in through `export`

Only declarations with semantic visibility `export` will be published into library export metadata
for consumer imports. Declarations with semantic visibility `internal` remain visible to files in
the same library but are omitted from the exported symbol table. `private` remains file-local.

Alternative considered:
- Keep omitted visibility exported and add `export` as a synonym.

Why rejected:
- That would preserve the current accidental-public API surface and fail to deliver the requested
  default-internal behavior.

### 4. Non-library programs reuse internal visibility as whole-program visibility

For builds that operate on root source files rather than imported libraries, declarations with no
visibility keyword remain visible throughout the same program. `export` is still legal syntax and
retains `export` semantic visibility, but it has no additional external effect until the code is
consumed as a library.

Alternative considered:
- Make `export` illegal or make omitted visibility context-dependent in non-library programs.

Why rejected:
- Context-dependent validity would complicate parsing and make the same source declaration change
  meaning depending on how the code is built.

### 5. Documentation and tests must migrate in the same change as the parser

Because this is a breaking source-language update, grammar docs, examples, tests, and diagnostics
must move together. The change should not leave the parser accepting `export` while documentation
still teaches public-by-default or examples still use `internal`.

Alternative considered:
- Land parser changes first and update docs/examples later.

Why rejected:
- That would leave the language definition internally inconsistent and make failures harder to
  interpret during migration.

## Risks / Trade-offs

- Existing libraries that relied on omitted visibility for public API will silently stop exporting
  those declarations if authors do not mark them `export`. -> Mitigation: add targeted tests and
  documentation that call out the need to mark consumer-facing declarations with `export`.
- `export` becomes a reserved top-level keyword and can break existing identifiers using that name.
  -> Mitigation: treat this as an intentional language-breaking change and update fixtures and docs
  accordingly.
- Keeping `internal` as a semantic state while removing it from source syntax can confuse future
  contributors. -> Mitigation: document the source-to-semantic mapping in code comments and specs.
- Allowing `export` in non-library programs can look redundant. -> Mitigation: document that it is
  semantically harmless there and avoids context-sensitive syntax rules.

## Migration Plan

1. Update [nx-grammar.md](/home/bret/src/nexara/external/nx/nx-grammar.md),
   [nx-grammar-spec.md](/home/bret/src/nexara/external/nx/nx-grammar-spec.md), and OpenSpec
   visibility docs to describe `private`, default internal visibility, and `export`.
2. Update grammar, parser, CST, HIR, and keyword handling to parse `export`, remove `internal` as
   a valid visibility modifier, and preserve omitted visibility as semantic `internal`.
3. Update name resolution and library artifact export-table construction so only `export`
   declarations are visible to importing consumer libraries.
4. Add tests for omitted internal visibility and explicit `export`.
5. Update examples, fixtures, and editor tooling to remove stale `internal` references and reflect
   the new visibility keywords.

## Open Questions

No blocking questions. If future tooling wants to warn about redundant `export` usage in a
non-library program, that can be layered on later without changing these semantics.
