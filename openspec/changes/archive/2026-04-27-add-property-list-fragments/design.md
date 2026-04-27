## Context

NX markup property lists already parse more than plain `name=value` entries. The grammar includes
simple conditional, condition-list, and match-style property-list entries, but HIR lowering only
collects direct `PROPERTY_VALUE` children into `Element.properties: Vec<Property>`. Non-value
property-list nodes are currently ignored, which can silently remove user-authored bindings.

Value expressions have `Expr::If` and `Expr::Match`, but property-list fragments are not value
expressions. They produce zero or more key/value bindings and need branch-aware validation for
required props, duplicates, content properties, handler bindings, and union narrowing.

## Goals / Non-Goals

**Goals:**
- Represent property-list entries explicitly in HIR, including direct values, conditional
  fragments, condition-list fragments, and match fragments.
- Preserve source order and spans for diagnostics while avoiding last-wins property semantics.
- Type check property fragments path-sensitively so each possible branch is validated as a
  coherent property set.
- Reuse existing value-match union narrowing and exhaustiveness semantics inside property-list
  match arms.
- Evaluate property fragments at runtime before invocation binding so component, function, record,
  and union-case calls receive the active property set.
- Replace silent drops with supported lowering and diagnostics.

**Non-Goals:**
- Do not change ordinary value `Expr::Match` or make property fragments first-class runtime values.
- Do not introduce spread/rest properties or dynamic property names.
- Do not allow duplicate properties on one execution path, even when a later duplicate could
  override an earlier one.
- Do not add `as` bindings or new pattern syntax for property-list matches.

## Decisions

### Add a property-fragment HIR instead of lowering to `Expr::Match`

Introduce an element property representation along these lines:

```rust
pub enum PropertyEntry {
    Value(Property),
    If {
        condition: ExprId,
        then_entries: Vec<PropertyEntry>,
        else_entries: Vec<PropertyEntry>,
        span: TextSpan,
    },
    ConditionList {
        arms: Vec<PropertyConditionArm>,
        else_entries: Vec<PropertyEntry>,
        span: TextSpan,
    },
    Match {
        scrutinee: ExprId,
        arms: Vec<PropertyMatchArm>,
        else_entries: Vec<PropertyEntry>,
        span: TextSpan,
    },
}
```

`Element` should retain enough structure for analysis and runtime evaluation. The implementation
may replace `properties: Vec<Property>` with `properties: Vec<PropertyEntry>`, or temporarily keep
both a raw property-entry list and helper APIs that expose direct value entries where older code
needs them during migration.

Alternative considered: lower property-list matches into `Expr::Match` whose arms return a list of
properties. This conflates value expressions with invocation binding fragments, makes property
duplicate analysis indirect, and would require inventing a runtime value for property sets. A
dedicated property-fragment HIR keeps the domains separate.

### Validate property sets path-sensitively

Type checking should expand property entries into possible property paths for validation rather
than flattening the entire syntax tree into one map. A direct property contributes to every active
path. A conditional contributes the then-paths and else-paths. A match contributes each arm path
and, when present, else paths.

Rules:
- A required prop is satisfied only if every reachable path contains that prop or content binding.
- A duplicate prop is rejected when two entries with the same key can occur on the same path.
- The same key is allowed in mutually exclusive branches.
- A static direct property plus a conditional branch that can provide the same key is rejected,
  because that duplicate can occur at runtime.

This validation should feed the existing binding logic for component props, record literals,
function-style markup invocations, and union-case constructors.

### Reuse match narrowing for property-list match arms

Property-list match arms should share the scrutinee and pattern semantics of value `if value is`
matches. When the scrutinee is a local identifier of union type, the identifier is narrowed in each
arm while property values in that arm are inferred. Non-exhaustive union matches without an else
fragment should be rejected using the same rule as value matches.

The implementation should factor common match-pattern validation and narrowing setup so value
matches and property-list matches do not diverge.

### Evaluate fragments before binding invocation arguments

Runtime evaluation should produce an ordered active property list from `PropertyEntry` values, then
reuse existing invocation binding behavior. Conditions evaluate to booleans. Match fragments choose
the first matching arm or the else fragment. Evaluation should preserve existing source order
within the selected path.

Because static analysis rejects duplicates on possible paths, runtime does not need last-wins
behavior. If runtime reaches an impossible duplicate due to an analysis bypass, it should fail
rather than silently overwrite.

### Surface unsupported syntax while implementation is incomplete

During incremental implementation, if any property-list fragment kind cannot yet be fully lowered
or analyzed, lowering should emit an explicit unsupported-feature diagnostic for that fragment
instead of dropping it.

## Risks / Trade-offs

- Property-set path expansion can grow with deeply nested conditionals. Mitigate by validating with
  compact key-set summaries and adding a conservative complexity limit if needed.
- Refactoring `Element.properties` touches several subsystems. Mitigate by adding helper methods
  for direct-property iteration and updating call sites incrementally.
- Required-property analysis may produce conservative errors for complex condition structures.
  Mitigate by documenting that required props must be present on every statically reachable path.
- Reusing union narrowing in a second expression context can duplicate inference state handling.
  Mitigate by factoring shared match-arm analysis helpers.

## Migration Plan

1. Add the property-entry HIR and lower all property-list fragment syntax into it.
2. Update HIR visitors, scope construction, handler rewrite collection, and diagnostics to walk
   nested property entries.
3. Update type checking to validate possible property paths before invoking existing target
   binding checks.
4. Update interpreter evaluation to resolve active property entries into the invocation property
   list.
5. Update docs, examples, and VS Code grammar tests.
6. Remove the future-note entry once the change is implemented and archived.

Rollback is straightforward before archive: keep the current grammar but emit unsupported-feature
diagnostics for property-list fragment nodes.

## Open Questions

- Should empty match arms be permitted as an explicit "provide no properties" branch? The existing
  grammar allows optional property lists; the proposal assumes yes.
- Should non-exhaustive property-list matches with no else be rejected only for union scrutinees, or
  should all match fragments require an else unless statically exhaustive? The proposal follows
  current value-match behavior and only requires union exhaustiveness when the scrutinee is a
  discriminated union.
