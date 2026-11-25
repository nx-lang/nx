## ADDED Requirements

### Requirement: An action handler's body sees the overrides written around it
An action handler's body SHALL read the environment of the place where the handler is written: the
environment of the component instance it runs against, with the overrides around the handler in the
body that makes it inside that environment. Those overrides are the `provide` blocks around the
handler and the `env.` attributes of the elements around it, the element the handler is bound on
included. They SHALL apply as they would to a read written at the handler, nearest first. An
override's value SHALL be the one evaluated where it was written when the handler was made, and an
override whose value is a formula SHALL be computed at each read in the handler's body that reaches
it, in the environment of that read when the handler runs, as every environment value is. Overrides
around the place where another body puts the element that holds the handler SHALL NOT apply to it.
A function, a `let` value and a default that the handler's body evaluates SHALL read in the
handler's environment, as they read in the environment of any call or read that evaluates them.

#### Scenario: A handler inside a provide block
- **WHEN** a module declares `environment mode: string = "outer"`, and a component with state
  `last: string` renders `{ provide env.mode="block" { <Button onTapped=<State.Update last={env.mode} /> /> } }`
  and the button is tapped
- **THEN** `last` SHALL be `"block"`

#### Scenario: A handler on an element with `env.` attributes
- **WHEN** the same component renders `<Button env.mode="attr" onTapped=<State.Update last={env.mode} /> />`
  and the button is tapped
- **THEN** `last` SHALL be `"attr"`

#### Scenario: A handler bound for an emit on an element with `env.` attributes
- **WHEN** `Page` renders `<Card env.mode="card" onLogged=<State.Update last={env.mode} /> />` and
  `Card` emits `Logged`
- **THEN** `Page`'s `last` SHALL be `"card"`

#### Scenario: A formula around a handler is computed when the handler runs
- **WHEN** a component renders `{ provide env.mode={env.mode + "!"} { <Button onTapped=<State.Update last={env.mode} /> /> } }`,
  the host's root value of `mode` is `"a"` when the component renders and `"b"` when the button is
  tapped, and nothing renders between the two
- **THEN** `last` SHALL be `"b!"`

#### Scenario: A handler placed by another body sees where it is written
- **WHEN** `Page` renders `{ provide env.mode="page" { <Box><Button onTapped=<State.Update last={env.mode} /> /></Box> } }`,
  `Box` renders its content inside `provide env.mode="box"`, and the button is tapped
- **THEN** `Page`'s `last` SHALL be `"page"`

#### Scenario: A function a handler calls reads in the handler's environment
- **WHEN** a module declares `let current() = { env.mode }`, and a component renders
  `<Button env.mode="attr" onTapped=<State.Update last={current()} /> />` and the button is tapped
- **THEN** `last` SHALL be `"attr"`

#### Scenario: A handler outside any provide is unchanged
- **WHEN** a component whose place sets `mode` to `"placed"` renders
  `<Button onTapped=<State.Update last={env.mode} /> />` with no override around it, and the button
  is tapped
- **THEN** `last` SHALL be `"placed"`
