# unbraced-literal-forms Specification

## Purpose
Defines what may be written as a value without braces — the invariant that an unbraced value is
always a literal, the signed numeric literal form, and the bare contextual name that resolves
against the declared type of the site it binds to — and requires first-party NX-syntax value output
to emit those forms so that it round-trips.

## Requirements
### Requirement: A bare name is a contextual literal resolved against the expected type
An unquoted single identifier SHALL be accepted as a property value, and SHALL resolve against the
declared type of the binding site rather than against lexical scope. The system SHALL resolve it
only against a closed nominal set: the constant cases of a discriminated union type. Resolution
SHALL NOT consult variables, parameters, imports, or any other lexical binding, and a bare name
SHALL NOT be affected by whether an identically named binding is in scope.

#### Scenario: Bare name resolves to an enum member at an enum-typed property
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit=cover />`
- **THEN** type checking SHALL accept `cover` as a value of type `Fit`
- **AND** interpretation SHALL produce the same value as `fit={Fit.cover}`

#### Scenario: Bare name resolves to a payloadless union case
- **WHEN** a file contains `type LoadState = idle | failed { message:string } component <View state:LoadState /> = { <div /> } let v = <View state=idle />`
- **THEN** type checking SHALL accept `idle` as a value of type `LoadState`
- **AND** interpretation SHALL produce the constant case value of `LoadState.idle`

#### Scenario: A lexical binding of the same name does not shadow the member
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let cover = "something else" let v = <Img fit=cover />`
- **THEN** type checking SHALL resolve `cover` to the constant case `Fit.cover`
- **AND** it SHALL NOT resolve `cover` to the `let` binding

#### Scenario: Nullable expected type accepts a bare name
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit? /> = { <img /> } let v = <Img fit=cover />`
- **THEN** type checking SHALL accept `cover` by resolving against the underlying union type `Fit`

#### Scenario: List-typed site accepts a bare name through scalar-to-list coercion
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fits:Fit[] /> = { <img /> } let v = <Img fits=cover />`
- **THEN** type checking SHALL resolve `cover` against the list's element type `Fit`
- **AND** the existing scalar-to-list coercion at typed binding sites SHALL apply to the resolved
  value

### Requirement: An unbraced value is a literal and never an expression
An unbraced value SHALL be a literal, a signed numeric literal, or a contextual literal, and SHALL
NOT be any other expression form. A contextual literal SHALL be exactly one identifier: the system
SHALL NOT accept a dotted or otherwise qualified name in unbraced value position. Naming a case
inside an expression SHALL continue to require the qualified form within braces. Any future
extension of the unbraced value forms SHALL preserve this invariant, so that an unbraced value can
be recognized without consulting lexical scope.

#### Scenario: Qualified name in unbraced position is rejected
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit=Fit.cover />`
- **THEN** parsing or validation SHALL reject the property value
- **AND** the diagnostic SHALL direct the author to `fit=cover` or `fit={Fit.cover}`

#### Scenario: Field access in unbraced position is rejected
- **WHEN** a file contains `type Opts = { fit:string } component <Img fit:string /> = { <img /> } let o:Opts = <Opts fit="cover" /> let v = <Img fit=o.fit />`
- **THEN** parsing or validation SHALL reject the property value

#### Scenario: Qualified member access inside braces remains accepted
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let wide = true let v = <Img fit={if wide { Fit.cover } else { Fit.contain }} />`
- **THEN** type checking SHALL accept the braced conditional
- **AND** a bare `cover` inside that braced expression SHALL NOT resolve as a contextual literal

### Requirement: Contextual literals are accepted at property defaults and match patterns
The system SHALL accept a contextual literal in every unbraced position where the expected type is
already declared: element and component property values, property defaults in record, component,
state, and emitted action declarations, and match patterns whose scrutinee type is known. In match
pattern position a bare name SHALL resolve as a case of the scrutinee's type in preference to any
lexically visible binding of that name, and the system SHALL report a diagnostic whenever that
preference displaces a visible binding.

#### Scenario: Property default accepts a bare name
- **WHEN** a file contains `type Fit = fill | contain | cover external component <Img fit:Fit = cover />`
- **THEN** parsing and type checking SHALL accept the declaration
- **AND** the default SHALL be the constant case `Fit.cover`

#### Scenario: Record field default accepts a bare name
- **WHEN** a file contains `type Fit = fill | contain | cover type Opts = { fit:Fit = contain }`
- **THEN** type checking SHALL accept `contain` as the default for `fit`

#### Scenario: Match pattern accepts a bare name
- **WHEN** a file contains `type Fit = fill | contain | cover let label(f:Fit) = if f is { cover => "cover" contain => "contain" fill => "fill" }`
- **THEN** type checking SHALL resolve each bare pattern as a case of `Fit`
- **AND** it SHALL accept the match as exhaustive

