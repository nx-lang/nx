## ADDED Requirements

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
