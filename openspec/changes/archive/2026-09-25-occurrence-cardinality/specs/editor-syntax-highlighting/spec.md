## MODIFIED Requirements

### Requirement: Declaration property lists are scoped independent of layout

Within an element-shaped declaration signature, the grammar SHALL scope each property's name, its
optional mark, its annotation colon, its type, its type suffix, and its default value, whether the
property list is written on one line or spread across many. A property name SHALL be scoped
`variable.other.property.nx`. The `?` that marks a property optional (`subtitle?:string`) SHALL be
scoped `keyword.operator.optional.nx` as a token of its own adjacent to the name: it SHALL NOT be
folded into the name's scope and SHALL NOT be folded into the colon's. The annotation colon SHALL be
scoped `punctuation.separator.type.annotation.nx` and SHALL NOT be folded into the type's scope. A
primitive type SHALL be scoped `support.type.primitive.nx` and a user-defined type
`entity.name.type.nx`, in each case covering the complete type name. The `?`, `+` and `*` occurrence
suffixes SHALL be scoped `keyword.operator.type-modifier.nx`; `[]` is not a type suffix and SHALL NOT
receive that scope.

A type in any annotation position — a signature property, a record property, or a value definition —
SHALL be scoped by the same rule, so that the same type text receives the same scopes in all three.
In particular `?` SHALL be a type modifier after a type, the optional mark after a property name and
before its colon, and the presence operator after a value expression, as "The presence operators are
scoped as operators" requires; position alone SHALL decide which.

A property's type SHALL receive the same scope whether or not a default value follows it, and a
default value SHALL receive the same scope whether the property's type is primitive or
user-defined.

A parenthesized function parameter is the same `PropertyDefinition` production as a signature
property, and SHALL be scoped by the same rules — its name `variable.other.property.nx`, its optional
mark, its type by the shared annotation rule, and its default by the `RhsExpression` rule.

#### Scenario: Property with a user-defined type and a contextual default
- **WHEN** a declaration signature contains the property line `format: TextFormat = plain`
- **THEN** `format` SHALL be scoped `variable.other.property.nx`
- **AND** the `:` SHALL be scoped `punctuation.separator.type.annotation.nx`
- **AND** the complete identifier `TextFormat` SHALL be scoped `entity.name.type.nx`
- **AND** no part of `TextFormat` SHALL be scoped `entity.other.attribute-name.nx`
- **AND** `plain` SHALL be scoped `variable.other.enummember.nx`

#### Scenario: Property with a primitive type and a numeric default
- **WHEN** a declaration signature contains the property line `letterSpacing: float64 = 0.0`
- **THEN** the complete identifier `float64` SHALL be scoped `support.type.primitive.nx`
- **AND** `=` SHALL be scoped `keyword.operator.assignment.nx`
- **AND** `0.0` SHALL be scoped `constant.numeric.float.nx`

#### Scenario: Property with type suffixes
- **WHEN** a declaration signature contains the property lines `colors: Color+` and `tags: string*`
- **THEN** `Color` SHALL be scoped `entity.name.type.nx` and `string` `support.type.primitive.nx`
- **AND** the `+` and the `*` SHALL each be scoped `keyword.operator.type-modifier.nx`

#### Scenario: Optional property mark
- **WHEN** a declaration signature contains the property lines `color?: Color` and `items?: string+`
- **THEN** `color` and `items` SHALL be scoped `variable.other.property.nx` and neither token SHALL
  include the `?`
- **AND** each `?` SHALL be scoped `keyword.operator.optional.nx` and SHALL NOT be scoped
  `keyword.operator.type-modifier.nx`
- **AND** each `:` SHALL be scoped `punctuation.separator.type.annotation.nx` and SHALL NOT include
  the `?`
- **AND** the `+` SHALL be scoped `keyword.operator.type-modifier.nx`

