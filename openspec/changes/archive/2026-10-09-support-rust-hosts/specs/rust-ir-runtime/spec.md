## MODIFIED Requirements

### Requirement: Rust runtime bounds evaluation and never panics
Every evaluation API SHALL accept runtime options holding a maximum call depth, defaulting to 100,
a maximum range length, defaulting to one million, an operation budget, absent by default, and a
stack budget, defaulting to one mebibyte. Exceeding the call depth or the range length SHALL fail
with `nx-ir-resource-limit` naming the limit, and a range above its limit SHALL be refused before
its body runs. Every API SHALL report every failure as an error value carrying diagnostics. No
image that preparation accepted, no host value, and no instance SHALL cause the runtime to panic
or, on a thread with the stack budget free where the call began, to exhaust the native stack.

The stack budget SHALL be the native stack one call of an evaluation API may use, measured from
where the call began, and SHALL bound every recursion the runtime makes over an image or a value
on behalf of that call: evaluating nested expressions, checking a value against a type, converting
a value at the host boundary, comparing two values, and finding how deeply state nests after a
patch. Reporting a failure SHALL NOT take stack in proportion to how deeply the failing value sits:
a diagnostic names where the value is, and that path is as long as the value is deep and is
written where the failure was found, which is where the least stack is left. An API that takes no options, which the update helpers and the comparisons of an instance's
handlers are, SHALL walk under the default budget. An evaluation that would use more SHALL fail with `nx-ir-resource-limit` naming
`maxStackBytes` and the budget in force, before the stack is exceeded. The budget is the host's
statement of what its thread has free: the runtime SHALL honor a budget below the default, so that
a host whose whole stack is smaller than a mebibyte gets a diagnostic where the default would
overrun it, and a budget above it. The bound on nested expressions SHALL stay fixed and SHALL apply
whatever the stack budget is.

The operation budget SHALL be counted as `nx-ir-format` defines an operation, and an absent budget
SHALL be unlimited. One budget SHALL cover one call of an evaluation API and everything that call
evaluates — for `dispatch_component_actions`, every handler the batch runs and the render that
follows — and each call SHALL start with the whole budget. A charge that would take the count above
the budget SHALL fail the call with `nx-ir-resource-limit` before the operation charged is
performed, naming the declaration the charge belongs to, as `nx-ir-format` assigns it, and, when
the charge is for a node and the image carries its debug section, the node's span. A walk the
runtime makes over a value for a purpose of its own, such as bounding how deeply state nests,
SHALL visit a value reached from several places once. For the same images, the same input and the same budget, the
Rust runtime SHALL fail at the node the TypeScript runtime fails at, and SHALL succeed where it
succeeds, wherever the two runtimes compute the same values.

#### Scenario: Unbounded recursion ends in a diagnostic
- **WHEN** a program evaluates a function that calls itself without end
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` naming the call-depth limit

#### Scenario: An oversized range is refused
- **WHEN** an image evaluates `for i in 0..2000000 { i }` under the default options
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` naming the limit of one million
- **AND** the same image SHALL evaluate under options that raise the limit above two million

#### Scenario: An altered image that prepares is evaluated safely
- **WHEN** an image with one overwritten cell passes preparation and its entrypoints are evaluated
- **THEN** each evaluation SHALL either succeed or return an error with a diagnostic
- **AND** SHALL NOT panic

#### Scenario: Nested loops end at the budget
- **WHEN** an image evaluates `for a in 0..1000 { for b in 0..1000 { for c in 0..1000 { a + b + c } } }` under an operation budget of one hundred thousand
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` naming the operation budget and its value
- **AND** no more than one hundred thousand operations SHALL have been performed

#### Scenario: An absent budget is unlimited
- **WHEN** an image evaluates `for i in 0..1000000 { i }` under the default options
- **THEN** evaluation SHALL return the list of one million integers

#### Scenario: The budget is exact and agrees with the TypeScript runtime
- **WHEN** an evaluation costs `n` operations in the TypeScript runtime
- **THEN** the Rust runtime SHALL evaluate it under a budget of `n`
- **AND** SHALL fail with `nx-ir-resource-limit` under a budget of `n - 1`, naming the same declaration and, with a debug section, the same span

#### Scenario: A value that doubles on each call is stopped before it is allocated
- **WHEN** a function doubles a list, or a string, on each of 60 nested calls under an operation budget of one hundred thousand
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` naming the operation budget
- **AND** the runtime SHALL NOT have allocated a list or a string the budget does not pay for

