# type-reference-suffixes Specification

## Purpose
Define how NX type references compose postfix list and nullable suffixes while preserving wrapper
order and rejecting redundant same-layer nullability.

## Requirements

### Requirement: Type references support composed postfix list and nullable suffixes
Anywhere NX accepts a type reference, the parser and analysis pipeline SHALL allow a primitive,
qualified, or user-defined base type followed by zero or more postfix suffixes. Supported suffixes
SHALL remain `[]` for list types and `?` for nullable types. The system SHALL apply those suffixes
in source order, preserving the distinction between nested lists, lists of nullable elements, and
nullable lists. The system SHALL reject a nullable suffix when it would make the same outer type
layer nullable twice.

#### Scenario: Nested list alias is accepted
- **WHEN** a file contains `type Matrix = string[][]`
- **THEN** parsing and lowering SHALL accept `Matrix`
- **AND** SHALL preserve it as a list whose element type is `string[]`

#### Scenario: Nullable list field is accepted
- **WHEN** a file contains `type SearchState = { queries:string[]? }`
- **THEN** parsing and lowering SHALL accept the `queries` field type
- **AND** SHALL preserve `queries` as a nullable list of `string`

#### Scenario: Suffix ordering remains semantically distinct
- **WHEN** a file contains `type ListOfMaybeStrings = string?[]` and `type MaybeStringList = string[]?`
- **THEN** analysis SHALL treat `ListOfMaybeStrings` as a list of nullable `string` values
- **AND** SHALL treat `MaybeStringList` as a nullable list of non-null `string` values

#### Scenario: Composed suffixes are accepted on callable type annotations
- **WHEN** a file contains `let loadUsers(): User[]? = result`
- **THEN** parsing and lowering SHALL accept the return annotation
- **AND** SHALL preserve it as a nullable list of `User`

#### Scenario: Duplicate nullable suffixes on the same layer are rejected
- **WHEN** a file contains `type TooNullable = string??` and `type TooNullableNested = string?[]??`
- **THEN** parsing SHALL produce a validation error on the second `?` in `TooNullable`
- **AND** SHALL produce a validation error on the second trailing `?` in `TooNullableNested`
- **AND** SHALL explain that the affected type layer is already nullable
- **AND** SHALL continue to accept `type MaybeNested = string?[]?`
