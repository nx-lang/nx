# component-instance-tree Specification

## Purpose
Defines the instance tree behind the output of a program whose components use other components:
one instance for each use of an authored component, props flowing down, state staying with its
place in the output, a handler run against the instance whose body bound it, and an emitted action
carried to the handler a parent bound. The tree is indifferent to what a host makes of the output:
a host may draw it, or keep some other structure derived from it, and one that follows changes
walks only what changed. The Rust IR runtime implements the tree, so a Rust host composes instances
without writing the composition itself.

## Requirements

### Requirement: The Rust runtime exports an instance tree over a linked program
The Rust IR runtime SHALL export an instance tree created for one linked program. The tree SHALL
compose instances only through the runtime's component lifecycle, so that every rendered output,
state and token it yields, and every diagnostic of an evaluation, is one the lifecycle operations
yield for the same props, state and batches. Every operation of the tree that evaluates SHALL take runtime options and SHALL
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
  `<Card onLogged=<Update last={action.text} /> />`
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

### Requirement: Props flow down and state stays with the node
A node visited with the descriptor it was last initialized from SHALL NOT be initialized again, and
its rendered output and tokens SHALL be unchanged. A node visited with any other descriptor of its
component SHALL be initialized again from that descriptor's fields with the state the node holds,
so that the output reflects the new props and the kept state, and its tokens SHALL be those of the
new render.

A descriptor is the one a node was initialized from only when it holds the same values and was
read from the output of the same instance of the parent. A token numbers the handlers of one
render, and a parent that is initialized again numbers them from one again, so its output can hold
a descriptor that reads exactly as an earlier one did and names other handlers. Such a descriptor
is another descriptor.

So a node that renders again reaches every node under it: each is handed a descriptor read from
another instance and is initialized again when a pass visits it, with the state it held, and so in
turn are the nodes under that one. A node beside the one that rendered, and a node above it that
no emitted action reached, SHALL NOT render again because of it.

#### Scenario: A parent that renders again re-initializes the child with the state it held
- **WHEN** a child node's state has been patched to `count` equal to `2`, its parent re-renders and
  the host visits the child with the parent's new descriptor
- **THEN** the child's rendered output SHALL reflect the new descriptor's props and `count` equal
  to `2`

#### Scenario: A render reaches what is under the node and nothing beside it
- **WHEN** a node with two children is dispatched against through one child's emit, and a pass
  visits everything
- **THEN** the node and both children SHALL hold instances of a new render, each child with the
  state it held
- **AND** a node beside the one dispatched against SHALL hold the instance and the rendered output
  it held before

#### Scenario: The same descriptor leaves the node alone
- **WHEN** a host visits a node twice with the same descriptor
- **THEN** the second visit SHALL return the first visit's rendered output with the same tokens

#### Scenario: A descriptor that reads the same from a parent initialized again is a new one
- **WHEN** a parent node is initialized again from other props, its output holds for a child a
  descriptor equal, token included, to the one the child was initialized from, and the host visits
  the child with it
- **THEN** the child SHALL be initialized again with the state it held
- **AND** the handler the child holds SHALL be the one the parent's new instance holds under that
  token

### Requirement: A pass drops the nodes whose places left the output
The tree SHALL let a host mark the start and the end of a pass, and an instance SHALL live as long
as its place in the output. A node has rendered output no pass has walked from the moment it is
created, initialized again or dispatched against until a pass that visits it ends. When a pass
ends:

- a node the pass visited SHALL be kept;
- a node under a visited node that held output no pass had walked SHALL be dropped unless the pass
  visited it: the host has just walked that output and did not find the node's place in it;
- a node under a visited node whose output a pass had already walked SHALL be kept whether or not
  the pass visited it: that output is what it was, so the place is still in it;
- a node under a node the pass did not reach SHALL be kept with it;
- a node under no node SHALL be dropped unless the pass visited it;
- every node under a dropped node SHALL be dropped with it.

A key visited again after its node was dropped SHALL get a new node in its initial state.

