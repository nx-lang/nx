## ADDED Requirements

### Requirement: Values braced expressions infer scalar or flat list types from source arity
A `ValuesBracedExpression` with one source item SHALL infer to that item's type. A
`ValuesBracedExpression` with more than one source item SHALL infer to a list of the most specific
common item type, where the item type an item contributes is the element type of its own type when
that type is a list, and its type otherwise. A list-typed item therefore splices: `{xs ys}` with
`xs:string[]` and `ys:string[]` is a `string[]`, and `{xs "a"}` is a `string[]`, never a list of
lists. If no more specific common type exists, the inferred list type SHALL be `object[]`. An item
of type `object` contributes `object`, whatever it holds at runtime.

A `ValuesBracedExpression` with no source items SHALL be list-valued. This is stated rather than
derived: because one item infers a scalar, zero is the one arity that cannot be scalar, and it does
not follow from the rule for one or for many.

The element type of an empty `ValuesBracedExpression` SHALL be the bottom type, so its type is
`never[]`. This is its type outright and not a placeholder awaiting a site: the system SHALL NOT
require an expected type in order to type it, and SHALL NOT fall back to `object[]`. The `object[]`
fallback above applies only to the heterogeneous multi-item case, where an item type exists to be
joined.

Because the bottom type is below every type, `never[]` SHALL satisfy every list type, and joining
`never` with any type SHALL yield that type. Those two rules SHALL be what carries an empty list to
every site that accepts one — a property, a parameter, a field default, a function body, body
content beside siblings, and a branch or arm body joined with its alternatives — rather than each
site resolving the element type for itself. An empty list among spliced body content SHALL therefore
contribute nothing to the item type its siblings determine.

A binding whose type is fixed by an empty list SHALL nonetheless be reported when it carries no
annotation, naming the binding. This SHALL apply to a value binding and to a function's inferred
return type alike. The reason SHALL be legibility rather than inability: `never[]` is a type the
binding could take, and the requirement is that a signature say what the list is a list of.

An empty `ValuesBracedExpression` as element body content SHALL bind the empty list to the content
property at runtime, not the absence of content. An element with no body at all is the different
case, and SHALL leave the content property to its declared default.

The distinction SHALL be drawn on whether a body was written, not on whether it was written as
`{}`. Body content that ran and produced no values SHALL bind the empty list on the same grounds —
a `for` that iterates zero times has said that there are no children, which is not the same as
saying nothing. This SHALL hold at a content property whose declared default is non-empty: the
default answers an absent body, and a body that produced nothing is not one.

A `for` in a value position SHALL yield a list whose item type is what its body contributes under
the same rule: the element type of a list-typed body, and the body's type otherwise. A `for` whose
body is the empty list SHALL therefore yield `never[]`, which satisfies every list type, and SHALL
NOT yield a list of lists.

#### Scenario: Singleton braced value keeps a scalar type
- **WHEN** type inference analyzes `let value = {1}`
- **THEN** `value` SHALL infer as `int` rather than `int[]`

#### Scenario: Multi-item braced value infers a list type
- **WHEN** type inference analyzes `let value = {1 2 3}`
- **THEN** `value` SHALL infer as `int[]`

#### Scenario: A list-typed item contributes its element type
- **WHEN** type inference analyzes `let xs:string[] = {"a" "b"}` and `let value = {xs "c"}`
- **THEN** `value` SHALL infer as `string[]`
- **AND** `let both = {xs xs}` SHALL infer as `string[]` as well

#### Scenario: Heterogeneous element list falls back to object
- **WHEN** type inference analyzes `let value = {<A/> <B/>}`
- **THEN** `value` SHALL infer as `object[]`

#### Scenario: Empty braced value takes its element type from an annotation
- **WHEN** type inference analyzes `let value:string[] = {}`
- **THEN** `value` SHALL infer as `string[]`
- **AND** type checking SHALL report no diagnostics

#### Scenario: Empty braced value takes its element type from a property site
- **WHEN** a file contains `external component <Img fits:Fit[] />` and binds `<Img fits={} />`
- **THEN** type checking SHALL accept the binding
- **AND** the value SHALL be a `Fit[]` with no elements

#### Scenario: Empty braced value at a nullable list site is a non-null empty list
- **WHEN** a file contains `type Brand = { links:ChatBrandLink[]? }` and constructs
  `<Brand links={} />`
- **THEN** type checking SHALL accept the binding
- **AND** the field SHALL hold a non-null `ChatBrandLink[]` with no elements rather than null

#### Scenario: Empty braced value takes its element type from a parameter
- **WHEN** a file contains `let echo(xs:string[]): string[] = {xs}` and `let value = {echo({})}`
- **THEN** type checking SHALL report no diagnostics
- **AND** `value` SHALL infer as `string[]`

