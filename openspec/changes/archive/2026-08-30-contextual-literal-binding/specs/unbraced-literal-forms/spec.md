## Purpose
Defines what may be written as a value without braces — the invariant that an unbraced value is
always a literal, the signed numeric literal form, and the bare contextual name that resolves
against the declared type of the site it binds to — and requires first-party NX-syntax value output
to emit those forms so that it round-trips.

## ADDED Requirements

### Requirement: A bare name is a contextual literal resolved against the expected type
An unquoted single identifier SHALL be accepted as a property value, and SHALL resolve against the
declared type of the binding site rather than against lexical scope. The system SHALL resolve it
only against a closed nominal set: the members of an enum type, and the payloadless cases of a
discriminated union type. Resolution SHALL NOT consult variables, parameters, imports, or any other
lexical binding, and a bare name SHALL NOT be affected by whether an identically named binding is
in scope.

#### Scenario: Bare name resolves to an enum member at an enum-typed property
- **WHEN** a file contains `enum Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit=cover />`
- **THEN** type checking SHALL accept `cover` as a value of type `Fit`
- **AND** interpretation SHALL produce the same value as `fit={Fit.cover}`

#### Scenario: Bare name resolves to a payloadless union case
- **WHEN** a file contains `type LoadState = | idle | loading component <View state:LoadState /> = { <div /> } let v = <View state=idle />`
- **THEN** type checking SHALL accept `idle` as a value of type `LoadState`
- **AND** interpretation SHALL produce a case value with discriminator `LoadState.idle`

#### Scenario: A lexical binding of the same name does not shadow the member
- **WHEN** a file contains `enum Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let cover = "something else" let v = <Img fit=cover />`
- **THEN** type checking SHALL resolve `cover` to the enum member `Fit.cover`
- **AND** it SHALL NOT resolve `cover` to the `let` binding

#### Scenario: Nullable expected type accepts a bare name
- **WHEN** a file contains `enum Fit = fill | contain | cover component <Img fit:Fit? /> = { <img /> } let v = <Img fit=cover />`
- **THEN** type checking SHALL accept `cover` by resolving against the underlying enum type `Fit`

#### Scenario: List-typed site accepts a bare name through scalar-to-list coercion
- **WHEN** a file contains `enum Fit = fill | contain | cover component <Img fits:Fit[] /> = { <img /> } let v = <Img fits=cover />`
- **THEN** type checking SHALL resolve `cover` against the list's element type `Fit`
- **AND** the existing scalar-to-list coercion at typed binding sites SHALL apply to the resolved
  value

### Requirement: An unbraced value is a literal and never an expression
An unbraced value SHALL be a literal, a signed numeric literal, or a contextual literal, and SHALL
NOT be any other expression form. A contextual literal SHALL be exactly one identifier: the system
SHALL NOT accept a dotted or otherwise qualified name in unbraced value position. Naming a member
inside an expression SHALL continue to require the qualified form within braces. Any future
extension of the unbraced value forms SHALL preserve this invariant, so that an unbraced value can
be recognized without consulting lexical scope.

#### Scenario: Qualified name in unbraced position is rejected
- **WHEN** a file contains `enum Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit=Fit.cover />`
- **THEN** parsing or validation SHALL reject the property value
- **AND** the diagnostic SHALL direct the author to `fit=cover` or `fit={Fit.cover}`

#### Scenario: Field access in unbraced position is rejected
- **WHEN** a file contains `type Opts = { fit:string } component <Img fit:string /> = { <img /> } let o:Opts = <Opts fit="cover" /> let v = <Img fit=o.fit />`
- **THEN** parsing or validation SHALL reject the property value

#### Scenario: Qualified member access inside braces remains accepted
- **WHEN** a file contains `enum Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let wide = true let v = <Img fit={if wide { Fit.cover } else { Fit.contain }} />`
- **THEN** type checking SHALL accept the braced conditional
- **AND** a bare `cover` inside that braced expression SHALL NOT resolve as a contextual literal

### Requirement: Contextual literals are accepted at property defaults and match patterns
The system SHALL accept a contextual literal in every unbraced position where the expected type is
already declared: element and component property values, property defaults in record, component,
state, and emitted action declarations, and match patterns whose scrutinee type is known. In match
pattern position a bare name SHALL resolve as a member or case of the scrutinee's type in preference
to any lexically visible binding of that name, and the system SHALL report a diagnostic whenever
that preference displaces a visible binding.

#### Scenario: Property default accepts a bare name
- **WHEN** a file contains `enum Fit = fill | contain | cover external component <Img fit:Fit = cover />`
- **THEN** parsing and type checking SHALL accept the declaration
- **AND** the default SHALL be the enum member `Fit.cover`

