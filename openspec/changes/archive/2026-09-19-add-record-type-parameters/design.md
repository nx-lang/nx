## Context

See proposal.md for motivation. This design extends
`openspec/changes/archive/2026-09-13-add-component-type-parameters/design.md`, which is the model
for everything here; what follows records only where records differ.

What exists today:

- **Syntax already parses.** `_property_type` (`grammar.js:404`) admits the `type` keyword as a
  property type everywhere, so `type Range = { T:type start:T }` produces a well-formed tree.
  `validate_type_parameter_definition` (`validation.rs:504`) rejects it because the parent is not a
  `COMPONENT_SIGNATURE`, and record lowering silently drops the field
  (`lower_record_fields_from_node`, `lower.rs:872`).
- **There is an element-shaped type already.** `function_type` is `'<' 'function' … '/' '>' ':' type`,
  a sibling of `user_defined_type` in the `type` rule.
- **`ast::TypeRef` has four variants** (`Name`, `Array`, `Nullable`, `Function`) and is reused
  as-is by typegen, the library interface and the interpreter. It has no way to carry arguments.
- **A record type is `Type::Named(NamedType { name, origin })`**, equal by declaring origin. A
  component element has that type whatever its arguments, which is what "erased" means.
- **The checker's use-site machinery is component-only**: `resolve_type_arguments`,
  `type_parameter_scope`, `consumed_type_arguments` and `remove_property_entries` serve
  `check_element_bindings_against_component`. Records have two construction paths that know
  nothing of it: a same-module, content-free element lowers to `Expr::RecordLiteral`
  (`lower.rs:1432`) and is checked by `infer_record_literal`; any other lowers to an element and is
  checked by `check_element_bindings_against_record`.
- **Field types survive to runtime.** The interpreter coerces host values against
  `EffectiveField.ty`, NX IR has a type table (`ir.rs:102`, kinds 0–4), and the TypeScript runtime
  normalizes against it. Components avoided an IR change by erasing parameters to `object`
  (`erase_type_parameters`, `components.rs:1094`). IR is schema 4, released as v0.2.0.
- **C# typegen erases component parameters; TypeScript keeps them.** The component design's D6
  anticipated this change: generic *records* map to real C# generics, because the host writes the
  concrete type at its own deserialization site.

## Goals / Non-Goals

**Goals:**

- One representation of "a record with arguments" in each layer — `TypeRef::Applied` in syntax-level
  types, arguments on `NamedType` in checked types — so distinctness and invariance hold by
  equality rather than by extra checks.
- The checker's type-argument machinery is shared between components and records, not copied.
- Nothing below the checker learns about type arguments. The existing erasure rule is extended to
  one more `TypeRef` variant and that is the whole runtime story.
- (Folded in) `cargo clippy --workspace --all-targets -- -D warnings` exits 0 under the pinned
  toolchain and CI fails when it stops doing so, with no behavior change: `cargo test --workspace`,
  `pnpm -r build` and `pnpm -r test` stay green.

**Non-Goals:**

- No inference of type arguments, at construction or anywhere else. The follow-up `Range` change
  types `a..b` by rule in the checker and does not need it.
- No inheritance for generic records (`abstract`, `extends`, extending an applied type). It needs
  parameter merging, subtyping with arguments, and open-generic polymorphic serialization in C#;
  none of it is needed by the records this change is for.
- No type parameters on aliases, unions, actions, functions or function types.
- No variance. Applied types are invariant; there are no constraints on a parameter either.
- No runtime check of a host-supplied value against a type argument. A `<Range T=int/>` prop
  accepts any `Range` at the host boundary, as a `TItem[]` prop accepts any list today.
- No completions inside an applied type; the language service must only not regress.
- (Folded in) No lint groups beyond clippy's defaults — no `pedantic`, no `nursery` — and no
  restructuring to satisfy `too_many_arguments` or `large_enum_variant` where the current shape is
  deliberate; those are allowed with a comment. The FFI surface and the .NET bindings do not change.

## Decisions

### D1: `RecordDef` carries `type_params` separate from `properties`