#### Scenario: Type suffixes on a record property
- **WHEN** a record body contains `catalogs: CatalogUse+` and `metadata?: DocumentMetadata`
- **THEN** `+` SHALL be scoped `keyword.operator.type-modifier.nx`
- **AND** the `?` SHALL be scoped `keyword.operator.optional.nx`
- **AND** in `let maybe: string? = {}` the `?` SHALL be scoped `keyword.operator.type-modifier.nx`

#### Scenario: Parenthesized function parameter
- **WHEN** a file contains `let f(alpha: int, beta: Color = red, gamma?: int) = alpha`
- **THEN** `alpha`, `beta` and `gamma` SHALL be scoped `variable.other.property.nx` and SHALL NOT be
  scoped `entity.name.qualifier.nx`
- **AND** `int` SHALL be scoped `support.type.primitive.nx` and `Color` `entity.name.type.nx`
- **AND** the `?` after `gamma` SHALL be scoped `keyword.operator.optional.nx`
- **AND** the default `red` SHALL be scoped `variable.other.enummember.nx`

#### Scenario: Content-marked property
- **WHEN** a declaration signature contains the property line `content text: string`
- **THEN** `content` SHALL be scoped `storage.modifier.content.nx`
- **AND** `text` SHALL be scoped `variable.other.property.nx`
- **AND** `string` SHALL be scoped `support.type.primitive.nx`

### Requirement: Comments inside a declaration are scoped as comments

The grammar SHALL scope a `//` line comment appearing anywhere code may appear — including trailing
a property in a declaration signature, a record body, a union case, or a value definition — as
`comment.line.double-slash.nx`, for the whole remainder of the line as a single token. No character
of a comment SHALL be scoped as punctuation, an operator, a literal, or a keyword, and a comment
SHALL NOT terminate the enclosing declaration regardless of the characters it contains.

A comment SHALL NOT be recognized inside text content, where `//` is literal text.

#### Scenario: Trailing comment on a property line
- **WHEN** a declaration signature contains the property line `fontWeight?: float64   // 1..1000`
- **THEN** `// 1..1000` SHALL be scoped `comment.line.double-slash.nx`

#### Scenario: Trailing comment on a record property
- **WHEN** a record body contains `hidden: boolean = false   // true means decorative / ignored`
- **THEN** the whole `// true means decorative / ignored` SHALL be one token scoped
  `comment.line.double-slash.nx`
- **AND** `true` SHALL NOT be scoped `constant.language.boolean.nx`
- **AND** neither `/` SHALL be scoped `keyword.operator.arithmetic.nx`

#### Scenario: Trailing comment on a value definition
- **WHEN** a file contains `let total: int   // true if unset`
- **THEN** `// true if unset` SHALL be scoped `comment.line.double-slash.nx`

#### Scenario: Two slashes in text content are not a comment
- **WHEN** text content contains `not a // comment`
- **THEN** `//` SHALL NOT be scoped `comment.line.double-slash.nx`

#### Scenario: Comment containing signature-terminating characters
- **WHEN** a declaration signature contains the property line `maxLines?: int   // >= 1; null`
- **THEN** the entire `// >= 1; null` SHALL be scoped `comment.line.double-slash.nx`
- **AND** the `/` SHALL NOT be scoped `punctuation.definition.tag.self-closing.nx`
- **AND** the `>` SHALL NOT be scoped `punctuation.definition.tag.end.nx`
- **AND** `null` SHALL be part of the comment token and SHALL carry no constant scope
- **AND** a property line following the comment SHALL still be scoped as a declaration property

### Requirement: A control form's own names are scoped

The grammar SHALL scope a `for` loop's binding variables and its iterable, and SHALL scope a
reserved value literal appearing as a condition. A name that merely shares a keyword's spelling —
`state` as a parameter, `type` as an attribute — SHALL keep the scope its position gives it, because
only `true` and `false` are reserved. The language has no `null`, so the grammar SHALL NOT define a
`constant.language.null.nx` scope: `null` written as a bare word is an ordinary identifier and SHALL
be scoped as one.

