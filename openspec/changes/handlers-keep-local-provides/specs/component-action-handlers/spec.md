## ADDED Requirements

### Requirement: A handler that can read the environment captures the overrides around it
When a runtime makes an action handler whose node carries the environment flag, it SHALL capture
the overrides of the `envProvide` nodes being evaluated around the node, outermost first, in the
form of `$env` entries. Each entry SHALL be one of three kinds. An override evaluated where it was
written is held with its value. A formula is held as its captured data, with the locals it captured.
A bundle applied by a block is held as one entry with its entries. The runtime SHALL capture neither
the overrides of the component's descriptor chain nor the host's root values. A handler whose node
does not carry the flag SHALL capture no overrides. The compiler SHALL set the flag exactly when the
handler body's static read set is not empty, by the rule that makes an override a formula. Capturing
SHALL cost nothing beyond evaluating the node. The Rust and TypeScript IR runtimes SHALL capture
alike.

#### Scenario: A handler that reads the environment captures the provide around it
- **WHEN** a body renders `{ provide env.mode="block" { <Button onTapped=<State.Update last={env.mode} /> /> } }`
- **THEN** the handler SHALL hold one captured override, of `mode`, with an empty path and the value
  `"block"`

#### Scenario: A formula is captured as a formula
- **WHEN** a body renders `{ provide env.gap={env.space * 2} { <Button onTapped=<State.Update last={env.gap} /> /> } }`
- **THEN** the handler SHALL hold the `gap` override as a formula, not as a value

#### Scenario: A handler that reads no environment captures nothing
- **WHEN** a body renders `{ provide env.mode="block" { <Button onTapped=<State.Update count={count + 1} /> /> } }`
- **THEN** the handler node SHALL NOT carry the environment flag, and the handler SHALL hold no
  captured overrides

#### Scenario: A read through a function sets the flag
- **WHEN** a module declares `let current() = { env.mode }` and a handler's body is
  `<State.Update last={current()} />`
- **THEN** the handler node SHALL carry the environment flag

### Requirement: A handler runs with its captured overrides inside the environment of its call
When a runtime runs an action handler, it SHALL evaluate the body in the environment the call runs
in, with the handler's captured overrides inside it, as the overrides of `envProvide` nodes around
the body would be, applied in the order they were captured. A captured value SHALL NOT be checked
against its type again. A captured formula SHALL be evaluated at each read that reaches it, in the
environment of that read, with its captured locals bound. While the formula is evaluated, a read of
the value it sets SHALL see only the sources after it. The captured overrides SHALL count as the
call's own layers for the reuse of derived results. A dispatch SHALL run each handler of its batch
in the environment the host gives for the dispatch. That is the environment of the handler's creator
for a handler the dispatched instance bound, and for a handler the host dispatches by its token
against the instance whose output holds it. A handler that runs against the instance that bound it
SHALL still read that instance's state live.

#### Scenario: A handler sees a value set around it
- **WHEN** a host dispatches the token of a handler made inside `provide env.mode="block"` against
  the instance that rendered it, in that instance's environment
- **THEN** the handler's reads of `env.mode` SHALL be `"block"`

#### Scenario: A captured formula follows the environment of the dispatch
- **WHEN** a handler is made inside `provide env.gap={env.space * 2}`, and the host dispatches it
  with the root value of `space` set to `5`
- **THEN** the handler's read of `env.gap` SHALL be `10`, whatever `space` was when it was made

#### Scenario: Both runtimes run handlers alike
- **WHEN** both IR runtimes dispatch the lifecycles of the `environment-handlers` corpus program
- **THEN** they SHALL give the same rendered output, effects and state, and count the same
  operations

#### Scenario: An emit dispatched through the parent's token is exact
- **WHEN** `Page` renders `<Card env.space={env.space * 2} onLogged=<State.Update last={env.space} /> />`
  with `space` equal to `4` at `Page`, and a host dispatches the `onLogged` token from `Page`'s output
  against `Page`'s instance, in `Page`'s environment, with a `Card.Logged` action
- **THEN** `Page`'s `last` SHALL be `8`

### Requirement: Handler equality covers the captured overrides
Two action handlers SHALL be equal when they were made by one node and both their captured slots
and their captured overrides are equal. Captured slots SHALL be compared first, in ascending slot
index, as before. Then, for handlers of a node with the environment flag, the two lists of captured
overrides SHALL be compared as two sequences of values are compared, entry by entry, up to and
including the first pair that differs. Handlers of a node without the flag SHALL compare nothing
more than their slots. The Rust and TypeScript IR runtimes SHALL compare alike and count the same
operations. This SHALL change what `==` gives for two handlers made by one flagged node whose
captured overrides differ: they SHALL be unequal, where before this change they were equal.

#### Scenario: Handlers under different overrides are unequal
- **WHEN** a body binds `<Button onTapped=<State.Update last={env.mode} /> />` inside
  `provide env.mode={label}` in two renders between which only `label` changed
- **THEN** the two handlers SHALL compare unequal
- **AND** they SHALL compare equal when `label` is the same in both renders

#### Scenario: A handler that reads no environment ignores the overrides around it
- **WHEN** a body binds `<Button onTapped=<State.Update count={count + 1} /> />` inside
  `provide env.mode={label}` in two renders between which only `label` changed
- **THEN** the two handlers SHALL compare equal, at the cost they compared at before this change
