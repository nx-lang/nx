## MODIFIED Requirements

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
output that does not read back.

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

#### Scenario: A value with no source spelling is reported rather than rendered
- **WHEN** first-party formatting encounters a value that has no NX source spelling, such as an empty
  list or an action handler
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
