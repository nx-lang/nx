## MODIFIED Requirements

### Requirement: Rust runtime runs the component lifecycle
The Rust runtime SHALL construct a component descriptor, initialize a component from props,
evaluate a component from props and explicit state, normalize a complete state, apply a state patch
or a component update record to a state, and dispatch an ordered batch of emitted actions and
handler invocations against an instance. Each operation SHALL behave as `typescript-ir-runtime`
requires of the corresponding operation: content binding by declared occurrence, handler properties
carried as `ActionHandler` records, a parent instance supplying the handlers its tokens name, a
supplied state replacing the initial state, live state reads for a handler the component's own body
bound, update records of the instance's component patching its state and every other result
collected as an effect, one re-render after the batch, and fresh tokens on every lifecycle render.
Tokens SHALL be `h<generation>-<n>` assigned by the same walk, so the same program with the same
props and batches yields the same tokens in every NX runtime. Output of pure evaluation SHALL carry
no tokens. No operation SHALL modify an instance it was given, and a failed dispatch SHALL leave
the instance usable.

#### Scenario: Initialization returns rendered output, state and an instance
- **WHEN** a caller initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }`
- **THEN** the rendered `Button`'s `onTapped` SHALL be an `ActionHandler` record with action
  `Button.Tapped` and token `h1-1`
- **AND** the state SHALL hold `count` equal to `0`

#### Scenario: A handler invocation patches state and patches compound within a batch
- **WHEN** a caller dispatches two invocations of token `h1-1` with `<Button.Tapped />` in one
  batch against that instance
- **THEN** dispatch SHALL return state with `count` equal to `2`, no effects, and rendered output
  whose token is `h2-1`

#### Scenario: A parent-bound handler's results are effects
- **WHEN** a caller initializes `SearchBox` from a parent's rendered descriptor with the parent's
  instance, and dispatches `<SearchBox.SearchSubmitted searchString="docs" />`
- **THEN** dispatch SHALL return the parent handler's result as the only effect
- **AND** the `SearchBox` state SHALL be unchanged

#### Scenario: A stale token is rejected and the instance survives
- **WHEN** a caller dispatches a batch whose first entry patches state and whose second names a
  token from an earlier render
- **THEN** dispatch SHALL fail with `nx-ir-handler-token`
- **AND** a valid batch dispatched afterwards against the same instance SHALL observe the original
  state

#### Scenario: An invalid state or patch is rejected
- **WHEN** a caller evaluates a component with a state holding an undeclared field, or applies a
  patch whose value does not match the state field's type
- **THEN** the runtime SHALL fail with a diagnostic naming the field
- **AND** SHALL NOT return a partially updated state

#### Scenario: Pure evaluation carries no tokens
- **WHEN** a caller evaluates `Counter` from explicit state
- **THEN** the rendered `onTapped` SHALL be an `ActionHandler` record without a token
