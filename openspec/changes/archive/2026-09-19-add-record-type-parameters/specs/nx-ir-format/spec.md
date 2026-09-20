## ADDED Requirements

### Requirement: NX IR erases record type parameters and type arguments
NX IR SHALL NOT carry record type parameters or type arguments. A generic record's field schema in
IR, and the field schema of its derived update record, SHALL describe each field with the type
parameter replaced by the top type `object`. An applied type SHALL be encoded as the nominal
reference to its record, with no encoding of its arguments, wherever a type reference appears —
including under list and nullable wrappers and inside a function type. A record construction in
IR SHALL NOT include a property for a type argument the source bound. Adding generic records SHALL
NOT change the IR schema version, and IR emitted for a program with generic records SHALL remain
deterministic and boundary-clean.

#### Scenario: Field schema carries the erased type
- **WHEN** NX source declares `type Range = { T:type start:T end:T endInclusive:boolean }`
- **AND** NX IR is emitted for the program
- **THEN** the record declaration in IR SHALL type `start` and `end` as `object` and `endInclusive` as `boolean`
- **AND** it SHALL NOT include a field or any other entry for `T`

#### Scenario: An applied type is a nominal reference
- **WHEN** NX source declares `type Range = { T:type start:T end:T }` and `type Slider = { range:<Range T=float64/> marks:<Range T=int/>[]? }`
- **AND** NX IR is emitted for the program
- **THEN** `range` SHALL be typed as the nominal reference to `Range` and `marks` as a nullable list of that same reference

#### Scenario: Construction omits the type argument
- **WHEN** NX source declares `type Range = { T:type start:T end:T }` and `let r = <Range T=int start={1} end={5} />`
- **AND** NX IR is emitted for the program
- **THEN** the construction of `r` SHALL carry properties for `start` and `end` and no property for `T`

#### Scenario: The schema version is unchanged
- **WHEN** NX IR is emitted for a program that declares and constructs a generic record
- **THEN** the image SHALL carry the same schema version as an image for a program without one
- **AND** an IR consumer built before generic records existed SHALL load and run it
