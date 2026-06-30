## Context

NX already models nullable type references with the `?` suffix and represents null literals as
fresh nullable values. It also emits NX IR artifacts that the TypeScript runtime can evaluate
without re-running the native interpreter. The current behavior has two observable gaps:

- Explicit `null` does not type-check in some nullable nominal union contexts, such as assigning
  `null` to a field or helper return typed as `InitialExperience?`.
- Some valid source programs emit IR whose evaluated value fails the TypeScript runtime's own
  schema boundary. Recent examples include a synthetic `FlowCompletion.undefined` value for an
  absent nullable union field and a missing `QuestionFlow.steps` field for a component/content
  construction that native normalized JSON preserves.

These failures force consumers to special-case stored normalized JSON even when they already have
valid IR for the same source.

## Goals / Non-Goals

**Goals:**

- Make explicit `null` accepted wherever a nullable expected type is accepted, including nullable
  nominal union roots and nullable record/component fields.
- Make absent nullable union values normalize to `null`, never to a synthetic union case.
- Make generated IR for valid source evaluate through the TypeScript IR runtime to the same
  canonical JSON-compatible value as native evaluation for nullable union fields and child-content
  record/component fields.
- Add regressions that exercise both native type checking/evaluation and emitted-IR execution
  through SDK Node and the TypeScript runtime.

**Non-Goals:**

- No new nullable syntax beyond existing `?` suffixes.
- No open unions, user-defined `undefined` value, or JavaScript-style optional property semantics.
- No broad redesign of component content handling outside the parity cases needed for generated IR.
- No compatibility layer that silently accepts malformed host input unrelated to generated valid IR.

## Decisions

1. Treat `null` as a bottom value for nullable expected types during compatibility checks.

   The type checker should keep `null` as a nullable fresh variable for inference, but typed binding
   sites should accept it directly when the expected type is `T?`. This avoids special-casing every
   nominal union root and keeps existing rejection behavior for non-nullable targets.

   Alternative considered: add a `Null` type distinct from `Nullable<T>`. That would make null
   assignability explicit, but it is larger than needed and risks disturbing existing inference.

2. Preserve `null` as absence for nullable union fields in IR generation.

   IR emission should encode the null literal/default/omitted nullable value as a null literal or
   missing optional field that normalizes to `null`, depending on the construction site. It must not
   encode absence by composing a scoped union case name that is not declared by the union.

   Alternative considered: teach runtimes to accept `<Union>.undefined` as a sentinel. That would
   leak an implementation artifact into canonical values and conflict with closed-union validation.

3. Make TypeScript IR runtime boundary validation use the same effective field and content-field
   contract as the native interpreter.

   Generated record, union-case, and component descriptor expressions already carry schema fields,
   defaults, content-field names, and nominal references. The runtime should normalize those
   generated values through that effective contract and should only reject values that are truly
   missing required fields after defaults/content have been applied.

   Alternative considered: skip boundary validation for values produced inside the same IR program.
   That would hide real IR-generation bugs and reduce diagnostics quality.

4. Validate by parity, not only by isolated unit tests.

   Regression coverage should compare native normalized JSON with TypeScript IR runtime output for
   source programs using imported libraries, nullable union fields, fieldless/payload union cases,
   element-style record construction, and child-content-derived fields.

## Risks / Trade-offs

- Nullable compatibility could accidentally allow `null` for non-nullable targets -> Mitigate with
  negative tests for non-nullable fields, parameters, and helper return annotations.
- Boundary runtime changes could become too permissive for host-supplied bad JSON -> Mitigate by
  keeping unknown-field, missing-required-field, and invalid-union-case diagnostics for public
  boundary APIs.
- IR parity tests may require realistic library fixtures that are larger than current unit cases ->
  Mitigate by adding focused miniature libraries that mirror the relevant shapes rather than
  copying consumer application sources.
