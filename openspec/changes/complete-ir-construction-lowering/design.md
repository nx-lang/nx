## Context

The shared codegen builder rejects conditional property entries and lowers a positional record
constructor as a function call. The runtime already supports conditions and record construction.
The retirement change retains source expectations as ignored tests naming this change.

## Goals / Non-Goals

**Goals:** Restore both source behaviors through compilation and enable their regression tests.
**Non-Goals:** Restore the HIR evaluator or source-evaluation SDK wrappers; alter other language rules.

## Decisions

### Preserve source semantics in compiler lowering

Positional record arguments map to the declaration's ordered fields, using existing construction
and normalization for omitted defaults. Resolve imported records and inherited fields through the
same semantic metadata used by markup construction. A record name must not become a function value.

Conditional fragments must select only the active branch, preserve expression evaluation order,
match-arm narrowing and omitted-versus-empty properties, and validate duplicates before invocation.
Use existing condition and construction operations where they express that behavior; do not eagerly
evaluate inactive branches or expand independent conditions into exponentially many constructions.
If a linear lowering needs an IR feature, specify and version that feature before implementation,
with both runtimes updated together. This follow-up does not assume the IR schema must remain fixed.

### Keep independent expectations

The ignored tests keep the successful HIR expectations captured before retirement. Enable them
as each behavior is implemented and add nested, imported, defaulted and rejected-input cases.
Rust/TypeScript parity supplements explicit expected results; it does not replace them.

## Risks / Trade-offs

- Conditional lowering may need an IR feature → settle the representation in this follow-up,
  preserve version negotiation and runtime/compiler separation.
- Wrong argument/field order or premature defaults → test inherited and imported declarations,
  omitted values, and failures in inactive branches.

## Migration Plan

Implement record construction and conditional fragments, enable the corresponding ignored tests,
run compiler/runtime/CLI parity suites, and remove the documented temporary limitations.
