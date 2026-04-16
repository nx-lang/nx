## MODIFIED Requirements

### Requirement: C# generated records keep a concrete `$type` value
C# code generation SHALL emit generated record and action DTO classes that can serialize and
deserialize with both MessagePack and `System.Text.Json`. The generated `$type` payload key SHALL
map to a non-null discriminator member for both serializers, generated data members SHALL preserve
their NX wire names across both serializers, generated abstract records and actions SHALL remain
inheritable, and derived concrete declarations SHALL expose their own discriminator value rather
than reusing the abstract base name.

#### Scenario: Concrete C# record initializes its discriminator and field names for both serializers
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated C# SHALL include a non-null discriminator member mapped to `$type` for both
  MessagePack and JSON
- **AND** that member SHALL default to `ShortTextQuestion`
- **AND** generated property `Label` SHALL be annotated so both serializers use wire name `label`

#### Scenario: Abstract C# base remains inheritable and derived record keeps its own discriminator
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder:string? }`
- **THEN** generated C# SHALL emit `Question` as an inheritable abstract generated record type
- **AND** `Question` SHALL advertise JSON polymorphism using `$type` and its concrete descendants
- **AND** generated `ShortTextQuestion` SHALL expose a discriminator member mapped to `$type` for
  both MessagePack and JSON
- **AND** that discriminator member SHALL default to `ShortTextQuestion` rather than `Question`

#### Scenario: Concrete C# action initializes its discriminator to the concrete action name
- **WHEN** source contains `export action SearchRequested = { query:string }`
- **THEN** generated C# SHALL include a non-null discriminator member mapped to `$type` for both
  MessagePack and JSON
- **AND** that member SHALL default to `SearchRequested`
- **AND** generated property `Query` SHALL be annotated so both serializers use wire name `query`

#### Scenario: Abstract C# action base remains inheritable and derived action keeps its own discriminator
- **WHEN** source contains `export abstract action SearchAction = { source:string } export action SearchRequested extends SearchAction = { query:string }`
- **THEN** generated C# SHALL emit `SearchAction` as an inheritable abstract generated record type
- **AND** `SearchAction` SHALL advertise JSON polymorphism using `$type` and its concrete
  descendants
- **AND** generated `SearchRequested` SHALL expose a discriminator member mapped to `$type` for
  both MessagePack and JSON
- **AND** that discriminator member SHALL default to `SearchRequested` rather than `SearchAction`

#### Scenario: Intermediate abstract C# records inherit the root discriminator contract
- **WHEN** source contains `export abstract type Question = { label:string } export abstract type TextQuestion extends Question = { placeholder:string? } export type ShortTextQuestion extends TextQuestion = { maxLength:int? }`
- **THEN** the generated root abstract type SHALL advertise JSON polymorphism for its concrete
  descendants using `$type`
- **AND** intermediate abstract generated records SHALL inherit that discriminator contract without
  redeclaring a conflicting `$type` member

### Requirement: Generated external component state contracts use stable companion names
Generated external component state contracts SHALL use stable companion names. When generated
output includes a companion state contract for an exported external component, the generator SHALL
name it `<ComponentName>_state`, SHALL include exactly the declared state fields from that
component, SHALL map those fields to the same wire names for both MessagePack and JSON, and SHALL
NOT include component props, emitted actions, or a `$type` discriminator.

#### Scenario: TypeScript companion state contract is a plain interface
- **WHEN** source contains `export external component <SearchBox /> = { state { query:string } }`
- **THEN** TypeScript generation SHALL emit `export interface SearchBox_state`
- **AND** SHALL include property `query: string`
- **AND** SHALL NOT emit `$type` on `SearchBox_state`

#### Scenario: C# companion state contract is a plain dual-annotated DTO
- **WHEN** source contains `export external component <SearchBox /> = { state { query:string } }`
- **THEN** C# generation SHALL emit a generated type `SearchBox_state`
- **AND** SHALL include the declared state field `query`
- **AND** SHALL annotate that field so both MessagePack and JSON use wire name `query`
- **AND** SHALL NOT emit a `$type` discriminator member on `SearchBox_state`