#### Scenario: A value shared many times over is stopped wherever it is walked
- **WHEN** a function builds a record that holds one value twice, forty levels deep, and the value is checked against its type at each call, compared with another like it, returned to the host, or stored in component state, under an operation budget of one hundred thousand
- **THEN** each evaluation SHALL fail with `nx-ir-resource-limit` naming the operation budget
- **AND** SHALL NOT take time or memory that grows with the 2^40 values the record is as a tree

#### Scenario: A batch shares one budget and leaves the instance usable
- **WHEN** a host dispatches a batch whose handlers together cost more than the operation budget
- **THEN** dispatch SHALL fail with `nx-ir-resource-limit` naming the operation budget
- **AND** the instance given SHALL dispatch a later, cheaper batch successfully


#### Scenario: A small stack budget ends deep recursion with a diagnostic
- **WHEN** a host evaluates a function that recurses several hundred calls deep under a raised
  call-depth limit and a stack budget of 64 kibibytes
- **THEN** evaluation SHALL fail with `nx-ir-resource-limit` whose limit is named `maxStackBytes`
  with the value `65536`
- **AND** the same call under a stack budget that holds the build's frames, which in an optimized
  build the default does, SHALL return the function's result or fail for the nesting bound

#### Scenario: The default stack budget is unchanged
- **WHEN** a host evaluates under default options
- **THEN** the stack budget in force SHALL be `1048576` bytes

#### Scenario: A deeply nested host value is refused within the stack budget
- **WHEN** a host passes a value nested 200 levels deep to a parameter of a declared type under a
  stack budget too small to check it
- **THEN** the call SHALL fail with `nx-ir-resource-limit` naming `maxStackBytes`
- **AND** SHALL NOT exhaust the native stack

#### Scenario: A comparison of deep values stops at the stack budget
- **WHEN** two values nested 200 levels deep are compared, or state that deep is checked after a
  patch, under a stack budget that a few levels use up
- **THEN** the walk SHALL fail with `nx-ir-resource-limit` naming `maxStackBytes`
- **AND** a function that recurses to a depth of its caller's choosing and then compares two host
  values nested 100 levels deep SHALL, on a thread whose stack is the budget and a margin, return
  the comparison's result or that diagnostic at every depth, and SHALL NOT exhaust the stack

#### Scenario: A failure at the bottom of a deep value is reported within the stack budget
- **WHEN** a host passes a record nested 200 levels deep whose innermost field has the wrong type,
  on a thread whose stack is the budget and a margin, for budgets on both sides of the one the
  check first fits in
- **THEN** each call SHALL fail with `nx-ir-resource-limit` naming `maxStackBytes` or with
  `nx-ir-boundary-type` naming the whole path to the field
- **AND** no call SHALL exhaust the stack

## ADDED Requirements

### Requirement: Rust runtime builds for the WebAssembly targets hosts use
The Rust IR runtime crate and the crates it depends on SHALL build for `wasm32-unknown-emscripten`
and `wasm32-wasip1` without features, patches or a C toolchain, and the repository's continuous
integration SHALL check both on every pull request. The runtime's documentation SHALL state how
much stack a release build's deepest permitted evaluation was measured to use on a WebAssembly
target, and what a host whose stack is smaller than the default budget sets: a stack budget no
larger than what is free where it calls the runtime, or a larger stack at link time.

#### Scenario: A pull request that breaks a WebAssembly build fails
- **WHEN** a change makes the runtime crate, the format crate or the value crate fail to compile
  for `wasm32-unknown-emscripten` or `wasm32-wasip1`
- **THEN** the pull request's build SHALL fail naming the target

#### Scenario: A host on a small stack finds what to set
- **WHEN** a host author reads the runtime crate's README
- **THEN** it SHALL say that the thread calling the runtime needs the stack budget free
- **AND** it SHALL give the measured stack use on a WebAssembly target and the option that lowers
  the budget