`RecordDef` gains `type_params: Vec<TypeParameter>`, filled by the existing
`lower_type_parameters`, which already takes any node with `property_definition` children. The
field filter at `lower.rs:872` stays. `EffectiveRecordShape` gains the parameters so consumers that
hold a shape need not find the declaration again, and `InterfaceItemKind::Record` gains them so a
generic record crosses a module boundary, as `InterfaceItemKind::Component` already does.

*Why:* the component design's D1 reasoning applies unchanged — every consumer of `properties` is a
value-level surface, and keeping parameters out of it makes each exclusion in the spec (not a
field, not a `Property` case, not a missing property) true by construction.

Validation changes in one place: the early return at `validation.rs:526` admits
`RECORD_DEFINITION` beside `COMPONENT_SIGNATURE`, after which the remaining placement, name, default
and modifier checks run for records too. The wording "before every prop" becomes "before every
field" for a record. A generic record that is `abstract` or has `extends` is rejected in
`validate_record_definition` (`records.rs:554`), where the other inheritance rules live, because it
needs the lowered declaration rather than the tree.

### D2: `TypeRef::Applied { name, args }` and an `applied_type` grammar rule

```js
applied_type: $ => seq(
  '<', field('name', $.qualified_name),
  repeat(field('arguments', $.type_argument)),
  '/', '>',
),
type_argument: $ => seq(field('name', $.identifier), '=', field('type', $.type)),
```

added to the `type` choice. `TypeRef::Applied { name: Name, args: Vec<(Name, TypeRef)> }` keeps
arguments in source order; the checker reorders them. `spell_type_ref` prints `<Range T=int/>`, and
the two suffix-aware spellers need no parentheses around it because `/>` closes it.

*Why a full `type` for the argument, when a construction site only takes a bare name:* in a type
position the parser already knows it is reading a type, so `T=int[]` and a nested applied type cost
nothing. A construction site goes through `property_value`, where the right-hand side is an
expression and a bare name is the only spelling that reads unambiguously as a type; that rule is
the component spec's and is not reopened. An alias bridges the two.

*Why `repeat`, not `repeat1`:* `<Range/>` in a type position should get "`T` was not specified",
not a parse error.

*Disambiguation:* `function` is a keyword only in this state, so `<function …/>:` and
`<Name …/>` split on the first token after `<`, and a function type still requires its `:` result.
A record named `function` cannot be applied, which costs nothing.

*Alternative considered:* `Range<int>`. Rejected in discussion: NX binds type arguments by name
with `T=int` everywhere else, and a positional form would be a second way to say the same thing.

### D3: Type arguments live on `NamedType`

`NamedType` gains `args: Vec<Type>` in the record's declaration order, empty for every non-generic
type. `PartialEq`/`Hash` include it, so two instantiations are different types, a map keyed by
`NamedType` keeps working, and invariance in `is_compatible_with` is the existing equality with no
new arm. `Display` prints the applied spelling, so every existing mismatch diagnostic names both
instantiations for free. Aliases are already resolved before a `Type` is built, so
`<Range T=Count/>` and `<Range T=int/>` are equal.

*Alternative considered:* a new `Type::Applied` variant. Rejected: every `match` on `Type::Named`
that means "a record" — field access, subtyping, joins, `object` satisfaction, companions — would
need a second arm that does the same thing.

Substitution is the existing `Type::substitute_parameters`: reading field `f` of a value of type
`R[args]` resolves `f`'s `TypeRef` under a scope that maps `R`'s parameters to rigid markers, then
substitutes `args`. Inside `R`'s own declaration, fields are checked under the rigid scope exactly
as a component body is (`rigid_type_parameter_scope`), which is what rejects `value:T = "text"`.

`record_type_satisfies_expected` compares declarations through `is_record_subtype`; it must also
require equal `args`. Generic records have no ancestors (D1), so this is one comparison.

### D4: One resolver for applied types, one for construction arguments

**Type position.** `type_from_type_ref_in` gains the `Applied` arm: resolve the tag to a record
declaration, read its parameters, match arguments by name, resolve each argument `TypeRef`
recursively, and report — each by record and argument name — an unknown argument, a duplicate, a
missing parameter, and a tag that is not a generic record. The `TypeRef::Name` arm reports a bare
generic record name with the missing-argument diagnostic, showing the applied form. On any of
these the result is `Type::Error`, so one mistake is one diagnostic.