#### Scenario: Record field default accepts a bare name
- **WHEN** a file contains `enum Fit = fill | contain | cover type Opts = { fit:Fit = contain }`
- **THEN** type checking SHALL accept `contain` as the default for `fit`

#### Scenario: Match pattern accepts a bare name
- **WHEN** a file contains `enum Fit = fill | contain | cover let label(f:Fit) = if f is { cover => "cover" contain => "contain" fill => "fill" }`
- **THEN** type checking SHALL resolve each bare pattern as a member of `Fit`
- **AND** it SHALL accept the match as exhaustive

#### Scenario: Bare pattern from another type is rejected
- **WHEN** a file contains `enum Fit = fill | contain | cover enum Align = start | center let label(f:Fit) = if f is { center => "c" else => "" }`
- **THEN** type checking SHALL reject `center` because it is not a member of `Fit`

#### Scenario: Nominal resolution takes precedence in pattern position
- **WHEN** a match scrutinee's type is an enum or a discriminated union, and a bare pattern name is
  both a member or case of that type and a lexically visible binding
- **THEN** type checking SHALL resolve the pattern as the member or case
- **AND** it SHALL report a diagnostic that the lexical binding of that name is not used as a
  pattern, so the change in meaning is never silent

### Requirement: Bare and quoted values resolve in disjoint tiers
In NX source a bare name SHALL resolve only against the closed nominal set, and a quoted string
SHALL resolve only as string data. The system SHALL NOT fall back from one tier to the other: a
bare name that is not a member or case SHALL NOT be reinterpreted as a string, and a quoted string
SHALL NOT be reinterpreted as a member or case. Where a type admits both nominal cases and string
data, the two SHALL therefore remain separately spellable, and adding a case to that type SHALL NOT
change the meaning of any existing quoted value.

#### Scenario: Quoted string at an enum-typed property is rejected
- **WHEN** a file contains `enum Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit="cover" />`
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
and SHALL list or suggest the members or cases of that type when it has any. When the expected type
is not an enum or a discriminated union, the diagnostic SHALL say so and SHALL indicate the form
the site does accept.

#### Scenario: Unknown member suggests a near match
- **WHEN** a file contains `enum Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit=containt />`
- **THEN** type checking SHALL reject `containt`
- **AND** the diagnostic SHALL report that `containt` is not a member of `Fit` and SHALL suggest
  `contain`

#### Scenario: Payload union case cannot be used as a bare name
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } component <View state:LoadState /> = { <div /> } let v = <View state=failed />`
- **THEN** type checking SHALL reject `failed` because that case requires payload construction
- **AND** the diagnostic SHALL direct the author to the element-style form `<LoadState.failed ... />`

### Requirement: A resolved contextual literal is indistinguishable from the qualified form
Once resolved, a contextual literal SHALL produce the same runtime value, the same serialized
payload, and the same generated code as the qualified form of the same member or case. The source
spelling SHALL NOT be observable downstream of type checking.

#### Scenario: Bare and qualified forms produce identical output
- **WHEN** two files differ only in that one writes `fit=cover` where the other writes `fit={Fit.cover}`
- **THEN** interpretation SHALL produce equal runtime values
- **AND** canonical raw payload conversion SHALL produce the identical bare authored member string
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
literals, booleans as boolean literals, null as the null literal, and enum members and payloadless
union cases as bare contextual names. A value of a float type SHALL be emitted with a real-literal
spelling, so that it binds at a float-typed site rather than as an integer literal.

The round-trip guarantee currently extends to values whose fields are all scalars. A record-valued
or list-valued field is not yet rendered in a form that reads back, and an empty record whose type
name is qualified is rendered as a bare name whether or not it is a union case. Rendering every
value so that it reads back as itself is specified separately.

#### Scenario: Scalar property values are emitted unquoted
- **WHEN** first-party formatting renders a record whose fields hold a float, a boolean, a null, and
  an enum member
- **THEN** the output SHALL be of the form `<Box w=1.5 flag=true opt=null fit=cover />`
- **AND** it SHALL NOT quote any of those values

#### Scenario: Formatted output re-parses and type checks
- **WHEN** first-party formatting renders a value whose fields are all scalars, and that source is
  parsed and type checked against the originating types
- **THEN** type checking SHALL report no diagnostics

#### Scenario: Negative float value is emitted as an unbraced real literal
- **WHEN** first-party formatting renders a `float64` field holding `-1.0`
- **THEN** the output SHALL be `neg=-1.0`
- **AND** it SHALL NOT be `neg="-1"` or `neg=-1`
