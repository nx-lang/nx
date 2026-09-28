## Why

A type reference that names no visible type is accepted without a diagnostic. `let f(x: MissingType)`
validates cleanly, and `type Broken = { id: MissingType }` loads as a library. The name becomes an
opaque nominal type, so a typo either passes silently or surfaces later as a confusing mismatch
("expects MissingType, found string") far from where it was written. NX already rejects an unresolved
name in a type *argument* (`unresolved-type-argument`, with a "did you mean" suggestion); every other
type position is unchecked. Hosts that compile tenant-authored NX, such as ReachMe's chat-link
configs, are the most exposed: their authors are the least able to diagnose the result.

## What Changes

- **BREAKING** Analysis reports `unresolved-type` for a type reference whose name does not resolve to
  a visible type — a primitive, a declared or imported record, union, alias, component or action, a
  built-in type such as `Range`, a type parameter in scope, or a derived name such as `T.Property` or
  `T.Update` — wherever a type is written: record, union-case and action fields, component props,
  state fields and emits, function parameters and return types, `let` annotations, type aliases and
  function types. An `extends` clause and a type argument already reject an unresolved name with
  their own diagnostics, which stay.
- The diagnostic names the unresolved type, labels the reference, and suggests the closest visible
  type name, as `unresolved-type-argument` already does.
- A name is resolved in the namespace of the module that wrote it (the `nominal-type-identity` rule),
  so a library's field types are checked when the library is analyzed, and a consumer never reports
  a library's own reference.
- Sources that relied on the silent acceptance fail validation; there is no compatibility mode.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `symbol-resolution-model`: a written type reference must resolve to a visible type, or analysis
  reports `unresolved-type`.
- `primitive-type-names`: an undeclared `void` is reported as `unresolved-type` at the reference, like
  any other undeclared name, rather than rejected only at a binding.

## Impact

- `crates/nx-types` (type-reference resolution in the checker), `crates/nx-hir` where type
  references are lowered (error-recovery stand-ins and where each type name was written), and
  `crates/nx-api` import preparation (which records imports that did not resolve); library loading,
  which fails a library whose declarations name an unresolved type.
- Every binding reports the new diagnostic through its existing diagnostics path; the language
  service and playground show it as they show other errors.
- Examples, fixtures and tests across the repository that name undeclared types must be fixed; the
  ReachMe built-in libraries, templates and seeds must keep validating cleanly.