#### Scenario: For loop header
- **WHEN** a value or elements expression contains `for item, index in items { … }`
- **THEN** `item` and `index` SHALL be scoped `variable.other.readwrite.nx`
- **AND** the `,` SHALL be scoped `punctuation.separator.comma.nx`
- **AND** `items` SHALL be scoped `variable.other.readwrite.nx`
- **AND** in `for filter in {"all" "active"}` the braced sequence SHALL keep its string scopes

#### Scenario: A loop header spanning trivia
- **WHEN** a loop header is broken across lines, as `for item,` then `index in items {`
- **THEN** `item`, the `,`, `index`, `in`, and `items` SHALL keep the scopes they have on one line
- **AND** the same SHALL hold when a comment separates the binder from `in`

#### Scenario: A compound iterable
- **WHEN** the iterable is not a lone name — `for item in (items)` or `for item in left + right`
- **THEN** each name in it SHALL be scoped `variable.other.readwrite.nx`
- **AND** an operator between them SHALL keep its operator scope

#### Scenario: A control form as the iterable
- **WHEN** the iterable is itself a control form — `for item in if ready { items } else { fallback } { … }`
- **THEN** `if` and `else` SHALL be scoped `keyword.control.conditional.nx`, not as names
- **AND** the names inside the branches SHALL be scoped by the conditional's own patterns
- **AND** the loop body SHALL still be recognized, so the element in it keeps the loop's meta scope

#### Scenario: A sole binder split from its `in`
- **WHEN** a header puts its only binder on one line and `in items {` on the next
- **THEN** the binder, the `in`, and the iterable SHALL be scoped as they are on one line
- **AND** blank lines and comments between the binder and `in` SHALL NOT change that

#### Scenario: A false loop header in wrapped prose ends at its own line
- **WHEN** prose wraps after `for <word>` and the next line neither begins with `in` nor is trivia
- **THEN** only that one word SHALL be scoped `variable.other.readwrite.nx`
- **AND** WHEN the next line does begin with `in`, which is indistinguishable from a header, the
  scoping SHALL still stop at the end of that line rather than continue to the next brace

#### Scenario: A prose `for` does not open a loop header
- **WHEN** `for` appears in running text, as in `An easier way for neighbors to share tools`
- **THEN** the words after it SHALL NOT be scoped `variable.other.readwrite.nx`

#### Scenario: Reserved literal as a condition
- **WHEN** a property list contains `if true { type = "x" }`
- **THEN** `true` SHALL be scoped `constant.language.boolean.nx`
- **AND** `type` SHALL still be scoped `entity.other.attribute-name.nx`
- **AND** `state` in `if state is { … }` SHALL NOT be scoped `keyword.declaration.state.nx`

#### Scenario: The word null is an ordinary name
- **WHEN** a file contains `let null = 1` or `for null in items { … }`
- **THEN** `null` SHALL be scoped as the name its position gives it — `variable.other.readwrite.nx`
  in the loop header
- **AND** SHALL NOT be scoped `constant.language.null.nx`

#### Scenario: A keyword-spelled name in a condition-list arm
- **WHEN** a property list contains `if { state => tone="danger" }`
- **THEN** `state` SHALL NOT be scoped `keyword.declaration.state.nx`
- **AND** `tone` SHALL still be scoped `entity.other.attribute-name.nx`
- **AND** in `if { true => type="x" }` `true` SHALL still be scoped `constant.language.boolean.nx`

#### Scenario: An arm's attribute ends at the arm
- **WHEN** a property list contains `if compact { density="tight" } else { density="normal" }`
- **THEN** both occurrences of `density` SHALL be scoped `entity.other.attribute-name.nx`
- **AND** the element's following `/>` SHALL be scoped as tag punctuation, not as operators

### Requirement: Hover annotation lines are scoped as declarations

