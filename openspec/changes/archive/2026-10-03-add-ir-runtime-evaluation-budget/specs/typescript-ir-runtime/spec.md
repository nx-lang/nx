## ADDED Requirements

### Requirement: TypeScript runtime enforces an operation budget
The runtime options every evaluation API accepts SHALL include `maxOperations`, the most
operations one call may cost, counted as `nx-ir-format` defines an operation. When the option is
absent the call SHALL be unlimited, so a host that does not set it sees no change. A value that is
not a non-negative safe integer SHALL be refused with a diagnostic naming the option before
anything is evaluated; it SHALL NOT be read as unlimited.

One budget SHALL cover one call of a public API and everything that call evaluates: the function
and every function it calls, parameter defaults, value initializers, a component's state defaults
and body, and, for `dispatchComponentActions`, every handler the batch runs and the render that
follows. The APIs are `evaluateFunction`, `callFunction`, `constructComponentDescriptor`,
`initializeComponent`, `evaluateComponent`, `dispatchComponentActions`, `normalizeComponentState`
and `applyComponentStatePatch`. Each call SHALL start with the whole budget; nothing SHALL carry
over from an earlier call.

A charge that would take the count above the budget SHALL fail the call with
`nx-ir-resource-limit` before the operation charged is performed. The diagnostic SHALL name the
declaration the charge belongs to, as `nx-ir-format` assigns it, and, when the charge is for a
node and the image carries its debug section, the node's span, and its `limit` SHALL be
`{ name: "maxOperations", value }` with the budget the host set. The call SHALL return nothing, and a failed dispatch SHALL leave the instance it was given
unchanged and usable, as for any other failure. A budget equal to the cost of the call SHALL
succeed.

#### Scenario: Nested loops end at the budget
- **WHEN** an image evaluates `for a in 0..1000 { for b in 0..1000 { for c in 0..1000 { a + b + c } } }` under `maxOperations: 100000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` is `{ name: "maxOperations", value: 100000 }`
- **AND** the diagnostic SHALL name the function the loops are written in
- **AND** no more than one hundred thousand operations SHALL have been performed

#### Scenario: An absent budget is unlimited
- **WHEN** an image evaluates `for i in 0..1000000 { i }` under options that do not set `maxOperations`
- **THEN** evaluation SHALL return the list of one million integers

#### Scenario: The budget is exact
- **WHEN** an evaluation costs `n` operations
- **THEN** it SHALL succeed under `maxOperations: n`
- **AND** it SHALL fail with `nx-ir-resource-limit` under `maxOperations: n - 1`

#### Scenario: A list that doubles on each call is stopped
- **WHEN** an image evaluates `let grow(n:int, xs:int+): int+ = { if n == 0 { xs } else { grow(n - 1, { xs xs }) } }` with `60` and a list of one integer under `maxOperations: 100000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`
- **AND** no list longer than one hundred thousand items SHALL have been built

#### Scenario: A string that doubles on each call is stopped
- **WHEN** an image evaluates `let grow(n:int, s:string): string = { if n == 0 { s } else { grow(n - 1, s + s) } }` with `60` and `"x"` under `maxOperations: 100000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`

#### Scenario: A value shared many times over is stopped wherever it is walked
- **WHEN** a function builds a record that holds one value twice, forty levels deep, and the value is checked against its type at each call, compared with another like it, or returned to the host, under `maxOperations: 100000`
- **THEN** each evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`
- **AND** SHALL NOT take time or memory that grows with the 2^40 values the record is as a tree

#### Scenario: A result too large for the budget is refused as it is written
- **WHEN** a function returns a list of 100 references to one string of 16,384 UTF-16 code units under `maxOperations: 1000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`
- **AND** the diagnostic SHALL name the function and carry no span

#### Scenario: A batch shares one budget
- **WHEN** a host dispatches a batch of three handler invocations, each of whose handlers costs more than a third of `maxOperations`
- **THEN** dispatch SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`
- **AND** the instance given SHALL dispatch a later, cheaper batch successfully

