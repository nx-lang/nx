## MODIFIED Requirements

### Requirement: A function tool runs its function under the evaluation budget
A `function` tool's `execute` SHALL call the function its definition names through the IR runtime's
`callFunction`, with the model's input as arguments by parameter name and every context parameter
filled by the host. The input SHALL be a JSON object, and anything else SHALL fail with code
`invalid-input`. The IR runtime's boundary validation SHALL be what checks the input against the
parameter types; the package SHALL NOT need a JSON Schema validator. A key of the input that names
a context parameter SHALL be discarded before the call, so the model cannot supply one.
The package SHALL pass the host's context record as the record its own members make, whatever
built it, an instance of a host's class included, with `callId` set, and SHALL NOT copy or look
into what the record holds: reading a host value is the IR runtime's. A member that is
`undefined`, of the record or of a plain object at any depth below it, is therefore absent, as
the runtime reads one, and a value below the top level that is not a canonical value, an instance
of a class or a typed array for one, is refused by the runtime where it is found, as a failure in
the context argument. The host's own record SHALL NOT be changed. A record over `maxContextSize`,
one that holds itself and one nested however deeply SHALL fail with code `resource-limit` naming
`maxContextSize`, and SHALL NOT make `execute` reject.

Every call SHALL run with a finite operation budget. The IR runtime applies none of its own when
`maxOperations` is unset, so the package SHALL pass the host's runtime options when they set
`maxOperations` and SHALL otherwise set it to the package's default of 100,000. Each call SHALL
get the whole budget; the package SHALL NOT share a budget across calls.

The result of every call that reached the IR runtime, a success or a failure, SHALL carry `usage`:
`operations`, the operations the call used, and `inputSize`, the size of its input, as the IR
runtime reports them. The package SHALL give each call a usage sink of its own, so that calls
started together each carry their own numbers, and SHALL NOT write to a `usage` in the host's
runtime options. A failure the package finds before it calls the runtime SHALL carry no `usage`;
a failure the runtime finds, a boundary failure on the arguments included, SHALL carry it. For an
HTTP tool the numbers SHALL be those of the call of its arguments function, on the result of
`execute` and of `evaluateHttpArguments` alike, and on a failure that follows that call (a request
that cannot be built, a request that fails) as on a success. A tool run by a host executor or by the provider SHALL carry
none.

Every call SHALL run with its input bounded, in two parts, each measured by the package with the
IR runtime's `measureInputSize` before the function is called. The model's input SHALL be held to
`maxArgumentsSize`, the host's value or the package's default of 1,000, and input over it SHALL
fail with code `invalid-input` carrying `limit: { name: "maxArgumentsSize", value }`. The host's
context record, as it is passed with `callId` set, SHALL be held to `maxContextSize`, the host's
value or the package's default of 1,000, and a context over it SHALL fail with code `resource-limit`
carrying `limit: { name: "maxContextSize", value }`. Neither failure SHALL call the function. The
call SHALL then run with the runtime's `maxInputSize` set by the package to the sum of
`maxArgumentsSize`; of `maxContextSize` and of what the parameter's name costs as a field name,
one for every 64 UTF-16 code units, for each context parameter of the function; and of the size of
the function record the call names, so that a call within both limits is never refused by the
runtime's. A `maxInputSize` in the host's runtime options SHALL NOT be used for
these calls. A host SHALL NOT be able to turn either limit off through the package. The same SHALL
hold for the call of an HTTP tool's arguments function, through `execute` and through
`evaluateHttpArguments`.

A success SHALL carry the function's canonical result as the runtime returns it. A failure of
evaluation SHALL carry the runtime's diagnostics and one of these codes, chosen by what the
diagnostics say and never by their message text:

- `resource-limit` when any diagnostic is `nx-ir-resource-limit`, with the diagnostic's `limit`
  carried on the error.
- `invalid-context` when every diagnostic is a boundary failure, which is `nx-ir-arguments` or a
  code that begins `nx-ir-boundary-`, that names an argument, and at least one names a context
  parameter of the tool's function. The host filled that argument in, so the mistake is the
  host's and the model has nothing to correct.
- `invalid-input` when every diagnostic is a boundary failure that names an argument and none
  names a context parameter. The model sent that argument and can send it again.
