## Why

Source-driven evaluation and generated NX IR should agree for validated NX programs. Recent
consumer integration found two gaps: explicit `null` values are rejected for nullable union-typed
fields, and some generated IR evaluates to values that the TypeScript IR runtime later rejects at
its own schema boundary even though native normalized JSON is valid.

## What Changes

- Accept explicit `null` at typed binding sites whose expected type is nullable, including
  nullable discriminated-union roots such as `InitialExperience?`.
- Preserve nullability through record and element-style construction so optional nullable fields can
  be omitted or set explicitly to `null` with equivalent normalized output.
- Prevent generated IR from materializing synthetic invalid union cases such as
  `FlowCompletion.undefined` for absent nullable union values.
- Ensure generated IR and the TypeScript IR runtime agree on record/component child-content fields
  and required-field validation so IR emitted from a valid program does not fail runtime boundary
  validation with missing fields that native evaluation already supplied.
- Add parity coverage using imported library records/unions and component-like record construction
  that mirrors chat-link/question-flow shapes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `type-reference-suffixes`: explicit `null` values must be compatible with nullable type
  references at typed binding sites.
- `discriminated-unions`: nullable union construction and normalization must use `null` for absence
  rather than inventing invalid fieldless union cases.
- `nx-ir-format`: emitted IR for valid programs must preserve enough schema/default/content
  metadata for runtimes to evaluate entrypoints to the same canonical value as native evaluation.
- `typescript-ir-runtime`: boundary normalization must accept generated values from valid IR
  programs when they conform to the originating schema, including nullable union fields and
  component/content-derived record fields.

## Impact

- Affects parser/lowering/type-checker compatibility logic for null literals and nullable nominal
  union types.
- Affects NX IR generation for nullable fields, fieldless union absence, record/component
  construction, and schema metadata.
- Affects TypeScript IR runtime boundary normalization and diagnostics.
- Adds regression tests across native evaluation, SDK Node IR generation, and TypeScript IR runtime
  execution.
