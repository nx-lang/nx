# braced-value-sequences Specification

## Purpose
TBD - created by archiving change support-multi-value-braced-expressions. Update Purpose after archive.

## Requirements

### Requirement: Values braced expressions support singleton and space-delimited list forms
The parser SHALL recognize `ValuesBracedExpression` as `{ ... }` containing either nothing at all, a
single `ValueExpression`, or a space-delimited sequence of two or more `ValueListItemExpression`
entries, preserving item order. The empty form `{}` SHALL parse in every position that accepts a
`ValuesBracedExpression`, and admitting it SHALL NOT change how any source that parses today parses.

This SHALL NOT extend to `ElementsBracedExpression`, which continues to require at least one item, so
an element-position `if` or `for` body SHALL still reject `{}`. `EmbedBracedExpression` inside
`@{ ... }` SHALL likewise continue to require content: the list semantics it shares with
`ValuesBracedExpression` apply to one or more items, and `@{}` SHALL remain a parse error.

A call argument SHALL accept a `ValuesBracedExpression` at every arity, so a function is passed a
sequence the same way a property is bound one. The arity rule applies to an argument unchanged:
`f({})` passes the empty value, `f({a b})` passes a sequence, and `f({a})` passes an exactly-one
value that the parameter's declared `+` or `*` type lifts, exactly as a property binding lifts it.
Every argument position accepts one independently, so a braced argument may appear anywhere in the
list and beside ordinary expressions.

Admitting zero items reaches every rule that references `ValuesBracedExpression`, not only the
positions this change set out to open. A value-position `if` branch, a value-position `for` body,
and a match or condition arm body are all that rule, so `{}` SHALL parse in each of them, where it
was a parse error before. Those positions declare no type of their own, so what the empty value means
in them is stated in the typing requirement rather than left to follow from the parse.

An item of a braced value SHALL continue to reject one. `value_list_item_expression` does not
include the braced rule, so `{{ ... }}` remains a syntax error at every arity, zero included: a
sequence is not an item of a sequence. Admitting the brace as an argument SHALL NOT be read as
admitting it inside another brace, so `f({{a} b})` SHALL be a syntax error as well.

#### Scenario: Empty braced value parses as a values braced expression
- **WHEN** a file contains `let value:string* = {}`
- **THEN** the parser SHALL produce a `values_braced_expression` node with no items
- **AND** it SHALL NOT report a parse error or insert a missing identifier

#### Scenario: Empty braced value parses in property position
- **WHEN** a file contains `let v = <Img fits={} />`
- **THEN** the parser SHALL produce a `values_braced_expression` node with no items for the property
  value

#### Scenario: Empty braced value parses in markup child position
- **WHEN** a file contains `component <N /> = { <List>{}</List> }`
- **THEN** the parser SHALL produce a `values_braced_expression` node with no items as the child
  content

#### Scenario: Empty braced value parses as a function body
- **WHEN** a file contains `let f():string* = {}`
- **THEN** the parser SHALL produce a `values_braced_expression` node with no items as the body
- **AND** type checking SHALL accept it as a `string*`

#### Scenario: An empty function body with no declared return type is reported
- **WHEN** a file contains `let <f /> = { }`
- **THEN** the parser SHALL accept it as an empty `values_braced_expression`
- **AND** type checking SHALL report that the element type cannot be determined, because an
  unannotated binding supplies no expected type

#### Scenario: Singleton braced value parses as a values braced expression
- **WHEN** a file contains `let value = {count}`
- **THEN** the parser SHALL produce a `values_braced_expression` node with one ordered item `count`

#### Scenario: Space-delimited brace list parses in source order
- **WHEN** a file contains `let value = {first second <Badge/>}`
- **THEN** the parser SHALL produce a `values_braced_expression` node with three ordered items in the source order `first`, `second`, `<Badge/>`

#### Scenario: Embedded text braces use the same list semantics
- **WHEN** a file contains `<p:html>Hello @{first second}</p>`
- **THEN** the parser SHALL produce an `embed_braced_expression` with two ordered items `first` and `second`

#### Scenario: An empty element body is still rejected
- **WHEN** a file contains `component <N r:boolean /> = { <div>if r {} else { <B/> }</div> }`
- **THEN** the parser SHALL report a parse error, because an element-position `if` body is an
  `ElementsBracedExpression`

