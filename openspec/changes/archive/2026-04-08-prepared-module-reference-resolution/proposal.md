## Why

NX currently binds names through several disconnected mechanisms: lowering-time temporary scopes,
`LoweredModule::find_item()`, a partial lexical `ScopeManager`, type-checker side maps and
`TypeEnvironment`, and runtime `ResolvedProgram` lookup tables. Prepared analysis currently makes
peer and imported names visible by copying or synthesizing HIR items into transient modules so
later phases can continue doing string-based lookups. That fixes isolated bugs like record
inheritance, but it leaves no single symbol model, forces each phase to rediscover names
independently, and makes cross-file and imported resolution fragile.

## What Changes

- Keep raw lowering strictly file-local and limit it to syntax-to-HIR normalization plus
  declaration checks that do not depend on peer files, imports, or visible-name lookup.
- Introduce an explicit `PreparedModule` step that separates preserved raw HIR from the prepared
  visible namespace used for analysis.
- Introduce a binding model with stable visible-symbol bindings and stable local definition
  identities for top-level declarations, including their origin as local, same-library peer, or
  imported library interface definitions.
- Replace prepared-analysis HIR copying for same-library and imported visibility with binding-table
  construction against raw modules and library interface metadata.
- Make prepared-module semantic validation, scope building, and type analysis resolve names through
  prepared bindings rather than `find_item()` and ad hoc side maps.
- Unify lexical binding so local scopes layer over prepared top-level bindings instead of living in
  a separate partially implemented symbol system.
- Update library, program, and runtime assembly to derive import/export/runtime lookup tables from
  symbol bindings and module-qualified definition references rather than rescanning modules by
  string name.
- Move record inheritance to the binding-based prepared semantic layer as the first migrated
  name-resolution-dependent validation family.
- **BREAKING**: Remove record-specific lowering workaround APIs and any analysis or runtime APIs
  that assume prepared visibility is represented as cloned `LoweredModule` items or string-based
  top-level rescans.

## Capabilities

### New Capabilities
- `symbol-resolution-model`: Define prepared visible namespaces, stable definition identities, and
  binding-based lookup contracts shared by analysis and runtime assembly.

### Modified Capabilities
- `artifact-model`: Define the persistent loaded-library snapshot as raw per-file module artifacts
  plus stable definition and binding metadata, while keeping prepared modules transient.
- `source-analysis-pipeline`: Define an explicit raw-lowering, preparation, prepared-module
  binding construction, semantic validation, and scope/type analysis sequence.
- `record-type-inheritance`: Resolve abstract base records through prepared bindings, including
  same-library peer declarations, imported library interfaces, and alias chains.
- `resolved-program-runtime`: Resolve runtime-visible item references through module-qualified
  definition identities instead of name-based rescans.

## Impact

- Affected code: `crates/nx-hir/src/lower.rs`, `crates/nx-hir/src/lib.rs`,
  `crates/nx-hir/src/scope.rs`, `crates/nx-api/src/artifacts.rs`, `crates/nx-types/src/check.rs`,
  `crates/nx-types/src/infer.rs`, `crates/nx-interpreter/src/resolved_program.rs`,
  `crates/nx-interpreter/src/interpreter.rs`, and related tests.
- Affected APIs: Lowering, prepared analysis, scope building, library/program artifact assembly,
  and runtime lookup entry points that currently depend on `find_item()` or copied prepared HIR.
- Affected behavior: Peer and imported name resolution, semantic validation, type checking, and
  runtime entry/import lookup all move onto one prepared binding architecture while stored artifacts
  remain file-preserving and loaded libraries persist raw snapshots plus stable library-owned
  binding metadata rather than prepared modules.
