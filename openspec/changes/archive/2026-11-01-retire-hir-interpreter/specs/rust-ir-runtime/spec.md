## ADDED Requirements

### Requirement: CLI source execution uses the Rust IR runtime
`nxlang run` SHALL execute NX source by analyzing it, emitting NX IR for every module the program
needs, preparing and linking those images and evaluating the entry module's `root` through the Rust
IR runtime. Existing `nxlang run <file.nx>` invocations SHALL keep their meaning and output
formats. The CLI SHALL use a default budget of 1,000,000 IR operations per invocation. The runtime
library's option defaults SHALL remain unchanged. No second Rust evaluator or fallback to direct
HIR execution SHALL exist. Running an NX IR file directly is not part of this requirement.

#### Scenario: Running source evaluates its emitted IR
- **WHEN** a caller runs source `let root() = { 1 + 2 }`
- **THEN** the CLI SHALL print `3`, computed by the Rust IR runtime from the emitted image

#### Scenario: Root calls an imported module
- **WHEN** a caller runs a file whose `root` calls a function imported from `../ui`
- **THEN** the CLI SHALL emit, prepare and link the library module's image beside the entry's
- **AND** SHALL print the value the imported function returns

#### Scenario: Runtime errors preserve available locations
- **WHEN** a caller runs a program whose root divides by zero
- **THEN** execution SHALL fail with `nx-ir-division-by-zero`
- **AND** SHALL report the failing source location when debug/source information is available
- **AND** SHALL identify the module/declaration without inventing source coordinates when it is unavailable

#### Scenario: CLI operation budget resets per invocation
- **WHEN** one CLI invocation exceeds its default operation budget and another runs a small program
- **THEN** the first SHALL fail with the runtime's operation-limit diagnostic
- **AND** the second SHALL execute with a fresh budget

### Requirement: Tooling APIs separate source compilation from IR execution
The public Rust tooling facade and C, .NET, Node and WebAssembly compiler bindings SHALL NOT expose
legacy source/ProgramArtifact evaluation or component lifecycle wrappers. Building, validation,
schemas, language queries, IR emission and canonical-value formatting SHALL remain available where
already supported. Rust IR execution SHALL remain available through the existing facade runtime
namespace and standalone runtime crate. This change SHALL NOT introduce replacement source-execution
convenience APIs or replacement C/.NET execution bindings.

#### Scenario: A Rust caller composes compilation and execution
- **WHEN** a Rust host builds source and emits IR using the tooling facade
- **THEN** it SHALL be able to prepare, link and execute the images through the facade's IR runtime namespace
- **AND** compilation SHALL NOT execute the source

#### Scenario: Tests do not enlarge the public execution surface
- **WHEN** semantic tests need to execute NX source
- **THEN** they SHALL compose build, emission and IR execution in test-only helpers
- **AND** SHALL NOT require a production source-evaluation export

### Requirement: Rust runtime results keep constant cases and function references
Beside `NxValue` results, the Rust runtime SHALL offer a read-only view of a result in which a
constant union case is distinguishable from a string and names its union and case, and a function
value is distinguishable from a record and names its module and declaration. The view SHALL NOT
expose the runtime's mutable internals or handler bodies, and the runtime SHALL stay independent of
the compiler. NX text written from such a view SHALL spell a constant case as its qualified case
and a function by its name, including where the value sits at a site typed `object`. Numbers SHALL
be spelled by the type their position declares, as canonical-value formatting spells them; the
runtime SHALL NOT carry a separate `float32` kind for this.

#### Scenario: An enum case prints as a case
- **WHEN** `type ThemeMode = light | dark` and `let root(): object = { ThemeMode.dark }` are
  evaluated and the result is written as NX text
- **THEN** the text SHALL be `ThemeMode.dark` and SHALL NOT be `"dark"`

#### Scenario: A declared float32 prints as a float32
- **WHEN** `let root(): float32 = { 0.1 }` is evaluated and the result is written as NX text
- **THEN** the text SHALL be `0.1`

#### Scenario: A float32 at an object site prints as its float64 widening
- **WHEN** `let f(): float32 = { 0.1 }` and `let root(): object = { f() }` are evaluated and the
  result is written as NX text
- **THEN** the text SHALL be the shortest `float64` spelling of the widened value, `0.10000000149011612`

