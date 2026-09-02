## Purpose

Defines the TextMate scopes the NX grammar assigns to NX source, so that editors and themes that
consume the published grammar colorize NX declarations consistently. This capability covers
element-shaped declarations — components and `let` functions — their modifiers, property lists,
`emits` and `state` groups, and the requirement that a declaration's scopes never leak past the
declaration's terminator.

## ADDED Requirements

### Requirement: Element-shaped declarations are scoped as declarations

The grammar SHALL recognize a component or `let` function declaration whose signature is written in
element form, and SHALL scope the declared name as a type declaration rather than as an element
reference. The grammar SHALL accept every combination of the `private`/`export` visibility
modifiers with the `abstract` and `external` modifiers, in the order the language permits, and SHALL
assign each modifier its own keyword scope. The `extends` base name SHALL be scoped as a type
reference.

Recognition SHALL NOT depend on the signature fitting on one line, on the signature having no
default values, or on the signature having no `emits` or `state` group.

#### Scenario: External component declaration is scoped as a declaration
- **WHEN** a file contains `export external component <Text extends UiCommon` followed by a property
  list and a closing `/>`
- **THEN** `export` SHALL be scoped `storage.modifier.visibility.nx`
- **AND** `external` SHALL be scoped `storage.modifier.external.nx`
- **AND** `component` SHALL be scoped `keyword.declaration.component.nx`
- **AND** `Text` SHALL be scoped `entity.name.type.nx` and SHALL NOT be scoped `entity.name.tag.nx`
- **AND** `extends` SHALL be scoped `keyword.declaration.extends.nx`
- **AND** `UiCommon` SHALL be scoped `entity.name.type.nx`

#### Scenario: Abstract external component declaration is scoped as a declaration
- **WHEN** a file contains `export abstract external component <UiCommon` followed by a property list
  and a closing `/>`
- **THEN** `abstract` SHALL be scoped `storage.modifier.abstract.nx`
- **AND** `external` SHALL be scoped `storage.modifier.external.nx`
- **AND** `UiCommon` SHALL be scoped `entity.name.type.nx`

#### Scenario: Element-shaped function declaration is scoped as a declaration
- **WHEN** a file contains `export let <Row gap: float64 = 0.0 content child: Element /> : Element = <HStack />`
- **THEN** `let` SHALL be scoped `keyword.declaration.let.nx`
- **AND** `Row` SHALL be scoped as a declared name and SHALL NOT be scoped `entity.name.tag.nx`
- **AND** the declaration's return type `Element` SHALL be scoped `entity.name.type.nx`

An attribute name SHALL be scoped `entity.other.attribute-name.nx` even when it is spelled the same
as a keyword, and its `=` and value SHALL be scoped as for any other attribute. The same keyword in
a position where it is not an attribute name SHALL keep its keyword scope.

#### Scenario: Element references keep their element scope
- **WHEN** a file contains the element reference `<Button x=1 y=2/>` in a value position
- **THEN** `Button` SHALL be scoped `entity.name.tag.nx`
- **AND** `x` SHALL be scoped `entity.other.attribute-name.nx`

#### Scenario: Attribute named after a keyword
- **WHEN** a file contains `<Question type = "multiple" />`, or the same attribute inside a
  property-list `if`
- **THEN** `type` SHALL be scoped `entity.other.attribute-name.nx` and SHALL NOT be scoped
  `keyword.declaration.type.nx`
- **AND** the `=` SHALL be scoped `keyword.operator.assignment.nx`
- **AND** `"multiple"` SHALL be scoped `string.quoted.double.nx` rather than left unscoped
- **AND** `type` in `export type Mode = light | dark` SHALL still be scoped
  `keyword.declaration.type.nx`

### Requirement: Declaration property lists are scoped independent of layout

Within an element-shaped declaration signature, the grammar SHALL scope each property's name, its
annotation colon, its type, its type suffixes, and its default value, whether the property list is
written on one line or spread across many. A property name SHALL be scoped
`variable.other.property.nx`. The annotation colon SHALL be scoped
`punctuation.separator.type.annotation.nx` and SHALL NOT be folded into the type's scope. A
primitive type SHALL be scoped `support.type.primitive.nx` and a user-defined type
`entity.name.type.nx`, in each case covering the complete type name. The `?` and `[]` type
suffixes SHALL be scoped `keyword.operator.type-modifier.nx`.