#### Scenario: Bare pattern from another type is rejected
- **WHEN** a file contains `type Fit = fill | contain | cover type Align = start | center let label(f:Fit) = if f is { center => "c" else => "" }`
- **THEN** type checking SHALL reject `center` because it is not a case of `Fit`

#### Scenario: Nominal resolution takes precedence in pattern position
- **WHEN** a match scrutinee's type is a discriminated union, and a bare pattern name is both a case
  of that type and a lexically visible binding
- **THEN** type checking SHALL resolve the pattern as the case
- **AND** it SHALL report a diagnostic that the lexical binding of that name is not used as a
  pattern, so the change in meaning is never silent

### Requirement: Bare and quoted values resolve in disjoint tiers
In NX source a bare name SHALL resolve only against the closed nominal set, and a quoted string
SHALL resolve only as string data. The system SHALL NOT fall back from one tier to the other: a
bare name that is not a case SHALL NOT be reinterpreted as a string, and a quoted string SHALL NOT
be reinterpreted as a case. Where a type admits both nominal cases and string data, the two SHALL
therefore remain separately spellable, and adding a case to that type SHALL NOT change the meaning
of any existing quoted value.

#### Scenario: Quoted string at an enum-typed property is rejected
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit="cover" />`
- **THEN** type checking SHALL reject the property value as a type mismatch between `string` and
  `Fit`
- **AND** the diagnostic SHALL direct the author to the bare form `fit=cover`

#### Scenario: Bare name at a string-typed property is rejected
- **WHEN** a file contains `component <Img alt:string /> = { <img /> } let v = <Img alt=cover />`
- **THEN** type checking SHALL reject `cover` because `string` is not a nominal type
- **AND** the diagnostic SHALL direct the author to the quoted form `alt="cover"`

### Requirement: An unresolvable bare name is a diagnostic that names the candidates
When a bare name does not resolve, the system SHALL report an error rather than silently accepting
the name as a string or as an unresolved identifier. The diagnostic SHALL name the expected type,
and SHALL list or suggest the constant cases of that type when it has any. When the expected type is
not a discriminated union, the diagnostic SHALL say so and SHALL indicate the form the site does
accept.

#### Scenario: Unknown member suggests a near match
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit=containt />`
- **THEN** type checking SHALL reject `containt`
- **AND** the diagnostic SHALL report that `containt` is not a case of `Fit` and SHALL suggest
  `contain`

#### Scenario: Payload union case cannot be used as a bare name
- **WHEN** a file contains `type LoadState = idle | failed { message:string } component <View state:LoadState /> = { <div /> } let v = <View state=failed />`
- **THEN** type checking SHALL reject `failed` because that case requires payload construction
- **AND** the diagnostic SHALL direct the author to the element-style form `<LoadState.failed ... />`

### Requirement: A resolved contextual literal is indistinguishable from the qualified form
Once resolved, a contextual literal SHALL produce the same runtime value, the same serialized
payload, and the same generated code as the qualified form of the same case. The source spelling
SHALL NOT be observable downstream of type checking.

#### Scenario: Bare and qualified forms produce identical output
- **WHEN** two files differ only in that one writes `fit=cover` where the other writes `fit={Fit.cover}`
- **THEN** interpretation SHALL produce equal runtime values
- **AND** canonical raw payload conversion SHALL produce the identical bare authored case string
- **AND** code generation SHALL produce equivalent output for both

### Requirement: A signed numeric literal is a literal where a literal is required
A numeric literal prefixed by `-` SHALL be accepted in every position that grammatically requires a
literal: unbraced property values, property defaults, record field defaults, and match patterns. The
`-` SHALL apply only to integer, real, and hexadecimal literals; it SHALL NOT apply to string,
boolean, or null literals, and no other prefix operator SHALL be accepted in those positions.

Tokenization SHALL be unchanged: `-` SHALL NOT be absorbed into a numeric literal by the lexer.
Expression positions SHALL be unchanged as well: `-` SHALL remain a prefix operator there, so that
binary subtraction keeps its current meaning and `a-1` continues to parse as subtraction.

#### Scenario: Negative default on a component property
- **WHEN** a file contains `external component <C x: float64 = -1.0 />`
- **THEN** parsing and type checking SHALL accept the declaration
- **AND** the default SHALL be the value `-1.0`

#### Scenario: Negative match pattern
- **WHEN** a file contains `let classify(n: int) = {if n is { -1 => "neg one" 0 => "zero" else => "other" }}`
- **THEN** parsing and type checking SHALL accept the negative pattern
- **AND** `classify(-1)` SHALL evaluate to `"neg one"`

