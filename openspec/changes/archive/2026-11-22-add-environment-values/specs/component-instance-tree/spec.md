## MODIFIED Requirements

### Requirement: Props flow down and state stays with the node
A node SHALL NOT be initialized again when it is visited with a descriptor equal to the one it
was last initialized from, leaving out the overrides both carry in `$env`, and its environment at
its new place gives every environment value its last render read the value it read then: it SHALL
keep its instance's state, and its rendered output and tokens SHALL be unchanged. When only the overrides
differ, the node SHALL take the new descriptor and the new environment, so that a later change is
judged against them. A node visited with any other descriptor of its component, or whose new
environment gives a value it read another value, SHALL be initialized again from that descriptor's
fields, in its new environment, with the state the node holds, so that the output reflects the new
props, the environment and the kept state, and its tokens SHALL be those of the new render.

Two descriptors are equal when they hold the same values at the same places, with handlers
compared as follows:

- A handler the component's body can see, in a prop or in content at any depth, SHALL be resolved
  through the instance each descriptor was read from, and the two handlers SHALL be equal as the
  language's `==` finds them: the same handler node with equal captures.
- A handler the parent bound for one of the component's emits is never in the body's scope, so the
  two descriptors SHALL only both bind it or both leave it out.
- The text of a token SHALL NOT count. A parent that renders again numbers its handlers from one
  again, so the same text can name another handler, and other text can name an equal one.

When one old handler would be replaced by two different new handlers, the descriptors SHALL be
treated as different. Two old handlers replaced by one new handler are allowed, since a render that
hands one handler to two places writes it at both. A handler bound for an emit SHALL also answer
the same emit in both descriptors, as initializing the node checks.

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

#### Scenario: A new override of a value the child didn't read keeps the child
- **WHEN** a parent renders `<Card title="x" />` and then, after a change to its state,
  `<Card title="x" env.locale="fr-FR" />`, and `Card`'s body reads no environment value
- **THEN** a pass that visits the `Card` node with the new descriptor SHALL keep it and its rendered
  output

#### Scenario: A new override of a value the child read renders it again
- **WHEN** a parent renders `<Card title="x" />` and then `<Card title="x" env.colorMode=dark />`,
  and `Card`'s body reads `env.colorMode`, whose value outside was `light`
- **THEN** the visit SHALL initialize the `Card` node again with the state it held, and its output
  SHALL reflect `dark`

## ADDED Requirements

### Requirement: Each node renders in the environment of its place
The tree SHALL give each node the environment of its place: the environment of the node it was
visited under, entered with the node's descriptor, or, for a node under no node, the tree's root
values entered with the descriptor. The tree SHALL hold root values that a host sets when it creates
the tree and changes with `set_environment`, checked as the environment-values capability defines.
Every initialization, render and dispatch the tree makes for a node SHALL run in the node's
environment. A host SHALL be able to ask the tree for the value of an environment value at a node.

#### Scenario: A node sees the override its parent placed around it
- **WHEN** `Page` renders `<div env.colorMode=dark><Card /></div>` and a host visits the `Card`
  descriptor under the `Page` node
- **THEN** the `Card` node's body SHALL read `dark` for `env.colorMode`
- **AND** the tree SHALL answer `dark` for `colorMode` at the `Card` node

#### Scenario: A dispatch renders in the node's environment
- **WHEN** a `Card` node whose environment has `colorMode` equal to `dark` is dispatched against
- **THEN** the render after the handlers SHALL read `dark`

### Requirement: A change to a root value reaches only the nodes that read it
Each node SHALL record the environment values its last render read, including those read while
evaluating a default or an entry-module provide for another read, together with the value each one
had. The tree SHALL keep an index from each environment value to the nodes whose last render read
it, and SHALL keep it current as nodes render, are replaced, removed and dropped. `set_environment`
SHALL replace the tree's root values and SHALL find the nodes to check through the index alone, from
the values whose root value changed, without walking the tree. For each of those nodes, the tree
SHALL look the values the node read up again in its environment, and SHALL render again, with the
state it holds, each node for which any of them differs. It SHALL leave the others alone. A node
rendered this way SHALL hold rendered output no pass has walked, as after a dispatch, so that the
host's next pass walks it and what is under it. `set_environment` SHALL be all or nothing: when a
root value is refused or a render fails, the tree SHALL be as it was.

#### Scenario: Only readers render again
- **WHEN** a tree holds `Page`, a `Header` node and a `Body` node under it, only `Header`'s body
  reads `env.colorMode`, and a host calls `set_environment` with `colorMode` changed from `light` to
  `dark`
- **THEN** the `Header` node SHALL render again with the state it held and read `dark`
- **AND** `Page` and `Body` SHALL hold the rendered output they held, and `Body` SHALL be settled

#### Scenario: A reader under an override that hides the change is left alone
- **WHEN** a `Card` node reads `env.colorMode` under `<div env.colorMode=light>`, and a host changes
  the root value of `colorMode` to `dark`
- **THEN** the `Card` node SHALL NOT render again

#### Scenario: A reader through a default renders again
- **WHEN** a `Footer` node reads `env.color`, whose default reads `env.colorMode`, and a host
  changes the root value of `colorMode`
- **THEN** the `Footer` node SHALL render again and read the colors of the new mode

#### Scenario: A refused root value changes nothing
- **WHEN** a host calls `set_environment` with a value of the wrong type for one value and a valid
  change to another
- **THEN** the call SHALL fail and every node SHALL hold what it held