The grammar SHALL scope a line of editor hover content that names a position having no NX spelling
of its own — written as a parenthesized kind followed by the position, as
`(property) User.name: string` — assigning the owner, the name, its optional mark, the annotation
colon, the type, and the type's suffix the scopes they carry in a declaration.

This is the one construct the grammar scopes that NX source cannot contain. The language has no
standalone form for a parameter, a field, or a union case, so hover writes them with a parenthesized
kind; without a rule for that shape such a line falls outside every declaration context the grammar
scopes inside, leaving its names unscoped and its annotation colon and its `?` without their
declaration scopes. Hover content reaches the grammar as a fenced `nx` block, so the scopes SHALL be
the same there as on a bare line.

The parenthesized kind SHALL carry a scope of its own, and that scope SHALL NOT be one a theme
styles by default: the kind is an editor convention rather than NX, and reads as the surrounding
prose does. The kinds recognized SHALL be exactly those hover writes.

Recognition is bounded by those literal kind words at the start of a line, which is the only
boundary a line-based grammar can draw. A line of element text that opens with one of them in the
same shape SHALL be scoped as hover content; that cost is accepted in exchange for scoping every
fragment hover emits.

#### Scenario: A property annotation is scoped as a declaration's property
- **WHEN** a line reads `(property) ShapeCommon.shadows?: Shadow+`
- **THEN** `ShapeCommon.` SHALL be scoped `entity.name.type.nx`
- **AND** `shadows` SHALL be scoped `variable.other.property.nx`
- **AND** the `?` SHALL be scoped `keyword.operator.optional.nx`
- **AND** `Shadow` SHALL be scoped `entity.name.type.nx`
- **AND** `+` SHALL be scoped `keyword.operator.type-modifier.nx`

#### Scenario: An annotation colon and a nullable suffix are not a ternary's
- **WHEN** a line reads `(parameter) p: Person?`
- **THEN** the `:` SHALL be scoped `punctuation.separator.type.annotation.nx`
- **AND** the `?` SHALL be scoped `keyword.operator.type-modifier.nx` and SHALL NOT be scoped
  `keyword.operator.presence.nx`

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
- **WHEN** a line reads any of `(property) ShapeCommon.shadows?: Shadow+`, `(case) Role.admin`, or
  `(built-in type) Element`
- **THEN** the line's tokens SHALL carry `meta.annotation.hover.nx`

#### Scenario: A line whose first word is not a kind is left alone
- **WHEN** a line reads `(counted) items: many`
- **THEN** `items` SHALL NOT be scoped `variable.other.property.nx`
- **AND** `counted` SHALL NOT be scoped `meta.annotation.hover.kind.nx`

#### Scenario: The scopes survive the markdown fence hover content travels in
- **WHEN** `(property) ShapeCommon.shadows?: Shadow+` is written inside a fenced code block tagged
  `nx` in a markdown document
- **THEN** its tokens SHALL carry the same scopes they carry on a bare line

### Requirement: A function type is scoped in every type position
In any type position — a signature property, a record property, a function parameter, a return
annotation, a value definition, or a type alias — the grammar SHALL recognize a function type and
SHALL scope the `function` keyword `keyword.other.function.nx`, its `<` and `/>` as the same
punctuation as an element-shaped declaration signature, each parameter by the same rules as a
signature property (`variable.other.property.nx`, its optional mark, the annotation colon, the
type), the `:` before the result `punctuation.separator.type.annotation.nx`, and the result type by
the shared type rule, including a `?`, `+` or `*` suffix on the result. Parentheses around a type
SHALL be scoped `punctuation.definition.type.group.nx`, and a `?`, `+` or `*` after the closing
parenthesis SHALL be scoped `keyword.operator.type-modifier.nx`. Recognition SHALL NOT depend on the
function type fitting on one line. The word `function` outside a type position SHALL NOT be scoped
as a keyword.

