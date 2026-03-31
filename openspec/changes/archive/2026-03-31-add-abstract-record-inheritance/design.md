## Context

NX record types are currently flat. The parser recognizes `type Name = { ... }`, lowering stores
records as `RecordDef { name, kind, properties, span }`, and both the type checker and interpreter
resolve record types by name and then work from the directly declared property list. Named-type
compatibility is also module-agnostic today: `Type::is_compatible_with` only knows about exact
equality, nullability, arrays, functions, and numeric promotion.

This change adds a new semantic layer across the whole stack:

- grammar and editor support for `abstract` and `extends`
- HIR metadata for abstract/base relationships
- static rejection of invalid inheritance and abstract instantiation
- runtime rejection of abstract instantiation
- subtype-aware record compatibility so derived records can be used where an abstract parent type is
  expected

The requested model is still intentionally narrow, but it allows abstract inheritance chains:
abstract records may extend other abstract records, concrete records may extend abstract records, and
only abstract records may act as bases. That means the implementation must enforce both ancestry
validity and abstract non-instantiability consistently everywhere record declarations and record
values are handled.

## Goals / Non-Goals

**Goals:**
- Add `abstract type Name = { ... }` for abstract record declarations.
- Add `abstract type Name extends Base = { ... }` for abstract records that derive from other
  abstract record types.
- Add `type Name extends Base = { ... }` for concrete record declarations that derive from an
  abstract record type.
- Enforce the requested inheritance rule: only abstract records may be extended, abstract records
  cannot be instantiated, and concrete records may be instantiated but cannot be used as base types.
- Make inherited fields and defaults visible anywhere record construction, element-style record
  construction, or typed field binding currently use record metadata.
- Make subtype checks consistent across the type checker and interpreter.
- Update grammar docs, parser artifacts, VS Code grammar, examples, and tests with the new syntax.

**Non-Goals:**
- Multiple inheritance, interfaces, or mixins.
- Extending `action` declarations, enums, or non-record type aliases.
- Structural subtyping between unrelated records.
- Field overriding between a base record and a derived record.
- Allowing concrete records to serve as base types.

## Decisions

### 1. Limit the new syntax to record declarations and encode abstract inheritance directly

Only record declarations gain the new keywords. `type Name = SomeOtherType` remains a type alias, and
`action Name = { ... }` remains a separate record-compatible declaration with no inheritance support.

Proposed grammar shape:

```text
record_definition
  : optional('abstract') 'type' identifier optional('extends' qualified_name) '=' '{'
      property_definition*
    '}'
```

Semantic rules on top of that syntax:

- `extends` is valid only on record declarations, not aliases or actions
- if `extends` is present, the resolved base must be an abstract record
- `abstract type Name extends Base = { ... }` produces an abstract derived record
- `type Name extends Base = { ... }` produces a concrete derived record
- concrete records remain invalid as base types for later declarations

This keeps the user-facing rule simple: abstract records define the inheritance hierarchy and may
form chains, while concrete records are the instantiable leaves.

Alternative considered: preserve the earlier rule that abstract records could only be roots.
Rejected because the updated language requirement explicitly allows abstract derived records, and the
grammar/design should reflect that directly.

### 2. Store inheritance metadata in HIR, but keep record fields local to the declaration

`nx_hir::RecordDef` should gain explicit inheritance metadata, minimally:

- `is_abstract: bool`
- `base: Option<Name>`

`properties` should remain the fields declared directly on that record, not a flattened inherited
list. Effective fields should be computed through helpers that walk the base chain and merge base and
local fields when a consumer needs the fully realized record shape.

This approach keeps lowering simple and preserves better diagnostics:

- forward references continue to work because the base does not need to be fully lowered first
- diagnostics can point at the child declaration for invalid base usage
- tools can distinguish declared fields from inherited fields

Alternative considered: flatten inherited fields into `RecordDef.properties` during lowering.
Rejected because it couples lowering to declaration order, hides whether a field was inherited, and
makes duplicate-field diagnostics less precise.

### 3. Resolve effective record shapes through shared helpers and reject duplicate inherited fields

Add a small record-resolution layer that can:

- resolve a base name through existing alias-aware item lookup
- confirm the resolved base is an abstract record
- produce an effective field list for a record in base-first order
- report duplicate field names between inherited and local fields

That effective-field view should be used by:

- record literal checking in `nx-types`
- element-style record construction checks in `nx-types`
- record construction and default application in `nx-interpreter`
- any later runtime path that needs the full record shape

Rejecting duplicate field names keeps inheritance additive and avoids introducing override semantics
for defaults, field types, or field ordering.

