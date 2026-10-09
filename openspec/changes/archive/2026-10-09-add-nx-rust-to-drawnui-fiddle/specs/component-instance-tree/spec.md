## ADDED Requirements

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