A dispatch MAY reach the tree while a pass is under way, and a visit SHALL count for what the node
held when the visit was made. A node that rendered again after the pass visited it, and that the
pass did not visit again, holds output the pass has not walked: the end of that pass SHALL NOT drop
a node under it for being unvisited and SHALL NOT take the node for walked, so it stays unsettled
for the next pass. A node under another that the pass visited before the other rendered again, and
not after, was read from output the other no longer holds: when the pass ends it SHALL be treated
as a node the pass did not visit.

The tree SHALL say of a node whether it is settled, which it is when neither it nor any node under
it holds output no pass has walked. A pass MAY leave everything under a settled node unvisited, so
a host that follows changes walks from the root only to the nodes that rendered and what is under
them. While a node does not render again the tree SHALL hand out the same rendered output for it,
so that a host can tell by identity that what it last read there still stands.

A host SHALL be able to remove a node, and the nodes under it, by key, for a place the output
still has and the host no longer wants an instance at.

#### Scenario: A use that left the output loses its state
- **WHEN** a `Page` renders a `Counter` only while its `shown` state is true, the `Counter` node's
  `count` is `5`, `shown` becomes false and a pass visits the `Page` node, and then `shown` becomes
  true again and a pass visits the `Page` node and the `Counter` descriptor at the same key
- **THEN** the first pass SHALL drop the `Counter` node
- **AND** the node of the later pass SHALL render with `count` equal to its initial value

#### Scenario: A visited node survives the pass
- **WHEN** a pass visits a node whose `count` is `5`
- **THEN** the node SHALL hold `count` equal to `5` after the pass ends

#### Scenario: A node whose parent left the output leaves with it
- **WHEN** a pass visits a child node and not the node under no node that it was found under
- **THEN** both SHALL be dropped when the pass ends

#### Scenario: An unchanged node keeps the nodes under it without a visit
- **WHEN** a node under a `Page` node holds `opened` equal to `5`, nothing has rendered since the
  last pass, and a pass visits the `Page` node and nothing under it
- **THEN** the node under it SHALL hold `opened` equal to `5` and the rendered output it held
- **AND** a later pass that visits the `Page` node after it has rendered again, and not the node
  under it, SHALL drop that node

#### Scenario: A settled node says so until something under it renders
- **WHEN** a pass has visited a `Top` node, a `Middle` node under it, a `Leaf` node under that, and
  a second node under `Top`
- **THEN** all four SHALL be settled
- **AND** after a dispatch against the `Leaf` node alone, the `Leaf`, `Middle` and `Top` nodes
  SHALL NOT be settled and the second node SHALL be
- **AND** after a pass that visits `Top`, `Middle` and `Leaf` and leaves the second node unvisited,
  all four SHALL be settled and the second node SHALL hold what it held

#### Scenario: A node dispatched against after its visit is not taken for walked
- **WHEN** a pass visits a settled `Page` node and nothing under it, the `Page` node is dispatched
  against before the pass ends, and the pass ends
- **THEN** the node under the `Page` node SHALL be kept with the state it held
- **AND** the `Page` node SHALL NOT be settled, and the pass that follows SHALL bring what is under
  it up to its new output

#### Scenario: A node visited before its parent rendered again does not count as visited
- **WHEN** a pass visits a `Page` node and the `Counter` node under it, the `Page` node is
  dispatched against so that its output no longer holds a `Counter`, the pass visits the `Page`
  node again and nothing under it, and the pass ends
- **THEN** the `Counter` node SHALL be dropped and the `Page` node SHALL be settled
- **AND** a `Counter` visited at that key once the place returns SHALL be in its initial state
- **AND** a node whose place the new output still holds, visited before its parent rendered again
  and not after, SHALL likewise be dropped, never kept under a settled node with handlers of the
  instance its parent no longer holds

#### Scenario: A node is removed by key with the nodes under it
- **WHEN** a host removes a node that has a node under it, in a tree where nothing has rendered
  since the last pass
- **THEN** neither SHALL be in the tree, and a pass that visits only their parent SHALL NOT bring
  them back
- **AND** a visit at the removed key SHALL get a new node in its initial state

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
- **WHEN** a host dispatches the token of `onTapped=<Update count={count + 1} />` read from a
  `Counter` node's output, twice, each time with the token of the output then current
- **THEN** the node's state SHALL hold `count` equal to `2`

