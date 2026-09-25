## MODIFIED Requirements

### Requirement: Union cases support scoped construction
The system SHALL construct discriminated union cases through the owning union's scoped case name.
Payload cases SHALL support element-style construction using `<Union.case ... />`. Fieldless cases
SHALL support scoped member construction such as `Union.case`, and MAY also be constructed with an
empty element-style case constructor. Payload case construction MUST validate required fields,
defaulted fields, optional fields, content fields, unknown fields, and field types using the same
binding rules as record construction, so an optional field that is not written binds the empty
value and writing `{}` to a required or defaulted field is rejected, as `optional-properties`
requires.

#### Scenario: Payload case construction validates fields
- **WHEN** a file contains `type LoadState = | failed { message:string retryable:boolean = true } let state:LoadState = <LoadState.failed message={"Offline"} />`
- **THEN** type checking SHALL accept the construction
- **AND** interpretation SHALL produce a case value with discriminator `LoadState.failed`
- **AND** the case value SHALL include `retryable = true` from the case default

#### Scenario: Payload case construction leaves an omitted optional field empty
- **WHEN** a file contains `type LoadState = | failed { message:string code?:int } let state:LoadState = <LoadState.failed message={"Offline"} />`
- **THEN** type checking SHALL accept the construction
- **AND** the case value's `code` SHALL be the empty value, and its canonical JSON SHALL NOT contain a `code` key
- **AND** `<LoadState.failed message={} />` SHALL be rejected because `message` is required

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
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } let states:LoadState+ = { LoadState.idle <LoadState.failed message={"Offline"} /> }`
- **THEN** type checking SHALL accept the sequence because both items are cases of `LoadState`

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
- **WHEN** a file contains `type LoadState = | failed { message:string } | loaded { items:string+ } let read(state:LoadState) = state.message`
- **THEN** type checking SHALL reject `state.message` because `message` is not available on every
  `LoadState` value

#### Scenario: Shared inherited field is accessible before narrowing
- **WHEN** a file contains `abstract type EventBase = { source:string } type UiEvent extends EventBase = | clicked { x:int } | closed let read(event:UiEvent) = event.source`
- **THEN** type checking SHALL accept `event.source` because it is inherited by every `UiEvent`
  case

#### Scenario: Case field is accessible after narrowing
- **WHEN** a file contains `type LoadState = | failed { message:string } | loaded { items:string+ } let read(state:LoadState) = if state is { LoadState.failed => state.message else => "" }`
- **THEN** type checking SHALL accept `state.message` in the `LoadState.failed` arm

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
- **WHEN** a file contains `type LoadState = idle | loading | failed { message:string retryable:boolean = true } | loaded { items:Item+ }`
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

## REMOVED Requirements

### Requirement: Nullable union absence normalizes to null
**Reason**: `null` is removed from the language. A union-typed value that may be absent is now an
optional property (`completion?:FlowCompletion`) or an optional type (`FlowCompletion?`), and its
absence is the empty value `{}`, as `occurrence-types` defines.
**Migration**: Replaced by "Union absence is the empty value" below. Rewrite `completion?:Union` as
`completion?:Union`; write `completion={}` or omit the field where `completion={null}` was written;
match absence with the `{}` pattern from `presence-operators` rather than `null =>`.

## ADDED Requirements

### Requirement: Union absence is the empty value
When a discriminated-union value is expected through a type whose occurrence admits zero — an
optional property `p?:Union` or an optional type `Union?` — the system SHALL represent absence as
the empty value `{}`. The system MUST NOT synthesize an undeclared fieldless case such as
`<Union>.undefined` or `Union.undefined` to represent absence, in the interpreter, the TypeScript IR
runtime or any code generation target. A declared fieldless case SHALL continue to normalize as a
scoped union case value and SHALL remain distinct from `{}`: a `State?` that holds `State.idle`
is present. Absence of a `?`-typed union SHALL be matched with the `{}` pattern, and a match over
`Union?` whose arms cover `{}` and every case SHALL be exhaustive, as `presence-operators` defines.

#### Scenario: Omitted optional union field is empty
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } type QuestionFlow = { completion?:FlowCompletion } let root(): QuestionFlow = <QuestionFlow />`
- **THEN** type checking SHALL accept the omitted optional `completion` field
- **AND** interpretation SHALL bind `completion` to the empty value
- **AND** the canonical output SHALL NOT contain a `completion` key and SHALL NOT include `$type: "FlowCompletion.undefined"`

#### Scenario: Explicit empty optional union field is empty
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } type QuestionFlow = { completion?:FlowCompletion } let root(): QuestionFlow = <QuestionFlow completion={} />`
- **THEN** type checking SHALL accept the explicit empty `completion` field
- **AND** the value SHALL equal `<QuestionFlow />`
- **AND** the normalized output SHALL NOT include a union discriminator for `completion`

#### Scenario: Declared fieldless union case remains a case value
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } let root(): FlowCompletion? = FlowCompletion.continue`
- **THEN** interpretation SHALL normalize the result as a `FlowCompletion.continue` union case
- **AND** the result SHALL remain distinct from the empty value, so `root()?` SHALL evaluate to `true`

#### Scenario: Absence of an optional union is matched with the empty pattern
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } let label(c?:FlowCompletion): string = { if c is { {} => "pending" continue => "continue" end => c.message } }`
- **THEN** type checking SHALL accept the match as exhaustive
- **AND** `label({})` SHALL evaluate to `"pending"`
