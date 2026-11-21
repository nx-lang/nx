## MODIFIED Requirements

### Requirement: The Rust runtime exports an instance tree over a linked program
The Rust IR runtime SHALL export an instance tree created for one linked program. The tree SHALL
compose instances only through the runtime's component lifecycle, and through rebinding a kept
node, which evaluates nothing, so that every rendered output, state and token it yields, and every
diagnostic of an evaluation, is one the lifecycle operations yield for the same props, state and
batches. Every operation of the tree that evaluates SHALL take runtime options and SHALL
pass them to each lifecycle call it makes; the operation budget and the input limit SHALL apply to
each of those calls separately. The tree SHALL say whether a name is a component the program can
instantiate, which is what the lifecycle can initialize: a component entrypoint of the entry module
declared with a body. A host tells by it an authored component in rendered output from a record
that is the host's own to interpret.

#### Scenario: A node holds what the lifecycle renders
- **WHEN** a host visits a descriptor of `Counter` in a tree and separately initializes `Counter`
  from the same props through the lifecycle
- **THEN** the node's rendered output and tokens SHALL equal the initialization's

#### Scenario: An authored component is told from an external one
- **WHEN** a program declares `component <Card /> = { <Label /> }` and `external component <Label />`
- **THEN** the tree SHALL answer that `Card` can be instantiated
- **AND** SHALL answer that `Label` and an undeclared name cannot

#### Scenario: A limit reached inside a tree operation is the runtime's diagnostic
- **WHEN** a host dispatches through a tree under an operation budget a handler exhausts
- **THEN** the operation SHALL fail with `nx-ir-resource-limit` naming the operation budget

### Requirement: Props flow down and state stays with the node
A node visited with a descriptor equal to the one it was last initialized from SHALL NOT be
initialized again: it SHALL keep its instance's state, and its rendered output and tokens SHALL be
unchanged. A node visited with any other descriptor of its component SHALL be initialized again
from that descriptor's fields with the state the node holds, so that the output reflects the new
props and the kept state, and its tokens SHALL be those of the new render.

Two descriptors are equal when they hold the same values at the same places, with handlers
compared as follows:

- A handler the component's body can see, in a prop or in content at any depth, SHALL be resolved
  through the instance each descriptor was read from, and the two handlers SHALL be equal as the
  language's `==` finds them: the same handler node with equal captures.
- A handler the parent bound for one of the component's emits is never in the body's scope, so the
  two descriptors SHALL only both bind it or both leave it out.
- The text of a token SHALL NOT count. A parent that renders again numbers its handlers from one
  again, so the same text can name another handler, and other text can name an equal one.

When the old handlers are not paired one-to-one with new ones, the descriptors SHALL be treated as
different.

So a node that renders again reaches only the nodes under it whose descriptors changed, and in
turn the nodes under those. A node beside the one that rendered, and a node above it that no
emitted action reached, SHALL NOT render again because of it.

#### Scenario: A parent that renders again re-initializes the child with the state it held
- **WHEN** a child node's state has been patched to `count` equal to `2`, its parent re-renders
  with a different value for one of the child's props, and the host visits the child with the
  parent's new descriptor
- **THEN** the child's rendered output SHALL reflect the new descriptor's props and `count` equal
  to `2`

#### Scenario: A render reaches only the children whose descriptors changed
- **WHEN** a node renders `<Card title={first} onLogged=<State.Update last={action.text} /> />`
  and `<Card title="fixed" onLogged=<State.Update last={action.text} /> />`, it is dispatched
  against through the second card's emit so that `last` and `first` change, and a pass visits
  everything
- **THEN** the node and the first card SHALL hold instances of a new render, the card with the
  state it held
- **AND** the second card SHALL hold the state and the very rendered output it held before, and
  SHALL be settled once the pass ends
- **AND** a node beside the one dispatched against SHALL hold the instance and the rendered output
  it held before

#### Scenario: The same descriptor leaves the node alone
- **WHEN** a host visits a node twice with the same descriptor
- **THEN** the second visit SHALL return the first visit's rendered output with the same tokens