#### Scenario: A handler in a content child patches its owner
- **WHEN** `Page` renders `<Stack><Button onTapped=<Update taps={taps + 1} /> /></Stack>`, `Stack`
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
- **WHEN** `Card` emits `Logged`, `Page` renders `<Card onLogged=<Update last={action.text} /> />`,
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

### Requirement: A change to the tree is all or nothing
A host dispatch that fails at any point, in the handler it named or in one an effect was routed to,
SHALL leave every node holding the descriptor, the instance and the rendered output it held before
the dispatch, and SHALL report the runtime's diagnostic for the failure. The tree SHALL also let a
host run a pass as one change: when the pass fails, the nodes, what they hold and the
record of what the pass visited SHALL be as they were before it.

#### Scenario: A chain that fails part way restores every node
- **WHEN** a child's handler patches the child and emits an action whose parent handler fails
- **THEN** the dispatch SHALL fail with the parent handler's diagnostic
- **AND** the child node's state SHALL be what it was before the dispatch
- **AND** a token of the child's earlier output SHALL still dispatch

#### Scenario: A pass that fails leaves the tree as it was
- **WHEN** a pass run as one change re-initializes one node and then fails at another
- **THEN** the first node SHALL hold the descriptor, instance and rendered output it held before
  the pass

### Requirement: A handler bound outside any component is reported as inert
A descriptor visited under no node comes from pure evaluation, so a handler it carries has no token
and names no handler of any instance. The tree SHALL remove every such handler from the fields it
initializes the node from, at any depth, and SHALL report each to the host for the pass as
`<Type>.<property>`, naming the record that carried it. A descriptor visited under a node SHALL be
passed as it is.

#### Scenario: A handler on the top-level element is stripped and named
- **WHEN** the root function renders `<Card onLogged=<Noted /> />` and the host visits it under no
  node
- **THEN** the node SHALL initialize without a handler for `onLogged`
- **AND** the pass SHALL report `Card.onLogged` as inert

#### Scenario: A handler bound inside a component is kept
- **WHEN** the same element is rendered by a component's body and visited under that component's
  node
- **THEN** the child SHALL hold the handler and the pass SHALL report nothing inert

### Requirement: The instance tree is validated from NX source
The repository's tests SHALL compile NX source, link it, and drive an instance tree through each
scenario of this capability, comparing states, rendered outputs, effects and diagnostics with
stated expectations. The tests SHALL include a program three components deep in which an action
emitted at the bottom is handled at the top.

#### Scenario: The tree tests run with the runtime's tests
- **WHEN** a contributor runs the workspace's Rust tests
- **THEN** the instance tree tests SHALL run and SHALL fail if any scenario of this capability does
  not hold

### Requirement: The tree bounds how deeply component instances nest
The runtime options SHALL include a component depth, `max_component_depth`, 100 by default: the
most component instances that may be nested in an instance tree, counting a node and every node
above it. A visit for a node deeper than the component depth of the options it is given SHALL fail
with `nx-ir-resource-limit`, SHALL name the limit `maxComponentDepth` with its value, SHALL name
the component in its message, and SHALL change nothing in the tree. The check SHALL
apply to every visit, whether or not the tree already holds a node at the key. A record the host
interprets itself is not a node and SHALL NOT count.

Each visit is a separate call of the lifecycle, so no limit on one call ends a component that
renders itself. This limit is the one such a program meets.

#### Scenario: A component that renders itself ends at the component depth
- **WHEN** a host walks, under the default options, a program whose component renders itself
  inside a record of the host's
- **THEN** the first hundred visits SHALL succeed
- **AND** the next SHALL fail with `nx-ir-resource-limit` whose limit is `maxComponentDepth` with
  the value `100` and whose message names the component
- **AND** the hundred nodes SHALL still be in the tree
- **AND** the same walk run inside `atomically` SHALL leave the tree empty

#### Scenario: A host sets the component depth
- **WHEN** a program nests a component 150 deep over its data
- **THEN** a walk under the default options SHALL fail at the visit for the 101st node
- **AND** a walk under a component depth of 150 SHALL visit all 150
- **AND** a later walk under a component depth of 3 SHALL fail at the fourth node, naming the
  value `3`, and SHALL leave that node as it was