- `evaluation-failed` otherwise. That includes a boundary failure that names no argument: it was
  found inside the function, in a default or in the result, not in what the call was given.

The package SHALL match the family of boundary codes and not a list of them, so that a check a
runtime adds at the boundary later, of a value against a constrained type for one, is reported
as input the model can correct with no change to the package, provided the runtime names the
argument as it does for every other boundary failure. A host SHALL NOT need to read IR diagnostic
codes to tell these apart.

#### Scenario: An optional context field set to undefined is absent
- **WHEN** the host passes the context record `{ $type: "ChatToolContext", conversationId: "conv_9", contactEmail: undefined }`
  to a tool whose function declares `context:ChatToolContext`, where `contactEmail` is optional
- **THEN** the call SHALL succeed as it does with the member left out

#### Scenario: A context record that holds itself is refused, not followed
- **WHEN** the host passes a context record one of whose members is the record itself, or a record
  nested twenty thousand levels deep
- **THEN** `execute` SHALL resolve to a failure with code `resource-limit` whose `limit` names
  `maxContextSize`, and SHALL NOT reject

#### Scenario: A function tool returns the function's result
- **WHEN** the model calls `find_plans` with `{ "teamSize": 5 }`
- **THEN** `execute` SHALL call `findPlans` with `teamSize` bound to `5` and `maxMonthlyPrice` absent
- **AND** SHALL resolve to a success whose output is the canonical `Plan` array

#### Scenario: A result carries what the call used
- **WHEN** the model calls `find_plans` with `{ "teamSize": 5 }` and the call costs `n` operations
- **THEN** the success SHALL carry `usage` whose `operations` is `n` and whose `inputSize` is the size of the call's input

#### Scenario: A failure after the call began carries what it used
- **WHEN** a tool's function loops beyond the budget
- **THEN** the failure SHALL carry `usage` whose `operations` is no more than the budget

#### Scenario: Calls started together carry their own numbers
- **WHEN** a host starts two tool calls that cost different numbers of operations and awaits both
- **THEN** each result SHALL carry the operations of its own call

#### Scenario: A failure before the call carries no usage
- **WHEN** the model's input is over `maxArgumentsSize`, or is not an object, or the call was aborted before it began
- **THEN** the failure SHALL carry no `usage`

#### Scenario: A failure the runtime finds carries usage
- **WHEN** the model calls `find_plans` with `{ "teamSize": "five" }`
- **THEN** the `invalid-input` failure SHALL carry `usage`, since the call reached the runtime

#### Scenario: The host's own usage object is left alone
- **WHEN** the host creates the tools with runtime options that hold a `usage` object, and a tool call runs
- **THEN** the result SHALL carry the call's `usage`
- **AND** the host's object SHALL be as it was before the call

#### Scenario: An HTTP failure after the arguments function ran carries usage
- **WHEN** an HTTP tool's arguments function returns and the request then cannot be built, or the host's request function fails
- **THEN** the failure SHALL carry the `usage` of the arguments function's call

#### Scenario: Input of the wrong type is invalid input
- **WHEN** the model calls `find_plans` with `{ "teamSize": "five" }`
- **THEN** `execute` SHALL resolve to a failure with code `invalid-input` carrying the runtime's
  boundary diagnostic, and SHALL NOT reject

#### Scenario: A missing required argument is invalid input
- **WHEN** the model calls `find_plans` with `{}`
- **THEN** `execute` SHALL resolve to a failure with code `invalid-input` naming `teamSize`

#### Scenario: A boundary failure of a kind added later is invalid input
- **WHEN** the runtime fails a call with one diagnostic whose code is `nx-ir-boundary-constraint`,
  a code this package has no knowledge of, naming the argument `teamSize`
- **THEN** the failure SHALL have code `invalid-input` and SHALL carry that diagnostic
- **AND** a failure whose diagnostics are that one and `nx-ir-division-by-zero` SHALL have code
  `evaluation-failed`

#### Scenario: A runaway function hits the budget
- **WHEN** a tool's function loops beyond the budget
- **THEN** `execute` SHALL resolve to a failure with code `resource-limit` whose `limit` names
  `maxOperations`, and SHALL NOT hang

