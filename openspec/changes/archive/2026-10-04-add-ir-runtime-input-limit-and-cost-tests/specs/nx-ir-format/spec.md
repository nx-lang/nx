## ADDED Requirements

### Requirement: A call's host input has a size
The input of one call of an evaluation API SHALL have a size that depends on the values the host
supplies alone, so that every runtime measures the same input alike. The size SHALL be the sum of
the sizes of those values, each measured as the cost model measures a value written for the host:
one for each value, a sequence, the empty value and a `null` included, and one more for every 64
UTF-16 code units, rounded down, of a string, of a record's type name and of each field name. The
values a host supplies are: each positional argument of a function; the arguments of a call by
name, measured together as one record; the function record such a call names; the props of a
component, measured as one record; each content item; a state the host passes in, measured as one
record; each entry of a dispatched batch; and a state patch, measured as one record. A component
instance, and the parent instance a host names when it initializes a child, SHALL NOT be input:
the runtime produced them. A helper that takes no runtime options and evaluates no program code,
as the update helpers do, has no input in this sense. The size SHALL be measured before any value
is checked against a type, and SHALL NOT depend on the types the values reach.

Where a host can spell one input in more than one way, or the forms two runtimes are given differ,
the size SHALL be the same:

- An integer is one value, a 64-bit integer the Rust runtime is given included. The record
  canonical JSON spells an integer outside the safe range with,
  `{ "$type": "nx.int", "value": "<digits>" }`, is a record to every runtime where a host passes
  it, and is measured as one: the record, its string, and the string's length.
- The arguments of a call by name and the props of a component, when the host leaves them out,
  are measured as an empty record: their size is one. A state the host does not pass is not input.
- A member named `$type` that holds a string is the type name of the record or the map it is in,
  and counts its length alone. A `$type` member that holds anything else is a field like any other.
- An element of the list of positional arguments that is undefined, or a hole in that list, is
  the empty value: its size is one. A parameter the list does not reach is not input.
- A value that is not a canonical value is one value and is not read further: in JavaScript,
  anything but `null`, a boolean, a number, a string, an array or a plain object, and an object
  the runtime can tell it made for its own use. An object that only carries the marking of one,
  as a value read from JSON can, is a plain object and is measured as one. What a runtime does
  with a value that is not canonical afterwards is unchanged.

A runtime MAY offer the host a limit on the input size. A runtime that does SHALL refuse a call
whose input is larger than the limit before it evaluates anything, and SHALL stop measuring as
soon as the size passes the limit. For input that is plain data, the work of measuring SHALL then
be bounded by the limit and, at most once in a call, by the number of members of the one record at
which the limit is passed, whose names a runtime may have to list before it reads any of them. The
text a runtime reads in order to refuse a call SHALL be bounded by a fixed multiple of the limit:
a string, a type name or a field name far longer than the limit allows SHALL be refused from its
length, without its contents being read. The limit SHALL be separate from the operation budget: measuring
the input SHALL charge no operation, and a limit SHALL NOT change what an evaluation costs.

#### Scenario: A list is its items and itself
- **WHEN** a host passes a list of 1,000 integers as the one argument of a function
- **THEN** the input size of the call SHALL be 1,001

#### Scenario: Props are measured as one record
- **WHEN** a host initializes a component with the props `{ "title": "Home", "count": 3 }`
- **THEN** the input size of the call SHALL be three: the record and its two values

#### Scenario: Props left out are an empty record
- **WHEN** one host initializes a component with no props and another with the props `{}`
- **THEN** the input size of each call SHALL be one

#### Scenario: Text costs its length
- **WHEN** a host passes a string of 6,400 UTF-16 code units
- **THEN** its size SHALL be 101: one for the value and 100 for its length

#### Scenario: A name costs its length
- **WHEN** a host passes an object whose one field name is 1,048,576 UTF-16 code units long and whose value is a number
- **THEN** its size SHALL be 16,386: the object, 16,384 for the name, and the number

