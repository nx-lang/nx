## MODIFIED Requirements

### Requirement: A handler runs against the instance whose body bound it
A host SHALL dispatch by naming the node whose rendered output it read, a token from that output,
and the action. The tree SHALL run the handler against the node whose body created it: the node
named, or the ancestor farthest from it that holds the same handler, which is where a handler a
component passed down through content or props was bound. The same handler is the very handler
that was handed down, not one equal to it: two instances of one component whose props and state
are equal hold handlers that are equal and are not one. The node dispatched against SHALL then
hold the instance and the rendered output the dispatch returned. A token the named node does not
hold SHALL fail with `nx-ir-handler-token`.

A dispatch replaces the instance of every node it runs against, and the nodes under such a node
hold handlers of the instance it had until a pass visits them again. A host therefore runs a pass
after every dispatch. A dispatch that reaches the tree before that pass, for a handler that
was handed down from an instance since replaced, SHALL fail with `nx-ir-handler-token` and SHALL
change nothing: it SHALL NOT run the handler against a node that was only handed it.

#### Scenario: A tap patches the instance that bound the handler
- **WHEN** a host dispatches the token of `onTapped=<State.Update count={count + 1} />` read from a
  `Counter` node's output, twice, each time with the token of the output then current
- **THEN** the node's state SHALL hold `count` equal to `2`

#### Scenario: A handler in a content child patches its owner
- **WHEN** `Page` renders `<Stack><Button onTapped=<State.Update taps={taps + 1} /> /></Stack>`, `Stack`
  is an authored component that renders its content, and the host dispatches the button's token
  through the `Stack` node whose output held it
- **THEN** the `Page` node's state SHALL hold `taps` equal to `1`
- **AND** the `Stack` node's own state SHALL be unchanged

#### Scenario: A retired token is refused
- **WHEN** a host dispatches a token from an output a previous dispatch replaced
- **THEN** the dispatch SHALL fail with `nx-ir-handler-token`

#### Scenario: A component nested under itself is told from its ancestor
- **WHEN** a component renders another instance of itself, both instances hold equal props and
  state, and the host dispatches the token of the inner instance's own handler through the inner
  node
- **THEN** the inner node's state SHALL be patched
- **AND** the outer node's state SHALL be unchanged

#### Scenario: An event from output a dispatch replaced is refused
- **WHEN** `Page` puts a button of its own in the content of a `Box`, the host dispatches the
  button's token through the `Box` node, and dispatches the same token again with no pass
  between
- **THEN** the second dispatch SHALL fail with `nx-ir-handler-token`
- **AND** the `Page` node SHALL hold what the first dispatch left and the `Box` node what it held
- **AND** after a pass, the token the `Box` node's output then holds SHALL dispatch against `Page`

### Requirement: A node is one use of an authored component at one place in the output
A host SHALL obtain a node by visiting a descriptor under a key it chooses for the descriptor's
place in the output and under the node whose rendered output held the descriptor, or under none.
The first visit at a key SHALL initialize the component from the descriptor's fields, through the
parent node's instance when there is one, so that a handler the parent's body bound in the
descriptor is the handler the child holds. A later visit at the same key with a descriptor of the
same component under the same parent SHALL return the same node. A visit at that key with a
descriptor of another component, or under another parent, SHALL replace the node with a new one
that starts from its initial state, and SHALL drop every node under the one replaced, which
belonged to output that place no longer has. A value that is not a component descriptor SHALL be
refused, and so SHALL a parent key that names no node and a parent that is the node itself or a
node under it, with `nx-ir-instance-key`.

#### Scenario: The first visit initializes from the initial state
- **WHEN** a host visits `<Counter />`, whose state declares `count:int = 0`, at a new key
- **THEN** the node's rendered output SHALL be the body rendered with `count` equal to `0`
- **AND** its handler tokens SHALL be those of a first render

#### Scenario: A child receives the handler its parent bound
- **WHEN** a host visits, under the node of `Page`, the descriptor `Page`'s output holds for
  `<Card onLogged=<State.Update last={action.text} /> />`
- **THEN** the handler `Card` holds for `onLogged` SHALL be the handler `Page` holds under the token
  that descriptor carries

#### Scenario: Another component at the same place starts again
- **WHEN** a key that held a `Counter` node whose `count` is `3` is visited with a descriptor of
  `Timer`
- **THEN** the node SHALL be a `Timer` in its initial state

#### Scenario: A node under another parent starts again and takes what was under it
- **WHEN** a key that held a `Box` node under one `Page` node is visited with a `Box` descriptor
  under another `Page` node
- **THEN** the node SHALL be a `Box` in its initial state holding the handlers the other `Page` bound
- **AND** a node replaced by a descriptor of another component SHALL leave no node under it

#### Scenario: A value that is not a descriptor and a key that names no node are refused
- **WHEN** a host visits a number, a record without a type or a list, visits under a key that names
  no node or under the node's own key, or dispatches at a key that names no node
- **THEN** the first three SHALL fail with `nx-ir-boundary-type` and the others with
  `nx-ir-instance-key`
- **AND** the tree SHALL be as it was
- **AND** a refused visit SHALL NOT count as a visit of the pass: a node at its key that no other
  visit of the pass reached SHALL be dropped when the pass ends

### Requirement: An emitted action is carried to the handler the parent bound
An effect a dispatch returns SHALL be routed when its type is an action the node's component emits
and the node holds a handler its parent bound for that emit: the tree SHALL dispatch the effect as
the action of that handler, against the node whose body created it. Any other effect SHALL be
returned to the host with the name of the component that produced it. A node that holds a handler
its parent bound for the emit, where no node above it holds that handler any longer because a
dispatch replaced the parent's instance and no pass has followed, SHALL fail the dispatch
with `nx-ir-handler-token`: the action SHALL NOT be returned to the host as though nobody had bound
a handler for it. Routing SHALL continue until
no effect remains to route. Within one host dispatch a node SHALL be dispatched against at most
once, with every action routed to it in one batch in the order the actions were produced, so that
no entry names a token an earlier dispatch of the same chain retired.

#### Scenario: A child's emit patches the parent
- **WHEN** `Card` emits `Logged`, `Page` renders `<Card onLogged=<State.Update last={action.text} /> />`,
  and a handler in `Card` returns `<Card.Logged text="hi" />`
- **THEN** the `Page` node's state SHALL hold `last` equal to `"hi"`
- **AND** the host SHALL receive no effect

#### Scenario: Two emitted actions reach the parent in one batch
- **WHEN** one handler in a child returns two actions its parent bound handlers for, each patching
  a different state field of the parent
- **THEN** the parent node's state SHALL hold both patches
- **AND** the parent SHALL have rendered once

#### Scenario: An emit whose handler belongs to a replaced render is refused
- **WHEN** a child's handler emits an action its parent bound a handler for, the host dispatches
  it, and dispatches the child's handler again with no pass between; or the parent's own
  handler is dispatched first and the child's after it
- **THEN** the later dispatch SHALL fail with `nx-ir-handler-token`
- **AND** every node SHALL hold what it held before that dispatch
- **AND** after a pass the child's handler SHALL dispatch and its emit SHALL reach the parent

#### Scenario: An emit nobody bound is a host effect
- **WHEN** a handler returns an action its component emits and no parent bound a handler for it
- **THEN** the dispatch SHALL return that action as an effect naming the component that produced it
