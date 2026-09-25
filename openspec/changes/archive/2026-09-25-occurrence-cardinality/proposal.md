## Why

NX has two ways to say "there might not be a value here", and they disagree. `T?` means "one value,
which may be `null`", and the `flat-sequences` work needed it to also mean "zero or one items" —
that is what an `if` with no `else` produces. The two readings collide at `T?[]`: `{ "a" null }` at a
`string?[]` has a `null` that is an item, not an absence, so nothing keyed on the type can tell them
apart. The `flat-sequences` review found that collision behind four of its findings, and left three
behaviours in place for this change to settle (`specs/future.md`, "Removing `null`"). Meanwhile `T[]`
is the only sequence spelling and it means "zero or more", which is the wrong default for a props
list: almost every sequence property in the corpus means "one or more when supplied".

The direction recorded in `future.md` is XPath's: there is no `null`, `{}` is the absence of a value,
and `?` is an occurrence indicator. This change adopts it and finishes the thought. Cardinality
becomes one suffix with four values — `T`, `T?`, `T+`, `T*` — and every join site the checker already
has gets a mechanical rule instead of a special case. NX is unusually well placed: there is no
null-handling surface to replace (no `?.`, `??`, `== null` or `is null` exists), absence already lives
in field presence for update records, and the empty sequence is already first class.

## What Changes

- **BREAKING** **`null` is removed from the language.** The literal, the value, the type, and the
  `null` match pattern are gone. `{}` is the one absent value; it is the empty sequence.
- **BREAKING** **`?`, `+` and `*` are the occurrence suffixes; `[]` is removed.** A type reference is
  a base type followed by at most one suffix: `T` exactly one, `T?` zero or one, `T+` one or more,
  `T*` zero or more. `T[]` is rejected with a fix-it naming `T*` or `T+`. `T?[]`, `T[]?`, `T??` and
  every other composition are rejected: there is one suffix rule, not two composing ones.
- **BREAKING** **A property admits zero only through `?` on its name.** A property, field, state
  field, emitted-action field, or parameter is `name:T`, `name?:T`, `name:T+` or `name?:T+`. `?` and
  `*` are rejected in a property's type slot — including through an alias — with a fix-it to the
  `name?:` form. An optional property has no default: `name?:T = x` is rejected, because a default
  fills the absence and the read type would be `T`. Reading `p` declared `p?:T` gives `T?`; reading
  `p?:T+` gives `T*`. Omitting an optional property at construction binds `{}`; a property that is
  not written and has a default takes the default; writing `{}` to a required or defaulted property
  is a type error, the same as writing `int?` to an `int`.
- **BREAKING** **Type arguments are exactly-one types.** `<Box T=int?/>` is rejected on the same
  terms as `<Box T=Ints/>` is today; optionality moves to the slot (`value?:T`).
- **Cardinality is arithmetic.** The checker computes an occurrence for every sequence-shaped
  expression from a four-point lattice (`1 ⊂ ?`, `1 ⊂ +`, `? ⊂ *`, `+ ⊂ *`): an `if` with no `else`
  over `T` is `T?`; a braced value adds its items' occurrences (`{ a? b }` is `T+`, `{ a? b? }` is
  `T*`); a `for` multiplies (`T+` over a body `U?` is `U*`); the join takes the least upper bound;
  `??` subtracts zero from its left operand. `{}` has type `never?`, which satisfies every `?` and `*`.
- **BREAKING** **The ternary `c ? a : b` is removed.** It was never specified, duplicates `if/else`,
  and its `?` collides with the presence test below.
- **Presence test: postfix `?`.** `x?` is `true` when `x` holds an item, valid on a `?` or `*`
  receiver and a diagnostic on a receiver that cannot be empty. In the taken branch of `if x? { … }`,
  and in the `else` of `if !x? { … }`, the tested path is narrowed: `T?` to `T`, `T*` to `T+`;
  `&&` chains narrow each operand. `{}` is a pattern, so `if x is { {} => … else => … }` matches
  absence, and the `else` arm narrows.
- **Step: `?.`.** `x?.m` is `{}` when `x` is empty and `x.m` otherwise; its type joins the receiver's
  `?` with the member's occurrence (`name:U` gives `U?`, `names:U+` gives `U*`). Plain `.` on a `?`
  receiver is rejected naming `?.`, and `?.` on an exactly-one receiver is a diagnostic. `?.` is
  defined for `?` receivers only; mapping over `+`/`*` is out of scope. A tested chain narrows every
  step: inside `if a?.b? { … }`, `a.b` is legal.
