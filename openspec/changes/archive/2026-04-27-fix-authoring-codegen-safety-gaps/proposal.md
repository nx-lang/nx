## Why

Several authoring and generation gaps currently force consumers to work around NX instead of relying
on its type system and generated outputs. Nullable list fields cannot be populated inline, generated
TypeScript omits imports for sibling-library references, and stale record fields can be silently
dropped during evaluation.

## What Changes

- Allow values of type `T[]` to bind to `T[]?` targets, including braced list literals assigned to
  nullable list fields, props, parameters, and annotated `let` bindings.
- Extend TypeScript library generation so cross-library exported type references emit type-only
  imports instead of unqualified unresolved names.
- Add an explicit TypeScript package prefix option for cross-library imports, and warn when the
  generator derives an import target from a dependency path rather than declared package metadata.
- **BREAKING**: Reject unknown fields during typed record construction and evaluation instead of
  silently discarding them.

## Capabilities

### New Capabilities
- `record-construction-validation`: Validation rules for record, action, and record-compatible
  literal construction, including strict handling of unknown fields.

### Modified Capabilities
- `typed-braced-expression-kinds`: Nullable list typed binding sites accept non-null list values
  produced by braced expressions and existing list expressions.
- `cli-code-generation`: TypeScript generation emits imports for exported type references owned by
  dependency libraries.

## Impact

- Type compatibility and binding-site checks for list and nullable-list types.
- Type checking and runtime evaluation paths that construct record-compatible values.
- TypeScript code generation for library dependencies, CLI options, diagnostics, and tests.
- Documentation and fixtures that describe nullable list authoring, cross-library TypeScript
  imports, and strict record-field validation.
