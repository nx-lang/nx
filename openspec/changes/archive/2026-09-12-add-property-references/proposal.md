## Why

`add-update-actions` gave every record-shaped declaration `T` a derived patch record `T.Update`,
and left a gap its follow-up notes name directly: an update is a record from the front and a sparse
map underneath, and NX can only see the record. Nothing in the language can name a field of `T` as
a value, so nothing can iterate the keys an update carries, apply a patch to a record, compose two
patches, or compute the patch between two records. Every host that wants a sort key, a column list,
or a validation rule expressed over fields has to take a `string` and check it by hand, and every
generic operation over patches has to be written per record.

NX has no generics, so the usual answer (`KeyPath<Root, Value>`, `keyof T`) is not available. But it
already has the concept whose observable behavior matches a property key in every respect: a
constant union. A closed nominal set of names, resolved bare at a typed site, matched exhaustively,
and serialized as its bare name on the wire. A derived constant union per record, built the way
`T.Update` is built, gives property references to the language with no new expression syntax and
no new runtime value.

## What Changes

- Add a derived **property union** `T.Property` for every record-shaped declaration `T`: plain
  records, actions, inline emitted actions, and a component's `state`. Its cases are `T`'s
  effective field names, including inherited fields; for a component they are its state fields and
  never its props. `T.Property` behaves as a constant union everywhere: `User.Property.name` in
  expressions, a bare `name` at a site typed `User.Property`, exhaustive `if prop is { ... }`
  matching, and the bare string `"name"` on the wire. Each case denotes one declared field.
- Inside a component body, a bare `Property` name resolves contextually to the enclosing
  component's own `<Component>.Property`, exactly as the bare `Update` tag does; outside a component
  body it is a diagnostic that names the qualified spelling. `Property` joins `Update` as a reserved
  emitted-action name, and `X.Property.Property`, `X.Update.Property`, and `X.Property.Update` do
  not exist.
- Add four **intrinsic functions** over update records, typed by rule rather than by generics:
  `apply(record, update): T` returns the record with each present field of the update replaced;
  `merge(first, second): T.Update` composes two updates with the later one winning per field;
  `diff(before, after): T.Update` produces the update that turns one record into the other; and
  `changed(update): T.Property[]` lists the present fields in declaration order. The absent-versus-
  null rule holds through every one of them: a present `null` is carried, an absent field is never
  invented. **BREAKING**: the four names are reserved; they resolve before lexical scope, cannot be
  shadowed, and a declaration named after one is rejected.
- Carry property unions and the intrinsics through NX IR, generated JavaScript, and the TypeScript
  IR runtime, which also exports the four operations as typed helpers for host code.
- Extend `typegen`: every exported record, action, and stateful component gets a `<Name>_property`
  companion generated as a constant union: a TypeScript string literal union, which is the `keyof`
  form under which `User[User_property]` already carries the field type, and a C# `enum` with the
  bare-string wire format the SDK's existing enum helpers provide.
- Update the language reference, the language tour, `nx-grammar.md`, and the component example.
- **BREAKING**: `==` and `!=` compare records and lists structurally, the equality `diff` is
  specified with; two records or two lists previously always compared unequal.
- **BREAKING** for hosts running authored programs through the TypeScript IR runtime: NX IR marks
  a constant union case as constant in expression position, so an authored constant case such as
  `Mode.dark` now evaluates to the bare string `"dark"` there, as it always did in the interpreter,
  instead of a `{ $type: "Mode.dark" }` map.

Out of scope, named here so the follow-ups are known: two-way binding (`<TextInput bind=query />`
and the component-side `binds` clause that pairs a prop with an emitted field), which is the first
consumer of the per-case field type inside the checker and gets its own change; the map-backed
`NxUpdateRecord` storage in the .NET SDK described in `docs/add-update-actions-followsup.md`
section 1.4, together with the typed `NxProperty<TRecord, TValue>` keys it indexes by; a generic
`get(record, property)` read, whose result would be `object` for an open key; computed keys in
construction (`<Update {prop}={value} />`); and nested paths or list operations, which would turn
the merge-patch model into JSON Patch.

## Capabilities

### New Capabilities
- `property-references`: the derived `T.Property` constant union: its cases, how it inherits
  bare-name resolution, matching, and wire form from constant unions, the contextual bare
  `Property` inside a component, and the reserved names.
- `value-equality`: the language's `==` and `!=`, structural over records and lists.

### Modified Capabilities
- `update-records`: adds the `apply`, `merge`, `diff`, and `changed` intrinsics over update
  records and the intrinsic-name resolution rule.
- `component-syntax`: `Property` is reserved as an emitted-action name alongside `Update`.
- `nx-ir-format`: IR declares derived property unions with their target, encodes intrinsic calls
  as a distinct expression kind carrying the field order `changed` sorts by, and marks a constant
  union case as constant in expression position.
- `typescript-ir-runtime`: property union values normalize as constant cases; the runtime evaluates
  the intrinsics and exports them as helpers.
- `executable-code-generation`: generated JavaScript evaluates property references and the
  intrinsics with the absent-versus-null rule.
- `cli-code-generation`: generated TypeScript and C# surfaces include a `<Name>_property`
  companion per exported record-shaped declaration.

## Impact

- `crates/nx-hir`: synthesize `T.Property` unions beside `T.Update`; contextual `Property`
  resolution; reserve the name.
- `crates/nx-types`: the derived union in type and expression positions; intrinsic call checking
  by rule; the nonexistent nested derived names.
- `crates/nx-interpreter`: intrinsic evaluation; `apply` reuses the dispatch patch application
  path; `changed` yields union case values.
- `crates/nx-codegen` and `runtime/typescript`: IR declarations for referenced property unions;
  intrinsic call encoding and evaluation; exported helpers.
- `crates/nx-cli/src/typegen`: `<Name>_property` companions for TypeScript and C#.
- `docs/`, `nx-grammar.md`, `examples/nx/component.nx`.
