## ADDED Requirements

### Requirement: The prelude is an ordinary NX IR module
The prelude SHALL be an NX IR module like any library module: it SHALL have one image, under its
reserved identity, holding its own declarations, and a module that references a prelude declaration
— by constructing it, by a nominal type reference, or as a derived declaration's target — SHALL list
the prelude in its module table and reference the declaration through that slot. No prelude
declaration SHALL be copied into another module's image. The prelude's module-table entry SHALL
carry the compiler's prelude version, which names the prelude's contract rather than its text: it
SHALL be bumped when a declaration's shape changes and SHALL be left unchanged by an edit that
changes no declaration. A runtime's prelude SHALL therefore be accepted by that version and by the
declarations it holds. An emit request SHALL produce the prelude's image when it names the
prelude's identity, and an emit request for every module SHALL include it exactly when some module
of the program references it; the file name an image is written under SHALL be derived from the
identity as for any module and SHALL be valid on every supported platform. A module that references
no prelude declaration SHALL be byte-for-byte what it was before the prelude existed.

#### Scenario: A range expression links against the prelude
- **WHEN** NX source contains `let r() = { 1..5 }`
- **AND** NX IR is emitted for the program
- **THEN** the module table SHALL list the prelude's identity with the compiler's prelude version
- **AND** the body of `r` SHALL be a record construction of `Range` through the prelude's slot with `start`, `end` and `endInclusive`
- **AND** the module's own declaration list SHALL NOT include `Range`

#### Scenario: The prelude image holds the declaration once
- **WHEN** an emit request names the prelude's identity
- **THEN** the emitted image SHALL declare the record `Range` with `start` and `end` typed `object` and `endInclusive` typed `boolean`, and its derived update record and property union

#### Scenario: Every-module emission includes the prelude only when used
- **WHEN** an emit request lists every module of a program in which one module uses a range expression
- **THEN** the emitted images SHALL include the prelude's
- **AND** the same request for a program that uses no prelude declaration SHALL NOT include it

#### Scenario: An unused prelude leaves no trace
- **WHEN** NX IR is emitted for a program that uses no prelude declaration
- **THEN** each image SHALL be identical to the image the same compiler emitted before the prelude existed

### Requirement: NX IR encodes iteration over a range as its own node behind a required feature
A `for` whose iterable is a `Range` SHALL be encoded as a `forRange` node, kind `22`, with the
layout of a `for` node — item slot and name, optional index slot and name, iterable, body — whose
iterable evaluates to a `Range` record. A `for` whose iterable is a list SHALL be encoded as before.
A module that contains a `forRange` node SHALL list the required feature `ranges-v1`, and a module
that contains none SHALL NOT, whether or not it constructs a range. The `explain` text of a
`forRange` node SHALL read as a `for` over a range. Adding the node kind and the feature SHALL NOT
change the IR schema version.

#### Scenario: A range loop is a `forRange` node
- **WHEN** NX source contains `let squares() = { for i, n in 0..4 { i * i + n } }`
- **AND** NX IR is emitted for the program
- **THEN** the body of `squares` SHALL be a `forRange` node carrying the item `i`, the index `n`, a `Range` construction and the body
- **AND** the module SHALL list `ranges-v1` among its required features

#### Scenario: Building a range without iterating needs no feature
- **WHEN** NX source contains `let r() = { 1..=5 }` and no `for` over a range
- **THEN** the emitted module SHALL NOT list `ranges-v1`

#### Scenario: A list loop is unchanged
- **WHEN** NX source contains `let doubled(items:int[]) = { for item in items { item * 2 } }`
- **THEN** the body SHALL be a `for` node and the module SHALL NOT list `ranges-v1`

### Requirement: A range is recognized by shape, and its element type is erased
A consumer of a `forRange` node SHALL recognize its iterable as a range by the value's shape — the
`$type` `Range` and the fields `start`, `end` and `endInclusive` — because a canonical value carries
`$type` as a bare declaration name with no module, and a nominal type carries no type arguments. The
prelude's `Range` SHALL therefore appear in an image with `start` and `end` typed `object`, and
`<Range T=int/>` and `<Range T=float64/>` SHALL be one declaration of one shape.

A `forRange` node SHALL be emitted only for an iterable whose declaration is the prelude's `Range`,
so a `Range` a module declares for itself SHALL NOT be iterated, and a host value SHALL be
normalized against the site's nominal type, which names its declaring module by slot. A host value
of another declaration that carries the same name and shape SHALL be accepted, as one is at any
site whose record name another module also declares.

An item's carrier SHALL be whatever numeric type the evaluating backend binds for the range's
bounds; NX's `int`, `int32` and `int64` SHALL be a distinction the checker draws rather than one an
image records. A backend whose numeric types do not separate them SHALL NOT be required to
reproduce them.

#### Scenario: The prelude's range is one declaration for every element type
- **WHEN** an image constructs `<Range T=int/>` and another constructs `<Range T=float64/>`
- **THEN** both SHALL reference the same prelude declaration, whose `start` and `end` are typed `object`

#### Scenario: A module's own `Range` is not iterated
- **WHEN** a module declares its own record named `Range` with integer `start` and `end` and a boolean `endInclusive`, and a `for` iterates a value of it
- **THEN** the compiler SHALL reject the loop rather than emit a `forRange` node

#### Scenario: The schema version is unchanged
- **WHEN** NX IR is emitted for a program that iterates a range
- **THEN** the image SHALL carry the same schema version as an image for a program without one