**Construction.** `resolve_type_arguments` already turns an element's `Name=TypeName` properties
into a scope, a consumed set and a list of unspecified parameters, and already reports braced,
quoted and conditional arguments. It is reused as is, with two differences at the record call
sites: an unspecified parameter is an error reported once for the element
(`type-parameter-not-specified`, with the `T=` form) rather than a bottom-type fallback, and the
resulting type is `NamedType` with the resolved `args` rather than the bare nominal type. When a
parameter is unspecified the field bindings that mention it are skipped, not checked against
`never`, so the author sees the one real problem.

Both record paths need this: `infer_record_literal` and `check_element_bindings_against_record`.
The shared part — resolve arguments, swap the scope, build the field spec, restore — is factored
out of `check_element_bindings_against_component` rather than written a third time.

*Removal.* `remove_property_entries` drops consumed entries from an element. `Expr::RecordLiteral`
holds `RecordLiteralProperty` values, not element entries, so the same pass gains a branch that
drops a record-literal property whose value `ExprId` was consumed. After it, nothing below the
checker can see `T=int`, which is what the runtime requirement in the spec asks for.

### D5: Companions copy the parameters; intrinsics carry the arguments

`predeclare_update_record` takes the target's `type_params` and puts them on the derived
`RecordDef`, so `Range.Update` is an ordinary generic record and `<Range.Update T=int/>` resolves
through D4 with no special case — the tag is already a qualified name. The derived `Property`
union is built from field names and is untouched.

`infer_intrinsic_call` forms `R.Update` from `R` by name today; it copies `args` across when it
does. That is the whole of the intrinsic change, and it makes `apply(<Range T=int/>, …)` expect
`<Range.Update T=int/>` by the same equality as everything else.

*Alternative considered:* no companions for generic records. Rejected: `update-records` says every
record-shaped declaration has one, and an exception would need its own diagnostic and its own spec
text for less benefit than the two small changes above.

### D6: Erasure below the checker, extended to `Applied`

`erase_type_parameters` gains an `Applied` arm that recurses into arguments, which is what makes a
component prop `range:<Range T=TItem/>` erase correctly. Separately, every place that turns a
`TypeRef` into a runtime or IR type treats `Applied { name, .. }` as `Name(name)`:

- interpreter: `runtime_type_from_type_ref`, and a generic record's own fields are erased with its
  parameters before coercion, mirroring `erase_contract_type_parameters`;
- IR: `CodegenTypeRef` is unchanged — an applied type resolves to `Nominal`, a parameter-typed
  field to `Primitive { "object" }` — so the type table keeps kinds 0–4 and the schema stays 4;
- TypeScript IR runtime: no change, since the image is unchanged.

*Alternative considered:* a type-table kind 5 for applied types, with runtime substitution, so a
host-supplied `<Range T=int/>` is validated field by field. Rejected for now: it is an IR schema
bump the day after schema 4 shipped, for a check no program depends on, and components already
accept the same looseness at the same boundary. It can be added later without touching source
semantics.

### D7: Typegen and executable codegen emit real generics

`ExportedRecord.type_params` is already there and is filled only for component contracts; plain
records fill it from `RecordDef.type_params`. TypeScript's `ts_generic_declaration` /
`ts_generic_arguments` then apply unchanged, except that a record's parameter has no `= unknown`
default: NX never leaves a record argument unspecified, so a TypeScript caller should not be able
to either.

C# stops erasing when the record is a plain generic record: `emit_record` erases only for a
component contract, and otherwise declares `Name<T, …>`. Generic records are concrete and outside
every polymorphic hierarchy (D1), so none of the `[JsonPolymorphic]` / MessagePack union attributes
apply to them.

**The update companion follows the record.** `update_companion_type_params` answers with the
record's parameters for a plain generic record and with none for anything else, so `Range_update<T>`
extends `NxUpdate<Range<T>>` and `RangeProperties<T>` keys it. The first draft erased here, by
analogy to a component's companion; the analogy does not hold. A component patches `Ticker_state`,
which is concrete, and its parameter-typed fields are `object` on *both* sides of the patch, so an
untyped value flows through. A generic record's fields are `T`, so an erased patch and the record
disagree — and in C# the disagreement is not a loss of precision but a failure: a JSON round trip
deserializes `start` as the schema's `typeof(object)`, and applying that to a `Range<long>` throws
`InvalidCastException`. The same argument that stops `emit_record` erasing stops the companion
erasing: the host names the instantiation at its own deserialization site.

