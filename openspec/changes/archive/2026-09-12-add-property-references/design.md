## Context

See proposal.md for motivation. The facts that shape the approach:

- `T.Update` is synthesized during lowering as a real `Item::Record` with
  `RecordKind::Update { target }`, predeclared before bodies lower so tags and annotations resolve,
  and appended after every authored item so definition identities (item indices) never shift
  (`crates/nx-hir/src/lower.rs`, `predeclare_update_record` and `pending_update_items`). Its
  fields come from the target's effective shape through the base chain
  (`records.rs`, `resolve_update_target_shape`). The bare `Update` tag is rewritten during
  lowering against `current_component` (`resolve_bare_update_tag`), and the checker reports the
  outside-a-component and unknown-target diagnostics (`infer.rs`, `report_unresolved_update_tag`).
- A constant union is `UnionDef { base: None, cases: all fieldless }`. Everything a property
  reference needs already hangs off that shape: bare contextual resolution against
  `Type::Union` cases (`infer.rs`, `resolve_contextual_name_in`), match exhaustiveness
  (`infer_match_expr`, `property_paths_for_match`), `Value::UnionCase { union, case }` at runtime
  with a bare-string canonical encoding (`nx-api/src/value.rs`), host-string lifting against the
  expected type (`interpreter.rs`, `coerce_value_to_resolved_type`), the IR `union` declaration
  with `isConstant` cases, the TypeScript runtime's constant-case normalization, and the typegen
  constant-union emitters (TS string literal union, C# enum with `INxEnumWireFormat`).
- NX has no generics and no intrinsic functions. `infer_call` handles only `Type::Function`;
  `eval_call` resolves the callee name through `resolve_item`. Dispatch already applies an update
  record to state field by field with re-validation (`interpreter.rs`, `apply_component_update`
  and `coerce_update_field_value`); the TypeScript runtime has `applyComponentStatePatch` and
  `normalizePatchFields`.
- typegen models companions as `ExportedType::Update` / `ExportedType::ExternalState` and
  resolves collisions and warning subjects through `generated_companion()`; derived update records
  in a dependency's interface are skipped and reached through the target's companion instead.

## Goals / Non-Goals

**Goals:**
- One new derived declaration kind, reusing the constant-union machinery end to end so that no
  site that already handles a constant union needs a special case for a property union.
- Intrinsics typed by rule, isolated behind one check-side and one eval-side branch, so adding a
  fifth later is local.
- Unchanged generated code and typegen output for programs that never touch the feature. IR for
  such a program differs in exactly one way: a constant union case in expression position is now
  marked constant (D5), which the TypeScript runtime already read and which an authored constant
  case needed in order to evaluate as its bare name rather than a `$type` map.

**Non-Goals:**
- A per-case field type inside the NX checker. Nothing in this change observes it; `bind` will,
  and introduces it.
- Any change to `Value`, the canonical encoding, or the `.NET` SDK.
- Lazy synthesis of derived declarations (follow-up item 2 in `docs/add-update-actions-followsup.md`)
  applies to `T.Property` exactly as to `T.Update` and is deferred with it.

## Decisions

### D1. `T.Property` is a synthesized `Item::Union` marked with its target

Add `property_target: Option<Name>` to `UnionDef` (with a `PROPERTY_UNION_SUFFIX`,
`property_union_name`, `is_property_union_name` beside the update-record helpers in `lib.rs`).
Lowering synthesizes it in the same predeclaration passes that build `T.Update`, from the same
`lower_field_signatures` result, as `UnionDef { name: "T.Property", base: None, cases: one
fieldless case per effective field, property_target: Some(T) }`, registered under the Type
namespace and pushed onto the same pending list so it is appended after authored items.
Effective fields (inherited first) come from the same base-chain resolution the update shape
uses, so the two derived declarations can never disagree about `T`'s fields.

Why a union rather than a new item kind: a constant union is precisely the semantics wanted, and
every consumer (checker, interpreter, IR, runtime, typegen, language service) already handles one.
The marker exists for the three places that must know it is derived: the "cannot extend"
diagnostic, IR (`propertyTarget`, feature flag, referenced-only emission), and typegen (companion
naming). Why not a flag on the record: the union is a distinct type with its own name and cases.

### D2. Bare `Property` is resolved during lowering, like `Update`

`resolve_bare_update_tag` gets a sibling for the name `Property` applied at two sites: a type
reference whose single segment is `Property`, and a member access whose base identifier is
`Property`. Inside `current_component` both rewrite to `<Component>.Property`; outside they are
left as written and the checker reports the two diagnostics (`bare-property-outside-component`,
`unknown-property-union`) from a sibling of `report_unresolved_update_tag`. The checker's
`X.Property` failure path also recognizes an `X` that itself ends in `.Update` or `.Property` and
says so, which is how the nested-name scenarios are rejected without synthesizing anything.

### D3. No new runtime value; `changed` yields `Value::UnionCase`

A property reference at runtime is `Value::UnionCase { union: "T.Property", case: field }`. The
canonical encoding is already the bare string; host input is already lifted against the expected
type. `Value::UnionCase` carries no declaring origin, which is the same limitation authored
constant cases live with today and is not made worse here.

### D4. Intrinsics are a closed table checked and evaluated by name

- **Checker.** `infer_expr`'s `Call` arm consults an `INTRINSICS` table before evaluating the
  callee when the callee is a bare identifier. Each entry has an arity and a rule closure over
  the argument types: `apply` requires `(Named T, Named T.Update)` and yields `Named T`; `merge`
  requires `(Named T.Update, Named T.Update)` and yields it; `diff` requires `(Named T, Named T)`
  with `T` concrete and yields `Named T.Update`; `changed` requires `(Named T.Update)` and yields
  `Array(Union T.Property)`. "Same `T`" is checked by declaring origin, so `Named.Update` is not
  `User.Update`. `register_function_signatures` rejects a top-level `let` named after an entry,
  and the intrinsic branch runs before `env.lookup`, which is what makes a same-named parameter
  or local invisible at a call. Because the derived names are one dotted `Item`, "the `T.Update`
  for this `T`" is a name computation plus origin lookup, no generics needed.
- **Interpreter.** `eval_call` checks the same table by flattened callee name before
  `resolve_item`. `apply` walks the target's effective shape, takes each present update field
  through `coerce_update_field_value`, and builds a full `Value::Record`; `merge` is a map union
  with right bias; `diff` compares with the equality `==` already uses and emits only differing
  fields; `changed` walks the effective shape in order and emits a case per present field. The equality
  `diff` uses is the language's `==`, which this change makes structural over records and lists
  (the `value-equality` capability); it was previously always `false` for two records or two
  lists, so the same rule now serves `==`, match patterns, and `diff`.
- **Alternatives rejected.** Declaring signatures as `Type::Function` cannot express "same `T`".
  Element-shaped forms (`<Apply to={u} update={p} />`) read as construction, not computation.
  Making the names ordinary and overridable would let a library silently change the semantics of
  every patch operation in a program.

### D5. IR carries a declaration marker and one new expression op

`NxIrDeclarationKind::Union` gains `property_target: Option<NxIrReference>` (omitted when
absent), emitted only for property unions the program references, discovered the way
`referenced_update_records` discovers update records (union-case references and type annotations).
A referenced property union adds `property-unions-v1` to `required_features`. Intrinsic calls are
a new op `intrinsicCall { intrinsic, args }` and add `update-intrinsics-v1`. Two flags rather than
one so a program that only applies patches does not claim a declaration kind it never emits. A
`changed` call carries the target's declared field order (`fieldOrder`), computed by the IR
builder from the argument's static type, so neither runtime has to find the update record's
declaration in the program to order the result. The union-case expression op gains `isConstant`,
which the declaration already carried and the TypeScript runtime already read: without it an
authored constant case evaluated through IR produced a `$type` map instead of its bare name.

The TypeScript runtime adds both features and the op tag to its known sets, evaluates
`intrinsicCall` through four internal functions that `normalizePatchFields` and the record
normalizer already make short, and exports them as `applyUpdate`, `mergeUpdates`, `diffRecords`,
and `changedFields` with a shared record type parameter; `changedFields` takes the prepared
program and fails when it does not declare the update record, rather than silently falling back
to the value's own key order. Generated program modules call the same
four through the supplied runtime helper module, whose names join the reserved-runtime-name list.

### D6. typegen models the companion as a constant union with a companion marker

The model exports `<Name>_property` as an `ExportedType::Union` whose `companion_target` is
`Some(Name)`, so `generated_companion()` reports it for collision handling and the two emitters
need no new branch: the constant-union path already produces `export type X = "a" | "b";` and a
C# enum with the wire-format class. A sibling of `rewrite_update_type_references` maps `X.Property`
references to `X_property`, and the imported-library path skips derived unions in a dependency's
interface and resolves `X.Property` to that library's companion, mirroring how `X.Update` is
handled. The C# side deliberately stays an enum: the typed `NxProperty<TRecord, TValue>` keys only
pay for themselves with the map-backed update storage, which is a follow-up.

### D7. The design is forward-compatible with generics

Generics are planned. They do not replace the derived union: a property key type is a function of
one record's declaration, which no parametric generic produces (TypeScript needs the separate
`keyof` operator; C#, F#, Swift, and Kotlin have generics and no spelling for "the keys of `T`" at
all). `User.Property` is the same type constructor `Property<User>` would be, spelled the way
`User.Update` already is. Two points keep this change compatible with what generics will add:

- **Derived suffixes must be allowed on type parameters.** When generics land, `T.Update` and
  `T.Property` must resolve for a type parameter `T` constrained to a record. Nothing here may
  assume the target of a derived name is a declared item; the checker's "same `T`" comparison
  (D4) is by declaring origin, which a type parameter can carry as well.
- **The intrinsic table is a stand-in for four prelude signatures.** With the suffixes on type
  parameters, `apply<T>(record:T, update:T.Update): T`, `merge<T>(a:T.Update, b:T.Update):
  T.Update`, `diff<T>(before:T, after:T): T.Update`, and `changed<T>(update:T.Update):
  T.Property[]` become ordinary declared signatures, and the rule closures in D4 are deleted. The
  evaluation side is unchanged. Keep the table's typing rules exactly what those signatures would
  say, and nothing else, so the migration is a deletion.

What generics add beyond this change is the value-typed key, `Property<T, V>`, which `bind`, a
typed `get`, and typed table columns want. `T.Property` is then the partial key (Swift's
`PartialKeyPath<Root>`): matchable, serializable, host-facing. The typed key is a refinement in
which the literal `User.Property.name` has type `Property<User, string>` and upcasts to
`User.Property`. Every wire, IR, typegen, and host surface defined here stays a bare string under
that refinement.

