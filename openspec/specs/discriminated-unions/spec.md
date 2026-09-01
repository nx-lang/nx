# discriminated-unions Specification

## Purpose
TBD - created by archiving change add-discriminated-unions. Update Purpose after archive.
## Requirements
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

### Requirement: Nullable union absence normalizes to null
When a discriminated-union value is expected through a nullable type reference, the system SHALL
represent absence as `null`. The system MUST NOT synthesize undeclared fieldless cases such as
`<Union>.undefined` or `Union.undefined` to represent nullable absence. Declared fieldless union
cases SHALL continue to normalize as scoped union case values.

#### Scenario: Omitted nullable union field produces null
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } type QuestionFlow = { completion:FlowCompletion? } let root(): QuestionFlow = <QuestionFlow />`
- **THEN** type checking SHALL accept the omitted nullable `completion` field
- **AND** interpretation SHALL normalize `completion` to `null`
- **AND** the normalized output SHALL NOT include `$type: "FlowCompletion.undefined"`

#### Scenario: Explicit null nullable union field produces null
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } type QuestionFlow = { completion:FlowCompletion? } let root(): QuestionFlow = <QuestionFlow completion={null} />`
- **THEN** type checking SHALL accept the explicit nullable `completion` field
- **AND** interpretation SHALL normalize `completion` to `null`
- **AND** the normalized output SHALL NOT include a union discriminator for `completion`

#### Scenario: Declared fieldless union case remains a case value
- **WHEN** source contains `type FlowCompletion = | continue | end { message:string } let root(): FlowCompletion = FlowCompletion.continue`
- **THEN** interpretation SHALL normalize the result as a `FlowCompletion.continue` union case
- **AND** the result SHALL remain distinct from `null`

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
qualified case names, and SHALL support bare case names resolved against the scrutinee's union
type. When the scrutinee type is a discriminated union and no `else` arm is
present, type checking SHALL require the case patterns to cover every case of that union. Within an
arm whose scrutinee is a local identifier, the type checker SHALL narrow that identifier to the
matched case for the arm body. This version SHALL NOT require or introduce an `as` binding.

#### Scenario: Exhaustive union match narrows each case
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } let view(state:LoadState) = if state is { LoadState.idle => "" LoadState.failed => state.message }`
- **THEN** type checking SHALL accept the match as exhaustive
- **AND** the `LoadState.failed` arm SHALL treat `state` as the `LoadState.failed` case

#### Scenario: Bare case patterns narrow and check identically to qualified ones
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } let view(state:LoadState) = if state is { idle => "" failed => state.message }`
- **THEN** type checking SHALL accept the match as exhaustive
- **AND** the `failed` arm SHALL treat `state` as the `LoadState.failed` case

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

#### Scenario: Bare pattern that is not a case of the scrutinee union is rejected
- **WHEN** a file contains `type LoadState = | idle | loading let view(state:LoadState) = if state is { pending => "" else => "fallback" }`
- **THEN** type checking SHALL reject `pending` because it is not a case of `LoadState`
- **AND** the diagnostic SHALL list the cases of `LoadState`

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

### Requirement: Payloadless union cases support contextual construction
At a binding site whose declared type is a discriminated union, the system SHALL accept a bare case
name and SHALL resolve it against that union's payloadless cases. A bare name that matches a case
requiring payload construction SHALL be rejected, and the diagnostic SHALL direct the author to the
element-style constructor. Resolution SHALL NOT require the union type to be in lexical scope at the
use site.

A resolved case SHALL lower and evaluate whether or not the using module can name its union,
including a case that inherits fields from an abstract base.

#### Scenario: Payloadless case is constructed by bare name
- **WHEN** a file contains `type LoadState = idle | loading component <View state:LoadState /> = { <div /> } let v = <View state=loading />`
- **THEN** type checking SHALL accept `loading` as a value of type `LoadState`
- **AND** interpretation SHALL produce a case value with discriminator `LoadState.loading`

#### Scenario: Bare name for a payload case is rejected
- **WHEN** a file contains `type LoadState = idle | failed { message:string } component <View state:LoadState /> = { <div /> } let v = <View state=failed />`
- **THEN** type checking SHALL reject `failed` because that case requires payload construction
- **AND** the diagnostic SHALL direct the author to `<LoadState.failed ... />`

#### Scenario: Union-typed property default accepts a bare case name
- **WHEN** a file contains `type LoadState = idle | loading external component <View state:LoadState = idle />`
- **THEN** type checking SHALL accept `idle` as the default for `state`

#### Scenario: Case of a union declared in another module is constructed by bare name
- **WHEN** a module imports a component whose property is typed by a union declared in another
  module, and does not import that union
- **THEN** a bare payloadless case name SHALL be accepted at that property
- **AND** it SHALL evaluate to the same value as the qualified form of that case

#### Scenario: Foreign case inheriting an abstract base is constructed with its base fields
- **WHEN** the union in the declaring module extends an abstract record base, and a bare payloadless
  case of it is written at a property of an imported component
- **THEN** the constructed value SHALL carry the base's fields and their defaults
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
