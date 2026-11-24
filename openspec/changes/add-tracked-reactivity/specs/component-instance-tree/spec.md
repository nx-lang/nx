## ADDED Requirements

### Requirement: A node handed a new descriptor renders again only when a prop it read changed
A node's render SHALL record the props its body read and the props read by each handler the body
bound. When a visit hands a node a descriptor that differs from the one it holds, and the node's
environment reads still hold, the tree SHALL compare only the recorded props of the two descriptors,
by value, charged as the descriptor comparison is charged, and SHALL render the node again only
when one of them differs. Otherwise the node SHALL take the new descriptor and keep its rendered
output, as it does for a change that touches only the overrides it carries, and SHALL stay settled.
Handler props bound for emits SHALL be rebound as for a kept child and SHALL NOT count as reads.

#### Scenario: A prop the body ignores does not render
- **WHEN** a `Badge` node's body reads `label` and not `tooltip`, and a pass hands it a descriptor
  whose only change is `tooltip`
- **THEN** the node SHALL keep its rendered output, hold the new descriptor, and stay settled

#### Scenario: A prop a handler reads renders
- **WHEN** a handler that `Badge`'s body binds reads `tooltip`, and a pass hands the node a new
  `tooltip`
- **THEN** the node SHALL render again

### Requirement: The tree keeps derived results across renders
The tree SHALL keep the derived results its renders evaluated and SHALL offer them to later renders
of any node. The tree SHALL keep one revision counter, and each root value that `set_environment`
changes, and each kept result whose value changes when it is evaluated again, SHALL take the next
revision. Each kept result SHALL hold the revision at which its value last changed, the revision at
which it was last found valid, and its direct inputs: the root values it read, the kept results it
used, and the values that an override above the use supplied. A kept result SHALL be valid when
every root value and kept result it used changed no later than it was last found valid, the used
results being validated first, and every override-supplied read finds an equal value at the use,
charged as the skip check's reads and comparisons are. Comparing revisions SHALL cost no operations.
A kept result that is not valid SHALL be evaluated again and its new value compared with the old,
charged as a comparison of values; when they are equal the tree SHALL keep the old value and its
revision. A result used at places whose override-supplied reads differ SHALL keep one variant for
each, and a variant no render used during a pass SHALL be dropped when the pass finishes. Within a
render, a result taken from the tree SHALL then follow the rule for reuse within one call. A
render's operation count SHALL be a function of the images, its input and what the tree holds.

#### Scenario: A render after a dispatch reuses the palette
- **WHEN** a node whose body reads `env.color.accent` is dispatched against and renders again with
  no change to the environment
- **THEN** the render SHALL reuse `color`'s kept default and `standardPalette`'s kept value
- **AND** it SHALL cost less than the same render in a new tree

#### Scenario: A root change makes kept results that read it stale
- **WHEN** a host changes the root value of `colorMode`, and a node then renders
- **THEN** `color`'s default SHALL be evaluated afresh in the new mode
- **AND** a kept result that read no changed value SHALL be reused without evaluating it

#### Scenario: Light and dark sections keep a variant each
- **WHEN** a pass renders one `Card` node inside `<div env.colorMode=light>` and another inside
  `<div env.colorMode=dark>`, and a later pass renders both again
- **THEN** the later pass SHALL reuse a `color` variant for each section without evaluating either

### Requirement: A change notifies only the computations that read it
The tree SHALL keep, for each environment value, an index of the nodes and kept results that read
its root value directly, and for each kept result the nodes and kept results that used it.
`set_environment` SHALL follow those indexes from the values whose root value changed to mark
possibly stale exactly the nodes and kept results they reach, evaluating nothing, and SHALL check
nothing else. Then, in order from the nodes nearest the root, it SHALL validate the kept results each
possibly stale node used, evaluating again only those that are not valid, and SHALL render the node
again only when one of its direct inputs changed. A dispatch SHALL decide whether to render from the
dispatched instance's own record of state reads. The indexes SHALL stay current as nodes render, are
replaced, removed and dropped, and as kept results are replaced and dropped.

#### Scenario: Unrelated kept results are not checked
- **WHEN** the tree holds kept results that read `colorMode` and others that read only `locale`, and
  a host changes `colorMode`
- **THEN** only the results that read `colorMode`, and what used them, SHALL be marked possibly stale
- **AND** the others SHALL NOT be validated

#### Scenario: A node whose inputs come back equal is left alone
- **WHEN** a node's render read only a kept `direction` result, a host changes `locale`, and
  `direction` evaluates again to an equal value
- **THEN** the node SHALL NOT render again and SHALL stay settled

### Requirement: The tree reports a change set to the host
The tree SHALL keep the keys of the nodes that rendered again, that were added, and that were
removed since the host last took the change set, and `take_changes` SHALL return them and clear
them. A node added and removed before the host took the set SHALL appear in neither list. A node
added and then rendered again SHALL appear as added only. A node that rendered several times SHALL
appear once. A dispatch that renders nothing SHALL add nothing to the set. `is_settled` SHALL remain,
and a node SHALL be unsettled exactly when it or a node under it rendered or was added since a pass
last walked it.

#### Scenario: A dispatch reports the node it rendered
- **WHEN** a host dispatches against a `Counter` node whose handler patches a field its body reads,
  and takes the change set
- **THEN** the set SHALL list the `Counter` node as rendered, and nothing as added or removed

#### Scenario: A pass reports what it created and dropped
- **WHEN** a pass visits a new `Row` descriptor and no longer finds an old one, and the host takes
  the change set
- **THEN** the set SHALL list the new node as added and the old one as removed

#### Scenario: A dispatch that renders nothing reports nothing
- **WHEN** a dispatch patches only a field no render read, and the host takes the change set
- **THEN** the set SHALL be empty

## MODIFIED Requirements

### Requirement: A change to a root value reaches only the nodes that read it
Each node SHALL record the environment values its last render read, including those read while
evaluating a default, an entry-module provide, or a formula or bundle entry carried in `$env` or in
an `envProvide`, for another read, together with the value each one had. The tree SHALL keep an
index from each environment value to the nodes whose last render read it, and SHALL keep it current
as nodes render, are replaced, removed and dropped. `set_environment` SHALL replace the tree's root
values and SHALL find the nodes to check through the index alone, from the values whose root value
changed, without walking the tree. For each of those nodes, the tree SHALL validate the node's
direct inputs as the requirement on kept derived results defines, and SHALL render again, with the
state it holds, each node for which any of them changed. It SHALL leave the others alone. A node rendered this way SHALL hold
rendered output no pass has walked, as after a dispatch, so that the host's next pass walks it and
what is under it. `set_environment` SHALL be all or nothing: when a root value is refused or a
render fails, the tree SHALL be as it was.

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

#### Scenario: A reader through an override formula renders again
- **WHEN** a `Cell` node reads `env.gap` under `<Row env.gap={env.space * 2}>`, and a host changes
  the root value of `space` from `4` to `6`
- **THEN** the `Cell` node SHALL render again and read `12`
