## ADDED Requirements

### Requirement: Rust runtime limits the size of host input
The runtime options every evaluation API accepts SHALL include an input limit, absent by default,
measured as `nx-ir-format` defines the input size of a call. An absent limit SHALL be unlimited.
The limit SHALL cover every host value passed to a public method that takes runtime options, the
same values the TypeScript runtime measures; a component instance SHALL NOT be measured, and
restoring a stored instance, which takes no options, SHALL keep the bounds it has. A call whose
input is larger than the limit SHALL fail with `nx-ir-resource-limit` naming the limit
`maxInputSize` and its value, before the program is looked at, before any value is checked against
a type and before any node is evaluated, having stopped measuring as soon as the size passed the
limit, within the bound `nx-ir-format` sets. Measuring the input SHALL charge nothing to the
operation budget.

The Rust runtime SHALL measure the size the TypeScript runtime measures for the same input, so
that the input limit refuses the same calls in both. The input limit SHALL be applied before the
runtime's bound on the nesting of a host value: input that passes both SHALL be reported for its
size. Input within the limit that nests more deeply than that bound SHALL still be refused for its
nesting, which the TypeScript runtime does not bound.

#### Scenario: Input over the limit is refused before anything runs
- **WHEN** a host evaluates a function with a list of 20,000 integers under an input limit of 1,000
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose limit has the name `maxInputSize` and the value `1000`
- **AND** no node SHALL have been evaluated

#### Scenario: The limit is exact and agrees with the TypeScript runtime
- **WHEN** the input of a call has the size `n` in the TypeScript runtime
- **THEN** the Rust runtime SHALL accept it under an input limit of `n`
- **AND** SHALL refuse it under a limit of `n - 1`

#### Scenario: An absent limit is unlimited
- **WHEN** a host evaluates a function with a list of one million integers under the default options
- **THEN** evaluation SHALL proceed as it did before the option existed

#### Scenario: A batch is measured whole before any of it runs
- **WHEN** a host dispatches a batch whose first entry is small and whose second is larger than the limit, under an operation budget of zero
- **THEN** the dispatch SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`, not the budget
- **AND** no handler SHALL have run

#### Scenario: Oversized input is reported before any other fault of the call
- **WHEN** a host evaluates a component with props of the wrong type and a state larger than the limit
- **THEN** the call SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`

#### Scenario: Size is reported before nesting
- **WHEN** a host passes a value nested 2,000 deep under an input limit of 1,000
- **THEN** the call SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`
- **AND** a value nested 300 deep, of size 301, under the same limit SHALL fail naming the nesting bound

#### Scenario: A refused dispatch leaves the instance usable
- **WHEN** a host dispatches a batch larger than the input limit
- **THEN** dispatch SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`
- **AND** the instance given SHALL dispatch a later, smaller batch successfully

### Requirement: Rust runtime reports what a call used and exports the input measure
The runtime options SHALL be able to carry a usage report that the host shares with the runtime.
When a call given one ends, whether it succeeds or fails, the report SHALL hold the operations the
call used if an operation budget was set and none otherwise, and the input size of the call if an
input limit was set and the input was within it and none otherwise, replacing what an earlier call
left there. The numbers SHALL be those `nx-ir-format` defines and SHALL equal what the TypeScript
runtime reports for the same call, for a call that fails as for one that succeeds: a charge the
budget refused SHALL NOT be counted. With no report given the runtime SHALL do nothing for one, and
with no budget it SHALL count nothing for one.

The runtime SHALL export a function that returns the size of one host value as `nx-ir-format`
defines it and as the input limit measures it, and that, given a limit, stops as soon as the size
passes the limit and returns a number greater than it; and one that does the same for a map of
named values measured as one record, which is the form props, a state, a patch and arguments by
name are passed in, so that a host measures one without copying it.

#### Scenario: A successful call reports its count
- **WHEN** a host evaluates a function that costs 29 operations under a budget of 100,000 with a usage report
- **THEN** the report SHALL hold 29 operations

#### Scenario: A failed call reports what it used
- **WHEN** a host evaluates a function under a budget of 10 with a usage report and it fails for the budget
- **THEN** the report SHALL hold at most 10 operations

#### Scenario: A refused charge is not reported
- **WHEN** a call under a budget of 10 has been charged 5 operations and its next charge, 31 for a concatenation, is refused
- **THEN** the report SHALL hold 5 operations

#### Scenario: A map is measured as the record it is passed as
- **WHEN** a host measures the props map `{ "title": "Home", "count": 3 }` with the exported function for maps
- **THEN** the size SHALL be three, as the call that takes those props reports

#### Scenario: The two runtimes report alike
- **WHEN** a corpus case is evaluated in both runtimes under a budget and an input limit it fits, with a usage report
- **THEN** both reports SHALL hold the recorded operation count and the recorded input size

#### Scenario: A value measured alone has the size it has in a call
- **WHEN** a host measures a value with the exported function and then passes it as the one argument of a function under an input limit with a usage report
- **THEN** the two sizes SHALL be equal

### Requirement: Rust runtime allocation is validated against its operation count
The repository's tests SHALL measure, for the Rust runtime and every generated case of
`nx-ir-format`'s cost validation at both of its scales, the bytes the runtime asks its allocator
for during the call. The tests SHALL fail when that number exceeds a fixed multiple of the call's
operation count plus its input size, and SHALL fail when the bytes for each unit of that sum at
the larger scale are more than twice what they are at the smaller. The measure SHALL be of bytes
requested, the check SHALL block a merge, and the generated cases a known finding covers SHALL be
exempt until it is fixed. A step that copies the input a fixed number of times in a call is within
these bounds and is not what the check looks for.

#### Scenario: Allocation follows the count
- **WHEN** a generated case is evaluated at its two scales
- **THEN** at each scale the bytes allocated SHALL be within the fixed multiple of the operation count plus the input size
- **AND** the bytes for each unit of that sum SHALL at most double between the scales

#### Scenario: A copy that is not charged is caught
- **WHEN** the runtime is changed so that a value written for the host is no longer charged
- **THEN** the allocation check SHALL fail naming the seed and a case that returns a value many times

#### Scenario: The check is repeatable
- **WHEN** the allocation check is run twice in one build
- **THEN** it SHALL measure the same number of bytes for the same case
- **AND** every case SHALL be within the bounds in an unoptimized build and in an optimized one
