## Context

NX has two adjacent concepts today:

- `enum` declarations model closed scalar choices and preserve authored member strings in raw and
  typed host values.
- Abstract record inheritance models polymorphic payloads and already serializes concrete records
  with a canonical `$type` discriminator.

Discriminated unions should sit between those concepts. They should keep enums for simple symbolic
choices, but give authors a closed record-like type for states and events that have per-case
payloads. The implementation touches the parser, HIR, prepared binding model, type checker,
interpreter, raw value conversion, code generators, .NET binding, VS Code grammar, and docs.

## Goals / Non-Goals

**Goals:**

- Add a first-class discriminated union declaration syntax:

  ```nx
  type LoadState =
    | idle
    | loading
    | failed {
        message: string
        retryable: bool = true
      }
    | loaded {
        items: Item[]
      }
  ```

- Require the leading `|` for union declarations so `type Name = OtherName` remains a plain type
  alias and `type Result = Success | Failure` stays available for a possible later union-alias
  feature.
- Keep `enum` as the only syntax for simple scalar choices.
- Support scoped case constructors, case field validation, case defaults, fieldless case shorthand,
  and closed-set matching.
- Narrow a matched identifier inside `if value is { ... }` arms without adding a required `as`
  binding form.
- Reuse the existing `$type` map shape for runtime output and typed host serialization.
- Generate usable TypeScript and C# contracts for exported unions.

**Non-Goals:**

- Adding `type Result = Success | Failure` union aliases over existing named types.
- Adding arbitrary nested destructuring patterns or a required `as` binding syntax.
- Replacing enums or changing the enum bare-string wire contract.
- Allowing unions to be extended after declaration; discriminated unions are closed.

## Decisions

### 1. Use `type` with required leading-pipe cases, and keep `enum`

The parser should add a separate `union_definition` form:

```ebnf
UnionDefinition ::=
    [VisibilityModifier] "type" Identifier ["extends" QualifiedName] "=" UnionCaseList

UnionCaseList ::=
    UnionCase+

UnionCase ::=
    "|" Identifier ["{" {PropertyDefinition} "}"]
```

The required first `|` makes the declaration visually distinct from an alias and avoids committing
NX to a meaning for `type Result = Success | Failure` now. Enums stay on `enum` because they are
scalar values, serialize as bare strings, and map differently in generated code.

Alternative considered:
- Allow `type LoadState = idle | loading | failed { ... }`.

Why rejected:
- The first case would be special for formatting and editing, and the syntax conflicts with future
  aliases over existing named types.

### 2. Model unions as HIR items with scoped cases, not as synthetic top-level records

HIR should gain `UnionDef` and `UnionCaseDef`, and `Item` should gain `Item::Union`. A union binds
only the union name in the type namespace. Cases are scoped under that union and are referenced by
qualified names such as `LoadState.failed`; they are not independent top-level declarations.

Prepared interface metadata should publish exported unions with their cases, fields, defaults, and
optional abstract record base. Imported and peer-file analysis then resolves `LoadState.failed`
through the visible union binding rather than through a separate top-level case binding.

Alternative considered:
- Lower each union case into a hidden record named `LoadState.failed`.

Why rejected:
- That would make hidden records visible to subsystems that do not understand closed unions, and it
  would make exhaustiveness and case ownership harder to enforce consistently.

### 3. Treat union cases as record-like values with nominal case identity

All cases should have a nominal case identity (`LoadState.failed`) and an owner union identity
(`LoadState`). Payload cases use element-style construction:

```nx
let failed: LoadState = <LoadState.failed message={"Offline"} />
```

Fieldless cases also have a value shorthand:

```nx
let idle: LoadState = LoadState.idle
```

The interpreter can reuse `Value::Record { type_name, fields }` with `type_name` set to the full
case discriminator, such as `LoadState.failed`. This keeps canonical raw conversion aligned with
existing polymorphic records and avoids adding a second union envelope format.

