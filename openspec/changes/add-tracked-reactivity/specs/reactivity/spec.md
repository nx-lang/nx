## ADDED Requirements

### Requirement: Reactivity follows one model of tracked computations
The runtimes SHALL decide what to compute again when data changes by one model. A *source* SHALL be
data that can change: an environment value, which an override, a host root value, an entry-module
provide or a default supplies; a component's state field; and a prop. A *tracked computation* SHALL
be a component render, a top-level value declaration's value, an environment value's default, an
entry-module provide or a captured formula. A tracked computation SHALL record the sources it read
at run time, at any depth, and what it found, and a read of another tracked computation, evaluated
or reused, SHALL add that computation's reads to its record, directly within one call and through
an edge to it in the instance tree. A tracked computation SHALL be reused only while every recorded
read would find the same thing, and SHALL be computed again otherwise. Within one call a read SHALL
compare by the identity of what supplied it. In the instance tree, across calls, a read SHALL first
be classified by whether an override supplies it at the use, a read classified otherwise than it
was recorded SHALL count as changed, a read of a root value or of another kept computation SHALL
then compare by revision, a read an override supplied SHALL compare by the value found, and a
computation evaluated again to the same value SHALL count as unchanged for everything that used it.
The language SHALL have no syntax for reactivity: authors write plain values and components.

#### Scenario: A branch not taken is not a dependency
- **WHEN** a tracked computation reads `env.colorMode` only in a branch it did not take
- **THEN** a change of `colorMode` SHALL NOT cause it to be computed again

#### Scenario: Reads through another computation count
- **WHEN** a render reads `env.button`, whose default reads `env.color`, whose default reads
  `env.colorMode`
- **THEN** a change of `colorMode` SHALL reach the render, through `color`'s and `button`'s
  defaults

### Requirement: A runtime reuses derived results within one call
Within one call of an evaluation API, a runtime SHALL keep the last result of each *derived result*
it evaluates: a top-level value declaration's value checked against its declared type, an
environment value's default checked against its type, a targeted entry-module provide's value, an
entry-module bundle provide's bundle for the value being read, and a captured formula's value. A
bundle provide's result SHALL be identified by the provide and the value being read, since the
bundle is evaluated under a cap for that value. With each result the runtime SHALL record every
environment value the evaluation read, at any depth, including reads inside derived results it
evaluated or reused, each with its *token*. An evaluation SHALL begin at the environment stack as it
stands when the derived result is asked for, before the evaluation pushes its own cap. The token of
a value at a read SHALL identify the topmost layer that overrides the value or caps reads of it,
among the layers below where the derived result's evaluation began, or SHALL be none when no such
layer exists. Every layer the call pushes, the host's layers and caps included, SHALL have an
identity of its own, so a new override has a new token even when it sets an equal value.

At a later use of a derived result in the same call, the runtime SHALL reuse the kept result when
the current token of every recorded value equals the recorded token, and SHALL otherwise evaluate
it again and keep the new result in its place. A reused result's recorded reads SHALL count as reads
of any derived result being evaluated around the use, and as reads of the call when the instance
tree records them, each with its token computed again against where that enclosing evaluation
began. Results SHALL NOT be kept from one call to the next, except as the instance tree keeps them.
The Rust and TypeScript IR runtimes SHALL make the same reuse decisions, so their operation counts
stay equal. Reuse SHALL change no result. Because it does less work, a call MAY stay under a depth,
native-stack or operation limit that evaluating every use afresh would reach, and both runtimes
SHALL do so alike.

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

#### Scenario: A bundle that reads a value it sets is kept for each value read
- **WHEN** the entry module provides `env with makeTheme(env.scale)`, which sets `scale` and `gap`,
  and a call reads `env.gap`, then `env.scale`, then both again
- **THEN** each read SHALL give what evaluating every source afresh gives: `scale` built from
  `scale`'s default, and `gap` built from the provided `scale`
- **AND** the bundle SHALL be evaluated once for `gap` and once for `scale`, and not for the reads
  that follow

#### Scenario: Results do not survive the call
- **WHEN** a host makes the same call twice
- **THEN** both calls SHALL count the same operations

### Requirement: The instance tree's records hold direct inputs
In the instance tree, a node's record of its last render and a kept result's record SHALL have one
form: the direct inputs of the evaluation, in the order it first read them. A read of an environment
value that no layer of the place sets SHALL be a root read, holding the revision of the value's root
value and, when a provide or the default supplied it, an edge to that kept result. A kept result the
evaluation used otherwise SHALL be an edge to it. A read that a layer of the place sets, wholly or
at a field path, SHALL be an override-supplied read, holding the value found. Reads made inside a
kept result SHALL NOT be direct inputs of the records that use it: the edge stands for them, and
checking an edge SHALL validate the used result first, evaluating it again with the equality cutoff
when it is not valid, the edge holding when that result has not changed since the record was last
found valid. A kept result SHALL also have the flattened set of the environment values its leaf reads
read, found from its inputs and edges, used only to take it into a call. An evaluation that is not a
kept result, a formula, a formula entry of an entry-module bundle and a result evaluated under the
render's own overrides, SHALL give its direct inputs to the record that encloses it, so that a
record holds every read, at any depth, that no kept result stands for, and a read the render's own
override supplies wholly SHALL NOT be one. An override-supplied read SHALL be followed by the
direct inputs of what it evaluated, the formulas of the render's own field overrides on top of it
included, and a read inside the source of a layer the render pushed itself SHALL be a read of what
is beneath that layer. A read that a layer of the place sets, when a source of it was evaluated
under a layer the render pushed itself, SHALL count as changed at every check, since what it found
is not what a lookup at the place gives, and the direct inputs of what it evaluated SHALL follow
it. An override-supplied value SHALL compare as a host would see it written, with the rule
of the equality cutoff. A render's record SHALL start with what normalizing its props read, so
that a prop default that reads the environment is a source of the render. A node's record SHALL
also hold the props and state fields its render read, and the node SHALL hold the revision at
which its record was last found valid. `set_environment`, the skip check of a visit and a dispatch
SHALL check a node's record by the rule on kept derived results, so that the tree decides alike
whether a derived result was evaluated or reused.