A type in any annotation position — a signature property, a record property, or a value definition —
SHALL be scoped by the same rule, so that the same type text receives the same scopes in all three.
In particular `?` SHALL be a type modifier in a type position and SHALL remain
`keyword.operator.conditional.nx` in a ternary.

A property's type SHALL receive the same scope whether or not a default value follows it, and a
default value SHALL receive the same scope whether the property's type is primitive or
user-defined.

A parenthesized function parameter is the same `PropertyDefinition` production as a signature
property, and SHALL be scoped by the same rules — its name `variable.other.property.nx`, its type by
the shared annotation rule, and its default by the `RhsExpression` rule.

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
- **WHEN** a declaration signature contains the property lines `color: Color?` and `items: string[]?`
- **THEN** `Color` SHALL be scoped `entity.name.type.nx` and `string` `support.type.primitive.nx`
- **AND** each `?` and each `[]` SHALL be scoped `keyword.operator.type-modifier.nx`

#### Scenario: Type suffixes on a record property
- **WHEN** a record body contains `catalogs: CatalogUse[]` and `metadata: DocumentMetadata?`
- **THEN** `[]` and `?` SHALL both be scoped `keyword.operator.type-modifier.nx`
- **AND** the `?` SHALL NOT be scoped `keyword.operator.conditional.nx`
- **AND** the `?` in `let ratio = ready ? 1 : 2` SHALL still be scoped
  `keyword.operator.conditional.nx`

#### Scenario: Parenthesized function parameter
- **WHEN** a file contains `let f(alpha: int, beta: Color? = red) = alpha`
- **THEN** `alpha` and `beta` SHALL be scoped `variable.other.property.nx` and SHALL NOT be scoped
  `entity.name.qualifier.nx`
- **AND** `int` SHALL be scoped `support.type.primitive.nx` and `Color` `entity.name.type.nx`
- **AND** the `?` SHALL be scoped `keyword.operator.type-modifier.nx`
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
- **WHEN** a declaration signature contains the property line `fontWeight: float64?   // 1..1000`
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
- **WHEN** a declaration signature contains the property line `maxLines: int?   // >= 1; null`
- **THEN** the entire `// >= 1; null` SHALL be scoped `comment.line.double-slash.nx`
- **AND** the `/` SHALL NOT be scoped `punctuation.definition.tag.self-closing.nx`
- **AND** the `>` SHALL NOT be scoped `punctuation.definition.tag.end.nx`
- **AND** `null` SHALL NOT be scoped `constant.language.null.nx`
- **AND** a property line following the comment SHALL still be scoped as a declaration property

### Requirement: An RhsExpression is scoped alike at every site that admits one

The grammar SHALL scope what follows `=` by the `RhsExpression` production — an element, a literal, a
signed numeric literal, a contextual name, or a braced expression — identically at every site the
language admits one: a value definition, a function definition, a record property default, a
signature property default, a parenthesized parameter default, and an attribute value.

A bare name in that position is a `ContextualName` and SHALL be scoped
`variable.other.enummember.nx`; it SHALL NOT be scoped `entity.name.qualifier.nx` and SHALL NOT be
left unscoped. Because a `ContextualName` is a single identifier and never a qualified name, a
dotted name in a value position SHALL NOT be scoped as a qualified union case.

#### Scenario: The same right-hand side at every site
- **WHEN** the right-hand side `miter` appears in `let lineJoin: LineJoin = miter`, in
  `let <Row gap: float64 /> : LineJoin = miter`, as a record property default, as a signature
  property default, as a parenthesized parameter default, and as an attribute value
- **THEN** `miter` SHALL be scoped `variable.other.enummember.nx` in all six
- **AND** the literals `42`, `1.5`, `true`, `"hi"`, and `-7` SHALL each receive their own literal
  scope in all six
- **AND** an element right-hand side `<Foo />` SHALL scope `Foo` as `entity.name.tag.nx` in all six
- **AND** a braced right-hand side SHALL carry `meta.values-braced-expression.nx` in all six

#### Scenario: A dotted name is not a qualified union case
- **WHEN** a record property default or an attribute value is written `LineCap.butt`
- **THEN** `LineCap` SHALL NOT be scoped `entity.name.type.union.nx`

#### Scenario: A module-qualified name is not a union case
- **WHEN** a record body contains `profile: models.Profile`
- **THEN** `models.Profile` SHALL be scoped `entity.name.type.nx`
- **AND** SHALL NOT be scoped `variable.other.enummember.nx`

