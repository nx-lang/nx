## Context

NX already models `abstract external component` / `external component ... extends Base` in the HIR
and exposes effective component contracts (including an ancestor name list). Record types had
record-subtyping and a shared “nearest common named supertype” path in the type checker, but named
type compatibility for values and inference did not consult that component contract metadata. The
interpreter’s runtime check that a record-like value matches an expected named type was also
record-only, so valid derived external values could fail at execution despite a clean type check
once static rules were tightened.

## Goals / Non-Goals

**Goals:**

- Treat derived concrete external component types as subtypes of their abstract external base for
  named-type compatibility (assignments, annotations, and list element typing).
- When inferring the common item type for a multi-item braced value list, consider external
  component inheritance so sibling derived types can unify to a shared abstract external base when
  one exists.
- Mirror the same contract-aware rules in the interpreter when validating external component record
  values against an expected named type, including resolving the contract across the same module
  import model used elsewhere.

**Non-Goals:**

- Changing parsing, external component declaration syntax, or host serialization wire shapes.
- Generalizing subtyping to arbitrary nominal types beyond what effective component contracts already
  describe.
- Altering how abstract external components are instantiated in markup (still not a goal here).

## Decisions

**Decision: Extend named-type satisfaction with a component branch alongside records.**

- **Rationale:** Record subtyping (`is_record_subtype`) already answers “is this named record a
  nominal subtype of that named record?” (declared `extends` chain / ancestor names, not
  field-shape compatibility). External components are a separate nominal family
  keyed by component contract and `extends` metadata, so compatibility is “same type name” or
  “actual’s ancestor chain contains expected’s component name”, using the same effective contract
  data the rest of analysis relies on.
- **Alternatives considered:** Treating all external components as unrelated nominal types (rejected:
  breaks the language’s `extends` story); synthesizing erased structural types for every component
  (rejected: heavy and redundant with contracts).

**Decision: Reuse effective component contracts for least upper bound of two named component types.**

- **Rationale:** The type checker already had `common_record_supertype` built from lineage lists.
  Adding `common_component_supertype` that walks `[self; ...ancestors]` for each side and picks the
  first shared name preserves ordering consistent with the record path and matches intuitive “most
  specific shared base” behavior for external hierarchies.
- **Alternatives considered:** Always falling back to `object[]` for mixed component tags (rejected:
  contradicts explicit annotations like `Question[]` and weakens UX).

**Decision: Interpreter resolves the contract module like other cross-item lookups, then loads the
prepared contract by local name.**

- **Rationale:** `effective_component_contract_by_name` mirrors how record shapes are resolved: if
  the name does not bind to an imported item, use the current module; otherwise follow the resolved
  module and read the prepared effective contract for that name. Failures surface as the existing
  runtime type mismatch path rather than inventing a new error taxonomy.
- **Alternatives considered:** Only checking within the current `LoweredModule` (rejected: wrong for
  qualified/imported definitions).

## Risks / Trade-offs

**[Risk] A visible name resolves to a non-component or a contract lookup fails.**

→ Mitigation: Component branches return false / error only on the component path; record checks
unchanged, so unrelated names keep prior behavior.

**[Risk] Divergence between static checker and interpreter if one side forgets the component path.**

→ Mitigation: Parallel tests in `nx-types` and `nx-interpreter` for the same NX snippets (single
value and mixed list).

**Implementation note:** `nx-types` and `nx-interpreter` each implement record and external-component
subtype checks against `PreparedModule` vs `LoweredModule` respectively. There is no shared helper
today; keep the two implementations aligned with the same rules as `nx_hir::is_record_subtype` and
the type checker’s `component_type_satisfies_expected` (both driven by effective shapes/contracts
from `nx-hir`).

## Migration Plan

Not applicable: language semantics clarification only; no data migration or host API change.

## Open Questions

None for this change set.
