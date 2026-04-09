## Context

The current code generation model exports record names, bases, and fields, but it does not carry
whether a record is abstract. That limitation is mostly invisible in TypeScript today because every
record is emitted as a plain structural interface, but it becomes important as soon as the generated
surface must represent runtime record identity.

The immediate consumer problem is in TypeScript: NX payloads already contain `$type`, while
generated record contracts omit it. Downstream renderer registries therefore receive structurally
typed payloads and narrow them with casts instead of using discriminated unions. C# generation is
partially closer to the wire shape because it already emits a `$type` MessagePack key, but it does
so as a nullable string property and does not have enough abstract-record metadata to keep that
discriminator aligned with concrete inheritance.

Because NX record inheritance only allows abstract base records, the generator has a clean boundary
to exploit: abstract records describe shared field shape, while concrete records describe runtime
payloads. The design below makes that distinction explicit in generated output.

## Goals / Non-Goals

**Goals:**
- Make generated TypeScript record contracts match the runtime NX payload shape by exposing `$type`.
- Emit TypeScript output that allows narrowing on `$type` without manual casts for exported concrete
  record families.
- Preserve exported abstract record inheritance in generated TypeScript and C# output.
- Keep generated C# discriminator data non-null and aligned with the concrete record name.
- Add tests that cover single-file and library generation, including cross-module abstract base and
  concrete descendant relationships.

**Non-Goals:**
- Changing NX runtime payload semantics or the existing wire key name `$type`.
- Introducing code generation for declarations beyond the existing exported aliases, enums, and
  record-like declarations.
- Updating downstream frontend packages in this repository change; those consumers are motivation
  for the generator contract, not part of this implementation.
- Reworking MessagePack serialization beyond the discriminator behavior needed for generated record
  types.

## Decisions

### 1. Extend the exported type graph with abstract-record metadata and descendant lookups

`ExportedRecord` should carry `is_abstract`, and the exported graph should expose enough
relationship information to answer:

- whether a referenced record name is abstract or concrete
- which exported concrete descendants belong to an exported abstract record
- which modules own those descendants for import generation

This lets the emitters make shape decisions from the shared model instead of re-deriving them from
raw HIR in language-specific code.

Alternative considered:
- Infer abstract/concrete behavior inside each emitter from only `base` links.

Why rejected:
- The emitters also need descendant ownership for TypeScript union imports, and duplicating that
  traversal in multiple emitters would make the behavior drift-prone.

### 2. TypeScript generation will separate abstract base contracts from concrete runtime types

TypeScript output should add a generated discriminator helper:

```ts
export interface NxRecord<TType extends string = string> {
  $type: TType
}
```

Generation rules:

- exported concrete root records emit a concrete interface that extends `NxRecord<"RecordName">`
- exported abstract records emit a generated `NameBase` interface that carries shared fields and
  extends `NxRecord`
- exported concrete derived records extend their abstract parent's `NameBase` contract and keep a
  literal `$type` field for their own concrete name
- exported action records are treated as concrete records and therefore also receive literal
  discriminators
- the original exported abstract record name becomes a generated type alias that represents the
  concrete runtime surface of that family

For abstract record families with exported concrete descendants, the generated alias should be the
union of those descendants. If an exported abstract record has no exported concrete descendants in
the generated graph, its exported alias should fall back to `NameBase` so existing type references
remain valid.

Alternative considered:
- Keep the current interface names untouched and only add `$type: string` or `$type: "Literal"` to
  the existing interfaces.

Why rejected:
- That smaller change does not solve the downstream narrowing problem when collections or fields are
  typed as the abstract record name, which is the main consumer pain point.

### 3. TypeScript union aliases should live in the abstract record's owning module, even when that requires type-only reverse imports

The generated alias for an abstract record should be emitted in the same module that owns the
abstract record name. When exported concrete descendants live in other modules, the owning module
should generate `import type` statements for those descendants before declaring the union alias.

This keeps the original exported abstract type name stable at its source location and lets existing
references continue to import that name from the same generated module path.

Alternative considered:
- Emit descendant unions only in `index.ts` or skip unions for cross-module families.

Why rejected:
- Moving the alias to a barrel changes where consumers import the type from, and skipping unions for
  cross-module families would reintroduce the exact unsafe-cast behavior this change is meant to
  remove.

### 4. C# generation will make abstract bases inheritable and concrete discriminators non-null

C# output should start honoring abstract record metadata directly:

- exported abstract records emit `abstract` classes instead of sealed classes
- concrete records continue to emit concrete classes
- the generated discriminator member remains mapped to MessagePack key `$type`
- concrete records initialize that discriminator member to their concrete generated record name
- derived concrete records override the inherited discriminator member so they never serialize the
  abstract base name by mistake

The generated member can keep the current language-safe identifier (`__NxType`) so user-defined
record fields do not collide with the MessagePack-mapped discriminator property.

Alternative considered:
- Leave C# generation unchanged because it already emits a `$type` key.

Why rejected:
- The current nullable string property is weaker than the wire contract, and without abstract-aware
  generation it cannot preserve the concrete discriminator correctly for inherited records.

Alternative considered:
- Generate getter-only constant discriminator properties in C#.

Why rejected:
- That shape is riskier for MessagePack object deserialization than an overrideable property with a
  concrete default value.

## Risks / Trade-offs

- [TypeScript abstract record output becomes a breaking generated API change] -> Mitigation:
  document the renamed base-contract pattern in the proposal and add focused tests that lock in the
  new shape.
- [TypeScript union aliases may create import cycles between abstract-base modules and concrete-leaf
  modules] -> Mitigation: emit `import type` only, and add library-generation tests that cover a
  cross-module abstract family.
- [C# discriminator overrides depend on MessagePack handling inherited properties as expected] ->
  Mitigation: add generation tests that assert the emitted inheritance shape and discriminator
  defaults, and expand to serializer-level tests if needed during implementation.
- [Generated output becomes more complex to read] -> Mitigation: keep helper naming predictable
  (`NxRecord`, `NameBase`) and preserve one emitted declaration family per source record.

## Migration Plan

1. Extend the exported code generation model with abstract-record metadata and descendant queries.
2. Update the TypeScript emitter to generate `NxRecord`, abstract `NameBase` contracts, concrete
   literal `$type` fields, and abstract-family unions with the required `import type` statements.
3. Update the C# emitter to honor abstract records, emit inheritable abstract bases, and initialize
   concrete discriminator members with the concrete record name.
4. Add or update code generation tests for TypeScript and C# single-file and library output,
   including abstract-base inheritance and action records.

## Open Questions

No blocking questions. If downstream renderer registries want stronger generic typing in the same
rollout, that can be handled as a consumer follow-up once the generator contract is in place.