**Companions are contravariant in the record, deferred with bounded generics.** If `X extends Y`,
every field of `Y` is a field of `X`, so a `Y.Property` is usable wherever an `X.Property` is
expected, and a `Y.Update` (a subset of `X.Update`'s fields, all optional) is a valid `X.Update`.
The reverse is unsound: `X.Property.email` names nothing on a `Y`. So the derived companions
relate in the opposite direction from the records they derive from. This change keeps them
nominal and distinct (the `property-references` scenario on distinct records, and the `apply`
scenario rejecting a base's update on a derived record) because without generics the relation
is almost never observable: a bare name at a typed site already resolves against the expected
type, and a function that takes `X.Property` names a concrete `X`. It becomes necessary the day
a helper is written as `sortBy<T: Named>(items:T[], key:T.Property)` and is called with
`Named.Property.name`. When it lands, it lands for `Property` and `Update` together, as one rule:
`Y.Property` satisfies `X.Property` and `Y.Update` satisfies `X.Update` whenever `Y` is in `X`'s
base chain. The prerequisite is a representation decision: a property union case is identified by
the record that declared the field plus the field name, so `Named.Property.name` and
`User.Property.name` are the same value and the subtyping is a pure checker rule with no runtime
coercion. Nothing in this change constrains that choice, since `Value::UnionCase` already carries
only names and the canonical encoding is the bare field name either way.

## Risks / Trade-offs

- [A field named `Property` or `Update`] → Allowed, as today. Its case is spelled
  `T.Property.Property`, and inside a component body a state field named `Property` is shadowed by
  the derived union just as one named `Update` is by the update record. Both are documented, not
  prevented.
- [One more synthesized item per declaration] → Appended last, so identities are stable; IR and
  typegen emit only referenced or exported ones. If the item count shows up in the language
  service, follow-up item 2 (lazy synthesis) covers both derived kinds at once.
- [`diff` equality on records and lists] → Uses the one equality the language already has for
  `==`, so `diff` agrees with what an author can test by hand. Floats compare as `==` does.
- [Intrinsic names become reserved] → Any existing program declaring `apply`, `merge`, `diff`, or
  `changed` breaks with a named diagnostic. There is no backward-compatibility obligation yet.
- [Case order across the hierarchy] → Inherited first, then own fields, the same order the
  effective shape and the update companion already use; `changed` and typegen both read it from
  one place.

## Migration Plan

Additive. Programs without property references, intrinsic calls, or exported records are
unchanged at every layer; the only observable break is the four reserved names.