### Requirement: A negated numeric literal has one representation
Prefix negation applied directly to a numeric literal SHALL be folded to a negative literal during
lowering, in every position including expressions. The unbraced form, the braced form, and an
occurrence inside a larger expression SHALL therefore produce the same lowered representation, and
no consumer of lowered output SHALL need to handle a negation node wrapping a numeric literal.
Folding SHALL apply only to a `-` applied directly to a numeric literal; negation of any other
operand SHALL remain a unary operation.

#### Scenario: Negative default on a record field
- **WHEN** a file contains `type Opts = { x: float64 = -1.0 }`
- **THEN** type checking SHALL accept `-1.0` as the default for `x`

#### Scenario: Negative property value needs no braces
- **WHEN** a file contains `component <C x: float64 /> = { <div /> } let v = <C x=-1.5 />`
- **THEN** type checking SHALL accept the property value

#### Scenario: Binary subtraction is unaffected
- **WHEN** a file contains an expression `a-1` or `a - 1` inside braces
- **THEN** it SHALL continue to parse as binary subtraction
- **AND** it SHALL NOT parse as `a` followed by the literal `-1`

#### Scenario: Other prefix operators remain expressions
- **WHEN** a file contains `component <C flag: boolean /> = { <div /> } let v = <C flag=!true />`
- **THEN** parsing or validation SHALL reject the property value
- **AND** the braced form `flag={!true}` SHALL remain accepted

#### Scenario: Braced and unbraced negative defaults lower identically
- **WHEN** one declaration writes `x: float64 = -1.0` and another writes `x: float64 = {-1.0}`
- **THEN** both SHALL lower to the same negative literal
- **AND** neither SHALL lower to a negation applied to a positive literal

#### Scenario: Negation inside a larger expression is folded
- **WHEN** a file contains `let currentRotation = 45 let newRotation = {-90 + currentRotation}`
- **THEN** `-90` SHALL lower to a negative literal rather than a negation of `90`
- **AND** the expression SHALL evaluate to `-45` as it does today

#### Scenario: Negation of a non-literal remains a unary operation
- **WHEN** a file contains `let f(n: int) = {-n}`
- **THEN** lowering SHALL produce a negation applied to `n`
- **AND** it SHALL NOT be folded

### Requirement: First-party NX-syntax value output round-trips
When first-party tooling renders a runtime value as NX source, the output SHALL be re-parseable and
SHALL type check against the same types the value came from. Scalar values in property position
SHALL be emitted in their unbraced literal form rather than wrapped in quotes: numbers as numeric
literals, booleans as boolean literals, null as the null literal, and constant union cases as bare
contextual names. A value of a float type SHALL be emitted with a real-literal spelling, so that it
reads back as a float wherever the site it is read at supplies no expected type — an unannotated
`let`, for instance, where the spelling is all that distinguishes a float from an integer. An
integer literal binds at a float-typed site as well, but rendering relies on the spelling rather
than on the reader's context.

A list value with no elements SHALL be emitted as the empty braced form `{}`. It SHALL be emitted
that way at every list-typed site, including a site whose declared default is itself the empty list,
so that rendering does not depend on reasoning about defaults. It SHALL NOT be emitted as a quoted
string, and it SHALL NOT be omitted.

A value rendered on its own, rather than in property position, SHALL follow the same rule. A list
with elements is a run of values one per line, and a list with none has no lines to be a run of, so
emitting nothing there would read back as no value rather than as the empty list. `{}` SHALL be
emitted instead.

A value that is not a constant case SHALL NOT be rendered as a bare contextual name. In particular a
record SHALL be rendered in element form whether or not it has fields and whether or not its type
name is qualified, so that an empty qualified record reads back as itself.

Every field of a rendered record SHALL be emitted in property position, as the field's name followed
by its value. A field's name SHALL NOT be omitted, and SHALL NOT be emitted as an element tag. A
field value SHALL NOT be emitted as element body content, because a body binds to the target's
declared content property rather than to a named field, and which field a body bound to cannot be
recovered from the rendered source. Whether a value is rendered on its own line SHALL be a layout
decision only, and SHALL NOT change which syntax is used.

When a value has no NX source spelling, first-party tooling SHALL report a failure rather than emit
output that does not read back. A list held directly as an item of another list SHALL be reported on
those grounds: a braced value is not admitted as an item of a braced value, so `{{ ... }}` does not
parse. That SHALL hold whether or not the inner list is empty — giving the empty list a spelling
SHALL NOT be read as giving the nested empty list one.

It SHALL hold on the own-value path as well as in property position. A run of values one per line
cannot say where an inner list ends, so a nested list rendered there would come back as the
flattened run of its elements — a different value, reported by nothing.

