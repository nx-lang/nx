## ADDED Requirements

### Requirement: Reactivity follows one model of tracked computations
The runtimes SHALL decide what to compute again when data changes by one model. A *source* SHALL be
data that can change: an environment value, which an override, a host root value, an entry-module
provide or a default supplies; a component's state field; and a prop. A *tracked computation* SHALL
be a component render, a top-level value declaration's value, an environment value's default, an
entry-module provide or a captured formula. A tracked computation SHALL record the sources it read
at run time, at any depth, and what it found, and a read of another tracked computation, evaluated
or reused, SHALL add that computation's reads to its record. A tracked computation SHALL be reused
only while every recorded read would find the same thing, and SHALL be computed again otherwise.
Within one call a read SHALL compare by the identity of what supplied it. In the instance tree,
across calls, a read SHALL compare by the value found. The language SHALL have no syntax for
reactivity: authors write plain values and components.

#### Scenario: A branch not taken is not a dependency
- **WHEN** a tracked computation reads `env.colorMode` only in a branch it did not take
- **THEN** a change of `colorMode` SHALL NOT cause it to be computed again

#### Scenario: Reads through another computation count
- **WHEN** a render reads `env.button`, whose default reads `env.color`, whose default reads
  `env.colorMode`
- **THEN** the render's record SHALL include `button`, `color` and `colorMode`

### Requirement: A runtime reuses derived results within one call
Within one call of an evaluation API, a runtime SHALL keep the last result of each *derived result*
it evaluates: a top-level value declaration's value checked against its declared type, an
environment value's default checked against its type, an entry-module provide's value, and a
captured formula's value. With each result it SHALL record every environment value the evaluation
read, at any depth, including reads inside derived results it evaluated or reused, each with its
*token*. The token of a value at a read SHALL identify the topmost layer of the environment stack
that overrides the value or caps reads of it, counting only the layers that existed when the
derived result's evaluation began, or SHALL be none when no such layer exists. Every layer the call
pushes, the host's layers included, SHALL have an identity of its own, so a new override has a new
token even when it sets an equal value.

At a later use of a derived result in the same call, the runtime SHALL reuse the kept result when
the current token of every recorded value equals the recorded token, and SHALL otherwise evaluate
it again and keep the new result in its place. A reused result's recorded reads SHALL count as reads
of any derived result being evaluated around the use, and as reads of the call when the instance
tree records them. Results SHALL NOT be kept from one call to the next. The Rust and TypeScript IR
runtimes SHALL make the same reuse decisions, so their operation counts stay equal.

#### Scenario: A default is evaluated once in one environment
- **WHEN** a call reads `env.button.radius` and then `env.button.fill`, with no override between
  them, and `button`'s default reads `env.color`
- **THEN** `button`'s default and `color`'s default SHALL each be evaluated once

#### Scenario: An override of a value read evaluates the default again
- **WHEN** `color`'s default reads `env.colorMode`, and a call reads `env.color.accent` outside and
  then inside `provide env.colorMode=dark`
- **THEN** `color`'s default SHALL be evaluated twice, once for each mode

#### Scenario: An override of an unrelated value reuses the default
- **WHEN** `color`'s default reads only `env.colorMode`, and a call reads `env.color.accent` outside
  and then inside `provide env.locale="fr-FR"`
- **THEN** `color`'s default SHALL be evaluated once

#### Scenario: Only the reads a default made are its dependencies
- **WHEN** `tone`'s default reads `env.colorMode` only when `env.highContrast` is `true`,
  `highContrast` is `false`, and a call reads `env.tone` outside and then inside
  `provide env.colorMode=dark`
- **THEN** `tone`'s default SHALL be evaluated once

#### Scenario: An equal override is still a new override
- **WHEN** `colorMode` is `light` and a call reads `env.color.accent` outside and then inside
  `provide env.colorMode=light`
- **THEN** `color`'s default SHALL be evaluated twice

#### Scenario: A field override is applied to a reused default
- **WHEN** a call reads `env.color.accent`, and then reads it again inside
  `provide env.color.accent=red`
- **THEN** the second read SHALL be `red`
- **AND** `color`'s default SHALL be evaluated once, because the field override is a source of
  `color` applied on top of the default, not a read the default made

#### Scenario: Results do not survive the call
- **WHEN** a host makes the same call twice
- **THEN** both calls SHALL count the same operations

### Requirement: The instance tree records the reads of a reused result
A node's record of the environment values its last render read SHALL include the reads made inside
each derived result the render reused, with the values they found, so that `set_environment` and
the skip check see the same reads whether a derived result was evaluated or reused.

#### Scenario: A reader through a reused default renders again
- **WHEN** a `Footer` node's render reads `env.color.accent` twice, the second read reuses
  `color`'s default, whose evaluation read `env.colorMode`, and a host changes the root value of
  `colorMode`
- **THEN** the `Footer` node SHALL render again and read the colors of the new mode

### Requirement: A dispatch renders again only when a state field the render read changed
A component render SHALL record, on the instance it produces, the state fields its body read. After
a dispatch's handlers run, the Rust and TypeScript IR runtimes SHALL compare the state the dispatch
ends with against the state it began with, field by field, charged as `diff` compares fields, and
SHALL render the body again only when a field the last render read differs. Otherwise the
dispatch SHALL return an instance holding the new state with the previous rendered output and
handlers, and SHALL report that it did not render. A stored instance SHALL carry its record, and an
instance restored without one SHALL render on every dispatch. Handlers SHALL keep reading state live
when they run.

#### Scenario: A field only handlers read causes no render
- **WHEN** a `Toggle` component's body reads `on` and not `pressedAt`, and a dispatch's handler
  patches only `pressedAt`
- **THEN** the dispatch SHALL NOT render the body, SHALL return the previous rendered output, and
  SHALL hold the new `pressedAt`
- **AND** a later handler that reads `pressedAt` SHALL see the new value

#### Scenario: A field the body read causes a render
- **WHEN** a dispatch's handler patches `on`
- **THEN** the dispatch SHALL render the body again with the new `on`

#### Scenario: Both runtimes skip alike
- **WHEN** both IR runtimes run the same dispatch that patches only an unread field
- **THEN** both SHALL return the same instance state and output and count the same operations