#### Scenario: The JSON form of a wide integer is measured as the record it is
- **WHEN** a host passes `{ "$type": "nx.int", "value": "9007199254740993" }`
- **THEN** its size SHALL be two in every runtime: the record and its string
- **AND** a list of two of them SHALL have the size five
- **AND** the same integer passed to the Rust runtime as a 64-bit integer SHALL have the size one

#### Scenario: Text in the form of a wide integer is counted
- **WHEN** a host passes `{ "$type": "nx.int", "value": <a string of 1,048,576 UTF-16 code units> }`
- **THEN** its size SHALL be 16,386 in every runtime

#### Scenario: An object that only looks like the runtime's own is measured
- **WHEN** a JavaScript host passes an object read from JSON that carries the marking the runtime gives an action handler and holds a string of 1,048,576 code units
- **THEN** its size SHALL be more than 16,384: it is a plain object, and what it holds is measured

#### Scenario: A type name among props is a type name
- **WHEN** a host passes the props `{ "$type": "Card", "title": "Home" }`
- **THEN** the input size SHALL be two: the record and the title

#### Scenario: An instance is not input
- **WHEN** a host dispatches a batch of one entry against an instance whose state holds 50,000 values
- **THEN** the input size of the call SHALL be the size of the one entry

#### Scenario: A limit costs no operations
- **WHEN** an evaluation costs `n` operations with no input limit
- **THEN** it SHALL cost `n` operations under any input limit its input fits

#### Scenario: A long string is refused without being read
- **WHEN** a host passes a string of 64 million code units under an input limit of 1,000
- **THEN** the call SHALL be refused for its input
- **AND** the runtime SHALL NOT have read the string's contents to refuse it

### Requirement: A runtime that reports what a call used reports the cost model's numbers
A runtime MAY report to the host what one call of an evaluation API used. A runtime that does SHALL
report the cost model's numbers and no others. The operations it reports for a call that succeeds
SHALL be the call's operation count: the least budget under which the call succeeds. For a call
that fails, they SHALL be the operations charged before the failure; a charge the budget refused
is not among them, so the number is no more than the budget and is the same in every runtime. The input size it reports SHALL be the input size of the call as this format defines it.
A runtime SHALL report operations only for a call that ran under an operation budget and an input
size only for a call that ran under an input limit, so that a host that sets neither pays for no
counting, and reporting SHALL NOT change what a call costs or returns. A runtime that offers an
input limit SHALL also let a host measure one value as the limit measures it, without a call, so
that a host can hold a part of what it passes to a number of its own. A value measured alone has
the size it adds to a call that is given it as one of the values a host supplies; a list of
positional arguments, of content or of batch entries, measured as one value, is one more than its
entries add to a call, since a call counts the entries and not the list.

#### Scenario: The operations reported are the count
- **WHEN** a call costs `n` operations and is run under a budget of `n` or more with a report asked for
- **THEN** the report SHALL say `n` operations in every runtime

#### Scenario: A failed call reports what it used
- **WHEN** a call fails for its operation budget
- **THEN** the report SHALL say no more operations than the budget

#### Scenario: A refused charge is not reported
- **WHEN** a call under a budget of 10 has been charged 5 operations and its next charge, 31 for a concatenation, is refused
- **THEN** the report SHALL say 5 operations in every runtime

#### Scenario: Nothing is counted for the report alone
- **WHEN** a call runs with a report asked for and no operation budget
- **THEN** the report SHALL carry no operations
- **AND** the call SHALL do no counting it would not have done without the report

#### Scenario: One value is measured as the limit measures it
- **WHEN** a host measures `{ "title": "Home", "count": 3 }` by itself
- **THEN** the size SHALL be three, the size that value has as the props of a call

