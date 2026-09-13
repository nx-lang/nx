# Follow-ups from `add-update-actions`

Work identified while reviewing and fixing the `add-update-actions` change that was deliberately
left for later.

The property design this file opened with has landed in full, in three changes: the language-side
property reference and its intrinsics, the .NET SDK's map-backed storage and typed keys, and the
cross-module fix underneath both. Two-way binding, the one piece of that design still unbuilt, has
no present plans — see "Not planned" below.

Open items come first; what has landed is recorded at the bottom so the history stays readable
without getting in the way.

## Open

### 1. Synthesize `T.Update` on demand

Lowering synthesizes an `Item::Record` named `<Name>.Update` for every record, action, inline
emit, and component with state, appended at the end of every module. NX IR emits only the ones a
program references; typegen emits a companion for every exported declaration by spec; library
interfaces carry the synthesized items and typegen filters them back out on import.

The lazy alternative is to build the definition at the resolver: when `resolve_record_definition`
is asked for `X.Update` and `X` resolves to a record-shaped declaration, synthesize the `RecordDef`
there. `records.rs` already derives the update shape from the target. Every consumer that resolves
by name gets it for free, nothing that enumerates items sees it, and an imported `X.Update` derives
from the imported `X` instead of needing the library to export it. Worth doing only if the item
bloat shows up in the performance tests or the language service; the eager form is simpler to
reason about.

### 2. Host record construction runs outside the call's resource limits

`construct_host_record_value` in `crates/nx-interpreter/src/interpreter.rs` rebuilds every
host-supplied record from its fields, and evaluates any default a nested plain record needs in a
fresh `ExecutionContext::new()`. That is what `construct_external_record_value` already did for the
action record at dispatch, so nothing got worse, but it means the `ResourceLimits` a host passes to
initialization, evaluation, or dispatch do not bound default evaluation for records inside its own
input. Thread the call's context (or at least its limits) into the host construction path once
those defaults can do real work; today they are literal or near-literal.

### 3. Union case payloads from the host reach the case builder by name only

The same path builds a host-supplied payload union case (`{ $type: "Shape.Circle", r: 1 }`) through
`resolve_union_case_definition`, which resolves the union by name from the field's owner module. A
case the owner module cannot name by that spelling (a library-local alias, or an import the host
knows but the module does not) is kept as supplied rather than checked. Plain records go through
`resolve_record_definition` with the same limit. Resolving by declaring origin, as
`eval_resolved_union_case` does for authored cases, would close it; no scenario needs it yet.

### 4. A transitive origin generates a reference to a library the package does not depend on

`resolve-inherited-companion-field-types` resolves an inherited companion field's type in the
module that wrote it, so when `people` imports `Named` from `named`, and `named` typed the field by
`Tag` from `tags`, the generated `people` output carries `import type { Tag } from "tags"` and
`global::<Tags.Namespace>.Tag`. That is what an explicit import would produce, but `people` has no
declared dependency on `tags`, and for npm and NuGet a direct reference needs a direct dependency.
The existing "assumed package" warning says the package *name* may be wrong, not that the edge is
undeclared. Either the generated package manifest and project references need the transitive
origin added, or the warning should say the origin library is not a direct dependency when that is
the case.

### 5. Loose ends in the .NET shape

- `Diff` compares nested plain records through their JSON encoding, since a generated record has
  no structural `Equals` and the SDK does not reflect over its members. That matches
  `nxValuesEqual`; a generated `Equals` would be the tidier long-term answer.
- A field whose C# member name matches a base-class member (`Fields`, `Schema`, `Apply`) hides it,
  explicitly with `new`; the base surface is still reachable through an `NxUpdateRecord`-typed
  reference. The companion's own members step around field names with a trailing underscore.
- Composing schemas across an inheritance edge (`NxSchema.Merge(Named_update.Schema, OwnSchema)`)
  was never needed: a companion already carries its inherited fields with resolved types, so its
  key table covers them directly. Noted only so the idea is not re-derived.

## Not planned

Named in the `add-property-references` proposal as out of scope, and recorded here so they are not
lost. None is scheduled; each would need its own change and its own case for being worth building.

