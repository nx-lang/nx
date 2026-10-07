## ADDED Requirements

### Requirement: TypeScript runtime carries a phase-level performance harness
The runtime package SHALL carry a benchmark harness, kept out of the published package, that times
each call a host makes on the conformance corpus's large programs: loading the runtime module,
preparing a program's images, linking, evaluating a zero-argument function, initializing a
component, initializing one from a stored state, evaluating one with a state, and dispatching an
action. It SHALL also time calls that take input: a call given at least 10,000 records as plain
data, as the same data with a member set to `undefined`, and as data nested deeper than the
runtime's check for plain data follows; and a call of a function by name with arguments under an
operation budget and an input limit.

The harness SHALL read the committed corpus images and SHALL NOT need the compiler. For each phase
it SHALL report the time of the first execution in a fresh isolate, the warm median and 95th
percentile with no limits set and with an operation budget and an input limit set, and the
operations and the input size the runtime's usage report gives, as a readable table and as JSON.

#### Scenario: A run reports every phase
- **WHEN** a contributor runs the harness
- **THEN** the report SHALL list every phase of each workload with its cold time, its warm median
  and 95th percentile with and without limits, its operations and its input size
- **AND** the same numbers SHALL be written as JSON

#### Scenario: The count in the report is the corpus's
- **WHEN** the harness runs a phase the corpus records an operation count for
- **THEN** the operations it reports SHALL equal the recorded count

#### Scenario: The harness needs no compiler
- **WHEN** the harness runs in a checkout where the compiler has not been built
- **THEN** it SHALL run every workload from the committed images

#### Scenario: The harness is not published
- **WHEN** the runtime package is packed
- **THEN** the tarball SHALL hold no file of the harness

### Requirement: The harness's workload steps run in any JavaScript engine
The steps the harness times SHALL be built by a module that imports nothing, reads no clock and
uses no Node API. Given a runtime module, a workload's images and its script, it SHALL return the
steps by phase, each of which makes its call once and returns what the usage report gave. So a
host can run the same steps in its own engine against its own build of the runtime and time them
as that engine allows, including an engine whose clock does not advance while code runs.

#### Scenario: A host runs the steps against its own runtime
- **WHEN** a host imports the module into an engine without Node APIs and passes its bundled
  runtime and the workload's images
- **THEN** each step SHALL run and return the operations and the input size of its call

#### Scenario: Two builds of the runtime run the same steps
- **WHEN** the module is given two different builds of the runtime in turn
- **THEN** it SHALL build the same steps for each, and neither SHALL affect the other

### Requirement: A change's effect on runtime speed is measured against the revision it is based on
The package SHALL provide a comparison that measures a base revision and the working tree on the
same machine in one run: each revision's committed runtime build against that revision's committed
corpus images, run alternately. The base SHALL default to the merge base with the main branch and
SHALL be nameable. No timing recorded on another machine SHALL be compared against.

For each phase the comparison SHALL report both warm medians, their ratio, how the ratio varied
between rounds and both operation counts. It SHALL name a phase as slower when the working tree is
slower than the base by more than a set fraction over the rounds, judged by the median of the
rounds' ratios and by most rounds agreeing, and SHALL list a phase that is faster by the same
measure. It SHALL compare in the same way each phase's first execution in a fresh isolate and the
time the runtime module takes to load in one. It SHALL report a phase as not comparable, and not
as slower, when the workload differs between the revisions, when the base has no such workload,
when the base runtime lacks a function the phase calls, or when the phase fails on either side.

Continuous integration SHALL run the comparison on a pull request, against the revision the
change is based on, and on a push to the main branch, against the last release, as a job that
reports and does not block a merge.

#### Scenario: A slower phase is named
- **WHEN** a change makes dispatch on the stateful program slower than the base revision by more
  than the allowed fraction
- **THEN** the comparison SHALL name that workload and phase with both medians and their ratio
- **AND** it SHALL name no phase the change did not slow

#### Scenario: A revision compared with itself names nothing
- **WHEN** the comparison is run with the base set to the working tree's own revision
- **THEN** it SHALL name no phase as slower or faster

#### Scenario: A change in operations is shown exactly
- **WHEN** a change makes a phase cost more operations than it does at the base revision
- **THEN** the report SHALL show both counts for that phase

#### Scenario: A workload that changed is not compared
- **WHEN** a change edits a workload's source or adds a workload the base revision does not have
- **THEN** the comparison SHALL report its phases as not comparable and SHALL succeed

#### Scenario: One wild round does not name a phase
- **WHEN** one round of an unchanged phase reads twice as slow as the base and the others agree
  with it
- **THEN** the comparison SHALL NOT name the phase

#### Scenario: A slower load is named
- **WHEN** a change makes the runtime module take longer to load by more than the allowed fraction
- **THEN** the comparison SHALL name the load time with both medians and their ratio

#### Scenario: Slowdowns that add up are named against the last release
- **WHEN** a push to the main branch leaves a phase slower than it was in the last release by more
  than the allowed fraction
- **THEN** the job SHALL name that phase in its report

#### Scenario: The job does not block a merge
- **WHEN** the comparison names a slower phase on a pull request
- **THEN** the job SHALL show the report on the pull request
- **AND** the pull request SHALL remain mergeable