### 4. Add union-aware type compatibility and field access

The type layer should distinguish union types and union case types enough to answer:

- whether a case belongs to a union
- whether a value of case type satisfies the owning union type
- whether a union extends an abstract record and therefore exposes inherited shared fields
- which case fields become available after narrowing

Implementation can add explicit `Type::Union` and `Type::UnionCase` variants, or keep nominal
`Type::Named` values with union metadata in the inference context. The important contract is that
compatibility and common-supertype calculation are context-aware, as record and component
inheritance already are.

### 5. Preserve match-style `if` in HIR for narrowing and exhaustiveness

Current lowering turns match-style `if value is { ... }` into nested equality checks. That loses the
original scrutinee and pattern list, which union narrowing and exhaustiveness need. This change
should introduce an explicit HIR match expression or otherwise preserve equivalent arm metadata
through type checking and interpretation.

For version one, automatic narrowing applies when the scrutinee is a local identifier:

```nx
let view(state: LoadState) = if state is {
  LoadState.failed => <Error message={state.message}/>
  LoadState.loaded => <ItemList items={state.items}/>
}
```

For non-identifier scrutinees, matching still works, but no new narrowed binding is introduced.
Users can bind the expression first. An optional `as` binding remains a future feature.

### 6. Generate language-native closed union surfaces

TypeScript should generate an exported union type whose members carry literal `$type`
discriminators matching the canonical NX case name. Payload fields use authored wire names, and
fieldless cases are represented as objects with only `$type` plus any shared inherited fields.

C# should generate an abstract root type for the union and sealed derived DTOs for each case,
advertising `$type`-based polymorphism with the existing JSON and MessagePack infrastructure.
If a union extends an abstract record, the generated root should inherit or otherwise include that
shared contract so case classes expose shared fields only once.

Alternative considered:
- Generate enum-like code when all cases are fieldless.

Why rejected:
- The source construct is still a discriminated union and uses the `$type` record-like wire shape.
  Pure scalar choices should use `enum`.

## Risks / Trade-offs

- [Changing match lowering may affect existing enum and literal matches] -> Mitigation: preserve
  current runtime behavior with parser, type-checker, and interpreter regression tests before adding
  union-specific narrowing.
- [Fieldless union cases can look like enum members] -> Mitigation: keep different declaration
  keywords and wire contracts explicit in docs, diagnostics, and examples.
- [Generated C# polymorphism for closed unions overlaps with abstract record generation] ->
  Mitigation: share serializer metadata helpers where possible and add tests that cover both
  abstract records and unions in one generated output.
- [Qualified case names may collide with existing qualified markup behavior] -> Mitigation: resolve
  union cases semantically only after parsing, and prefer union-case construction only when the
  qualified tag resolves to a visible union case.
- [Non-exhaustive diagnostics can be noisy during migration] -> Mitigation: require exhaustiveness
  only for union-typed scrutinees without an `else`; existing enum and literal matches keep their
  current behavior.

## Migration Plan

1. Add parser and HIR support for union declarations and case metadata.
2. Add prepared binding and interface metadata so same-library and imported unions resolve through
   the existing module graph.
3. Add semantic validation for duplicate cases, invalid bases, duplicate inherited fields, case
   fields, defaults, and content markers.
4. Add union-aware type checking, construction validation, field access, match narrowing, and
   exhaustiveness diagnostics.
5. Update interpreter construction, matching, canonical value conversion, JSON, and MessagePack
   output.
6. Update TypeScript and C# code generation, then the .NET binding tests for raw and typed union
   workflows.
7. Update VS Code grammar, snippets, fixtures, samples, language reference, language tour, and
   examples.

Rollback is source-compatible for existing NX code because the new syntax is additive and the
existing `enum` and record inheritance behavior stays unchanged.

## Open Questions

No blocking questions. Optional `as` bindings and union aliases over existing types can be proposed
later if real examples justify the added syntax.