#### Scenario: Input that is too large is invalid input
- **WHEN** the model calls `find_plans` with an input object whose size is over `maxArgumentsSize`
- **THEN** `execute` SHALL resolve to a failure with code `invalid-input` whose `limit` is
  `{ name: "maxArgumentsSize", value: 1000 }`
- **AND** the function SHALL NOT have been called

#### Scenario: The host's context does not count against the model's cap
- **WHEN** the host passes a context record of size 900 and the model an input of size 900, under the defaults
- **THEN** the call SHALL proceed

#### Scenario: A context that is too large is a resource limit
- **WHEN** the host passes a context record whose size is one over `maxContextSize` and the model an input of size 5
- **THEN** `execute` SHALL resolve to a failure with code `resource-limit` whose `limit` names
  `maxContextSize`
- **AND** the function SHALL NOT have been called

#### Scenario: A function with two context parameters is given the context twice
- **WHEN** a tool's function has two context parameters and the host passes a context record of size 900 under the defaults
- **THEN** the call SHALL proceed: the runtime's limit allows the context once for each

#### Scenario: The host's own input limit is not used
- **WHEN** the host creates the tools with runtime options that set `maxInputSize: 10` and the model sends an input of size 50
- **THEN** the call SHALL proceed under the package's limits
- **AND** SHALL NOT fail with code `resource-limit`

#### Scenario: A host raises the limits through the package's options
- **WHEN** the host creates the tools with `maxArgumentsSize: 5000` and the model sends an input of size 3,000
- **THEN** the call SHALL proceed

#### Scenario: The arguments function runs under the same limits
- **WHEN** the model calls an HTTP tool, or the host calls `evaluateHttpArguments`, with an input over `maxArgumentsSize`
- **THEN** the result SHALL be a failure with code `invalid-input` whose `limit` names `maxArgumentsSize`
- **AND** the arguments function SHALL NOT have been called

#### Scenario: The package's default applies when the host sets no budget
- **WHEN** the host creates the tools with no runtime options
- **THEN** every function-tool and arguments-function call SHALL run under `maxOperations: 100000`

#### Scenario: The host's budget is used
- **WHEN** the host creates the tools with runtime options that set `maxOperations: 200000`
- **THEN** every function-tool and arguments-function call SHALL run under that budget

#### Scenario: A failure inside the function is an evaluation failure
- **WHEN** a tool's function fails at run time for a reason other than its arguments or a limit
- **THEN** `execute` SHALL resolve to a failure with code `evaluation-failed`

#### Scenario: A context record that does not fit is the host's mistake
- **WHEN** a tool's function is `let lookupOrder(orderId:string, context:ChatToolContext): string`,
  where `ChatToolContext` declares a required `conversationId:string`, and the host passes the
  context record `{ "$type": "ChatToolContext" }` with the input `{ "orderId": "A1" }`
- **THEN** `execute` SHALL resolve to a failure with code `invalid-context` carrying the runtime's
  diagnostic, whose `argument` is `context`
- **AND** a call of the same tool with the input `{ "orderId": 7 }` and a context record that fits
  SHALL fail with code `invalid-input`, with a diagnostic whose `argument` is `orderId`

#### Scenario: A context record of another type is the host's mistake
- **WHEN** the host passes that tool a context record whose `$type` names a type that is not
  `ChatToolContext` and does not extend it
- **THEN** the failure SHALL have code `invalid-context`

#### Scenario: A boundary failure that names no argument is not the model's
- **WHEN** the runtime fails a call with one diagnostic whose code is `nx-ir-boundary-type` and
  which names no argument
- **THEN** the failure SHALL have code `evaluation-failed`

#### Scenario: A value in the context that is not a canonical value is the host's mistake
- **WHEN** a context type declares `inner?:Inner`, a record, and `extra?:object`, and the host
  passes a context record whose `inner` is an instance of a class of its own, or whose `extra`
  holds a `Date`
- **THEN** `execute` SHALL resolve to a failure with code `invalid-context` that carries the
  runtime's diagnostic, whose `argument` is `context` and whose message names `context.inner` or
  the path to the `Date` and says what the value is not
- **AND** the members of `inner` written as a plain object, one of them `undefined`, SHALL succeed