#### Scenario: An equal descriptor from a parent initialized again keeps the child
- **WHEN** a parent node is initialized again from other props, its output holds for a child a
  descriptor equal to the one the child was initialized from, and the host visits the child with it
- **THEN** the child SHALL NOT be initialized again and SHALL return the rendered output it held
- **AND** the handler the child holds SHALL be the one the parent's new instance holds at that
  place in the descriptor

#### Scenario: A visible handler with another capture is a changed descriptor
- **WHEN** `Page` renders `<Stack><Button onTapped=<State.Update taps={taps + 1} /> /></Stack>`,
  `Stack` is an authored component that renders its content, `taps` changes, and a pass visits the
  `Stack` node with `Page`'s new descriptor
- **THEN** the `Stack` node SHALL be initialized again with the state it held, because the handler
  in its content captures another value of `taps`

## ADDED Requirements

### Requirement: A kept child is rebound to its parent's new handlers
The tree SHALL rebind a node, instead of initializing it again, when a visit keeps the node because
its descriptor is equal to the one it was initialized from but was read from another instance of
the parent than the one the node was initialized through. Each handler the node
held from the parent's earlier instance SHALL be replaced by the handler the parent's current
instance holds at the same place in the new descriptor. The replacement SHALL reach everything the
node holds: its props, the handlers bound for its emits, its state, the handlers of its rendered
output, and the captures of handlers its own body made. A handler rebuilt that way SHALL in turn
replace the earlier one in every node initialized through the node, and so on down the tree.

A rebound node SHALL keep its generation, the order of its handlers and its rendered output, which
SHALL be the same allocation as before, so that every token a host read from that output names the
handler at the same place. A rebound node SHALL count as initialized through the parent's current
instance. Rebinding SHALL evaluate no NX code and SHALL count as a change to the tree, so a pass
that fails leaves every rebound node as it was.

#### Scenario: A kept child's emit reaches the parent's current instance
- **WHEN** a `Card` node is kept under a `Page` node that rendered again, and the host dispatches a
  `Card` handler that emits `Logged`, which `Page` bound to `<State.Update last={action.text} />`
- **THEN** the `Page` node's state SHALL hold the action's `text` in `last`
- **AND** the dispatch SHALL NOT fail with `nx-ir-handler-token`

#### Scenario: A handler handed through a kept child runs against its owner
- **WHEN** `Page` renders `<Stack><Button onTapped=<State.Update taps={taps + 1} /> /></Stack>`
  with a content handler whose captures don't change, `Page` renders again for another reason, a
  pass keeps the `Stack` node, and the host dispatches the button's token through the `Stack` node
- **THEN** the `Page` node's state SHALL be patched
- **AND** the `Stack` node SHALL hold the state it held

#### Scenario: Tokens read before the rebinding stay valid
- **WHEN** a host reads a token from a `Card` node's rendered output, the `Card`'s parent renders
  again, a pass keeps and rebinds the `Card`, and the host dispatches that token through the `Card`
- **THEN** the dispatch SHALL run the handler at that place of the `Card`'s output

#### Scenario: A node under a kept child is rebound with it
- **WHEN** `Page` renders a `Panel` that renders a `Card`, `Page` binds a handler for `Panel`'s emit
  and `Panel` binds a handler for `Card`'s emit that captures `Panel`'s props, `Page` renders again
  and a pass visits `Page` and `Panel` only
- **THEN** the `Panel` node SHALL be kept and rebound, and the `Card` node SHALL be kept
- **AND** an emit from the `Card` SHALL reach the `Panel` node, and an emit from the `Panel` the
  `Page` node

#### Scenario: A component nested under itself is still told from its ancestor after a skip
- **WHEN** a component renders another instance of itself with equal props, the outer instance's
  parent renders again so that both are kept and rebound, and the host dispatches the inner
  instance's own handler through the inner node
- **THEN** the inner node's state SHALL be patched
- **AND** the outer node's state SHALL be unchanged
