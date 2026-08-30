## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Payloadless union cases support contextual construction
At a binding site whose declared type is a discriminated union, the system SHALL accept a bare case
name and SHALL resolve it against that union's payloadless cases. A bare name that matches a case
requiring payload construction SHALL be rejected, and the diagnostic SHALL direct the author to the
element-style constructor.

A declaration's property types SHALL be resolved in the namespace of the module that declares them,
so the union a bare case name resolves against is the one the declaring module named when the use
site declares no union of that name. Resolving a case is distinct from carrying it through
lowering: until nominal types carry their declaring origin, a resolved case whose union is not
nameable in the using module SHALL be reported as needing an import, rather than accepted and
lowered to a reference that fails to resolve during code generation.

Union resolution reaches its definition by name, and a union declared in the using module takes
precedence over the one the declaring module named. A same-named local union therefore stands in for
a foreign one whatever cases it declares, which the member-list comparison applied to enums does not
cover here. Closing that gap requires declaring origin in type identity and is deferred with the
rest of it.

#### Scenario: Payloadless case is constructed by bare name
- **WHEN** a file contains `type LoadState = | idle | loading component <View state:LoadState /> = { <div /> } let v = <View state=loading />`
- **THEN** type checking SHALL accept `loading` as a value of type `LoadState`
- **AND** interpretation SHALL produce a case value with discriminator `LoadState.loading`

#### Scenario: Bare name for a payload case is rejected
- **WHEN** a file contains `type LoadState = | idle | failed { message:string } component <View state:LoadState /> = { <div /> } let v = <View state=failed />`
- **THEN** type checking SHALL reject `failed` because that case requires payload construction
- **AND** the diagnostic SHALL direct the author to `<LoadState.failed ... />`

#### Scenario: Union-typed property default accepts a bare case name
- **WHEN** a file contains `type LoadState = | idle | loading external component <View state:LoadState = idle />`
- **THEN** type checking SHALL accept `idle` as the default for `state`
