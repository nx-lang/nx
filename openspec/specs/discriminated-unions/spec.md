# discriminated-unions Specification

## Purpose
TBD - created by archiving change add-discriminated-unions. Update Purpose after archive.
## Requirements
### Requirement: Discriminated union declaration syntax
The parser SHALL support discriminated union declarations using the `type` keyword followed by a
required leading-pipe case list. A union declaration SHALL have at least one case. Each case SHALL
be scoped to the owning union, SHALL use an identifier case name, and MAY declare record-like
fields using the existing `PropertyDefinition` shape. Simple scalar choices SHALL continue to use
`enum`; a `type` declaration without a leading-pipe case list SHALL NOT be interpreted as a
discriminated union.

#### Scenario: Union declaration with fieldless and payload cases parses
- **WHEN** a file contains `type LoadState = | idle | loading | failed { message:string retryable:bool = true } | loaded { items:Item[] }`
- **THEN** the parser and lowering SHALL preserve a union definition named `LoadState`
- **AND** the union SHALL contain cases `idle`, `loading`, `failed`, and `loaded` in source order
- **AND** the `failed` and `loaded` cases SHALL preserve their declared fields and defaults

#### Scenario: Missing leading pipe is rejected as a union declaration
- **WHEN** a file contains `type LoadState = idle | loading`
- **THEN** parsing or validation SHALL reject the declaration for this change
- **AND** the system SHALL NOT treat it as a discriminated union case list

#### Scenario: Regular enum syntax remains valid
- **WHEN** a file contains `enum CardSortMode = closed | open`
- **THEN** the parser and lowering SHALL preserve an enum definition named `CardSortMode`
- **AND** the enum SHALL NOT be represented as a discriminated union

#### Scenario: Duplicate union cases are rejected
- **WHEN** a file contains `type LoadState = | idle | idle`
- **THEN** parsing, lowering, or semantic validation SHALL reject `LoadState` because case `idle`
  is declared more than once

### Requirement: Union cases support scoped construction
The system SHALL construct discriminated union cases through the owning union's scoped case name.
Payload cases SHALL support element-style construction using `<Union.case ... />`. Fieldless cases
SHALL support scoped member construction such as `Union.case`, and MAY also be constructed with an
empty element-style case constructor. Payload case construction MUST validate required fields,
defaulted fields, nullable fields, content fields, unknown fields, and field types using the same
binding rules as record construction.

#### Scenario: Payload case construction validates fields
- **WHEN** a file contains `type LoadState = | failed { message:string retryable:bool = true } let state:LoadState = <LoadState.failed message={"Offline"} />`
- **THEN** type checking SHALL accept the construction
- **AND** interpretation SHALL produce a case value with discriminator `LoadState.failed`
- **AND** the case value SHALL include `retryable = true` from the case default

#### Scenario: Fieldless case supports member shorthand
- **WHEN** a file contains `type LoadState = | idle | loading let state:LoadState = LoadState.idle`
- **THEN** type checking SHALL accept `LoadState.idle` as a value of type `LoadState`
- **AND** interpretation SHALL produce a case value with discriminator `LoadState.idle`

#### Scenario: Payload case cannot be used as a bare value
- **WHEN** a file contains `type LoadState = | failed { message:string } let state:LoadState = LoadState.failed`
- **THEN** type checking SHALL reject `LoadState.failed` because the `failed` case requires
  payload construction

#### Scenario: Unknown case field is rejected
- **WHEN** a file contains `type LoadState = | failed { message:string } let state = <LoadState.failed message={"Offline"} code={500} />`
- **THEN** type checking SHALL reject `code` because it is not a field of `LoadState.failed`

### Requirement: Union case values are compatible with their owning union
The type system SHALL treat each union case value as compatible with its owning union. When a union
extends an abstract record, every case SHALL inherit the effective field set and defaults of that
abstract base, and case values SHALL also be compatible with the abstract base type. Discriminated
unions SHALL remain closed; declarations outside the union case list MUST NOT add cases.

#### Scenario: Case value is accepted where union is expected
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } let render(state:LoadState) = state let value = render(<LoadState.failed message={"Offline"} />)`
- **THEN** type checking SHALL accept the call because `LoadState.failed` is compatible with
  `LoadState`

#### Scenario: Sibling cases infer the owning union as common type
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } let states:LoadState[] = { LoadState.idle <LoadState.failed message={"Offline"} /> }`
- **THEN** type checking SHALL accept the list because both items are cases of `LoadState`

#### Scenario: Union cases inherit abstract base fields
- **WHEN** a file contains `abstract type EventBase = { source:string = "ui" } type UiEvent extends EventBase = | clicked { x:int y:int } let event:EventBase = <UiEvent.clicked x={1} y={2} />`
- **THEN** type checking SHALL accept the case value where `EventBase` is expected
- **AND** interpretation SHALL include inherited field `source = "ui"` on the constructed case

#### Scenario: Union cannot be extended after declaration
- **WHEN** a file contains `type LoadState = | idle type MoreLoadState extends LoadState = | failed { message:string }`
- **THEN** semantic validation SHALL reject `MoreLoadState extends LoadState` because a union is
  not an abstract record base

### Requirement: Union field access respects narrowing
The type checker SHALL allow access to fields that are known on the static type of an expression.
On an unnarrowed union value, only fields inherited from an abstract base extended by the union
SHALL be accessible. Fields declared on individual cases SHALL be accessible only after control
flow has narrowed the value to that case.

#### Scenario: Case field is inaccessible before narrowing
- **WHEN** a file contains `type LoadState = | failed { message:string } | loaded { items:string[] } let read(state:LoadState) = state.message`
- **THEN** type checking SHALL reject `state.message` because `message` is not available on every
  `LoadState` value

