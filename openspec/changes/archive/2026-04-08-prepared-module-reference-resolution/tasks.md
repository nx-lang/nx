## 1. Definition And Prepared-Module Model

- [x] 1.1 Introduce stable local definition identities for top-level declarations and an explicit
  `PreparedModule` type that separates preserved raw `LoweredModule` state from prepared visible
  bindings.
- [x] 1.2 Change raw lowering so `lower()` is strictly file-local, preserves unresolved prepared
  references, and removes the current record-specific lowering workaround API.
- [x] 1.3 Update standalone/shared analysis helpers to build a trivial prepared module and binding
  table before validation, scope building, and type checking.

## 2. Binding Construction

- [x] 2.1 Populate prepared local bindings from raw module definitions and expose shared resolver
  APIs for value, type, and element-like lookup.
- [x] 2.2 Populate same-library peer bindings and imported library-interface bindings without
  cloning foreign HIR into analysis modules.
- [x] 2.3 Move duplicate-name, ambiguity, and visibility diagnostics that arise during namespace
  construction onto the binding-building phase.

## 3. Analysis Consumers

- [x] 3.1 Add a centralized prepared-module semantic-validation step in the shared analysis path
  and move record inheritance onto binding-based resolution.
- [x] 3.2 Migrate `ScopeManager` and undefined-name checking so lexical scopes compose with prepared
  top-level bindings instead of remaining a disconnected partial resolver.
- [x] 3.3 Migrate type inference and type-reference resolution away from `find_item()` and
  ad hoc alias/enum side maps toward the shared prepared binding APIs.

## 4. Library, Program, And Runtime Assembly

- [x] 4.1 Update `LibraryArtifact` so its persistent loaded snapshot stores raw per-file
  `ModuleArtifact`s plus stable definition identities, export/library-visible binding indexes, and
  interface metadata, without persisting prepared modules.
- [x] 4.2 Update `ResolvedProgram` import and entry tables to use module-qualified definition
  references instead of name-based rescans.
- [x] 4.3 Update interpreter lookup to resolve functions, components, records, and imported items
  from module-qualified definition references.

## 5. Verification

- [x] 5.1 Add regression tests showing record inheritance resolves across same-library peer files,
  imported libraries, and alias chains through prepared bindings.
- [x] 5.2 Add analysis tests for lexical shadowing and undefined-name diagnostics when local scopes
  layer over prepared top-level bindings.
- [x] 5.3 Add artifact and runtime tests showing loaded libraries persist raw snapshots plus stable
  library-owned binding metadata while cross-module entry and import lookup uses module-qualified
  definition references rather than string-based rescans.