#### Scenario: A record-valued property keeps its property name
- **WHEN** first-party formatting renders a record with a field named `home` holding a record of type
  `Address`
- **THEN** the output SHALL bind the value to `home` in property position
- **AND** it SHALL NOT render the value as body content of the enclosing element
- **AND** the output SHALL type check against the originating types

#### Scenario: Two properties of the same record type stay distinguishable
- **WHEN** first-party formatting renders a record with two fields of the same record type holding
  different values
- **THEN** each field's name SHALL appear in the output
- **AND** re-reading the output SHALL bind each value to the field it came from

#### Scenario: A list-valued property keeps its property name
- **WHEN** first-party formatting renders a field named `items` holding a list of records
- **THEN** the output SHALL bind the list to `items` in property position
- **AND** it SHALL NOT emit an element whose tag is the field name

#### Scenario: An empty list is rendered as the empty braced form
- **WHEN** first-party formatting renders a record whose list-typed field holds no elements
- **THEN** the output SHALL bind `{}` to that field in property position
- **AND** it SHALL NOT quote the value, omit the field, or report a failure

#### Scenario: An empty list round-trips
- **WHEN** first-party formatting renders a record whose list-typed field holds no elements, and that
  source is parsed and type checked against the originating types
- **THEN** type checking SHALL report no diagnostics
- **AND** re-evaluating the rendered source SHALL produce a list with no elements

#### Scenario: An empty list at a field with a non-empty default still renders
- **WHEN** first-party formatting renders a record whose list-typed field declares a non-empty
  default and holds no elements
- **THEN** the output SHALL emit `{}` for that field
- **AND** re-evaluating the rendered source SHALL produce a list with no elements rather than the
  declared default

#### Scenario: An empty list nested in a list is reported rather than rendered
- **WHEN** first-party formatting renders a field holding a list whose single item is a list with no
  elements
- **THEN** it SHALL report a failure
- **AND** it SHALL NOT emit `{{}}`, which is a syntax error

#### Scenario: A non-empty list nested in a list is reported the same way
- **WHEN** first-party formatting renders a field holding a list whose single item is a list with one
  element
- **THEN** it SHALL report a failure
- **AND** the reason SHALL be that a list nested in a list has no spelling, not that a list is empty

#### Scenario: A record between two lists still renders
- **WHEN** first-party formatting renders a field holding a list of records, each of which has a
  list-typed field holding no elements
- **THEN** the output SHALL render each record in element form with `{}` bound to its list field
- **AND** it SHALL NOT report a failure, because the records' braces are not nested directly

#### Scenario: An empty list rendered on its own is the braced form
- **WHEN** first-party formatting renders a list value with no elements, not in property position
- **THEN** the output SHALL be `{}`
- **AND** it SHALL NOT be empty output

#### Scenario: A nested list rendered on its own is reported
- **WHEN** first-party formatting renders a list whose items are themselves lists, not in property
  position
- **THEN** it SHALL report a failure
- **AND** it SHALL NOT emit the flattened run of the inner lists' elements

#### Scenario: A value with no source spelling is reported rather than rendered
- **WHEN** first-party formatting encounters a value that has no NX source spelling, such as an
  action handler
- **THEN** it SHALL report a failure
- **AND** it SHALL NOT emit a placeholder or a synthetic element in that position

#### Scenario: Scalar property values are emitted unquoted
- **WHEN** first-party formatting renders a record whose fields hold a float, a boolean, a null, and
  a constant union case
- **THEN** the output SHALL be of the form `<Box w=1.5 flag=true opt=null fit=cover />`
- **AND** it SHALL NOT quote any of those values

#### Scenario: Formatted output re-parses and type checks
- **WHEN** first-party formatting renders a value as NX source and that source is parsed and type
  checked against the originating types
- **THEN** type checking SHALL report no diagnostics

#### Scenario: An empty qualified record is not rendered as a bare name
- **WHEN** first-party formatting renders a property whose value is an empty record whose type name
  contains a dot
- **THEN** the output SHALL render it in element form
- **AND** it SHALL NOT emit the last segment of the type name as a bare contextual name

#### Scenario: Negative float value is emitted as an unbraced real literal
- **WHEN** first-party formatting renders a `float64` field holding `-1.0`
- **THEN** the output SHALL be `neg=-1.0`
- **AND** it SHALL NOT be `neg="-1"` or `neg=-1`

#### Scenario: A whole-valued float keeps its real-literal spelling
- **WHEN** first-party formatting renders a `float64` field holding `24.0`
- **THEN** the output SHALL be `24.0`
- **AND** it SHALL NOT be shortened to `24`
