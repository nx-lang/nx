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
The tree SHALL keep the derived results its renders evaluated, each with the reads its evaluation
made and the value each found, and SHALL offer them to later renders of any node. A kept result
SHALL be reused at a use when every recorded read, looked up in the environment of the use, finds an
equal value. The lookup SHALL be charged as the skip check's reads are, and each comparison as a
comparison of values. A kept result that fails the check SHALL be replaced by the new evaluation.
Within the render, a result taken from the tree SHALL then follow the rule for reuse within one
call. A render's operation count SHALL be a function of the images, its input and the results the
tree holds.

#### Scenario: A render after a dispatch reuses the palette
- **WHEN** a node whose body reads `env.color.accent` is dispatched against and renders again with
  no change to the environment
- **THEN** the render SHALL reuse `color`'s kept default and `standardPalette`'s kept value
- **AND** it SHALL cost less than the same render in a new tree

#### Scenario: A root change makes kept results that read it stale
- **WHEN** a host changes the root value of `colorMode`, and a node then renders
- **THEN** `color`'s default SHALL be evaluated afresh in the new mode
- **AND** a kept result that read no changed value SHALL still be reused

### Requirement: A change notifies only the computations that read it
The tree SHALL keep, for each environment value, an index of the nodes and kept results whose
records read it. `set_environment` SHALL mark stale exactly the nodes and kept results the index
names for the values whose root value changed, and SHALL check nothing else. A dispatch SHALL decide
whether to render from the dispatched instance's own record of state reads. The indexes SHALL stay
current as nodes render, are replaced, removed and dropped, and as kept results are replaced.

#### Scenario: Unrelated kept results are not checked
- **WHEN** the tree holds kept results that read `colorMode` and others that read only `locale`, and
  a host changes `colorMode`
- **THEN** only the results that read `colorMode` SHALL be marked stale
- **AND** the others SHALL NOT be looked up

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