- **Two-way binding**, `<TextInput bind=query />` with a component-side `binds` clause pairing a
  prop with an emitted field. This is the piece of the original property design that was never
  built. It remains the most interesting of these for NX's domain — it is the first real consumer
  of a per-case field type inside the checker — but there are **no present plans to implement it**.
  The .NET and TypeScript helper shapes that landed in `map-backed-update-records` and
  `add-property-references` are the typegen target whenever it is picked up.
- **Computed keys in construction**, `<Update {prop}={value} />`.
- **A generic `get(record, property)` read**, whose result would be `object` for an open key. Note
  that `add-component-type-parameters` does not enable this: its type parameters are scoped to a
  component signature, and parameters on records, functions, aliases, and unions are explicitly out
  of scope and staying out.
- **Nested paths and list operations**, which would turn the merge-patch model into JSON Patch.

## Landed

### `resolve-inherited-companion-field-types` (archived 2026-09-11)

Resolves an inherited companion field's type in the module that wrote it, and closes the test gaps
the `add-update-actions` review left open. Its own loose end is item 4 above.

### `add-property-references` (archived 2026-09-12)

Gave the language the property half of the design this file opened with, under a different shape
than the `KeyPath`-style reference first sketched. NX has no general-purpose generics — the type
parameters added later are scoped to a component signature and do not extend to records — so the
property reference is a derived **constant union** `T.Property` whose cases are `T`'s effective
field names. It resolves bare at a typed site, matches exhaustively, and serializes as the bare
field name.

Four intrinsics operate on the map that an update record already is: `apply(record, update)`,
`merge(first, second)`, `diff(before, after)`, and `changed(update): T.Property[]`. The TypeScript
IR runtime exports the same four as typed helpers, and typegen emits a `<Name>_property` companion
for every exported record, action, and stateful component: a string literal union in TypeScript,
and a C# `enum` with the SDK's existing enum wire-format helpers.

The `T.Update` type, the element-shaped construction syntax, the absent-versus-null rule, the
`$type` discriminator, and the JSON and MessagePack encodings did not change and are not expected
to.

### `map-backed-update-records` (archived 2026-09-12)

Replaced the generated .NET storage. A generated `<T>_update` derives from `NxUpdateRecord`, which
keeps the patch as an ordinal `Dictionary<string, object?>` and exposes it through `Fields`,
`IsSet`, and `Unset`; the generated `NxOptional<T>` properties are `Get`/`Set` accessor pairs over
that map, so `new User_update { Email = null }` still means "set email to null". Each companion
passes an `NxUpdateSchema` (the `$type` discriminator and its fields in declared order) to the
base, and one `NxUpdateRecordJsonConverter<T>` and one `NxUpdateRecordMessagePackFormatter<T>`,
named on the class, write `$type` and the set fields in ordinal key order from that schema. The
bytes on the wire did not change. An unknown key on read now throws, naming the key and the DTO,
where the reflection formatter used to drop it. Both `NxOptional<T>` converters, the converter
factory, the per-property wire attributes, and the reflection-based formatter are gone;
`NxOptional<T>` is an in-memory value only.

Typed keys build on the property enum. Typegen emits a `<T>Properties` class beside each record or
action that has a plain generated type: one `NxProperty<T, TValue>` per field, named through the
generated `<T>_propertyWireFormat`, and `Of(<T>_property)` mapping each enum case to its key. The
companion of such a target derives from `NxUpdate<T>`, which adds typed `Get(key)`/`Set(key, value)`
(C# has no generic indexers, so the `update[key]` spelling first sketched became a method pair),
`Apply(record)`, and `Diff(before, after)`; `Merge` and `ChangedNames` live on `NxUpdateRecord`, so
a component's `Counter_update`, which has no plain type and derives from the untyped base, still
merges and reports changes. Every companion narrows `Changed()` to its `<T>_property[]`. An
abstract record's companion also takes the untyped base, since nothing can instantiate the record
to apply a patch to it; an external component's companion binds to its `<Name>_state` record.

### `add-rust-ci-job` (archived 2026-09-12)

`build.yml` has a `🦀 Rust` job that installs pnpm and Node 24, restores the pnpm workspace so the
repository's own `tsc` is present, caches the Cargo build, and runs `cargo fmt --all --check` and
`cargo test --workspace` on every pull request and on the branches the rest of the workflow covers.
The codegen tests no longer skip: `node_command` and `tsc_command` panic with a message naming
Node 24 and `PATH`, or `pnpm install` at the repository root, so a run without either tool fails
instead of passing while executing nothing.