### Requirement: The conformance corpus evaluates entrypoints with arguments
A corpus program MAY name, for a function entrypoint, the arguments to evaluate it with, written as
canonical values, together with a case name, and MAY name several cases of one function. An
entrypoint with arguments SHALL have a case name; one without SHALL have none and keeps the key it
has today. The expected results, the operation counts and the recorded failures SHALL be kept for
each case, keyed by the entrypoint and the case name. A corpus program MAY ask for input sizes to
be recorded; the input size of each of its cases, and of each of its lifecycles' initialization
and batches, SHALL then be recorded beside the operation counts. Every supported runtime SHALL
evaluate each case with its arguments and check its result, its operation count and its recorded
failures as it does for an entrypoint without arguments, and every runtime that offers an input
limit SHALL check each recorded input size: the call SHALL proceed under a limit equal to the size
and SHALL be refused under a limit one less. The regeneration command that writes the expected
results SHALL write these in the same run.

The corpus SHALL hold cases for the host values an entrypoint without arguments cannot supply: a
list at a parameter with a sequence type, a value at `object`, a string of at least 64 UTF-16 code
units, an object with a field name of at least 64 code units, two objects of one type that each
hold a name the other does not, a list bound to a content parameter, two lists that differ in
their first item, and the record canonical JSON spells an integer outside the safe range with, at
`object`; and a lifecycle with props and batches whose input sizes are recorded. A corpus
case SHALL use only arguments that every supported runtime accepts, spelled alike in each, and
for which each gives the same result. The kinds of input the corpus cannot express (arguments by
name, content, a state the host passes in and a state patch) SHALL be checked in each runtime's
own tests against the sizes the format's documentation works out, the same numbers in every
runtime.

#### Scenario: A case is evaluated with its arguments
- **WHEN** a corpus program names an entrypoint with the arguments `[[1, 2, 3]]` and the case name `three`
- **THEN** every runtime SHALL evaluate the function with that list as its one argument
- **AND** SHALL compare the result, the operation count and the recorded failures kept under the entrypoint and `three`

#### Scenario: Two cases of one function are kept apart
- **WHEN** a corpus program names one function twice with different arguments and case names
- **THEN** the expected results SHALL hold one entry for each case
- **AND** a difference in either SHALL fail naming the program, the entrypoint and the case

#### Scenario: Arguments without a case name are refused
- **WHEN** a corpus program names an entrypoint with arguments and no case name
- **THEN** the corpus tests SHALL fail naming the program and the entrypoint

#### Scenario: A recorded input size is exact
- **WHEN** a runtime evaluates a corpus case, or runs a step of a corpus lifecycle, under an input limit equal to its recorded input size
- **THEN** the call SHALL yield the recorded result
- **AND** the same call under a limit one less SHALL fail with `nx-ir-resource-limit` naming the input limit

#### Scenario: What existing programs expect is unchanged
- **WHEN** the corpus is regenerated after this requirement is implemented
- **THEN** no expected artifact, result, operation count or recorded failure of an existing corpus program SHALL change

