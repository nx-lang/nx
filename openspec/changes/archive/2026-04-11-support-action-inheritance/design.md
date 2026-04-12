## Context

NX already models actions as record-like declarations in HIR. `RecordDef` carries both `kind` and
inheritance metadata, and most downstream consumers already treat actions and records through the
same lookup and shape-resolution paths. The main gap is that the parser still limits top-level
`action` syntax to `action Name = { ... }`, and record validation still assumes only plain records
participate in inheritance.

That leaves the codebase in an awkward state: the shared record model can already represent
abstract/base metadata on actions, but syntax, validation, and some diagnostics still reject the
feature outright. The requested change is therefore more about aligning the language surface with
the existing record architecture than inventing a separate action system.

## Goals / Non-Goals

**Goals:**
- Add top-level `abstract action Name = { ... }` and `action Name extends Base = { ... }` syntax.
- Add inline component emitted action syntax `ActionName extends Base { ... }` for concrete emitted
  actions that reuse abstract action bases.
- Allow action inheritance through the same alias-aware prepared binding model that record
  inheritance already uses.
- Preserve action identity across inherited shape resolution, subtype checks, and generated output.
- Reject mixed inheritance families so plain records and actions cannot extend each other.
- Add parser, lowering, validation, type-checking, interpreter, and code generation coverage for
  abstract and concrete action inheritance chains.

**Non-Goals:**
- Adding `abstract` inline emitted action declarations inside component `emits` blocks.
- Changing component dispatch matching from exact emitted action names to polymorphic action-family
  dispatch.
- Introducing multiple inheritance, field overriding, or action-only runtime semantics beyond
  existing action identity checks.

## Decisions

### 1. Mirror record inheritance syntax on top-level `action` declarations while keeping a distinct CST node

`action_definition` will gain the same optional `abstract` modifier and single optional `extends`
clause that `record_definition` already supports:

```text
action_definition
  : optional(visibility) optional('abstract') 'action' identifier
      optional('extends' qualified_name) '=' '{' property_definition* '}'
```

The parser will continue emitting a dedicated `ACTION_DEFINITION` node rather than merging actions
into `RECORD_DEFINITION`. That keeps lowering, highlighting, and diagnostics able to distinguish
action syntax while still exposing the shared `abstract` and `base` fields through the existing
`ast::RecordDef` wrapper.

Alternative considered: collapse `type` and `action` declarations into one parser node.

Why rejected:
- The codebase already relies on a stable action-vs-record syntax distinction for lowering and
  diagnostics, and action inheritance does not remove that need.

### 1a. Extend inline emitted action definitions with a concrete-only `extends` clause

`emit_definition` should gain the same single optional `extends` clause as top-level actions, but
not the `abstract` modifier:

```text
emit_definition
  : identifier optional('extends' qualified_name) '{' property_definition* '}'
```

Inline emitted actions already synthesize a component-scoped public record name
`<ComponentName>.<ActionName>` during component predeclaration. This change should preserve that
model and simply let the synthesized action record carry a resolved base action.

Inline emitted action rules:
- an inline emitted action remains concrete, even when it extends an abstract base action
- the synthesized public action name remains `<ComponentName>.<ActionName>`
- the optional base resolves through the same alias-aware prepared binding path as top-level action
  inheritance
- only abstract action declarations or aliases to them are valid bases

Alternative considered: require users to declare a top-level derived action and reference it from
  `emits` instead of allowing inline `extends`.

Why rejected:
- Inline emitted actions already exist as first-class public action names. Forcing inheritance
  users to rewrite them as separate top-level declarations would create an arbitrary inconsistency
  between two action forms that otherwise share record-compatible semantics.

### 2. Generalize record-family validation to all record-like declarations, but require same-kind abstract bases

`nx_hir::records` should stop validating inheritance only for `RecordKind::Plain`. Instead, it
should validate every `RecordDef` that declares or may inherit a base, using the same shared base
resolution and effective-shape helpers for records and actions. That includes synthesized inline
emitted action records created during component predeclaration.

Base validation rules:
- `type` declarations may extend only abstract plain records.
- `action` declarations may extend only abstract action records.
- Alias resolution may target either family, but the resolved base kind must match the derived
  declaration kind.