Alternative considered: allow child fields to shadow base fields. Rejected because it introduces
override rules the language does not currently define and weakens the "explicit, simple" model
requested for this feature.

### 4. Keep generic `Type` compatibility module-agnostic and add module-aware record subtype checks

Record inheritance cannot be modeled inside `Type::is_compatible_with` alone because that API does
not have access to module items. The correct place for inheritance-aware compatibility is a
module-aware helper used by the type checker and interpreter.

Design direction:

- keep `Type::is_compatible_with` for the existing generic rules
- add a record-subtype predicate that answers whether `ActualRecord` is the same as or derives from
  `ExpectedRecord`
- thread that predicate into `check_typed_binding`, record/element construction checks, and any
  common-supertype logic that merges named record types
- mirror the same ancestry check in interpreter runtime validation so runtime calls and host-facing
  typed record construction agree with static analysis

This preserves a clean split: generic type math stays local to `nx-types::ty`, while record
inheritance uses HIR/module knowledge where it actually exists.

Alternative considered: teach `Type::Named` or `Type::is_compatible_with` about record ancestry
directly. Rejected because it would either require global mutable type metadata or duplicate module
resolution logic inside a type representation that is currently intentionally context-free.

### 5. Enforce abstract non-instantiability at both static and runtime construction sites

Abstract records are valid in type positions but invalid in construction positions. The implementation
should reject abstract targets anywhere the language can currently build a record value, including:

- record literals lowered from `<RecordName ... />`
- direct record-constructor calls when a record name is invoked like a function
- interpreter APIs such as `instantiate_record_defaults`
- host-facing typed record coercion paths that materialize a concrete record value

Both abstract derived records and concrete derived records should still receive inherited defaults and
field validation through the effective-field helper described above; only the concrete ones are
constructible.

Alternative considered: rely only on the type checker to reject abstract construction. Rejected
because the interpreter and host APIs can be called independently and must enforce the same rule.

### 6. Update editor/docs/tests from the same source-of-truth grammar changes

The syntax addition is cross-cutting, so the parser-facing work must ship with its surrounding
artifacts:

- `crates/nx-syntax/grammar.js` and regenerated parser outputs
- syntax kinds, node metadata, AST helpers, validation hints, and highlight queries
- `src/vscode/syntaxes/nx.tmLanguage.json` and grammar tests
- `nx-grammar.md` and `nx-grammar-spec.md`
- language docs, samples, and examples
- parser, lowering, type-checking, and interpreter tests for valid and invalid inheritance cases

Alternative considered: update only Rust parsing/runtime code first and leave editor/docs follow-up
work for later. Rejected because the request is explicitly to update the whole language surface, and
this feature introduces reserved keywords that should be reflected everywhere immediately.

## Risks / Trade-offs

- [Keeping fields local in HIR means more helper-based resolution] -> Centralize effective-record and
  ancestry helpers instead of open-coding recursive base walks in each consumer.
- [Module-aware subtype checks create a second compatibility path next to `Type::is_compatible_with`]
  -> Keep the split explicit and narrowly scoped to named record inheritance, rather than overloading
  the generic type API.
- [Allowing abstract inheritance chains adds recursive validation paths] -> Keep cycle detection and
  base resolution inside shared helpers so parser, checker, and runtime code all use the same
  ancestry rules.
- [New reserved keywords can conflict with existing identifiers named `abstract` or `extends`] ->
  Accept the reservation now and update docs/tests/examples in the same change.
- [Abstract construction might still be reachable through aliases or host APIs] -> Always resolve the
  ultimate record definition before deciding whether a target is constructible.

## Migration Plan

1. Update `nx-grammar.md` and `nx-grammar-spec.md` with the new record declaration forms and the
   abstract/concrete semantic rules.
2. Extend tree-sitter grammar, generated artifacts, syntax kinds, AST helpers, validation, and VS
   Code grammar/highlighting for `abstract` and `extends`.
3. Add HIR metadata for `is_abstract` and `base`, then update lowering and record-resolution helpers.
4. Update `nx-types` to use effective record fields plus module-aware subtype checks at binding and
   common-supertype sites.
5. Update `nx-interpreter` to reject abstract instantiation and to build concrete derived records
   from the effective field/default set.
6. Add end-to-end fixtures and tests across parsing, lowering, type checking, interpreter behavior,
   examples, and docs.

## Open Questions

None. The requested semantics are explicit enough to implement without leaving design gaps. If NX
later wants interfaces or multiple inheritance, that should be a separate change rather than
loosening this one implicitly.