### Requirement: A control form's own names are scoped

The grammar SHALL scope a `for` loop's binding variables and its iterable, and SHALL scope a
reserved value literal appearing as a condition. A name that merely shares a keyword's spelling —
`state` as a parameter, `type` as an attribute — SHALL keep the scope its position gives it, because
only `true`, `false`, and `null` are reserved.

#### Scenario: For loop header
- **WHEN** a value or elements expression contains `for item, index in items { … }`
- **THEN** `item` and `index` SHALL be scoped `variable.other.readwrite.nx`
- **AND** the `,` SHALL be scoped `punctuation.separator.comma.nx`
- **AND** `items` SHALL be scoped `variable.other.readwrite.nx`
- **AND** in `for filter in ["all", "active"]` the list literal SHALL keep its string scopes

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

#### Scenario: A keyword-spelled name in a condition-list arm
- **WHEN** a property list contains `if { state => tone="danger" }`
- **THEN** `state` SHALL NOT be scoped `keyword.declaration.state.nx`
- **AND** `tone` SHALL still be scoped `entity.other.attribute-name.nx`
- **AND** in `if { true => type="x" }` `true` SHALL still be scoped `constant.language.boolean.nx`

#### Scenario: An arm's attribute ends at the arm
- **WHEN** a property list contains `if compact { density="tight" } else { density="normal" }`
- **THEN** both occurrences of `density` SHALL be scoped `entity.other.attribute-name.nx`
- **AND** the element's following `/>` SHALL be scoped as tag punctuation, not as operators

### Requirement: Emits and state groups are scoped

The grammar SHALL scope the `emits` and `state` keywords as declaration keywords, SHALL scope each
emitted action name declared in an `emits` group as a type declaration, and SHALL scope the
properties inside an `emits` or `state` group by the same rules as a signature property list.

#### Scenario: Emits group inside a component signature
- **WHEN** a component signature contains `emits { ValueChanged extends InputAction { value: string } }`
- **THEN** `emits` SHALL be scoped `keyword.declaration.emits.nx`
- **AND** `ValueChanged` SHALL be scoped `entity.name.type.nx`
- **AND** `extends` SHALL be scoped `keyword.declaration.extends.nx`
- **AND** `InputAction` SHALL be scoped `entity.name.type.nx`
- **AND** `value` SHALL be scoped `variable.other.property.nx`

#### Scenario: State group inside a component body
- **WHEN** a component body contains `state { query: string = "" }`
- **THEN** `state` SHALL be scoped `keyword.declaration.state.nx`
- **AND** `query` SHALL be scoped `variable.other.property.nx`
- **AND** `""` SHALL be scoped `string.quoted.double.nx`

### Requirement: Declaration scopes end at the declaration's terminator

The grammar SHALL end an element-shaped declaration's scope at the signature's closing `/>`, and
SHALL scope that `/>` as tag punctuation rather than as arithmetic or comparison operators. A
declaration's scopes SHALL NOT extend into the following declaration.

Every construct that introduces a top-level declaration — including one preceded by the `abstract`
and `external` modifiers — SHALL terminate an open union case, record body, or signature from an
earlier declaration. Consequently, the scopes assigned to a declaration SHALL NOT depend on which
declarations precede it in the file.

#### Scenario: Signature terminator is tag punctuation
- **WHEN** a file contains a component declaration whose signature ends with `/>` on its own line
- **THEN** the `/` SHALL be scoped `punctuation.definition.tag.self-closing.nx`
- **AND** the `>` SHALL be scoped `punctuation.definition.tag.end.nx`
- **AND** neither SHALL be scoped `keyword.operator.arithmetic.nx` or
  `keyword.operator.comparison.nx`

#### Scenario: External declaration terminates a preceding multi-line union
- **WHEN** a file contains a multi-line union declaration such as `type TrackSize =` with
  leading-pipe cases, followed by `export external component <Icon extends UiCommon`
- **THEN** the component declaration's tokens SHALL NOT carry
  `meta.definition.type.union.case.nx`
- **AND** `Icon` SHALL be scoped `entity.name.type.nx`

#### Scenario: Declaration scoping is independent of file position
- **WHEN** a component declaration is tokenized on its own and again as the last declaration of a
  file containing many preceding unions, records, and component declarations
- **THEN** every token of that declaration SHALL receive the same scopes in both cases
