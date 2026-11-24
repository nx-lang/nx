## MODIFIED Requirements

### Requirement: A change to a root value reaches only the nodes that read it
Each node SHALL record the environment values its last render read, including those read while
evaluating a default, an entry-module provide, or a formula or bundle entry carried in `$env` or in
an `envProvide`, for another read, together with the value each one had. The tree SHALL keep an
index from each environment value to the nodes whose last render read it, and SHALL keep it current
as nodes render, are replaced, removed and dropped. `set_environment` SHALL replace the tree's root
values and SHALL find the nodes to check through the index alone, from the values whose root value
changed, without walking the tree. For each of those nodes, the tree SHALL look the values the node
read up again in its environment, and SHALL render again, with the state it holds, each node for
which any of them differs. It SHALL leave the others alone. A node rendered this way SHALL hold
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
