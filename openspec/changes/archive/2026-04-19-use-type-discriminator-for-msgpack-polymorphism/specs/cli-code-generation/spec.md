## MODIFIED Requirements

### Requirement: C# generated records use serializer metadata without emitted discriminator members
C# code generation SHALL emit generated record and action DTO classes that can serialize and
deserialize with both MessagePack and `System.Text.Json` without declaring a generated data member
mapped to wire key `$type`. Generated data members SHALL preserve their NX wire names across both
serializers, generated abstract records and actions SHALL remain inheritable, abstract roots SHALL
advertise polymorphism using `$type` and their concrete descendants for both serializers, and
MessagePack polymorphism SHALL use the same `$type`-keyed map contract as canonical `NxValue`.

#### Scenario: Concrete C# record emits only declared fields
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated C# SHALL emit `ShortTextQuestion` without a generated member mapped to `$type`
- **AND** generated property `Label` SHALL be annotated so both serializers use wire name `label`

#### Scenario: Concrete C# action emits only declared fields
- **WHEN** source contains `export action SearchRequested = { query:string }`
- **THEN** generated C# SHALL emit `SearchRequested` without a generated member mapped to `$type`
- **AND** generated property `Query` SHALL be annotated so both serializers use wire name `query`

#### Scenario: Abstract C# record root advertises polymorphism without a discriminator member
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder:string? }`
- **THEN** generated C# SHALL emit `Question` as an inheritable abstract generated record type
- **AND** `Question` SHALL advertise polymorphism using `$type` and its concrete descendants
- **AND** generated `ShortTextQuestion` SHALL not declare a generated member mapped to `$type`

#### Scenario: Intermediate abstract C# records inherit the root metadata without redeclaring a member
- **WHEN** source contains `export abstract type Question = { label:string } export abstract type TextQuestion extends Question = { placeholder:string? } export type ShortTextQuestion extends TextQuestion = { maxLength:int? }`
- **THEN** the generated root abstract type SHALL advertise polymorphism for its concrete
  descendants using `$type`
- **AND** intermediate abstract generated records SHALL inherit that metadata without redeclaring a
  generated member mapped to `$type`

#### Scenario: Abstract C# root without concrete descendants omits invalid polymorphism metadata and warns
- **WHEN** source contains `export abstract type Question = { label:string }`
- **THEN** generated C# SHALL emit `Question` without invalid polymorphism metadata
- **AND** generated C# SHALL include a comment explaining that no polymorphism metadata was
  generated because the abstract type had no concrete exported descendants at code-generation time
- **AND** the generator SHALL emit a warning that `Question` has no concrete exported descendants
  for C# polymorphic generation

#### Scenario: User field names do not collide with a synthetic discriminator member
- **WHEN** source contains `export type Payload = { nx_type:string }`
- **THEN** generated C# SHALL emit a property for wire name `nx_type`
- **AND** generated C# SHALL not emit any extra `__NxType` or `$type` data member on `Payload`
