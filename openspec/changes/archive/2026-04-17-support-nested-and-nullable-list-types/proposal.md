## Why

NX currently treats list and nullable markers as one-off type modifiers, which rejects syntax that
users naturally expect to work, such as `string[][]` and `string[]?`. That limitation is now
hurting clarity more than it helps simplicity because the internal type model, generated-language
backends, and documentation already imply that these composed list/nullability forms should exist.

## What Changes

- Allow type references to accept repeated postfix suffixes instead of a single optional modifier.
- Support nested list types such as `string[][]`, `User[][]`, and deeper list nesting.
- Support nullable list types such as `string[]?`, alongside existing list-of-nullable forms such
  as `string?[]`.
- Define suffix ordering semantics so `T?[]` means a list of nullable `T` values, while `T[]?`
  means a nullable list of `T`.
- Reject redundant nullable suffixes on the same outer type layer, such as `string??` or
  `string?[]??`, while still allowing `string?[]?`.
- Update parsing, lowering, type display, diagnostics, tests, and generated-language output to keep
  composed suffix types coherent across the toolchain.

## Capabilities

### New Capabilities
- `type-reference-suffixes`: Define composed postfix list and nullable suffixes for NX type
  references, including nested lists and nullable lists.

### Modified Capabilities
- `cli-code-generation`: Preserve composed list and nullable type references in generated
  TypeScript and C# exported type surfaces.

## Impact

- Affects the formal grammar in `nx-grammar.md` and the parser grammar in `crates/nx-syntax`.
- Affects HIR lowering and type-reference handling in `crates/nx-hir` and `crates/nx-types`.
- Affects generated type surfaces in `crates/nx-cli` for TypeScript and C#.
- Requires updated parser, type-system, codegen, and documentation coverage for composed suffix
  forms.