- **Fallback: `??`.** `x ?? y` is `y` when `x` is empty and `x` otherwise. Its type is the join of
  `x` with zero removed and `y`: `T? ?? T` is `T`, `T* ?? T+` is `T+`. It binds tighter than
  arithmetic, so `"Author: " + book.author?.name ?? "Anonymous"` reads as written; the C#/Swift
  low-binding parse would be a type error in NX every time, since `+` takes exactly-one operands. A
  left operand that cannot be empty is a diagnostic.
- **Update records keep the absent-versus-cleared distinction by key presence.** An absent field is
  unchanged; a present `{}` clears, and is accepted only where the target field is optional. Every
  `null` in the intrinsics, the wire form and the managed SDK becomes the empty value.
- **Host boundary.** Inbound `null`, a missing key, and `[]` all decode to `{}` at an optional or `*`
  site; `[]` at a `+` site is rejected. Outbound, an absent optional field is omitted from canonical
  JSON as an update record's already is; `+` and `*` encode as arrays. Typegen maps `T?` to a nullable
  type, `T+` and `T*` to arrays, and validates non-emptiness at construction. The NX IR gets one
  sequence type kind carrying an occurrence and three node kinds for the new operators; `null` and
  `nullable` stay assigned and are never emitted.
- **A call leaves out a parameter the function fills.** A positional call may stop before the
  trailing optional or defaulted parameters, and an element call may leave any of them out; the
  function fills each with its default, which it evaluates itself and which may read the parameters
  before it, or with `{}`. So a paren-style function lists those parameters last, and an
  element-style function, whose attributes have no order a caller relies on, is called only as an
  element. Parameter defaults, which were parsed and dropped, take effect.
- **Diagnostics stop saying "nullable".** Types render as `Person?`, `Person+`, `Person*`; the type
  of `{}` renders as `{}`.
- Every spec, reference page, example and playground snippet that spells `T[]`, writes `null`, or
  uses a ternary is rewritten. `specs/future.md`'s "Removing `null`" section is replaced by a pointer
  to this change and the items it did not settle.

