## MODIFIED Requirements

### Requirement: A bare name is a contextual literal resolved against the expected type
An unquoted single identifier SHALL be accepted as a property value, and SHALL resolve against the
declared type of the binding site rather than against lexical scope. The system SHALL resolve it
only against a closed nominal set determined by the site: at a site typed by a discriminated union,
the constant cases of that union; at a type-parameter site of a component, the type names visible
at the use site. Resolution SHALL look through the site's occurrence to its item type: a bare name
at a site that reads `Fit?`, `Fit+` or `Fit*` resolves against `Fit`, and the resolved exactly-one
value then satisfies the site by the lattice in `occurrence-types`. Resolution SHALL NOT consult
variables, parameters, imports of values, or any other lexical value binding, and a bare name SHALL
NOT be affected by whether an identically named value binding is in scope.

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
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit?:Fit /> = { <img /> } let v = <Img fit=cover />`
- **THEN** type checking SHALL accept `cover` by resolving against the item type `Fit` of the site's read type `Fit?`

#### Scenario: List-typed site accepts a bare name through scalar-to-list coercion
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fits:Fit+ /> = { <img /> } let v = <Img fits=cover />`
- **THEN** type checking SHALL resolve `cover` against the sequence's item type `Fit`
- **AND** the resolved exactly-one value SHALL lift to a sequence of one at the `Fit+` site, as
  `occurrence-types` defines