#### Scenario: A reader through a reused default renders again
- **WHEN** a `Footer` node's render reads `env.color.accent` twice, the second read reuses
  `color`'s default, whose evaluation read `env.colorMode`, and a host changes the root value of
  `colorMode`
- **THEN** the `Footer` node SHALL render again and read the colors of the new mode

#### Scenario: An equal result stops a change from spreading
- **WHEN** a kept `direction` result reads `env.locale`, a node's render reads only `env.direction`
  through it, and a host changes `locale` from `en-US` to `en-GB`
- **THEN** `direction` SHALL be evaluated again and give `ltr` as before
- **AND** the node SHALL NOT render again

#### Scenario: A result evaluated again to a value written otherwise is a change
- **WHEN** a kept result of type `object` evaluates to `1` and, after a root change, to `1.0`
- **THEN** the result SHALL count as changed, and every node that used it SHALL render again

#### Scenario: A read through a bundle's formula entry follows what the formula reads
- **WHEN** the entry module declares `provide env with <env.Update gap={env.space * 2} />`, a node
  renders `env.gap`, and a host changes the root value of `space`
- **THEN** the node SHALL render again and read the new `gap`

#### Scenario: A source read under the body's own override follows its other reads
- **WHEN** `loud`'s default reads `env.mode` and `env.locale`, and a node's body renders
  `env.loud` inside its own `provide env.mode="dark"`
- **THEN** a host change of the root value of `mode` SHALL NOT render the node again
- **AND** a host change of the root value of `locale` SHALL render it again with the new `locale`

#### Scenario: A place's formula read under the body's own override follows what it reads
- **WHEN** a node's place sets `locale` to `if env.mode == "dark" { env.region } else { "fixed" }`,
  its body renders `env.loud`, whose default reads `env.locale`, inside its own
  `provide env.mode="dark"`, and a host changes the root value of `region`
- **THEN** the node SHALL render again and show what a fresh render shows

#### Scenario: A formula the body provides that reads its own value follows what is beneath it
- **WHEN** a body renders `env.x` inside its own `provide env.x={env.x + 1}`, and a host changes the
  root value of `x`
- **THEN** the node SHALL render again with the new value plus one

#### Scenario: A prop default that reads the environment is a source
- **WHEN** a host visits a descriptor that leaves out a prop whose default is `{ env.mode }`, and
  then changes the root value of `mode`
- **THEN** the node SHALL render again with the prop's new default

### Requirement: A dispatch in the instance tree renders again only when a state field the render read changed
A component render SHALL record the state fields its body read: a state field's `slot` node it
evaluated, and a state field captured by a formula the body wrote. A handler's state slots SHALL
NOT count as reads, since the tree runs every handler against the node whose body created it, with
that node's state live. After a dispatch's handlers run against a node, the instance tree SHALL
compare, for each state field the handlers patched that the render read, its value at the end with
its value at the start by the written identity the tree's equality cutoff uses, charged as `diff`
compares a field, and SHALL render the body again only
when one of them differs or the node's other records do not hold. Otherwise the node SHALL take the
new state and SHALL keep its rendered output, its tokens and its handler generation, with no output
written and no origins reported for it, and its instance SHALL count as the same render for every
comparison the tree makes, so that nothing is rebound. The tree SHALL run the batch through one
internal lifecycle call that applies the handlers and returns the new state and the effects, and
renders only when the tree needs it. The bare lifecycle APIs of the Rust and TypeScript IR runtimes
SHALL keep rendering the body after every batch.

#### Scenario: A field only handlers read causes no render
- **WHEN** a `Toggle` node's body reads `on` and not `pressedAt`, and a dispatch's handler patches
  only `pressedAt`
- **THEN** the node SHALL keep its rendered output and its tokens, SHALL hold the new `pressedAt`,
  and SHALL stay settled
- **AND** a later handler that reads `pressedAt` SHALL see the new value
- **AND** a token read from the output before the dispatch SHALL still dispatch

#### Scenario: A field the body read causes a render
- **WHEN** a dispatch's handler patches `on` to another value
- **THEN** the tree SHALL render the body again with the new `on`, with new tokens

#### Scenario: A field patched to the value it had causes no render
- **WHEN** a dispatch's handler patches `on` to the value it already holds
- **THEN** the node SHALL keep its rendered output

#### Scenario: A field captured by an override formula causes a render
- **WHEN** a body renders `<Row env.gap={env.space * scale}>` with a state field `scale` and reads
  no other use of `scale`, and a dispatch patches `scale`
- **THEN** the tree SHALL render the body again, and the `Row`'s formula SHALL capture the new
  `scale`

#### Scenario: The bare lifecycle renders on every dispatch
- **WHEN** a host dispatches, through `dispatch_component_actions` or the TypeScript runtime's
  dispatch, a batch that patches only a field the body does not read
- **THEN** the body SHALL be rendered again with fresh tokens, as before this change
