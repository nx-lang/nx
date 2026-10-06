## ADDED Requirements

### Requirement: TypeScript runtime limits the size of host input
The runtime options every evaluation API accepts SHALL include `maxInputSize`, the largest input
size one call may be given, measured as `nx-ir-format` defines the input size of a call. When the
option is absent the input SHALL be unlimited and the runtime SHALL NOT measure it, so a host that
does not set it sees no change. A value that is not a non-negative safe integer SHALL be refused
with a diagnostic naming the option before anything is measured or evaluated.

The limit SHALL cover what the host passes to one call: the arguments of `evaluateFunction`; the
function record and the arguments of `callFunction`; the props and the content of
`constructComponentDescriptor`; the props of `initializeComponent` and the state its options carry;
the props and the state of `evaluateComponent`; the batch of `dispatchComponentActions`; the state
of `normalizeComponentState`; and the current state and the patch of `applyComponentStatePatch`. A
component instance, and the parent instance of `initializeComponent`, SHALL NOT be measured.

A call whose input is larger than the limit SHALL fail with `nx-ir-resource-limit` whose `limit`
is `{ name: "maxInputSize", value }` with the limit the host set, before the program is looked
at, before any value is checked against a type and before any node is evaluated. The runtime SHALL
stop measuring as soon as the size passes the limit, within the bound `nx-ir-format` sets, and
SHALL measure without recursion, so a value that is too large, that nests deeply, or that holds
itself SHALL be refused by the limit and SHALL NOT exhaust the engine's stack or memory first.
Measuring the input SHALL charge nothing to `maxOperations`. A refused dispatch SHALL leave the
instance it was given usable.

#### Scenario: Input over the limit is refused before anything runs
- **WHEN** a host evaluates a function with a list of 20,000 integers under `maxInputSize: 1000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` is `{ name: "maxInputSize", value: 1000 }`
- **AND** no node SHALL have been evaluated and no argument checked against its parameter's type

#### Scenario: Input at the limit is accepted
- **WHEN** the input of a call has the size `n`
- **THEN** the call SHALL proceed under `maxInputSize: n`
- **AND** SHALL be refused under `maxInputSize: n - 1`

#### Scenario: An absent limit measures nothing
- **WHEN** a host evaluates a function with a list of one million integers under options that do not set `maxInputSize`
- **THEN** evaluation SHALL proceed as it did before the option existed

#### Scenario: A value that holds itself is refused by the limit
- **WHEN** a host passes an object one of whose fields is the object itself, under `maxInputSize: 1000`
- **THEN** the call SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`
- **AND** SHALL NOT fail for a limit of the JavaScript engine

#### Scenario: A deeply nested value is refused by the limit
- **WHEN** a host passes an array nested 100,000 deep under `maxInputSize: 1000`
- **THEN** the call SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`

#### Scenario: Every kind of input is measured
- **WHEN** a host passes more than the limit as props, as content, as a state, as a batch, or as a state patch
- **THEN** the call SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`

#### Scenario: A batch is measured whole before any of it runs
- **WHEN** a host dispatches a batch whose first entry is small and whose second is larger than the limit
- **THEN** the dispatch SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`
- **AND** no handler SHALL have run