- Concrete declarations of either family remain invalid as bases.

This keeps inheritance behavior uniform while preventing accidental flattening of action-specific
identity into ordinary record hierarchies. Practically, it means the current hard-coded
`ActionRecord` base rejection should become a kind-mismatch diagnostic rather than a blanket ban on
action bases.

Alternative considered: add a parallel action-inheritance resolver beside the existing record
  helpers.

Why rejected:
- The effective shape, alias resolution, cycle detection, and subtype mechanics are already shared.
  Forking them would create drift for almost no benefit.

### 3. Reuse effective-shape and subtype helpers, with action-specific consumers still checking action identity

Inherited actions should use the same base-first field/default resolution and ancestry checks that
records already use. That means `effective_record_shape`, `is_record_subtype`, typed element
construction, and runtime record materialization can all stay on the shared record path.

Action-specific semantics remain layered on top:
- abstract actions are valid in type positions but MUST NOT be instantiated
- consumers that require an action, such as handler input validation, still verify the resolved
  declaration kind is `Action`
- subtype compatibility is family-aware, so a derived action is compatible with an abstract parent
  action of the same lineage but not with an unrelated plain record
- inline emitted derived actions expose inherited base fields through their synthesized
  `<Component>.<Action>` public action name and through the implicit `action` value bound inside
  matching component handlers

Alternative considered: keep action compatibility exact-name-only even after adding action bases.

Why rejected:
- That would make inherited action declarations syntactically legal but semantically weak in the
  type system, undermining the main value of the feature.

### 4. Keep code generation on the shared exported record graph and extend coverage to inherited actions

The code generation model already exports record kind, abstractness, and base links. Action
inheritance should continue flowing through that shared graph instead of introducing separate
action-only emitters.

Generation expectations:
- exported abstract actions participate in the same abstract-base modeling as abstract records
- exported concrete derived actions preserve inherited fields and keep their own concrete `$type`
  discriminator
- cross-module inherited action families keep any needed TypeScript imports and C# base references
  resolvable without special action-only plumbing

The main implementation burden here is locking the behavior down with tests rather than inventing a
new data model.

Alternative considered: defer code generation follow-up work because actions are already
record-compatible internally.

Why rejected:
- Exported inherited actions change observable generated output, so the change is incomplete if the
  generators and their specs do not acknowledge that surface.

## Risks / Trade-offs

- [Adding `abstract` and `extends` to `action` reserves more syntax] -> Mitigation: update
  validation hints, grammar fixtures, docs, and highlighting in the same change.
- [Generalizing record validation to actions may surface new diagnostics in existing edge cases] ->
  Mitigation: keep same-kind rules explicit and add focused tests for both valid and invalid mixed
  hierarchies.
- [Inline emitted actions now share more of the action inheritance surface] -> Mitigation: keep
  them concrete-only, preserve exact emitted-action-name dispatch, and add focused parser/lowering
  coverage for `emits { Action extends Base { ... } }`.
- [Shared subtype helpers make abstract actions more visible in runtime-adjacent paths] ->
  Mitigation: keep action-only gates, such as handler input validation, checking `RecordKind::Action`
  after resolution.
- [Component emit references to abstract actions remain a potential future design space] ->
  Mitigation: keep this rollout scoped to top-level action inheritance and exact-name component
  dispatch, then address polymorphic emit contracts separately if needed.

## Migration Plan

1. Extend `action_definition` and `emit_definition` grammar, generated parser artifacts, AST
   helpers, validation hints, and syntax fixtures for top-level `abstract action` / `action ...
   extends ...` plus inline emitted `Action extends Base { ... }`.
2. Update component predeclaration plus record-family validation and diagnostics in `nx_hir::records`
   to preserve inline emitted action bases, validate action inheritance, and enforce same-kind
   abstract bases.
3. Add or update lowering, type-checking, component-handler, and interpreter tests for inherited
   action field access, inline emitted inherited fields, abstract action non-instantiability, and
   action-family subtype compatibility.
4. Add code generation tests for exported abstract and concrete top-level action families in
   TypeScript and C# single-file and library output.

## Open Questions

None. The requested scope is clear once inline emitted `extends` support is included while
polymorphic component dispatch and abstract inline emits remain separate follow-up concerns.
