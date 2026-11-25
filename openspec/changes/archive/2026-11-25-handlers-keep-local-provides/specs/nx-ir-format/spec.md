## ADDED Requirements

### Requirement: An action handler whose body can read the environment is flagged behind `environment-v3`
NX IR SHALL encode an `actionHandler` node whose body's static read set is not empty with one
trailing cell, `1`, after its body: `[19, ref, str, ref, slot, ref?, node, 1]`. The read set SHALL
follow `env.` reads, the functions and values the body calls, the defaults of the values they read,
component elements whose prop defaults can read, and calls whose callee is known only at run time.
This is the rule that makes an override a formula. Any other `actionHandler` node SHALL be encoded
as before, with no trailing cell. A module that contains a flagged node SHALL list the required
feature `environment-v3`, and a module that contains none SHALL NOT. A runtime SHALL refuse, as
malformed, a module that holds a flagged node without listing `environment-v3`, and a module whose
trailing cell is anything but `1`. The `explain` text SHALL mark a flagged handler. Adding the flag
and the feature SHALL NOT change the IR schema version.

#### Scenario: A handler that reads the environment is flagged
- **WHEN** NX source renders `<Button onTapped=<State.Update last={env.mode} /> />`
- **THEN** the `actionHandler` node SHALL end with the flag cell `1`
- **AND** the module SHALL list `environment-v3`

#### Scenario: A handler that reads no environment is emitted as before
- **WHEN** NX source renders `{ provide env.mode="block" { <Button onTapped=<State.Update count={count + 1} /> /> } }`
- **THEN** the `actionHandler` node SHALL have no trailing cell
- **AND** the module SHALL NOT list `environment-v3`, and its image SHALL be the one emitted before
  this change

#### Scenario: A flagged handler without the feature is refused
- **WHEN** a runtime prepares an image whose `actionHandler` node ends with the flag cell and whose
  module does not list `environment-v3`
- **THEN** preparing SHALL fail with the malformed-image diagnostic naming `environment-v3`

### Requirement: Captured overrides have a defined cost
The format's documentation SHALL state how a handler captures, runs with and compares overrides, and
the runtimes SHALL count it alike. Making a flagged handler SHALL cost what evaluating its node
costs, with nothing for capturing the overrides around it. Running a handler SHALL cost its body's
nodes, and the defaults, entry-module provides and formulas evaluated for its reads under the rules
for reusing derived results, with nothing for applying its captured overrides. Comparing two handlers
of a flagged node SHALL cost, after their captured slots, what comparing their two lists of captured
overrides as two sequences costs, stopping at the first pair that differs. A comparison that stops
at a slot SHALL compare no overrides. The conformance corpus SHALL hold a program,
`environment-handlers`, whose lifecycles dispatch handlers bound in five places, with their results
and operation counts:
- inside a `provide` block;
- on an element with `env.` attributes;
- under a formula;
- in content another component places;
- for a child's emit, dispatched by the parent's token.

#### Scenario: Two flagged handlers compare their overrides after their slots
- **WHEN** both runtimes compare two handlers of one flagged node that read one bound slot, hold
  equal values in it, and captured one override each, with values that differ
- **THEN** both SHALL count one operation for the pair of handlers, one for the pair of slots, one
  for the two lists and one for the first pair of entries, and the operations that comparing those
  two entries' fields costs

#### Scenario: A dispatch under a captured formula is charged at the read
- **WHEN** both runtimes dispatch a handler that reads `env.gap` twice and captured the formula
  `env.space * 2`
- **THEN** both SHALL charge the formula's nodes once, and nothing for applying the captured override