#### Scenario: Bare name resolves to a visible type at a type-parameter site
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type items?:TItem+ /> let Contact = "shadow" let v = <List TItem=Contact />`
- **THEN** type checking SHALL resolve `TItem=Contact` to the record type `Contact`
- **AND** it SHALL NOT consult the `let` binding named `Contact`

### Requirement: A signed numeric literal is a literal where a literal is required
A numeric literal prefixed by `-` SHALL be accepted in every position that grammatically requires a
literal: unbraced property values, property defaults, record field defaults, and match patterns. The
`-` SHALL apply only to integer, real, and hexadecimal literals; it SHALL NOT apply to string or
boolean literals, and no other prefix operator SHALL be accepted in those positions.

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

### Requirement: First-party NX-syntax value output round-trips
When first-party tooling renders a runtime value as NX source, the output SHALL be re-parseable and
SHALL type check against the same types the value came from. Scalar values in property position
SHALL be emitted in their unbraced literal form rather than wrapped in quotes: numbers as numeric
literals, booleans as boolean literals, and constant union cases as bare contextual names. A value
of a float type SHALL be emitted with a real-literal spelling, so that it reads back as a float
wherever the site it is read at supplies no expected type — an unannotated `let`, for instance,
where the spelling is all that distinguishes a float from an integer. An integer literal binds at a
float-typed site as well, but rendering relies on the spelling rather than on the reader's context.

An empty value in property position SHALL be rendered by the field's declaration, following the
canonical encoding in `occurrence-types`. An optional field (`name?:`) that is empty SHALL be
omitted: an omitted optional field reads back as the empty value, so the omission is the value's
spelling. A `*` field that is empty SHALL be emitted as the empty braced form `{}`: such a field
carries a default, and omitting it would read back as the default rather than as empty, so
rendering SHALL NOT depend on reasoning about defaults. An exactly-one or `+` field is never empty.
The empty value SHALL NOT be emitted as a quoted string, and a `*` field SHALL NOT be omitted.

A value rendered on its own, rather than in property position, SHALL be emitted as `{}` when it is
empty. A sequence with items is a run of values one per line, and the empty value has no lines to
be a run of, so emitting nothing there would read back as no value written rather than as the empty
value. `{}` SHALL be emitted instead.

A value that is not a constant case SHALL NOT be rendered as a bare contextual name. In particular a
record SHALL be rendered in element form whether or not it has fields and whether or not its type
name is qualified, so that an empty qualified record reads back as itself.

Every field of a rendered record SHALL be emitted in property position, as the field's name followed
by its value. A field's name SHALL NOT be omitted except for an empty optional field as above, and
SHALL NOT be emitted as an element tag. A field value SHALL NOT be emitted as element body content,
because a body binds to the target's declared content property rather than to a named field, and
which field a body bound to cannot be recovered from the rendered source. Whether a value is
rendered on its own line SHALL be a layout decision only, and SHALL NOT change which syntax is used.

When a value has no NX source spelling, first-party tooling SHALL report a failure rather than emit
output that does not read back. A sequence held directly as an item of another sequence SHALL be
reported on those grounds: a braced value is not admitted as an item of a braced value, so
`{{ ... }}` does not parse. That SHALL hold whether or not the inner sequence is empty — giving the
empty value a spelling SHALL NOT be read as giving the nested empty sequence one.

It SHALL hold on the own-value path as well as in property position. A run of values one per line
cannot say where an inner sequence ends, so a nested sequence rendered there would come back as the
flattened run of its items — a different value, reported by nothing.

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
- **WHEN** first-party formatting renders a field named `items` holding a sequence of records
- **THEN** the output SHALL bind the sequence to `items` in property position
- **AND** it SHALL NOT emit an element whose tag is the field name

#### Scenario: An empty list is rendered as the empty braced form
- **WHEN** first-party formatting renders the empty value as a top-level value rather than as a
  record field
- **THEN** the output SHALL be `{}`
- **AND** it SHALL NOT quote the value or report a failure; a record field that holds the empty
  value is always optional and is omitted instead

#### Scenario: An empty optional attribute is omitted
- **WHEN** first-party formatting renders a record whose fields declared `subtitle?:string` and
  `tags?:string+` are both empty
- **THEN** the output SHALL NOT contain `subtitle` or `tags`
- **AND** re-evaluating the rendered source SHALL produce a record equal to the original, both
  fields empty

#### Scenario: An empty list round-trips
- **WHEN** first-party formatting renders a record whose optional field holds no items, and that source is parsed and type checked against the originating types
- **THEN** type checking SHALL report no diagnostics
- **AND** re-evaluating the rendered source SHALL produce the empty value at that field

#### Scenario: An empty list at a field with a non-empty default still renders
- **WHEN** a record declares `tags:string+ = {"a"}`
- **THEN** no value of that record holds `tags` empty, because `<Box tags={} />` is rejected
  statically naming `{}` and `string+`
- **AND** formatting therefore never meets an empty defaulted field; an empty field is optional and
  is omitted, and re-evaluating the rendered source binds it empty rather than to any default

#### Scenario: An empty list nested in a list is reported rather than rendered
- **WHEN** first-party formatting renders a field holding a sequence whose single item is a sequence
  with no items
- **THEN** it SHALL report a failure
- **AND** it SHALL NOT emit `{{}}`, which reads back as the empty value rather than a nested sequence

#### Scenario: A non-empty list nested in a list is reported the same way
- **WHEN** first-party formatting renders a field holding a sequence whose single item is a sequence
  with one item
- **THEN** it SHALL report a failure
- **AND** the reason SHALL be that a sequence nested in a sequence has no spelling, not that a
  sequence is empty

#### Scenario: A record between two lists still renders
- **WHEN** first-party formatting renders a field holding a sequence of records, each of which has a
  `*` field holding no items
- **THEN** the output SHALL render each record in element form with `{}` bound to its `*` field
- **AND** it SHALL NOT report a failure, because the records' braces are not nested directly

#### Scenario: An empty list rendered on its own is the braced form
- **WHEN** first-party formatting renders the empty value, not in property position
- **THEN** the output SHALL be `{}`
- **AND** it SHALL NOT be empty output

#### Scenario: A nested list rendered on its own is reported
- **WHEN** first-party formatting renders a sequence whose items are themselves sequences, not in
  property position
- **THEN** it SHALL report a failure
- **AND** it SHALL NOT emit the flattened run of the inner sequences' items

#### Scenario: A value with no source spelling is reported rather than rendered
- **WHEN** first-party formatting encounters a value that has no NX source spelling, such as an
  action handler
- **THEN** it SHALL report a failure
- **AND** it SHALL NOT emit a placeholder or a synthetic element in that position

#### Scenario: Scalar property values are emitted unquoted
- **WHEN** first-party formatting renders a record whose fields hold a float, a boolean, and a
  constant union case, and whose optional field `opt?:int` is empty
- **THEN** the output SHALL be of the form `<Box w=1.5 flag=true fit=cover />`
- **AND** it SHALL NOT quote any of those values, and SHALL NOT write `opt=null` or any other
  spelling for the empty optional field

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
