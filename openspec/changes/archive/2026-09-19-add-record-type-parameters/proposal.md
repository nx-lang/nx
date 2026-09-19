## Why

NX type parameters exist on components only, and they are erased once checking is done: every
`<List TItem=.../>` has the one type `List`. That is enough for a collection-drawing component, but
it cannot describe a *value* whose type depends on a parameter. The motivating case is a range: a
`Range` of `int` can drive a `for`, a `Range` of `float64` can bound a slider, and the two must be
different types for either rule to be checkable. The same gap blocks `Pair`, `Page`, `Result` and
every other container record. This change adds the missing piece — generic records and a way to name
one instantiation — so that a following change can declare `Range` in the prelude and add the
`..`/`..=` operators as a small addition rather than a special-cased built-in.

## What Changes

- A record declaration accepts leading `Name:type` properties as type parameters, with the same
  syntax and the same placement rules as a component signature:
  `type Range = { T:type start:T end:T endInclusive:boolean }`. A parameter is a type in the record's
  own field types and nowhere else.
- A new **applied type** names one instantiation of a generic record in any type position, spelled
  as the element that constructs it with only its type arguments: `<Range T=int/>`. It composes with
  the `?` and `[]` suffixes, nests, and may be the target of an alias
  (`type IntRange = <Range T=int/>`).
- Applied types are **distinct per argument and invariant**: `<Range T=int/>` and
  `<Range T=float64/>` are different types, and neither satisfies the other. This is the difference
  from component type parameters, which stay erased.
- Constructing a generic record binds its type arguments the way a component use site does —
  `<Range T=int start={1} end={5} endInclusive={false}/>` — and the element has the applied type.
  Every type parameter must be bound; there is no inference and, unlike components, no bottom-type
  fallback, because the value's type needs the argument.
- A generic record's name with no arguments is not a type: `range:Range` is rejected with a
  diagnostic that shows the `<Range T=.../>` form. An applied type over a record with no type
  parameters, an unknown argument name, a duplicate argument, and a missing argument are each
  rejected by name.
- The derived companions follow the record: `Range.Update` carries the same type parameters and is
  named as `<Range.Update T=int/>`; `Range.Property` is parameter-independent and unchanged. The
  generated companions follow it too — `Range_update<T>` in both languages — so a patch of a
  `Range<long>` is usable at that instantiation. Only a *component's* state companion erases, where
  the type being patched is concrete.
- Generic records take no part in inheritance in this change: a record with type parameters cannot
  be `abstract` and cannot have an `extends` clause. Type parameters stay rejected on `action`
  declarations, emitted actions, state groups, functions, function types, aliases and unions.
- Below the type checker nothing changes shape: the interpreter, NX IR and the TypeScript IR runtime
  see an applied type as its record and a parameter-typed field as `object`, exactly the erasure
  rule components already use. **The IR schema version does not change.**
- `typegen` emits a generic record as a real generic in both languages — `Range<T>` in C# and in
  TypeScript — and an applied type as the instantiation (`Range<long>`, `Range<number>`).
  Executable TypeScript codegen does the same. The `<Name>_update` companion and, in C#, the
  `<Name>Properties` key table carry the same parameters, so a host diffs and applies at the
  instantiation it holds. The wire is unchanged: a patch is still `{"$type":"Range.Update", …}`
  with no type argument.
- A generic record crosses a library boundary with its type parameters, so an importing module can
  apply and construct it.

- **Folded in from `fix-clippy-diagnostics`:** the workspace's clippy diagnostics are fixed and
  `cargo clippy --workspace --all-targets -- -D warnings` becomes a CI gate. Clippy has never been
  clean here and has never been in the gate, so diagnostics accumulated — about 140 of them,
  including deny-level errors and a benchmark that does not compile. The cleanup is
  behavior-preserving and is done here so that the generic-record code lands under the same gate
  as everything else rather than into a workspace that cannot be linted.

## Capabilities

### New Capabilities

- `record-type-parameters`: declaring type parameters on a record, naming an instantiation with an
  applied type, constructing a generic record, the identity and invariance of applied types, the
  derived companions of a generic record, and the positions where type parameters remain rejected.

### Modified Capabilities