### Requirement: Evaluation cost is validated against generated host values
The repository SHALL hold a deterministic generator of host values and a fixed library of NX
functions and components, the probes, that between them perform on a host value at least these
operations: holding it, passing it through a typed and an untyped parameter, binding it to a
content parameter, comparing it by `eq`, by a pattern and by `diff`, placing it in a sequence,
concatenating it and converting it to text, constructing a record that holds it as a property and
as content, capturing it in an action handler and comparing two such handlers, applying and
merging an update record that holds it, testing its presence, reading a member of it and iterating
over it where it has a type that allows that, receiving it as a prop, as an action's payload and
as an argument by name, storing it in component state and patching that state, and returning it.
An operation a runtime performs on a value that has no probe SHALL be listed with the reason. The
generator SHALL produce, from a seed, values that vary in the dimensions a host controls: the
number of items of a list, the number of fields of an object, the length of a string and of a
name, nesting, empty values and `null`s, integers outside the safe range, lengths just under
and just over 64 UTF-16 code units, and the names a runtime gives a meaning to (the type names of
a wide integer, a function value, an action handler, an action handler invocation and an update
record, and a runtime's internal markings), each with small and with large contents, both in
the members the form defines and in members it does not, and spelled the same for every runtime. The same seed SHALL produce the same cases on every run.

The generator SHALL NOT give a probe a value that the format documents one runtime as refusing
where another accepts it, as a `null` that is a whole argument at `object` and the `nx.int`
record at a typed integer, whatever its digits, are. For every other generated case, every runtime that offers
an operation budget SHALL be run on the same case, and the runtimes SHALL agree on the operation
count and the input size at both of its scales, inputs for which the format documents different
values included, and, at its base scale:

- on the result, except where the format documents that they compute different values for an
  input both accept;
- on the declaration and the span of the failure under a budget below the count, and on the
  operations reported for the failed call: under every such budget when the count is at most
  200, and otherwise under at least three, one of them the count less one.

In each runtime, the result under a budget equal to the count SHALL be the result under none. A
case every runtime refuses with no budget set has no count, and the runtimes SHALL then agree on
the code of the diagnostic. A disagreement SHALL fail the test naming the seed and the case.

Each case SHALL also be run at two scales, to compare the work a runtime does with what it
charges: the second with a host value eight times the size of the first and, where the probe
repeats a step, eight times the repeats. A case SHALL be reported when, between the two scales, a
runtime's time grows by more than twice the factor by which the sum of its operation count and
its input size grows, and its time at the larger scale is past a floor set for that runtime from
the noise measured in it. A time SHALL be the least of several runs. The scales of the report
SHALL be large enough that a step of each kind the report is for, re-introduced in a runtime, is
past that runtime's floor. The report SHALL be made in a run that does not block a merge.

A step these tests show to be uncharged SHALL be recorded as a known finding: one concrete case
that shows it, a description of the generated cases it covers, and where it is tracked. The
blocking tests SHALL skip the generated cases a finding covers, SHALL run the finding's own case,
and SHALL fail when that case no longer shows the step, so that the list holds only what is still
true. A finding that only the time report shows is listed for that report and is exempt from
that check, since time cannot be required to reproduce.

#### Scenario: Two runtimes agree on a generated case
- **WHEN** a generated case is evaluated in the TypeScript runtime and in the Rust runtime
- **THEN** both SHALL give the same result and cost the same number of operations
- **AND** under a budget below that number both SHALL fail naming the same declaration and, with a debug section, the same span

#### Scenario: A case both runtimes refuse is compared by its diagnostic
- **WHEN** a generated case passes a record named `ActionHandlerInvocation` whose token names no handler, and both runtimes refuse it with no budget set
- **THEN** the two diagnostics SHALL have the same code
- **AND** no operation count SHALL be sought for the case

#### Scenario: A result does not depend on the budget
- **WHEN** a generated case is evaluated in one runtime with no budget and with a budget equal to its count
- **THEN** both evaluations SHALL give the same result

#### Scenario: A missing charge is caught
- **WHEN** one runtime is changed so that it no longer charges an item bound to a content parameter
- **THEN** the differential test SHALL fail naming the seed and a case that binds a list to one

#### Scenario: A failure can be reproduced
- **WHEN** the differential test fails and is run again with the seed it named
- **THEN** it SHALL fail on the same case

#### Scenario: A step that copies without charging is reported
- **WHEN** a runtime copies a list of `n` items at every call of a function for a fixed number of operations, and the case calls it `r` times
- **THEN** the case SHALL be reported: between the two scales its count and input size grow about eightfold and its time about sixty-four-fold

#### Scenario: A walk repeated within one call is reported
- **WHEN** a runtime walks a component's whole state once for every entry of a batch and charges for one walk
- **THEN** the case that dispatches the batch SHALL be reported

#### Scenario: Honest work is not reported
- **WHEN** a runtime converts a host value once on the way in and charges nothing for it
- **THEN** the case SHALL NOT be reported: its time grows no faster than its input size

#### Scenario: A finding that is fixed leaves the list
- **WHEN** a step recorded as a known finding is charged
- **THEN** the blocking tests SHALL fail on the finding's own case until the finding is removed from the list