Two C# mechanics fall out. `[JsonConverter(typeof(NxUpdateRecordJsonConverter<Range_update<T>>))]`
does not compile — CS0416, an attribute argument cannot use type parameters — so a generic
companion names a new non-generic `NxUpdateRecordJsonConverterFactory` in the SDK, which closes the
converter over whichever instantiation is serialized. MessagePack's attribute resolver *does* take
an open generic, but it closes it over the *annotated type's* arguments, so naming
`NxUpdateRecordMessagePackFormatter<>` would close it over `long` rather than over
`Range_update<long>`; typegen emits a shim `Range_updateFormatter<T>` of matching arity that
delegates to the real formatter. Neither touches the wire format, which stays `$type` plus the set
fields. A non-generic companion keeps naming its converter and formatter directly.

*Alternative considered:* keep the companion non-generic and make `Apply`/`Diff` generic *methods*
over `Range<T>`. Prototyped: it compiles, keeps both attributes unchanged, and makes the two
helpers reachable at a real instantiation — but the schema stays one non-generic thing, so
`NxField.ValueType` stays `object` and the JSON round-trip-then-apply still throws. It fixes the
signature and not the defect underneath it.

TypeScript follows for symmetry rather than necessity: a TS companion is a structural interface, so
erasure there costs precision (`start?: unknown`) and not usability. `Range_update<T>` with
`start?: T` is what a caller holding a `Range<number>` expects, and costs nothing — TS has no
attribute problem and no runtime generics.

Both `ts_type` and `csharp_type_inner` gain the `Applied` arm: look up the record's parameter
order, map each argument with the same function, and print `Name<A, B>`. The record's parameter
order comes from the export model, so an applied type that names a record from another module of
the library resolves the same way a cross-module name does today. Update companions keep using
`erase_field_type_parameters`, which already takes the parameter list.

A record imported from a *dependency* library is not in the export graph at all — it lives in the
module's `imported_types` — so `ExportedTypeGraph::record_type_params` answers from either, and
`ImportedTypeKind::Record` carries the parameters the dependency's interface item declares.
C# has no type aliases, so an imported alias whose target is an applied type is expanded at every
use and must render the instantiation there too (`csharp_imported_alias_target_type`); TypeScript
imports the alias by name and lets the dependency's own file resolve it. Where the declaration
cannot be reached at all, the arguments are rendered in source order rather than dropped: the bare
name is an open generic, which is CS0305 in C# and TS2314 in TypeScript, and a checked program
wrote its arguments against a declaration that does exist.

Executable TypeScript (`emit.rs`) follows the same split: generic interface for the record,
`emit_generic_prop_type` for fields, instantiation for an applied type.

### D8: Clippy is fixed per crate and then gated (folded in from `fix-clippy-diagnostics`)

Under the pinned toolchain, `cargo clippy --workspace --all-targets --keep-going` reports about 140
diagnostics that predate this change. Deny-level: five `approx_constant` literals
(`nx-cli/src/format.rs`, `nx-hir/src/ast/expr.rs`, `nx-value/src/lib.rs`), sixteen
`not_unsafe_ptr_arg_deref` entry points in `nx-ffi`, and `E0432 unresolved import criterion` in the
`nx-syntax` benchmark. Warn-level, by count: `cmp_owned`, `too_many_arguments`, `needless_borrow`,
`large_enum_variant`, `manual_range_contains`, and about twenty singletons.

**Fix per crate, gate at the end.** Each crate is verified by
`cargo clippy -p <crate> --all-targets -- -D warnings`, so partial progress is visible and
reviewable, and the CI step lands last, once the workspace command already passes, so the gate is
never added red. The alternative — gate first with an `-A` per outstanding lint — would let the
allows outlive the cleanup.

