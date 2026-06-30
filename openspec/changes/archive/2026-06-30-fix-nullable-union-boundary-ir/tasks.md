## 1. Reproduce And Pin Current Failures

- [x] 1.1 Add native type-check/evaluation regressions for explicit `null` assigned to nullable union fields and nullable union helper returns.
- [x] 1.2 Add negative native regressions proving explicit `null` is still rejected for non-nullable union targets.
- [x] 1.3 Add IR parity regression fixtures with a nullable union field omitted and explicitly set to `null`.
- [x] 1.4 Add IR parity regression fixtures where element body content supplies a required content property on an imported record or external component shape.

## 2. Nullable Type Compatibility

- [x] 2.1 Update type compatibility/inference so a `null` literal satisfies any nullable expected type without widening non-nullable targets.
- [x] 2.2 Ensure record construction, element-style construction, function returns, and local bindings all use the corrected nullable expected-type path.
- [x] 2.3 Verify diagnostics for non-nullable targets remain precise and do not regress to fresh-type-variable wording.

## 3. Union Absence And Native Normalization

- [x] 3.1 Ensure omitted nullable union fields materialize as canonical `null` during native evaluation.
- [x] 3.2 Ensure explicit `null` nullable union fields materialize as canonical `null` during native evaluation.
- [x] 3.3 Confirm declared fieldless union cases still materialize as scoped union case values and remain distinct from `null`.

## 4. NX IR Generation

- [x] 4.1 Update IR generation so nullable union absence is encoded as null or omitted nullable input that normalizes to null, never as a synthetic union case.
- [x] 4.2 Preserve content-property field names and body expressions in record, union-case, component descriptor, and external component IR operations.
- [x] 4.3 Preserve module-qualified nominal references for imported record, component, and union schemas used by the new parity fixtures.
- [x] 4.4 Add IR artifact assertions that emitted schema/default/content metadata is present for the new nullable and content-property fixtures.

## 5. TypeScript IR Runtime Boundary Normalization

- [x] 5.1 Update runtime normalization so nullable union `null` values pass boundary validation and return canonical `null`.
- [x] 5.2 Apply content-property bindings before required-field validation for generated record, union-case, and component descriptor values.
- [x] 5.3 Keep malformed host input rejection for unknown fields, missing non-nullable fields, and undeclared union discriminators.
- [x] 5.4 Add TypeScript runtime tests covering valid generated IR values and malformed public boundary inputs.

## 6. SDK And Binding Coverage

- [x] 6.1 Add SDK Node tests that compile a workspace with imported library records/unions/components, emit IR, and compare TypeScript IR runtime output with native normalized JSON.
- [x] 6.2 Add .NET binding or FFI smoke coverage if the changed IR metadata crosses those public artifact APIs.
- [x] 6.3 Update any generated TypeScript declarations or native fixture snapshots affected by the corrected nullable/content metadata.

## 7. Verification

- [x] 7.1 Run the relevant Rust unit/integration tests for `nx-types`, `nx-api`, and `nx-codegen`.
- [x] 7.2 Run TypeScript IR runtime tests.
- [x] 7.3 Run SDK Node tests.
- [x] 7.4 Run .NET binding tests when binding-visible metadata changed.
- [x] 7.5 Run OpenSpec validation for `fix-nullable-union-boundary-ir`.