#### Scenario: A one-item braced argument coerces to the parameter's list type
- **WHEN** a file contains `let echo(xs:string[]): string[] = {xs}` and
  `let value = {echo({"only"})}`
- **THEN** type checking SHALL accept the call
- **AND** the parameter SHALL receive a one-element list, by the same coercion a property binding
  uses, rather than by the brace being a list at arity one

#### Scenario: Empty braced value at a non-list parameter reports only the mismatch
- **WHEN** a file contains `let f(s:string): int = 1` and `let value = {f({})}`
- **THEN** type checking SHALL report exactly one diagnostic, naming the parameter's type
- **AND** it SHALL NOT also direct the author to annotate a binding

#### Scenario: A call that cannot be checked reports only its own diagnostic
- **WHEN** type checking analyzes a call with an empty braced argument whose callee is undefined, or
  whose argument count does not match the declaration
- **THEN** the system SHALL report exactly one diagnostic, the call's own
- **AND** it SHALL NOT also report that the empty list's element type cannot be determined, because
  the call is the thing to fix and the argument has a site again once it is

#### Scenario: A braced value beside sibling body content takes the content property's type
- **WHEN** a file contains `external component <List content items:Badge[] />` and binds
  `<List>{}<Badge/></List>`
- **THEN** type checking SHALL report no diagnostics
- **AND** the empty list SHALL contribute no items, so the sibling decides the item type rather
  than the join falling to `object`

#### Scenario: Empty body content evaluates to an empty list
- **WHEN** `type Box = { content items: string[] }` is constructed as `<Box>{}</Box>`
- **THEN** evaluation SHALL bind a `string[]` with no elements to `items`
- **AND** it SHALL do so at a `string[]?` content field as well, binding the empty list rather than
  null
- **AND** `<Box />`, which has no body at all, SHALL leave `items` to its declared default

#### Scenario: Body content that produced no values binds the empty list
- **WHEN** `type Box = { content items: object[] = {<A n=9 />} }` is constructed with a body of
  `for x in xs { <A n=2 /> }` and `xs` holds no elements
- **THEN** evaluation SHALL bind an empty list to `items`
- **AND** it SHALL NOT fall back to the declared default, because a body that ran and produced
  nothing is not an absent body
- **AND** this SHALL hold for source containing no `{}`, since the distinction is drawn on whether
  a body was written rather than on how it was spelled

#### Scenario: A function whose return type an empty list fixed is reported
- **WHEN** type checking analyzes `let f(c:boolean) = {if { c => {} else => {} }}`, whose every
  alternative is an empty list and whose binding declares no return type
- **THEN** the system SHALL report exactly one diagnostic, naming `f` and asking for the return type
- **AND** the same SHALL hold for `let f(ys:string[]) = {for y in ys {}}`
- **AND** annotating the return type SHALL make each accepted, since the empty list satisfies
  whatever list type is declared

#### Scenario: Empty braced value at a site that admits a list without being one
- **WHEN** a file contains `type Box = { thing:object }` and binds `<Box thing={} />`
- **THEN** type checking SHALL accept the binding, because `never[]` is a list and every list
  satisfies `object`
- **AND** it SHALL accept `<Box thing={"a" "b"} />` too, so the empty form is writable wherever the
  non-empty form is

#### Scenario: An empty arm takes its element type from the arm it is joined with
- **WHEN** type checking analyzes `let pick(c:boolean): string[] = {if { c => {} else => {"a" "b"} }}`
- **THEN** it SHALL report no diagnostics
- **AND** the empty arm SHALL evaluate to a `string[]` with no elements
- **AND** the same SHALL hold for the branch form, `{if c {"a" "b"} else {}}`

#### Scenario: An empty `for` body yields the empty list
- **WHEN** type checking analyzes `let ys:string[] = {"q"}` and `let xs:string[] = {for y in ys {}}`
- **THEN** it SHALL report no diagnostics, since `never[]` satisfies `string[]`
- **AND** evaluation SHALL bind an empty `string[]` to `xs`
- **AND** `let a = {for y in ys {}}` SHALL still report that the element type of the list bound to
  `a` cannot be determined

#### Scenario: Empty braced value with no expected type is reported
- **WHEN** type inference analyzes `let value = {}`
- **THEN** the system SHALL report a diagnostic that the element type cannot be determined
- **AND** the diagnostic SHALL direct the author to annotate the binding
- **AND** `value` SHALL NOT infer as `object[]`

## REMOVED Requirements

### Requirement: Values braced expressions infer scalar or list types from source arity
**Reason**: A list-typed item of a multi-item braced value, and a list-typed `for` body, now contribute their items rather than nesting, so the arity rule and its `for` scenario are restated.
**Migration**: No source change is needed where the nesting was never named. Where a `string[][]` annotation named it, the site is now `string[]` and holds the concatenated items.
