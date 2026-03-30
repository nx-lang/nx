## Context

NX currently has one record-shaped top-level declaration, `type Name = { ... }`, and component `emits`
entries only support inline payload definitions such as `Changed { value:string }`. Downstream code in
`nx-hir`, scope building, and the interpreter therefore treats record declarations as a single category,
and the parser does not preserve whether an emitted action entry defines a new action or references an
existing one.

This change needs to add first-class `action` declarations without breaking existing record behavior.
Actions must remain usable anywhere normal records are valid, but the language also needs a reliable way
to tell "this record is an action" apart from ordinary records for future action-only contexts. At the
same time, component syntax must preserve whether an `emits` entry creates a component-owned action or
points at an already-declared shared action.

## Goals / Non-Goals

**Goals:**
- Add a top-level `action <Name> = { ... }` declaration form with the same property syntax as record
  definitions.
- Preserve the fact that actions are record-compatible in existing type lookup, record construction, and
  interpreter paths.
- Extend component `emits` grammar so braced entries define component-scoped actions and unbraced entries
  reference existing actions.
- Preserve enough CST and HIR metadata for later work to require actions in specific contexts.
- Update highlighting, diagnostics, examples, and tests to cover action declarations and mixed `emits`
  groups.

**Non-Goals:**
- Add new runtime behavior or type-checking rules for action-only contexts that do not exist yet.
- Lower component emit metadata into executable runtime behavior in this change.
- Change ordinary record syntax, record construction syntax, or existing `type` declarations.
- Add component-local inline handlers or other new component runtime syntax.

## Decisions

### 1. Add a dedicated `action_definition` CST node

The parser should introduce a distinct top-level node for action declarations rather than folding `action`
into `record_definition`.

Proposed shape:

```text
action_definition
  : 'action' identifier '=' '{' property_definition* '}'
```

`module_definition` should accept `action_definition` alongside `record_definition`, `type_definition`,
and the other existing top-level items. The `action` keyword should also be added to highlight queries,
syntax-kind mappings, generated parser artifacts, and validation fallback messages.

Alternative considered: make `record_definition` accept either `type` or `action`.
Rejected because later diagnostics, highlighting, and lowering need a stable way to distinguish action
declarations from ordinary record declarations.

### 2. Lower actions as records with an explicit kind marker

HIR should keep actions inside the existing record model instead of creating a parallel action item type.
The simplest shape is to add a `RecordKind` enum or equivalent marker to `RecordDef`, with values for
plain records and action records.

Lowering should:
- map `record_definition` to `RecordDef { kind: Plain, ... }`
- map `action_definition` to `RecordDef { kind: Action, ... }`
- continue storing both under `Item::Record`

This keeps existing behavior intact where records are already accepted:
- `Module::find_item` still returns a record item for actions
- scope building still registers actions as `SymbolKind::Type`
- HIR lowering of `<ActionName ... />` can keep producing record literals through the existing
  `Item::Record` path
- interpreter record resolution can continue treating actions like records unless a future caller asks
  specifically for action-only behavior

Alternative considered: add `Item::Action(ActionDef)` as a separate top-level item kind.
Rejected because that would force every record consumer to special-case actions even though the language
rule is that actions are records.

### 3. Preserve `emits` definitions and references as distinct syntax forms

The component grammar should keep the difference between "define a new action here" and "reference an
existing action" explicit in the CST.

Proposed shape:

```text
emits_group
  : 'emits' '{' emit_entry+ '}'

emit_entry
  : emit_definition
  | emit_reference

emit_definition
  : identifier '{' property_definition* '}'

emit_reference
  : qualified_name
```

Using `qualified_name` for references keeps the surface aligned with existing named type references, so
shared actions can later come from imports or namespaces without another grammar change. Braced entries
remain component-scoped definitions; unbraced entries remain references to pre-existing action
declarations.

Alternative considered: keep a single `emit_definition` node and make the payload block optional.
Rejected because later lowering would lose the source-level distinction between an empty-payload new
action and a reference to an existing action.

### 4. Keep record compatibility as the default behavior in existing consumers

The current system already has multiple places that assume "record-shaped declaration" means
`Item::Record`, including module lookup, scope definition, HIR record literal lowering, and interpreter
record resolution. This change should preserve that default instead of threading special action behavior
through every consumer immediately.

The design principle is:
- if a path currently accepts records, it should accept action records unchanged
- only future action-only contexts should branch on the record kind marker

That keeps this change focused on structural support instead of speculative semantic restrictions. It
also means the interpreter and HIR do not need a parallel notion of "action literal" or "action type";
they keep using record behavior until a later feature needs stricter checks.

Alternative considered: start enforcing "must be action" rules in current parser or interpreter paths.
Rejected because no current language surface requires that distinction yet.

### 5. Update parser-facing validation, highlighting, and tests around the new distinction

Parser-owned artifacts should reflect both new syntactic forms:
- highlight `action` as a keyword and action declaration names as types
- add explicit syntax kinds and node metadata for `action_definition` and `emit_reference`
- update validation fallback hints to show canonical `action` syntax and mixed `emits` syntax
- add valid fixtures for standalone actions, components with shared action references, and components
  that mix inline action definitions with shared references
- add malformed parser coverage for incomplete `action` declarations and invalid `emits` references
- add HIR-level tests that confirm action declarations lower as record-compatible items with an action
  marker

Alternative considered: limit this change to grammar production updates and leave the rest for follow-up.
Rejected because the new syntax will be hard to maintain if diagnostics, highlighting, and tests keep the
old record-only assumptions.

## Risks / Trade-offs

- New `action` keyword may conflict with existing identifiers named `action` -> accept the reservation
  now and document it in parser-facing docs and examples.
- Representing actions as records with a kind marker means future action-specific code must remember to
  check that marker -> add small helpers or predicates rather than open-coding record-kind checks.
- Allowing `emit_reference` to use `qualified_name` slightly broadens the initial syntax surface -> keep
  it because it matches existing type-name syntax and avoids an avoidable follow-up grammar change.
- Component `emits` references remain structural only until later lowering/runtime work lands -> keep the
  proposal and specs explicit that this change provides syntax and metadata groundwork, not runtime
  behavior.

## Migration Plan

No migration is needed for existing `type` records or existing component declarations that only use inline
`emits` definitions. Implementation should proceed in this order:

1. Add the grammar, syntax kinds, validation, and highlighting changes for `action_definition` and
   `emit_reference`.
2. Update HIR lowering and any record-resolution helpers so action declarations remain record-compatible
   while carrying an action marker.
3. Extend parser fixtures, HIR tests, and syntax docs/examples to cover top-level actions and mixed
   component `emits` groups.
