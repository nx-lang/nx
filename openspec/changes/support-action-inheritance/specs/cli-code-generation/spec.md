## MODIFIED Requirements

### Requirement: TypeScript generated records preserve concrete runtime discriminators
TypeScript code generation SHALL emit record-like declarations that preserve the NX `$type` payload
discriminator. Every generated concrete record or action record SHALL include a `$type` property
whose type is the string literal of that declaration's exported name. When a concrete record or
action derives from an exported abstract base of the same family, the generated output SHALL
preserve the abstract base's shared fields through a reusable base contract while keeping each
concrete descendant discriminated by its own literal `$type`.

#### Scenario: Concrete record includes a literal `$type`
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated TypeScript SHALL include a `ShortTextQuestion` contract with
  `$type: "ShortTextQuestion"`

#### Scenario: Abstract record family exposes a shared base and concrete runtime surface
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder:string? } export type LongTextQuestion extends Question = { wordLimit:int? }`
- **THEN** generated TypeScript SHALL preserve the shared `Question` fields in a generated base
  contract for descendants
- **AND** the generated `ShortTextQuestion` and `LongTextQuestion` contracts SHALL each include
  their own literal `$type`
- **AND** the exported `Question` type surface SHALL remain usable as the concrete runtime type for
  values of either descendant

#### Scenario: Cross-module abstract record family remains generated as a coherent TypeScript surface
- **WHEN** library module `questions/base.nx` exports `abstract type Question = { label:string }`
- **AND** library module `questions/short-text.nx` exports
  `type ShortTextQuestion extends Question = { placeholder:string? }`
- **THEN** library TypeScript generation SHALL emit any needed `import type` statements so the
  exported `Question` type surface in `questions/base.ts` can reference `ShortTextQuestion` without
  manual edits

#### Scenario: Exported action record includes a literal `$type`
- **WHEN** source contains `export action SearchRequested = { query:string }`
- **THEN** generated TypeScript SHALL include `$type: "SearchRequested"` on the generated
  `SearchRequested` contract

#### Scenario: Abstract action family exposes a shared base and concrete runtime surface
- **WHEN** source contains `export abstract action SearchAction = { source:string } export action SearchRequested extends SearchAction = { query:string } export action SearchSubmitted extends SearchAction = { submittedAt:string }`
- **THEN** generated TypeScript SHALL preserve the shared `SearchAction` fields in a generated base
  contract for descendants
- **AND** the generated `SearchRequested` and `SearchSubmitted` contracts SHALL each include their
  own literal `$type`
- **AND** the exported `SearchAction` type surface SHALL remain usable as the concrete runtime type
  for values of either descendant

#### Scenario: Cross-module abstract action family remains generated as a coherent TypeScript surface
- **WHEN** library module `actions/base.nx` exports `abstract action SearchAction = { source:string }`
- **AND** library module `actions/requested.nx` exports
  `action SearchRequested extends SearchAction = { query:string }`
- **THEN** library TypeScript generation SHALL emit any needed `import type` statements so the
  exported `SearchAction` type surface in `actions/base.ts` can reference `SearchRequested` without
  manual edits

### Requirement: C# generated records keep a concrete `$type` value
C# code generation SHALL map the NX `$type` payload key to a non-null discriminator member whose
runtime value matches the concrete generated declaration name. Generated abstract records and
actions SHALL remain inheritable, and derived concrete declarations SHALL expose their own
discriminator value rather than reusing the abstract base name.

#### Scenario: Concrete C# record initializes its discriminator to the concrete record name
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated C# SHALL include a non-null discriminator member mapped to `$type`
- **AND** that member SHALL default to `ShortTextQuestion`

#### Scenario: Abstract C# base remains inheritable and derived record keeps its own discriminator
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder:string? }`
- **THEN** generated C# SHALL emit `Question` as an inheritable abstract generated record type
- **AND** generated `ShortTextQuestion` SHALL expose a discriminator member mapped to `$type`
- **AND** that discriminator member SHALL default to `ShortTextQuestion` rather than `Question`

#### Scenario: Concrete C# action initializes its discriminator to the concrete action name
- **WHEN** source contains `export action SearchRequested = { query:string }`
- **THEN** generated C# SHALL include a non-null discriminator member mapped to `$type`
- **AND** that member SHALL default to `SearchRequested`

#### Scenario: Abstract C# action base remains inheritable and derived action keeps its own discriminator
- **WHEN** source contains `export abstract action SearchAction = { source:string } export action SearchRequested extends SearchAction = { query:string }`
- **THEN** generated C# SHALL emit `SearchAction` as an inheritable abstract generated record type
- **AND** generated `SearchRequested` SHALL expose a discriminator member mapped to `$type`
- **AND** that discriminator member SHALL default to `SearchRequested` rather than `SearchAction`
