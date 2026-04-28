## MODIFIED Requirements

### Requirement: Typed binding sites coerce scalars to lists and reject list narrowing
When a typed binding site expects a list type, the system SHALL coerce a scalar braced result into
a one-item list. When a typed binding site expects a nullable list type, the system SHALL accept a
non-null list value whose element type is compatible with the nullable list's element type. When a
typed binding site expects a scalar type, the system MUST reject a list-valued braced result unless
an explicit conversion exists. The system SHALL preserve the semantic distinction between a nullable
list `T[]?` and a list of nullable elements `T?[]`.

#### Scenario: Scalar braced value is coerced at a list-typed binding site
- **WHEN** a list-typed parameter or field receives `{item}` and `item` has the expected element type
- **THEN** type checking and interpretation SHALL treat the argument as a one-item list

#### Scenario: Multi-item braced value binds to nullable list field
- **WHEN** a record declares `links:ChatBrandLink[]?` and source constructs `<Brand links={ <ChatBrandLink /> <ChatBrandLink /> } />`
- **THEN** type checking SHALL accept the `links` binding as a non-null `ChatBrandLink[]` value
  assigned to the nullable list field
- **AND** interpretation SHALL preserve the supplied list items rather than treating the field as
  null or omitted

#### Scenario: Annotated nullable list let accepts braced list literal
- **WHEN** source contains `let links:ChatBrandLink[]? = { <ChatBrandLink /> <ChatBrandLink /> }`
- **THEN** type checking SHALL accept the binding because `ChatBrandLink[]` is compatible with
  `ChatBrandLink[]?`

#### Scenario: Nullable list widening still rejects incompatible element types
- **WHEN** a typed binding site expects `ChatBrandLink[]?` and receives `{ <ChatBrandLink /> <OtherLink /> }`
- **THEN** type checking SHALL reject the binding unless `OtherLink` is compatible with
  `ChatBrandLink`
- **AND** the diagnostic SHALL preserve the expected nullable-list shape in its message

#### Scenario: List of nullable elements remains distinct from nullable list
- **WHEN** a typed binding site expects `string[]?` and receives a value of type `string?[]`
- **THEN** type checking SHALL reject the binding because a list containing nullable elements is not
  the same type as a nullable list of non-null elements

#### Scenario: Multi-value brace is rejected at a scalar-typed binding site
- **WHEN** a scalar-typed parameter or field receives `{first second}`
- **THEN** the system SHALL report a semantic compatibility error because the braced result is
  list-valued