#### Scenario: A braced value parses as a call argument
- **WHEN** a file contains `let n = {count({})}`
- **THEN** the parser SHALL produce a `values_braced_expression` node with no items as the argument
- **AND** it SHALL NOT report a parse error or insert a missing identifier
- **AND** it SHALL parse `count({"a"})` and `count({"a" "b"})` the same way, with one and two items

#### Scenario: Every argument position accepts a braced value
- **WHEN** a file contains `let n = {pick({}, x, {"a" "b"})}`
- **THEN** the parser SHALL produce two `values_braced_expression` arguments
- **AND** it SHALL NOT report a parse error

#### Scenario: A braced value is still not an item of a braced value
- **WHEN** a file contains `let v = <Box items={{"a" "b"}} />`
- **THEN** the parser SHALL report a syntax error
- **AND** it SHALL report the same error for `items={{"a"} "b"}`
- **AND** it SHALL accept `items={{}}` and `{ "a" {} }`, because the empty braced value `{}` is the
  empty value and is admitted as an item, contributing nothing, as an untaken conditional does

#### Scenario: A braced argument does not admit braced items
- **WHEN** a file contains `let n = {count({{"a"} "b"})}`
- **THEN** the parser SHALL report a syntax error, because the argument rule admits one brace and
  not a brace whose items are braces

#### Scenario: The empty form parses in a value-position control-flow body
- **WHEN** a file contains `let pick(c:boolean): string* = {if c {"a" "b"} else {}}`
- **THEN** the parser SHALL accept it with no recovery node
- **AND** it SHALL accept `{if { c => {} else => {"a" "b"} }}` and `{for y in ys {}}` the same way,
  each of which was a parse error before the empty form was admitted

#### Scenario: An empty interpolation is still rejected
- **WHEN** a file contains `component <N /> = { <p:html>Hi @{}</p> }`
- **THEN** the parser SHALL report a parse error

### Requirement: Non-list-safe expressions must be parenthesized inside braced lists
A braced value sequence SHALL accept bare list items only from the `ValueListItemExpression`
subset. Expressions outside that subset, including binary and prefix-unary expressions, MUST be
parenthesized before they can appear as list items.

#### Scenario: Parenthesized binary expression is accepted as a list item
- **WHEN** a file contains `let value = {(a + b) c}`
- **THEN** the parser SHALL accept the braced list and treat `(a + b)` and `c` as separate ordered items

#### Scenario: Bare binary expression in a list is rejected
- **WHEN** a file contains `let value = {a + b c}`
- **THEN** the parser SHALL report a parse error because `a + b` is not parenthesized as a list item

#### Scenario: Singleton binary expression remains legal
- **WHEN** a file contains `let value = {a - b}`
- **THEN** the parser SHALL accept the braced expression as a single `ValueExpression`

### Requirement: Values braced expressions infer scalar or flat list types from source arity
A `ValuesBracedExpression` with one source item SHALL infer to that item's type. A
`ValuesBracedExpression` with more than one source item SHALL infer to a sequence whose item type is
the most specific common item type of its items and whose occurrence is the sum of the items'
occurrences, as `occurrence-types` defines for a collecting position. The item type an item
contributes is its own item type, so a sequence-typed item splices: `{xs ys}` with `xs:string+` and
`ys:string+` is a `string+`, and `{xs "a"}` is a `string+`, never a sequence of sequences. If no more
specific common item type exists, the inferred item type SHALL be `object`. An item of type `object`
contributes `object`, whatever it holds at runtime.

A `ValuesBracedExpression` with no source items SHALL be the empty value. This is stated rather than
derived: because one item infers that item's type, zero is the one arity that has no item to take a
type from, and it does not follow from the rule for one or for many.

The type of an empty `ValuesBracedExpression` SHALL be the empty type, rendered `{}`, as
`occurrence-types` defines it: the bottom item type with the zero-or-one occurrence. This is its type
outright and not a placeholder awaiting a site: the system SHALL NOT require an expected type in
order to type it, and SHALL NOT fall back to `object*`. The `object` fallback above applies only to
the heterogeneous multi-item case, where an item type exists to be joined.

Because the empty type satisfies every `?` and `*` type, and joining it with any type contributes its
zero-or-one occurrence and no item type, those two rules SHALL be what carries the empty value to
every site that accepts one — an optional property, a `*` parameter or field, a function body, body
content beside siblings, and a branch or arm body joined with its alternatives — rather than each
site resolving the item type for itself. An empty value among spliced body content SHALL therefore
contribute nothing to the item type its siblings determine, and SHALL leave their occurrence sum
undisturbed.