#### Scenario: Shared inherited field is accessible before narrowing
- **WHEN** a file contains `abstract type EventBase = { source:string } type UiEvent extends EventBase = | clicked { x:int } | closed let read(event:UiEvent) = event.source`
- **THEN** type checking SHALL accept `event.source` because it is inherited by every `UiEvent`
  case

#### Scenario: Case field is accessible after narrowing
- **WHEN** a file contains `type LoadState = | failed { message:string } | loaded { items:string[] } let read(state:LoadState) = if state is { LoadState.failed => state.message else => "" }`
- **THEN** type checking SHALL accept `state.message` in the `LoadState.failed` arm

### Requirement: Union matches are checked for case validity and exhaustiveness
Match-style `if value is { ... }` expressions SHALL support discriminated union case patterns using
qualified case names. When the scrutinee type is a discriminated union and no `else` arm is
present, type checking SHALL require the case patterns to cover every case of that union. Within an
arm whose scrutinee is a local identifier, the type checker SHALL narrow that identifier to the
matched case for the arm body. This version SHALL NOT require or introduce an `as` binding.

#### Scenario: Exhaustive union match narrows each case
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } let view(state:LoadState) = if state is { LoadState.idle => "" LoadState.failed => state.message }`
- **THEN** type checking SHALL accept the match as exhaustive
- **AND** the `LoadState.failed` arm SHALL treat `state` as the `LoadState.failed` case

#### Scenario: Non-exhaustive union match without else is rejected
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } let view(state:LoadState) = if state is { LoadState.idle => "" }`
- **THEN** type checking SHALL reject the match because `LoadState.failed` is not covered and there
  is no `else` arm

#### Scenario: Else arm permits partial union match
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } let view(state:LoadState) = if state is { LoadState.idle => "" else => "fallback" }`
- **THEN** type checking SHALL accept the match because the `else` arm covers unmatched cases

#### Scenario: Pattern from another union is rejected
- **WHEN** a file contains `type LoadState = | idle type SaveState = | idle let view(state:LoadState) = if state is { SaveState.idle => "" else => "fallback" }`
- **THEN** type checking SHALL reject `SaveState.idle` because it is not a case of `LoadState`

### Requirement: Property-list match fragments support union narrowing
Match-style property-list fragments SHALL use the same discriminated union pattern validation,
identifier narrowing, and exhaustiveness behavior as value match expressions. When a property-list
match arm matches a union case and the scrutinee is a local identifier, property values in that arm
SHALL type check with the scrutinee narrowed to the matched case.

#### Scenario: Property-list match arm narrows union case fields
- **WHEN** a file contains `type LoadState = | failed { message:string } | idle component <Notice message:string /> = { <div>{message}</div> } let view(state:LoadState) = <Notice if state is { LoadState.failed => message=state.message else => message="" } />`
- **THEN** type checking SHALL accept `state.message` in the `LoadState.failed` property fragment
  arm
- **AND** the `else` branch SHALL cover the remaining `LoadState` cases

#### Scenario: Non-exhaustive property-list union match is rejected
- **WHEN** a file contains `type LoadState = | failed { message:string } | idle component <Notice message:string /> = { <div>{message}</div> } let view(state:LoadState) = <Notice if state is { LoadState.failed => message=state.message } />`
- **THEN** type checking SHALL reject the property-list match because `LoadState.idle` is not
  covered and there is no `else` branch

#### Scenario: Wrong-union property-list pattern is rejected
- **WHEN** a file contains `type LoadState = | failed { message:string } type SaveState = | failed component <Notice message:string /> = { <div>{message}</div> } let view(state:LoadState) = <Notice if state is { SaveState.failed => message="" else => message="" } />`
- **THEN** type checking SHALL reject `SaveState.failed` because it is not a case of `LoadState`

### Requirement: Union declarations participate in module visibility and imports
Discriminated union declarations SHALL use existing declaration visibility rules. Exported unions
SHALL be visible to importing modules through the union name, and cases SHALL be referenced through
the imported union name. Union cases SHALL NOT be imported or exported as independent top-level
declarations.

#### Scenario: Imported exported union case can be constructed
- **WHEN** library `../ui` exports `type LoadState = | idle | failed { message:string }`
- **AND** `app/main.nx` imports `../ui` and contains `let state:LoadState = <LoadState.failed message={"Offline"} />`
- **THEN** analysis SHALL resolve `LoadState.failed` through the imported `LoadState` union
- **AND** type checking SHALL accept the construction

#### Scenario: Private union is not visible to importers
- **WHEN** library `../ui` declares `private type LoadState = | idle`
- **AND** `app/main.nx` imports `../ui` and references `LoadState.idle`
- **THEN** analysis SHALL report that `LoadState` is not visible to the importing module

### Requirement: Tooling and docs describe discriminated unions
The system SHALL update first-party syntax references, language-tour documentation, examples,
fixtures, VS Code grammars, and snippets so they recognize and document discriminated union
declarations, case constructors, and match narrowing. Documentation SHALL distinguish regular enums
from discriminated unions and SHALL state that simple scalar choices use `enum`.

#### Scenario: VS Code highlights union syntax
- **WHEN** the VS Code grammar tokenizes `type LoadState = | idle | failed { message:string }`
- **THEN** it SHALL highlight `type`, case separators, case names, and case fields consistently
  with surrounding NX type syntax

#### Scenario: Documentation explains enum versus union usage
- **WHEN** a reader opens the NX type reference documentation
- **THEN** the documentation SHALL show `enum CardSortMode = closed | open` for scalar choices
- **AND** it SHALL show `type LoadState = | idle | failed { message:string }` for discriminated
  unions with cases
