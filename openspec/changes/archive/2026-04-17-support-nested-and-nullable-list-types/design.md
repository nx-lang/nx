## Context

NX currently documents type declarations as allowing at most one postfix modifier in the formal
grammar, and the parser grammar follows that restriction. As a result, syntax such as
`string[][]` and `string[]?` is rejected even though the underlying type model is already
recursive:

- `TypeRef` and `Type` can already represent nested `Array` and `Nullable` wrappers.
- HIR lowering already walks suffix tokens in source order and wraps the current type as it goes.
- TypeScript and C# code generation already recurse over nested array and nullable wrappers.
- User-facing docs already describe nested sequence types.

This change is therefore mostly about exposing an already-natural capability consistently at the
surface syntax layer and making sure every user-visible renderer preserves the same distinction
between `T?[]` and `T[]?`.

## Goals / Non-Goals

**Goals:**
- Allow repeated postfix type suffixes anywhere NX accepts a type reference.
- Support nested list types such as `string[][]` and deeper compositions.
- Support nullable list types such as `string[]?` without collapsing them into `string?[]`.
- Preserve suffix ordering semantics consistently through parsing, lowering, type checking,
  diagnostics, and generated TypeScript/C# output.
- Add focused tests and documentation updates for composed suffix forms.

**Non-Goals:**
- Adding new collection syntax beyond existing `[]` list suffixes.
- Introducing tuple, generic, or map type syntax.
- Changing runtime list values, array literals, or array indexing semantics.
- Reworking unrelated type-system features outside composed list/nullability suffix handling.

## Decisions

### 1. Make type suffixes repeatable in source order

The formal grammar and tree-sitter grammar will change from “base type plus optional single
modifier” to “base type plus zero or more suffixes,” where each suffix is either `?` or `[]`.

That keeps the syntax model small: NX still has exactly two postfix suffix operators for type
references, but they now compose instead of being artificially limited to one use.

Rationale:
- This matches user expectations from other typed languages.
- It aligns the grammar with existing documentation and internal type structures.
- It avoids introducing a second list syntax or one-off nullable-list syntax.

Alternative considered:
- Keep the single-modifier grammar and force users to introduce named aliases for nested/nullable
  list shapes.

Why rejected:
- That preserves the same type-system capability while adding ceremony and surprise at the syntax
  level.

### 2. Preserve suffix order by lowering left-to-right into nested wrappers

Lowering will continue to apply suffixes in source order, wrapping the current `TypeRef` each time:

- `string?[]` lowers as `Array(Nullable(string))`
- `string[]?` lowers as `Nullable(Array(string))`
- `string[][]` lowers as `Array(Array(string))`

No normalization step should rewrite one form into another.

Rationale:
- Source-order wrapping is easy to explain and test.
- It cleanly distinguishes list-of-nullable from nullable-list semantics.
- It fits the existing recursive AST and semantic type representations without adding new node
  kinds.

Alternative considered:
- Canonicalize list/nullability compositions into a smaller subset of accepted shapes.

Why rejected:
- That would erase meaningful distinctions users are explicitly asking for.

### 3. Reject duplicate nullable suffixes on the same layer during post-parse validation

The grammar will continue to accept a repeated suffix list, but validation must reject a `?` suffix
when the current outer type layer is already nullable:

- `string?[]?` remains valid because `[]` introduces a new outer array layer before the final `?`
- `string??` is invalid because the second `?` repeats nullability on the same base type layer
- `string?[]??` is invalid because the second trailing `?` repeats nullability on the same outer
  array layer

Rationale:
- This matches user expectations from nullability annotations in mainstream languages.
- It keeps `?` as a simple nullable marker rather than an arbitrarily nestable option constructor.
- Post-parse validation preserves tree-sitter recovery while allowing precise diagnostics on the
  redundant suffix token.

Alternative considered:
- Encode the restriction directly in the grammar.

Why rejected:
- That makes the parser more complex and harms recovery for malformed type annotations without
  improving the semantic model.

### 4. Keep recursive consumers consistent and precedence-aware

Every consumer of `TypeRef` and `Type` will treat composed suffixes as ordinary recursive wrappers:

- parser validation must allow composed suffixes while rejecting redundant same-layer `?` suffixes
- type analysis and compatibility checks must accept the newly legal annotations
- generated TypeScript and C# output must preserve nested-list and nullable-list structure
- display/diagnostic formatting must parenthesize wrapped function types when needed so rendered
  types remain unambiguous

Rationale:
- The change is only complete if the entire toolchain agrees on the same structure.
- Most backends already recurse correctly, so the main work is tightening edge cases and locking
  them down with tests.

Alternative considered:
- Limit the change to parser acceptance and leave rendering/diagnostic cleanup for later.

Why rejected:
- That would make newly accepted source types round-trip inconsistently and leave obvious user-facing
  ambiguity in error messages or debug output.

## Risks / Trade-offs

- [Composed suffixes can expose previously untested precedence bugs in type rendering] → Mitigation:
  add focused tests for nested lists, nullable lists, list-of-nullable elements, and wrapped
  function types.
- [Relaxing the grammar may accept forms that parser diagnostics previously treated as invalid] →
  Mitigation: update syntax fixtures and validation expectations together with the grammar change.
- [Target-language nullability differs between TypeScript and C#] → Mitigation: add generator tests
  that assert distinct outputs for `T?[]` versus `T[]?` in both languages.

## Migration Plan

1. Update the formal grammar and parser grammar to allow repeated postfix type suffixes.
2. Adjust lowering, validation, and any type-display helpers that assume only one suffix.
3. Add parser, lowering, semantic, and generator coverage for `T[][]`, `T?[]`, `T[]?`, and
   invalid duplicate-nullable forms such as `T??`.
4. Update reference docs and examples to use the new grammar consistently.

This is an additive language change. No data migration is required, and rollback is limited to
restoring the old grammar and associated tests.

## Open Questions

No blocking questions. If NX later formalizes function type syntax in the public grammar, that work
should reuse the same precedence rules introduced here for wrapped suffix rendering.