Out of scope: `?.` and `for`-free mapping over `+`/`*`; effective boolean value (`if` still takes a
boolean); a binding form (`if let`); making `never` writable (`future.md`, "`never` Renders As A
Name…"); the `Element` and `object` naming questions, except where this change must decide how
`object` and the empty sequence meet.

## Capabilities

### New Capabilities
- `occurrence-types`: The occurrence model that replaces `type-reference-suffixes`. One suffix, four
  cardinalities, the lattice, how occurrences add in a braced value and multiply in a `for`, the
  subtyping rule, `{}` as the empty value and its type, and what a diagnostic renders.
- `optional-properties`: `name?:T` — which declaration sites take it, what may follow the colon, the
  no-default rule, the read type, construction with an omitted, written, or empty value, and what
  aliases resolving to `?` or `*` do in a property slot.
- `function-parameters`: How a `let` function is called — a paren-style one by position or as an
  element, an element-style one only as an element — the order a paren-style signature puts
  omissible parameters in, what a call may leave out, and the function evaluating its own defaults.
- `presence-operators`: Postfix `?`, `?.`, `??` and the `{}` pattern: syntax, precedence, typing,
  the diagnostics for receivers that cannot be empty, the narrowing they induce and its scope, and
  runtime semantics every engine shares.

### Modified Capabilities
- `type-reference-suffixes`: Removed entirely; superseded by `occurrence-types`.
- `sequence-model`: `T[]` becomes `T+`/`T*`; "a nullable type is an item type" is reversed; the
  splice rule is restated in occurrence terms; `object` and the empty sequence.
- `conditional-result-types`: An `if` with no `else` over `T` is `T?`, not `T[]`; "a nullable value
  requires an explicit `else`" is removed with the concept; the join rule is restated on the lattice.
- `braced-value-sequences`: Arity-to-type becomes occurrence addition; `{}` at an optional site;
  `{}` is `never?` rather than `never[]`.
- `update-records`: Absent-versus-null becomes absent-versus-empty; `{}` accepted only for optional
  targets; intrinsics, wire form.
- `implicit-primitive-conversions`: The join no longer lifts nullability out; the `null` literal
  clause is removed; `+` rejects `?`/`+`/`*` operands; widening applies through an occurrence.
- `discriminated-unions`: "Nullable union absence normalizes to null" becomes absence is `{}`; case
  construction validates optional fields.
- `typed-braced-expression-kinds`: The `T[]?` versus `T?[]` distinction is removed; scalar lift is
  restated as `T` satisfying `T?`, `T+`, `T*`.
- `record-type-parameters`, `component-type-parameters`: A type argument is an exactly-one type;
  `T?` is not an item type; the bottom-type spellings change; `next:T?` becomes `next?:T`.
- `function-types`: A suffix after a result binds to the result, restated for `?`/`+`/`*`;
  parameters follow `optional-properties`.
- `record-type-inheritance`: A `?`-typed base reads its fields through `?.`, not `.`.
- `record-construction-validation`, `component-syntax`, `content-properties`,
  `component-runtime-bindings`, `component-action-handlers`: Required-field and host-construction
  rules stated in optional/`{}` terms; an empty body is "not written"; list spellings.
- `value-equality`: `null` clause removed; an item compares as a sequence of one.
- `unbraced-literal-forms`: `null` leaves the unbraced literal set and the signed-literal rule;
  contextual names resolve through an occurrence.
- `contextual-numeric-literals`: "after nullability is stripped" becomes "after the occurrence is
  stripped".
- `property-references`: Case resolution through occurrence wrappers; `T.Property+`.
- `primitive-type-names`: The bottom type's spelling notes (`never[]?` → `never*`, `{}`).
- `nx-ir-format`, `typescript-ir-runtime`, `executable-code-generation`: The sequence type kind
  with an occurrence, the three operator node kinds, no `null` node or value, update-record and
  union-absence rules restated, one lone-child binding rule.
- `cli-code-generation`, `external-components`, `dotnet-binding`: Typegen and SDK
  mappings for `?`/`+`/`*`, optional properties, update companions with `{}` as clear.
- `editor-syntax-highlighting`: `+`/`*` as type modifiers, `?:` on a property name, the three
  operators, no ternary scopes, no `null` constant.
- `drawnui-nx-catalog`, `fiddle-nx-language`: The normative catalog spellings (`ItemsSource?: TItem+`,
  `ItemTemplate?: <function …/>: DrawnNode`).

## Impact

- **Grammar and validation** (`crates/nx-syntax`): `type` gains `+`/`*` and loses `[]`;
  `property_definition` gains an optional `?` after the name; `conditional_expression` is removed;
  `null_literal` is removed; postfix `?`, `?.`, `??` and the `{}` pattern are added.
  `validate_type_suffixes` enforces one suffix and the property-slot rule. TextMate grammar and
  parser tests follow.
- **HIR** (`crates/nx-hir`): `TypeRef::Array`/`Nullable` become one occurrence wrapper; `Literal::Null`
  and the ternary lowering go; three expression forms and one pattern form are added; property
  definitions carry `optional`.
- **Types** (`crates/nx-types`): `Type::Array` and `Type::Nullable` become `Type::Seq { item, occ }`
  (108 `Nullable` sites outside tests, plus `Array`); the join, the contribution rule, `for`, `if`,
  match and braced-value inference compute occurrences; the property-slot and type-argument rules;
  narrowing; typing of the three operators; rendering.
- **Interpreter and values** (`crates/nx-interpreter`, `crates/nx-value`): `Value::Null` (48 sites)
  and `NxValue::Null` are removed; `{}` is the empty array; coercion, update intrinsics, record
  construction and the new operators.
- **IR, runtimes, emitters** (`crates/nx-codegen`, `runtime/typescript`): schema 4 → 5 with a
  `seq` type kind and `exists`/`optionalMember`/`coalesce` node kinds; `null`/`nullable` retired;
  `normalizeValue`, patch normalization, union absence; emitted JS for the operators; the
  three-engine parity test gains the operators and a lone-child case.
- **Typegen and SDKs** (`crates/nx-cli/src/typegen`, `bindings/dotnet`, `bindings/node`; the Node SDK's value
  contract is the canonical encoding `occurrence-types` defines, so `sdk-node` needs no delta): C# and TypeScript
  mappings; update companions; the managed update-record API's "set to null" vocabulary.
- **Formatter and editor** (`crates/nx-cli/src/format.rs`, `crates/nx-language-service`,
  `src/vscode`): print the new forms, never emit `*/>`, hover renders occurrences.
- **Corpus**: 45 `.nx` files spell `[]`, 11 write `null`, 3 use the ternary; 6 reference pages spell
  `T[]`; `nx-grammar.md`, `nx-grammar-spec.md`; `specs/ir-conformance` cases; the playground and
  DrawnUI catalog generators.
- **Specs**: three new, the list above modified, `type-reference-suffixes` removed. A whole-corpus
  before/after diff of diagnostics and canonical output is the acceptance evidence, as it was for
  `flat-sequences`.
- **Downstream**: any NX source spelling `T[]`, `null` or a ternary stops parsing; the fiddle's and
  the playground's catalogs regenerate; hosts that sent `null` for an optional field keep working,
  hosts that read `null` back for an absent field read a missing key instead.