- `cli-code-generation`: adds a requirement that generated C# and TypeScript carry a generic record
  as a generic type and an applied type as its instantiation. The existing component erasure
  requirement is unchanged.
- `nx-ir-format`: adds a requirement that NX IR carries neither record type parameters nor type
  arguments — an applied type is encoded as a nominal reference to its record and a
  parameter-typed field as `object` — and that the schema version is unchanged.

## Impact

- **Grammar** (`crates/nx-syntax`): a new `applied_type` alternative in the `type` rule beside
  `function_type`; validation admits `Name:type` in a plain record and keeps every other rejection.
  Regenerated `parser.c`, `grammar.json`, `node-types.json`; highlight queries.
- **HIR** (`crates/nx-hir`): `RecordDef.type_params`; a new `ast::TypeRef::Applied` variant and its
  spelling; update-record predeclaration copies the parameters; the record interface item carries
  them across modules; `erase_type_parameters` reaches inside an applied type.
- **Checker** (`crates/nx-types`): `NamedType` gains type arguments that take part in equality;
  resolution of an applied type; record construction binds and consumes type arguments on both the
  record-literal and the element path; the update intrinsics carry arguments from `R` to
  `R.Update`. `is_visible_type_name` gains the builtin type names, so `Element` is a type argument
  on a generic record — and, because the same predicate serves the component use site, on a
  component too, where it previously reported "is not a visible type".
- **Diagnostic placement** (`crates/nx-types`): a `TypeRef` carries no span, so every conversion
  whose caller knows one — a value definition, a function parameter or return type, a component
  prop or state field, a record or union-case field — hands it over. A declaration's type
  reference is reported once, by the pass that owns the declaration: the signature pre-pass and
  every use site that resolves the same reference again (a field read, an element's binding spec,
  an inherited prop, a call site's return type, an imported value's annotation) resolve quietly.
- **Below the checker** (`crates/nx-interpreter`, `crates/nx-codegen`, `runtime/typescript`):
  erasure only. No IR schema bump, no runtime change in the TypeScript runtime. One pre-existing
  bug is fixed along the way, because the generic-record tests are what first reached it: a
  top-level value whose construction omits a defaulted field binds its own module's top-level
  values so the default can name one, and the value in flight is one of them — a cycle that used
  to be a stack overflow with no diagnostic. `ExecutionContext` now tracks the values being bound
  and the nested pass leaves them out.
- **Library interface** (`crates/nx-api`): record interface entries carry type parameters, and a
  checked type converts back to an applied `TypeRef`.
- **Typegen** (`crates/nx-cli/src/typegen`) and executable codegen (`crates/nx-codegen/src/emit.rs`):
  generic record declarations and applied type references; a generic record's update companion and
  C# key table declare the record's parameters. Executable TypeScript is unaffected — it emits
  update records as runtime objects, not as typed companions.
- **.NET SDK** (`bindings/dotnet/src/NxLang.Sdk`): a new `NxUpdateRecordJsonConverterFactory`. A
  generic companion cannot name its own converter in an attribute (CS0416, an attribute argument
  cannot use type parameters), so it names the factory; for MessagePack, typegen emits a small open
  generic formatter shim beside the companion, which the attribute resolver closes over the same
  arguments. A non-generic companion is untouched.
- **Language service**: hover on a record's type parameter; completions inside an applied type are a
  follow-up.
- **Docs**: `reference/syntax/types.md`, `reference/syntax/functions.md` (cross-link),
  `nx-grammar.md`, `nx-grammar-spec.md` — the last two also gain the component type-parameter
  syntax they never recorded.
- **Clippy cleanup** (folded in): `crates/nx-ffi/src/lib.rs`'s `extern "C"` entry points become
  `unsafe extern "C"` — the C ABI and the .NET P/Invoke declarations are unaffected, only Rust-side
  callers need `unsafe` blocks; the parser benchmark, which has never run and whose input does not
  parse, is deleted along with its undeclared `criterion` dependency; `approx_constant` literals in `nx-cli`, `nx-hir` and `nx-value` are replaced;
  the remaining warn-level lints are fixed crate by crate, allowed only where the code is right and
  the lint is not, with a comment saying why. `.github/workflows/build.yml` gains one step in the
  `rust` job.
- No breaking change: every program that checks today checks the same way.
