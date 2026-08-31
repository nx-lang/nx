## MODIFIED Requirements

### Requirement: Union cases support scoped construction
The system SHALL construct discriminated union cases through the owning union's scoped case name.
Payload cases SHALL support element-style construction using `<Union.case ... />`. Fieldless cases
SHALL support scoped member construction such as `Union.case`, and MAY also be constructed with an
empty element-style case constructor. Payload case construction MUST validate required fields,
defaulted fields, nullable fields, content fields, unknown fields, and field types using the same
binding rules as record construction.

#### Scenario: Payload case construction validates fields
- **WHEN** a file contains `type LoadState = | failed { message:string retryable:boolean = true } let state:LoadState = <LoadState.failed message={"Offline"} />`
- **THEN** type checking SHALL accept the construction
- **AND** interpretation SHALL produce a case value with discriminator `LoadState.failed`
- **AND** the case value SHALL include `retryable = true` from the case default

#### Scenario: Fieldless case supports member shorthand
- **WHEN** a file contains `type LoadState = idle | loading let state:LoadState = LoadState.idle`
- **THEN** type checking SHALL accept `LoadState.idle` as a value of type `LoadState`
- **AND** interpretation SHALL produce the constant case value of `LoadState.idle`

#### Scenario: Payload case cannot be used as a bare value
- **WHEN** a file contains `type LoadState = | failed { message:string } let state:LoadState = LoadState.failed`
- **THEN** type checking SHALL reject `LoadState.failed` because the `failed` case requires
  payload construction

#### Scenario: Unknown case field is rejected
- **WHEN** a file contains `type LoadState = | failed { message:string } let state = <LoadState.failed message={"Offline"} code={500} />`
- **THEN** type checking SHALL reject `code` because it is not a field of `LoadState.failed`

## ADDED Requirements

### Requirement: Union declaration syntax
The parser SHALL support discriminated union declarations using the `type` keyword followed by a
case list. A case list SHALL be one or more cases separated by `|`. A leading `|` before the first
case SHALL be optional when the list holds two or more cases, and SHALL be required when the list
holds exactly one case, because `type A = B` without it is a type alias. Each case SHALL be scoped
to the owning union, SHALL use an identifier case name, and MAY declare record-like fields using the
existing `PropertyDefinition` shape.

There SHALL be no separate declaration form for a closed set of constants. A union whose cases all
declare no fields, and which declares no base, is the form that scalar choices use.

#### Scenario: Union declaration with fieldless and payload cases parses
- **WHEN** a file contains `type LoadState = idle | loading | failed { message:string retryable:boolean = true } | loaded { items:Item[] }`
- **THEN** the parser and lowering SHALL preserve a union definition named `LoadState`
- **AND** the union SHALL contain cases `idle`, `loading`, `failed`, and `loaded` in source order
- **AND** the `failed` and `loaded` cases SHALL preserve their declared fields and defaults

#### Scenario: Multi-case union parses without a leading pipe
- **WHEN** a file contains `type CardSortMode = closed | open`
- **THEN** the parser and lowering SHALL preserve a union definition named `CardSortMode` with cases
  `closed` and `open`
- **AND** it SHALL be the same declaration as `type CardSortMode = | closed | open`

#### Scenario: Single-case union requires the leading pipe
- **WHEN** a file contains `type Wrapper = | only`
- **THEN** the parser and lowering SHALL preserve a union definition named `Wrapper` with the single
  case `only`

#### Scenario: A single name without a leading pipe remains a type alias
- **WHEN** a file contains `type Handle = string`
- **THEN** the declaration SHALL be a type alias
- **AND** it SHALL NOT be interpreted as a single-case discriminated union

#### Scenario: Duplicate union cases are rejected
- **WHEN** a file contains `type LoadState = idle | idle`
- **THEN** parsing, lowering, or semantic validation SHALL reject `LoadState` because case `idle`
  is declared more than once

### Requirement: A constant case is a union case with no fields in a union with no base
A union case SHALL be a **constant case** when it declares no fields and its owning union declares
no base. Every other case SHALL be a **payload case**, including a case that declares no fields in a
union that extends an abstract base, because such a case carries the base's fields. A **constant
union** SHALL be a union all of whose cases are constant.

Constant-ness SHALL be a property of the declaration, derivable without evaluating any value, and
SHALL be the sole basis on which the system decides a case's runtime and wire representation.

#### Scenario: Fieldless case in a base-less union is constant
- **WHEN** a file contains `type Shape = circle | square { n:int }`
- **THEN** `circle` SHALL be a constant case
- **AND** `square` SHALL be a payload case
- **AND** `Shape` SHALL NOT be a constant union

