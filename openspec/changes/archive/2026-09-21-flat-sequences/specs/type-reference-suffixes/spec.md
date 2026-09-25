## ADDED Requirements

### Requirement: Type references compose one sequence suffix with nullable suffixes
Anywhere NX accepts a type reference, the parser and analysis pipeline SHALL allow a primitive,
qualified, user-defined, function, or parenthesized base type followed by zero or more postfix
suffixes. A parenthesized type SHALL be `(`, a type reference, `)`, and SHALL denote the type it
encloses. Supported suffixes SHALL remain `[]` for sequence types and `?` for nullable types. The
system SHALL apply those suffixes in source order, preserving the distinction between a sequence of
nullable elements and a nullable sequence. A suffix written after a function type's result SHALL
apply to the result, so a suffix on the function itself requires parentheses. The system SHALL
reject a nullable suffix when it would make the same outer type layer nullable twice, including
across a parenthesis. The system SHALL reject a `[]` suffix when the type it applies to is already a
sequence, whether that sequence was spelled in the same suffix chain, inside a parenthesis, as a
function type's result, or through an alias, so that at most one `[]` applies along any type
reference chain.

#### Scenario: Nested sequence suffixes are rejected
- **WHEN** a file contains `type Matrix = string[][]`, `type Maybe = string[]?[]` and `type Paren = (string[])[]`
- **THEN** parsing SHALL produce a validation error on the second `[]` of each
- **AND** SHALL explain that a sequence cannot contain sequences
- **AND** SHALL continue to accept `type Names = string[]` and `type MaybeNames = string[]?`

#### Scenario: A sequence alias does not take a further sequence suffix
- **WHEN** a file contains `type Names = string[]` and `type Rows = Names[]`
- **THEN** analysis SHALL reject `Rows` with a diagnostic naming `Names` as already a sequence
- **AND** SHALL continue to accept `type MaybeNames = Names?`

#### Scenario: A function type's sequence result does not take a further sequence suffix
- **WHEN** a file contains `type Bad = <function />: string[][]` and `type Loaders = (<function />: string[])[]`
- **THEN** parsing SHALL reject `Bad` on the second `[]`
- **AND** SHALL accept `Loaders` as a sequence of functions, each returning a sequence of strings

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

#### Scenario: A parenthesized function type takes a suffix
- **WHEN** a file contains `type Template = (<function Item:object />: string)?` and `type Templates = (<function Item:object />: string)[]`
- **THEN** parsing and lowering SHALL accept both
- **AND** SHALL preserve `Template` as a nullable function type and `Templates` as a list of
  non-null function types

#### Scenario: Parentheses around a plain type are accepted
- **WHEN** a file contains `type Names = (string)[]`
- **THEN** parsing and lowering SHALL accept `Names` as a list of `string`, the same type as
  `string[]`

#### Scenario: A duplicate nullable across a parenthesis is rejected
- **WHEN** a file contains `type Twice = (string?)?`
- **THEN** parsing SHALL produce a validation error on the outer `?`
- **AND** SHALL explain that the affected type layer is already nullable

## REMOVED Requirements

### Requirement: Type references support composed postfix list and nullable suffixes
**Reason**: A sequence never contains a sequence, so the composed-suffix rule that accepted `string[][]` is replaced by one that admits at most one `[]` per type reference chain.
**Migration**: Replace any `T[][]` with a record whose field is `T[]`, or with `T[]` where the nesting was not meant. `T?[]` and `T[]?` are unchanged.
