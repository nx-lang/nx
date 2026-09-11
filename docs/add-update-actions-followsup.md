# Follow-ups from `add-update-actions`

Work identified while reviewing and fixing the `add-update-actions` change that was deliberately
left for later. Item 1 patches a known gap; item 2 is the design proposal for where update records
and host representations should go next; the rest are smaller options and loose ends.

## 1. Resolve inherited companion field types in the module that wrote them

**Gap.** A `<T>_update` companion copies its target's effective fields, inherited ones included,
because a patch of `User` is not a patch of its base and so the companion extends nothing in either
target language. A copied field's type was written in the base's module. When that base lives in
another library and the importing module does not itself import the field's type, typegen writes
the bare name on the companion and neither emitter can resolve it:

```nx
// named/Named.nx
export type Tag = a | b
export abstract type Named = { name:string tag:Tag }

// types.nx
import { Named } from "./named"
export type User extends Named = { email:string }
// User_update gets `tag?: Tag`, and `Tag` is not in scope here.
```

Plain records never hit this: `User extends NamedBase` in TypeScript and `User : Named` in C#
inherit the field, and the base's own generated file resolves `Tag`.

**Today.** Generation warns, naming the companion, the field, the type, and the import that would
fix it (`warn_about_unresolvable_inherited_fields` in `crates/nx-cli/src/typegen/model.rs`). The
generated file still carries the bare name.

**To do.** Resolve the copied field's type in the namespace of the module that declared the field,
and emit a reference to that library's generated declaration: a package import with a local alias
in TypeScript, `global::<Dependency.Namespace>.Tag` in C#. Both emitters already do this for types
the module imports explicitly, so the work is in the model:

- Carry the declaring module on `ExportedRecordField`, which `nx_hir::EffectiveField.module_identity`
  already provides.
- Map that module identity back to the library root and name the emitters use for dependency
  packages and namespaces.
- Resolve the type name through that library's namespace, since it may be a library-local alias,
  and emit the exported name; a type the library does not export cannot be generated and should
  keep the warning.
- Add a cli-code-generation scenario: a companion field typed by a dependency's exported type
  references that type's generated declaration without the importing module naming it.

## 2. Proposal: first-class properties, and update records as property maps

This is the design direction the review discussion settled on. It keeps the `T.Update` type, the
element-shaped construction syntax, and the wire format exactly as they are today, and changes what
sits underneath them in the language, the runtime, and the generated host code.

### 2.1 What an update already is

An update record is the same value seen from two sides:

- **Runtime and wire:** a sparse map. `Value::Record` holds only the present keys, construction
  never inserts an absent field, and the encoding is `{ $type: "User.Update", email: null }`. That
  is JSON Merge Patch: absent means unchanged, a present `null` means set to null.
- **Static type:** a record with every field optional, which is TypeScript's `Partial<T>`, itself
  defined as a mapped type over `keyof T`. The checker already treats `T.Update` as "a map from
  `T`'s property names to that property's value type."

What the record-shaped view gives is per-field typing at construction. What the map view gives is
genericity: iterate the changed keys, apply, merge, diff. NX has the first and none of the second.

### 2.2 Language: a first-class property reference

Add a typed property reference, in the sense of Swift's `KeyPath<Root, Value>` and Kotlin's
`KProperty1<T, V>`: a value that names one property of a record type and carries that property's
value type. Spelling is open; `User.name` in type position, or a dedicated form, both work. Then
define `T.Update` as a map keyed by `T`'s properties, where each key's value type is that
property's type. The existing `<Update count={count + 1} />` syntax stays as the front door and
type checks exactly as it does now; it is sugar over the map.

Built-ins fall out generically instead of one at a time:

- `apply(record, update): T` and `merge(a, b): T.Update` (both already listed as follow-ups in the
  proposal).
- `diff(before, after): T.Update`, which is what a form or an optimistic UI needs.
- `changed(update): Property<T>[]`, and computed keys in construction, `<Update {prop}={value} />`.
- Two-way binding, the case that matters most for NX's domain: `<TextInput bind={user.name} />`
  can expand to `value={user.name}` plus `onTextChanged=<Update name={action.text} />` only if
  `user.name` can be reified as a property rather than read as a value.
- Table columns, sort keys, grouping, and validation rules expressed as property lists.

Two constraints to hold:

- **Keep per-key typing.** The key carries the value type, so the map is typed per key. A
  `Map<string, object>` update is what every dynamic host regrets once patches cross a service
  boundary.
- **Update stays a merge patch.** Nested paths and list operations turn it into JSON Patch, which
  is a different and much larger design. A property reference makes that extension possible later
  without forcing it now.

### 2.3 Runtime

No change to `Value::Record` or the encoding. The property reference is a new `Value` variant
carrying the record type, the field name, and the field's declared type; `apply`, `merge`, and
`diff` operate on the map that update records already are. The absent-versus-null rule is enforced
where it is today, at construction and at patch application.

### 2.4 Generated C#: a map with typed accessors

Today the generated `<T>_update` class stores one `NxOptional<T>` per field and needs the
`NxOptional<T>` JSON converter, the `NxOptional<T>` MessagePack formatter, a per-property
`[JsonIgnore(Condition = WhenWritingDefault)]`, and the reflection-based
`NxUpdateRecordMessagePackFormatter`. Replace the storage with a map and keep the properties as
typed accessors over it:

```csharp
public abstract class NxUpdateRecord
{
    private readonly Dictionary<string, object?> _fields = new(StringComparer.Ordinal);

    public abstract string NxType { get; }
    protected abstract IReadOnlyDictionary<string, Type> FieldTypes { get; }

    public IReadOnlyDictionary<string, object?> Fields => _fields;
    public bool IsSet(string name) => _fields.ContainsKey(name);
    public void Unset(string name) => _fields.Remove(name);

    protected NxOptional<T> Get<T>(string name) =>
        _fields.TryGetValue(name, out object? value) ? new((T)value!) : default;

    protected void Set<T>(string name, NxOptional<T> value)
    {
        if (value.HasValue) { _fields[name] = value.Value; } else { _fields.Remove(name); }
    }
}

public sealed class User_update : NxUpdateRecord
{
    private static readonly IReadOnlyDictionary<string, Type> Schema = new Dictionary<string, Type>
    {
        ["name"] = typeof(string),
        ["email"] = typeof(string),
    };

    public override string NxType => "User.Update";
    protected override IReadOnlyDictionary<string, Type> FieldTypes => Schema;

    public NxOptional<string> Name { get => Get<string>("name"); set => Set("name", value); }
    public NxOptional<string?> Email { get => Get<string?>("email"); set => Set("email", value); }
}
```

What this removes: both `NxOptional<T>` converters (the struct becomes a pure in-memory value that
never reaches a serializer, so the misuse case that RF8 hardened cannot happen), the per-property
wire attributes, and every use of reflection. One `NxUpdateRecord` MessagePack formatter and one
JSON converter walk `Fields` and decode through the generated schema, which also makes the class
NativeAOT-friendly. What it adds: the schema table per class, which lets the base reject unknown
keys on read as NX does, and boxing of value-typed fields, which does not matter at patch sizes.
What it gives hosts: iterate changed fields, `IsSet`, `Unset`, merge, build an update from a form's
dirty fields, apply by name. This is the OData `Delta<T>` model with typed accessors. The public
property surface is unchanged, so `new User_update { Email = null }` still works.

Then make the schema entry a property reference, matching 2.2:

```csharp
public static class UserFields
{
    public static readonly NxProperty<User, string> Name = new("name");
    public static readonly NxProperty<User, string?> Email = new("email");
}

update[UserFields.Name] = "Ada";
NxOptional<string?> email = update[UserFields.Email];
```

With a generated getter and setter delegate on `NxProperty<TRecord, TValue>`, `Apply(user, update)`
and `Diff(before, after)` become base-class methods, and `User_update` is little more than
`NxUpdate<User>` plus named accessors. Composing `FieldTypes` from a dependency's companion schema
(`NxSchema.Merge(Named_update.Schema, OwnSchema)`) also makes an imported base's fields reachable
through their keys even where item 1 has not yet generated an accessor for them.

### 2.5 Generated TypeScript

The runtime value is already a plain object holding only present keys, and `User_update` with
optional properties is already the typed map at the type level, so TypeScript needs no storage
change. For symmetry with C# and with the NX property feature, generate the property keys
(`UserFields.name: NxProperty<User, string>`) and ship `apply`, `merge`, and `diff` helpers typed by
them in the runtime package.

### 2.6 What does not change, and sequencing

The `T.Update` type, the absent-versus-null rule, the `$type` discriminator, the JSON and
MessagePack encodings, the `NxOptional<T>` accessor type on generated C#, and the dotnet-binding
spec all stay as they are. Suggested order:

1. Land `NxProperty<TRecord, TValue>` and the map-backed `NxUpdateRecord` in the .NET SDK, rebase
   the C# emitter and the checked-in fixture on them, and delete the converters they retire.
2. Add the property keys and helpers to the TypeScript runtime and emitter.
3. Design the NX-side property reference and the built-ins, with the host shapes from steps 1 and
   2 as the target for typegen.

## 3. Compose TypeScript companions instead of flattening

`export interface User_update extends Omit<Named_update, "$type">` lets the TypeScript compiler
pull inherited fields from the base's generated companion, which removes typegen's need to know
them and sidesteps item 1 for TypeScript. It generates less and reads well. Not adopted yet because
C# has no equivalent without class inheritance, and the two emitters should agree on where
inherited fields come from.

## 4. Synthesize `T.Update` on demand

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

## 5. Test gaps left open

- A checker test for `T.Update` including fields inherited across modules needs a multi-module
  session fixture; typegen now covers the cross-library case, the checker does not.
- The nx-codegen test asserting the canonical `Counter.Update` value skips when `node` is absent,
  like every other generated-JavaScript test. That pattern predates this change.

## 6. Host record construction runs outside the call's resource limits

`construct_host_record_value` in `crates/nx-interpreter/src/interpreter.rs` rebuilds every
host-supplied record from its fields, and evaluates any default a nested plain record needs in a
fresh `ExecutionContext::new()`. That is what `construct_external_record_value` already did for the
action record at dispatch, so nothing got worse, but it means the `ResourceLimits` a host passes to
initialization, evaluation, or dispatch do not bound default evaluation for records inside its own
input. Thread the call's context (or at least its limits) into the host construction path once
those defaults can do real work; today they are literal or near-literal.

## 7. Union case payloads from the host reach the case builder by name only

The same path builds a host-supplied payload union case (`{ $type: "Shape.Circle", r: 1 }`) through
`resolve_union_case_definition`, which resolves the union by name from the field's owner module. A
case the owner module cannot name by that spelling (a library-local alias, or an import the host
knows but the module does not) is kept as supplied rather than checked. Plain records go through
`resolve_record_definition` with the same limit. Resolving by declaring origin, as
`eval_resolved_union_case` does for authored cases, would close it; no scenario needs it yet.