A binding whose type is fixed by an empty value SHALL nonetheless be reported when it carries no
annotation, naming the binding. This SHALL apply to a value binding and to a function's inferred
return type alike. The reason SHALL be legibility rather than inability: `{}` is a type the binding
could take, and the requirement is that a signature say what the value, when present, is.

An empty `ValuesBracedExpression` as element body content SHALL bind the empty value to the content
property at runtime, not the absence of content. An element with no body at all is the different
case, and SHALL leave the content property to its declared default, as `optional-properties`
requires.

The distinction SHALL be drawn on whether a body was written, not on whether it was written as
`{}`. Body content that ran and produced no values SHALL bind the empty value on the same grounds —
a `for` that iterates zero times has said that there are no children, which is not the same as
saying nothing. This SHALL hold at a content property whose declared default is non-empty: the
default answers an absent body, and a body that produced nothing is not one.

A `for` in a value position SHALL yield a sequence whose item type is what its body contributes
under the same rule and whose occurrence is the product of the iterated occurrence and the body's,
as `occurrence-types` defines. A `for` whose body is the empty value SHALL therefore yield `never*`,
which satisfies every `*` type, and SHALL NOT yield a sequence of sequences.

#### Scenario: Singleton braced value keeps a scalar type
- **WHEN** type inference analyzes `let value = {1}`
- **THEN** `value` SHALL infer as `int` rather than `int+`

#### Scenario: Multi-item braced value infers a list type
- **WHEN** type inference analyzes `let value = {1 2 3}`
- **THEN** `value` SHALL infer as `int+`

#### Scenario: A list-typed item contributes its element type
- **WHEN** type inference analyzes `let xs:string+ = {"a" "b"}` and `let value = {xs "c"}`
- **THEN** `value` SHALL infer as `string+`
- **AND** `let both = {xs xs}` SHALL infer as `string+` as well

#### Scenario: Heterogeneous element list falls back to object
- **WHEN** type inference analyzes `let value = {<A/> <B/>}`
- **THEN** `value` SHALL infer as `object+`

#### Scenario: Empty braced value takes its element type from an annotation
- **WHEN** type inference analyzes `let value:string* = {}`
- **THEN** `value` SHALL have the type `string*`
- **AND** type checking SHALL report no diagnostics

#### Scenario: Empty braced value takes its element type from a property site
- **WHEN** a file contains `external component <Img fits?:Fit+ />` and binds `<Img fits={} />`
- **THEN** type checking SHALL accept the binding
- **AND** the value of `fits` SHALL be the empty value, read as `Fit*`

#### Scenario: Empty braced value at a nullable list site is a non-null empty list
- **WHEN** a file contains `type Brand = { links?:ChatBrandLink+ }` and constructs
  `<Brand links={} />`
- **THEN** type checking SHALL accept the binding
- **AND** the field SHALL hold the empty value, equal to the field of `<Brand />`

#### Scenario: Empty braced value takes its element type from a parameter
- **WHEN** a file contains `let echo(xs?:string+): string* = {xs}` and `let value = {echo({})}`
- **THEN** type checking SHALL report no diagnostics
- **AND** `value` SHALL infer as `string*`

#### Scenario: A one-item braced argument coerces to the parameter's list type
- **WHEN** a file contains `let echo(xs:string+): string+ = {xs}` and
  `let value = {echo({"only"})}`
- **THEN** type checking SHALL accept the call
- **AND** the parameter SHALL receive a one-element sequence, by the same lift a property binding
  uses, rather than by the brace being a sequence at arity one

#### Scenario: Empty braced value at a non-list parameter reports only the mismatch
- **WHEN** a file contains `let f(s:string): int = 1` and `let value = {f({})}`
- **THEN** type checking SHALL report exactly one diagnostic, naming the parameter's type `string`
  and the type found `{}`
- **AND** it SHALL NOT also direct the author to annotate a binding

#### Scenario: A call that cannot be checked reports only its own diagnostic
- **WHEN** type checking analyzes a call with an empty braced argument whose callee is undefined, or
  whose argument count does not match the declaration
- **THEN** the system SHALL report exactly one diagnostic, the call's own
- **AND** it SHALL NOT also report that the empty value's type cannot be determined, because the
  call is the thing to fix and the argument has a site again once it is

