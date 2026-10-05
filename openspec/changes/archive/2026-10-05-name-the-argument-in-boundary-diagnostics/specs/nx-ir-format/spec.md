## MODIFIED Requirements

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
case SHALL use only arguments that every supported runtime treats alike, spelled alike in each:
each accepts them and gives the same result, or, for a case the program marks as one that fails,
each fails the call with the same diagnostic. The kinds of input the corpus cannot express
(arguments by name, content, a state the host passes in and a state patch) SHALL be checked in
each runtime's own tests against the sizes the format's documentation works out, the same numbers
in every runtime.

A corpus program MAY mark a case as one that fails. The expected files SHALL then hold, in place
of the case's result, the code of the diagnostic the call fails with and the argument the
diagnostic names, or that it names none, as the Rust IR runtime reports them, and SHALL hold no
operation count, no failure under a smaller budget and no input size for the case: those say what
a call that succeeds costs. Every supported runtime SHALL evaluate each such case from the image
with its debug section and from the one without, and SHALL check that the call fails with the
recorded code and names the recorded argument, or none where none is recorded. A case marked as
one that fails and that succeeds, and a case not so marked that fails, SHALL fail the corpus
tests, the regeneration command included, so that a case never turns from a result into a recorded
failure without its program saying so. The corpus SHALL hold a failing case for each of: a value
of the wrong type; a record with a wrong field three levels down; a required parameter given no
argument; a `Function` record that names no function; a record that fits and whose field default
divides by zero; arguments that fit followed by a division by zero in the body; a record that
fits and whose field default fails with a code a failure in an argument can have; and a parameter
the call gives nothing for whose default fails with such a code. The first four SHALL be recorded
with the argument they name and the rest with none. The last two are what hold a runtime to
telling a default's failure from an argument's by where it was raised: a runtime that went by the
diagnostic's code alone would name an argument for them.

Wherever a runtime checks that a case fails under an operation budget below its count, or under an
input limit below its input size, it SHALL also check that the diagnostic names no argument. The
corpus SHALL hold a case whose recorded failure under a smaller budget is reached while its
argument is checked.

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

#### Scenario: A case that fails is recorded with its code and its argument
- **WHEN** a corpus program marks as failing a case that passes the string `five` to
  `let findPlans(teamSize:int): string`
- **THEN** the expected files SHALL record `nx-ir-boundary-type` and the argument `teamSize` for the
  case, and no result, operation count or input size
- **AND** every runtime SHALL fail the case when its diagnostic has another code, names another
  argument or names none

#### Scenario: A failure that is not in an argument is recorded with none
- **WHEN** a corpus program marks as failing a case whose arguments fit and whose function then
  divides by zero
- **THEN** the expected files SHALL record the diagnostic's code and that it names no argument
- **AND** every runtime SHALL fail the case when its diagnostic names one

#### Scenario: A default that fails with an argument's code is recorded with none
- **WHEN** a corpus program marks as failing a case that passes a record whose field default calls
  a function value the record holds without an argument that function requires, or a case that
  leaves out a parameter whose default makes such a call
- **THEN** the expected files SHALL record `nx-ir-arguments` and that the diagnostic names no
  argument
- **AND** every runtime SHALL fail the case when its diagnostic names the parameter the record was
  passed for, or the parameter that was left out

#### Scenario: A case fails only where its program says so
- **WHEN** a case that is not marked as one that fails is refused by the Rust runtime, or a case
  that is marked succeeds
- **THEN** the corpus tests SHALL fail naming the program and the case, and the regeneration
  command SHALL NOT write expected files for it

#### Scenario: A limit reached while an argument is checked names no argument
- **WHEN** a runtime evaluates a corpus case under a recorded budget that runs out while the case's
  argument is checked, or under an input limit one less than its recorded input size
- **THEN** the call SHALL fail with `nx-ir-resource-limit` and the diagnostic SHALL name no argument
