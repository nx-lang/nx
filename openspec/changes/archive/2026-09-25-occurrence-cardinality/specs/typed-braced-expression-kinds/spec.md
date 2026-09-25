## MODIFIED Requirements

### Requirement: Typed binding sites coerce scalars to lists and reject list narrowing
When a typed binding site expects a `+` or `*` type, the system SHALL lift an exactly-one braced
result into a one-item sequence, by the lattice in `occurrence-types`: an exactly-one value
satisfies every occurrence, and the lift applies once. When a typed binding site is an optional
property declared `name?:T+`, the system SHALL check a written value against the property's read
type `T*`, so a non-empty braced sequence whose item type is compatible with `T` binds as its items
and the empty value binds as empty. When a typed binding site expects an exactly-one type, the
system MUST reject a braced result that carries an occurrence — a multi-item sequence, a `+` or `*`
value, or an optional — unless an explicit conversion exists. There is no distinction between an
optional sequence and a sequence of optionals: neither is expressible, and the system SHALL NOT
carry one.

#### Scenario: Scalar braced value is coerced at a list-typed binding site
- **WHEN** a `+`-typed parameter or field receives `{item}` and `item` has the expected item type
- **THEN** type checking and interpretation SHALL treat the argument as a one-item sequence

#### Scenario: Multi-item braced value binds to nullable list field
- **WHEN** a record declares `links?:ChatBrandLink+` and source constructs `<Brand links={ <ChatBrandLink /> <ChatBrandLink /> } />`
- **THEN** type checking SHALL accept the `links` binding as a `ChatBrandLink+` value at the
  optional field, whose read type is `ChatBrandLink*`
- **AND** interpretation SHALL preserve the supplied items rather than treating the field as empty
  or omitted

#### Scenario: Annotated nullable list let accepts braced list literal
- **WHEN** source contains `let links:ChatBrandLink* = { <ChatBrandLink /> <ChatBrandLink /> }`
- **THEN** type checking SHALL accept the binding because `ChatBrandLink+` satisfies
  `ChatBrandLink*`

#### Scenario: Nullable list widening still rejects incompatible element types
- **WHEN** a typed binding site is declared `links?:ChatBrandLink+` and receives `{ <ChatBrandLink /> <OtherLink /> }`
- **THEN** type checking SHALL reject the binding unless `OtherLink` is compatible with
  `ChatBrandLink`
- **AND** the diagnostic SHALL name the expected type as `ChatBrandLink*`

#### Scenario: List of nullable elements remains distinct from nullable list
- **WHEN** a file contains `type A = string?+`, `type B = string+?` and `type C = (string?)+`
- **THEN** the system SHALL reject each on its second suffix, as `occurrence-types` requires, so
  neither an optional sequence nor a sequence of optionals is a type a binding site can expect
- **AND** a site declared `names?:string+` SHALL be the one spelling of a sequence that may be
  absent, reading `string*`

#### Scenario: Multi-value brace is rejected at a scalar-typed binding site
- **WHEN** an exactly-one-typed parameter or field receives `{first second}`
- **THEN** the system SHALL report a semantic compatibility error because the braced result is a
  `+` sequence

#### Scenario: An optional is rejected at an exactly-one binding site
- **WHEN** a file contains `type Box = { name:string }`, `let o:string? = {}` and `let b = <Box name={o} />`
- **THEN** type checking SHALL reject the binding, naming `string?` and `string`
