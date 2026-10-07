## ADDED Requirements

### Requirement: The conformance corpus holds programs of the size hosts run
The conformance corpus SHALL hold, beside its small programs, at least two programs of the size a
host runs, each with everything a corpus program has: committed images, results, operation counts
and, where it has a lifecycle, the rendered output and effects of each batch.

- A catalog program SHALL declare at least 40 external components, some of them abstract bases
  that others extend, and at least 300 properties among them, with a snippet that is compiled
  against the catalog as an implicit import.
- A stateful program SHALL declare a library of at least 25 components, a component with state
  that renders some of its content only when its state holds a given value, and a lifecycle of at
  least 30 batches.

Every runtime that passes the corpus SHALL give the recorded results and operation counts for
these programs as it does for the others. The sweep that overwrites every cell of an image MAY
leave out a program that has an image above a size the corpus states; every image of such a
program SHALL still be cut at every four-byte boundary and refused.

#### Scenario: A large program is held to its recorded results
- **WHEN** a runtime evaluates the catalog program's entrypoint or drives the stateful program's
  lifecycle
- **THEN** it SHALL give the recorded result, rendered output and effects
- **AND** a runtime that offers a budget SHALL use the recorded number of operations for each

#### Scenario: The two runtimes agree on a large program
- **WHEN** the Rust runtime and the TypeScript runtime each run the stateful program's lifecycle
- **THEN** each of the 30 batches SHALL render the same output, with the same handler tokens, for
  the same number of operations

#### Scenario: A large program is cut but not swept cell by cell
- **WHEN** a program has an image larger than the size the corpus states
- **THEN** every truncation of each of its images at a four-byte boundary SHALL be refused with a
  diagnostic
- **AND** the test that overwrites every cell SHALL leave the program out and say that it did