#### Scenario: A function-typed property is scoped
- **WHEN** a signature contains `ItemTemplate?: <function Item:TItem Index?:int />: DrawnNode`
- **THEN** `function` SHALL be scoped `keyword.other.function.nx`
- **AND** `Item` and `Index` SHALL be scoped `variable.other.property.nx`
- **AND** the `?` after `ItemTemplate` and the `?` after `Index` SHALL each be scoped
  `keyword.operator.optional.nx`
- **AND** `TItem`, `int` and `DrawnNode` SHALL be scoped as types by the shared rule

#### Scenario: A suffix on a parenthesized function type
- **WHEN** a file contains `type Loaders = (<function />: string?)+`
- **THEN** the parentheses SHALL be scoped `punctuation.definition.type.group.nx`
- **AND** the `?` after `string` and the `+` after the closing parenthesis SHALL each be scoped
  `keyword.operator.type-modifier.nx`

#### Scenario: A function type alias is scoped
- **WHEN** a file contains `type RowTemplate = <function Item:Contact Index:int />: DrawnNode`
- **THEN** `RowTemplate` SHALL be scoped `entity.name.type.nx`
- **AND** the right-hand side SHALL be scoped as a function type

#### Scenario: function as an identifier is not a keyword
- **WHEN** a file contains `let function = 1`
- **THEN** `function` SHALL NOT be scoped `keyword.other.function.nx`

## ADDED Requirements

### Requirement: The presence operators are scoped as operators
The TextMate grammar and the tree-sitter highlight queries SHALL scope the three operators of
`presence-operators` wherever a value expression admits them: the postfix presence test `x?` as
`keyword.operator.presence.nx`, the step `?.` as one token `keyword.operator.optional-access.nx`,
and the fallback `??` as one token `keyword.operator.coalesce.nx`. A `?` SHALL be the presence
operator only when it follows a value expression and is not followed by `:` or `.`; a `?` between
a property name and its colon is the optional mark and a `?` after a type is a type modifier, as
"Declaration property lists are scoped independent of layout" requires. Neither `?.` nor `??` SHALL
be scoped as a presence test followed by a member-access dot or by a second `?`. The grammar SHALL
NOT define `keyword.operator.conditional.nx` or `punctuation.separator.conditional.nx`: the
language has no conditional operator, so a `:` after an expression is not a separator of one.

#### Scenario: A presence test is an operator
- **WHEN** a file contains `if book.author? { … }`
- **THEN** the `?` SHALL be scoped `keyword.operator.presence.nx`
- **AND** `author` SHALL keep its member scope and SHALL NOT include the `?`

#### Scenario: A step is one token
- **WHEN** a file contains `let name = book.author?.name`
- **THEN** `?.` SHALL be one token scoped `keyword.operator.optional-access.nx`
- **AND** neither character SHALL be scoped `keyword.operator.presence.nx` or as a plain
  member-access dot

#### Scenario: A fallback is one token and binds inside concatenation
- **WHEN** a file contains `let byline = "Author: " + book.author?.name ?? "Anonymous"`
- **THEN** `??` SHALL be one token scoped `keyword.operator.coalesce.nx`
- **AND** `+` SHALL keep `keyword.operator.arithmetic.nx` and both string literals SHALL keep
  `string.quoted.double.nx`

#### Scenario: The three spellings of a question mark are told apart by position
- **WHEN** a file contains `let f(p?: Person, q: Person?) = { p?.name ?? q? }`
- **THEN** the `?` after `p` SHALL be scoped `keyword.operator.optional.nx`
- **AND** the `?` after `Person` SHALL be scoped `keyword.operator.type-modifier.nx`
- **AND** `?.` SHALL be scoped `keyword.operator.optional-access.nx`, `??`
  `keyword.operator.coalesce.nx`, and the final `?` `keyword.operator.presence.nx`

#### Scenario: A former ternary carries no conditional scopes
- **WHEN** a file contains `let ratio = ready ? 1 : 2`
- **THEN** no token SHALL be scoped `keyword.operator.conditional.nx` or
  `punctuation.separator.conditional.nx`
