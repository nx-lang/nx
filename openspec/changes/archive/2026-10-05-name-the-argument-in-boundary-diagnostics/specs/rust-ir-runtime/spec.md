## ADDED Requirements

### Requirement: Rust runtime diagnostics name the argument a failure is in
A diagnostic for a failure the Rust runtime finds in a value a host passed as an argument of the
function it called, by name or by position, SHALL carry the name of the parameter the value was
passed for, as data beside the message. It SHALL carry that name for the same failures the
TypeScript runtime names an argument for, and SHALL carry none for the failures the TypeScript
runtime names none for: a failure a default raises, a resource limit whenever it is reached, a
failure of the image or the program found while a value is checked, a failure while the body is
evaluated, a failure in the result, more positional arguments than the function has parameters,
a `Function` record given to `call_function` to say which function to call that is not such a
record or names no function, and a failure of any other entry point. For the same image and the
same arguments the two runtimes SHALL name the same argument, or both name none. No diagnostic's
code SHALL change.

#### Scenario: A value of the wrong type names its parameter
- **WHEN** a host calls `let findPlans(teamSize:int, note?:string): string` with `teamSize` bound
  to the string `five`
- **THEN** the call SHALL fail with `nx-ir-boundary-type` and the diagnostic SHALL name the
  argument `teamSize`

#### Scenario: A missing required argument names its parameter
- **WHEN** a host calls that `findPlans` with no arguments
- **THEN** the call SHALL fail with `nx-ir-arguments` and the diagnostic SHALL name the argument
  `teamSize`

#### Scenario: The two runtimes name an argument alike
- **WHEN** the same image is called in both runtimes with each of: a value of the wrong type, a
  record with a wrong field three levels down, a missing required argument, a `Function` record
  that names no function, input over the input limit, an operation budget spent while an argument
  is checked, a record whose field default divides by zero, arguments that fit followed by a
  division by zero in the body, a record whose field default fails with `nx-ir-arguments`, and a
  parameter left out whose default fails with `nx-ir-arguments`
- **THEN** for each call both diagnostics SHALL name the same argument, or both SHALL name none
- **AND** the conformance corpus SHALL hold each of those calls as a case or as a check it makes
  of a case, so that a difference fails the corpus tests of the runtime that differs

#### Scenario: A limit names no argument
- **WHEN** a host calls a function with input over the input limit, or with fifty records for a
  parameter `items:Item+` under an operation budget of twenty
- **THEN** each diagnostic SHALL name its limit and no argument

#### Scenario: A failure a field default raises names no argument
- **WHEN** a program declares `type Req = { n:int share:int = { 100 / n } }` and
  `let f(req:Req): int`, and a host calls `f` with a `Req` whose `n` is `0`
- **THEN** the diagnostic SHALL name no argument

#### Scenario: A default that fails with a code an argument's failure has names no argument
- **WHEN** a host passes a record that fits its type and one of whose field defaults calls a
  function value the record holds without an argument that function requires, or leaves out a
  parameter whose default makes such a call
- **THEN** the call SHALL fail with `nx-ir-arguments` and the diagnostic SHALL name no argument

#### Scenario: A failure inside the function names no argument
- **WHEN** a host calls a function with arguments that fit its parameters and the body then fails
- **THEN** the diagnostic SHALL name no argument