### Requirement: Rust runtime behavior is validated against the corpus and source-level expectations
The Rust runtime's tests SHALL prepare, link where the module table requires, and evaluate every
artifact of the conformance corpus, with and without its debug section, and SHALL compare each
named entrypoint's value and each lifecycle's rendered outputs, tokens and effects with the
corpus's expected results. The tests SHALL cut every corpus image at every four-byte boundary and
overwrite every cell of at least one corpus image, and SHALL fail if any such image causes a panic.
The repository's integration tests SHALL also compile NX source and execute emitted IR through
test-only helpers, comparing each result with an expected value written in the test, covering
primitives, arithmetic and its error cases,
conditionals, match expressions, sequences and occurrences, the presence operators, loops and
ranges, recursion and its limit, records and inheritance, type parameters, unions, enums, function
parameters and values, property references, update records and intrinsics, text content, and the
component lifecycle.

#### Scenario: The corpus is part of the runtime's test run
- **WHEN** the Rust runtime's tests run
- **THEN** every corpus entrypoint SHALL be evaluated and every corpus lifecycle initialized and
  dispatched in order
- **AND** a difference from an expected result SHALL fail the run naming the program and the
  entrypoint or lifecycle

#### Scenario: Damage is survived
- **WHEN** the damage tests run
- **THEN** every truncated image SHALL be refused
- **AND** every overwritten image SHALL be refused or evaluated without a panic

#### Scenario: Source-level expectations do not depend on another engine
- **WHEN** a source-level test evaluates an NX program
- **THEN** the expected value SHALL be stated in the test
- **AND** SHALL NOT be computed by evaluating the program another way


### Requirement: Retirement preserves pending construction regression expectations
Interpreter retirement MAY precede executable support for conditional property fragments and
positional record constructors. The source tests for these two gaps SHALL retain their intended
successful results as ignored regressions naming `complete-ir-construction-lowering`. No other
unexpected discrepancy SHALL be accepted under this exception. Temporary CLI limitations SHALL be
documented until the follow-up enables those tests.

#### Scenario: A deferred source behavior keeps its intended result
- **WHEN** a retained test exercises either approved construction gap
- **THEN** it SHALL be ignored with the follow-up change named in its reason
- **AND** its assertions SHALL describe successful evaluation, not the current failure

### Requirement: Component descriptors are recursively validated at typed sites
When a component-typed value has no discriminator or names the expected component declaration,
its props SHALL be recursively normalized against that declaration at host and program-internal
typed sites. Defaults SHALL be filled, unknown props and invalid nested records SHALL be rejected,
and bound emit handlers SHALL retain their identity. Existing cross-component descriptor behavior
SHALL remain unchanged. Both runtimes SHALL charge the same operations for equivalent work.

#### Scenario: A descriptor passes through a typed parameter and result
- **WHEN** a program constructs a component descriptor with a defaulted prop and a bound emit handler
- **AND** passes it through a component-typed function parameter and result
- **THEN** Rust and TypeScript SHALL preserve the same default and handler and report equal operation counts

#### Scenario: A nested host update record is invalid
- **WHEN** a host component-typed prop names the expected declaration and contains an update record with an unknown field or an invalid required-field clear
- **THEN** both runtimes SHALL reject it with a boundary diagnostic


## MODIFIED Requirements

### Requirement: The Rust IR runtime does not depend on the NX compiler
The Rust IR runtime SHALL be a crate whose dependency closure contains the NX IR format crate and
the NX value crate and no crate that parses, lowers, type-checks or emits NX. The NX IR format —
its constants, kind tables, image writer, validating reader and explainer — SHALL be available from
a crate that the emitter and the runtime both depend on and that depends on neither.

#### Scenario: A host builds the runtime without the compiler
- **WHEN** a Rust project depends on the IR runtime crate alone
- **THEN** the build SHALL NOT compile the NX syntax, HIR, type-checking or codegen
  crates

#### Scenario: The emitter and the runtime share one format implementation
- **WHEN** the emitter writes an image and the runtime reads it
- **THEN** both SHALL use the same schema version, runtime ABI, feature names and kind numbers from
  the format crate

## REMOVED Requirements

### Requirement: Rust runtime behavior is validated against the corpus and the interpreter
**Reason**: The interpreter is deleted, so there is no second native engine to compare with.
**Migration**: Replaced by "Rust runtime behavior is validated against the corpus and source-level
expectations", which keeps the corpus and damage tests and replaces the differential run with
tests that state their expected values.