#### Scenario: Oversized input is reported before any other fault of the call
- **WHEN** a host evaluates a component with props of the wrong type and a state larger than the limit, or names an entrypoint the program lacks with arguments larger than the limit
- **THEN** each call SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxInputSize`

#### Scenario: An instance is not measured
- **WHEN** a host dispatches a batch of one small entry against an instance whose state holds 50,000 values, under `maxInputSize: 100`
- **THEN** the dispatch SHALL proceed

#### Scenario: The limit and the budget are told apart
- **WHEN** one call fails for its input and another for its operation budget
- **THEN** the two diagnostics SHALL have the same code and different `limit.name` values

#### Scenario: A limit that is not a size is refused
- **WHEN** a host evaluates a function under `maxInputSize: NaN`, `maxInputSize: -1` or `maxInputSize: 1.5`
- **THEN** each call SHALL fail with a diagnostic naming `maxInputSize`
- **AND** nothing SHALL have been measured or evaluated

### Requirement: TypeScript runtime reports what a call used
The runtime options every evaluation API accepts SHALL include `usage`, an object of the host's
that the runtime writes to. On every call given one, the runtime SHALL first remove `operations`
and `inputSize` from it, and when the call ends, whether it returns or throws, SHALL set
`operations` to the operations the call used if `maxOperations` was set, and `inputSize` to the
input size of the call if `maxInputSize` was set and the input was within it. The numbers SHALL be
those `nx-ir-format` defines; for a call that fails for its budget, the charge that was refused
SHALL NOT be counted. With `usage` absent the runtime SHALL do nothing for it, and with
`maxOperations` absent it SHALL count nothing for it. The object SHALL NOT be measured as input.

A `usage` that is not an object the runtime can write to SHALL be refused with `nx-ir-options`
before anything is measured or evaluated: a value that is no object, or an object to which the
runtime cannot write both members and remove them again at the start of the call, as a frozen
object or one that cannot be extended. The write at the end of a call SHALL
NOT change the call's outcome: if it fails, the call SHALL still return what it returned or throw
what it threw. One object given to calls that overlap holds the numbers of whichever ended last;
a host that wants each call's numbers gives each call its own.

#### Scenario: A successful call reports its count
- **WHEN** a host evaluates a function that costs 29 operations under `{ maxOperations: 100000, usage }`
- **THEN** `usage.operations` SHALL be 29 when the call returns

#### Scenario: A call that fails reports what it used
- **WHEN** a host evaluates a function under `{ maxOperations: 10, usage }` and it fails for the budget
- **THEN** the call SHALL throw as it does without `usage`
- **AND** `usage.operations` SHALL be at most 10

#### Scenario: A refused charge is not reported
- **WHEN** a call under `{ maxOperations: 10, usage }` has been charged 5 operations and its next charge, 31 for a concatenation, is refused
- **THEN** `usage.operations` SHALL be 5, as the Rust runtime reports

#### Scenario: A sink that cannot be written is refused
- **WHEN** a host passes `usage: Object.freeze({})`, `usage: Object.preventExtensions({})`, or `usage: 5`
- **THEN** the call SHALL fail with `nx-ir-options` naming `usage`
- **AND** nothing SHALL have been measured or evaluated

#### Scenario: Writing the report never replaces the outcome
- **WHEN** a call fails with a diagnostic and the write to `usage` at its end throws
- **THEN** the call SHALL throw the diagnostic it failed with

#### Scenario: A dispatch reports one number for the batch
- **WHEN** a host dispatches a batch of three entries under a budget with `usage`
- **THEN** `usage.operations` SHALL be what the three handlers and the render after them used together

#### Scenario: The input size is reported when it is measured
- **WHEN** a host evaluates a function with a list of 1,000 integers under `{ maxInputSize: 5000, usage }`
- **THEN** `usage.inputSize` SHALL be 1,001
- **AND** under options that do not set `maxInputSize` it SHALL be absent

#### Scenario: A report does not carry over
- **WHEN** a host passes one `usage` object to a call under a budget and then to a call with none
- **THEN** after the second call `usage.operations` SHALL be absent

### Requirement: TypeScript runtime exports the input measure
The runtime SHALL export `measureInputSize(value, limit?)`, which returns the size of one value as
`nx-ir-format` defines it and as `maxInputSize` measures it. Given a limit, it SHALL stop as soon
as the size passes the limit and return a number greater than the limit, within the bound on work
that `nx-ir-format` sets for the limit. It SHALL evaluate nothing and need no program. An object
measured this way is measured as one record, which is how the props, the state, the patch and the
arguments by name of a call are measured; an array is measured as the list it is, one more than
its items add to a call that takes them as its positional arguments, content or batch.

#### Scenario: A value measured alone has the size it has in a call
- **WHEN** a host measures a value with `measureInputSize` and then passes it as the one argument of a function under `{ maxInputSize, usage }`
- **THEN** the two sizes SHALL be equal

#### Scenario: Measuring stops at the limit
- **WHEN** a host measures a list of one million integers with a limit of 1,000
- **THEN** the result SHALL be greater than 1,000
- **AND** no more than the limit allows of the list SHALL have been read