#### Scenario: Every case of a base-less fieldless union is constant
- **WHEN** a file contains `type CardSortMode = closed | open`
- **THEN** both cases SHALL be constant cases
- **AND** `CardSortMode` SHALL be a constant union

#### Scenario: An abstract base makes every case a payload case
- **WHEN** a file contains `abstract type EventBase = { source:string = "ui" } type UiEvent extends EventBase = clicked { x:int } | closed`
- **THEN** `closed` SHALL be a payload case even though it declares no fields
- **AND** `UiEvent` SHALL NOT be a constant union

### Requirement: A constant case is a scalar runtime value
A constant case SHALL evaluate to a scalar runtime value that carries its owning union and its case
name, and SHALL NOT be represented as a record. A payload case SHALL evaluate to a record value
carrying its case discriminator and fields. Consumers of runtime values SHALL NOT infer a value's
kind from an empty field map or from a dotted type name.

#### Scenario: A constant case is not a record
- **WHEN** a file contains `type LoadState = idle | loading let state:LoadState = LoadState.idle`
- **THEN** the runtime value of `state` SHALL be a scalar constant-case value of `LoadState.idle`
- **AND** it SHALL NOT be a record with zero fields

#### Scenario: An empty qualified record stays a record
- **WHEN** a runtime value is a record with no fields whose type name contains a dot, and it is not
  a union case
- **THEN** first-party formatting SHALL render it in element form
- **AND** the rendered source SHALL read back as the same value

#### Scenario: A constant case of a union with payload cases is still scalar
- **WHEN** a file contains `type Shape = circle | square { n:int } let s:Shape = Shape.circle let t:Shape = <Shape.square n={2} />`
- **THEN** the value of `s` SHALL be a scalar constant-case value
- **AND** the value of `t` SHALL be a record value carrying the `Shape.square` discriminator

### Requirement: The removed enum keyword is reported with the replacement form
The system SHALL recognize `enum` in declaration position and SHALL report a diagnostic naming the
`type` declaration to write instead, rather than reporting a generic parse error. `enum` SHALL NOT
be usable as a declaration form. Recognition SHALL be limited to declaration position: text that
merely reads like the removed declaration, but sits where the language holds prose or data, SHALL
NOT be reported.

#### Scenario: An enum declaration reports the type form
- **WHEN** a file contains `enum Fit = fill | contain | cover`
- **THEN** the system SHALL reject the declaration
- **AND** the diagnostic SHALL name the replacement `type Fit = fill | contain | cover`

#### Scenario: A declaration-shaped line in prose is not reported
- **WHEN** the line `enum Fit = fill | contain | cover` appears inside element text content — raw,
  typed, or plain — or inside a comment, or inside a string literal
- **THEN** the system SHALL NOT report the removed-keyword diagnostic for it
- **AND** a real `enum` declaration elsewhere in the same file SHALL still be reported

### Requirement: Tooling and docs describe unions as one declaration form
The system SHALL update first-party syntax references, language-tour documentation, examples,
fixtures, VS Code grammars, and snippets so they recognize and document discriminated union
declarations, case constructors, and match narrowing. Documentation SHALL present the constant union
as the form scalar choices use, and SHALL NOT document a separate enum declaration form.

#### Scenario: VS Code highlights union syntax
- **WHEN** the VS Code grammar tokenizes `type LoadState = idle | failed { message:string }`
- **THEN** it SHALL highlight `type`, case separators, case names, and case fields consistently
  with surrounding NX type syntax

#### Scenario: Documentation explains scalar choices and payload cases with one form
- **WHEN** a reader opens the NX type reference documentation
- **THEN** the documentation SHALL show `type CardSortMode = closed | open` for scalar choices
- **AND** it SHALL show `type LoadState = idle | failed { message:string }` for unions with
  payload cases
- **AND** it SHALL NOT present these as two different kinds of declaration

## REMOVED Requirements

### Requirement: Discriminated union declaration syntax
**Reason**: The requirement described two declaration forms and required a leading `|` on every
case. Replaced by *Union declaration syntax*, which removes the enum form and makes the leading `|`
optional for a list of two or more cases, keeping it required for a single-case union so that
`type A = B` remains unambiguously a type alias.
**Migration**: Write `type CardSortMode = closed | open` instead of `enum CardSortMode = closed | open`.
Existing union declarations with a leading `|` on every case remain valid and are unchanged.

### Requirement: Tooling and docs describe discriminated unions
**Reason**: The requirement obliged documentation to distinguish enums from discriminated unions and
to state that simple scalar choices use `enum`. With one declaration form there is no distinction to
draw. Replaced by *Tooling and docs describe unions as one declaration form*.
**Migration**: First-party documentation and snippets that show `enum` are rewritten to the `type`
form; no tooling contract changes.
