## ADDED Requirements

### Requirement: TypeScript runtime diagnostics name the argument a failure is in
A diagnostic for a failure the runtime finds in a value a host passed as an argument of the
function it called, through `callFunction` or `evaluateFunction`, SHALL carry `argument`: the name
of the parameter the value was passed for. That covers a value that does not fit the parameter's
type, at any depth inside the value; any other refusal of the value itself, such as a `Function`
record in it that names no function; and a required parameter that was given no argument. The
name SHALL be the parameter's declared name, whether the host passed the arguments by name or by
position.

A diagnostic SHALL carry no `argument` when the failure is not in a value the host passed for one
parameter, even when the runtime finds it while it is checking an argument:

- a failure raised by a default, a parameter's or a record field's: by evaluating its expression,
  by a function it calls, or by its value not fitting the type it is declared with;
- a resource limit, whichever limit it is and whenever it is reached, the budget spent while an
  argument is checked included;
- a failure of the image or the program that the runtime finds while it checks a value, such as a
  schema or a reference it cannot resolve;
- a failure while the function's body is evaluated, including one in the arguments of a function
  the body calls;
- a failure in the function's result;
- more positional arguments than the function has parameters;
- the `Function` record `callFunction` is given to say which function to call, when it is not such
  a record or names no function: it is not an argument of the function;
- a failure of any other entry point.

The member is data for a program to read. The message SHALL continue to name the argument in
words, and no diagnostic's code SHALL change.

#### Scenario: A value of the wrong type names its parameter
- **WHEN** a host calls `let findPlans(teamSize:int, note?:string): string` through `callFunction`
  with `{ teamSize: "five" }`
- **THEN** the call SHALL fail with `nx-ir-boundary-type` and the diagnostic's `argument` SHALL be
  `teamSize`

#### Scenario: A failure deep inside a value names the parameter that holds it
- **WHEN** a host calls a function whose parameter `request` is a record with a field `items` of
  records, and the third item's `quantity` is a string where an `int` is declared
- **THEN** the diagnostic's `argument` SHALL be `request`

#### Scenario: A missing required argument names its parameter
- **WHEN** a host calls that `findPlans` with `{}`
- **THEN** the call SHALL fail with `nx-ir-arguments` and the diagnostic's `argument` SHALL be
  `teamSize`

#### Scenario: A positional argument is named by its parameter
- **WHEN** a host calls `findPlans` through `evaluateFunction` with the positional arguments
  `["five"]`
- **THEN** the diagnostic's `argument` SHALL be `teamSize`

#### Scenario: A function record that names no function names its parameter
- **WHEN** a host calls a function whose parameter `step` is of a function type with the record
  `{ $type: "Function", module: "main.nx", name: "missing" }`
- **THEN** the call SHALL fail with `nx-ir-function-value` and the diagnostic's `argument` SHALL
  be `step`

#### Scenario: The function record of the call itself names no argument
- **WHEN** a host calls `callFunction` with the record
  `{ $type: "Function", module: "main.nx", name: "missing" }` as the function to call
- **THEN** the call SHALL fail with `nx-ir-function-value` and the diagnostic SHALL carry no
  `argument`

#### Scenario: A failure a field default raises names no argument
- **WHEN** a program declares `type Req = { n:int share:int = { 100 / n } }` and
  `let f(req:Req): int`, and a host calls `f` with `{ req: { $type: "Req", n: 0 } }`, a value that
  fits `Req`
- **THEN** the call SHALL fail with the code a division by zero has and the diagnostic SHALL carry
  no `argument`
- **AND** a field default whose value does not fit its field's type SHALL fail with a diagnostic
  that carries no `argument`, whatever its code

#### Scenario: A parameter's default names no argument
- **WHEN** a host calls `let g(n:int, share:int = { 100 / n }): int` with `{ n: 0 }`
- **THEN** the diagnostic SHALL carry no `argument`

#### Scenario: A default that fails with a code an argument's failure has names no argument
- **WHEN** a program declares
  `type Stepper = { step: <function n:int />: int value:int = { step(1) } }`,
  `let stepperValue(s:Stepper): int` and `let needsTwo(n:int, m:int): int`, and a host calls
  `stepperValue` with a `Stepper` whose `step` is the `Function` record of `needsTwo` and that has
  no `value`
- **THEN** the call SHALL fail with `nx-ir-arguments`, raised by the call the default makes, and
  the diagnostic SHALL carry no `argument`
- **AND** a call of
  `let stepped(step: <function n:int />: int, value:int = { <step n={1} /> }): int` with that
  record for `step` and nothing for `value` SHALL fail with `nx-ir-arguments` and carry no
  `argument`
- **AND** the same call of `stepped` with the string `one` for `value` SHALL fail with a diagnostic
  whose `argument` is `value`

#### Scenario: A failure inside the function names no argument
- **WHEN** a host calls a function with arguments that fit its parameters, and the function's body
  then fails, by dividing by zero or in a call it makes to another function
- **THEN** the diagnostic SHALL carry no `argument`

#### Scenario: A limit names no argument
- **WHEN** a host calls a function with input over `maxInputSize`, or with fifty records for a
  parameter `items:Item+` under an operation budget of twenty, so that the budget is spent while
  the argument is checked
- **THEN** each diagnostic SHALL carry its `limit` and no `argument`
