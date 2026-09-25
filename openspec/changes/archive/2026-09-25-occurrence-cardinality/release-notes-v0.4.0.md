# v0.4.0 release notes (draft)

Draft for the GitHub Release the `v0.4.0` tag creates (`docs/deployment.md`, "Publish A Package
Release"). Paste it as the release body before publishing.

---

NX drops `null`. A value is either there or it is not, and how many of it there can be is one
suffix on the type: `T` exactly one, `T?` zero or one, `T+` one or more, `T*` zero or more. `{}`
is the one empty value. Three operators test, step through and fall back from an empty value, and
the checker computes cardinality everywhere a value is collected, joined or iterated.

NX IR moves to **schema 5**; a module that uses the new operators or the `{}` pattern lists the
`occurrence-v1` feature.

## Breaking changes

| Before | After | Notes |
| --- | --- | --- |
| `T[]` | `T+` or `T*` | `+` when the sequence cannot be empty, which is what almost every props list meant; `*` when it can. `T[]` is rejected with a fix-it naming both. |
| `T?[]`, `T[]?` | `T*` | One suffix per type. `T??`, `T+?`, `(T+)*` and every other composition are rejected. |
| `name:T?` | `name?:T` | A property, field, state field, action field or parameter admits zero only through `?` on its name. `?` and `*` in the type slot are rejected with the fix-it, also through an alias. |
| `items:T[]?`, `items:T*` | `items?:T+` | Same rule for sequences. Reading `items` gives `T*`. |
| `name:T? = x` | `name:T = x` or `name?:T` | An optional property has no default. |
| `null` | `{}` | The literal, the value, the type and the `null =>` pattern are gone. `{}` is written where a value is empty, or the optional property is omitted; `{} =>` matches absence. |
| `<Box T=int?/>` | `<Box T=int value?:T/>` | A type argument is an exactly-one type; optionality moves to the slot. |
| `c ? a : b` | `if c { a } else { b }` | The ternary is removed. Its `?` is now the presence test. |
| `<User.Update email={null} />` | `<User.Update email={} />` | A present `{}` clears an optional field; an absent field is unchanged. Clearing a field the target does not declare optional is rejected. |
| `changed(patch): T.Property[]` | `T.Property*` | |

Also:

- **Diagnostics no longer say "nullable".** Types render as `Person?`, `Person+`, `Person*`, and
  the type of `{}` renders as `{}`.
- **`object` is exactly one value.** A `+` or `*` value satisfies `object*`, not `object`, and
  `{}` does not satisfy `object`.
- **Typegen.** C# and TypeScript map `T?` to a nullable type, `p?:T` to a nullable property, and
  `T+`/`T*` to the list mappings. Update companions mark a field clearable only when the target
  field is optional: `email?: string | null` beside `name?: string`.
- **Host boundary.** Inbound `null`, `[]` and a missing key all decode to the empty value at a `?`
  or `*` site; `[]` and `null` at a `+` or exactly-one site are rejected, even where the field has a
  default, which only a missing key takes. Outbound, an empty optional field is an omitted key; a
  cleared update-record field is still written as `null`; and an entry call whose result type is a
  standalone `T?` returns an empty result as `null`, so `NxRuntime.Evaluate<string?>` and
  typegen's `T | null` read it. An empty `T*` result stays `[]`.
- **Entry-call arguments are validated.** A record passed as an argument of an entry call is now
  constructed against its declared type, as props and state already were: defaults are applied, and
  an unknown field, a missing required field or an empty value where one is required is reported at
  the call rather than when the function reads the field.
- **NX IR.** The `array`, `nullable` and `null` kinds are retired (their numbers stay assigned and
  a reader reports them as malformed). One `seq` type kind carries the item type and an occurrence;
  `exists`, `optionalMember` and `coalesce` are the operator nodes. A function declaration carries
  its declared result type and an optional-result flag, and a value its declared type, so the IR
  runtime and generated JavaScript lift a result to its declared occurrence as the interpreter
  does. A schema-4 image is refused naming both versions.
- **Language service and editor.** Hover prints the new spellings; there is no `null` completion;
  the TextMate grammar scopes `?`, `+` and `*` as type modifiers and highlights `?.` and `??`.

## Occurrences

A type reference is a base type followed by at most one suffix. The suffixes form a lattice —
exactly one fits every other; `?` and `+` each fit `*` and nothing narrower — and every join in the
checker takes the least upper bound: an `if` with no `else` over `T` is `T?`, `{a b}` is `T+`,
`{o p}` with two optionals is `T*`, and a `for` over `T+` with a body of `U?` is `U*`. Numeric
literals still convert at suffixed sites, so `let xs:float64+ = {1 2}` binds floats.

## Optional properties

`name?:T` declares a property that may be absent. Omitting it at construction binds `{}`; reading
it gives `T?` (`T*` for `name?:T+`); a record stores no entry for it, so its canonical JSON omits
the key. Optional state initializes empty. A required or defaulted property rejects `{}`.

## Presence operators

- `x?` is `true` when `x` holds at least one item. In the taken branch of `if b.author? { … }` the
  tested path is narrowed, so `b.author.name` is legal there; `!` and `&&` narrow, `||` does not.
- `x?.m` is `{}` when `x` is empty and `x.m` otherwise, and its type joins the receiver's `?` with
  the member's occurrence. Plain `.m` on a `?` receiver is an error that names `?.m`.
- `x ?? y` is `y` when `x` is empty. It binds tighter than arithmetic, so
  `"by " + b.author?.name ?? "anonymous"` reads as written — unlike C#, where `??` binds below `+`.
- `{}` is a match pattern: `if b.author is { {} => "anonymous" else => b.author.name }`.

A test, step or fallback on a value that cannot be empty is a warning, not an error.

## Migration

The table above is the whole of it. `nxlang check` reports every removed spelling with the form
to write, and each fix-it is local: the suffix, the `?` on the name, the `{}`, or the `if/else`.
