## ADDED Requirements

### Requirement: TypeScript runtime reads a host value in its JavaScript form
A value a host passes to an evaluation API SHALL be read, at any depth, as a canonical value in
the form `nx-ir-format` requires this runtime to define, which is: `null`, a boolean, a number, a
string, an array, or a plain object, which is an object whose prototype is `Object.prototype`, of
this realm or another, or nothing. The handler and function values the runtime makes for its own
use, which a host passes back as it received them, SHALL be accepted where they are today. A host
SHALL NOT need to have read the value from JSON or to write it as JSON: an object and an array
the host built are the value. This SHALL hold for every value the input limit covers: arguments,
props, content, state, a state patch and the entries of a batch. What holds those values is the
host's too: positional arguments, content and a batch SHALL each be an array, and props, a state,
a state patch and arguments by name SHALL each be a plain object. Anything else in their place
SHALL be refused as a value that is not canonical is, with no `argument`, so that nothing the
input measure counts as one value is a list the call goes on to read. The types the evaluation
APIs declare for their input SHALL admit a member that is `undefined`, and the types of what they
return SHALL NOT.

A member of a plain object whose value is `undefined` SHALL be absent, everywhere the runtime
reads the object. The input measure SHALL NOT count it. The check SHALL treat it as a member that
was left out: a field with a default takes its default, an optional field is empty, a required
field with no default is reported missing, and a name the type does not declare is not reported
unknown. The program SHALL NOT be given it: a value held at a site typed `object` SHALL be held
without the member, and the host's own object SHALL NOT be changed.

A value that is not canonical SHALL be refused: after the input limit, so that input over the
limit is still refused by the limit, and before any value is checked against a type or any node
is evaluated. The failure SHALL be `nx-ir-boundary-type` with a message that names the path to
the value and says what it is not. Every value the call goes on to check or to hold SHALL be read
for this, whether or not a type would have looked at it: a value at a site typed `object`
included, at any depth inside it. A plain object or an array that holds itself SHALL be refused
the same way when no input limit refuses it first, and SHALL NOT exhaust the engine's stack or
memory; the same object held twice, side by side, is not a value that holds itself and SHALL be
accepted, and is read once for each place it is held, as the input size counts it. An item of an array that is `undefined` is not a member that can be absent: it SHALL be
read as it is today where the array is a call's positional arguments, and SHALL be refused
anywhere else.

A refusal of a value that is in what the host passed for a parameter of the function it called,
through `callFunction` or `evaluateFunction`, is a failure in that argument, and its diagnostic
SHALL carry `argument` as *TypeScript runtime diagnostics name the argument a failure is in*
requires. A refusal anywhere else SHALL carry none.

Reading the input SHALL charge nothing to `maxOperations` and SHALL NOT recurse more deeply than
a fixed bound, however deeply the input is nested. The input size
SHALL still be taken first, and `measureInputSize` SHALL still need no program: a value that is
not canonical counts one there and is refused after. No recorded result, operation count or input
size SHALL change for input that was accepted before and is accepted now.

#### Scenario: A value the host built is the value
- **WHEN** a host calls `let greet(person:Person): string`, where
  `type Person = { name:string nickname?:string title:string = "Dr" }`, with a `person` it built
  as an object literal, and again with one it read with `JSON.parse`
- **THEN** the two calls SHALL have the same result, operation count and input size

#### Scenario: An optional field set to undefined is absent
- **WHEN** a host calls `greet` with
  `{ person: { name: "Ada", nickname: undefined, title: undefined } }`
- **THEN** the call SHALL succeed, with `nickname` empty and `title` holding `Dr`
- **AND** its `inputSize` SHALL equal that of the call with `{ person: { name: "Ada" } }`

#### Scenario: A required field set to undefined is missing
- **WHEN** a host passes that function `{ person: { name: undefined } }`
- **THEN** the call SHALL fail with `nx-ir-boundary-field` naming the missing field `name`

#### Scenario: An undeclared member that is undefined is not unknown
- **WHEN** a host passes that function `{ person: { name: "Ada", extra: undefined } }`
- **THEN** the call SHALL succeed
- **AND** the same call with `extra: 1` SHALL fail with `nx-ir-boundary-field` naming `extra`

#### Scenario: An instance of a class is not a record
- **WHEN** a host passes that function a `person` that is an instance of a class with the members
  `name` and `nickname`
- **THEN** the call SHALL fail with `nx-ir-boundary-type` naming `person` and saying it is not a
  plain object
- **AND** the diagnostic's `argument` SHALL be `person`
- **AND** an object with no prototype holding the same members SHALL be accepted

#### Scenario: A value that is not canonical is refused inside an open object
- **WHEN** a host calls `let keep(extra:object): string` with `extra` holding, three levels down
  inside plain objects and arrays, a `Uint8Array` of five million bytes, or a `Date`, a `Map`, a
  function, a symbol or a big integer
- **THEN** each call SHALL fail with `nx-ir-boundary-type` naming the path to the value, with the
  `argument` `extra`
- **AND** the typed array SHALL NOT be read: the refusal SHALL take no longer than for one of
  eight bytes

#### Scenario: A refusal outside a function's arguments names no argument
- **WHEN** a host renders a component with props that hold a `Date`
- **THEN** the call SHALL fail with `nx-ir-boundary-type` naming the path to the value
- **AND** the diagnostic SHALL carry no `argument`

#### Scenario: An undefined member inside an open object is not given to the program
- **WHEN** a host calls `let echo(extra:object): object = { extra }` with
  `{ extra: { a: 1, b: undefined, c: { d: undefined } } }`
- **THEN** the result SHALL be `{ a: 1, c: {} }`
- **AND** the object the host passed SHALL still have its member `b`

#### Scenario: An open object that holds itself is refused
- **WHEN** a host passes `keep` a plain object one of whose members is the object itself, with no
  input limit set
- **THEN** the call SHALL fail with `nx-ir-boundary-type` and SHALL NOT exhaust the stack or fail
  to return

#### Scenario: The same object held twice is accepted
- **WHEN** a host passes `keep` `{ first: shared, second: shared }`, where `shared` is one plain
  object
- **THEN** the call SHALL succeed

#### Scenario: A list that is not an array is refused
- **WHEN** a host passes content or a batch as a `Set`, or positional arguments as an object with
  a `length`, whatever they hold and under any input limit they are within
- **THEN** each call SHALL fail with `nx-ir-boundary-type` saying an array was expected, with no
  `argument`
- **AND** nothing the `Set` or the object holds SHALL have been checked or evaluated

#### Scenario: A member set to undefined is written without a cast
- **WHEN** a TypeScript host passes `{ name: "Ada", nickname: undefined }` as an argument, as
  props, as a state or as a state patch
- **THEN** the call SHALL type-check against the package's declarations

#### Scenario: Input over the limit is refused by the limit first
- **WHEN** a host passes `keep` a value that holds a `Date` and is larger than `maxInputSize`
- **THEN** the call SHALL fail with `nx-ir-resource-limit` naming `maxInputSize`
- **AND** the diagnostic SHALL carry no `argument`

#### Scenario: A handler the runtime made is still accepted
- **WHEN** a host renders a component, takes an action handler from the rendered output and passes
  it back as a prop of a child
- **THEN** the call SHALL be accepted as it is today

#### Scenario: Accepted input costs what it cost
- **WHEN** the conformance corpus is evaluated
- **THEN** every recorded result, operation count and input size SHALL be as recorded
