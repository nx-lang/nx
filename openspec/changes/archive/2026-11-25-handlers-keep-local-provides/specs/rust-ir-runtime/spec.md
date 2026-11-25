## ADDED Requirements

### Requirement: A stored instance keeps the overrides its handlers captured
The serialized form of a component instance SHALL hold, for each handler, the overrides it captured
as well as its node and captured slots. A list of overrides that several handlers share SHALL be
written once. A restored handler SHALL run, and compare, as the original did. Restoring SHALL check
every captured override against the program, as part of checking that the instance's contents are
handlers of that program. An entry that is not of the `$env` entry shape, that names an
environment value the program does not declare, or that holds a formula the program would refuse
SHALL make restoring fail with a diagnostic, not a panic. Rebinding a kept instance to its parent's
new handlers SHALL replace a handler held inside a captured override value as it replaces one held
in a captured slot.

#### Scenario: A restored handler keeps its provide
- **WHEN** a host initializes a component that binds a handler reading `env.mode` inside
  `provide env.mode="block"`, serializes the instance, restores it in a process that has prepared the
  same images, and dispatches the handler's token
- **THEN** the handler SHALL read `"block"`, as against the original instance

#### Scenario: Handlers under one provide share their stored overrides
- **WHEN** a component renders a thousand rows inside one `provide`, each binding a handler that
  reads the environment
- **THEN** the serialized instance SHALL hold the captured list of overrides once

#### Scenario: An altered captured override is refused
- **WHEN** a serialized instance's handler holds a captured override that names an environment value
  the program does not declare
- **THEN** restoring SHALL fail with a diagnostic and SHALL NOT panic
