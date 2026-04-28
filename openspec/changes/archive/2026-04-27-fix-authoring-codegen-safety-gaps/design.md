## Context

NX already distinguishes `T[]` from `T[]?` in type references and generated type surfaces. The type
compatibility layer supports `T` to `T?` widening for scalar types, but list-valued braced
expressions still fail when the binding target is a nullable list. This makes optional list fields
valid in schemas but awkward or impossible to author inline.

Library code generation already has partial cross-library metadata. The model records imported
types for C# so the C# generator can derive dependency namespaces and warn when those namespaces
are assumed. The TypeScript generator currently emits relative imports only for declarations owned
by another generated module in the same output graph; imported dependency declarations can remain
as unqualified, unresolved TypeScript names.

Record-compatible construction now has several validation paths. Type checking rejects unknown
properties in many element-style bindings, while runtime record construction can still receive an
override map and build a value by known fields only. The runtime path needs the same closed-record
behavior so stale authored fields cannot be discarded silently when evaluation is reached directly
or through a lower-level API.

## Goals / Non-Goals

**Goals:**
- Treat non-null list values as assignable to nullable list targets without weakening the distinction
  between `T?[]` and `T[]?`.
- Make braced list literals authorable for nullable list fields, props, parameters, and annotated
  bindings.
- Emit TypeScript type-only imports for dependency-library references, using an explicit
  `--typescript-package-prefix` when supplied and a documented assumed package name otherwise.
- Warn for assumed TypeScript dependency package names, matching the existing C# warning pattern.
- Reject unknown fields in typed record-compatible construction during static analysis and runtime
  evaluation.

**Non-Goals:**
- Do not add a new syntax for null versus empty list values.
- Do not make `T[]?` assignable to `T[]` or make `T?[]` interchangeable with `T[]?`.
- Do not introduce a package manifest format for per-library TypeScript package names in this
  change.
- Do not change intrinsic element behavior; strict field validation applies to NX record-compatible
  targets with declared fields.

## Decisions

### Reuse nullable widening for list layers

`Type::is_compatible_with` already implements `T` to `T?` by checking compatibility with the
nullable inner type. The implementation should preserve that recursive rule for `Type::Array`, so
`T[]` is compatible with `T[]?` because the nullable target unwraps to `T[]`, then array element
compatibility checks `T` against `T`.

Binding-site coercion should keep its current scalar-to-list rule and apply compatibility after the
target list layer is normalized. This lets `{ <Item/> <Item/> }` bind to `Item[]?` without treating
`Item?[]` as equivalent. Diagnostics should continue to render the full composed type shapes when
an element type mismatch remains.

Alternative considered: contextually infer braced list literals as nullable lists when an annotation
is present. That would solve only braced literals and leave ordinary list values unable to bind to
nullable list targets. Widening at compatibility points is smaller and matches scalar nullable
behavior.

### Keep TypeScript dependency imports package-based

Same-library TypeScript imports should remain relative because both modules are emitted under one
output root. Cross-library imports should be package imports because the generator does not know
where sibling library outputs will be placed relative to the current output root.

Add `typescript_package_prefix: Option<String>` to generation options and expose it on
`nxlang generate` as `--typescript-package-prefix`. When a generated module references an imported
type whose owner is a dependency library, the TypeScript emitter should import from:

```text
<typescript-package-prefix><sanitized-dependency-library-name>
```

If no prefix is supplied, the package target is the sanitized dependency library name alone. In both
cases the generator should emit one warning per dependency package because the target is assumed
from the dependency directory name until a manifest provides an explicit package name.

Imported visible names must be handled separately from exported names. When the generated local type
name differs from the exported dependency name, emit a type import alias such as
`import type { QuestionFlow as Flow_QuestionFlow } from "@org/nx-question-flow";` so generated type
references match the names written in the local output.

Alternative considered: derive relative imports to sibling generated directories. That makes
generated output depend on a particular monorepo layout and does not work when each library is
published as a package. A package prefix is a clearer contract and mirrors the C# namespace
assumption warning.

### Validate record overrides before defaulting or coercion

Centralize record-compatible construction through a helper that receives the effective record shape
and the supplied property/override map. Before applying defaults or type coercions, compare every
supplied field name with the effective shape. Unknown names should produce a type diagnostic during
static analysis and a runtime error during direct evaluation.

The same helper shape should be used for plain records, action records, inherited record shapes,
and record-compatible element construction. Union case payloads already have their own unknown-field
diagnostic; they should stay aligned but do not need to move into this helper unless that is the
simplest implementation.

Alternative considered: warn and continue for unknown fields. That preserves compatibility but
does not provide migration safety, which is the reason this change exists. The proposal intentionally
makes closed record construction the default.

## Risks / Trade-offs

- Existing authored sources with stale record fields will start failing. Mitigate by using a clear
  diagnostic code and message naming the target record and field.
- Assumed TypeScript package names may not match every package layout. Mitigate with
  `--typescript-package-prefix` and a warning that points at the derived package target.
- TypeScript import aliasing for namespaced or selective imports can expose gaps in current exported
  type graph metadata. Mitigate by adding focused model tests before emitter tests.
- Compatibility widening must not collapse `T?[]` and `T[]?`. Mitigate with direct unit tests for
  both successful nullable-list widening and rejected list-of-nullable mismatches.

## Migration Plan

1. Add type compatibility and binding-site tests for nullable list authoring before changing the
   implementation.
2. Add TypeScript generator model and emitter tests for dependency-library imports, package-prefix
   handling, assumed-package warnings, and import aliasing.
3. Add type-checker and runtime tests for unknown record fields across record literals,
   element-style record construction, action records, and inherited fields.
4. Implement the changes behind the existing CLI and runtime surfaces, adding only the
   `--typescript-package-prefix` option.
5. Update language and CLI documentation after behavior is implemented, and verify no stale future
   notes remain for this completed work.

Rollback before archive is limited to reverting this change directory and the implementation
branch. After implementation, users who hit strict unknown-field errors should remove stale fields
or add them back to the target NX schema.