#### Scenario: A braced value beside sibling body content takes the content property's type
- **WHEN** a file contains `external component <List content items:Badge+ />` and binds
  `<List>{}<Badge/></List>`
- **THEN** type checking SHALL report no diagnostics
- **AND** the empty value SHALL contribute no items, so the sibling decides the item type rather
  than the join falling to `object`, and the collected content SHALL be `Badge+`

#### Scenario: Empty body content evaluates to an empty list
- **WHEN** `type Box = { content items?:string+ }` is constructed as `<Box>{}</Box>` and as `<Box />`
- **THEN** evaluation SHALL bind the empty value to `items` in both
- **AND** at `type Def = { content items:string+ = {"d"} }`, `<Def>{}</Def>` SHALL be rejected naming
  `{}` and `string+`, while `<Def />`, which has no body at all, SHALL bind the declared default `"d"`

#### Scenario: Body content that produced no values binds the empty list
- **WHEN** `type Box = { content items?:object+ }` is constructed with a body of
  `for x in xs { <A n=2 /> }` and `xs` holds no elements
- **THEN** evaluation SHALL bind the empty value to `items`
- **AND** at `type Def = { content items:object+ = {<A n=9 />} }` the same body SHALL be rejected
  statically, since a `for` over a `*` yields a `*`, so no written body ever falls back to a default
- **AND** this SHALL hold for source containing no `{}`, since the distinction is drawn on whether
  a body was written rather than on how it was spelled

#### Scenario: A function whose return type an empty list fixed is reported
- **WHEN** type checking analyzes `let f(c:boolean) = {if { c => {} else => {} }}`, whose every
  alternative is the empty value and whose binding declares no return type
- **THEN** the system SHALL report exactly one diagnostic, naming `f` and asking for the return type
- **AND** the same SHALL hold for `let f(ys:string+) = {for y in ys {}}`
- **AND** annotating the return type with any `?` or `*` type SHALL make each accepted, since the
  empty value satisfies whatever such type is declared

#### Scenario: Empty braced value at a site that admits a list without being one
- **WHEN** a file contains `type Box = { thing:object }` and binds `<Box thing={} />`
- **THEN** type checking SHALL reject the binding, naming `{}` and `object`, because the empty type
  does not satisfy an exactly-one type
- **AND** `type Maybe = { thing?:object }` with `<Maybe thing={} />` SHALL be accepted

#### Scenario: An empty arm takes its element type from the arm it is joined with
- **WHEN** type checking analyzes `let pick(c:boolean): string* = {if { c => {} else => {"a" "b"} }}`
- **THEN** it SHALL report no diagnostics, the join of `{}` with `string+` being `string*`
- **AND** the empty arm SHALL evaluate to the empty value
- **AND** the same SHALL hold for the branch form, `{if c {"a" "b"} else {}}`

#### Scenario: An empty `for` body yields the empty list
- **WHEN** type checking analyzes `let ys:string+ = {"q"}` and `let xs:string* = {for y in ys {}}`
- **THEN** it SHALL report no diagnostics, since `never*` satisfies `string*`
- **AND** evaluation SHALL bind the empty value to `xs`
- **AND** `let a = {for y in ys {}}` SHALL still report that the type of the value bound to `a`
  cannot be determined

#### Scenario: Empty braced value with no expected type is reported
- **WHEN** type inference analyzes `let value = {}`
- **THEN** the system SHALL report a diagnostic that the type cannot be determined
- **AND** the diagnostic SHALL direct the author to annotate the binding
- **AND** `value` SHALL NOT infer as `object*`

### Requirement: Multi-item braced value lists use external component inheritance for common item types

When a `ValuesBracedExpression` has more than one item and type inference computes the most specific
common item type between two named types that both resolve to external component contracts, the
system SHALL consider the declared external component `extends` ancestry when determining whether
one named type subsumes the other or when computing their shared named supertype. This SHALL apply
in addition to the existing record inheritance rules used for record-named types.

#### Scenario: Sibling external components unify to shared abstract base for annotated list

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question /> external component <LongTextQuestion extends Question /> let questions: Question+ = { <ShortTextQuestion label={"Name"} /> <LongTextQuestion label={"Details"} /> }`
- **THEN** type checking SHALL report no errors for `questions`
- **AND** analysis SHALL treat the braced list elements as compatible with `Question`