**Mark the `nx-ffi` entry points `unsafe`.** `not_unsafe_ptr_arg_deref` is deny by default because a
safe `pub fn` that dereferences a caller-supplied pointer is unsound by Rust's rules. Each entry
point becomes `pub unsafe extern "C"` with a `# Safety` section stating the pointer contract. The C
ABI does not change, so the .NET P/Invoke side is untouched; only Rust callers in the crate's tests
need `unsafe` blocks. A crate-level `#![allow(clippy::not_unsafe_ptr_arg_deref)]` would keep
documenting the functions as safe when they are not.

**Delete the parser benchmark rather than declare `criterion` for it.** The original change proposed
declaring the dev-dependency on the grounds that the benchmark is the only performance check the
parser has. It is not a check. Its dev-dependency was never declared, so `cargo bench` has failed
with `E0432` since the file landed in its single commit on 2025-10-29 and it has never produced a
number; nothing in CI, the docs or `tools/` runs it. Worse, its generator emits
`fn name(x: int): int { return ...; }`, which is not NX — 31% of the bytes it measures at every one
of its three sizes are a syntax error, so its throughput figure would blend parsing with
tree-sitter's error recovery in a ratio nobody chose. Keeping it alive costs 34 crates in the
lockfile (`plotters`, `rayon`, `clap`, `tinytemplate` and their trees), which the new
`--all-targets` clippy gate then compiles. A benchmark worth having would parse the corpus already
in `examples/nx/` or `specs/ir-conformance/` rather than a synthetic generator that can rot without
anyone noticing; that is its own change, driven by a real performance question.

**Replace `approx_constant` literals, do not allow them.** Where the value is a mathematical
constant, use `std::f64::consts`; where it is test data that happens to look like one, change the
digits.

**Prefer fixing over allowing, and allow with a reason.** `cargo clippy --fix` handles the mechanical
lints. `too_many_arguments` and `large_enum_variant` are the two where the code may be right as it
is; each allow carries a comment naming why, and a crate-wide allow is acceptable where the pattern
is consistent across the crate — for example the `nx-codegen` emitters that thread many context
arguments.

## Risks / Trade-offs

- [`NamedType` equality now includes `args`] Any code that builds a `NamedType` for a generic
  record by name alone gets a type unequal to every real instantiation. → `nominal_named_type` and
  `resolve_named_type` are the two constructors; both are made to refuse a generic record without
  arguments (D4), so there is no path that produces the argument-less type silently. A checker
  test asserts `Range` alone is always a diagnostic.
- [`TypeRef` is matched exhaustively in many crates] A new variant breaks the build in each. → That
  is the point: the compiler lists every site. D6 gives the answer for all the runtime ones.
- [Two record construction paths] A fix applied to one and not the other would make same-module and
  imported generic records behave differently. → Every construction scenario in the spec is tested
  both in one file and across a library boundary.
- [MessagePack and `System.Text.Json` with open generic C# records] Closed instantiations serialize
  with the reflection-based resolvers; an AOT/source-generated resolver needs each instantiation
  named. → Documented in the typegen section of the docs; the .NET binding tests round-trip one
  generic record through both serializers.
- [Host boundary is not checked against type arguments] A host can pass a `Range` of strings where
  `<Range T=int/>` is declared. → Accepted (Non-Goals); the same is true of `TItem[]` today. D6
  records the way to close it.
- [Verbose construction] `<Pair TKey=string TValue=int key="a" value={1}/>` repeats what the values
  say. → Inference is the known follow-up; D4 keeps the component design's shape, where inference
  changes only how the substitution map is filled.

- [`cargo clippy --fix` changes semantics during the folded-in cleanup] → It applies only
  machine-applicable suggestions, and `cargo test --workspace` and `pnpm -r test` run after each
  crate's cleanup regardless.
- [`-D warnings` in CI makes a future toolchain bump fail on new lints] → Intended. The bump's pull
  request fixes them, which is what `update-rust-toolchain` did by hand.
- [Marking the FFI entry points `unsafe` breaks a Rust-side caller] → `cargo test --workspace` and
  `pnpm -r build` compile every caller in the repository; the .NET side calls through the C ABI and
  cannot observe the change.

## Migration Plan

Additive. No source that checks today changes meaning, the IR schema stays 4, and no released
runtime needs an update. `parser.c` and the other generated grammar files are regenerated in the
same commit as `grammar.js`.