#### Scenario: A state default is under the budget
- **WHEN** a component's state field defaults to `for i in 0..100000 { i }` and a host initializes it under `maxOperations: 1000`
- **THEN** initialization SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxOperations`

#### Scenario: Each call starts with the whole budget
- **WHEN** a host evaluates a function costing 600 operations twice with the same options object, whose `maxOperations` is 1000
- **THEN** both evaluations SHALL succeed

#### Scenario: A budget that is not a count is refused
- **WHEN** a host evaluates a function under `maxOperations: NaN`, `maxOperations: -1` or `maxOperations: 1.5`
- **THEN** each call SHALL fail with a diagnostic naming `maxOperations`
- **AND** no node SHALL have been evaluated

### Requirement: TypeScript runtime resource-limit diagnostics name the limit
Every diagnostic the runtime reports with the code `nx-ir-resource-limit` SHALL carry a `limit`
with the `name` of the limit that was reached and, where the limit is a number, its `value`. The
names SHALL be `maxOperations`, `maxCallDepth` and `maxRangeLength` for the limits a host sets,
`maxExpressionNesting` for the fixed nesting bound, and `engine` for a limit of the JavaScript
engine, which has no value. A diagnostic with any other code SHALL carry no `limit`. The message
SHALL continue to state the limit in words.

#### Scenario: Runaway recursion names the call-depth limit
- **WHEN** a program evaluates a function that calls itself without end under the default options
- **THEN** the diagnostic's `limit` SHALL be `{ name: "maxCallDepth", value: 100 }`

#### Scenario: An oversized range names the range limit
- **WHEN** an image evaluates `for i in 0..2000000 { i }` under the default options
- **THEN** the diagnostic's `limit` SHALL be `{ name: "maxRangeLength", value: 1000000 }`

#### Scenario: A host tells the limits apart without the message
- **WHEN** one evaluation fails for an exhausted budget and another for runaway recursion
- **THEN** the two diagnostics SHALL have the same code and different `limit.name` values

### Requirement: TypeScript runtime bounds expression nesting and reports engine limits as diagnostics
The runtime SHALL refuse to nest expressions more than 1,000 deep across every call of one
evaluation, the bound the Rust runtime holds, failing with `nx-ir-resource-limit` whose `limit` is
`{ name: "maxExpressionNesting", value: 1000 }`. The bound SHALL be fixed: raising `maxCallDepth`
SHALL NOT raise it.

A `RangeError` the JavaScript engine raises while a public API evaluates — an exhausted call
stack, a string longer than the engine holds, an array longer than the engine holds — SHALL be
reported as an `NxIrRuntimeError` with the code `nx-ir-resource-limit` whose `limit` names
`engine`, and SHALL NOT reach the host as a `RangeError`. Placing the items of one sequence in
another, or the effects of a dispatched handler in a batch's effects, SHALL NOT depend on how many
arguments the engine accepts in one call.

#### Scenario: Recursion past a raised call depth is still a diagnostic
- **WHEN** a program evaluates a function that calls itself without end under `maxCallDepth: 1000000`
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose `limit` names `maxExpressionNesting` or `engine`
- **AND** the call SHALL NOT throw a `RangeError`

#### Scenario: A long list is spliced
- **WHEN** an image evaluates an element whose content is `for i in 0..200000 { i }`
- **THEN** evaluation SHALL return the element with two hundred thousand content items

#### Scenario: A handler returns many effects
- **WHEN** a host dispatches an action whose parent-bound handler returns 200,000 actions
- **THEN** dispatch SHALL return the 200,000 effects

#### Scenario: A string past the engine's limit is a diagnostic
- **WHEN** a function doubles a string on each of 40 nested calls under the default options
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit`
- **AND** the call SHALL NOT throw a `RangeError`

### Requirement: TypeScript runtime refuses an integer outside the safe range
The runtime carries every number as a JavaScript number and SHALL NOT hold an integer outside
JavaScript's safe range in any other form. Reading such a literal from an image SHALL fail with
`nx-ir-number`, naming the declaration and, with a debug section, the literal's span; the image
SHALL still prepare, and a path that does not reach the literal SHALL run. The refusal is charged
as any node is: under a budget the node's operation is taken before it fails.

A value the host passes that has the shape canonical JSON gives such an integer,
`{ "$type": "nx.int", "value": "<digits>" }`, SHALL be a record at `object`: returned unchanged,
compared as a record is, matched as a pattern as a record is, and charged as a record is, whatever
else it holds. At a parameter typed as an integer it SHALL be refused, as any value that is not a
number is. The runtime SHALL export nothing that makes such an integer.

#### Scenario: A literal outside the safe range is refused where it is read
- **WHEN** a prepared program evaluates `let big() = { 9007199254740993 }`
- **THEN** evaluation SHALL fail with `nx-ir-number` naming `big` and the literal
- **AND** another function of the same image that does not reach the literal SHALL evaluate

#### Scenario: Whatever would have used it does not run
- **WHEN** a function compares its argument with a literal outside the safe range, or matches it as a pattern
- **THEN** evaluation SHALL fail with `nx-ir-number`

#### Scenario: The record a host passes is a record
- **WHEN** a host passes `{ "$type": "nx.int", "value": "1152921504606846976" }` at `object` and the function returns it
- **THEN** the result SHALL be that record, for the cost of a record with one string field
- **AND** the same value at a parameter typed `int` SHALL fail with `nx-ir-boundary-type`

#### Scenario: Text or fields in that shape are paid for
- **WHEN** a host passes a record named `nx.int` whose `value` is a megabyte of text, or that holds a megabyte beside its `value`, and a function returns it 20 times
- **THEN** the call SHALL be refused under a budget of 100,000
