## ADDED Requirements

### Requirement: Hover annotation lines are scoped as declarations

The grammar SHALL scope a line of editor hover content that names a position having no NX spelling
of its own — written as a parenthesized kind followed by the position, as `(property) User.name:
string` — assigning the owner, the name, the annotation colon, the type, and the type's suffixes the
scopes they carry in a declaration.

<para>This is the one construct the grammar scopes that NX source cannot contain. The language has
no standalone form for a parameter, a field, or a union case, so hover writes them with a
parenthesized kind; without a rule for that shape such a line falls outside every declaration
context the grammar scopes inside, leaving its names unscoped and its annotation colon and nullable
`?` scoped as a ternary's. Hover content reaches the grammar as a fenced `nx` block, so the scopes
SHALL be the same there as on a bare line.</para>

<para>The parenthesized kind SHALL carry a scope of its own, and that scope SHALL NOT be one a theme
styles by default: the kind is an editor convention rather than NX, and reads as the surrounding
prose does. The kinds recognized SHALL be exactly those hover writes.</para>

<para>Recognition is bounded by those literal kind words at the start of a line, which is the only
boundary a line-based grammar can draw. A line of element text that opens with one of them in the
same shape SHALL be scoped as hover content; that cost is accepted in exchange for scoping every
fragment hover emits.</para>

#### Scenario: A property annotation is scoped as a declaration's property
- **WHEN** a line reads `(property) ShapeCommon.shadows: Shadow[]?`
- **THEN** `ShapeCommon.` SHALL be scoped `entity.name.type.nx`
- **AND** `shadows` SHALL be scoped `variable.other.property.nx`
- **AND** `Shadow` SHALL be scoped `entity.name.type.nx`
- **AND** `[]?` SHALL be scoped `keyword.operator.type-modifier.nx`

#### Scenario: An annotation colon and a nullable suffix are not a ternary's
- **WHEN** a line reads `(property) ShapeCommon.shadows: Shadow[]?`
- **THEN** the `:` SHALL be scoped `punctuation.separator.type.annotation.nx` and SHALL NOT be
  scoped `punctuation.separator.conditional.nx`
- **AND** the `?` SHALL NOT be scoped `keyword.operator.conditional.nx`

#### Scenario: A union case is scoped the way source spells one
- **WHEN** a line reads `(case) Role.admin`
- **THEN** `Role` SHALL be scoped `entity.name.type.union.nx`
- **AND** `admin` SHALL be scoped `entity.name.type.union.case.nx`

#### Scenario: The kind carries its own scope and no keyword scope
- **WHEN** a line reads `(primitive type) int`
- **THEN** `primitive type` SHALL be scoped `meta.annotation.hover.kind.nx`
- **AND** it SHALL NOT be scoped `keyword.declaration.type.nx`, because the word `type` inside a
  kind does not open a declaration
- **AND** `int` SHALL be scoped `support.type.primitive.nx`

#### Scenario: Every shape marks the line as hover content
- **WHEN** a line reads any of `(property) ShapeCommon.shadows: Shadow[]?`, `(case) Role.admin`, or
  `(built-in type) Element`
- **THEN** the line's tokens SHALL carry `meta.annotation.hover.nx`

#### Scenario: A line whose first word is not a kind is left alone
- **WHEN** a line reads `(counted) items: many`
- **THEN** `items` SHALL NOT be scoped `variable.other.property.nx`
- **AND** `counted` SHALL NOT be scoped `meta.annotation.hover.kind.nx`

#### Scenario: The scopes survive the markdown fence hover content travels in
- **WHEN** `(property) ShapeCommon.shadows: Shadow[]?` is written inside a fenced code block tagged
  `nx` in a markdown document
- **THEN** its tokens SHALL carry the same scopes they carry on a bare line
