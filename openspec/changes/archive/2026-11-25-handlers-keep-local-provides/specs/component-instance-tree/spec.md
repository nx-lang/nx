## ADDED Requirements

### Requirement: A handler runs in its creator's current environment with the overrides it captured
The tree SHALL run every handler against its creator, in the environment the creator node holds when
the handler runs, with the handler's captured overrides inside it. That holds for a handler the
creator bound itself, for one it handed down through content or props, and for one it bound for a
child's emit. A handler's captured overrides SHALL NOT be a new kind of read in a node's record. They
are made from what the render read, so the records the tree keeps SHALL decide, as they do now,
whether a kept render's handlers are still the ones a fresh render would make. A handler's
environment reads when it runs SHALL NOT be reads of the creator's render. A change to the root
values SHALL reach a handler when it next runs, without rendering its creator.

#### Scenario: A handler in a content child sees the provide around it in its creator
- **WHEN** `Page` renders `{ provide env.mode="page" { <Box><Button onTapped=<State.Update last={env.mode} /> /></Box> } }`,
  `Box` is an authored component that renders its content inside `provide env.mode="box"`, and the
  host dispatches the button's token through the `Box` node
- **THEN** the `Page` node's `last` SHALL be `"page"`

#### Scenario: An emit reaches the handler with the overrides on the child's element
- **WHEN** `Page` renders `<Card env.mode="card" onLogged=<State.Update last={env.mode} /> />`, and
  the host dispatches a token of the `Card` node whose handler emits `Logged`
- **THEN** the `Page` node's `last` SHALL be `"card"`

#### Scenario: A kept node's handlers read its new place
- **WHEN** `App` renders `<Page env.mode={mode} />`, `Page`'s render does not read `env.mode`,
  `Page`'s handler inside its own `provide env.gap="g"` reads `env.mode`, `App`'s `mode` changes from
  `"a"` to `"b"`, a pass visits `Page` and keeps its render, and the host then taps the handler
- **THEN** the `Page` node SHALL NOT have rendered again
- **AND** the handler SHALL read `"b"` for `env.mode`

#### Scenario: A root change reaches a captured formula without a render
- **WHEN** `Page`'s render reads no environment value and binds a handler inside
  `provide env.gap={env.space * 2}` that reads `env.gap`, and a host calls `set_environment` with
  `space` changed from `1` to `3` and then taps the handler
- **THEN** `set_environment` SHALL NOT render `Page` again
- **AND** the handler SHALL read `6`

#### Scenario: A provide whose value changes renders the creator, which makes the new handler
- **WHEN** `Page` keeps state `label`, renders `{ provide env.mode={label} { <Button onTapped=<State.Update last={env.mode} /> /> } }`,
  and a dispatch patches only `label`
- **THEN** the tree SHALL render `Page` again, because its render read `label`
- **AND** a tap on the new button SHALL read the new `label` for `env.mode`

#### Scenario: A kept child holding a handler that reads no environment stays kept
- **WHEN** `Page` keeps state `mode` and renders `{ provide env.mode={mode} { <Box><Button onTapped=<State.Update count={count + 1} /> /></Box> } }`,
  `Box` reads no environment value, and a dispatch patches only `mode`
- **THEN** the pass after the dispatch SHALL keep the `Box` node's render, because the handler in
  its content compares equal
